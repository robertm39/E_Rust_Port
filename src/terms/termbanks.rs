use crate::basics::error::Diagnostic;
use crate::terms::dbvars::DbVarBank;
use crate::terms::functypes::FunCode;
use crate::terms::garbage_coll::GcAdmin;
use crate::terms::signature::{Signature, SIG_TRUE_CODE};
use crate::terms::signature::{SIG_DB_LAMBDA_CODE, SIG_FALSE_CODE, SIG_NAMED_LAMBDA_CODE};
use crate::terms::simpletypes::TypeUniqueId;
use crate::terms::termcellstore::TermCellStore;
use crate::terms::termtypes::{
    term_deref, DerefType, Term, TermProperties, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TP_GARBAGE_FLAG,
    TP_HAS_APP_VAR, TP_HAS_BOOL_SUBTERM, TP_HAS_DB_SUBTERM, TP_HAS_EQ_NEQ_SYM,
    TP_HAS_ETA_EXPANDABLE_SUBTERM, TP_HAS_LAMBDA_SUBTERM, TP_HAS_NON_PATTERN_VAR, TP_IGNORE_PROPS,
    TP_IS_BETA_REDUCIBLE, TP_IS_GROUND, TP_IS_SHARED, TP_PRED_POS,
};
use crate::terms::termvars::VarBank;
use crate::terms::typecheck::type_infer_sort;
use std::collections::BTreeMap;

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

    fn insert_with_mode(
        &mut self,
        term: &Term,
        deref: DerefType,
        mode: InsertMode,
    ) -> Result<Term, Diagnostic> {
        let mut current_deref = deref;
        let term = term_deref(term, &mut current_deref);

        if term.is_free_var() {
            if mode == InsertMode::KeepVariables {
                return Ok(term);
            }
            let type_ = term.type_().expect("free variable must have a type");
            return Ok(self.vars.var_assert_alloc(term.f_code(), &type_));
        }
        if term.is_db_var() {
            let type_ = term.type_().expect("DB variable must have a type");
            return Ok(self.db_vars.request_db_var(&type_, term.f_code()));
        }

        let copy = Term::top_copy_without_args(&term);
        if mode == InsertMode::NoProperties {
            copy.set_properties(TP_IGNORE_PROPS);
        }
        for (index, arg) in term.argument_clones().into_iter().enumerate() {
            let arg = arg.unwrap_or_else(|| panic!("term argument {index} is uninitialized"));
            let shared = self.insert_with_mode(&arg, current_deref, mode)?;
            copy.set_argument(index, shared);
        }
        self.term_top_insert(copy)
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
            term.set_prop(TP_HAS_APP_VAR | TP_HAS_NON_PATTERN_VAR);
        }

        term.set_v_count(v_count);
        term.set_f_count(f_count);
        term.set_weight(weight);
        if v_count == 0 {
            term.set_prop(TP_IS_GROUND);
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{
        tb_cell_ident, tb_term_is_ground, term_is_false_term, term_is_true_term, TermBank,
    };
    use crate::terms::signature::{Signature, SIG_FALSE_CODE, SIG_PHONY_APP_CODE, SIG_TRUE_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termtypes::{
        DerefType, Term, DEFAULT_FWEIGHT, DEFAULT_VWEIGHT, TP_HAS_APP_VAR, TP_HAS_NON_PATTERN_VAR,
        TP_IS_GROUND, TP_IS_SHARED, TP_PRED_POS,
    };
    use crate::terms::typebanks::TypeBank;

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
    fn applied_free_var_is_marked_non_pattern_until_normalization_is_ported() {
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
}
