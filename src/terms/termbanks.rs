use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::inout::scanner::{test_tok as scanner_test_tok, token_pos_rep, Scanner, TokenType};
use crate::terms::dbvars::DbVarBank;
use crate::terms::functypes::{func_symb_parse, FunCode, FuncSymbType};
use crate::terms::garbage_coll::GcAdmin;
use crate::terms::lambda::apply_terms as lambda_apply_terms;
use crate::terms::signature::{Signature, SIG_CONS_CODE, SIG_NIL_CODE, SIG_TRUE_CODE};
use crate::terms::signature::{
    FP_FOF_OP, SIG_DB_LAMBDA_CODE, SIG_FALSE_CODE, SIG_ITE_CODE, SIG_LET_CODE,
    SIG_NAMED_LAMBDA_CODE,
};
use crate::terms::simpletypes::{
    alloc_arrow_type, flatten_type, type_drop_first_arg, type_get_max_arity, type_is_predicate,
    Type, TypeUniqueId,
};
use crate::terms::termcellstore::TermCellStore;
use crate::terms::termfunc::{
    reject_term_bank_distinct_argument_list, term_apply_arg as term_apply_arg_unshared,
    term_array_no_duplicates, term_copy, term_is_ground_compute, term_parse_operator,
    term_sig_insert, term_standard_weight, var_print_string,
};
use crate::terms::termtypes::{
    term_deref, term_identity_id, DerefType, Term, TermProperties, DEFAULT_FWEIGHT,
    DEFAULT_VWEIGHT, TP_GARBAGE_FLAG, TP_HAS_APP_VAR, TP_HAS_BOOL_SUBTERM, TP_HAS_DB_SUBTERM,
    TP_HAS_EQ_NEQ_SYM, TP_HAS_ETA_EXPANDABLE_SUBTERM, TP_HAS_LAMBDA_SUBTERM,
    TP_HAS_NON_PATTERN_VAR, TP_IGNORE_PROPS, TP_IS_BETA_REDUCIBLE, TP_IS_GROUND, TP_IS_SHARED,
    TP_OP_FLAG, TP_OUTPUT_FLAG, TP_PRED_POS, TP_TOP_POS,
};
use crate::terms::termvars::VarBank;
use crate::terms::typecheck::{type_declare_is_predicate, type_infer_sort};
use std::collections::BTreeMap;
use std::fmt;

const INSERT_NO_PROPS_CACHE_THRESHOLD: u32 = 8096;

#[derive(Clone, Debug)]
pub struct TermBank {
    in_count: u64,
    insertions: u64,
    recovered: u64,
    sig: Signature,
    vars: VarBank,
    db_vars: DbVarBank,
    true_term: Term,
    false_term: Term,
    min_terms: BTreeMap<TypeUniqueId, Term>,
    garbage_state: TermProperties,
    gc: GcAdmin,
    term_store: TermCellStore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LetTypeDeclaration {
    name: String,
    f_code: FunCode,
    type_: Type,
}

fn let_type_declaration_codes(declarations: &[LetTypeDeclaration]) -> Vec<FunCode> {
    declarations
        .iter()
        .map(|declaration| declaration.f_code)
        .collect()
}

impl TermBank {
    pub fn new(sig: Signature) -> Result<Self, Diagnostic> {
        let vars = VarBank::new(sig.type_bank());
        let true_term = Term::const_cell_alloc(SIG_TRUE_CODE);
        let false_term = Term::const_cell_alloc(SIG_FALSE_CODE);
        let mut bank = Self {
            in_count: 0,
            insertions: 0,
            recovered: 0,
            sig,
            vars,
            db_vars: DbVarBank::new(),
            true_term: true_term.clone(),
            false_term: false_term.clone(),
            min_terms: BTreeMap::new(),
            garbage_state: TP_IGNORE_PROPS,
            gc: GcAdmin::new(),
            term_store: TermCellStore::new(),
        };

        true_term.set_type(Some(bank.sig.type_bank().bool_type()));
        true_term.set_prop(TP_PRED_POS);
        bank.true_term = bank.insert(&true_term, DerefType::Never)?;

        false_term.set_type(Some(bank.sig.type_bank().bool_type()));
        false_term.set_prop(TP_PRED_POS);
        bank.false_term = bank.insert(&false_term, DerefType::Never)?;

        Ok(bank)
    }

    #[must_use]
    pub const fn in_count(&self) -> u64 {
        self.in_count
    }

    #[must_use]
    pub const fn insertions(&self) -> u64 {
        self.insertions
    }

    #[must_use]
    pub const fn recovered(&self) -> u64 {
        self.recovered
    }

    #[must_use]
    pub const fn signature(&self) -> &Signature {
        &self.sig
    }

    #[must_use]
    pub fn signature_mut(&mut self) -> &mut Signature {
        &mut self.sig
    }

    #[must_use]
    pub const fn vars(&self) -> &VarBank {
        &self.vars
    }

    pub(crate) fn request_db_var(&mut self, type_: &Type, db_index: FunCode) -> Term {
        self.db_vars.request_db_var(type_, db_index)
    }

    #[must_use]
    pub(crate) fn copy_term(&mut self, source: &Term, deref: DerefType) -> Term {
        term_copy(source, &self.vars, Some(&mut self.db_vars), deref)
    }

    #[must_use]
    pub const fn gc(&self) -> &GcAdmin {
        &self.gc
    }

    #[must_use]
    pub const fn true_term(&self) -> &Term {
        &self.true_term
    }

    #[must_use]
    pub const fn false_term(&self) -> &Term {
        &self.false_term
    }

    #[must_use]
    pub const fn garbage_state(&self) -> TermProperties {
        self.garbage_state
    }

    #[must_use]
    pub const fn non_var_term_nodes(&self) -> i64 {
        self.term_store.entries()
    }

    #[must_use]
    pub(crate) fn stored_terms(&self) -> Vec<Term> {
        self.term_store.terms()
    }

    /// Returns non-variable term nodes plus bank-owned variables.
    ///
    /// # Panics
    ///
    /// Panics if the term-cell store's maintained entry count differs from a
    /// full node count, matching the C consistency assertion in `TBTermNodes`.
    #[must_use]
    pub fn term_nodes(&self) -> i64 {
        assert_eq!(self.term_store.entries(), self.term_store.count_nodes());
        self.term_store.entries() + self.vars.cardinality()
    }

    /// Writes the C `TBPrintBankInOrder` DAG view, sorted by ascending
    /// `entry_no`.
    ///
    /// # Panics
    ///
    /// Panics if a non-constant term has an uninitialized argument.
    pub fn write_bank_in_order(&self, output: &mut impl fmt::Write) -> fmt::Result {
        let mut terms = self.term_store.terms();
        terms.sort_by_key(Term::entry_no);
        for term in terms {
            self.write_dag_term(output, &term)?;
            writeln!(output)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn bank_in_order_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_bank_in_order(&mut output);
        output
    }

    /// Writes a term either in conventional form or C compact bank form.
    ///
    /// # Panics
    ///
    /// Panics if a non-constant term has an uninitialized argument.
    pub fn write_term(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        full_terms: bool,
    ) -> fmt::Result {
        self.write_term_with_type_suffixes(output, term, full_terms, false)
    }

    /// Writes the C `TermPrint` shape with an explicit dereference mode.
    ///
    /// This follows the `TermPrint` macro dispatch: first-order problems use
    /// the conventional first-order surface, while higher-order problems use
    /// the currently ported `TermPrintHO` application surface. FOOL formula,
    /// `let`, and lambda pretty-printing remain part of the full
    /// formula-printer integration.
    ///
    /// # Panics
    ///
    /// Panics if a printed non-constant term has an uninitialized argument.
    pub fn write_term_deref_for_problem(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        problem_type: ProblemType,
        deref: DerefType,
    ) -> fmt::Result {
        if problem_type == ProblemType::HigherOrder {
            self.write_term_ho_deref(output, term, deref)
        } else {
            self.write_plain_term_deref(output, term, deref)
        }
    }

    /// Writes a term with optional `TermPrintTypes`-style suffixes.
    ///
    /// C appends type suffixes only on the conventional full-term printer; the
    /// compact DAG printer keeps its abbreviation-only shape.
    ///
    /// # Panics
    ///
    /// Panics if typed output is requested for a term without a known type, or
    /// if a non-constant term has an uninitialized argument, matching the C
    /// term-printer preconditions.
    pub fn write_term_with_type_suffixes(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        full_terms: bool,
        print_types: bool,
    ) -> fmt::Result {
        if full_terms {
            if print_types {
                self.write_plain_term_with_type_suffixes(output, term)
            } else {
                self.write_plain_term(output, term)
            }
        } else {
            self.write_term_compact(output, term)
        }
    }

    #[must_use]
    pub fn term_string(&self, term: &Term, full_terms: bool) -> String {
        let mut output = String::new();
        let _ = self.write_term(&mut output, term, full_terms);
        output
    }

    /// Writes the C `TermPrintDbg` shape for the selected problem type.
    ///
    /// # Panics
    ///
    /// Panics if a printed non-constant term has an uninitialized argument.
    pub fn write_term_debug(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        problem_type: ProblemType,
    ) -> fmt::Result {
        self.write_term_debug_deref(output, term, problem_type, DerefType::Never)
    }

    /// Writes the C `TermPrintDbg` shape with an explicit dereference mode.
    ///
    /// The higher-order path mirrors the LFHO no-WHNF `DEREF_LIMIT`/
    /// `CONVERT_DEREF` prefix rule for applied free variables. It deliberately
    /// does not populate the C `binding_cache`; global cache-backed
    /// dereferencing remains part of the termtypes/lambda integration slice.
    ///
    /// # Panics
    ///
    /// Panics if a printed non-constant term has an uninitialized argument.
    pub fn write_term_debug_deref(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        problem_type: ProblemType,
        deref: DerefType,
    ) -> fmt::Result {
        if problem_type == ProblemType::HigherOrder {
            self.write_ho_debug_term(output, term, deref)
        } else {
            self.write_plain_term_deref(output, term, deref)
        }
    }

    #[must_use]
    pub fn term_debug_string(&self, term: &Term, problem_type: ProblemType) -> String {
        let mut output = String::new();
        let _ = self.write_term_debug(&mut output, term, problem_type);
        output
    }

    #[must_use]
    pub fn term_debug_deref_string(
        &self,
        term: &Term,
        problem_type: ProblemType,
        deref: DerefType,
    ) -> String {
        let mut output = String::new();
        let _ = self.write_term_debug_deref(&mut output, term, problem_type, deref);
        output
    }

    /// Writes the C `TermPrintHO` application surface with an explicit deref mode.
    ///
    /// This covers the term-application part of `do_ho_print`: DB variables use
    /// C's `Z<depth-index-1>` spelling, phony applications print only visible
    /// arguments with ` @ ` separators, `$ite` uses its dedicated syntax, and
    /// LFHO applied free-variable dereferencing uses the same no-cache
    /// `DEREF_LIMIT`/`CONVERT_DEREF` prefix rule as the debug printer. FOOL
    /// formula and lambda pretty-printing remain deferred until the full
    /// formula printer is integrated.
    ///
    /// # Panics
    ///
    /// Panics if a printed non-constant term has an uninitialized argument.
    pub fn write_term_ho_deref(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        deref: DerefType,
    ) -> fmt::Result {
        self.write_ho_term(output, term, deref, 0)
    }

    #[must_use]
    pub fn term_ho_deref_string(&self, term: &Term, deref: DerefType) -> String {
        let mut output = String::new();
        let _ = self.write_term_ho_deref(&mut output, term, deref);
        output
    }

    /// Writes the C `TBPrintTermCompact` form and sets `TPOutputFlag` on
    /// printed non-variable bank terms.
    ///
    /// # Panics
    ///
    /// Panics if a non-constant term has an uninitialized argument.
    pub fn write_term_compact(&self, output: &mut impl fmt::Write, term: &Term) -> fmt::Result {
        if term.query_prop(TP_OUTPUT_FLAG) {
            return write!(output, "*{}", term.entry_no());
        }
        if term.is_free_var() {
            return write!(output, "{}", var_print_string(term.f_code()));
        }

        write!(output, "*{}:", term.entry_no())?;
        term.set_prop(TP_OUTPUT_FLAG);
        self.write_symbol(output, term.f_code())?;
        if !term.is_const() {
            self.write_compact_arg_list(output, term)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn term_compact_string(&self, term: &Term) -> String {
        let mut output = String::new();
        let _ = self.write_term_compact(&mut output, term);
        output
    }

    /// Writes the C `TBPrintBankTerms` view for terms marked `TPTopPos`.
    ///
    /// # Panics
    ///
    /// Panics if a printed non-constant term has an uninitialized argument.
    pub fn write_bank_terms(&self, output: &mut impl fmt::Write) -> fmt::Result {
        for term in self.term_store.terms() {
            if term.query_prop(TP_TOP_POS) {
                self.write_term_compact(output, &term)?;
                writeln!(output)?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn bank_terms_string(&self) -> String {
        let mut output = String::new();
        let _ = self.write_bank_terms(&mut output);
        output
    }

    fn write_dag_term(&self, output: &mut impl fmt::Write, term: &Term) -> fmt::Result {
        write!(output, "*{} : ", term.entry_no())?;
        if term.is_free_var() {
            return write!(output, "{}", var_print_string(term.f_code()));
        }

        self.write_symbol(output, term.f_code())?;
        if !term.is_const() {
            write!(output, "(")?;
            for index in 0..term.arity() {
                if index != 0 {
                    write!(output, ",")?;
                }
                let arg = term
                    .argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                write!(output, "*{}", tb_cell_ident(&arg))?;
            }
            write!(output, ")")?;
        }
        write!(output, "   =   ")?;
        self.write_plain_term(output, term)
    }

    fn write_plain_term(&self, output: &mut impl fmt::Write, term: &Term) -> fmt::Result {
        if term.is_free_var() {
            return write!(output, "{}", var_print_string(term.f_code()));
        }
        if term.is_db_var() {
            return write!(output, "db({})", term.f_code());
        }
        if self.should_print_cons_list(term) {
            return self.write_cons_list(output, term);
        }

        self.write_symbol(output, term.f_code())?;
        if !term.is_const() {
            self.write_plain_arg_list(output, term)?;
        }
        Ok(())
    }

    fn write_plain_term_deref(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        deref: DerefType,
    ) -> fmt::Result {
        let (term, current_deref, _) = Self::print_deref_root_no_whnf(term, deref);
        if term.is_free_var() {
            return write!(output, "{}", var_print_string(term.f_code()));
        }
        if term.is_db_var() {
            return write!(output, "db({})", term.f_code());
        }
        if self.should_print_cons_list(&term) {
            return self.write_cons_list_deref(output, &term, current_deref);
        }

        self.write_symbol(output, term.f_code())?;
        if !term.is_const() {
            self.write_plain_arg_list_deref(output, &term, current_deref)?;
        }
        Ok(())
    }

    fn write_plain_term_with_type_suffixes(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
    ) -> fmt::Result {
        if term.is_free_var() {
            write!(output, "{}", var_print_string(term.f_code()))?;
        } else if term.is_db_var() {
            write!(output, "db({})", term.f_code())?;
        } else if self.should_print_cons_list(term) {
            self.write_cons_list_with_type_suffixes(output, term)?;
        } else {
            self.write_symbol(output, term.f_code())?;
            if !term.is_const() {
                self.write_plain_arg_list_with_type_suffixes(output, term)?;
            }
        }
        self.write_type_suffix(output, term)
    }

    fn should_print_cons_list(&self, term: &Term) -> bool {
        self.sig.supports_lists()
            && (term.f_code() == SIG_NIL_CODE || term.f_code() == SIG_CONS_CODE)
    }

    fn write_cons_list(&self, output: &mut impl fmt::Write, term: &Term) -> fmt::Result {
        output.write_str("[")?;
        let mut list = term.clone();
        if list.f_code() == SIG_CONS_CODE {
            self.write_plain_term(output, &initialized_arg(&list, 0))?;
            list = initialized_arg(&list, 1);
            while list.f_code() == SIG_CONS_CODE {
                output.write_str(",")?;
                self.write_plain_term(output, &initialized_arg(&list, 0))?;
                list = initialized_arg(&list, 1);
            }
            assert_eq!(
                list.f_code(),
                SIG_NIL_CODE,
                "C list printing requires a proper $nil tail"
            );
        }
        output.write_str("]")
    }

    fn write_cons_list_deref(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        deref: DerefType,
    ) -> fmt::Result {
        output.write_str("[")?;
        let mut list = term.clone();
        if list.f_code() == SIG_CONS_CODE {
            self.write_plain_term_deref(output, &initialized_arg(&list, 0), deref)?;
            list = initialized_arg(&list, 1);
            while list.f_code() == SIG_CONS_CODE {
                output.write_str(",")?;
                self.write_plain_term_deref(output, &initialized_arg(&list, 0), deref)?;
                list = initialized_arg(&list, 1);
            }
            assert_eq!(
                list.f_code(),
                SIG_NIL_CODE,
                "C list printing requires a proper $nil tail"
            );
        }
        output.write_str("]")
    }

    fn write_cons_list_with_type_suffixes(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
    ) -> fmt::Result {
        output.write_str("[")?;
        let mut list = term.clone();
        if list.f_code() == SIG_CONS_CODE {
            self.write_plain_term_with_type_suffixes(output, &initialized_arg(&list, 0))?;
            list = initialized_arg(&list, 1);
            while list.f_code() == SIG_CONS_CODE {
                output.write_str(",")?;
                self.write_plain_term_with_type_suffixes(output, &initialized_arg(&list, 0))?;
                list = initialized_arg(&list, 1);
            }
            assert_eq!(
                list.f_code(),
                SIG_NIL_CODE,
                "C list printing requires a proper $nil tail"
            );
        }
        output.write_str("]")
    }

    fn write_plain_arg_list(&self, output: &mut impl fmt::Write, term: &Term) -> fmt::Result {
        write!(output, "(")?;
        for index in 0..term.arity() {
            if index != 0 {
                write!(output, ",")?;
            }
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            self.write_plain_term(output, &arg)?;
        }
        write!(output, ")")
    }

    fn write_plain_arg_list_deref(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        deref: DerefType,
    ) -> fmt::Result {
        write!(output, "(")?;
        for index in 0..term.arity() {
            if index != 0 {
                write!(output, ",")?;
            }
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            self.write_plain_term_deref(output, &arg, deref)?;
        }
        write!(output, ")")
    }

    fn write_plain_arg_list_with_type_suffixes(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
    ) -> fmt::Result {
        write!(output, "(")?;
        for index in 0..term.arity() {
            if index != 0 {
                write!(output, ",")?;
            }
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            self.write_plain_term_with_type_suffixes(output, &arg)?;
        }
        write!(output, ")")
    }

    fn write_type_suffix(&self, output: &mut impl fmt::Write, term: &Term) -> fmt::Result {
        let type_ = term
            .type_()
            .expect("typed term printing requires a known term type");
        let mut rendered = Vec::new();
        self.sig
            .type_bank()
            .print_tstp(&mut rendered, &type_, problem_type())
            .map_err(|_| fmt::Error)?;
        let rendered = String::from_utf8(rendered).map_err(|_| fmt::Error)?;
        output.write_char(':')?;
        output.write_str(&rendered)
    }

    fn write_ho_term(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        deref: DerefType,
        depth: i64,
    ) -> fmt::Result {
        let (term, current_deref, limit) = Self::print_deref_root_no_whnf(term, deref);
        if term.f_code() == SIG_ITE_CODE {
            assert_eq!(term.arity(), 3, "$ite expects three arguments");
            output.write_str("$ite(")?;
            self.write_ho_term(output, &initialized_arg(&term, 0), current_deref, depth)?;
            output.write_str(", ")?;
            self.write_ho_term(output, &initialized_arg(&term, 1), current_deref, depth)?;
            output.write_str(", ")?;
            self.write_ho_term(output, &initialized_arg(&term, 2), current_deref, depth)?;
            output.write_char(')')?;
            return Ok(());
        }

        if term.is_db_var() {
            write!(output, "Z{}", depth - term.f_code() - 1)?;
        } else if !term.is_top_level_any_var() {
            if term.is_phony_app() {
                let head = initialized_arg(&term, 0);
                if head.is_lambda() {
                    output.write_str("( ")?;
                }
                self.write_ho_term(output, &head, current_deref, depth)?;
                if head.is_lambda() {
                    output.write_str(" )")?;
                }
            } else {
                self.write_symbol(output, term.f_code())?;
            }
        } else {
            let var = if term.is_any_var() {
                term.clone()
            } else {
                initialized_arg(&term, 0)
            };
            if var.is_free_var() {
                write!(output, "{}", var_print_string(var.f_code()))?;
            } else {
                write!(output, "Z{}", depth - var.f_code() - 1)?;
            }
        }

        let first_visible_arg = usize::from(term.is_phony_app());
        for index in first_visible_arg..term.arity() {
            output.write_str(" @ ")?;
            let arg = initialized_arg(&term, index);
            let child_deref = Self::convert_lfho_deref(index, limit, current_deref);
            if arg.arity() != 0
                || (child_deref != DerefType::Never
                    && arg.binding().is_some_and(|binding| binding.arity() != 0))
            {
                output.write_char('(')?;
                self.write_ho_term(output, &arg, child_deref, depth)?;
                output.write_char(')')?;
            } else {
                self.write_ho_term(output, &arg, child_deref, depth)?;
            }
        }
        Ok(())
    }

    fn write_ho_debug_term(
        &self,
        output: &mut impl fmt::Write,
        term: &Term,
        deref: DerefType,
    ) -> fmt::Result {
        let (term, current_deref, limit) = Self::print_deref_root_no_whnf(term, deref);
        if term.is_db_var() {
            write!(output, "db({})", term.f_code())?;
        } else if term.is_free_var() {
            write!(output, "{}", var_print_string(term.f_code()))?;
        } else {
            self.write_symbol(output, term.f_code())?;
        }

        for index in 0..term.arity() {
            output.write_char(' ')?;
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let child_deref = Self::convert_lfho_deref(index, limit, current_deref);
            if arg.arity() != 0
                || (child_deref != DerefType::Never
                    && arg.binding().is_some_and(|binding| binding.arity() != 0))
            {
                output.write_char('(')?;
                self.write_ho_debug_term(output, &arg, child_deref)?;
                output.write_char(')')?;
            } else {
                self.write_ho_debug_term(output, &arg, child_deref)?;
            }
        }
        Ok(())
    }

    fn print_deref_root_no_whnf(term: &Term, deref: DerefType) -> (Term, DerefType, usize) {
        let limit = Self::deref_limit(term, deref);
        match deref {
            DerefType::Never => (term.clone(), deref, limit),
            DerefType::Always => {
                let mut current = term.clone();
                while let Some(next) = Self::print_deref_step_no_whnf(&current) {
                    current = next;
                }
                (current, deref, limit)
            }
            DerefType::Once => {
                let mut current = term.clone();
                let mut current_deref = deref;
                let originally_app_var = current.is_applied_free_var();
                if let Some(next) = Self::print_deref_step_no_whnf(&current) {
                    current = next;
                    if !originally_app_var {
                        current_deref = DerefType::Never;
                    }
                }
                (current, current_deref, limit)
            }
        }
    }

    fn print_deref_step_no_whnf(term: &Term) -> Option<Term> {
        if term.is_free_var() {
            return term.binding();
        }
        if term.is_applied_free_var()
            && term
                .argument(0)
                .is_some_and(|head| head.binding().is_some())
        {
            return Some(Self::print_deref_applied_free_var_once(term));
        }
        None
    }

    fn print_deref_applied_free_var_once(term: &Term) -> Term {
        assert!(term.is_applied_free_var(), "expected applied free variable");
        assert!(term.arity() > 1, "applied variable must have arguments");
        let head = term.argument(0).expect("applied variable has a head");
        let binding = head.binding().expect("applied variable head is bound");

        let expanded = if binding.is_any_var() || binding.is_lambda() {
            let expanded = Term::top_alloc(term.f_code(), term.arity());
            expanded.set_properties(term.give_props(TP_PRED_POS));
            expanded.set_type(term.type_());
            expanded.set_argument(0, binding);
            for index in 1..term.arity() {
                let arg = term
                    .argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                expanded.set_argument(index, arg);
            }
            expanded
        } else {
            let expanded = Term::top_alloc(binding.f_code(), binding.arity() + term.arity() - 1);
            expanded.set_properties(binding.give_props(TP_PRED_POS));
            expanded.set_type(term.type_());
            for index in 0..binding.arity() {
                let arg = binding
                    .argument(index)
                    .unwrap_or_else(|| panic!("binding argument {index} is uninitialized"));
                expanded.set_argument(index, arg);
            }
            for index in 1..term.arity() {
                let arg = term
                    .argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                expanded.set_argument(binding.arity() + index - 1, arg);
            }
            expanded
        };

        expanded
    }

    fn write_compact_arg_list(&self, output: &mut impl fmt::Write, term: &Term) -> fmt::Result {
        write!(output, "(")?;
        for index in 0..term.arity() {
            if index != 0 {
                write!(output, ",")?;
            }
            let arg = term
                .argument(index)
                .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            self.write_term_compact(output, &arg)?;
        }
        write!(output, ")")
    }

    fn write_symbol(&self, output: &mut impl fmt::Write, f_code: FunCode) -> fmt::Result {
        write!(
            output,
            "{}",
            self.sig.find_name(f_code).unwrap_or("<unknown>")
        )
    }

    /// Inserts a term, recursively sharing all non-variable subterms.
    ///
    /// # Panics
    ///
    /// Panics if a free or DB variable has no type, matching the C
    /// precondition for `TBInsert`.
    pub fn insert(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        self.insert_with_mode(term, deref, InsertMode::ShareVariables)
    }

    /// Inserts a term without replacing free variables by bank-owned variables.
    ///
    /// # Panics
    ///
    /// Panics if a DB variable has no type, matching the C precondition.
    pub fn insert_ignore_var(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        self.insert_with_mode(term, deref, InsertMode::KeepVariables)
    }

    /// Inserts a term after clearing copied top-cell properties.
    ///
    /// # Panics
    ///
    /// Panics if a free or DB variable has no type, matching the C
    /// precondition for `TBInsertNoProps`.
    pub fn insert_no_props(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        self.insert_with_mode(term, deref, InsertMode::NoProperties)
    }

    /// Inserts without properties, using a term-identity cache for large terms.
    ///
    /// # Panics
    ///
    /// Panics if a free/DB variable has no type, matching the C preconditions
    /// for `TBInsertNoPropsCached`.
    pub fn insert_no_props_cached(
        &mut self,
        term: &Term,
        deref: DerefType,
    ) -> Result<Term, Diagnostic> {
        if term.f_count() > INSERT_NO_PROPS_CACHE_THRESHOLD {
            let mut cache = BTreeMap::new();
            self.insert_no_props_cached_inner(term, deref, &mut cache)
        } else {
            self.insert_no_props(term, deref)
        }
    }

    /// Inserts a term while reusing shared ground subterms unchanged.
    ///
    /// # Panics
    ///
    /// Panics if a ground term is not shared, or if a free/DB variable has no
    /// type, matching the C preconditions for `TBInsertOpt`.
    pub fn insert_opt(&mut self, term: &Term, deref: DerefType) -> Result<Term, Diagnostic> {
        let (term, current_deref, limit) = self.deref_root_no_whnf(term, deref)?;
        if term_is_ground_for_insert(&term) {
            assert!(
                term.is_shared(),
                "optimized ground insertion expects sharing"
            );
            return Ok(term);
        }
        self.insert_opt_derefed(&term, current_deref, limit)
    }

    /// Inserts a term with one subterm replaced after instantiation.
    ///
    /// # Panics
    ///
    /// Panics if `repl` is not already present in this bank, or if a free/DB
    /// variable has no type, matching `TBInsertRepl`.
    pub fn insert_repl(
        &mut self,
        term: &Term,
        deref: DerefType,
        old: &Term,
        repl: &Term,
    ) -> Result<Term, Diagnostic> {
        if term == old {
            assert!(
                self.find(repl).is_some(),
                "replacement must already be in the term bank"
            );
            return Ok(repl.clone());
        }

        let (term, current_deref, limit) = self.deref_root_no_whnf(term, deref)?;
        if term.is_free_var() {
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(&term);
        copy.set_properties(TP_IGNORE_PROPS);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_repl(
                &arg,
                Self::convert_lfho_deref(index, limit, current_deref),
                old,
                repl,
            )?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    /// Inserts a plain, non-instantiated replacement when a subterm changes.
    ///
    /// # Panics
    ///
    /// Panics if `repl` is not already present in this bank, matching
    /// `TBInsertReplPlain`.
    pub fn insert_repl_plain(
        &mut self,
        term: &Term,
        old: &Term,
        repl: &Term,
    ) -> Result<Term, Diagnostic> {
        if term == old {
            assert!(
                self.find(repl).is_some(),
                "replacement must already be in the term bank"
            );
            return Ok(repl.clone());
        }
        if term_standard_weight(term) <= term_standard_weight(old) || term.is_any_var() {
            return Ok(term.clone());
        }

        let copy = Term::top_copy_without_args(term);
        copy.set_properties(TP_IGNORE_PROPS);
        let mut changed = false;
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let replaced = self.insert_repl_plain(&arg, old, repl)?;
            if replaced != arg {
                changed = true;
            }
            copy.set_argument(index, replaced);
        }

        if changed {
            self.term_top_insert(copy)
        } else {
            Ok(term.clone())
        }
    }

    /// Inserts an instantiated term while preserving already-shared ground terms.
    ///
    /// # Panics
    ///
    /// Panics if `deref` exposes a free/DB variable without a type, or if an
    /// inserted parent receives an unshared ground child, matching the C
    /// caller preconditions for `TBInsertInstantiatedDeref`.
    pub fn insert_instantiated_deref(
        &mut self,
        term: &Term,
        deref: DerefType,
    ) -> Result<Term, Diagnostic> {
        if deref == DerefType::Never {
            return Ok(term.clone());
        }

        let (term, current_deref, limit) = self.deref_root_no_whnf(term, deref)?;
        if term.is_any_var() || term_is_ground_for_insert(&term) {
            return Ok(term);
        }

        let copy = Term::top_copy_without_args(&term);
        copy.set_properties(TP_IGNORE_PROPS);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_instantiated_deref(
                &arg,
                Self::convert_lfho_deref(index, limit, current_deref),
            )?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    fn deref_root_no_whnf(
        &mut self,
        term: &Term,
        deref: DerefType,
    ) -> Result<(Term, DerefType, usize), Diagnostic> {
        let limit = Self::deref_limit(term, deref);
        if deref == DerefType::Once
            && term.is_applied_free_var()
            && term
                .argument(0)
                .is_some_and(|head| head.binding().is_some())
        {
            return Ok((self.deref_applied_free_var_once(term)?, deref, limit));
        }

        let mut current_deref = deref;
        let term = term_deref(term, &mut current_deref);
        Ok((term, current_deref, limit))
    }

    fn deref_limit(term: &Term, deref: DerefType) -> usize {
        if deref == DerefType::Once
            && term.is_applied_free_var()
            && term
                .argument(0)
                .is_some_and(|head| head.binding().is_some())
        {
            let binding = term
                .argument(0)
                .and_then(|head| head.binding())
                .expect("bound applied free variable has a binding");
            Self::applied_binding_ignore_args(&binding)
        } else {
            0
        }
    }

    fn convert_lfho_deref(index: usize, limit: usize, deref: DerefType) -> DerefType {
        if deref == DerefType::Once && index < limit {
            DerefType::Never
        } else {
            deref
        }
    }

    /// Inserts a first-order instantiated term.
    ///
    /// # Panics
    ///
    /// Panics if a ground or bound term is not already present in the bank, or
    /// if a free/DB variable has no type, matching `TBInsertInstantiatedFO`.
    pub fn insert_instantiated_fo(&mut self, term: &Term) -> Result<Term, Diagnostic> {
        if term_is_ground_for_insert(term) {
            assert!(
                self.find(term).is_some(),
                "ground instantiated terms must already be in the bank"
            );
            return Ok(term.clone());
        }
        if let Some(binding) = term.binding() {
            assert!(
                self.find(&binding).is_some(),
                "variable binding must already be in the bank"
            );
            return Ok(binding);
        }
        if term.is_free_var() {
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(term);
        copy.set_properties(TP_IGNORE_PROPS);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_instantiated_fo(&arg)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    pub fn insert_instantiated(&mut self, term: &Term) -> Result<Term, Diagnostic> {
        self.insert_instantiated_for_problem(term, problem_type())
    }

    pub fn insert_instantiated_for_problem(
        &mut self,
        term: &Term,
        problem_type: ProblemType,
    ) -> Result<Term, Diagnostic> {
        if problem_type == ProblemType::HigherOrder {
            self.insert_instantiated_ho(term, true)
        } else {
            self.insert_instantiated_fo(term)
        }
    }

    /// Inserts a higher-order instantiated term, sharing variable bindings.
    ///
    /// Applied free-variable bindings are expanded once for this insertion path,
    /// but the LFHO `binding_cache`/owner-bank fields are still deferred.
    ///
    /// # Panics
    ///
    /// Panics if a free/DB variable has no type or an argument slot is
    /// uninitialized, matching the C preconditions for
    /// `TBInsertInstantiatedHO`.
    pub fn insert_instantiated_ho(
        &mut self,
        term: &Term,
        follow_bind: bool,
    ) -> Result<Term, Diagnostic> {
        if term_is_ground_for_insert(term) && term.is_shared() {
            return Ok(term.clone());
        }

        if term.is_free_var() {
            if let Some(binding) = term.binding() {
                return if follow_bind {
                    self.insert(&binding, DerefType::Never)
                } else {
                    Ok(term.clone())
                };
            }
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let (term, ignore_args) = if term.is_applied_free_var() && follow_bind {
            let head = term.argument(0).expect("applied variable has a head");
            if let Some(binding) = head.binding() {
                let ignore_args = Self::applied_binding_ignore_args(&binding);
                (self.deref_applied_free_var_once(term)?, ignore_args)
            } else {
                (term.clone(), 0)
            }
        } else {
            (term.clone(), 0)
        };
        if term_is_ground_for_insert(&term) && term.is_shared() {
            return Ok(term);
        }

        let copy = Term::top_copy_without_args(&term);
        copy.set_properties(TP_IGNORE_PROPS);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_instantiated_ho(&arg, follow_bind && index >= ignore_args)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    fn applied_binding_ignore_args(binding: &Term) -> usize {
        if binding.is_lambda() {
            1
        } else {
            binding.arity() + usize::from(binding.is_free_var())
        }
    }

    /// Expands an applied free variable once, like LFHO `applied_var_deref`.
    ///
    /// This deliberately does not populate the C `binding_cache`; the expanded
    /// result is shared immediately through this bank.
    ///
    /// # Panics
    ///
    /// Panics if `term` is not an applied free variable with a bound head.
    pub fn deref_applied_free_var_once(&mut self, term: &Term) -> Result<Term, Diagnostic> {
        assert!(term.is_applied_free_var(), "expected applied free variable");
        assert!(term.arity() > 1, "applied variable must have arguments");
        let head = term.argument(0).expect("applied variable has a head");
        let binding = head.binding().expect("applied variable head is bound");

        let expanded = if binding.is_any_var() || binding.is_lambda() {
            let expanded = Term::top_alloc(term.f_code(), term.arity());
            expanded.set_properties(term.give_props(TP_PRED_POS));
            expanded.set_type(term.type_());
            expanded.set_argument(0, binding);
            for index in 1..term.arity() {
                let arg = term
                    .argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                expanded.set_argument(index, arg);
            }
            expanded
        } else {
            let expanded = Term::top_alloc(binding.f_code(), binding.arity() + term.arity() - 1);
            expanded.set_properties(binding.give_props(TP_PRED_POS));
            expanded.set_type(term.type_());
            for index in 0..binding.arity() {
                let arg = binding
                    .argument(index)
                    .unwrap_or_else(|| panic!("binding argument {index} is uninitialized"));
                expanded.set_argument(index, arg);
            }
            for index in 1..term.arity() {
                let arg = term
                    .argument(index)
                    .unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                expanded.set_argument(binding.arity() + index - 1, arg);
            }
            expanded
        };

        for (index, arg) in expanded.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("expanded argument {index} is uninitialized"));
            if !arg.is_free_var() && !arg.is_shared() {
                expanded.set_argument(index, self.insert_ignore_var(&arg, DerefType::Never)?);
            }
        }
        self.term_top_insert(expanded)
    }

    /// Inserts a copy whose free variables are replaced by alternative vars.
    ///
    /// # Panics
    ///
    /// Panics if an input free variable is already an alternative variable, or
    /// if a free/DB variable has no type, matching the C `TBInsertDisjoint`
    /// preconditions.
    pub fn insert_disjoint(&mut self, term: &Term) -> Result<Term, Diagnostic> {
        if term_is_ground_for_insert(term) {
            return Ok(term.clone());
        }
        if term.is_free_var() {
            return Ok(self.vars.get_alt_var(term));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(term);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_disjoint(&arg)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    /// Inserts a top cell whose arguments are already shared.
    ///
    /// # Panics
    ///
    /// Panics if `term` is a variable, lambda/application invariants are
    /// violated, or a top-cell child is neither shared nor a free variable.
    pub fn term_top_insert(&mut self, term: Term) -> Result<Term, Diagnostic> {
        assert!(!term.is_any_var(), "top insertion expects a non-variable");
        assert!(
            term.f_code() != SIG_NAMED_LAMBDA_CODE
                || (term.arity() == 2 && term.argument(0).is_some_and(|arg| arg.is_free_var())),
            "named lambda top cell has invalid binder shape"
        );
        assert!(
            term.f_code() != SIG_DB_LAMBDA_CODE
                || (term.arity() == 2 && term.argument(0).is_some_and(|arg| arg.is_db_var())),
            "DB lambda top cell has invalid binder shape"
        );
        assert!(
            !term.is_phony_app()
                || term
                    .argument(0)
                    .is_some_and(|arg| arg.is_any_var() || arg.is_lambda()),
            "phony application must apply a variable or lambda"
        );
        assert!(
            !term.is_phony_app() || term.arity() > 1,
            "phony application needs at least one argument"
        );
        for arg in term.argument_clones().into_iter().flatten() {
            assert!(
                arg.is_shared() || arg.is_free_var(),
                "term-bank top insertion requires shared arguments"
            );
        }

        if term.type_().is_none() {
            type_infer_sort(&mut self.sig, &term)?;
            assert!(term.type_().is_some(), "type inference assigned a sort");
        }

        self.insertions += 1;
        if let Some(existing) = self.term_store.insert(term.clone()) {
            existing.set_prop(term.properties());
            return Ok(existing);
        }

        self.in_count += 1;
        term.set_entry_no(i64::try_from(self.in_count).unwrap_or(i64::MAX));
        term.assign_prop(TP_GARBAGE_FLAG, self.garbage_state);
        term.set_prop(TP_IS_SHARED);
        self.set_top_insert_metadata(&term);
        assert_eq!(self.find(&term), Some(term.clone()));
        Ok(term)
    }

    /// Finds a term representation in this bank.
    ///
    /// # Panics
    ///
    /// Panics if a DB variable has no type, matching `TBFind`/`TBRequestDBVar`.
    pub fn find(&mut self, term: &Term) -> Option<Term> {
        if term.is_free_var() {
            self.vars.f_code_find(term.f_code())
        } else if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            Some(self.db_vars.request_db_var(&type_, term.f_code()))
        } else {
            self.term_store.find(term)
        }
    }

    pub fn create_const_term(&mut self, f_code: FunCode) -> Result<Term, Diagnostic> {
        let term = Term::const_cell_alloc(f_code);
        self.insert(&term, DerefType::Never)
    }

    pub fn parse_term_simple(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        self.parse_term_simple_with_distinct_checks(scanner, false)
    }

    pub fn parse_term_with_distinct_checks(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Term, Diagnostic> {
        self.parse_term_real(scanner, true)
    }

    /// Parses a TSTP term-encoded formula.
    ///
    /// This exposes the existing C `TFormulaTSTPParse` port used internally by
    /// Boolean term-argument parsing.
    pub fn parse_tformula_tstp(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        self.parse_tformula_tstp_subset(scanner)
    }

    /// Parses an old-TPTP term-encoded formula.
    ///
    /// This matches C `TFormulaTPTPParse`: every binary operator has the same
    /// precedence and is parsed right-recursively.
    pub fn parse_tformula_tptp(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        self.parse_tformula_tptp_subset(scanner)
    }

    /// Parses a `$distinct(...)` pseudo-formula in TSTP syntax.
    ///
    /// This matches C `TSTPDistinctParse`: every argument is parsed as a
    /// zero-arity function symbol, including variable-looking identifiers, and
    /// all arguments must have the same inferred type.
    pub fn parse_tstp_distinct(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        scanner.accept_id("$distinct")?;
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;

        let first = self.parse_constant_term(scanner)?;
        let expected_type = first.type_().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                "$distinct first argument has no inferred type",
            )
        })?;
        let mut args = vec![first];

        while scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            let position = token_pos_rep(scanner.current_token());
            let arg = self.parse_constant_term(scanner)?;
            if arg.type_().as_ref() != Some(&expected_type) {
                return Err(Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    format!(
                        "{position} All $distinct arguments have to be constants of the same type"
                    ),
                ));
            }
            args.push(arg);
        }
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

        let distinct_code = Self::require_formula_op_code(self.sig.distinct_code())?;
        let distinct = Term::top_alloc(distinct_code, args.len());
        for (index, arg) in args.into_iter().enumerate() {
            distinct.set_argument(index, arg);
        }
        self.term_top_insert(distinct)
    }

    fn parse_constant_term(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        let position = token_pos_rep(scanner.current_token());
        let mut id = DynamicString::new();
        let id_type = func_symb_parse(scanner, &mut id)?;
        let name = id.view().into_owned();
        if scanner.test_tok(TokenType::OPEN_BRACKET) {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!("{position} constant expected in $distinct argument list"),
            ));
        }

        let f_code = term_sig_insert(&mut self.sig, &name, 0, false, id_type);
        if f_code == 0 {
            let registered_code = self.sig.find_f_code(&name);
            let registered_arity = self.sig.find_arity(registered_code).unwrap_or(0);
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{position} constant expected but {name} registered with arity {registered_arity}"
                ),
            ));
        }
        self.create_const_term(f_code)
    }

    fn parse_term_real(
        &mut self,
        scanner: &mut Scanner,
        check_symbol_properties: bool,
    ) -> Result<Term, Diagnostic> {
        if self.sig.supports_lists() && scanner.test_tok(TokenType::OPEN_SQUARE) {
            return self.parse_cons_list_real(scanner, check_symbol_properties);
        }

        if scanner.test_tok(TokenType::ITE_TOKEN) {
            return self.parse_ite_tformula_tstp_subset(scanner);
        }
        if scanner.test_tok(TokenType::LET_TOKEN) {
            return self.parse_let_tformula_tstp_subset(scanner);
        }

        let mut id = DynamicString::new();
        let id_type = term_parse_operator(scanner, &mut id)?;
        let name = id.view().into_owned();
        if id_type == FuncSymbType::IdentVar {
            if scanner.test_tok(TokenType::COLON) {
                scanner.accept_tok(TokenType::COLON)?;
                let type_ = self
                    .sig
                    .type_bank_mut()
                    .parse_type_from_current_problem(scanner)?;
                return Ok(self.vars.ext_name_assert_alloc_sort(&name, &type_));
            }
            return Ok(self.vars.ext_name_assert_alloc(&name));
        }

        if scanner.test_tok(TokenType::OPEN_BRACKET) && check_symbol_properties {
            reject_term_bank_distinct_argument_list(&self.sig, id_type)?;
        }
        let existing_code = self.sig.find_f_code(&name);
        let symbol_type = if existing_code == 0 {
            None
        } else {
            self.sig.get_type(existing_code).cloned()
        };
        let args =
            self.parse_real_arg_list_opt(scanner, check_symbol_properties, symbol_type.as_ref())?;
        let arity = i32::try_from(args.len()).map_err(|_| {
            Diagnostic::new(
                ErrorCode::RESOURCE_OUT,
                "Term arity is too large for C-compatible signatures",
            )
        })?;
        let f_code = term_sig_insert(&mut self.sig, &name, arity, false, id_type);
        if f_code == 0 {
            return Err(Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                format!("{name} used with incompatible arity {arity}"),
            ));
        }

        let term = Term::top_alloc(f_code, args.len());
        for (index, arg) in args.into_iter().enumerate() {
            term.set_argument(index, arg);
        }
        self.term_top_insert(term)
    }

    fn parse_term_simple_with_distinct_checks(
        &mut self,
        scanner: &mut Scanner,
        check_distinct_argument_lists: bool,
    ) -> Result<Term, Diagnostic> {
        if self.sig.supports_lists() && scanner.test_tok(TokenType::OPEN_SQUARE) {
            return self.parse_cons_list(scanner, check_distinct_argument_lists);
        }

        let mut id = DynamicString::new();
        let id_type = term_parse_operator(scanner, &mut id)?;
        let name = id.view().into_owned();
        if id_type == FuncSymbType::IdentVar {
            if scanner.test_tok(TokenType::COLON) {
                scanner.accept_tok(TokenType::COLON)?;
                let type_ = self
                    .sig
                    .type_bank_mut()
                    .parse_type_from_current_problem(scanner)?;
                return Ok(self.vars.ext_name_assert_alloc_sort(&name, &type_));
            }
            return Ok(self.vars.ext_name_assert_alloc(&name));
        }

        if scanner.test_tok(TokenType::OPEN_BRACKET) && check_distinct_argument_lists {
            reject_term_bank_distinct_argument_list(&self.sig, id_type)?;
        }
        let args = self.parse_simple_arg_list_opt(scanner, check_distinct_argument_lists)?;
        let arity = i32::try_from(args.len()).map_err(|_| {
            Diagnostic::new(
                ErrorCode::RESOURCE_OUT,
                "Term arity is too large for C-compatible signatures",
            )
        })?;
        let f_code = term_sig_insert(&mut self.sig, &name, arity, false, id_type);
        if f_code == 0 {
            return Err(Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                format!("{name} used with incompatible arity {arity}"),
            ));
        }

        let term = Term::top_alloc(f_code, args.len());
        for (index, arg) in args.into_iter().enumerate() {
            term.set_argument(index, arg);
        }
        self.term_top_insert(term)
    }

    fn parse_cons_list(
        &mut self,
        scanner: &mut Scanner,
        check_distinct_argument_lists: bool,
    ) -> Result<Term, Diagnostic> {
        scanner.accept_tok(TokenType::OPEN_SQUARE)?;
        let mut elements = Vec::new();
        if !scanner.test_tok(TokenType::CLOSE_SQUARE) {
            elements.push(
                self.parse_term_simple_with_distinct_checks(
                    scanner,
                    check_distinct_argument_lists,
                )?,
            );
            while scanner.test_tok(TokenType::COMMA) {
                scanner.accept_tok(TokenType::COMMA)?;
                elements.push(self.parse_term_simple_with_distinct_checks(
                    scanner,
                    check_distinct_argument_lists,
                )?);
            }
        }
        scanner.accept_tok(TokenType::CLOSE_SQUARE)?;

        let mut list = self.create_const_term(SIG_NIL_CODE)?;
        for element in elements.into_iter().rev() {
            let cons = Term::top_alloc(SIG_CONS_CODE, 2);
            cons.set_argument(0, element);
            cons.set_argument(1, list);
            list = self.term_top_insert(cons)?;
        }
        Ok(list)
    }

    fn parse_cons_list_real(
        &mut self,
        scanner: &mut Scanner,
        check_symbol_properties: bool,
    ) -> Result<Term, Diagnostic> {
        scanner.accept_tok(TokenType::OPEN_SQUARE)?;
        let mut elements = Vec::new();
        if !scanner.test_tok(TokenType::CLOSE_SQUARE) {
            elements.push(self.parse_term_real(scanner, check_symbol_properties)?);
            while scanner.test_tok(TokenType::COMMA) {
                scanner.accept_tok(TokenType::COMMA)?;
                elements.push(self.parse_term_real(scanner, check_symbol_properties)?);
            }
        }
        scanner.accept_tok(TokenType::CLOSE_SQUARE)?;

        let mut list = self.create_const_term(SIG_NIL_CODE)?;
        for element in elements.into_iter().rev() {
            let cons = Term::top_alloc(SIG_CONS_CODE, 2);
            cons.set_argument(0, element);
            cons.set_argument(1, list);
            list = self.term_top_insert(cons)?;
        }
        Ok(list)
    }

    fn parse_real_arg_list_opt(
        &mut self,
        scanner: &mut Scanner,
        check_symbol_properties: bool,
        type_: Option<&Type>,
    ) -> Result<Vec<Term>, Diagnostic> {
        if !scanner.test_tok(TokenType::OPEN_BRACKET) {
            return Ok(Vec::new());
        }

        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        if scanner.test_tok(TokenType::CLOSE_BRACKET) {
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            return Ok(Vec::new());
        }

        let mut args = vec![self.parse_real_arg(scanner, check_symbol_properties, type_, 0)?];
        while scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            let index = args.len();
            args.push(self.parse_real_arg(scanner, check_symbol_properties, type_, index)?);
        }
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        Ok(args)
    }

    fn parse_real_arg(
        &mut self,
        scanner: &mut Scanner,
        check_symbol_properties: bool,
        type_: Option<&Type>,
        index: usize,
    ) -> Result<Term, Diagnostic> {
        if type_
            .is_some_and(|type_| index < type_get_max_arity(type_) && type_.args()[index].is_bool())
        {
            let formula = self.parse_tformula_tstp_subset(scanner)?;
            return Ok(self.normalize_boolean_term_arg(formula));
        }

        let term = if check_symbol_properties {
            self.parse_subterm(scanner)?
        } else {
            self.parse_term_real(scanner, true)?
        };
        Ok(self.normalize_boolean_term_arg(term))
    }

    fn parse_subterm(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        let term = self.parse_term_real(scanner, true)?;
        if !term.is_free_var() {
            if self.sig.is_predicate(term.f_code()) {
                if self.sig.is_fixed_type(term.f_code()) {
                    return Err(Diagnostic::new(
                        ErrorCode::SYNTAX_ERROR,
                        "Predicate used as function symbol in preceding term",
                    ));
                }
                self.sig.declare_is_function(term.f_code())?;
                type_infer_sort(&mut self.sig, &term)?;
                assert!(term.type_().is_some(), "type inference assigned a sort");
            } else {
                self.sig.fix_type(term.f_code());
            }
        }
        Ok(term)
    }

    fn parse_tformula_tstp_subset(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        let mut formula = self.parse_tformula_tstp_disjunction(scanner)?;
        if scanner.test_tok(TokenType::FOF_BIN_OP) && !scanner.test_tok(TokenType::FOF_ASSOC_OP) {
            let mut op = self.tptp_operator_parse(scanner)?;
            let right = if (op == self.sig.eqn_code() || op == self.sig.neqn_code())
                && !formula.type_().as_ref().is_some_and(Type::is_bool)
            {
                self.parse_literal_tformula_tstp_with_applications(scanner)?
            } else {
                self.parse_tformula_tstp_disjunction(scanner)?
            };
            if formula.type_().as_ref().is_some_and(Type::is_bool)
                && (op == self.sig.eqn_code() || op == self.sig.neqn_code())
            {
                if !right.type_().as_ref().is_some_and(Type::is_bool) {
                    return Err(Diagnostic::new(
                        ErrorCode::TYPE_ERROR,
                        "Boolean formula equality requires Boolean right operand",
                    ));
                }
                op = if op == self.sig.eqn_code() {
                    self.sig.equiv_code()
                } else {
                    self.sig.xor_code()
                };
            }
            formula = self.tformula_fcode_alloc(op, formula, Some(right))?;
        }
        Ok(formula)
    }

    fn parse_tformula_tstp_disjunction(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Term, Diagnostic> {
        let mut formula = self.parse_tformula_tstp_conjunction(scanner)?;
        while scanner.test_tok(TokenType::FOF_OR) {
            let op = self.tptp_operator_parse(scanner)?;
            let right = self.parse_tformula_tstp_conjunction(scanner)?;
            formula = self.tformula_fcode_alloc(op, formula, Some(right))?;
        }
        Ok(formula)
    }

    fn parse_tformula_tstp_conjunction(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Term, Diagnostic> {
        let mut formula = self.parse_literal_tformula_tstp_with_applications(scanner)?;
        while scanner.test_tok(TokenType::FOF_AND) {
            let op = self.tptp_operator_parse(scanner)?;
            let right = self.parse_literal_tformula_tstp_with_applications(scanner)?;
            formula = self.tformula_fcode_alloc(op, formula, Some(right))?;
        }
        Ok(formula)
    }

    fn parse_literal_tformula_tstp_with_applications(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Term, Diagnostic> {
        let mut formula = self.parse_literal_tformula_tstp_subset(scanner)?;
        if scanner.test_tok(TokenType::APPLICATION) {
            formula = self.parse_applied_tformula_tstp_subset(scanner, &formula)?;
        }
        Ok(formula)
    }

    fn parse_tformula_tptp_subset(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        let left = self.parse_elem_tformula_tptp(scanner)?;
        if scanner.test_tok(TokenType::FOF_BIN_OP) {
            let op = self.tptp_operator_parse(scanner)?;
            let right = self.parse_tformula_tptp_subset(scanner)?;
            self.tformula_fcode_alloc(op, left, Some(right))
        } else {
            Ok(left)
        }
    }

    fn parse_elem_tformula_tptp(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        if scanner.test_tok(TokenType::UNIV_QUANTOR | TokenType::EXIST_QUANTOR) {
            let quantor = self.tptp_quantor_parse(scanner)?;
            scanner.accept_tok(TokenType::OPEN_SQUARE)?;
            self.parse_quantified_tformula_tptp(scanner, quantor)
        } else if scanner.test_tok(TokenType::OPEN_BRACKET) {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            let formula = self.parse_tformula_tptp_subset(scanner)?;
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            Ok(formula)
        } else if scanner.test_tok(TokenType::TILDE_SIGN) {
            scanner.accept_tok(TokenType::TILDE_SIGN)?;
            let child = self.parse_elem_tformula_tptp(scanner)?;
            self.tformula_fcode_alloc(
                Self::require_formula_op_code(self.sig.not_code())?,
                child,
                None,
            )
        } else {
            self.parse_tformula_tptp_atom(scanner)
        }
    }

    fn parse_tformula_tptp_atom(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        let left = self.parse_term_real(scanner, true)?;
        let mut positive = true;
        if scanner.test_tok(TokenType::NEG_EQUAL_SIGN | TokenType::EQUAL_SIGN) {
            if scanner.test_tok(TokenType::NEG_EQUAL_SIGN) {
                positive = false;
            }
            scanner.accept_tok(TokenType::NEG_EQUAL_SIGN | TokenType::EQUAL_SIGN)?;
            let right = self.parse_term_real(scanner, true)?;
            self.encode_equality_term(left, right, positive)
        } else {
            if self.tformula_atom_can_stay_plain_term(&left) {
                return Ok(left);
            }
            self.prepare_predicate_formula_atom(&left)?;
            self.encode_predicate_as_eqn(left)
        }
    }

    fn parse_literal_tformula_tstp_subset(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Term, Diagnostic> {
        let formula = if scanner.test_tok(
            TokenType::UNIV_QUANTOR | TokenType::EXIST_QUANTOR | TokenType::LAMBDA_QUANTOR,
        ) {
            let quantor = self.tptp_quantor_parse(scanner)?;
            scanner.accept_tok(TokenType::OPEN_SQUARE)?;
            self.parse_quantified_tformula_tstp_subset(scanner, quantor)?
        } else if scanner.test_tok(TokenType::OPEN_BRACKET) {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            let formula = if scanner.test_tok(TokenType::FOF_BIN_OP)
                && scanner.look_token(1).kind() == TokenType::CLOSE_BRACKET
            {
                let op = self.tptp_operator_parse(scanner)?;
                self.make_logical_tformula_head(op)
            } else if scanner.test_tok(TokenType::TILDE_SIGN)
                && scanner.look_token(1).kind() == TokenType::CLOSE_BRACKET
            {
                scanner.accept_tok(TokenType::TILDE_SIGN)?;
                let op = Self::require_formula_op_code(self.sig.not_code())?;
                self.make_logical_tformula_head(op)
            } else if scanner.test_tok(TokenType::NAME | TokenType::SEM_IDENT)
                && scanner_test_tok(scanner.look_token(1), TokenType::CLOSE_BRACKET)
                && scanner_test_tok(scanner.look_token(2), TokenType::APPLICATION)
            {
                let head = self.parse_term_real(scanner, true)?;
                self.prepare_tformula_application_head(head)
            } else {
                self.parse_tformula_tstp_subset(scanner)?
            };
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            formula
        } else if scanner.test_tok(TokenType::TILDE_SIGN) {
            scanner.accept_tok(TokenType::TILDE_SIGN)?;
            if scanner.test_tok(TokenType::APPLICATION) {
                scanner.accept_tok(TokenType::APPLICATION)?;
            }
            let child = self.parse_literal_tformula_tstp_with_applications(scanner)?;
            self.tformula_fcode_alloc(
                Self::require_formula_op_code(self.sig.not_code())?,
                child,
                None,
            )?
        } else if scanner.test_tok(TokenType::ITE_TOKEN) {
            self.parse_ite_tformula_tstp_subset(scanner)?
        } else if scanner.test_tok(TokenType::LET_TOKEN) {
            self.parse_let_tformula_tstp_subset(scanner)?
        } else {
            self.parse_tformula_atom(scanner)?
        };
        if scanner.test_tok(TokenType::APPLICATION) {
            Ok(formula)
        } else {
            self.encode_predicate_as_eqn(formula)
        }
    }

    fn parse_quantified_tformula_tptp(
        &mut self,
        scanner: &mut Scanner,
        quantor: FunCode,
    ) -> Result<Term, Diagnostic> {
        self.vars.push_env();
        let parsed = (|| {
            let variable = self.parse_term_real(scanner, true)?;
            if !variable.is_free_var() {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    "Variable expected, non-variable term found",
                ));
            }
            let rest = if scanner.test_tok(TokenType::COMMA) {
                scanner.accept_tok(TokenType::COMMA)?;
                self.parse_quantified_tformula_tptp(scanner, quantor)?
            } else {
                scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
                scanner.accept_tok(TokenType::COLON)?;
                self.parse_elem_tformula_tptp(scanner)?
            };
            self.tformula_fcode_alloc(quantor, variable, Some(rest))
        })();
        self.vars.pop_env();
        parsed
    }

    fn parse_applied_tformula_tstp_subset(
        &mut self,
        scanner: &mut Scanner,
        head: &Term,
    ) -> Result<Term, Diagnostic> {
        let head_type = self.tformula_head_type(head)?;
        let max_args = type_get_max_arity(&head_type);
        let head_is_logical = !head.is_free_var() && self.sig.query_prop(head.f_code(), FP_FOF_OP);
        let mut args = Vec::new();

        while scanner.test_tok(TokenType::APPLICATION) {
            if args.len() >= max_args {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    "Too many arguments applied to the term",
                ));
            }
            let expected_type = head_type.args().get(args.len()).ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "Applied formula head type is missing an argument sort",
                )
            })?;

            scanner.accept_tok(TokenType::APPLICATION)?;
            let mut arg = self.parse_tformula_application_arg(scanner, expected_type)?;
            if head_is_logical {
                arg = self.encode_predicate_as_eqn(arg)?;
            }
            Self::require_term_sort(&arg, expected_type, "formula application argument")?;
            args.push(arg);
        }

        let applied = lambda_apply_terms(self, head, &args)?;
        self.encode_predicate_as_eqn(applied)
    }

    fn parse_tformula_application_arg(
        &mut self,
        scanner: &mut Scanner,
        expected_type: &Type,
    ) -> Result<Term, Diagnostic> {
        self.parse_tformula_application_arg_with_tail(scanner, expected_type, false)
    }

    fn parse_tformula_application_arg_with_tail(
        &mut self,
        scanner: &mut Scanner,
        expected_type: &Type,
        allow_application_tail: bool,
    ) -> Result<Term, Diagnostic> {
        if scanner.test_tok(TokenType::OPEN_BRACKET) && !expected_type.is_bool() {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            let arg =
                self.parse_tformula_application_arg_with_tail(scanner, expected_type, true)?;
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            return Ok(arg);
        }

        let arg = if expected_type.is_bool() {
            self.parse_tformula_application_bool_arg(scanner)?
        } else if scanner.test_tok(TokenType::LAMBDA_QUANTOR) {
            self.parse_literal_tformula_tstp_with_applications(scanner)?
        } else {
            self.parse_tformula_application_term_arg(scanner, allow_application_tail)?
        };
        Self::require_term_sort(&arg, expected_type, "formula application argument")?;
        Ok(arg)
    }

    fn parse_tformula_application_bool_arg(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Term, Diagnostic> {
        if scanner.test_tok(TokenType::OPEN_BRACKET) {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            let arg = self.parse_tformula_tstp_subset(scanner)?;
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            Ok(arg)
        } else {
            self.parse_literal_tformula_tstp_subset(scanner)
        }
    }

    fn parse_tformula_application_term_arg(
        &mut self,
        scanner: &mut Scanner,
        allow_application_tail: bool,
    ) -> Result<Term, Diagnostic> {
        let mut term = self.parse_term_real(scanner, true)?;
        if !term.is_any_var() {
            term = self.prepare_tformula_application_head(term);
            term = self.term_top_insert(term)?;
        }
        if allow_application_tail && scanner.test_tok(TokenType::APPLICATION) {
            term = self.parse_applied_tformula_term_tstp_subset(scanner, &term)?;
        }
        Ok(term)
    }

    fn parse_applied_tformula_term_tstp_subset(
        &mut self,
        scanner: &mut Scanner,
        head: &Term,
    ) -> Result<Term, Diagnostic> {
        let head_type = self.tformula_head_type(head)?;
        let max_args = type_get_max_arity(&head_type);
        let mut args = Vec::new();

        while scanner.test_tok(TokenType::APPLICATION) {
            if args.len() >= max_args {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    "Too many arguments applied to the term",
                ));
            }
            let expected_type = head_type.args().get(args.len()).ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "Applied formula head type is missing an argument sort",
                )
            })?;

            scanner.accept_tok(TokenType::APPLICATION)?;
            let arg = self.parse_tformula_application_arg(scanner, expected_type)?;
            Self::require_term_sort(&arg, expected_type, "formula application argument")?;
            args.push(arg);
        }

        lambda_apply_terms(self, head, &args)
    }

    fn make_logical_tformula_head(&mut self, op: FunCode) -> Term {
        let head = Term::top_alloc(op, 0);
        if let Some(type_) = self.sig.get_type(op).cloned() {
            head.set_type(Some(type_));
        }
        // This is only an application head; inserting it would collide with
        // bool-typed zero-arity logical terms that use the same f-code.
        head
    }

    fn parse_quantified_tformula_tstp_subset(
        &mut self,
        scanner: &mut Scanner,
        quantor: FunCode,
    ) -> Result<Term, Diagnostic> {
        self.vars.push_env();
        let parsed = (|| {
            let variable = self.parse_term_real(scanner, true)?;
            if !variable.is_free_var() {
                return Err(Diagnostic::new(
                    ErrorCode::SYNTAX_ERROR,
                    "Variable expected, non-variable term found",
                ));
            }
            let rest = if scanner.test_tok(TokenType::COMMA) {
                scanner.accept_tok(TokenType::COMMA)?;
                self.parse_quantified_tformula_tstp_subset(scanner, quantor)?
            } else {
                scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
                scanner.accept_tok(TokenType::COLON)?;
                self.parse_literal_tformula_tstp_with_applications(scanner)?
            };
            self.tformula_fcode_alloc(quantor, variable, Some(rest))
        })();
        self.vars.pop_env();
        parsed
    }

    fn parse_ite_tformula_tstp_subset(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Term, Diagnostic> {
        scanner.accept_tok(TokenType::ITE_TOKEN)?;
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        let condition = self.parse_tformula_tstp_subset(scanner)?;
        scanner.accept_tok(TokenType::COMMA)?;
        let if_true = self.parse_tformula_tstp_subset(scanner)?;
        scanner.accept_tok(TokenType::COMMA)?;
        let if_false = self.parse_tformula_tstp_subset(scanner)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

        Self::require_same_sort(&condition, &self.true_term, "$ite condition")?;
        Self::require_same_sort(&if_true, &if_false, "$ite branches")?;
        let result_type = if_true.type_().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                "$ite true branch must have an inferred type",
            )
        })?;

        let ite = Term::top_alloc(SIG_ITE_CODE, 3);
        ite.set_type(Some(result_type));
        ite.set_argument(0, condition);
        ite.set_argument(1, if_true);
        ite.set_argument(2, if_false);
        self.term_top_insert(ite)
    }

    fn parse_let_tformula_tstp_subset(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Term, Diagnostic> {
        scanner.accept_tok(TokenType::LET_TOKEN)?;
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;

        let type_declarations = self.parse_let_type_declarations(scanner)?;
        scanner.accept_tok(TokenType::COMMA)?;

        let definitions = self.parse_let_symbol_definitions(scanner, &type_declarations)?;
        scanner.accept_tok(TokenType::COMMA)?;

        self.sig
            .enter_let_scope(&let_type_declaration_codes(&type_declarations));
        let body = match self.parse_tformula_tstp_subset(scanner) {
            Ok(body) => {
                self.sig.exit_let_scope();
                body
            }
            Err(error) => {
                self.sig.exit_let_scope();
                return Err(error);
            }
        };

        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        self.make_let_term(definitions, body)
    }

    fn parse_let_type_declarations(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<Vec<LetTypeDeclaration>, Diagnostic> {
        let mut declarations = Vec::new();
        if scanner.test_tok(TokenType::OPEN_SQUARE) {
            scanner.accept_tok(TokenType::OPEN_SQUARE)?;
            declarations.push(self.parse_let_type_declaration(scanner)?);
            while scanner.test_tok(TokenType::COMMA) {
                scanner.accept_tok(TokenType::COMMA)?;
                declarations.push(self.parse_let_type_declaration(scanner)?);
            }
            scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
        } else {
            declarations.push(self.parse_let_type_declaration(scanner)?);
        }
        Ok(declarations)
    }

    fn parse_let_type_declaration(
        &mut self,
        scanner: &mut Scanner,
    ) -> Result<LetTypeDeclaration, Diagnostic> {
        let mut id = DynamicString::new();
        let sym_type = func_symb_parse(scanner, &mut id)?;
        if sym_type != FuncSymbType::IdentFreeFun {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "let declaration expects a function symbol",
            ));
        }

        scanner.accept_tok(TokenType::COLON)?;
        let type_ = self
            .sig
            .type_bank_mut()
            .parse_type_from_current_problem(scanner)?;
        let name = id.view().into_owned();
        let f_code = self.sig.insert_let_id(&name, type_.clone());
        Ok(LetTypeDeclaration {
            name,
            f_code,
            type_,
        })
    }

    fn parse_let_symbol_definitions(
        &mut self,
        scanner: &mut Scanner,
        type_declarations: &[LetTypeDeclaration],
    ) -> Result<Vec<Term>, Diagnostic> {
        let mut definitions = Vec::new();
        if scanner.test_tok(TokenType::OPEN_SQUARE) {
            scanner.accept_tok(TokenType::OPEN_SQUARE)?;
            definitions.push(self.parse_let_symbol_definition(scanner, type_declarations)?);
            while scanner.test_tok(TokenType::COMMA) {
                scanner.accept_tok(TokenType::COMMA)?;
                definitions.push(self.parse_let_symbol_definition(scanner, type_declarations)?);
            }
            scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
        } else {
            definitions.push(self.parse_let_symbol_definition(scanner, type_declarations)?);
        }
        Ok(definitions)
    }

    fn parse_let_symbol_definition(
        &mut self,
        scanner: &mut Scanner,
        type_declarations: &[LetTypeDeclaration],
    ) -> Result<Term, Diagnostic> {
        let mut id = DynamicString::new();
        let _sym_type = func_symb_parse(scanner, &mut id)?;
        let name = id.view();
        let Some(declaration) = type_declarations
            .iter()
            .find(|declaration| declaration.name == name)
        else {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "symbol not in let declaration list",
            ));
        };

        let variables = self.parse_let_definition_variables(scanner, &declaration.type_)?;
        let parsed = (|| {
            scanner.accept_tok(TokenType::COLON)?;
            scanner.accept_tok(TokenType::EQUAL_SIGN)?;
            let rhs = self.parse_tformula_tstp_subset(scanner)?;
            let lhs = self.let_definition_lhs(declaration.f_code, &variables)?;
            self.encode_equality_term(lhs, rhs, true)
        })();
        for _ in &variables {
            self.vars.pop_env();
        }
        parsed
    }

    fn parse_let_definition_variables(
        &mut self,
        scanner: &mut Scanner,
        type_: &Type,
    ) -> Result<Vec<Term>, Diagnostic> {
        let arity = type_get_max_arity(type_);
        if arity == 0 {
            return Ok(Vec::new());
        }

        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        let mut variables = Vec::with_capacity(arity);
        let mut names = Vec::with_capacity(arity);
        let parsed = (|| {
            for index in 0..arity {
                let mut id = DynamicString::new();
                let sym_type = func_symb_parse(scanner, &mut id)?;
                let name = id.view().into_owned();
                if names.iter().any(|seen| seen == &name) {
                    return Err(Diagnostic::new(
                        ErrorCode::SYNTAX_ERROR,
                        "variables must be distinct",
                    ));
                }
                names.push(name.clone());
                if sym_type != FuncSymbType::IdentVar {
                    return Err(Diagnostic::new(
                        ErrorCode::SYNTAX_ERROR,
                        "variable is expected",
                    ));
                }

                let arg_type = type_.args().get(index).ok_or_else(|| {
                    Diagnostic::new(
                        ErrorCode::TYPE_ERROR,
                        "let definition type is missing an argument sort",
                    )
                })?;
                self.vars.push_env();
                variables.push(self.vars.ext_name_assert_alloc_sort(&name, arg_type));
                if index + 1 != arity {
                    scanner.accept_tok(TokenType::COMMA)?;
                }
            }
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            Ok(())
        })();
        if let Err(error) = parsed {
            for _ in &variables {
                self.vars.pop_env();
            }
            return Err(error);
        }
        Ok(variables)
    }

    fn let_definition_lhs(
        &mut self,
        f_code: FunCode,
        variables: &[Term],
    ) -> Result<Term, Diagnostic> {
        let lhs = Term::top_alloc(f_code, variables.len());
        for (index, variable) in variables.iter().enumerate() {
            lhs.set_argument(index, variable.clone());
        }
        self.term_top_insert(lhs)
    }

    fn make_let_term(&mut self, definitions: Vec<Term>, body: Term) -> Result<Term, Diagnostic> {
        let body_type = body.type_().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                "$let body must have an inferred type",
            )
        })?;
        let let_term = Term::top_alloc(SIG_LET_CODE, definitions.len() + 1);
        let_term.set_type(Some(body_type));
        for (index, definition) in definitions.into_iter().enumerate() {
            let_term.set_argument(index, definition);
        }
        let_term.set_argument(let_term.arity() - 1, body);
        self.term_top_insert(let_term)
    }

    fn require_same_sort(left: &Term, right: &Term, context: &str) -> Result<(), Diagnostic> {
        let left_type = left.type_().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                format!("{context} left term has no type"),
            )
        })?;
        let right_type = right.type_().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                format!("{context} right term has no type"),
            )
        })?;
        if left_type != right_type {
            return Err(Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                format!("{context} terms should have the same sort"),
            ));
        }
        Ok(())
    }

    fn require_term_sort(term: &Term, expected: &Type, context: &str) -> Result<(), Diagnostic> {
        let Some(actual) = term.type_() else {
            return Err(Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                format!("{context} has no inferred type"),
            ));
        };
        if &actual != expected {
            return Err(Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                format!("{context} has the wrong sort"),
            ));
        }
        Ok(())
    }

    fn tformula_head_type(&mut self, term: &Term) -> Result<Type, Diagnostic> {
        if term.f_code() == SIG_ITE_CODE || term.f_code() == SIG_LET_CODE {
            term.type_().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "formula application head has no inferred type",
                )
            })
        } else if term.f_code() == self.sig.qex_code() || term.f_code() == self.sig.qall_code() {
            Ok(self.sig.type_bank().bool_type())
        } else if term.is_applied_any_var() {
            let head = term.argument(0).ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "applied variable head is uninitialized",
                )
            })?;
            head.type_().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "applied variable head has no inferred type",
                )
            })
        } else if term.is_any_var() || term.is_lambda() {
            term.type_().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "formula application head has no inferred type",
                )
            })
        } else if term.is_phony_app() {
            let head = term.argument(0).ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "phony application head is uninitialized",
                )
            })?;
            let head_type = self.tformula_head_type(&head)?;
            if !head_type.is_arrow() {
                return Err(Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "phony application head type must be an arrow",
                ));
            }
            Ok(self
                .sig
                .type_bank_mut()
                .insert_type_shared(type_drop_first_arg(&head_type)))
        } else if term.arity() != 0 {
            term.type_().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "formula application head has no inferred type",
                )
            })
        } else {
            self.sig.get_type(term.f_code()).cloned().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::TYPE_ERROR,
                    "formula application head has no declared type",
                )
            })
        }
    }

    fn parse_tformula_atom(&mut self, scanner: &mut Scanner) -> Result<Term, Diagnostic> {
        let mut left = self.parse_term_real(scanner, true)?;
        let mut positive = true;
        if scanner.test_tok(TokenType::NEG_EQUAL_SIGN | TokenType::EQUAL_SIGN) {
            if scanner.test_tok(TokenType::NEG_EQUAL_SIGN) {
                positive = false;
            }
            scanner.accept_tok(TokenType::NEG_EQUAL_SIGN | TokenType::EQUAL_SIGN)?;
            let right = if left.type_().as_ref().is_some_and(Type::is_bool) {
                let right = self.parse_tformula_tstp_subset(scanner)?;
                left = self.encode_equality_term(left, self.true_term.clone(), true)?;
                right
            } else {
                self.parse_tformula_application_term_arg(scanner, true)?
            };
            self.encode_equality_term(left, right, positive)
        } else {
            if scanner.test_tok(TokenType::APPLICATION) {
                return Ok(self.prepare_tformula_application_head(left));
            }
            if self.tformula_atom_can_stay_plain_term(&left) {
                return Ok(left);
            }
            self.prepare_predicate_formula_atom(&left)?;
            Ok(left)
        }
    }

    fn prepare_tformula_application_head(&mut self, head: Term) -> Term {
        let Some((f_code, head_type)) = self.tformula_application_head_code_and_type(&head) else {
            return head;
        };
        if f_code == head.f_code() && head.type_().as_ref() == Some(&head_type) {
            return head;
        }

        let recovered = if head.arity() == 0 {
            Term::const_cell_alloc(f_code)
        } else {
            let recovered = Term::top_alloc(f_code, head.arity());
            for (index, arg) in head.argument_clones().into_iter().enumerate() {
                recovered.set_argument(
                    index,
                    arg.unwrap_or_else(|| {
                        panic!("application head argument {index} is uninitialized")
                    }),
                );
            }
            recovered
        };
        recovered.set_type(Some(head_type));
        recovered
    }

    fn tformula_application_head_code_and_type(&mut self, head: &Term) -> Option<(FunCode, Type)> {
        if let Some(type_) = self
            .sig
            .get_type(head.f_code())
            .cloned()
            .and_then(|type_| self.tformula_application_residual_type(&type_, head.arity()))
        {
            return Some((head.f_code(), type_));
        }

        let name = self.sig.find_name(head.f_code())?.to_owned();
        let f_code = self.sig.find_f_code(&name);
        if f_code == 0 || f_code == head.f_code() {
            return None;
        }
        self.sig
            .get_type(f_code)
            .cloned()
            .and_then(|type_| self.tformula_application_residual_type(&type_, head.arity()))
            .map(|type_| (f_code, type_))
    }

    fn tformula_application_residual_type(
        &mut self,
        symbol_type: &Type,
        consumed_args: usize,
    ) -> Option<Type> {
        if !symbol_type.is_arrow() || consumed_args >= type_get_max_arity(symbol_type) {
            return None;
        }
        let residual = if consumed_args == 0 {
            symbol_type.clone()
        } else {
            self.sig
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(
                    symbol_type.args()[consumed_args..].to_vec(),
                ))
        };
        residual.is_arrow().then_some(residual)
    }

    fn tformula_atom_can_stay_plain_term(&self, term: &Term) -> bool {
        if term.is_any_var() || term.type_().as_ref().is_some_and(type_is_predicate) {
            return false;
        }
        term.is_phony_app()
            || term.is_lambda()
            || term.f_code() == SIG_ITE_CODE
            || term.f_code() == SIG_LET_CODE
            || self.sig.is_function(term.f_code())
    }

    fn prepare_predicate_formula_atom(&mut self, term: &Term) -> Result<(), Diagnostic> {
        if term.is_free_var() {
            if term.type_().as_ref().is_some_and(type_is_predicate) {
                return Ok(());
            }
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Individual variable used at predicate position",
            ));
        }
        if (term.f_code() == SIG_ITE_CODE || term.f_code() == SIG_LET_CODE)
            && term.type_().as_ref().is_some_and(Type::is_bool)
        {
            return Ok(());
        }
        type_declare_is_predicate(&mut self.sig, term)
    }

    fn encode_predicate_as_eqn(&mut self, formula: Term) -> Result<Term, Diagnostic> {
        let is_encodable = (formula.is_any_var()
            || !self.sig.is_logical_symbol(formula.f_code())
            || formula.f_code() == self.sig.answer_code()
            || formula.f_code() == SIG_TRUE_CODE
            || formula.f_code() == SIG_FALSE_CODE
            || formula.f_code() == SIG_ITE_CODE
            || formula.f_code() == SIG_LET_CODE
            || formula.is_phony_app())
            && formula.type_().as_ref().is_some_and(Type::is_bool);
        if !is_encodable {
            return Ok(formula);
        }

        let (left, positive) = if formula.is_any_var() {
            (formula, true)
        } else if formula.f_code() == SIG_FALSE_CODE {
            (self.true_term.clone(), false)
        } else {
            (formula, true)
        };
        self.encode_equality_term(left, self.true_term.clone(), positive)
    }

    fn encode_equality_term(
        &mut self,
        left: Term,
        right: Term,
        positive: bool,
    ) -> Result<Term, Diagnostic> {
        let f_code = self.sig.get_eqn_code(positive);
        assert_ne!(f_code, 0, "equality code allocation must succeed");
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(self.sig.type_bank().bool_type()));
        term.set_argument(0, left);
        term.set_argument(1, right);
        self.term_top_insert(term)
    }

    fn tformula_fcode_alloc(
        &mut self,
        op: FunCode,
        arg1: Term,
        arg2: Option<Term>,
    ) -> Result<Term, Diagnostic> {
        let arity = self.sig.find_arity(op).ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "TFormulaFCodeAlloc requires a known signature arity",
            )
        })?;
        let arity = usize::try_from(arity).map_err(|_| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "TFormulaFCodeAlloc requires unary or binary formula arity",
            )
        })?;
        if arity != 1 && arity != 2 {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "TFormulaFCodeAlloc requires unary or binary formula arity",
            ));
        }
        if arity == 2 && arg2.is_none() {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "TFormulaFCodeAlloc binary formula is missing its second argument",
            ));
        }

        let term = Term::top_alloc(op, arity);
        if op != SIG_NAMED_LAMBDA_CODE {
            term.set_type(Some(self.sig.type_bank().bool_type()));
        }
        if self.sig.is_predicate(op) {
            term.set_prop(TP_PRED_POS);
        }
        term.set_argument(0, arg1);
        if let Some(arg2) = arg2 {
            term.set_argument(1, arg2);
        }
        self.term_top_insert(term)
    }

    fn tptp_operator_parse(&mut self, scanner: &mut Scanner) -> Result<FunCode, Diagnostic> {
        scanner.check_tok(TokenType::FOF_BIN_OP)?;
        let op = self.tptp_operator_convert(scanner.current_token().kind())?;
        scanner.next_token()?;
        Ok(op)
    }

    fn tptp_quantor_parse(&self, scanner: &mut Scanner) -> Result<FunCode, Diagnostic> {
        scanner.check_tok(
            TokenType::UNIV_QUANTOR | TokenType::EXIST_QUANTOR | TokenType::LAMBDA_QUANTOR,
        )?;
        let quantor = if scanner.test_tok(TokenType::EXIST_QUANTOR) {
            self.sig.qex_code()
        } else if scanner.test_tok(TokenType::UNIV_QUANTOR) {
            self.sig.qall_code()
        } else {
            SIG_NAMED_LAMBDA_CODE
        };
        let quantor = Self::require_formula_op_code(quantor)?;
        scanner.next_token()?;
        Ok(quantor)
    }

    fn tptp_operator_convert(&mut self, token: TokenType) -> Result<FunCode, Diagnostic> {
        let op = if token == TokenType::FOF_OR {
            self.sig.or_code()
        } else if token == TokenType::FOF_AND {
            self.sig.and_code()
        } else if token == TokenType::FOF_LR_IMPL {
            self.sig.impl_code()
        } else if token == TokenType::FOF_RL_IMPL {
            self.sig.bimpl_code()
        } else if token == TokenType::FOF_EQUIV {
            self.sig.equiv_code()
        } else if token == TokenType::EQUAL_SIGN {
            self.sig.get_eqn_code(true)
        } else if token == TokenType::FOF_XOR {
            self.sig.xor_code()
        } else if token == TokenType::NEG_EQUAL_SIGN {
            self.sig.get_eqn_code(false)
        } else if token == TokenType::FOF_NAND {
            self.sig.nand_code()
        } else if token == TokenType::FOF_NOR {
            self.sig.nor_code()
        } else {
            0
        };
        Self::require_formula_op_code(op)
    }

    fn require_formula_op_code(op: FunCode) -> Result<FunCode, Diagnostic> {
        if op == 0 {
            Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Boolean formula operator requires initialized internal FOF symbols",
            ))
        } else {
            Ok(op)
        }
    }

    fn normalize_boolean_term_arg(&self, term: Term) -> Term {
        if term.f_code() == self.sig.eqn_code() {
            let Some(left) = term.argument(0) else {
                return term;
            };
            let Some(right) = term.argument(1) else {
                return term;
            };
            if left.is_free_var() && right.f_code() == SIG_TRUE_CODE {
                return left;
            }
            if left.f_code() == SIG_TRUE_CODE && right.f_code() == SIG_TRUE_CODE {
                return self.true_term.clone();
            }
        } else if term.f_code() == self.sig.neqn_code() {
            let Some(left) = term.argument(0) else {
                return term;
            };
            let Some(right) = term.argument(1) else {
                return term;
            };
            if left.f_code() == SIG_TRUE_CODE && right.f_code() == SIG_TRUE_CODE {
                return self.false_term.clone();
            }
        }
        term
    }

    /// Creates and inserts a new Skolem term or definition atom.
    ///
    /// The generated symbol type is built from `variables` followed by
    /// `ret_type`, flattened, shared in the signature type bank, and declared
    /// on the generated `esk`/`epred` symbol like C `TermAllocNewSkolem`.
    ///
    /// # Panics
    ///
    /// Panics if the variable count does not fit in a C `int`, or if any
    /// variable lacks a type.
    pub fn alloc_new_skolem(
        &mut self,
        variables: &[Term],
        ret_type: Option<&Type>,
    ) -> Result<Term, Diagnostic> {
        let ret_type = ret_type
            .cloned()
            .unwrap_or_else(|| self.sig.type_bank().i_type());
        let arity = i32::try_from(variables.len()).expect("skolem arity fits in i32");
        let type_ = if variables.is_empty() {
            flatten_type(&ret_type)
        } else {
            let mut args = Vec::with_capacity(variables.len() + 1);
            for variable in variables {
                args.push(variable.type_().expect("skolem variable has a type"));
            }
            args.push(ret_type.clone());
            flatten_type(&alloc_arrow_type(args))
        };
        let shared_type = self.sig.type_bank_mut().insert_type_shared(type_);
        let f_code = if type_is_predicate(&shared_type) {
            self.sig.get_new_predicate_code(arity)
        } else {
            self.sig.get_new_skolem_code(arity)
        };
        self.sig.declare_type(f_code, shared_type)?;

        let term = if variables.is_empty() {
            Term::const_cell_alloc(f_code)
        } else {
            let term = Term::top_alloc(f_code, variables.len());
            for (index, variable) in variables.iter().enumerate() {
                term.set_argument(index, variable.clone());
            }
            term
        };
        term.set_type(Some(ret_type));
        self.insert(&term, DerefType::Never)
    }

    /// Returns the cached minimal term for the constant's type.
    ///
    /// # Panics
    ///
    /// Panics if the signature has no type for `min_const`, matching the C
    /// assertion in `TBCreateMinTerm`.
    pub fn create_min_term(&mut self, min_const: FunCode) -> Result<Term, Diagnostic> {
        let type_ = self
            .sig
            .get_type(min_const)
            .expect("minimal constant must have a declared type")
            .clone();
        let type_uid = type_.type_uid();
        if let Some(term) = self.min_terms.get(&type_uid) {
            return Ok(term.clone());
        }
        let term = self.create_const_term(min_const)?;
        assert!(term.type_().is_some(), "minimal term has a type");
        self.min_terms.insert(type_uid, term.clone());
        Ok(term)
    }

    pub fn get_first_const_term(&mut self, sort: &Type) -> Result<Option<Term>, Diagnostic> {
        self.sig
            .collect_sort_consts(sort)
            .first()
            .copied()
            .map(|f_code| self.create_const_term(f_code))
            .transpose()
    }

    /// Returns a shared constant selected by the supplied C-style comparator.
    ///
    /// # Panics
    ///
    /// Panics if either distribution array is not large enough to address all
    /// currently known function codes, matching the C precondition that these
    /// arrays are sized to `sig->f_count + 1`.
    pub fn get_freq_const_term<F>(
        &mut self,
        sort: &Type,
        conj_dist_array: &mut [i64],
        dist_array: &[i64],
        mut is_better: F,
    ) -> Result<Option<Term>, Diagnostic>
    where
        F: FnMut(FunCode, FunCode, &mut [i64], &[i64]) -> bool,
    {
        let candidates = self.sig.collect_sort_consts(sort);
        let Some((&first, rest)) = candidates.split_first() else {
            return Ok(None);
        };

        let required_len = usize::try_from(self.sig.f_count())
            .unwrap_or(usize::MAX)
            .saturating_add(1);
        assert!(
            conj_dist_array.len() >= required_len,
            "conjecture distribution must cover all function codes"
        );
        assert!(
            dist_array.len() >= required_len,
            "global distribution must cover all function codes"
        );

        let mut best = first;
        for &candidate in rest {
            if is_better(candidate, best, conj_dist_array, dist_array) {
                best = candidate;
            }
        }
        self.create_const_term(best).map(Some)
    }

    /// Applies `mapper` recursively like C `TermMap`.
    ///
    /// `mapper` must return either `None` to stop recursion at `term`, or a
    /// shared term with the same type as the input term. Returning a different
    /// term restarts mapping at that replacement; returning the same term maps
    /// the arguments recursively.
    ///
    /// # Panics
    ///
    /// Panics if `mapper` returns an unshared or differently typed term, or if
    /// recursive argument mapping produces an unshared or differently typed
    /// argument, matching the C assertions in `TermMap`.
    pub fn map_term<F>(&mut self, term: &Term, map_fn: &mut F) -> Result<Term, Diagnostic>
    where
        F: FnMut(&mut Self, &Term) -> Result<Option<Term>, Diagnostic>,
    {
        let Some(mapped_term) = map_fn(self, term)? else {
            return Ok(term.clone());
        };
        assert!(
            mapped_term.is_shared(),
            "term mapper must return shared terms"
        );
        assert_eq!(
            mapped_term.type_(),
            term.type_(),
            "term mapper must preserve term type"
        );

        if mapped_term != term.clone() {
            return self.map_term(&mapped_term, map_fn);
        }
        if term.is_any_var() {
            return Ok(term.clone());
        }

        let copy = Term::top_copy_without_args(term);
        let mut changed = false;
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let mapped_arg = self.map_term(&arg, map_fn)?;
            assert!(
                mapped_arg.is_shared(),
                "term mapper must return shared arguments"
            );
            assert_eq!(
                mapped_arg.type_(),
                arg.type_(),
                "term mapper must preserve argument type"
            );
            if mapped_arg != arg {
                changed = true;
            }
            copy.set_argument(index, mapped_arg);
        }

        if changed {
            self.term_top_insert(copy)
        } else {
            Ok(term.clone())
        }
    }

    #[must_use]
    pub fn term_apply_arg(&mut self, source: &Term, arg: &Term) -> Term {
        term_apply_arg_unshared(self.sig.type_bank_mut(), source, arg)
    }

    /// Sets properties by repointing `term_ref` to the banked top-cell variant.
    ///
    /// # Panics
    ///
    /// Panics if `term_ref` is a variable, matching the C assertion in
    /// `TBRefSetProp`.
    pub fn ref_set_prop(
        &mut self,
        term_ref: &mut Term,
        prop: TermProperties,
    ) -> Result<(), Diagnostic> {
        assert!(
            !term_ref.is_any_var(),
            "properties do not work for variables"
        );
        if term_ref.query_prop(prop) {
            return Ok(());
        }
        let new = Term::top_copy(term_ref);
        new.set_prop(prop);
        *term_ref = self.term_top_insert(new)?;
        Ok(())
    }

    pub fn ref_del_prop(
        &mut self,
        term_ref: &mut Term,
        prop: TermProperties,
    ) -> Result<(), Diagnostic> {
        if !term_ref.is_any_prop_set(prop) || term_ref.is_any_var() {
            return Ok(());
        }
        let new = Term::top_copy(term_ref);
        new.del_prop(prop);
        *term_ref = self.term_top_insert(new)?;
        Ok(())
    }

    pub fn gc_mark_term(&self, term: &Term) {
        let mut stack = vec![term.clone()];
        while let Some(current) = stack.pop() {
            if current.give_props(TP_GARBAGE_FLAG) == self.garbage_state {
                current.flip_prop(TP_GARBAGE_FLAG);
                stack.extend(current.argument_clones().into_iter().flatten());
                if current.is_rewritten() {
                    if let Some(replacement) = current.rw_replace_field() {
                        stack.push(replacement);
                    }
                }
            }
        }
    }

    /// Sweeps unmarked terms and flips the bank garbage state.
    ///
    /// # Panics
    ///
    /// Panics if the bank's `$true` term is currently marked rewritten,
    /// matching the C assertion before `TBGCSweep` marks roots.
    #[must_use]
    pub fn gc_sweep(&mut self) -> i64 {
        assert!(
            !self.true_term.is_rewritten(),
            "true term must not be rewritten during GC"
        );
        self.gc_mark_term(&self.true_term);
        self.gc_mark_term(&self.false_term);
        for term in self.min_terms.values() {
            self.gc_mark_term(term);
        }
        let recovered = self.term_store.gc_sweep(self.garbage_state);
        self.garbage_state = if self.garbage_state == TP_IGNORE_PROPS {
            TP_GARBAGE_FLAG
        } else {
            TP_IGNORE_PROPS
        };
        self.recovered += u64::try_from(recovered).unwrap_or(0);
        recovered
    }

    #[must_use]
    pub fn find_repr(&mut self, term: &Term) -> Option<Term> {
        if term.is_any_var() || term.is_const() {
            return self.find(term);
        }

        let work = Term::top_copy(term);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg?;
            let repr = self.find_repr(&arg)?;
            work.set_argument(index, repr);
        }
        self.find(&work)
    }

    fn insert_with_mode(
        &mut self,
        term: &Term,
        deref: DerefType,
        mode: InsertMode,
    ) -> Result<Term, Diagnostic> {
        let (term, current_deref, limit) = self.deref_root_no_whnf(term, deref)?;
        self.insert_derefed_with_mode(&term, current_deref, limit, mode)
    }

    fn insert_derefed_with_mode(
        &mut self,
        term: &Term,
        deref: DerefType,
        limit: usize,
        mode: InsertMode,
    ) -> Result<Term, Diagnostic> {
        if term.is_free_var() {
            if mode == InsertMode::KeepVariables {
                return Ok(term.clone());
            }
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(term);
        if mode == InsertMode::NoProperties {
            copy.set_properties(TP_IGNORE_PROPS);
        }
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared =
                self.insert_with_mode(&arg, Self::convert_lfho_deref(index, limit, deref), mode)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    fn insert_opt_derefed(
        &mut self,
        term: &Term,
        deref: DerefType,
        limit: usize,
    ) -> Result<Term, Diagnostic> {
        if term_is_ground_for_insert(term) {
            assert!(
                term.is_shared(),
                "optimized ground insertion expects sharing"
            );
            return Ok(term.clone());
        }
        if term.is_free_var() {
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(term);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_opt(&arg, Self::convert_lfho_deref(index, limit, deref))?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
    }

    fn parse_simple_arg_list_opt(
        &mut self,
        scanner: &mut Scanner,
        check_distinct_argument_lists: bool,
    ) -> Result<Vec<Term>, Diagnostic> {
        if !scanner.test_tok(TokenType::OPEN_BRACKET) {
            return Ok(Vec::new());
        }

        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        if scanner.test_tok(TokenType::CLOSE_BRACKET) {
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            return Ok(Vec::new());
        }

        let mut args =
            vec![self
                .parse_term_simple_with_distinct_checks(scanner, check_distinct_argument_lists)?];
        while scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            args.push(
                self.parse_term_simple_with_distinct_checks(
                    scanner,
                    check_distinct_argument_lists,
                )?,
            );
        }
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        Ok(args)
    }

    fn insert_no_props_cached_inner(
        &mut self,
        term: &Term,
        deref: DerefType,
        cache: &mut BTreeMap<usize, Term>,
    ) -> Result<Term, Diagnostic> {
        let (term, current_deref, limit) = self.deref_root_no_whnf(term, deref)?;
        let cache_key = term_identity_id(&term);
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let inserted = if term.is_free_var() {
            let type_ = term.type_().expect("free variable must have a type");
            self.vars.var_assert_alloc(term.f_code(), &type_)
        } else if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            self.db_vars.request_db_var(&type_, term.f_code())
        } else {
            let copy = Term::top_copy_without_args(&term);
            copy.set_properties(TP_IGNORE_PROPS);
            for (index, arg) in term.argument_clones().into_iter().enumerate() {
                let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
                let shared = self.insert_no_props_cached_inner(
                    &arg,
                    Self::convert_lfho_deref(index, limit, current_deref),
                    cache,
                )?;
                copy.set_argument(index, shared);
            }
            self.term_top_insert(copy)?
        };
        cache.insert(cache_key, inserted.clone());
        Ok(inserted)
    }

    fn set_top_insert_metadata(&self, term: &Term) {
        let type_ = term.type_().expect("shared terms have types");
        if term.is_db_var() {
            term.set_prop(TP_HAS_DB_SUBTERM);
        }
        if type_.is_bool() {
            term.set_prop(TP_HAS_BOOL_SUBTERM);
        }
        if term.is_phony_app() && term.argument(0).is_some_and(|arg| arg.is_lambda()) {
            term.set_prop(TP_IS_BETA_REDUCIBLE);
        }
        if term.is_lambda() {
            term.set_prop(TP_HAS_LAMBDA_SUBTERM);
        }
        if type_.is_arrow() && !term.is_lambda() {
            term.set_prop(TP_HAS_ETA_EXPANDABLE_SUBTERM);
        }
        if matches!(term.f_code(), code if code == self.sig.eqn_code() || code == self.sig.neqn_code())
        {
            term.set_prop(TP_HAS_EQ_NEQ_SYM);
        }

        let mut v_count = 0_u32;
        let mut f_count = u32::from(!term.is_phony_app());
        let mut weight = DEFAULT_FWEIGHT * i64::from(f_count);
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            term.set_prop(arg.give_props(TP_IS_BETA_REDUCIBLE));
            term.set_prop(arg.give_props(TP_HAS_DB_SUBTERM));
            term.set_prop(arg.give_props(TP_HAS_EQ_NEQ_SYM));
            term.set_prop(arg.give_props(TP_HAS_BOOL_SUBTERM));
            if arg.type_().is_some_and(|type_| type_.is_bool()) {
                term.set_prop(TP_HAS_BOOL_SUBTERM);
            }
            term.set_prop(arg.give_props(TP_HAS_LAMBDA_SUBTERM));
            if (!(term.is_phony_app() || term.is_lambda())) || index != 0 {
                term.set_prop(arg.give_props(TP_HAS_ETA_EXPANDABLE_SUBTERM));
            }
            term.set_prop(arg.give_props(TP_HAS_NON_PATTERN_VAR));
            term.set_prop(arg.give_props(TP_HAS_APP_VAR));

            if arg.is_free_var() {
                v_count += 1;
                weight += DEFAULT_VWEIGHT;
            } else {
                v_count += arg.v_count();
                f_count += arg.f_count();
                weight += arg.weight();
            }
        }

        if term.f_code() == SIG_DB_LAMBDA_CODE {
            f_count = f_count
                .checked_sub(2)
                .expect("DB lambda count includes binder and lambda sign");
            weight -= 2 * DEFAULT_FWEIGHT;
        }

        if term.is_applied_free_var() {
            term.set_prop(TP_HAS_APP_VAR);
            if normalize_pattern_app_var_no_eta(term).is_some() {
                f_count = 0;
                v_count = 1;
                weight = DEFAULT_VWEIGHT;
            } else {
                term.set_prop(TP_HAS_NON_PATTERN_VAR);
            }
        }

        term.set_v_count(v_count);
        term.set_f_count(f_count);
        term.set_weight(weight);
        if v_count == 0 {
            term.set_prop(TP_IS_GROUND);
        }
    }
}

fn normalize_pattern_app_var_no_eta(term: &Term) -> Option<Term> {
    if term.is_free_var() {
        return Some(term.clone());
    }
    assert!(term.is_applied_free_var(), "expected applied free variable");

    let mut args = Vec::with_capacity(term.arity());
    for index in 0..term.arity() {
        let arg = initialized_arg(term, index);
        if index != 0 && !arg.is_db_var() {
            return None;
        }
        args.push(arg);
    }

    if term_array_no_duplicates(&args) {
        Some(term.clone())
    } else {
        None
    }
}

fn initialized_arg(term: &Term, index: usize) -> Term {
    term.argument(index)
        .unwrap_or_else(|| panic!("term argument {index} is uninitialized"))
}

#[must_use]
pub fn tb_term_del_prop_count(term: &Term, prop: TermProperties) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if current.query_prop(prop) {
            current.del_prop(prop);
            count += 1;
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    count
}

#[must_use]
pub fn tb_term_set_prop_count(term: &Term, prop: TermProperties) -> i64 {
    let mut count = 0;
    let mut stack = vec![term.clone()];
    while let Some(current) = stack.pop() {
        if !current.query_prop(prop) {
            current.set_prop(prop);
            count += 1;
            stack.extend(current.argument_clones().into_iter().flatten());
        }
    }
    count
}

/// Collects shared subterms, using `TPOpFlag` as the visited marker.
///
/// # Panics
///
/// Panics if `term` is not shared, matching the C assertion in
/// `TBTermCollectSubterms`.
pub fn tb_term_collect_subterms(term: &Term, collector: &mut PStack<Term>) -> i64 {
    assert!(term.is_shared(), "subterm collection expects shared terms");
    if term.query_prop(TP_OP_FLAG) {
        return 0;
    }

    let mut count = 1;
    term.set_prop(TP_OP_FLAG);
    collector.push(term.clone());
    for arg in term.argument_clones().into_iter().flatten() {
        count += tb_term_collect_subterms(&arg, collector);
    }
    count
}

#[must_use]
pub fn tb_cell_ident(term: &Term) -> i64 {
    if term.is_free_var() {
        term.f_code()
    } else {
        term.entry_no()
    }
}

#[must_use]
pub fn term_is_true_term(term: &Term) -> bool {
    term.f_code() == SIG_TRUE_CODE
}

#[must_use]
pub fn term_is_false_term(term: &Term) -> bool {
    term.f_code() == SIG_FALSE_CODE
}

#[must_use]
pub fn tb_term_is_ground(term: &Term) -> bool {
    term.query_prop(TP_IS_GROUND)
}

#[must_use]
pub fn tb_term_is_type_term(term: &Term) -> bool {
    term.weight() == DEFAULT_VWEIGHT + DEFAULT_FWEIGHT
}

#[must_use]
pub fn tb_term_is_x_type_term(term: &Term) -> bool {
    term.arity() != 0
        && term.weight()
            == DEFAULT_FWEIGHT + i64::try_from(term.arity()).unwrap_or(0) * DEFAULT_VWEIGHT
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InsertMode {
    ShareVariables,
    KeepVariables,
    NoProperties,
}

#[must_use]
fn term_is_ground_for_insert(term: &Term) -> bool {
    if term.is_shared() {
        tb_term_is_ground(term)
    } else {
        term_is_ground_compute(term)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        tb_cell_ident, tb_term_collect_subterms, tb_term_del_prop_count, tb_term_is_ground,
        tb_term_set_prop_count, term_is_false_term, term_is_true_term, TermBank,
        INSERT_NO_PROPS_CACHE_THRESHOLD,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::pstacks::PStack;
    use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
    use crate::inout::scanner::{Scanner, TokenType};
    use crate::terms::functypes::FunCode;
    use crate::terms::replace::{term_add_rw_link, RwResultType};
    use crate::terms::signature::{
        Signature, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL, SIG_CONS_CODE,
        SIG_FALSE_CODE, SIG_ITE_CODE, SIG_LET_CODE, SIG_NAMED_LAMBDA_CODE, SIG_NIL_CODE,
        SIG_PHONY_APP_CODE, SIG_TRUE_CODE,
    };
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort};
    use crate::terms::termtypes::{
        DerefType, Term, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TP_CHECK_FLAG, TP_GARBAGE_FLAG,
        TP_HAS_APP_VAR, TP_HAS_NON_PATTERN_VAR, TP_IS_GROUND, TP_IS_SHARED, TP_OP_FLAG,
        TP_OUTPUT_FLAG, TP_PRED_POS, TP_TOP_POS,
    };
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    fn set_problem_type_for_test(problem_type: ProblemType) -> ProblemTypeReset {
        reset_problem_type();
        set_problem_type(problem_type).unwrap_or_else(|err| panic!("{err}"));
        ProblemTypeReset
    }

    fn bank_with_symbol(name: &str, arity: i32) -> (TermBank, i64) {
        let mut sig = Signature::new(TypeBank::new());
        let f_code = sig.insert_id(name, arity, false);
        let type_ = if arity == 0 {
            sig.type_bank().i_type()
        } else {
            let mut args = vec![sig.type_bank().i_type(); usize::try_from(arity).unwrap_or(0)];
            args.push(sig.type_bank().i_type());
            sig.type_bank_mut()
                .insert_type_shared(alloc_arrow_type(args))
        };
        sig.declare_type(f_code, type_).unwrap();
        (TermBank::new(sig).unwrap(), f_code)
    }

    fn bool_arg_bank(outer_name: &str) -> TermBank {
        let mut sig = Signature::new(TypeBank::new());
        sig.insert_internal_codes().unwrap();
        let bool_type = sig.type_bank().bool_type();
        let i_type = sig.type_bank().i_type();
        let outer = sig.insert_id(outer_name, 1, false);
        let outer_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![bool_type, i_type]));
        sig.declare_type(outer, outer_type).unwrap();
        TermBank::new(sig).unwrap()
    }

    fn formula_bank() -> TermBank {
        let mut sig = Signature::new(TypeBank::new());
        sig.insert_internal_codes().unwrap();
        TermBank::new(sig).unwrap()
    }

    fn unary_i_arg_bank(outer_name: &str) -> TermBank {
        let mut sig = Signature::new(TypeBank::new());
        sig.insert_internal_codes().unwrap();
        let i_type = sig.type_bank().i_type();
        let outer = sig.insert_id(outer_name, 1, false);
        let outer_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        sig.declare_type(outer, outer_type).unwrap();
        TermBank::new(sig).unwrap()
    }

    fn declare_i_const(bank: &mut TermBank, name: &str) -> FunCode {
        let i_type = bank.signature().type_bank().i_type();
        let code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(code, i_type)
            .unwrap();
        code
    }

    fn declare_bool_const(bank: &mut TermBank, name: &str) -> FunCode {
        let bool_type = bank.signature().type_bank().bool_type();
        let code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(code, bool_type)
            .unwrap();
        code
    }

    fn declare_unary_i_fun(bank: &mut TermBank, name: &str) -> FunCode {
        let i_type = bank.signature().type_bank().i_type();
        let function_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type]));
        let code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(code, function_type)
            .unwrap();
        code
    }

    fn parse_bool_arg(input: &str) -> (TermBank, Term) {
        let mut bank = bool_arg_bank("takes_bool_arg");
        let mut scanner = Scanner::from_user_string(input, false).unwrap();
        let parsed = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();
        let arg = parsed
            .argument(0)
            .unwrap_or_else(|| panic!("parsed Boolean wrapper has an argument"));
        (bank, arg)
    }

    struct AppliedPrefixFixture {
        bank: TermBank,
        f_code: i64,
        app: Term,
        y: Term,
        b: Term,
        c: Term,
        old: Term,
        repl: Term,
    }

    fn applied_prefix_fixture(prefix: &str) -> AppliedPrefixFixture {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();

        let f_code = sig.insert_id(&format!("{prefix}_f"), 0, false);
        sig.declare_type(f_code, i_type.clone()).unwrap();
        let b_code = sig.insert_id(&format!("{prefix}_b"), 0, false);
        sig.declare_type(b_code, i_type.clone()).unwrap();
        let c_code = sig.insert_id(&format!("{prefix}_c"), 0, false);
        sig.declare_type(c_code, i_type.clone()).unwrap();
        let old_code = sig.insert_id(&format!("{prefix}_old"), 0, false);
        sig.declare_type(old_code, i_type.clone()).unwrap();
        let repl_code = sig.insert_id(&format!("{prefix}_repl"), 0, false);
        sig.declare_type(repl_code, i_type.clone()).unwrap();

        let mut bank = TermBank::new(sig).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let c = bank.create_const_term(c_code).unwrap();
        let old = bank.create_const_term(old_code).unwrap();
        let repl = bank.create_const_term(repl_code).unwrap();

        let y = Term::const_cell_alloc(-4);
        y.set_type(Some(i_type.clone()));
        y.set_binding(Some(b.clone()));
        let z = Term::const_cell_alloc(-6);
        z.set_type(Some(i_type.clone()));
        z.set_binding(Some(c.clone()));

        let head_binding = Term::top_alloc(f_code, 1);
        head_binding.set_type(Some(i_type.clone()));
        head_binding.set_argument(0, y.clone());
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(i_type.clone()));
        head.set_binding(Some(head_binding));

        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(i_type));
        app.set_argument(0, head);
        app.set_argument(1, z);

        AppliedPrefixFixture {
            bank,
            f_code,
            app,
            y,
            b,
            c,
            old,
            repl,
        }
    }

    fn assert_expanded_with_bank_var(fixture: &AppliedPrefixFixture, inserted: &Term) {
        assert_eq!(inserted.f_code(), fixture.f_code);
        assert_eq!(inserted.arity(), 2);
        let prefix = inserted.argument(0).unwrap();
        assert!(prefix.is_free_var());
        assert_eq!(prefix.f_code(), fixture.y.f_code());
        assert!(prefix.binding().is_none());
        assert_ne!(prefix, fixture.b.clone());
        assert_eq!(inserted.argument(1), Some(fixture.c.clone()));
        assert!(inserted.is_shared());
    }

    fn assert_expanded_with_original_var(fixture: &AppliedPrefixFixture, inserted: &Term) {
        assert_eq!(inserted.f_code(), fixture.f_code);
        assert_eq!(inserted.arity(), 2);
        assert_eq!(inserted.argument(0), Some(fixture.y.clone()));
        assert_eq!(inserted.argument(1), Some(fixture.c.clone()));
        assert!(inserted.is_shared());
    }

    fn parse_simple(source: &str) -> (TermBank, Term) {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let term = bank.parse_term_simple(&mut scanner).unwrap();
        (bank, term)
    }

    #[test]
    fn tformula_tstp_parse_lowers_boolean_equality_to_equivalence() {
        let mut bank = formula_bank();
        declare_bool_const(&mut bank, "tstp_formula_bool_left");
        declare_bool_const(&mut bank, "tstp_formula_bool_right");
        let mut scanner = Scanner::from_user_string(
            "(tstp_formula_bool_left) = (tstp_formula_bool_right)",
            false,
        )
        .unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().equiv_code());
        assert_eq!(
            formula.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        let left = formula.argument(0).unwrap();
        let right = formula.argument(1).unwrap();
        assert_eq!(left.f_code(), bank.signature().eqn_code());
        assert_eq!(right.f_code(), bank.signature().eqn_code());
        assert_eq!(left.argument(1), Some(bank.true_term().clone()));
        assert_eq!(right.argument(1), Some(bank.true_term().clone()));
        assert_eq!(
            bank.signature()
                .find_name(left.argument(0).unwrap().f_code()),
            Some("tstp_formula_bool_left")
        );
        assert_eq!(
            bank.signature()
                .find_name(right.argument(0).unwrap().f_code()),
            Some("tstp_formula_bool_right")
        );
    }

    #[test]
    fn tformula_tptp_parse_uses_right_recursive_binary_shape() {
        let mut bank = formula_bank();
        let mut scanner =
            Scanner::from_user_string("tptp_right_p(a)|tptp_right_q(b)&tptp_right_r(c)", false)
                .unwrap();

        let formula = bank.parse_tformula_tptp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().or_code());
        let left = formula.argument(0).unwrap();
        let right = formula.argument(1).unwrap();
        assert_eq!(
            bank.signature()
                .find_name(left.argument(0).unwrap().f_code()),
            Some("tptp_right_p")
        );
        assert_eq!(right.f_code(), bank.signature().and_code());
        assert_eq!(
            bank.signature()
                .find_name(right.argument(0).unwrap().argument(0).unwrap().f_code()),
            Some("tptp_right_q")
        );
        assert_eq!(
            bank.signature()
                .find_name(right.argument(1).unwrap().argument(0).unwrap().f_code()),
            Some("tptp_right_r")
        );
    }

    #[test]
    fn tformula_tptp_parse_quantifier_scope_is_elementary() {
        let mut bank = formula_bank();
        let mut scanner =
            Scanner::from_user_string("![X]:tptp_scope_p(X)|tptp_scope_q(X)", false).unwrap();

        let formula = bank.parse_tformula_tptp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().or_code());
        let quantified = formula.argument(0).unwrap();
        let outside = formula.argument(1).unwrap();
        assert_eq!(quantified.f_code(), bank.signature().qall_code());
        assert_eq!(
            bank.signature().find_name(
                quantified
                    .argument(1)
                    .unwrap()
                    .argument(0)
                    .unwrap()
                    .f_code()
            ),
            Some("tptp_scope_p")
        );
        assert_eq!(
            bank.signature()
                .find_name(outside.argument(0).unwrap().f_code()),
            Some("tptp_scope_q")
        );
    }

    #[test]
    fn tformula_tptp_parse_keeps_formula_equality_as_equality_operator() {
        let mut bank = formula_bank();
        declare_bool_const(&mut bank, "tptp_formula_bool_left");
        declare_bool_const(&mut bank, "tptp_formula_bool_right");
        let mut scanner = Scanner::from_user_string(
            "(tptp_formula_bool_left) = (tptp_formula_bool_right)",
            false,
        )
        .unwrap();

        let formula = bank.parse_tformula_tptp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        assert_eq!(
            formula.argument(0).unwrap().f_code(),
            bank.signature().eqn_code()
        );
        assert_eq!(
            formula.argument(1).unwrap().f_code(),
            bank.signature().eqn_code()
        );
    }

    #[test]
    fn tstp_distinct_parse_allocates_variable_looking_constants_like_c() {
        let mut bank = formula_bank();
        let mut scanner =
            Scanner::from_user_string("$distinct(X,distinct_const_a)", false).unwrap();

        let distinct = bank.parse_tstp_distinct(&mut scanner).unwrap();

        assert_eq!(distinct.f_code(), bank.signature().distinct_code());
        assert_eq!(distinct.arity(), 2);
        let uppercase = distinct.argument(0).unwrap();
        let lowercase = distinct.argument(1).unwrap();
        assert!(!uppercase.is_free_var());
        assert_eq!(bank.signature().find_name(uppercase.f_code()), Some("X"));
        assert_eq!(
            bank.signature().find_name(lowercase.f_code()),
            Some("distinct_const_a")
        );
        assert_eq!(uppercase.type_(), lowercase.type_());
    }

    #[test]
    fn tstp_distinct_parse_rejects_compound_arguments() {
        let mut bank = formula_bank();
        let mut scanner = Scanner::from_user_string("$distinct(f(a),b)", false).unwrap();

        let error = bank.parse_tstp_distinct(&mut scanner).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("constant expected in $distinct argument list"));
    }

    #[test]
    fn tstp_distinct_parse_rejects_mixed_argument_types() {
        let mut bank = formula_bank();
        declare_i_const(&mut bank, "distinct_i_arg");
        declare_bool_const(&mut bank, "distinct_bool_arg");
        let mut scanner =
            Scanner::from_user_string("$distinct(distinct_i_arg,distinct_bool_arg)", false)
                .unwrap();

        let error = bank.parse_tstp_distinct(&mut scanner).unwrap_err();

        assert_eq!(error.code(), ErrorCode::TYPE_ERROR);
        assert!(error
            .message()
            .contains("All $distinct arguments have to be constants of the same type"));
    }

    fn cons_cell(head: Term, tail: Term) -> Term {
        let cons = Term::top_alloc(SIG_CONS_CODE, 2);
        cons.set_argument(0, head);
        cons.set_argument(1, tail);
        cons
    }

    fn list_term(bank: &mut TermBank, elements: &[Term]) -> Term {
        let mut list = bank.create_const_term(SIG_NIL_CODE).unwrap();
        for element in elements.iter().rev() {
            list = bank
                .insert(&cons_cell(element.clone(), list), DerefType::Never)
                .unwrap();
        }
        list
    }

    #[test]
    fn allocation_inserts_true_and_false_constants() {
        let bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();

        assert_eq!(bank.in_count(), 2);
        assert_eq!(bank.insertions(), 2);
        assert_eq!(bank.non_var_term_nodes(), 2);
        assert_eq!(bank.term_nodes(), 2);
        assert!(term_is_true_term(bank.true_term()));
        assert!(term_is_false_term(bank.false_term()));
        assert_eq!(bank.true_term().f_code(), SIG_TRUE_CODE);
        assert_eq!(bank.false_term().f_code(), SIG_FALSE_CODE);
        assert!(bank
            .true_term()
            .query_prop(TP_IS_SHARED | TP_IS_GROUND | TP_PRED_POS));
        assert_eq!(bank.true_term().weight(), DEFAULT_FWEIGHT);
    }

    #[test]
    fn simple_parser_inserts_shared_terms_and_variables() {
        let (bank, parsed) = parse_simple("f(a,X,g(Y))");

        assert_eq!(bank.signature().find_name(parsed.f_code()), Some("f"));
        assert_eq!(parsed.arity(), 3);
        assert!(parsed.is_shared());
        assert!(parsed.query_prop(TP_IS_SHARED));
        assert_eq!(bank.term_string(&parsed, true), "f(a,X1,g(X2))");
        assert_eq!(bank.vars().ext_name_find("X").unwrap().f_code(), -2);
        assert_eq!(bank.vars().ext_name_find("Y").unwrap().f_code(), -4);
    }

    #[test]
    fn simple_parser_allows_distinct_number_and_object_argument_lists_like_c() {
        let (bank, parsed_number) = parse_simple("12(a)");
        assert_eq!(
            bank.signature().find_name(parsed_number.f_code()),
            Some("12")
        );
        assert_eq!(parsed_number.arity(), 1);

        let (bank, parsed_object) = parse_simple("\"obj\"(a)");
        assert_eq!(
            bank.signature().find_name(parsed_object.f_code()),
            Some("\"obj\"")
        );
        assert_eq!(parsed_object.arity(), 1);
    }

    #[test]
    fn checked_parser_rejects_distinct_number_argument_lists() {
        let mut number_bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut number = Scanner::from_user_string("12(a)", false).unwrap();
        let error = number_bank
            .parse_term_with_distinct_checks(&mut number)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Number cannot have argument list"));

        let mut rational_bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut rational = Scanner::from_user_string("3/4(a)", false).unwrap();
        let error = rational_bank
            .parse_term_with_distinct_checks(&mut rational)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("Rational number cannot have argument list"));

        let mut float_bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut float = Scanner::from_user_string("1.5(a)", false).unwrap();
        let error = float_bank
            .parse_term_with_distinct_checks(&mut float)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("Floating point number cannot have argument list"));
    }

    #[test]
    fn checked_parser_rejects_distinct_object_argument_lists() {
        let mut object_bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut object = Scanner::from_user_string("\"obj\"(a)", false).unwrap();
        let error = object_bank
            .parse_term_with_distinct_checks(&mut object)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Object cannot have argument list"));
    }

    #[test]
    fn checked_parser_allows_freed_number_and_object_argument_lists() {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .remove_distinct_props(FP_IS_INTEGER | FP_IS_RATIONAL | FP_IS_FLOAT | FP_IS_OBJECT);
        let mut bank = TermBank::new(signature).unwrap();

        let mut number = Scanner::from_user_string("12(a)", false).unwrap();
        let parsed_number = bank.parse_term_with_distinct_checks(&mut number).unwrap();
        assert_eq!(
            bank.signature().find_name(parsed_number.f_code()),
            Some("12")
        );
        assert_eq!(parsed_number.arity(), 1);

        let mut rational = Scanner::from_user_string("3/4(a)", false).unwrap();
        let parsed_rational = bank.parse_term_with_distinct_checks(&mut rational).unwrap();
        assert_eq!(parsed_rational.arity(), 1);

        let mut float = Scanner::from_user_string("1.5(a)", false).unwrap();
        let parsed_float = bank.parse_term_with_distinct_checks(&mut float).unwrap();
        assert_eq!(parsed_float.arity(), 1);

        let mut object = Scanner::from_user_string("\"obj\"(a)", false).unwrap();
        let parsed_object = bank.parse_term_with_distinct_checks(&mut object).unwrap();
        assert_eq!(
            bank.signature().find_name(parsed_object.f_code()),
            Some("\"obj\"")
        );
        assert_eq!(parsed_object.arity(), 1);
    }

    #[test]
    fn term_debug_string_matches_first_order_and_higher_order_shapes() {
        let (bank, parsed) = parse_simple("f(a,g(b),X)");

        assert_eq!(
            bank.term_debug_string(&parsed, ProblemType::FirstOrder),
            "f(a,g(b),X1)"
        );
        assert_eq!(
            bank.term_debug_string(&parsed, ProblemType::HigherOrder),
            "f a (g b) X1"
        );
    }

    #[test]
    fn ho_term_string_prints_application_surface_with_at_separators() {
        let (bank, parsed) = parse_simple("f(a,g(b),X)");

        assert_eq!(
            bank.term_ho_deref_string(&parsed, DerefType::Never),
            "f @ a @ (g @ b) @ X1"
        );
    }

    #[test]
    fn first_order_list_printing_uses_bracket_notation_when_signature_supports_lists() {
        let mut sig = Signature::new_with_list_support(TypeBank::new(), true);
        let a_code = sig.insert_id("list_print_a", 0, false);
        let b_code = sig.insert_id("list_print_b", 0, false);
        let mut bank = TermBank::new(sig).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let empty = bank.create_const_term(SIG_NIL_CODE).unwrap();
        let list = list_term(&mut bank, &[a, b]);

        assert_eq!(bank.term_string(&empty, true), "[]");
        assert_eq!(bank.term_string(&list, true), "[list_print_a,list_print_b]");
        assert_eq!(
            bank.term_debug_deref_string(&list, ProblemType::FirstOrder, DerefType::Never),
            "[list_print_a,list_print_b]"
        );
    }

    #[test]
    fn first_order_list_printing_keeps_symbols_when_lists_are_not_supported() {
        let mut sig = Signature::new(TypeBank::new());
        let nil_code = sig.insert_id("$nil", 0, true);
        assert_eq!(nil_code, SIG_NIL_CODE);
        let mut bank = TermBank::new(sig).unwrap();
        let nil = bank.create_const_term(SIG_NIL_CODE).unwrap();

        assert_eq!(bank.term_string(&nil, true), "$nil");
    }

    #[test]
    fn first_order_list_printing_dereferences_elements_not_tail_shape() {
        let mut sig = Signature::new_with_list_support(TypeBank::new(), true);
        let a_code = sig.insert_id("list_deref_a", 0, false);
        let mut bank = TermBank::new(sig).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let nil = bank.create_const_term(SIG_NIL_CODE).unwrap();
        let var = Term::const_cell_alloc(-2);
        var.set_binding(Some(a));
        let list = cons_cell(var, nil);

        assert_eq!(
            bank.term_debug_deref_string(&list, ProblemType::FirstOrder, DerefType::Never),
            "[X1]"
        );
        assert_eq!(
            bank.term_debug_deref_string(&list, ProblemType::FirstOrder, DerefType::Always),
            "[list_deref_a]"
        );
    }

    #[test]
    #[should_panic(expected = "C list printing requires a proper $nil tail")]
    fn first_order_list_printing_rejects_improper_tails_like_c_assertion() {
        let mut sig = Signature::new_with_list_support(TypeBank::new(), true);
        let a_code = sig.insert_id("list_bad_tail_a", 0, false);
        let mut bank = TermBank::new(sig).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let list = cons_cell(a.clone(), a);

        let _ = bank.term_string(&list, true);
    }

    #[test]
    fn term_bank_parser_reads_list_literals_as_shared_cons_terms() {
        let mut bank =
            TermBank::new(Signature::new_with_list_support(TypeBank::new(), true)).unwrap();
        let mut scanner = Scanner::from_user_string("[a,X,[b]]", false).unwrap();
        let list = bank.parse_term_simple(&mut scanner).unwrap();

        assert_eq!(bank.term_string(&list, true), "[a,X1,[b]]");
        assert!(list.is_shared());
        assert_eq!(list.f_code(), SIG_CONS_CODE);
        let tail = list.argument(1).unwrap();
        assert!(tail.is_shared());
        assert_eq!(tail.f_code(), SIG_CONS_CODE);
        assert_eq!(tail.argument(1).unwrap().f_code(), SIG_CONS_CODE);
    }

    #[test]
    fn term_bank_parser_reads_empty_list_literal() {
        let mut bank =
            TermBank::new(Signature::new_with_list_support(TypeBank::new(), true)).unwrap();
        let mut scanner = Scanner::from_user_string("[]", false).unwrap();
        let list = bank.parse_term_simple(&mut scanner).unwrap();

        assert_eq!(list.f_code(), SIG_NIL_CODE);
        assert!(list.is_shared());
        assert_eq!(bank.term_string(&list, true), "[]");
    }

    #[test]
    fn checked_term_bank_list_parser_preserves_distinct_argument_diagnostics() {
        let mut bank =
            TermBank::new(Signature::new_with_list_support(TypeBank::new(), true)).unwrap();
        let mut scanner = Scanner::from_user_string("[12(a)]", false).unwrap();
        let error = bank
            .parse_term_with_distinct_checks(&mut scanner)
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Number cannot have argument list"));
    }

    #[test]
    fn checked_parser_rejects_fixed_predicate_in_function_argument_position() {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let outer = sig.insert_id("pred_arg_outer", 1, false);
        let outer_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type]));
        sig.declare_type(outer, outer_type).unwrap();
        let predicate = sig.insert_id("pred_arg_p", 0, false);
        sig.declare_final_type(predicate, bool_type).unwrap();
        let mut bank = TermBank::new(sig).unwrap();

        let mut scanner = Scanner::from_user_string("pred_arg_outer(pred_arg_p)", false).unwrap();
        let error = bank
            .parse_term_with_distinct_checks(&mut scanner)
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("Predicate used as function symbol in preceding term"));
    }

    #[test]
    fn checked_parser_fixes_unfixed_predicate_argument_ambiguity() {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let outer = sig.insert_id("soft_pred_outer", 1, false);
        let outer_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        sig.declare_type(outer, outer_type).unwrap();
        let predicate = sig.insert_id("soft_pred_p", 1, false);
        let predicate_type = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), bool_type.clone()]));
        sig.declare_type(predicate, predicate_type).unwrap();
        let mut bank = TermBank::new(sig).unwrap();

        let mut scanner =
            Scanner::from_user_string("soft_pred_outer(soft_pred_p(soft_pred_a))", false).unwrap();
        let parsed = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();
        let arg = parsed.argument(0).unwrap();

        assert!(bank.signature().is_fixed_type(predicate));
        assert!(bank.signature().is_predicate(predicate));
        assert_eq!(arg.type_(), Some(bool_type));
    }

    #[test]
    fn checked_parser_reads_boolean_truth_constant_arguments() {
        let (bank, true_arg) = parse_bool_arg("takes_bool_arg($true)");
        assert_eq!(true_arg, bank.true_term().clone());

        let (bank, false_arg) = parse_bool_arg("takes_bool_arg($false)");
        assert_eq!(false_arg, bank.false_term().clone());
    }

    #[test]
    fn checked_parser_encodes_boolean_predicate_arguments_like_tformula() {
        let (bank, arg) = parse_bool_arg("takes_bool_arg(pred_bool_arg)");

        assert_eq!(arg.f_code(), bank.signature().eqn_code());
        let left = arg.argument(0).unwrap();
        let right = arg.argument(1).unwrap();
        assert_eq!(
            bank.signature().find_name(left.f_code()),
            Some("pred_bool_arg")
        );
        assert!(bank.signature().is_predicate(left.f_code()));
        assert_eq!(right, bank.true_term().clone());
    }

    #[test]
    fn checked_parser_reads_equality_boolean_arguments() {
        let (bank, arg) = parse_bool_arg("takes_bool_arg(eq_bool_a = eq_bool_b)");

        assert_eq!(arg.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(arg.argument(0).unwrap().f_code()),
            Some("eq_bool_a")
        );
        assert_eq!(
            bank.signature()
                .find_name(arg.argument(1).unwrap().f_code()),
            Some("eq_bool_b")
        );
    }

    #[test]
    fn checked_parser_reads_negated_boolean_formula_arguments() {
        let (bank, arg) = parse_bool_arg("takes_bool_arg(~pred_bool_neg)");

        assert_eq!(arg.f_code(), bank.signature().not_code());
        let child = arg.argument(0).unwrap();
        assert_eq!(child.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(child.argument(0).unwrap().f_code()),
            Some("pred_bool_neg")
        );
    }

    #[test]
    fn checked_parser_reads_binary_boolean_formula_arguments() {
        let (bank, arg) = parse_bool_arg("takes_bool_arg(pred_bool_l & pred_bool_r)");

        assert_eq!(arg.f_code(), bank.signature().and_code());
        let left = arg.argument(0).unwrap();
        let right = arg.argument(1).unwrap();
        assert_eq!(left.f_code(), bank.signature().eqn_code());
        assert_eq!(right.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(left.argument(0).unwrap().f_code()),
            Some("pred_bool_l")
        );
        assert_eq!(
            bank.signature()
                .find_name(right.argument(0).unwrap().f_code()),
            Some("pred_bool_r")
        );
    }

    #[test]
    fn checked_parser_reads_universal_boolean_formula_arguments() {
        let (bank, arg) = parse_bool_arg("takes_bool_arg(![X]: pred_bool_q(X))");

        assert_eq!(arg.f_code(), bank.signature().qall_code());
        let variable = arg.argument(0).unwrap();
        assert!(variable.is_free_var());
        let body = arg.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().eqn_code());
        let predicate = body.argument(0).unwrap();
        assert_eq!(
            bank.signature().find_name(predicate.f_code()),
            Some("pred_bool_q")
        );
        assert_eq!(predicate.argument(0), Some(variable));
        assert_eq!(body.argument(1), Some(bank.true_term().clone()));
    }

    #[test]
    fn checked_parser_reads_nested_existential_boolean_formula_arguments() {
        let (bank, arg) =
            parse_bool_arg("takes_bool_arg(?[X,Y]:(pred_bool_ex(X) | pred_bool_ex(Y)))");

        assert_eq!(arg.f_code(), bank.signature().qex_code());
        let x = arg.argument(0).unwrap();
        let inner = arg.argument(1).unwrap();
        assert_eq!(inner.f_code(), bank.signature().qex_code());
        let y = inner.argument(0).unwrap();
        let body = inner.argument(1).unwrap();

        assert_eq!(body.f_code(), bank.signature().or_code());
        let left = body.argument(0).unwrap();
        let right = body.argument(1).unwrap();
        assert_eq!(
            left.argument(0).unwrap().argument(0),
            Some(x),
            "left predicate keeps the first quantified variable"
        );
        assert_eq!(
            right.argument(0).unwrap().argument(0),
            Some(y),
            "right predicate keeps the second quantified variable"
        );
    }

    #[test]
    fn checked_parser_reads_lambda_boolean_formula_operands() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let (bank, arg) = parse_bool_arg(
            "takes_bool_arg((^[X:$i]: pred_bool_lam_l(X)) = (^[Y:$i]: pred_bool_lam_r(Y)))",
        );

        assert_eq!(arg.f_code(), bank.signature().eqn_code());
        assert_eq!(arg.type_(), Some(bank.signature().type_bank().bool_type()));

        let left = arg.argument(0).unwrap();
        let right = arg.argument(1).unwrap();
        assert_eq!(left.f_code(), SIG_NAMED_LAMBDA_CODE);
        assert_eq!(right.f_code(), SIG_NAMED_LAMBDA_CODE);
        assert_eq!(left.type_(), right.type_());

        let lambda_type = left.type_().unwrap();
        assert!(lambda_type.is_arrow());
        assert_eq!(lambda_type.arity(), 2);
        assert_eq!(lambda_type.args()[0], bank.signature().type_bank().i_type());
        assert_eq!(
            lambda_type.args()[1],
            bank.signature().type_bank().bool_type()
        );

        let binder = left.argument(0).unwrap();
        assert!(binder.is_free_var());
        assert_eq!(binder.type_(), Some(bank.signature().type_bank().i_type()));

        let body = left.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().eqn_code());
        let predicate = body.argument(0).unwrap();
        assert_eq!(
            bank.signature().find_name(predicate.f_code()),
            Some("pred_bool_lam_l")
        );
        assert_eq!(predicate.argument(0), Some(binder));
        assert_eq!(body.argument(1), Some(bank.true_term().clone()));
    }

    #[test]
    fn checked_parser_reads_applied_lambda_boolean_formula_arguments() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let (bank, arg) =
            parse_bool_arg("takes_bool_arg((^[X:$i]: pred_bool_app_lam(X)) @ bool_app_arg)");

        assert_eq!(arg.f_code(), bank.signature().eqn_code());
        assert_eq!(arg.argument(1), Some(bank.true_term().clone()));

        let applied = arg.argument(0).unwrap();
        assert!(applied.is_phony_app());
        assert_eq!(
            applied.type_(),
            Some(bank.signature().type_bank().bool_type())
        );

        let head = applied.argument(0).unwrap();
        assert_eq!(head.f_code(), SIG_NAMED_LAMBDA_CODE);
        let binder = head.argument(0).unwrap();
        assert_eq!(binder.type_(), Some(bank.signature().type_bank().i_type()));

        let body = head.argument(1).unwrap();
        let predicate = body.argument(0).unwrap();
        assert_eq!(
            bank.signature().find_name(predicate.f_code()),
            Some("pred_bool_app_lam")
        );
        assert_eq!(predicate.argument(0), Some(binder));

        let argument = applied.argument(1).unwrap();
        assert_eq!(
            bank.signature().find_name(argument.f_code()),
            Some("bool_app_arg")
        );
        assert_eq!(
            argument.type_(),
            Some(bank.signature().type_bank().i_type())
        );
    }

    #[test]
    fn checked_parser_reads_applied_logical_formula_heads() {
        let (bank, arg) = parse_bool_arg("takes_bool_arg((~) @ pred_bool_app_logical)");

        assert_eq!(arg.f_code(), bank.signature().not_code());
        let child = arg.argument(0).unwrap();
        assert_eq!(child.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(child.argument(0).unwrap().f_code()),
            Some("pred_bool_app_logical")
        );
        assert_eq!(child.argument(1), Some(bank.true_term().clone()));
    }

    #[test]
    fn checked_parser_reads_ite_boolean_formula_arguments() {
        let (bank, arg) =
            parse_bool_arg("takes_bool_arg($ite(pred_ite_cond, pred_ite_then, ~pred_ite_else))");

        assert_eq!(arg.f_code(), bank.signature().eqn_code());
        assert_eq!(arg.argument(1), Some(bank.true_term().clone()));
        let ite = arg.argument(0).unwrap();
        assert_eq!(ite.f_code(), SIG_ITE_CODE);
        assert_eq!(ite.type_(), Some(bank.signature().type_bank().bool_type()));

        let condition = ite.argument(0).unwrap();
        assert_eq!(condition.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(condition.argument(0).unwrap().f_code()),
            Some("pred_ite_cond")
        );

        let true_branch = ite.argument(1).unwrap();
        assert_eq!(true_branch.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(true_branch.argument(0).unwrap().f_code()),
            Some("pred_ite_then")
        );

        let false_branch = ite.argument(2).unwrap();
        assert_eq!(false_branch.f_code(), bank.signature().not_code());
        assert_eq!(
            bank.signature().find_name(
                false_branch
                    .argument(0)
                    .unwrap()
                    .argument(0)
                    .unwrap()
                    .f_code()
            ),
            Some("pred_ite_else")
        );
    }

    #[test]
    fn checked_parser_reads_top_level_ite_terms_like_tbterm_parse_real() {
        let mut bank = bool_arg_bank("takes_bool_arg");
        let mut scanner = Scanner::from_user_string(
            "$ite(pred_top_ite_cond, pred_top_ite_then, pred_top_ite_else)",
            false,
        )
        .unwrap();
        let ite = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();

        assert_eq!(ite.f_code(), SIG_ITE_CODE);
        assert_eq!(ite.type_(), Some(bank.signature().type_bank().bool_type()));
        assert_eq!(
            ite.argument(0).unwrap().f_code(),
            bank.signature().eqn_code()
        );
        assert_eq!(
            ite.argument(1).unwrap().f_code(),
            bank.signature().eqn_code()
        );
        assert_eq!(
            ite.argument(2).unwrap().f_code(),
            bank.signature().eqn_code()
        );
    }

    #[test]
    fn checked_parser_reads_top_level_non_boolean_ite_terms_like_c() {
        let mut bank = unary_i_arg_bank("takes_i_arg");
        let i_type = bank.signature().type_bank().i_type();
        for name in ["ite_i_then", "ite_i_else"] {
            let code = bank.signature_mut().insert_id(name, 0, false);
            bank.signature_mut()
                .declare_final_type(code, i_type.clone())
                .unwrap();
        }
        let mut scanner =
            Scanner::from_user_string("$ite(pred_i_ite_cond, ite_i_then, ite_i_else)", false)
                .unwrap();

        let ite = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();

        assert_eq!(ite.f_code(), SIG_ITE_CODE);
        assert_eq!(ite.type_(), Some(i_type));
        assert_eq!(
            ite.argument(0).unwrap().f_code(),
            bank.signature().eqn_code()
        );
        assert_eq!(
            bank.signature()
                .find_name(ite.argument(1).unwrap().f_code()),
            Some("ite_i_then")
        );
        assert_eq!(
            bank.signature()
                .find_name(ite.argument(2).unwrap().f_code()),
            Some("ite_i_else")
        );
    }

    #[test]
    fn checked_parser_reads_non_boolean_ite_arguments() {
        let mut bank = unary_i_arg_bank("takes_i_arg");
        let i_type = bank.signature().type_bank().i_type();
        for name in ["ite_arg_then", "ite_arg_else"] {
            let code = bank.signature_mut().insert_id(name, 0, false);
            bank.signature_mut()
                .declare_final_type(code, i_type.clone())
                .unwrap();
        }
        let mut scanner = Scanner::from_user_string(
            "takes_i_arg($ite(pred_i_arg_cond, ite_arg_then, ite_arg_else))",
            false,
        )
        .unwrap();

        let parsed = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();
        let arg = parsed.argument(0).unwrap();

        assert_eq!(arg.f_code(), SIG_ITE_CODE);
        assert_eq!(arg.type_(), Some(i_type));
        assert_eq!(
            arg.argument(0).unwrap().f_code(),
            bank.signature().eqn_code()
        );
        assert_eq!(
            bank.signature()
                .find_name(arg.argument(1).unwrap().f_code()),
            Some("ite_arg_then")
        );
        assert_eq!(
            bank.signature()
                .find_name(arg.argument(2).unwrap().f_code()),
            Some("ite_arg_else")
        );
    }

    #[test]
    fn checked_parser_reads_non_boolean_ite_compound_branches() {
        let mut bank = unary_i_arg_bank("takes_i_arg");
        let i_type = bank.signature().type_bank().i_type();
        declare_i_const(&mut bank, "ite_compound_arg_a");
        declare_i_const(&mut bank, "ite_compound_arg_b");
        declare_unary_i_fun(&mut bank, "ite_compound_then");
        declare_unary_i_fun(&mut bank, "ite_compound_else");
        let mut scanner = Scanner::from_user_string(
            concat!(
                "takes_i_arg($ite(",
                "pred_i_compound_cond,",
                "ite_compound_then(ite_compound_arg_a),",
                "ite_compound_else(ite_compound_arg_b)",
                "))"
            ),
            false,
        )
        .unwrap();

        let parsed = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();
        let ite = parsed.argument(0).unwrap();

        assert_eq!(ite.f_code(), SIG_ITE_CODE);
        assert_eq!(ite.type_(), Some(i_type));
        assert_eq!(
            ite.argument(0).unwrap().f_code(),
            bank.signature().eqn_code()
        );
        let true_branch = ite.argument(1).unwrap();
        assert_eq!(
            bank.signature().find_name(true_branch.f_code()),
            Some("ite_compound_then")
        );
        assert_eq!(
            bank.signature()
                .find_name(true_branch.argument(0).unwrap().f_code()),
            Some("ite_compound_arg_a")
        );
        let false_branch = ite.argument(2).unwrap();
        assert_eq!(
            bank.signature().find_name(false_branch.f_code()),
            Some("ite_compound_else")
        );
        assert_eq!(
            bank.signature()
                .find_name(false_branch.argument(0).unwrap().f_code()),
            Some("ite_compound_arg_b")
        );
    }

    #[test]
    fn checked_parser_reads_top_level_boolean_let_terms_like_tbterm_parse_real() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = bool_arg_bank("takes_bool_arg");
        let mut scanner =
            Scanner::from_user_string("$let(f:$o, f := pred_let_value, f)", false).unwrap();
        let let_term = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();

        assert_eq!(let_term.f_code(), SIG_LET_CODE);
        assert_eq!(
            let_term.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        assert_eq!(let_term.arity(), 2);

        let definition = let_term.argument(0).unwrap();
        assert_eq!(definition.f_code(), bank.signature().eqn_code());
        let defined_head = definition.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(defined_head.f_code()), Some("f"));
        assert_eq!(
            defined_head.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        let rhs = definition.argument(1).unwrap();
        assert_eq!(rhs.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(rhs.argument(0).unwrap().f_code()),
            Some("pred_let_value")
        );

        let body = let_term.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().eqn_code());
        assert_eq!(body.argument(0).unwrap().f_code(), defined_head.f_code());
        assert_eq!(body.argument(1), Some(bank.true_term().clone()));
    }

    #[test]
    fn checked_parser_reads_parameterized_boolean_let_terms() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = bool_arg_bank("takes_bool_arg");
        let mut scanner = Scanner::from_user_string(
            "$let(f:$i>$o, f(X) := pred_let_param(X), f(let_arg_a))",
            false,
        )
        .unwrap();
        let let_term = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();

        assert_eq!(let_term.f_code(), SIG_LET_CODE);
        let definition = let_term.argument(0).unwrap();
        let defined_head = definition.argument(0).unwrap();
        let local_f_code = defined_head.f_code();
        assert_eq!(defined_head.arity(), 1);
        let variable = defined_head.argument(0).unwrap();
        assert!(variable.is_free_var());

        let rhs = definition.argument(1).unwrap();
        let rhs_predicate = rhs.argument(0).unwrap();
        assert_eq!(
            bank.signature().find_name(rhs_predicate.f_code()),
            Some("pred_let_param")
        );
        assert_eq!(rhs_predicate.argument(0), Some(variable));

        let body = let_term.argument(1).unwrap();
        let body_head = body.argument(0).unwrap();
        assert_eq!(body_head.f_code(), local_f_code);
        assert_eq!(
            bank.signature()
                .find_name(body_head.argument(0).unwrap().f_code()),
            Some("let_arg_a")
        );
    }

    #[test]
    fn checked_parser_reads_top_level_non_boolean_let_terms_like_c() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = unary_i_arg_bank("takes_i_arg");
        let value_code = declare_i_const(&mut bank, "let_i_value");
        let mut scanner =
            Scanner::from_user_string("$let(f:$i, f := let_i_value, f)", false).unwrap();

        let let_term = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();

        assert_eq!(let_term.f_code(), SIG_LET_CODE);
        assert_eq!(
            let_term.type_(),
            Some(bank.signature().type_bank().i_type())
        );
        assert_eq!(let_term.arity(), 2);

        let definition = let_term.argument(0).unwrap();
        assert_eq!(definition.f_code(), bank.signature().eqn_code());
        let defined_head = definition.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(defined_head.f_code()), Some("f"));
        assert_eq!(
            defined_head.type_(),
            Some(bank.signature().type_bank().i_type())
        );
        assert_eq!(definition.argument(1).unwrap().f_code(), value_code);

        let body = let_term.argument(1).unwrap();
        assert_eq!(body.f_code(), defined_head.f_code());
        assert_eq!(body.type_(), Some(bank.signature().type_bank().i_type()));
    }

    #[test]
    fn checked_parser_reads_parameterized_non_boolean_let_terms() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = unary_i_arg_bank("takes_i_arg");
        declare_i_const(&mut bank, "let_i_arg");
        declare_unary_i_fun(&mut bank, "let_i_h");
        let mut scanner =
            Scanner::from_user_string("$let(f:$i>$i, f(X) := let_i_h(X), f(let_i_arg))", false)
                .unwrap();

        let let_term = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();

        assert_eq!(let_term.f_code(), SIG_LET_CODE);
        assert_eq!(
            let_term.type_(),
            Some(bank.signature().type_bank().i_type())
        );
        let definition = let_term.argument(0).unwrap();
        let defined_head = definition.argument(0).unwrap();
        let local_f_code = defined_head.f_code();
        assert_eq!(defined_head.arity(), 1);
        let variable = defined_head.argument(0).unwrap();
        assert!(variable.is_free_var());

        let rhs = definition.argument(1).unwrap();
        assert_eq!(bank.signature().find_name(rhs.f_code()), Some("let_i_h"));
        assert_eq!(rhs.argument(0), Some(variable));

        let body = let_term.argument(1).unwrap();
        assert_eq!(body.f_code(), local_f_code);
        assert_eq!(
            bank.signature()
                .find_name(body.argument(0).unwrap().f_code()),
            Some("let_i_arg")
        );
    }

    #[test]
    fn checked_parser_encodes_boolean_let_formula_arguments() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = bool_arg_bank("takes_bool_arg");
        let mut scanner =
            Scanner::from_user_string("takes_bool_arg($let(f:$o, f := pred_let_arg, f))", false)
                .unwrap();
        let parsed = bank.parse_term_with_distinct_checks(&mut scanner).unwrap();
        let arg = parsed.argument(0).unwrap();

        assert_eq!(arg.f_code(), bank.signature().eqn_code());
        assert_eq!(arg.argument(1), Some(bank.true_term().clone()));
        let let_term = arg.argument(0).unwrap();
        assert_eq!(let_term.f_code(), SIG_LET_CODE);
        assert_eq!(
            let_term.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
    }

    #[test]
    fn checked_parser_rejects_non_function_let_declaration_like_c() {
        let mut bank = bool_arg_bank("takes_bool_arg");
        let mut scanner = Scanner::from_user_string("$let(X:$o, X := p, X)", false).unwrap();
        let error = bank
            .parse_term_with_distinct_checks(&mut scanner)
            .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("let declaration expects a function symbol"));
    }

    #[test]
    fn tstp_formula_application_uses_thf_declared_predicate_type() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations =
            Scanner::from_user_string("person: $tType. a: person. p: person > $o.", false).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("p @ a", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        assert_eq!(
            formula.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
    }

    #[test]
    fn tstp_formula_application_accepts_parenthesized_predicate_head() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations =
            Scanner::from_user_string("person: $tType. a: person. p: person > $o.", false).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("(p) @ a", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        assert_eq!(
            formula.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        let applied = formula.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(applied.f_code()), Some("p"));
        assert_eq!(applied.arity(), 1);
    }

    #[test]
    fn tstp_formula_application_can_be_equality_operand_under_first_order_global_state() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. b: person. f: person > person.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("f @ a = b", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        assert_eq!(
            formula.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        let left = formula.argument(0).unwrap();
        assert_eq!(left.arity(), 1);
        assert_eq!(bank.signature().find_name(left.f_code()), Some("f"));
    }

    #[test]
    fn tstp_formula_application_can_be_right_equality_operand() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. b: person. f: person > person.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("b = f @ a", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        assert_eq!(
            formula.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        let right = formula.argument(1).unwrap();
        assert_eq!(right.arity(), 1);
        assert_eq!(bank.signature().find_name(right.f_code()), Some("f"));
    }

    #[test]
    fn tstp_quantified_formula_application_uses_thf_declared_predicate_type() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations =
            Scanner::from_user_string("person: $tType. p: person > $o.", false).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("![X: person]: p @ X", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().qall_code());
        assert_eq!(
            formula.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        let body = formula.argument(1).unwrap();
        assert_eq!(body.f_code(), bank.signature().eqn_code());
    }

    #[test]
    fn tstp_negated_formula_application_binds_application_before_negation() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations =
            Scanner::from_user_string("person: $tType. a: person. p: person > $o.", false).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("~ p @ a", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().not_code());
        assert_eq!(
            formula.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        let child = formula.argument(0).unwrap();
        assert_eq!(child.f_code(), bank.signature().eqn_code());
    }

    #[test]
    fn tstp_formula_application_accepts_nested_non_boolean_application_argument() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. f: person > person. p: person > $o.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("p @ (f @ a)", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        let predicate = formula.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(predicate.f_code()), Some("p"));
        let nested = predicate.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(nested.f_code()), Some("f"));
        assert_eq!(nested.arity(), 1);
        assert_eq!(nested.type_(), nested.argument(0).unwrap().type_());
    }

    #[test]
    fn tstp_formula_application_preserves_left_association_for_arrow_arguments() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. b: person. f: person > person. appfun: (person > person) > person > person.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("appfun @ f @ a = b", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        let left = formula.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(left.f_code()), Some("appfun"));
        assert_eq!(left.arity(), 2);
        assert_eq!(
            bank.signature()
                .find_name(left.argument(0).unwrap().f_code()),
            Some("f")
        );
        assert_eq!(
            bank.signature()
                .find_name(left.argument(1).unwrap().f_code()),
            Some("a")
        );
        assert_eq!(
            bank.signature()
                .find_name(formula.argument(1).unwrap().f_code()),
            Some("b")
        );
    }

    #[test]
    fn tstp_formula_application_accepts_parenthesized_applied_head() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. b: person. f: person > person. appfun: (person > person) > person > person.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("(appfun @ f) @ a = b", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        let left = formula.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(left.f_code()), Some("appfun"));
        assert_eq!(left.arity(), 2);
        assert_eq!(
            bank.signature()
                .find_name(left.argument(0).unwrap().f_code()),
            Some("f")
        );
        assert_eq!(
            bank.signature()
                .find_name(left.argument(1).unwrap().f_code()),
            Some("a")
        );
        assert_eq!(
            bank.signature()
                .find_name(formula.argument(1).unwrap().f_code()),
            Some("b")
        );
    }

    #[test]
    fn tstp_formula_application_accepts_parenthesized_boolean_application_argument() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. p: person > $o. r: $o > $o.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("r @ (p @ a)", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().eqn_code());
        let applied_r = formula.argument(0).unwrap();
        assert_eq!(applied_r.arity(), 1);
        assert_eq!(bank.signature().find_name(applied_r.f_code()), Some("r"));
        let encoded_p = applied_r.argument(0).unwrap();
        assert_eq!(encoded_p.f_code(), bank.signature().eqn_code());
        let applied_p = encoded_p.argument(0).unwrap();
        assert_eq!(applied_p.arity(), 1);
        assert_eq!(bank.signature().find_name(applied_p.f_code()), Some("p"));
    }

    #[test]
    fn tstp_formula_application_preserves_left_association_for_boolean_arguments() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. p: person > $o. r: $o > $o.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("r @ p @ a", false).unwrap();

        let error = bank.parse_tformula_tstp(&mut scanner).unwrap_err();

        assert_eq!(error.code(), ErrorCode::TYPE_ERROR);
        assert!(error
            .message()
            .contains("formula application argument has the wrong sort"));
    }

    #[test]
    fn tstp_formula_application_accepts_binary_logical_formula_head() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. p: person > $o. q: person > $o.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("(&) @ (p @ a) @ (q @ a)", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().and_code());
        assert_eq!(
            formula.type_(),
            Some(bank.signature().type_bank().bool_type())
        );
        let left = formula.argument(0).unwrap();
        assert_eq!(left.f_code(), bank.signature().eqn_code());
        let applied_p = left.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(applied_p.f_code()), Some("p"));
        let right = formula.argument(1).unwrap();
        assert_eq!(right.f_code(), bank.signature().eqn_code());
        let applied_q = right.argument(0).unwrap();
        assert_eq!(bank.signature().find_name(applied_q.f_code()), Some("q"));
    }

    #[test]
    fn tstp_formula_parser_accepts_mixed_application_conjunction_implication() {
        let _problem_type = set_problem_type_for_test(ProblemType::FirstOrder);
        let mut bank = formula_bank();
        let mut declarations = Scanner::from_user_string(
            "person: $tType. a: person. p: person > $o. q: person > $o. r: person > $o.",
            false,
        )
        .unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        bank.signature_mut()
            .parse_tff_type_declaration(&mut declarations, ProblemType::HigherOrder)
            .unwrap();
        declarations.accept_tok(TokenType::FULLSTOP).unwrap();
        let mut scanner = Scanner::from_user_string("p @ a & q @ a => r @ a", false).unwrap();

        let formula = bank.parse_tformula_tstp(&mut scanner).unwrap();

        assert_eq!(formula.f_code(), bank.signature().impl_code());
        let antecedent = formula.argument(0).unwrap();
        assert_eq!(antecedent.f_code(), bank.signature().and_code());
        let left = antecedent.argument(0).unwrap();
        assert_eq!(left.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(left.argument(0).unwrap().f_code()),
            Some("p")
        );
        let right = antecedent.argument(1).unwrap();
        assert_eq!(right.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(right.argument(0).unwrap().f_code()),
            Some("q")
        );
        let consequent = formula.argument(1).unwrap();
        assert_eq!(consequent.f_code(), bank.signature().eqn_code());
        assert_eq!(
            bank.signature()
                .find_name(consequent.argument(0).unwrap().f_code()),
            Some("r")
        );
        assert!(scanner.test_tok(TokenType::NO_TOKEN));
    }

    #[test]
    fn ho_term_string_skips_hidden_phony_application_head() {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let arrow = sig
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let head_code = sig.insert_id("ho_app_f", 0, false);
        sig.declare_type(head_code, arrow).unwrap();
        let arg_code = sig.insert_id("ho_app_a", 0, false);
        sig.declare_type(arg_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let head = bank.create_const_term(head_code).unwrap();
        let arg = bank.create_const_term(arg_code).unwrap();
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(i_type));
        app.set_argument(0, head);
        app.set_argument(1, arg);

        assert_eq!(
            bank.term_ho_deref_string(&app, DerefType::Never),
            "ho_app_f @ ho_app_a"
        );
    }

    #[test]
    fn ho_term_string_prints_db_variables_like_c_depth_formula() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let i_type = bank.signature().type_bank().i_type();
        let db = bank.request_db_var(&i_type, 0);

        assert_eq!(bank.term_ho_deref_string(&db, DerefType::Never), "Z-1");
    }

    #[test]
    fn ho_term_string_prints_non_formula_ite_shape() {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let a_code = sig.insert_id("ite_a", 0, false);
        let b_code = sig.insert_id("ite_b", 0, false);
        let c_code = sig.insert_id("ite_c", 0, false);
        sig.declare_type(a_code, i_type.clone()).unwrap();
        sig.declare_type(b_code, i_type.clone()).unwrap();
        sig.declare_type(c_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let ite = Term::top_alloc(SIG_ITE_CODE, 3);
        ite.set_type(Some(i_type));
        ite.set_argument(0, bank.create_const_term(a_code).unwrap());
        ite.set_argument(1, bank.create_const_term(b_code).unwrap());
        ite.set_argument(2, bank.create_const_term(c_code).unwrap());

        assert_eq!(
            bank.term_ho_deref_string(&ite, DerefType::Never),
            "$ite(ite_a, ite_b, ite_c)"
        );
    }

    #[test]
    fn debug_deref_string_follows_ordinary_bindings() {
        let (bank, binding) = parse_simple("f(a)");
        let var = Term::const_cell_alloc(-2);
        var.set_binding(Some(binding));

        assert_eq!(
            bank.term_debug_deref_string(&var, ProblemType::FirstOrder, DerefType::Once),
            "f(a)"
        );
        assert_eq!(
            bank.term_debug_deref_string(&var, ProblemType::HigherOrder, DerefType::Once),
            "f a"
        );
        assert_eq!(bank.term_debug_string(&var, ProblemType::HigherOrder), "X1");
    }

    #[test]
    fn ho_debug_deref_parenthesizes_bound_function_arguments() {
        let mut bank = TermBank::new(Signature::new(TypeBank::new())).unwrap();
        let mut root_scanner = Scanner::from_user_string("h(X)", false).unwrap();
        let root = bank.parse_term_simple(&mut root_scanner).unwrap();
        let mut binding_scanner = Scanner::from_user_string("g(a)", false).unwrap();
        let binding = bank.parse_term_simple(&mut binding_scanner).unwrap();
        let var = bank.vars().ext_name_find("X").unwrap();
        var.set_binding(Some(binding));

        assert_eq!(
            bank.term_debug_deref_string(&root, ProblemType::HigherOrder, DerefType::Once),
            "h (g a)"
        );
    }

    #[test]
    fn ho_debug_deref_expands_applied_free_vars_with_prefix_limit() {
        let fixture = applied_prefix_fixture("debug_app_deref");

        assert_eq!(
            fixture.bank.term_debug_deref_string(
                &fixture.app,
                ProblemType::HigherOrder,
                DerefType::Once,
            ),
            "debug_app_deref_f X2 debug_app_deref_c"
        );
    }

    #[test]
    fn ho_term_deref_expands_applied_free_vars_with_prefix_limit() {
        let fixture = applied_prefix_fixture("print_app_deref");

        assert_eq!(
            fixture
                .bank
                .term_ho_deref_string(&fixture.app, DerefType::Once),
            "print_app_deref_f @ X2 @ print_app_deref_c"
        );
    }

    #[test]
    fn simple_parser_treats_uppercase_application_as_function_symbol() {
        let (bank, parsed) = parse_simple("F(a)");

        assert_eq!(bank.signature().find_name(parsed.f_code()), Some("F"));
        assert_eq!(parsed.arity(), 1);
        assert!(parsed.argument(0).unwrap().is_shared());
    }

    #[test]
    fn simple_parser_accepts_empty_argument_lists_as_constants() {
        let (bank, parsed) = parse_simple("f()");

        assert_eq!(bank.signature().find_name(parsed.f_code()), Some("f"));
        assert_eq!(parsed.arity(), 0);
        assert!(parsed.is_shared());
    }

    #[test]
    fn insert_recursively_shares_non_variable_terms_and_computes_counts() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = Term::const_cell_alloc(a_code);
        let f = Term::top_alloc(f_code, 1);
        f.set_argument(0, a);

        let shared = bank.insert(&f, DerefType::Never).unwrap();

        assert!(shared.query_prop(TP_IS_SHARED | TP_IS_GROUND));
        assert_eq!(shared.v_count(), 0);
        assert_eq!(shared.f_count(), 2);
        assert_eq!(shared.weight(), DEFAULT_FWEIGHT * 2);
        assert!(shared.argument(0).unwrap().is_shared());
        assert_eq!(bank.in_count(), 4);
        assert_eq!(bank.term_nodes(), 4);
        assert_eq!(bank.find(&shared), Some(shared.clone()));
        assert_eq!(tb_cell_ident(&shared), shared.entry_no());
        assert!(tb_term_is_ground(&shared));
    }

    #[test]
    fn duplicate_top_insertion_reuses_existing_cell_and_merges_properties() {
        let (mut bank, a_code) = bank_with_symbol("a", 0);
        let first = Term::const_cell_alloc(a_code);
        first.set_prop(TP_PRED_POS);
        let shared_first = bank.insert(&first, DerefType::Never).unwrap();
        let duplicate = Term::const_cell_alloc(a_code);

        let shared_duplicate = bank.insert(&duplicate, DerefType::Never).unwrap();

        assert_eq!(shared_first, shared_duplicate);
        assert_eq!(bank.in_count(), 3);
        assert_eq!(bank.non_var_term_nodes(), 3);
        assert!(shared_duplicate.query_prop(TP_PRED_POS));
    }

    #[test]
    fn insert_uses_bank_variables_unless_asked_to_keep_variables() {
        let (mut bank, _f_code) = bank_with_symbol("a", 0);
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(bank.signature().type_bank().i_type()));

        let shared = bank.insert(&var, DerefType::Never).unwrap();
        let kept = bank.insert_ignore_var(&var, DerefType::Never).unwrap();

        assert_ne!(shared, var);
        assert_eq!(kept, var);
        assert_eq!(shared.f_code(), -2);
        assert_eq!(tb_cell_ident(&shared), -2);
        assert_eq!(bank.term_nodes(), 3);
    }

    #[test]
    fn cached_no_property_insertion_reuses_duplicate_source_subterms() {
        let (mut bank, f_code) = bank_with_symbol("f", 2);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = Term::const_cell_alloc(a_code);
        a.set_prop(TP_CHECK_FLAG);
        let root = Term::top_alloc(f_code, 2);
        root.set_f_count(INSERT_NO_PROPS_CACHE_THRESHOLD + 1);
        root.set_prop(TP_CHECK_FLAG);
        root.set_argument(0, a.clone());
        root.set_argument(1, a);

        let inserted = bank
            .insert_no_props_cached(&root, DerefType::Never)
            .unwrap();
        let left = inserted.argument(0).unwrap();
        let right = inserted.argument(1).unwrap();

        assert_eq!(left, right);
        assert!(left.is_shared());
        assert!(!left.query_prop(TP_CHECK_FLAG));
        assert!(!inserted.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn recursive_insertion_expands_applied_deref_with_prefix_limit() {
        let mut fixture = applied_prefix_fixture("insert_app_deref");
        let app = fixture.app.clone();
        let inserted = fixture.bank.insert(&app, DerefType::Once).unwrap();
        assert_expanded_with_bank_var(&fixture, &inserted);

        let mut fixture = applied_prefix_fixture("insert_ignore_app_deref");
        let app = fixture.app.clone();
        let inserted = fixture
            .bank
            .insert_ignore_var(&app, DerefType::Once)
            .unwrap();
        assert_expanded_with_original_var(&fixture, &inserted);

        let mut fixture = applied_prefix_fixture("insert_no_props_app_deref");
        let app = fixture.app.clone();
        let inserted = fixture.bank.insert_no_props(&app, DerefType::Once).unwrap();
        assert_expanded_with_bank_var(&fixture, &inserted);
    }

    #[test]
    fn cached_no_props_expands_applied_deref_with_prefix_limit() {
        let mut fixture = applied_prefix_fixture("cached_app_deref");
        fixture.app.set_f_count(INSERT_NO_PROPS_CACHE_THRESHOLD + 1);
        let app = fixture.app.clone();
        let inserted = fixture
            .bank
            .insert_no_props_cached(&app, DerefType::Once)
            .unwrap();

        assert_expanded_with_bank_var(&fixture, &inserted);
    }

    #[test]
    fn optimized_insertion_reuses_shared_ground_terms_and_follows_bindings() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(a_code, i_type.clone())
            .unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let before = bank.term_nodes();

        assert_eq!(bank.insert_opt(&a, DerefType::Never).unwrap(), a);
        assert_eq!(bank.term_nodes(), before);

        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        var.set_binding(Some(a.clone()));
        assert_eq!(bank.insert_opt(&var, DerefType::Always).unwrap(), a);

        let unshared = Term::top_alloc(f_code, 1);
        unshared.set_argument(0, var);
        let inserted = bank.insert_opt(&unshared, DerefType::Always).unwrap();
        assert_eq!(inserted.argument(0), Some(a));
        assert!(inserted.is_shared());
    }

    #[test]
    fn optimized_insertion_expands_applied_deref_with_prefix_limit() {
        let mut fixture = applied_prefix_fixture("opt_app_deref");
        let app = fixture.app.clone();
        let inserted = fixture.bank.insert_opt(&app, DerefType::Once).unwrap();

        assert_expanded_with_bank_var(&fixture, &inserted);
    }

    #[test]
    fn replacement_insertions_clear_new_top_properties_and_skip_plain_noops() {
        let (mut bank, f_code) = bank_with_symbol("f", 2);
        let old_code = bank.signature_mut().insert_id("old", 0, false);
        let repl_code = bank.signature_mut().insert_id("repl", 0, false);
        let keep_code = bank.signature_mut().insert_id("keep", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(old_code, i_type.clone())
            .unwrap();
        bank.signature_mut()
            .declare_type(repl_code, i_type.clone())
            .unwrap();
        bank.signature_mut()
            .declare_type(keep_code, i_type)
            .unwrap();
        let old = bank.create_const_term(old_code).unwrap();
        let repl = bank.create_const_term(repl_code).unwrap();
        let keep = bank.create_const_term(keep_code).unwrap();
        let root = Term::top_alloc(f_code, 2);
        root.set_prop(TP_CHECK_FLAG);
        root.set_argument(0, old.clone());
        root.set_argument(1, keep.clone());

        let instantiated = bank
            .insert_repl(&root, DerefType::Never, &old, &repl)
            .unwrap();
        assert_eq!(instantiated.argument(0), Some(repl.clone()));
        assert_eq!(instantiated.argument(1), Some(keep.clone()));
        assert!(!instantiated.query_prop(TP_CHECK_FLAG));

        let shared_root = bank.insert_no_props(&root, DerefType::Never).unwrap();
        let plain = bank.insert_repl_plain(&shared_root, &old, &repl).unwrap();
        assert_eq!(plain.argument(0), Some(repl));
        assert_eq!(plain.argument(1), Some(keep.clone()));
        assert!(plain.is_shared());

        let no_change = bank.insert_repl_plain(&keep, &old, &plain).unwrap();
        assert_eq!(no_change, keep);
    }

    #[test]
    fn replacement_insertion_expands_applied_deref_with_prefix_limit() {
        let mut fixture = applied_prefix_fixture("repl_app_deref");
        let app = fixture.app.clone();
        let old = fixture.old.clone();
        let repl = fixture.repl.clone();
        let inserted = fixture
            .bank
            .insert_repl(&app, DerefType::Once, &old, &repl)
            .unwrap();

        assert_expanded_with_bank_var(&fixture, &inserted);
    }

    #[test]
    fn instantiated_insertions_reuse_bound_and_ground_bank_terms() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(a_code, i_type.clone())
            .unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        var.set_binding(Some(a.clone()));

        assert_eq!(
            bank.insert_instantiated_deref(&var, DerefType::Never)
                .unwrap(),
            var
        );
        assert_eq!(
            bank.insert_instantiated_deref(&var, DerefType::Once)
                .unwrap(),
            a
        );

        let root = Term::top_alloc(f_code, 1);
        root.set_prop(TP_CHECK_FLAG);
        root.set_argument(0, var.clone());
        let derefed = bank
            .insert_instantiated_deref(&root, DerefType::Once)
            .unwrap();
        assert_eq!(derefed.argument(0), Some(a.clone()));
        assert!(!derefed.query_prop(TP_CHECK_FLAG));

        let fo = bank.insert_instantiated_fo(&root).unwrap();
        assert_eq!(fo.argument(0), Some(a.clone()));
        assert!(!fo.query_prop(TP_CHECK_FLAG));
        assert_eq!(bank.insert_instantiated_fo(&a).unwrap(), a);
    }

    #[test]
    fn instantiated_deref_expands_applied_function_bindings() {
        let mut type_bank = TypeBank::new();
        let i_type = type_bank.i_type();
        let arrow =
            type_bank.insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let mut sig = Signature::new(type_bank);
        let g_code = sig.insert_id("inst_deref_g", 0, false);
        sig.declare_type(g_code, arrow.clone()).unwrap();
        let b_code = sig.insert_id("inst_deref_b", 0, false);
        sig.declare_type(b_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let binding = Term::const_cell_alloc(g_code);
        binding.set_type(Some(arrow.clone()));
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(arrow));
        head.set_binding(Some(binding));
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(i_type));
        app.set_argument(0, head);
        app.set_argument(1, b.clone());

        let expanded = bank
            .insert_instantiated_deref(&app, DerefType::Once)
            .unwrap();

        assert_eq!(expanded.f_code(), g_code);
        assert_eq!(expanded.arity(), 1);
        assert_eq!(expanded.argument(0), Some(b));
        assert!(expanded.is_shared());
    }

    #[test]
    fn instantiated_deref_preserves_ignored_bound_prefix_args() {
        let mut sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let f_code = sig.insert_id("inst_deref_prefix_f", 0, false);
        sig.declare_type(f_code, i_type.clone()).unwrap();
        let b_code = sig.insert_id("inst_deref_prefix_b", 0, false);
        sig.declare_type(b_code, i_type.clone()).unwrap();
        let c_code = sig.insert_id("inst_deref_prefix_c", 0, false);
        sig.declare_type(c_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let c = bank.create_const_term(c_code).unwrap();
        let y = Term::const_cell_alloc(-4);
        y.set_type(Some(i_type.clone()));
        y.set_binding(Some(b));
        let z = Term::const_cell_alloc(-6);
        z.set_type(Some(i_type.clone()));
        z.set_binding(Some(c.clone()));
        let head_binding = Term::top_alloc(f_code, 1);
        head_binding.set_type(Some(i_type.clone()));
        head_binding.set_argument(0, y.clone());
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(i_type.clone()));
        head.set_binding(Some(head_binding));
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(i_type));
        app.set_argument(0, head);
        app.set_argument(1, z);

        let expanded = bank
            .insert_instantiated_deref(&app, DerefType::Once)
            .unwrap();

        assert_eq!(expanded.f_code(), f_code);
        assert_eq!(expanded.arity(), 2);
        assert_eq!(expanded.argument(0), Some(y));
        assert_eq!(expanded.argument(1), Some(c));
        assert!(expanded.is_shared());
    }

    #[test]
    fn higher_order_instantiated_insertion_shares_unshared_bindings() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(a_code, i_type.clone())
            .unwrap();
        let unshared_a = Term::const_cell_alloc(a_code);
        unshared_a.set_type(Some(i_type.clone()));
        let unshared_binding = Term::top_alloc(f_code, 1);
        unshared_binding.set_type(Some(i_type.clone()));
        unshared_binding.set_argument(0, unshared_a);
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        var.set_binding(Some(unshared_binding.clone()));

        let shared_binding = bank.insert_instantiated_ho(&var, true).unwrap();

        assert!(shared_binding.is_shared());
        assert_eq!(shared_binding.f_code(), f_code);
        assert!(shared_binding.argument(0).unwrap().is_shared());
        assert_eq!(bank.find(&shared_binding), Some(shared_binding.clone()));

        let kept = bank.insert_instantiated_ho(&var, false).unwrap();
        assert_eq!(kept, var);
    }

    #[test]
    fn instantiated_problem_type_wrapper_dispatches_to_higher_order_path() {
        let (mut bank, _f_code) = bank_with_symbol("holder", 0);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(a_code, i_type.clone())
            .unwrap();
        let unshared_a = Term::const_cell_alloc(a_code);
        unshared_a.set_type(Some(i_type.clone()));
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        var.set_binding(Some(unshared_a));

        let shared = bank
            .insert_instantiated_for_problem(&var, ProblemType::HigherOrder)
            .unwrap();

        assert!(shared.is_shared());
        assert_eq!(shared.f_code(), a_code);
    }

    #[test]
    fn higher_order_instantiated_insertion_expands_applied_function_bindings() {
        let mut type_bank = TypeBank::new();
        let i_type = type_bank.i_type();
        let arrow =
            type_bank.insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let mut sig = Signature::new(type_bank);
        let g_code = sig.insert_id("g", 0, false);
        sig.declare_type(g_code, arrow.clone()).unwrap();
        let b_code = sig.insert_id("b", 0, false);
        sig.declare_type(b_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let binding = Term::const_cell_alloc(g_code);
        binding.set_type(Some(arrow.clone()));
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(arrow));
        head.set_binding(Some(binding));
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(i_type));
        app.set_argument(0, head);
        app.set_argument(1, b.clone());

        let expanded = bank.insert_instantiated_ho(&app, true).unwrap();

        assert_eq!(expanded.f_code(), g_code);
        assert_eq!(expanded.arity(), 1);
        assert_eq!(expanded.argument(0), Some(b));
        assert!(expanded.is_shared());
    }

    #[test]
    fn higher_order_applied_expansion_preserves_ignored_bound_prefix_args() {
        let mut type_bank = TypeBank::new();
        let i_type = type_bank.i_type();
        let arrow =
            type_bank.insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let mut sig = Signature::new(type_bank);
        let b_code = sig.insert_id("b", 0, false);
        let c_code = sig.insert_id("c", 0, false);
        sig.declare_type(b_code, i_type.clone()).unwrap();
        sig.declare_type(c_code, arrow.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let c = bank.create_const_term(c_code).unwrap();
        let ignored_head = Term::const_cell_alloc(-4);
        ignored_head.set_type(Some(arrow.clone()));
        ignored_head.set_binding(Some(c));
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(arrow.clone()));
        head.set_binding(Some(ignored_head.clone()));
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_type(Some(i_type));
        app.set_argument(0, head);
        app.set_argument(1, b.clone());

        let expanded = bank.insert_instantiated_ho(&app, true).unwrap();

        assert!(expanded.is_phony_app());
        assert_eq!(expanded.argument(0), Some(ignored_head));
        assert_eq!(expanded.argument(1), Some(b));
    }

    #[test]
    fn disjoint_insertion_maps_normal_variables_to_alternative_variables() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let i_type = bank.signature().type_bank().i_type();
        let var = Term::const_cell_alloc(-2);
        var.set_type(Some(i_type));
        let root = Term::top_alloc(f_code, 1);
        root.set_argument(0, var);

        let disjoint = bank.insert_disjoint(&root).unwrap();
        let alt = disjoint.argument(0).unwrap();

        assert_eq!(alt.f_code(), -1);
        assert!(alt.is_free_var());
        assert!(disjoint.is_shared());
    }

    #[test]
    fn create_min_term_caches_by_declared_type() {
        let mut sig = Signature::new(TypeBank::new());
        let a = sig.insert_id("a", 0, false);
        let b = sig.insert_id("b", 0, false);
        let i_type = sig.type_bank().i_type();
        sig.declare_type(a, i_type.clone()).unwrap();
        sig.declare_type(b, i_type).unwrap();
        let mut bank = TermBank::new(sig).unwrap();

        let first = bank.create_min_term(a).unwrap();
        let second = bank.create_min_term(b).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.f_code(), a);
    }

    #[test]
    fn alloc_new_skolem_creates_typed_function_terms_from_variables() {
        let sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let mut bank = TermBank::new(sig).unwrap();
        let x = Term::const_cell_alloc(-2);
        x.set_type(Some(i_type.clone()));
        let y = Term::const_cell_alloc(-4);
        y.set_type(Some(i_type.clone()));

        let skolem = bank
            .alloc_new_skolem(&[x.clone(), y.clone()], Some(&i_type))
            .unwrap();

        assert_eq!(bank.signature().find_name(skolem.f_code()), Some("esk1_2"));
        assert_eq!(skolem.type_(), Some(i_type.clone()));
        assert!(skolem.is_shared());
        assert_eq!(skolem.argument(0).unwrap().f_code(), x.f_code());
        assert_eq!(skolem.argument(1).unwrap().f_code(), y.f_code());
        let declared = bank.signature().get_type(skolem.f_code()).unwrap();
        assert_eq!(declared.arity(), 3);
        assert_eq!(declared.args()[0], i_type);
    }

    #[test]
    fn alloc_new_skolem_uses_predicate_codes_for_bool_return_type() {
        let sig = Signature::new(TypeBank::new());
        let i_type = sig.type_bank().i_type();
        let bool_type = sig.type_bank().bool_type();
        let mut bank = TermBank::new(sig).unwrap();
        let x = Term::const_cell_alloc(-2);
        x.set_type(Some(i_type));

        let predicate = bank.alloc_new_skolem(&[x], Some(&bool_type)).unwrap();

        assert_eq!(
            bank.signature().find_name(predicate.f_code()),
            Some("epred1_1")
        );
        assert!(bank.signature().is_predicate(predicate.f_code()));
        assert_eq!(predicate.type_(), Some(bool_type));
    }

    #[test]
    fn sort_constant_selection_uses_signature_candidate_order_and_frequency_comparator() {
        let mut sig = Signature::new(TypeBank::new());
        let individual = sig.type_bank().i_type();
        let animal_code = sig.type_bank_mut().define_simple_sort("$animal").unwrap();
        let animal = sig
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(animal_code));
        let mineral_code = sig.type_bank_mut().define_simple_sort("$mineral").unwrap();
        let mineral = sig
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(mineral_code));

        let first_individual = sig.insert_id("first_individual", 0, false);
        let second_individual = sig.insert_id("second_individual", 0, false);
        sig.declare_final_type(second_individual, individual.clone())
            .unwrap();
        let animal_const = sig.insert_id("animal_const", 0, false);
        sig.declare_final_type(animal_const, animal.clone())
            .unwrap();
        let unary = sig.insert_id("unary", 1, false);
        sig.declare_final_type(
            unary,
            alloc_arrow_type(vec![individual.clone(), individual.clone()]),
        )
        .unwrap();

        let mut bank = TermBank::new(sig).unwrap();
        assert_eq!(
            bank.get_first_const_term(&individual)
                .unwrap()
                .unwrap()
                .f_code(),
            first_individual
        );
        assert_eq!(
            bank.get_first_const_term(&animal)
                .unwrap()
                .unwrap()
                .f_code(),
            animal_const
        );
        assert!(bank.get_first_const_term(&mineral).unwrap().is_none());

        let len = usize::try_from(bank.signature().f_count()).unwrap() + 1;
        let mut conj_dist_array = vec![0; len];
        let mut dist_array = vec![0; len];
        dist_array[usize::try_from(first_individual).unwrap()] = 7;
        dist_array[usize::try_from(second_individual).unwrap()] = 2;

        let selected = bank
            .get_freq_const_term(
                &individual,
                &mut conj_dist_array,
                &dist_array,
                |candidate, best, _, dist| {
                    dist[usize::try_from(candidate).unwrap()] < dist[usize::try_from(best).unwrap()]
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(selected.f_code(), second_individual);
        assert!(selected.is_shared());
    }

    #[test]
    fn term_map_recurses_only_when_mapper_returns_the_same_shared_term() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let b_code = bank.signature_mut().insert_id("b", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut()
            .declare_type(a_code, i_type.clone())
            .unwrap();
        bank.signature_mut().declare_type(b_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let b = bank.create_const_term(b_code).unwrap();
        let root = Term::top_alloc(f_code, 1);
        root.set_argument(0, a.clone());
        let shared_root = bank.insert(&root, DerefType::Never).unwrap();

        let mut stopped_visits = 0;
        let stopped = bank
            .map_term(&shared_root, &mut |_, _| {
                stopped_visits += 1;
                Ok(None)
            })
            .unwrap();
        assert_eq!(stopped, shared_root);
        assert_eq!(stopped_visits, 1);

        let mut replace_a = |_: &mut TermBank, term: &Term| {
            if term == &a {
                Ok(Some(b.clone()))
            } else {
                Ok(Some(term.clone()))
            }
        };
        let mapped_root = bank.map_term(&shared_root, &mut replace_a).unwrap();

        assert_ne!(mapped_root, shared_root);
        assert_eq!(mapped_root.argument(0), Some(b));
        assert!(mapped_root.is_shared());
    }

    #[test]
    fn term_apply_arg_uses_bank_type_storage_and_preserves_application_shape() {
        let mut type_bank = TypeBank::new();
        let i_type = type_bank.i_type();
        let arrow =
            type_bank.insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let mut sig = Signature::new(type_bank);
        let f_code = sig.insert_id("f", 0, false);
        sig.declare_type(f_code, arrow).unwrap();
        let a_code = sig.insert_id("a", 0, false);
        sig.declare_type(a_code, i_type.clone()).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let function = bank.create_const_term(f_code).unwrap();
        let arg = bank.create_const_term(a_code).unwrap();

        let applied = bank.term_apply_arg(&function, &arg);

        assert_eq!(applied.f_code(), f_code);
        assert_eq!(applied.arity(), 1);
        assert_eq!(applied.argument(0), Some(arg));
        assert_eq!(applied.type_(), Some(i_type));
    }

    #[test]
    fn applied_free_var_with_db_args_is_pattern_and_counts_as_single_var() {
        let mut type_bank = TypeBank::new();
        let i_type = type_bank.i_type();
        let arrow =
            type_bank.insert_type_shared(alloc_arrow_type(vec![i_type.clone(), i_type.clone()]));
        let sig = Signature::new(type_bank);
        let mut bank = TermBank::new(sig).unwrap();
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(arrow));
        let arg = bank.request_db_var(&i_type, 0);
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_argument(0, head);
        app.set_argument(1, arg);

        let shared = bank.insert_ignore_var(&app, DerefType::Never).unwrap();

        assert!(shared.query_prop(TP_HAS_APP_VAR));
        assert!(!shared.query_prop(TP_HAS_NON_PATTERN_VAR));
        assert!(shared.is_pattern());
        assert_eq!(shared.v_count(), 1);
        assert_eq!(shared.f_count(), 0);
        assert_eq!(shared.weight(), DEFAULT_VWEIGHT);
    }

    #[test]
    fn applied_free_var_with_non_db_arg_is_marked_non_pattern() {
        let mut type_bank = TypeBank::new();
        let arrow = type_bank.insert_type_shared(alloc_arrow_type(vec![
            type_bank.i_type(),
            type_bank.i_type(),
        ]));
        let mut sig = Signature::new(type_bank);
        let arg_code = sig.insert_id("a", 0, false);
        sig.declare_type(arg_code, sig.type_bank().i_type())
            .unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let head = Term::const_cell_alloc(-2);
        head.set_type(Some(arrow));
        let arg = bank.create_const_term(arg_code).unwrap();
        let app = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        app.set_argument(0, head);
        app.set_argument(1, arg);

        let shared = bank.insert_ignore_var(&app, DerefType::Never).unwrap();

        assert!(shared.query_prop(TP_HAS_APP_VAR | TP_HAS_NON_PATTERN_VAR));
        assert_eq!(shared.weight(), DEFAULT_VWEIGHT + DEFAULT_FWEIGHT);
    }

    #[test]
    fn reference_property_helpers_follow_current_top_key_behavior() {
        let (mut bank, a_code) = bank_with_symbol("a", 0);
        let mut term = bank.create_const_term(a_code).unwrap();

        bank.ref_set_prop(&mut term, TP_CHECK_FLAG).unwrap();

        assert!(term.query_prop(TP_CHECK_FLAG));
        let same = term.clone();
        bank.ref_del_prop(&mut term, TP_CHECK_FLAG).unwrap();

        assert_eq!(term, same);
        assert!(term.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn property_count_helpers_prune_by_current_property_state() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let f = Term::top_alloc(f_code, 1);
        f.set_argument(0, a.clone());
        let shared = bank.insert(&f, DerefType::Never).unwrap();

        assert_eq!(tb_term_set_prop_count(&shared, TP_CHECK_FLAG), 2);
        assert!(shared.query_prop(TP_CHECK_FLAG));
        assert!(a.query_prop(TP_CHECK_FLAG));
        assert_eq!(tb_term_set_prop_count(&shared, TP_CHECK_FLAG), 0);
        assert_eq!(tb_term_del_prop_count(&shared, TP_CHECK_FLAG), 2);
        assert!(!shared.query_prop(TP_CHECK_FLAG));
        assert!(!a.query_prop(TP_CHECK_FLAG));
    }

    #[test]
    fn gc_mark_and_sweep_keep_roots_min_terms_and_rewrite_targets() {
        let mut sig = Signature::new(TypeBank::new());
        let keep_code = sig.insert_id("keep", 0, false);
        let dead_code = sig.insert_id("dead", 0, false);
        let repl_code = sig.insert_id("repl", 0, false);
        let i_type = sig.type_bank().i_type();
        sig.declare_type(keep_code, i_type.clone()).unwrap();
        sig.declare_type(dead_code, i_type.clone()).unwrap();
        sig.declare_type(repl_code, i_type).unwrap();
        let mut bank = TermBank::new(sig).unwrap();
        let keep = bank.create_min_term(keep_code).unwrap();
        let dead = bank.create_const_term(dead_code).unwrap();
        let replacement = bank.create_const_term(repl_code).unwrap();
        term_add_rw_link(
            &keep,
            &replacement,
            None,
            false,
            RwResultType::LimitedRewritable,
        );

        let recovered = bank.gc_sweep();

        assert_eq!(recovered, 1);
        assert_eq!(bank.recovered(), 1);
        assert_eq!(bank.garbage_state(), TP_GARBAGE_FLAG);
        assert_eq!(bank.find(&dead), None);
        assert_eq!(bank.find(&keep), Some(keep.clone()));
        assert_eq!(bank.find(&replacement), Some(replacement));
    }

    #[test]
    fn find_repr_uses_this_banks_shared_arguments() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let f = Term::top_alloc(f_code, 1);
        f.set_argument(0, a.clone());
        let shared = bank.insert(&f, DerefType::Never).unwrap();
        let external_a = Term::const_cell_alloc(a_code);
        external_a.set_type(a.type_());
        let external_f = Term::top_alloc(f_code, 1);
        external_f.set_type(shared.type_());
        external_f.set_argument(0, external_a);

        assert_eq!(bank.find_repr(&external_f), Some(shared));
    }

    #[test]
    fn collect_subterms_sets_op_flag_and_skips_existing_marks() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let f = Term::top_alloc(f_code, 1);
        f.set_argument(0, a.clone());
        let shared = bank.insert(&f, DerefType::Never).unwrap();
        let mut collector = PStack::new();

        assert_eq!(tb_term_collect_subterms(&shared, &mut collector), 2);
        assert_eq!(collector.len(), 2);
        assert!(shared.query_prop(TP_OP_FLAG));
        assert!(a.query_prop(TP_OP_FLAG));
        assert_eq!(tb_term_collect_subterms(&shared, &mut collector), 0);
    }

    #[test]
    fn bank_in_order_prints_entry_ordered_dag() {
        let (mut bank, f_code) = bank_with_symbol("f", 2);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let root = Term::top_alloc(f_code, 2);
        root.set_argument(0, a.clone());
        root.set_argument(1, a);
        let shared_root = bank.insert(&root, DerefType::Never).unwrap();

        assert_eq!(shared_root.entry_no(), 4);
        assert_eq!(
            bank.bank_in_order_string(),
            "*1 : $true   =   $true\n*2 : $false   =   $false\n*3 : a   =   a\n*4 : f(*3,*3)   =   f(a,a)\n"
        );
        assert_eq!(bank.term_string(&shared_root, true), "f(a,a)");
    }

    #[test]
    fn compact_printing_sets_output_flags_and_reuses_abbreviations() {
        let (mut bank, f_code) = bank_with_symbol("f", 2);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let root = Term::top_alloc(f_code, 2);
        root.set_argument(0, a.clone());
        root.set_argument(1, a.clone());
        let shared_root = bank.insert(&root, DerefType::Never).unwrap();

        assert_eq!(bank.term_compact_string(&shared_root), "*4:f(*3:a,*3)");
        assert!(shared_root.query_prop(TP_OUTPUT_FLAG));
        assert!(a.query_prop(TP_OUTPUT_FLAG));
        assert_eq!(bank.term_compact_string(&shared_root), "*4");
    }

    #[test]
    fn bank_terms_prints_only_top_position_terms() {
        let (mut bank, f_code) = bank_with_symbol("f", 1);
        let a_code = bank.signature_mut().insert_id("a", 0, false);
        let i_type = bank.signature().type_bank().i_type();
        bank.signature_mut().declare_type(a_code, i_type).unwrap();
        let a = bank.create_const_term(a_code).unwrap();
        let root = Term::top_alloc(f_code, 1);
        root.set_argument(0, a);
        let shared_root = bank.insert(&root, DerefType::Never).unwrap();
        shared_root.set_prop(TP_TOP_POS);

        assert_eq!(bank.bank_terms_string(), "*4:f(*3:a)\n");
    }
}
