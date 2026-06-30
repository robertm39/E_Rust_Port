use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::ProblemType;
use crate::clauses::clause::{
    clause_print_lop_format_string, clause_print_tptp_format_string, clause_print_tstp_core_string,
    clause_tstp_string, Clause,
};
use crate::clauses::clause_props::{
    FormulaProperties, CP_IGNORE_PROPS, CP_INPUT_FORMULA, CP_IS_LAMBDA_DEF, CP_TYPE_AXIOM,
    CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS, CP_TYPE_LEMMA, CP_TYPE_NEG_CONJECTURE,
    CP_TYPE_QUESTION,
};
use crate::clauses::clausefunc::{
    post_cnf_encode_clause_terms, tformula_app_encode_string, tformula_clause_closed_encode,
    tformula_closure, tformula_collect_clause, tformula_conjunctive_nf3, tformula_copy_def,
    tformula_create_def, tformula_decode_polarity, tformula_fcode_alloc, tformula_find_defs,
    tformula_is_complex_bool, tformula_is_literal, tformula_is_prop_true, tformula_mark_polarity,
    tformula_preload_types, tformula_simplify, tformula_to_cnf, tformula_tptp_string,
    tformula_unroll_fool_result, TFormulaDefinitions, TFormulaTptpPrintOptions,
};
use crate::clauses::clauseinfo::ClauseInfo;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{
    clause_push_formula_derivation, FormulaDerivationRef, DC_ANNO_QUESTION, DC_APPLY_DEF,
    DC_EQ_TO_EQ, DC_FOF_QUOTE, DC_FOF_SIMPLIFY, DC_FOOL_UNROLL, DC_INTRO_DEF, DC_NEGATE_CONJECTURE,
    DC_SPLIT_EQUIV,
};
use crate::clauses::garbage_coll::tb_gc_collect;
use crate::terms::functypes::FunCode;
use crate::terms::lambda::lambda_normalize_db;
use crate::terms::signature::Signature;
use crate::terms::simpletypes::type_is_predicate;
use crate::terms::termbanks::tb_term_collect_subterms;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{
    term_compute_order, term_has_f_code, term_is_untyped, term_standard_weight,
};
use crate::terms::termtypes::{
    term_del_prop, term_has_interpreted_symbol, DerefType, Term, TP_CHECK_FLAG, TP_NEG_POLARITY,
    TP_OP_FLAG, TP_POS_POLARITY,
};
use crate::terms::termvars::VarBank;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

static WRAPPED_FORMULA_ENTRY_ID: AtomicU64 = AtomicU64::new(1);
static FORMULA_IDENT_COUNTER: AtomicI64 = AtomicI64::new(i64::MIN);
const TFORMULA_GC_LIMIT_NUMERATOR: i64 = 3;
const TFORMULA_GC_LIMIT_DENOMINATOR: i64 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaPrintFormat {
    Lop,
    Tptp,
    Tstp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaTstpCompleteness {
    Complete,
    Open,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FormulaTstpClauseMode {
    AsFormula,
    AsClauseCore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormulaTstpPrintOptions {
    pub full_terms: bool,
    pub completeness: FormulaTstpCompleteness,
    pub clause_mode: FormulaTstpClauseMode,
    pub keep_input_names: bool,
}

impl FormulaTstpPrintOptions {
    #[must_use]
    pub const fn complete_formula(full_terms: bool, keep_input_names: bool) -> Self {
        Self {
            full_terms,
            completeness: FormulaTstpCompleteness::Complete,
            clause_mode: FormulaTstpClauseMode::AsFormula,
            keep_input_names,
        }
    }

    #[must_use]
    pub const fn open_formula(full_terms: bool, keep_input_names: bool) -> Self {
        Self {
            full_terms,
            completeness: FormulaTstpCompleteness::Open,
            clause_mode: FormulaTstpClauseMode::AsFormula,
            keep_input_names,
        }
    }

    #[must_use]
    pub const fn with_clause_mode(mut self, clause_mode: FormulaTstpClauseMode) -> Self {
        self.clause_mode = clause_mode;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrappedFormulaCnfResult {
    pub clauses_generated: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetCnfResult {
    pub clauses_generated: i64,
    pub original_formulas_archived: i64,
    pub cnf_formulas_archived: i64,
    pub term_garbage_collections: i64,
    pub terms_recovered_by_gc: i64,
    pub formulas_simplified: i64,
    pub boolean_equalities_replaced: i64,
    pub formulas_fool_unrolled: i64,
    pub definitions_introduced: i64,
    pub definition_applications: i64,
    pub definition_formulas_archived: i64,
    pub active_definition_formulas_inserted: i64,
    pub formulas_rewritten_by_defs: i64,
    pub quoted_formula_sources: Vec<FormulaDerivationRef>,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormulaSetCnfOptions {
    pub miniscope_limit: i64,
    pub def_limit: i64,
    pub fool_unroll: bool,
    pub problem_type: ProblemType,
}

impl FormulaSetCnfOptions {
    #[must_use]
    pub const fn new(miniscope_limit: i64, fool_unroll: bool, problem_type: ProblemType) -> Self {
        Self {
            miniscope_limit,
            def_limit: 0,
            fool_unroll,
            problem_type,
        }
    }

    #[must_use]
    pub const fn with_def_limit(mut self, def_limit: i64) -> Self {
        self.def_limit = def_limit;
        self
    }
}

const fn formula_set_gc_threshold(old_nodes: i64) -> i64 {
    old_nodes.saturating_mul(TFORMULA_GC_LIMIT_NUMERATOR) / TFORMULA_GC_LIMIT_DENOMINATOR
}

fn collect_formula_set_cnf_garbage(
    bank: &mut TermBank,
    set: &FormulaSet,
    archive: &FormulaSet,
    clauseset: &ClauseSet,
    result: &mut FormulaSetCnfResult,
) {
    let recovered = tb_gc_collect(bank, &[clauseset], &[set, archive]);
    result.term_garbage_collections += 1;
    result.terms_recovered_by_gc += recovered;
}

fn collect_formula_set_simplify_garbage(
    bank: &mut TermBank,
    set: &FormulaSet,
    result: &mut FormulaSetSimplifyResult,
) {
    let clause_sets: [&ClauseSet; 0] = [];
    let recovered = tb_gc_collect(bank, &clause_sets, &[set]);
    result.term_garbage_collections += 1;
    result.terms_recovered_by_gc += recovered;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetSimplifyResult {
    pub formulas_changed: i64,
    pub term_garbage_collections: i64,
    pub terms_recovered_by_gc: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetPreprocessConjecturesResult {
    pub conjectures_negated: i64,
    pub questions_annotated: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetFoolUnrollResult {
    pub boolean_equalities_replaced: i64,
    pub formulas_unrolled: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetIntroduceDefsResult {
    pub definitions_introduced: i64,
    pub archived_definitions: i64,
    pub active_definitions_inserted: i64,
    pub formulas_rewritten: i64,
    pub definition_applications: i64,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSetArchiveResult {
    pub formulas_archived: i64,
    pub quoted_formula_sources: Vec<FormulaDerivationRef>,
    pub formula_derivation_ops: Vec<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrappedFormula {
    entry_id: u64,
    properties: FormulaProperties,
    is_clause: bool,
    ident: i64,
    info: Option<ClauseInfo>,
    formula: Option<Term>,
}

impl WrappedFormula {
    #[must_use]
    pub fn default_alloc() -> Self {
        Self {
            entry_id: next_entry_id(),
            properties: CP_IGNORE_PROPS,
            is_clause: false,
            ident: 0,
            info: None,
            formula: None,
        }
    }

    #[must_use]
    pub fn wt_formula_alloc(formula: Term) -> Self {
        Self {
            formula: Some(formula),
            ident: next_formula_ident(),
            ..Self::default_alloc()
        }
    }

    #[must_use]
    pub fn flat_copy(&self) -> Self {
        Self {
            entry_id: next_entry_id(),
            properties: self.properties,
            is_clause: self.is_clause,
            ident: self.ident,
            info: None,
            formula: self.formula.clone(),
        }
    }

    #[must_use]
    pub const fn entry_id(&self) -> u64 {
        self.entry_id
    }

    #[must_use]
    pub const fn properties(&self) -> FormulaProperties {
        self.properties
    }

    pub fn set_properties(&mut self, properties: FormulaProperties) {
        self.properties = properties;
    }

    pub fn set_prop(&mut self, prop: FormulaProperties) {
        self.properties.set(prop);
    }

    pub fn del_prop(&mut self, prop: FormulaProperties) {
        self.properties.delete(prop);
    }

    #[must_use]
    pub const fn query_prop(&self, prop: FormulaProperties) -> bool {
        self.properties.query(prop)
    }

    #[must_use]
    pub const fn give_props(&self, prop: FormulaProperties) -> FormulaProperties {
        self.properties.give(prop)
    }

    pub fn set_tptp_type(&mut self, type_: FormulaProperties) {
        self.properties.set_tptp_type(type_);
    }

    #[must_use]
    pub const fn query_tptp_type(&self) -> FormulaProperties {
        self.properties.query_tptp_type()
    }

    #[must_use]
    pub const fn is_clause(&self) -> bool {
        self.is_clause
    }

    pub fn set_is_clause(&mut self, is_clause: bool) {
        self.is_clause = is_clause;
    }

    #[must_use]
    pub const fn ident(&self) -> i64 {
        self.ident
    }

    #[must_use]
    pub const fn info(&self) -> Option<&ClauseInfo> {
        self.info.as_ref()
    }

    pub fn set_info(&mut self, info: Option<ClauseInfo>) {
        self.info = info;
    }

    /// Returns the wrapped formula term.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper was allocated without a formula term.
    #[must_use]
    pub fn formula(&self) -> &Term {
        self.formula
            .as_ref()
            .unwrap_or_else(|| panic!("wrapped formula has no term formula"))
    }

    pub fn set_formula(&mut self, formula: Term) {
        self.formula = Some(formula);
    }

    pub fn gc_mark_cells(&self, bank: &TermBank) {
        if let Some(formula) = &self.formula {
            bank.gc_mark_term(formula);
        }
    }

    pub fn mark_polarity(&self, bank: &TermBank) {
        if let Some(formula) = &self.formula {
            tformula_mark_polarity(bank, formula, 1);
        }
    }

    /// Applies C `WFormulaSimplify` to this wrapped formula.
    ///
    /// C calls `TFormulaSimplify` with a quantifier-optimization limit of zero.
    /// The staged Rust wrapper updates the formula term and reports whether it
    /// changed; formula-level derivation storage remains deferred.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if simplifying or rebuilding the term-encoded
    /// formula fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if the formula is
    /// malformed.
    pub fn simplify(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let original = self.formula().clone();
        let simplified = tformula_simplify(bank, &original, 0)?;
        if simplified == original {
            return Ok(false);
        }
        self.set_formula(simplified);
        Ok(true)
    }

    /// Applies C `WFormulaConjectureNegate`.
    ///
    /// If the wrapper role is `conjecture`, this wraps the formula in a root
    /// negation and changes the role to `negated_conjecture`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the negation term cannot be allocated.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn conjecture_negate(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        if self.query_tptp_type() != CP_TYPE_CONJECTURE {
            return Ok(false);
        }
        let negated = tformula_fcode_alloc(
            bank,
            bank.signature().not_code(),
            self.formula().clone(),
            None,
        )?;
        self.set_formula(negated);
        self.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        Ok(true)
    }

    /// Applies C `WFormulaAnnotateQuestion`.
    ///
    /// Questions, and optionally conjectures, become conjectures. When
    /// `add_answer_lits` is true, leading existential quantifiers receive the
    /// C-shaped `~$answer(esk(...))` conjunct.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if an answer literal or rebuilt formula cannot be
    /// allocated.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a leading existential
    /// has uninitialized arguments.
    pub fn annotate_question(
        &mut self,
        bank: &mut TermBank,
        add_answer_lits: bool,
        conjectures_are_questions: bool,
    ) -> Result<bool, Diagnostic> {
        let role = self.query_tptp_type();
        if role != CP_TYPE_QUESTION && !(role == CP_TYPE_CONJECTURE && conjectures_are_questions) {
            return Ok(false);
        }
        if add_answer_lits {
            let annotated = tformula_annotate_question(bank, self.formula())?;
            self.set_formula(annotated);
        }
        self.set_tptp_type(CP_TYPE_CONJECTURE);
        Ok(true)
    }

    /// Applies C `WFormulaReplaceEqnWithEquiv`.
    ///
    /// Complex Boolean equalities become equivalence formulas, complex Boolean
    /// disequalities become XOR formulas, and comparisons against `$true`
    /// collapse to the Boolean side or its negation.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if a rebuilt formula cannot be allocated.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a formula cell has
    /// uninitialized arguments.
    pub fn replace_eqn_with_equiv(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let replaced = tformula_replace_eqn_with_equiv(bank, self.formula())?;
        if replaced == *self.formula() {
            return Ok(false);
        }
        self.set_formula(replaced);
        Ok(true)
    }

    /// Applies C `TFormulaUnrollFOOL` to this wrapper.
    ///
    /// Literal expansion always updates the wrapped formula when it changes.
    /// The returned flag reports only whether the FOOL unrolling mapper changed
    /// the expanded formula, matching C's `TFormulaUnrollFOOL` return value and
    /// `DCFoolUnroll` derivation condition.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if literal expansion, lambda eta-reduction, term
    /// replacement, or formula allocation fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term, a located FOOL subterm is not
    /// Boolean, or formula cells are malformed.
    pub fn unroll_fool(&mut self, bank: &mut TermBank) -> Result<bool, Diagnostic> {
        let result = tformula_unroll_fool_result(bank, self.formula())?;
        if result.formula() != self.formula() {
            self.set_formula(result.formula().clone());
        }
        Ok(result.fool_unrolled())
    }

    /// Applies C `TFormulaApplyDefs` to this wrapper.
    ///
    /// Definition parents are reported as the archived neutral-definition terms
    /// until full formula-derivation ownership can store stable formula handles.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if rebuilding the formula fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term, a marked subformula has no
    /// definition entry, or definition metadata has not been populated.
    pub fn apply_defs(
        &mut self,
        bank: &mut TermBank,
        defs: &TFormulaDefinitions,
    ) -> Result<Vec<Term>, Diagnostic> {
        let mut defs_used = Vec::new();
        let reduced = tformula_copy_def(bank, self.formula(), self.ident, defs, &mut defs_used)?;
        if !defs_used.is_empty() {
            self.set_formula(reduced);
        }
        Ok(defs_used)
    }

    #[must_use]
    pub fn get_id(&self, keep_input_names: bool) -> String {
        if keep_input_names {
            if let Some(name) = self.info.as_ref().and_then(ClauseInfo::name) {
                return name.to_owned();
            }
        }
        if self.ident < 0 {
            format!("i_0_{}", i128::from(self.ident) - i128::from(i64::MIN))
        } else {
            format!("c_0_{}", self.ident)
        }
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        term_is_untyped(self.formula())
    }

    #[must_use]
    pub fn has_interpreted_symbol(&self) -> bool {
        term_has_interpreted_symbol(self.formula())
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        term_standard_weight(self.formula())
    }

    #[must_use]
    pub fn is_hypothesis(&self) -> bool {
        self.properties.is_hypothesis()
    }

    #[must_use]
    pub fn is_conjecture(&self) -> bool {
        matches!(
            self.query_tptp_type(),
            CP_TYPE_CONJECTURE | CP_TYPE_QUESTION
        ) || self.properties.is_conjecture()
    }

    #[must_use]
    pub fn conjecture_order(&self, signature: &Signature) -> usize {
        term_compute_order(signature, self.formula())
    }

    #[must_use]
    pub fn contains_f_code(&self, f_code: FunCode) -> bool {
        term_has_f_code(self.formula(), f_code)
    }

    /// Pushes each distinct non-variable, non-phony-app f-code in traversal order.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if the formula is not
    /// shared by its term bank.
    pub fn return_f_codes(&self, f_codes: &mut Vec<FunCode>) -> i64 {
        let start = f_codes.len();
        let mut subterms = PStack::new();
        let _ = tb_term_collect_subterms(self.formula(), &mut subterms);
        let mut seen = BTreeSet::new();
        for term in subterms.as_slice() {
            term.del_prop(TP_OP_FLAG);
            if !term.is_any_var() && !term.is_phony_app() && seen.insert(term.f_code()) {
                f_codes.push(term.f_code());
            }
        }
        usize_to_i64(f_codes.len() - start)
    }

    /// Returns the count of distinct non-variable, non-phony-app f-codes.
    ///
    /// # Panics
    ///
    /// Panics under the same conditions as [`Self::return_f_codes`].
    #[must_use]
    pub fn symbol_diversity(&self) -> i64 {
        let mut f_codes = Vec::new();
        self.return_f_codes(&mut f_codes)
    }

    /// Extracts the defined symbol from C's lambda-definition formula shapes.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper is tagged as a lambda definition but has no
    /// formula term.
    #[must_use]
    pub fn get_lambda_defined_symbol(&self, signature: &Signature) -> Option<FunCode> {
        if !self.query_prop(CP_IS_LAMBDA_DEF) {
            return None;
        }

        let mut formula = self.formula().clone();
        while formula.f_code() == signature.qall_code() && formula.arity() == 2 {
            formula = formula.argument(1)?;
        }

        let left = if formula.f_code() == signature.eqn_code() {
            formula.argument(0)
        } else if formula.f_code() == signature.equiv_code() {
            let equivalence_left = formula.argument(0)?;
            if equivalence_left.f_code() == signature.eqn_code() {
                equivalence_left.argument(0)
            } else {
                None
            }
        } else {
            None
        }?;

        (left.f_code() > signature.internal_symbols()).then_some(left.f_code())
    }

    /// Renders C's `WFormulaAppEncode` shape for this wrapped formula.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if term application encoding fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or is marked as a clause.
    pub fn app_encode_string(
        &self,
        bank: &mut TermBank,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        assert!(!self.is_clause, "WFormulaAppEncode expects a formula");
        let encoded = tformula_app_encode_string(bank, self.formula())?;
        Ok(format!(
            "tff({}, {}, {encoded}).",
            self.get_id(keep_input_names),
            self.app_encode_role_name()
        ))
    }

    /// Converts C's wrapped clause formula back into a `Clause`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if collecting a literal or allocating the resulting
    /// clause fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if an encoded literal has
    /// malformed arguments.
    pub fn form_clause_to_clause(&self, bank: &mut TermBank) -> Result<Clause, Diagnostic> {
        let mut clause = tformula_collect_clause(bank, self.formula(), None)?;
        clause.set_properties(self.properties);
        clause.set_info(self.info.clone());
        Ok(clause)
    }

    /// Universally closes a clause encoding as C `WFormulaOfClause` does.
    ///
    /// C allocates a fresh formula wrapper and intentionally does not copy
    /// clause metadata onto it.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if literal encoding, closure allocation, or
    /// quantifier allocation fails.
    ///
    /// # Panics
    ///
    /// Panics if any clause literal violates term-bank sharing preconditions.
    pub fn of_clause(
        bank: &mut TermBank,
        clause: &Clause,
        problem_type: ProblemType,
    ) -> Result<Self, Diagnostic> {
        Ok(Self::wt_formula_alloc(tformula_clause_closed_encode(
            bank,
            clause,
            problem_type,
        )?))
    }

    /// Transforms this wrapped formula into CNF clauses, matching C
    /// `WFormulaCNF2` for the currently ported term-level CNF phases.
    ///
    /// The wrapper formula is first DB-lambda normalized. Clause-backed
    /// wrappers are converted directly into one clause with `DCFofQuote`
    /// provenance. Formula-backed wrappers are transformed through
    /// `WTFormulaConjunctiveNF3`, update this wrapper's formula payload, and
    /// then delegate to the staged `TFormulaToCNF` core.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if lambda normalization, clause conversion,
    /// higher-order post-CNF encoding, CNF transformation, or clause insertion
    /// preparation fails.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term, if the miniscope limit is
    /// negative, or if a malformed encoded literal/formula violates the C CNF
    /// preconditions.
    pub fn cnf2_into(
        &mut self,
        bank: &mut TermBank,
        set: &mut ClauseSet,
        fresh_vars: &VarBank,
        miniscope_limit: i64,
        fool_unroll: bool,
        problem_type: ProblemType,
    ) -> Result<WrappedFormulaCnfResult, Diagnostic> {
        let normalized = lambda_normalize_db(bank, self.formula())?;
        self.set_formula(normalized);
        let source = FormulaDerivationRef::new(self.ident);

        if self.is_clause {
            let mut clause = self.form_clause_to_clause(bank)?;
            clause_push_formula_derivation(&mut clause, DC_FOF_QUOTE, Some(source), None);
            if problem_type == ProblemType::HigherOrder {
                post_cnf_encode_clause_terms(bank, &mut clause)?;
            }
            set.insert(clause);
            return Ok(WrappedFormulaCnfResult {
                clauses_generated: 1,
                formula_derivation_ops: Vec::new(),
            });
        }

        let cnf_result =
            tformula_conjunctive_nf3(bank, self.formula(), miniscope_limit, fool_unroll)?;
        self.set_formula(cnf_result.formula().clone());
        let clauses_generated = tformula_to_cnf(
            bank,
            self.formula(),
            self.query_tptp_type(),
            set,
            fresh_vars,
            source,
            problem_type,
        )?;
        Ok(WrappedFormulaCnfResult {
            clauses_generated,
            formula_derivation_ops: cnf_result.derivation_ops().to_vec(),
        })
    }

    /// Renders C's `WFormulaTPTPPrint` shape for a formula-backed wrapper.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the contained term formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn tptp_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        if self.is_clause {
            return Err(formula_set_write_error(
                "WFormulaTPTPPrint clause conversion is not ported",
            ));
        }
        let rendered = tformula_tptp_string(
            bank,
            self.formula(),
            full_terms,
            TFormulaTptpPrintOptions::tptp(problem_type),
        )?;
        Ok(format!(
            "input_formula({},{},{rendered}).",
            self.get_id(keep_input_names),
            self.tptp_role_name()
        ))
    }

    /// Renders C's `WFormulaTSTPPrintFlex` shape for a formula-backed wrapper.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the contained term formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn tstp_string_flex(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        options: FormulaTstpPrintOptions,
    ) -> Result<String, Diagnostic> {
        let formula_kind = if problem_type == ProblemType::HigherOrder {
            "thf"
        } else if self.is_clause && options.clause_mode == FormulaTstpClauseMode::AsClauseCore {
            if self.is_untyped() {
                "cnf"
            } else {
                "tcf"
            }
        } else if self.is_untyped() {
            "fof"
        } else {
            "tff"
        };
        let rendered = if self.is_clause {
            match options.clause_mode {
                FormulaTstpClauseMode::AsFormula => {
                    let closure = tformula_closure(bank, self.formula(), true)?;
                    tformula_tptp_string(
                        bank,
                        &closure,
                        options.full_terms,
                        TFormulaTptpPrintOptions::tstp(problem_type),
                    )?
                }
                FormulaTstpClauseMode::AsClauseCore => {
                    let clause = self.form_clause_to_clause(bank)?;
                    clause_print_tstp_core_string(bank, &clause, options.full_terms, false)
                }
            }
        } else {
            tformula_tptp_string(
                bank,
                self.formula(),
                options.full_terms,
                TFormulaTptpPrintOptions::tstp(problem_type),
            )?
        };
        let mut output = format!(
            "{formula_kind}({}, {}, {rendered}",
            self.get_id(options.keep_input_names),
            self.tstp_role_name()
        );
        if options.completeness == FormulaTstpCompleteness::Complete {
            output.push_str(").");
        }
        Ok(output)
    }

    /// Renders C's `WFormulaTSTPPrint` macro shape with `as_formula=true`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the contained term formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn tstp_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        complete: bool,
        problem_type: ProblemType,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        self.tstp_string_flex(
            bank,
            problem_type,
            FormulaTstpPrintOptions {
                full_terms,
                completeness: if complete {
                    FormulaTstpCompleteness::Complete
                } else {
                    FormulaTstpCompleteness::Open
                },
                clause_mode: FormulaTstpClauseMode::AsFormula,
                keep_input_names,
            },
        )
    }

    /// Renders C's `WFormulaPrint` dispatch for formula-backed wrappers.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the selected output format or contained term
    /// formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    pub fn print_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        output_format: FormulaPrintFormat,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        if self.is_clause {
            let clause = self.form_clause_to_clause(bank)?;
            return match output_format {
                FormulaPrintFormat::Lop => {
                    Ok(clause_print_lop_format_string(bank, &clause, full_terms))
                }
                FormulaPrintFormat::Tptp => Ok(clause_print_tptp_format_string(bank, &clause)),
                FormulaPrintFormat::Tstp => {
                    clause_tstp_string(bank, &clause, full_terms, true, problem_type)
                }
            };
        }

        match output_format {
            FormulaPrintFormat::Lop | FormulaPrintFormat::Tptp => {
                self.tptp_string(bank, full_terms, problem_type, keep_input_names)
            }
            FormulaPrintFormat::Tstp => {
                self.tstp_string(bank, full_terms, true, problem_type, keep_input_names)
            }
        }
    }

    /// Counts lambda cells that occur below a non-logical/non-lambda formula node.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term or if a malformed lambda or
    /// phony-application cell has uninitialized arguments.
    #[must_use]
    pub fn count_non_top_level_lambdas(&self, signature: &Signature) -> i32 {
        tformula_count_non_top_level_lambdas(signature, self.formula())
    }

    /// Returns whether any literal side is an applied free variable.
    ///
    /// # Panics
    ///
    /// Panics if this wrapper has no formula term.
    #[must_use]
    pub fn has_app_var_literal(&self, bank: &TermBank) -> bool {
        tformula_has_app_var_literal(bank, self.formula())
    }

    fn lambda_definition_head_for_statistics(&self, bank: &TermBank) -> Option<Term> {
        if !self.query_prop(CP_IS_LAMBDA_DEF) {
            return None;
        }

        let signature = bank.signature();
        let mut formula = self.formula().clone();
        while formula.f_code() == signature.qall_code() && formula.arity() == 2 {
            formula = formula.argument(1)?;
        }

        if formula.f_code() == signature.eqn_code() {
            return formula.argument(0);
        }
        if formula.f_code() != signature.equiv_code() {
            return None;
        }

        let equivalence_left = formula.argument(0)?;
        if equivalence_left.f_code() != signature.eqn_code()
            || equivalence_left.argument(1).as_ref() != Some(bank.true_term())
        {
            return None;
        }
        equivalence_left.argument(0)
    }

    fn app_encode_role_name(&self) -> &'static str {
        self.tstp_role_name()
    }

    fn tstp_role_name(&self) -> &'static str {
        match self.query_tptp_type() {
            CP_TYPE_AXIOM if self.query_prop(CP_INPUT_FORMULA) => "axiom",
            CP_TYPE_HYPOTHESIS => "hypothesis",
            CP_TYPE_CONJECTURE => "conjecture",
            CP_TYPE_QUESTION => "question",
            CP_TYPE_LEMMA => "lemma",
            CP_TYPE_NEG_CONJECTURE => "negated_conjecture",
            _ => "plain",
        }
    }

    fn tptp_role_name(&self) -> &'static str {
        match self.query_tptp_type() {
            CP_TYPE_AXIOM => "axiom",
            CP_TYPE_HYPOTHESIS => "hypothesis",
            CP_TYPE_CONJECTURE | CP_TYPE_NEG_CONJECTURE => "conjecture",
            CP_TYPE_QUESTION => "question",
            _ => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FormulaDefinitionStatistics {
    pub num_defs: i32,
    pub percentage_form_defs: f64,
    pub num_lams: i32,
    pub has_app_var_lits: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormulaSet {
    formulas: Vec<WrappedFormula>,
    identifier: String,
}

impl FormulaSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn cardinality(&self) -> i64 {
        usize_to_i64(self.formulas.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.formulas.is_empty()
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn set_identifier(&mut self, identifier: impl Into<String>) {
        self.identifier = identifier.into();
    }

    pub fn iter(&self) -> impl Iterator<Item = &WrappedFormula> {
        self.formulas.iter()
    }

    pub fn insert(&mut self, formula: WrappedFormula) -> u64 {
        let entry_id = formula.entry_id();
        self.formulas.push(formula);
        entry_id
    }

    pub fn insert_set(&mut self, from: &mut Self) -> i64 {
        let moved = from.cardinality();
        self.formulas.append(&mut from.formulas);
        moved
    }

    pub fn extract_first(&mut self) -> Option<WrappedFormula> {
        if self.formulas.is_empty() {
            None
        } else {
            Some(self.formulas.remove(0))
        }
    }

    pub fn extract_entry(&mut self, entry_id: u64) -> Option<WrappedFormula> {
        let position = self
            .formulas
            .iter()
            .position(|formula| formula.entry_id() == entry_id)?;
        Some(self.formulas.remove(position))
    }

    #[must_use]
    pub fn get(&self, entry_id: u64) -> Option<&WrappedFormula> {
        self.formulas
            .iter()
            .find(|formula| formula.entry_id() == entry_id)
    }

    pub fn delete_entry(&mut self, entry_id: u64) -> bool {
        self.extract_entry(entry_id).is_some()
    }

    pub fn move_formula_from(&mut self, from: &mut Self, entry_id: u64) -> bool {
        let Some(formula) = from.extract_entry(entry_id) else {
            return false;
        };
        self.insert(formula);
        true
    }

    /// Applies C `FormulaSetArchive`.
    ///
    /// Each formula is extracted in insertion order, the original wrapper is
    /// moved to `archive`, and a flat copy is inserted back into this set. The
    /// formula-level `DCFofQuote` derivation stack is deferred, so this returns
    /// the quote sources and opcodes that should be attached by a future owner.
    #[must_use]
    pub fn archive_into(&mut self, archive: &mut Self) -> FormulaSetArchiveResult {
        let mut result = FormulaSetArchiveResult::default();
        let mut tmpset = Self::new();

        while let Some(handle) = self.extract_first() {
            let source = FormulaDerivationRef::new(handle.ident());
            let newform = handle.flat_copy();
            tmpset.insert(newform);
            archive.insert(handle);
            result.formulas_archived += 1;
            result.quoted_formula_sources.push(source);
            result.formula_derivation_ops.push(DC_FOF_QUOTE);
        }

        self.insert_set(&mut tmpset);
        result
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.formulas.iter().all(WrappedFormula::is_untyped)
    }

    #[must_use]
    pub fn has_interpreted_symbol(&self) -> bool {
        self.formulas
            .iter()
            .any(WrappedFormula::has_interpreted_symbol)
    }

    #[must_use]
    pub fn split_conjectures(&self) -> (i64, Vec<&WrappedFormula>, Vec<&WrappedFormula>) {
        let mut conjectures = Vec::new();
        let mut rest = Vec::new();
        for formula in &self.formulas {
            if formula.is_conjecture() {
                conjectures.push(formula);
            } else {
                rest.push(formula);
            }
        }
        (usize_to_i64(conjectures.len()), conjectures, rest)
    }

    #[must_use]
    pub fn standard_weight(&self) -> i64 {
        self.formulas
            .iter()
            .filter(|formula| !formula.query_prop(CP_IS_LAMBDA_DEF))
            .map(WrappedFormula::standard_weight)
            .sum()
    }

    pub fn count_conjectures(&self, hypotheses: &mut i64) -> i64 {
        let mut conjectures = 0;
        for formula in &self.formulas {
            if formula.is_conjecture() {
                conjectures += 1;
            }
            if formula.is_hypothesis() {
                *hypotheses += 1;
            }
        }
        conjectures
    }

    #[must_use]
    pub fn conjecture_order(&self, signature: &Signature) -> usize {
        self.formulas
            .iter()
            .filter(|formula| formula.is_conjecture() || formula.is_hypothesis())
            .map(|formula| formula.conjecture_order(signature))
            .max()
            .unwrap_or(0)
    }

    pub fn collect_f_code(&self, f_code: FunCode, result: &mut Vec<u64>) -> i64 {
        let start = result.len();
        result.extend(
            self.formulas
                .iter()
                .filter(|formula| formula.contains_f_code(f_code))
                .map(WrappedFormula::entry_id),
        );
        usize_to_i64(result.len() - start)
    }

    pub fn gc_mark_cells(&self, bank: &TermBank) {
        for formula in &self.formulas {
            formula.gc_mark_cells(bank);
        }
    }

    pub fn mark_polarity(&self, bank: &TermBank) {
        for formula in &self.formulas {
            formula.mark_polarity(bank);
        }
    }

    fn del_term_props(&self, props: crate::terms::termtypes::TermProperties) {
        for formula in &self.formulas {
            if let Some(term) = &formula.formula {
                term_del_prop(term, DerefType::Never, props);
            }
        }
    }

    /// Applies C `FormulaSetSimplify` to each formula in insertion order
    /// without the optional term-bank garbage collection side effect.
    ///
    /// This stages the mutating simplification and changed-count behavior.
    /// Changed formulas are represented by `DCFofSimplify` opcodes in the
    /// result metadata; the formula derivation stack remains deferred until
    /// formula owners are ported.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if any wrapped formula cannot be simplified.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term or if a formula is malformed.
    pub fn simplify(
        &mut self,
        bank: &mut TermBank,
    ) -> Result<FormulaSetSimplifyResult, Diagnostic> {
        self.simplify_with_garbage_collection(bank, false)
    }

    /// Applies C `FormulaSetSimplify` to each formula in insertion order.
    ///
    /// When `do_garbage_collect` is true, this mirrors C's thresholded
    /// `TBGCCollect` checks using this set as the formula root set. Formula
    /// derivation stacks and proof-document output remain deferred; changed
    /// formulas are represented by `DCFofSimplify` opcodes in the result
    /// metadata.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if any wrapped formula cannot be simplified.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term or if a formula is malformed.
    pub fn simplify_with_garbage_collection(
        &mut self,
        bank: &mut TermBank,
        do_garbage_collect: bool,
    ) -> Result<FormulaSetSimplifyResult, Diagnostic> {
        let mut result = FormulaSetSimplifyResult::default();
        let mut old_nodes = bank.non_var_term_nodes();
        let mut gc_threshold = formula_set_gc_threshold(old_nodes);
        let mut index = 0;

        while index < self.formulas.len() {
            let changed = {
                let formula = &mut self.formulas[index];
                formula.simplify(bank)?
            };
            if changed {
                result.formulas_changed += 1;
                result.formula_derivation_ops.push(DC_FOF_SIMPLIFY);
                if do_garbage_collect && bank.non_var_term_nodes() > gc_threshold {
                    collect_formula_set_simplify_garbage(bank, self, &mut result);
                    old_nodes = bank.non_var_term_nodes();
                    gc_threshold = formula_set_gc_threshold(old_nodes);
                }
            }
            index += 1;
        }

        if do_garbage_collect && bank.non_var_term_nodes() != old_nodes {
            collect_formula_set_simplify_garbage(bank, self, &mut result);
        }
        Ok(result)
    }

    /// Applies C `FormulaSetPreprocConjectures` in insertion order.
    ///
    /// Each formula is first annotated as a question when applicable, then
    /// conjectures are negated. Formula-level derivation storage is deferred, so
    /// this returns the C derivation opcodes that should be attached by a future
    /// owner.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if answer-literal allocation or conjecture negation
    /// fails.
    ///
    /// # Panics
    ///
    /// Panics if any preprocessed wrapper has no formula term.
    pub fn preproc_conjectures(
        &mut self,
        bank: &mut TermBank,
        add_answer_lits: bool,
        conjectures_are_questions: bool,
    ) -> Result<FormulaSetPreprocessConjecturesResult, Diagnostic> {
        let mut result = FormulaSetPreprocessConjecturesResult::default();
        for formula in &mut self.formulas {
            if formula.annotate_question(bank, add_answer_lits, conjectures_are_questions)? {
                result.questions_annotated += 1;
                result.formula_derivation_ops.push(DC_ANNO_QUESTION);
            }
            if formula.conjecture_negate(bank)? {
                result.conjectures_negated += 1;
                result.formula_derivation_ops.push(DC_NEGATE_CONJECTURE);
            }
        }
        Ok(result)
    }

    /// Applies C `WFormulaSetUnrollFOOL` in insertion order.
    ///
    /// Each formula first runs `WFormulaReplaceEqnWithEquiv`, then
    /// `TFormulaUnrollFOOL`. Formula-level derivation storage is deferred, so
    /// this returns the C derivation opcodes that should be attached by a future
    /// owner.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if formula rebuilding, literal expansion, lambda
    /// eta-reduction, or term replacement fails.
    ///
    /// # Panics
    ///
    /// Panics if any preprocessed wrapper has no formula term or a malformed
    /// formula payload.
    pub fn unroll_fool(
        &mut self,
        bank: &mut TermBank,
    ) -> Result<FormulaSetFoolUnrollResult, Diagnostic> {
        let mut result = FormulaSetFoolUnrollResult::default();
        for formula in &mut self.formulas {
            if formula.replace_eqn_with_equiv(bank)? {
                result.boolean_equalities_replaced += 1;
                result.formula_derivation_ops.push(DC_EQ_TO_EQ);
            }
            if formula.unroll_fool(bank)? {
                result.formulas_unrolled += 1;
                result.formula_derivation_ops.push(DC_FOOL_UNROLL);
            }
        }
        Ok(result)
    }

    /// Applies C `TFormulaSetIntroduceDefs`.
    ///
    /// This stages the set-level definition introduction pipeline: clear the
    /// definition/polarity term flags, mark formula polarities, find expensive
    /// subformulas in non-clause wrappers, archive neutral definitions, insert
    /// active definitions into the set, and apply the definitions across the
    /// resulting set in insertion order.
    ///
    /// Formula-level derivation stacks and proof-document output are deferred,
    /// so this returns the C derivation opcodes that should be attached by a
    /// future owner.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if a definition atom, definition formula, or copied
    /// formula cannot be allocated.
    ///
    /// # Panics
    ///
    /// Panics if any processed wrapper has no formula term, if formula cells are
    /// malformed, or if definition metadata invariants are violated.
    pub fn introduce_defs(
        &mut self,
        archive: &mut Self,
        bank: &mut TermBank,
        limit: i64,
    ) -> Result<FormulaSetIntroduceDefsResult, Diagnostic> {
        let mut result = FormulaSetIntroduceDefsResult::default();
        let mut defs = TFormulaDefinitions::new();
        let mut renamed_forms = Vec::new();

        self.del_term_props(TP_CHECK_FLAG | TP_POS_POLARITY | TP_NEG_POLARITY);
        self.mark_polarity(bank);
        for formula in &self.formulas {
            if limit != 0 && !formula.is_clause {
                tformula_find_defs(
                    bank,
                    formula.formula(),
                    1,
                    limit,
                    &mut defs,
                    &mut renamed_forms,
                )?;
            }
        }

        result.definitions_introduced = usize_to_i64(renamed_forms.len());
        for form in renamed_forms {
            let entry_no = form.entry_no();
            let polarity = tformula_decode_polarity(&form);
            let def_atom = defs
                .get(&entry_no)
                .unwrap_or_else(|| panic!("renamed formula {entry_no} must have a definition"))
                .rename_atom()
                .clone();
            let neutral_def = tformula_create_def(bank, &def_atom, &form, 0)?;
            let neutral_wrapper = WrappedFormula::wt_formula_alloc(neutral_def);
            let archived_wrapper = neutral_wrapper.flat_copy();
            let archived_formula = archived_wrapper.formula().clone();
            archive.insert(archived_wrapper);
            result.archived_definitions += 1;
            result.formula_derivation_ops.push(DC_INTRO_DEF);

            if polarity == 0 {
                let real_definition_id = neutral_wrapper.ident();
                defs.get_mut(&entry_no)
                    .unwrap_or_else(|| panic!("definition {entry_no} disappeared"))
                    .set_definition_metadata(real_definition_id, archived_formula);
                self.insert(neutral_wrapper);
                result.active_definitions_inserted += 1;
                result.formula_derivation_ops.push(DC_FOF_QUOTE);
            } else {
                let active_def = tformula_create_def(bank, &def_atom, &form, polarity)?;
                let active_wrapper = WrappedFormula::wt_formula_alloc(active_def);
                let real_definition_id = active_wrapper.ident();
                defs.get_mut(&entry_no)
                    .unwrap_or_else(|| panic!("definition {entry_no} disappeared"))
                    .set_definition_metadata(real_definition_id, archived_formula);
                self.insert(active_wrapper);
                result.active_definitions_inserted += 1;
                result.formula_derivation_ops.push(DC_SPLIT_EQUIV);
            }
        }

        for formula in &mut self.formulas {
            let defs_used = formula.apply_defs(bank, &defs)?;
            if !defs_used.is_empty() {
                result.formulas_rewritten += 1;
                let used_count = usize_to_i64(defs_used.len());
                result.definition_applications += used_count;
                result
                    .formula_derivation_ops
                    .extend(std::iter::repeat_n(DC_APPLY_DEF, defs_used.len()));
            }
        }

        Ok(result)
    }

    /// Drains this set into CNF clauses using the staged core of C
    /// `FormulaSetCNF2`.
    ///
    /// This preserves the supported C phase order: optional set-level FOOL
    /// unrolling, formula simplification, definition introduction, then the
    /// archive/copy/CNF drain loop. Higher-order set preprocessing, post-CNF
    /// clause lambda lifting, proof-document output, and term-bank GC side
    /// effects from full `FormulaSetCNF2` are still deferred.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if any wrapped formula cannot be transformed into
    /// clauses.
    ///
    /// # Panics
    ///
    /// Panics under the same malformed-formula preconditions as
    /// [`WrappedFormula::cnf2_into`].
    pub fn cnf2_into(
        &mut self,
        archive: &mut Self,
        clauseset: &mut ClauseSet,
        bank: &mut TermBank,
        fresh_vars: &VarBank,
        options: FormulaSetCnfOptions,
    ) -> Result<FormulaSetCnfResult, Diagnostic> {
        let mut result = FormulaSetCnfResult::default();
        let mut old_nodes = bank.non_var_term_nodes();
        let mut gc_threshold = formula_set_gc_threshold(old_nodes);

        if options.fool_unroll {
            let unroll_result = self.unroll_fool(bank)?;
            result.boolean_equalities_replaced = unroll_result.boolean_equalities_replaced;
            result.formulas_fool_unrolled = unroll_result.formulas_unrolled;
            result
                .formula_derivation_ops
                .extend(unroll_result.formula_derivation_ops);
        }

        let simplify_result = self.simplify_with_garbage_collection(bank, true)?;
        result.formulas_simplified = simplify_result.formulas_changed;
        result.term_garbage_collections += simplify_result.term_garbage_collections;
        result.terms_recovered_by_gc += simplify_result.terms_recovered_by_gc;
        result
            .formula_derivation_ops
            .extend(simplify_result.formula_derivation_ops);

        let intro_result = self.introduce_defs(archive, bank, options.def_limit)?;
        result.definitions_introduced = intro_result.definitions_introduced;
        result.definition_applications = intro_result.definition_applications;
        result.definition_formulas_archived = intro_result.archived_definitions;
        result.active_definition_formulas_inserted = intro_result.active_definitions_inserted;
        result.formulas_rewritten_by_defs = intro_result.formulas_rewritten;
        result
            .formula_derivation_ops
            .extend(intro_result.formula_derivation_ops);

        while let Some(handle) = self.extract_first() {
            let source = FormulaDerivationRef::new(handle.ident());
            let mut form = handle.flat_copy();
            archive.insert(handle);
            result.original_formulas_archived += 1;
            result.quoted_formula_sources.push(source);

            let cnf_result = form.cnf2_into(
                bank,
                clauseset,
                fresh_vars,
                options.miniscope_limit,
                options.fool_unroll,
                options.problem_type,
            )?;
            result.clauses_generated += cnf_result.clauses_generated;
            result
                .formula_derivation_ops
                .extend(cnf_result.formula_derivation_ops);

            let cnf_copy_has_formula = form.formula.is_some();
            archive.insert(form);
            result.cnf_formulas_archived += 1;
            if cnf_copy_has_formula && bank.non_var_term_nodes() > gc_threshold {
                collect_formula_set_cnf_garbage(bank, self, archive, clauseset, &mut result);
                old_nodes = bank.non_var_term_nodes();
                gc_threshold = formula_set_gc_threshold(old_nodes);
            }
        }

        if bank.non_var_term_nodes() != old_nodes {
            collect_formula_set_cnf_garbage(bank, self, archive, clauseset, &mut result);
        }

        Ok(result)
    }

    /// Renders C's `FormulaSetPrint` dispatch in formula insertion order.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if a wrapped formula cannot be rendered.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term.
    pub fn print_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        output_format: FormulaPrintFormat,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        let mut output = String::new();
        for formula in &self.formulas {
            output.push_str(&formula.print_string(
                bank,
                full_terms,
                problem_type,
                output_format,
                keep_input_names,
            )?);
            output.push('\n');
        }
        Ok(output)
    }

    /// Renders C's `FormulaSetPrettyPrintTSTP` in formula insertion order.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if type declarations or a wrapped formula cannot
    /// be rendered.
    ///
    /// # Panics
    ///
    /// Panics if any wrapper has no formula term.
    pub fn pretty_print_tstp_string(
        &self,
        bank: &mut TermBank,
        full_terms: bool,
        problem_type: ProblemType,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        let mut output = if self.is_untyped() {
            String::new()
        } else {
            let mut declarations = Vec::new();
            bank.signature()
                .print_type_decls_tstp(&mut declarations, problem_type)
                .map_err(|_| formula_set_write_error("failed to write TSTP type declarations"))?;
            String::from_utf8(declarations).map_err(|_| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "TSTP type declarations are not valid UTF-8",
                )
            })?
        };

        for formula in &self.formulas {
            output.push_str(&formula.tstp_string_flex(
                bank,
                problem_type,
                FormulaTstpPrintOptions::complete_formula(full_terms, keep_input_names),
            )?);
            output.push('\n');
        }
        Ok(output)
    }

    /// Renders C's `FormulaSetAppEncode` output for this formula set.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if type preloading, declaration printing, or
    /// formula application encoding fails.
    ///
    /// # Panics
    ///
    /// Panics under the same malformed-formula conditions as the underlying
    /// term-formula app-encoding helpers.
    pub fn app_encode_string(
        &self,
        bank: &mut TermBank,
        problem_type: ProblemType,
        keep_input_names: bool,
    ) -> Result<String, Diagnostic> {
        if self.formulas.is_empty() {
            return Ok(String::new());
        }

        for formula in &self.formulas {
            tformula_preload_types(bank, formula.formula())?;
        }

        let mut output = Vec::new();
        bank.signature()
            .type_bank()
            .app_encode_types(&mut output, problem_type, true)
            .map_err(|_| {
                formula_set_write_error("failed to write app-encoded type declarations")
            })?;
        bank.signature()
            .print_app_encoded_decls(&mut output)
            .map_err(|_| {
                formula_set_write_error("failed to write app-encoded symbol declarations")
            })?;
        let mut output = String::from_utf8(output).map_err(|_| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "app-encoded declarations are not valid UTF-8",
            )
        })?;

        for formula in &self.formulas {
            if !tformula_is_prop_true(bank, formula.formula()) {
                output.push_str(&formula.app_encode_string(bank, keep_input_names)?);
                output.push('\n');
            }
        }
        Ok(output)
    }
}

#[must_use]
pub fn formula_set_stack_cardinality(stack: &[&FormulaSet]) -> i64 {
    stack.iter().map(|set| set.cardinality()).sum()
}

pub fn formula_stack_cond_set_type(stack: &mut [WrappedFormula], type_: FormulaProperties) {
    for formula in stack {
        if formula.query_tptp_type() != CP_TYPE_CONJECTURE || type_ == CP_TYPE_CONJECTURE {
            formula.set_tptp_type(type_);
        }
    }
}

#[must_use]
pub fn formula_set_definition_statistics(
    orig: &FormulaSet,
    arch: &FormulaSet,
    bank: &TermBank,
) -> FormulaDefinitionStatistics {
    let mut num_defs = 0_i32;
    let mut form_defs = 0_i32;
    let mut num_lams = 0_i32;
    let mut has_app_var_lits = false;

    for set in [orig, arch] {
        for formula in set.iter() {
            has_app_var_lits |= formula.has_app_var_literal(bank);
            num_lams =
                num_lams.saturating_add(formula.count_non_top_level_lambdas(bank.signature()));

            if let Some(head) = formula.lambda_definition_head_for_statistics(bank) {
                num_defs = num_defs.saturating_add(1);
                if head.type_().as_ref().is_some_and(type_is_predicate) {
                    form_defs = form_defs.saturating_add(1);
                }
            }
        }
    }

    FormulaDefinitionStatistics {
        num_defs,
        percentage_form_defs: if num_defs == 0 {
            0.0
        } else {
            f64::from(form_defs) / f64::from(num_defs)
        },
        num_lams,
        has_app_var_lits,
    }
}

fn next_entry_id() -> u64 {
    WRAPPED_FORMULA_ENTRY_ID.fetch_add(1, Ordering::Relaxed)
}

fn next_formula_ident() -> i64 {
    loop {
        let current = FORMULA_IDENT_COUNTER.load(Ordering::Relaxed);
        let next = current
            .checked_add(1)
            .unwrap_or_else(|| panic!("formula ident counter overflow"));
        if FORMULA_IDENT_COUNTER
            .compare_exchange(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return next;
        }
    }
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn tformula_count_non_top_level_lambdas(signature: &Signature, formula: &Term) -> i32 {
    let mut stack = vec![(formula.clone(), true)];
    let mut result = 0_i32;

    while let Some((formula, mut is_at_top)) = stack.pop() {
        if !formula.has_lambda_subterm() {
            continue;
        }

        if is_at_top {
            is_at_top = !formula.is_free_var()
                && ((formula.f_code() > 0 && signature.is_logical_symbol(formula.f_code()))
                    || formula.is_lambda());
        } else if formula.is_lambda() {
            result = result.saturating_add(1);
        }

        let start = usize::from(formula.is_phony_app() || formula.is_lambda());
        for index in start..formula.arity() {
            let child = formula
                .argument(index)
                .unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
            if child.has_lambda_subterm() {
                stack.push((child, is_at_top));
            }
        }
    }

    result
}

fn tformula_has_app_var_literal(bank: &TermBank, formula: &Term) -> bool {
    let mut stack = vec![formula.clone()];

    while let Some(formula) = stack.pop() {
        if tformula_is_literal(bank, &formula) {
            let left_has_app_var = formula
                .argument(0)
                .is_some_and(|term| term.is_applied_free_var());
            let right_has_app_var = formula
                .argument(1)
                .is_some_and(|term| term.is_applied_free_var());
            if left_has_app_var || right_has_app_var {
                return true;
            }
        } else if formula.f_code() > 0 && bank.signature().is_logical_symbol(formula.f_code()) {
            stack.extend(formula.argument_clones().into_iter().flatten());
        }
    }

    false
}

fn tformula_annotate_question(bank: &mut TermBank, formula: &Term) -> Result<Term, Diagnostic> {
    let qex_code = bank.signature().qex_code();
    let mut variables = Vec::new();
    let mut handle = formula.clone();
    while handle.f_code() == qex_code && handle.arity() == 2 {
        variables.push(
            handle
                .argument(0)
                .unwrap_or_else(|| panic!("existential quantifier variable is uninitialized")),
        );
        handle = handle
            .argument(1)
            .unwrap_or_else(|| panic!("existential quantifier body is uninitialized"));
    }
    if variables.is_empty() {
        return Ok(formula.clone());
    }

    let answer = answer_lit_alloc(bank, &variables)?;
    let mut result = tformula_fcode_alloc(bank, bank.signature().and_code(), handle, Some(answer))?;
    while let Some(variable) = variables.pop() {
        result = tformula_fcode_alloc(bank, qex_code, variable, Some(result))?;
    }
    Ok(result)
}

fn answer_lit_alloc(bank: &mut TermBank, variables: &[Term]) -> Result<Term, Diagnostic> {
    let answer_payload = bank.alloc_new_skolem(variables, None)?;
    let answer = Term::top_alloc(bank.signature().answer_code(), 1);
    answer.set_type(Some(bank.signature().type_bank().bool_type()));
    answer.set_argument(0, answer_payload);
    let answer = bank.term_top_insert(answer)?;
    let true_term = bank.true_term().clone();
    let neqn_code = bank.signature_mut().get_eqn_code(false);
    tformula_fcode_alloc(bank, neqn_code, answer, Some(true_term))
}

fn tformula_replace_eqn_with_equiv(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    if form.is_db_var() || !form.has_eq_neq() || form.is_any_var() {
        return Ok(form.clone());
    }

    let copy = Term::top_copy_without_args(form);
    for (index, arg) in form.argument_clones().into_iter().enumerate() {
        let arg = arg.unwrap_or_else(|| panic!("formula argument {index} is uninitialized"));
        copy.set_argument(index, tformula_replace_eqn_with_equiv(bank, &arg)?);
    }
    let current = bank.term_top_insert(copy)?;
    rewrite_bool_eqn_root(bank, &current)
}

fn rewrite_bool_eqn_root(bank: &mut TermBank, form: &Term) -> Result<Term, Diagnostic> {
    let (eqn_code, neqn_code, equiv_code, xor_code, not_code) = {
        let signature = bank.signature();
        (
            signature.eqn_code(),
            signature.neqn_code(),
            signature.equiv_code(),
            signature.xor_code(),
            signature.not_code(),
        )
    };
    if form.arity() != 2 || (form.f_code() != eqn_code && form.f_code() != neqn_code) {
        return Ok(form.clone());
    }

    let left = form
        .argument(0)
        .unwrap_or_else(|| panic!("Boolean equality left argument is uninitialized"));
    let right = form
        .argument(1)
        .unwrap_or_else(|| panic!("Boolean equality right argument is uninitialized"));
    if !tformula_is_complex_bool(bank, &left) || !tformula_is_complex_bool(bank, &right) {
        return Ok(form.clone());
    }

    let true_term = bank.true_term().clone();
    if form.f_code() == eqn_code {
        if right != true_term {
            return tformula_fcode_alloc(bank, equiv_code, left, Some(right));
        }
        if left != true_term {
            return Ok(left);
        }
    } else if right != true_term {
        return tformula_fcode_alloc(bank, xor_code, left, Some(right));
    } else if left != true_term {
        return tformula_fcode_alloc(bank, not_code, left, None);
    }
    Ok(form.clone())
}

fn formula_set_write_error(message: &'static str) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        formula_set_definition_statistics, formula_set_stack_cardinality,
        formula_stack_cond_set_type, FormulaDefinitionStatistics, FormulaPrintFormat, FormulaSet,
        FormulaSetCnfOptions, FormulaTstpClauseMode, FormulaTstpPrintOptions, WrappedFormula,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_IGNORE_PROPS, CP_INPUT_FORMULA, CP_IS_LAMBDA_DEF, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE,
        CP_TYPE_HYPOTHESIS, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION,
    };
    use crate::clauses::clausefunc::{tformula_clause_encode, tformula_decode_polarity};
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        DerivationEntry, FormulaDerivationRef, DC_ANNO_QUESTION, DC_APPLY_DEF,
        DC_DIST_DISJUNCTIONS, DC_EQ_TO_EQ, DC_FOF_QUOTE, DC_FOF_SIMPLIFY, DC_FOOL_UNROLL,
        DC_INTRO_DEF, DC_NEGATE_CONJECTURE, DC_SPLIT_CONJUNCT, DC_SPLIT_EQUIV,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::terms::lambda::close_with_db_var;
    use crate::terms::signature::{
        Signature, SIG_DB_LAMBDA_CODE, SIG_PHONY_APP_CODE, SIG_TRUE_CODE,
    };
    use crate::terms::simpletypes::{alloc_arrow_type, alloc_simple_sort, Type, ST_INTEGER};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termfunc::term_standard_weight;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::termvars::VarBank;
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_const_with_type(bank, name, &type_)
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_var(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        typed_unary_with_types(bank, name, arg, &type_, &type_)
    }

    fn typed_unary_with_types(
        bank: &mut TermBank,
        name: &str,
        arg: &Term,
        arg_type: &crate::terms::simpletypes::Type,
        ret_type: &crate::terms::simpletypes::Type,
    ) -> Term {
        let arrow = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), ret_type.clone()]));
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, arrow)
            .unwrap();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(ret_type.clone()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn typed_predicate_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_)
            .unwrap();
        bank.signature_mut().declare_is_predicate(f_code).unwrap();
        bank.create_const_term(f_code).unwrap()
    }

    fn bool_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn c_complex_bool_shape(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let payload = typed_const(bank, name);
        let term = Term::top_alloc(SIG_TRUE_CODE, 1);
        term.set_type(Some(type_));
        term.set_argument(0, payload);
        bank.term_top_insert(term).unwrap()
    }

    fn eqn(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn quantified_with_code(
        bank: &mut TermBank,
        f_code: i64,
        variable: &Term,
        body: &Term,
    ) -> Term {
        let type_ = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, variable.clone());
        term.set_argument(1, body.clone());
        bank.term_top_insert(term).unwrap()
    }

    fn phony_app(bank: &mut TermBank, head: &Term, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(SIG_PHONY_APP_CODE, 2);
        term.set_type(Some(type_));
        term.set_argument(0, head.clone());
        term.set_argument(1, arg.clone());
        bank.term_top_insert(term).unwrap()
    }

    #[test]
    fn wrapped_formula_allocation_defaults_and_ids_match_c_shape() {
        let mut bank = test_bank();
        let atom = typed_const(&mut bank, "wf_atom");
        let default = WrappedFormula::default_alloc();

        assert_eq!(default.properties(), CP_IGNORE_PROPS);
        assert!(!default.is_clause());
        assert_eq!(default.ident(), 0);
        assert_eq!(default.info(), None);
        assert_eq!(default.get_id(false), "c_0_0");

        let mut formula = WrappedFormula::wt_formula_alloc(atom);
        assert!(formula.get_id(false).starts_with("i_0_"));
        formula.set_info(Some(ClauseInfo::new(
            Some("input_formula_name"),
            Some("problem.p"),
            7,
            3,
        )));
        assert_eq!(formula.get_id(true), "input_formula_name");
        assert!(formula.get_id(false).starts_with("i_0_"));
    }

    #[test]
    fn wrapped_formula_flat_copy_preserves_formula_state_but_not_source_info() {
        let mut bank = test_bank();
        let atom = typed_const(&mut bank, "wf_copy_atom");
        let mut formula = WrappedFormula::wt_formula_alloc(atom);
        formula.set_tptp_type(CP_TYPE_CONJECTURE);
        formula.set_prop(CP_INPUT_FORMULA);
        formula.set_is_clause(true);
        formula.set_info(Some(ClauseInfo::new(Some("copy_source"), None, 1, 1)));

        let copied = formula.flat_copy();

        assert_ne!(copied.entry_id(), formula.entry_id());
        assert_eq!(copied.ident(), formula.ident());
        assert_eq!(copied.properties(), formula.properties());
        assert!(copied.is_clause());
        assert_eq!(copied.info(), None);
        assert_eq!(copied.formula(), formula.formula());
    }

    #[test]
    fn formula_set_insert_extract_and_move_preserve_c_list_order() {
        let mut bank = test_bank();
        let first = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_first"));
        let second = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_second"));
        let third = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "set_third"));
        let first_id = first.entry_id();
        let second_id = second.entry_id();
        let third_id = third.entry_id();
        let mut set = FormulaSet::new();

        assert!(set.is_empty());
        assert_eq!(set.insert(first), first_id);
        assert_eq!(set.insert(second), second_id);
        assert_eq!(set.cardinality(), 2);
        assert_eq!(
            set.iter().map(WrappedFormula::entry_id).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );

        assert_eq!(set.extract_first().unwrap().entry_id(), first_id);
        assert_eq!(set.cardinality(), 1);
        assert_eq!(set.extract_entry(second_id).unwrap().entry_id(), second_id);
        assert!(set.extract_first().is_none());

        let mut from = FormulaSet::new();
        from.insert(WrappedFormula::wt_formula_alloc(typed_const(
            &mut bank,
            "move_first",
        )));
        from.insert(third);
        let moved_first_id = from.iter().next().unwrap().entry_id();
        let mut to = FormulaSet::new();

        assert!(to.move_formula_from(&mut from, third_id));
        assert_eq!(from.cardinality(), 1);
        assert_eq!(to.cardinality(), 1);
        assert_eq!(to.iter().next().unwrap().entry_id(), third_id);

        assert_eq!(to.insert_set(&mut from), 1);
        assert!(from.is_empty());
        assert_eq!(
            to.iter().map(WrappedFormula::entry_id).collect::<Vec<_>>(),
            vec![third_id, moved_first_id]
        );
        assert_eq!(formula_set_stack_cardinality(&[&to, &from]), 2);
    }

    #[test]
    fn formula_set_archive_moves_originals_and_replaces_flat_copies() {
        let mut bank = test_bank();
        let first_term = typed_const(&mut bank, "archive_first");
        let second_term = typed_const(&mut bank, "archive_second");
        let mut first = WrappedFormula::wt_formula_alloc(first_term.clone());
        first.set_tptp_type(CP_TYPE_AXIOM);
        first.set_info(Some(ClauseInfo::new(Some("archive_name"), None, 1, 1)));
        let first_entry = first.entry_id();
        let first_source = FormulaDerivationRef::new(first.ident());
        let mut second = WrappedFormula::wt_formula_alloc(second_term.clone());
        second.set_is_clause(true);
        let second_entry = second.entry_id();
        let second_source = FormulaDerivationRef::new(second.ident());
        let mut set = FormulaSet::new();
        set.insert(first);
        set.insert(second);
        let mut archive = FormulaSet::new();

        let result = set.archive_into(&mut archive);

        assert_eq!(result.formulas_archived, 2);
        assert_eq!(
            result.quoted_formula_sources,
            vec![first_source, second_source]
        );
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_FOF_QUOTE, DC_FOF_QUOTE]
        );
        assert_eq!(
            archive
                .iter()
                .map(WrappedFormula::entry_id)
                .collect::<Vec<_>>(),
            vec![first_entry, second_entry]
        );
        let copied = set.iter().collect::<Vec<_>>();
        assert_eq!(copied.len(), 2);
        assert_ne!(copied[0].entry_id(), first_entry);
        assert_ne!(copied[1].entry_id(), second_entry);
        assert_eq!(copied[0].ident(), first_source.ident());
        assert_eq!(copied[1].ident(), second_source.ident());
        assert_eq!(copied[0].query_tptp_type(), CP_TYPE_AXIOM);
        assert!(copied[1].is_clause());
        assert_eq!(copied[0].info(), None);
        assert_eq!(copied[0].formula(), &first_term);
        assert_eq!(copied[1].formula(), &second_term);
    }

    #[test]
    fn formula_set_type_queries_and_counts_match_c_macros() {
        let mut bank = test_bank();
        let mut axiom = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_axiom"));
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut hypothesis = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_hyp"));
        hypothesis.set_tptp_type(CP_TYPE_HYPOTHESIS);
        let mut conjecture = WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_conj"));
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut question =
            WrappedFormula::wt_formula_alloc(typed_const(&mut bank, "count_question"));
        question.set_tptp_type(CP_TYPE_QUESTION);
        let mut set = FormulaSet::new();
        set.insert(axiom);
        set.insert(hypothesis);
        set.insert(conjecture);
        set.insert(question);

        let (count, conjectures, rest) = set.split_conjectures();
        assert_eq!(count, 2);
        assert_eq!(conjectures.len(), 2);
        assert_eq!(rest.len(), 2);

        let mut hypothesis_count = 5;
        assert_eq!(set.count_conjectures(&mut hypothesis_count), 2);
        assert_eq!(hypothesis_count, 6);

        let mut stack = set.iter().cloned().collect::<Vec<_>>();
        formula_stack_cond_set_type(&mut stack, CP_TYPE_HYPOTHESIS);
        assert_eq!(stack[0].query_tptp_type(), CP_TYPE_HYPOTHESIS);
        assert_eq!(stack[2].query_tptp_type(), CP_TYPE_CONJECTURE);
        formula_stack_cond_set_type(&mut stack, CP_TYPE_CONJECTURE);
        assert!(stack
            .iter()
            .all(|formula| formula.query_tptp_type() == CP_TYPE_CONJECTURE));
    }

    #[test]
    fn formula_set_weight_fcode_and_symbol_queries_use_formula_terms() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "formula_set_a");
        let f_a = typed_unary(&mut bank, "formula_set_f", &a);
        let mut normal = WrappedFormula::wt_formula_alloc(f_a.clone());
        normal.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut lambda_def = WrappedFormula::wt_formula_alloc(a.clone());
        lambda_def.set_prop(CP_IS_LAMBDA_DEF);
        let normal_id = normal.entry_id();
        let mut set = FormulaSet::new();
        set.insert(normal);
        set.insert(lambda_def);

        assert_eq!(set.standard_weight(), term_standard_weight(&f_a));
        let mut result = Vec::new();
        assert_eq!(set.collect_f_code(f_a.f_code(), &mut result), 1);
        assert_eq!(result, vec![normal_id]);
        assert_eq!(set.conjecture_order(bank.signature()), 0);
        assert!(!set.has_interpreted_symbol());
        assert!(set.is_untyped());

        let int_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_simple_sort(ST_INTEGER));
        let int_code = bank.signature_mut().insert_id("formula_set_int", 0, false);
        bank.signature_mut()
            .declare_final_type(int_code, int_type)
            .unwrap();
        let int_term = bank.create_const_term(int_code).unwrap();
        set.insert(WrappedFormula::wt_formula_alloc(int_term));
        assert!(set.has_interpreted_symbol());
        assert!(!set.is_untyped());
    }

    #[test]
    fn formula_set_mark_polarity_marks_each_wrapped_formula_like_c() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "pol_first");
        let second = typed_const(&mut bank, "pol_second");
        let third = typed_const(&mut bank, "pol_third");
        let fourth = typed_const(&mut bank, "pol_fourth");
        let fifth = typed_const(&mut bank, "pol_fifth");
        let sixth = typed_const(&mut bank, "pol_sixth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let impl_code = bank.signature().impl_code();
        let or_code = bank.signature().or_code();
        let first_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let second_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let third_atom = bool_binary_with_code(&mut bank, eqn_code, &fifth, &sixth);
        let left_disjunction = bool_binary_with_code(&mut bank, or_code, &first_atom, &second_atom);
        let implication =
            bool_binary_with_code(&mut bank, impl_code, &left_disjunction, &third_atom);
        let disjunction = bool_binary_with_code(&mut bank, or_code, &second_atom, &third_atom);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::default_alloc());
        set.insert(WrappedFormula::wt_formula_alloc(implication.clone()));
        set.insert(WrappedFormula::wt_formula_alloc(disjunction.clone()));

        set.mark_polarity(&bank);

        assert_eq!(tformula_decode_polarity(&implication), 1);
        assert_eq!(tformula_decode_polarity(&left_disjunction), -1);
        assert_eq!(tformula_decode_polarity(&disjunction), 1);
    }

    #[test]
    fn wrapped_formula_return_f_codes_preserves_order_and_skips_phony_apps() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "fc_first");
        let second = typed_const(&mut bank, "fc_second");
        let third = typed_const(&mut bank, "fc_third");
        let var = typed_var(&bank, -41);
        let applied = phony_app(&mut bank, &var, &third);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let pair = bool_binary_with_code(&mut bank, and_code, &first, &applied);
        let formula = bool_binary_with_code(&mut bank, or_code, &pair, &second);
        let wrapped = WrappedFormula::wt_formula_alloc(formula);
        let mut f_codes = vec![999_999];

        assert_eq!(wrapped.return_f_codes(&mut f_codes), 5);
        assert_eq!(
            f_codes,
            vec![
                999_999,
                or_code,
                and_code,
                first.f_code(),
                third.f_code(),
                second.f_code(),
            ]
        );
        assert_eq!(wrapped.symbol_diversity(), 5);
        assert!(!f_codes.contains(&SIG_PHONY_APP_CODE));
    }

    #[test]
    fn wrapped_formula_get_lambda_defined_symbol_matches_c_shapes() {
        let mut bank = test_bank();
        let head = typed_const(&mut bank, "lambda_defined_head");
        let rhs = typed_const(&mut bank, "lambda_defined_rhs");
        let quantified_var = typed_var(&bank, -42);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let qall_code = bank.signature().qall_code();
        let equiv_code = bank.signature().equiv_code();
        let equation = bool_binary_with_code(&mut bank, eqn_code, &head, &rhs);
        let quantified = quantified_with_code(&mut bank, qall_code, &quantified_var, &equation);
        let mut equation_definition = WrappedFormula::wt_formula_alloc(quantified);
        equation_definition.set_prop(CP_IS_LAMBDA_DEF);

        assert_eq!(
            equation_definition.get_lambda_defined_symbol(bank.signature()),
            Some(head.f_code())
        );

        let true_term = bank.true_term().clone();
        let predicate_equation = bool_binary_with_code(&mut bank, eqn_code, &head, &true_term);
        let true_formula = bank.true_term().clone();
        let equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &predicate_equation, &true_formula);
        let mut equivalence_definition = WrappedFormula::wt_formula_alloc(equivalence);
        equivalence_definition.set_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(
            equivalence_definition.get_lambda_defined_symbol(bank.signature()),
            Some(head.f_code())
        );

        let mut untagged = equivalence_definition.flat_copy();
        untagged.del_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(untagged.get_lambda_defined_symbol(bank.signature()), None);
    }

    #[test]
    fn wrapped_formula_app_encode_renders_c_role_and_id_shape() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_app_a");
        let f_a = typed_unary(&mut bank, "wf_app_f", &a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &f_a, &a);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        wrapped.set_info(Some(ClauseInfo::new(Some("wf_app_named"), None, 1, 1)));

        let rendered = wrapped.app_encode_string(&mut bank, true).unwrap();

        assert!(rendered.starts_with("tff(wf_app_named, axiom, "));
        assert!(rendered.ends_with(")."));
        assert!(rendered.contains("wf_app_a"));
    }

    #[test]
    fn wrapped_formula_tptp_and_tstp_printers_match_c_roles_and_spacing() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_print_a");
        let b = typed_const(&mut bank, "wf_print_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_info(Some(ClauseInfo::new(Some("wf_print_neg"), None, 7, 3)));

        let legacy_output = wrapped
            .tptp_string(&mut bank, true, ProblemType::FirstOrder, true)
            .unwrap();
        let tstp_output = wrapped
            .tstp_string(&mut bank, true, true, ProblemType::FirstOrder, true)
            .unwrap();
        let incomplete_tstp = wrapped
            .tstp_string_flex(
                &mut bank,
                ProblemType::FirstOrder,
                FormulaTstpPrintOptions::open_formula(true, true),
            )
            .unwrap();

        assert!(legacy_output.starts_with("input_formula(wf_print_neg,conjecture,"));
        assert!(legacy_output.contains("wf_print_a"));
        assert!(legacy_output.contains("wf_print_b"));
        assert!(legacy_output.ends_with(")."));
        assert!(tstp_output.starts_with("fof(wf_print_neg, negated_conjecture, "));
        assert!(tstp_output.contains("wf_print_a=wf_print_b"));
        assert!(tstp_output.ends_with(")."));
        assert!(incomplete_tstp.starts_with("fof(wf_print_neg, negated_conjecture, "));
        assert!(!incomplete_tstp.ends_with(")."));
    }

    #[test]
    fn wrapped_formula_clause_conversion_preserves_properties_and_source_info() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_clause_a");
        let b = typed_const(&mut bank, "wf_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &a, false),
        ]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_prop(CP_INPUT_FORMULA);
        wrapped.set_info(Some(ClauseInfo::new(
            Some("wrapped_clause"),
            Some("source.p"),
            4,
            2,
        )));

        let converted = wrapped.form_clause_to_clause(&mut bank).unwrap();

        assert_eq!(converted.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert!(converted.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            converted.info().and_then(ClauseInfo::name),
            Some("wrapped_clause")
        );
        assert_eq!(
            converted.info().and_then(ClauseInfo::source),
            Some("source.p")
        );
        assert_eq!(converted.literal_number(), 2);
        assert_eq!(converted.positive_literal_count(), 1);
        assert_eq!(converted.negative_literal_count(), 1);
    }

    #[test]
    fn wrapped_formula_of_clause_closes_formula_and_drops_clause_metadata() {
        let mut bank = test_bank();
        let x = typed_var(&bank, -2);
        let a = typed_const(&mut bank, "wf_of_clause_a");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &x, &a, true)]));
        clause.set_tptp_type(CP_TYPE_AXIOM);
        clause.set_prop(CP_INPUT_FORMULA);
        clause.set_info(Some(ClauseInfo::new(Some("source_clause"), None, 1, 1)));

        let wrapped = WrappedFormula::of_clause(&mut bank, &clause, ProblemType::FirstOrder)
            .expect("clause can be encoded as a closed formula");
        let rendered = wrapped
            .tstp_string(&mut bank, true, true, ProblemType::FirstOrder, true)
            .unwrap();

        assert!(!wrapped.is_clause());
        assert_eq!(wrapped.properties(), CP_IGNORE_PROPS);
        assert_eq!(wrapped.info(), None);
        assert!(rendered.contains("plain, ![X1]:(X1=wf_of_clause_a)"));
    }

    #[test]
    fn wrapped_formula_print_string_dispatches_clause_wrappers_like_clause_print() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_print_clause_a");
        let b = typed_const(&mut bank, "wf_print_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &a, false),
        ]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_info(Some(ClauseInfo::new(
            Some("wrapped_print_clause"),
            None,
            1,
            1,
        )));

        let lop = wrapped
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Lop,
                true,
            )
            .unwrap();
        let old_tptp_output = wrapped
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Tptp,
                true,
            )
            .unwrap();
        let structured_output = wrapped
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Tstp,
                true,
            )
            .unwrap();

        assert_eq!(
            lop,
            "?- wf_print_clause_a!=wf_print_clause_b, wf_print_clause_b=wf_print_clause_a."
        );
        assert!(old_tptp_output.starts_with("input_clause("));
        assert!(old_tptp_output.contains(",conjecture,["));
        assert!(old_tptp_output.contains("++equal(wf_print_clause_a, wf_print_clause_b)"));
        assert!(old_tptp_output.contains("--equal(wf_print_clause_b, wf_print_clause_a)"));
        assert!(structured_output.starts_with("cnf("));
        assert!(structured_output.contains(", negated_conjecture, "));
        assert!(structured_output.contains("(wf_print_clause_a=wf_print_clause_b|"));
        assert!(structured_output.ends_with(")."));
    }

    #[test]
    fn wrapped_formula_tstp_flex_handles_clause_as_formula_and_core_modes() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_flex_clause_a");
        let b = typed_const(&mut bank, "wf_flex_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![
            eqn(&mut bank, &a, &b, true),
            eqn(&mut bank, &b, &a, false),
        ]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        wrapped.set_info(Some(ClauseInfo::new(Some("flex_clause"), None, 1, 1)));

        let as_formula = wrapped
            .tstp_string_flex(
                &mut bank,
                ProblemType::FirstOrder,
                FormulaTstpPrintOptions::complete_formula(true, true),
            )
            .unwrap();
        let as_clause_core = wrapped
            .tstp_string_flex(
                &mut bank,
                ProblemType::FirstOrder,
                FormulaTstpPrintOptions::complete_formula(true, true)
                    .with_clause_mode(FormulaTstpClauseMode::AsClauseCore),
            )
            .unwrap();

        assert!(as_formula.starts_with("fof(flex_clause, negated_conjecture, "));
        assert!(as_formula.contains("wf_flex_clause_a=wf_flex_clause_b"));
        assert!(as_formula.ends_with(")."));
        assert!(as_clause_core.starts_with("cnf(flex_clause, negated_conjecture, ("));
        assert!(as_clause_core.contains("wf_flex_clause_a=wf_flex_clause_b|"));
        assert!(as_clause_core.ends_with("))."));
    }

    #[test]
    fn wrapped_formula_simplify_updates_formula_like_c() {
        let mut bank = test_bank();
        let atom_left = typed_const(&mut bank, "wf_simpl_left");
        let atom_right = typed_const(&mut bank, "wf_simpl_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &atom_left, &atom_right);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let disjunction = bool_binary_with_code(&mut bank, or_code, &false_formula, &atom);
        let mut wrapped = WrappedFormula::wt_formula_alloc(disjunction);

        assert!(wrapped.simplify(&mut bank).unwrap());
        assert_eq!(wrapped.formula(), &atom);
        assert!(!wrapped.simplify(&mut bank).unwrap());
    }

    #[test]
    fn formula_set_simplify_counts_changes_and_preserves_order() {
        let mut bank = test_bank();
        let changed_left = typed_const(&mut bank, "set_simpl_changed_left");
        let changed_right = typed_const(&mut bank, "set_simpl_changed_right");
        let stable_left = typed_const(&mut bank, "set_simpl_stable_left");
        let stable_right = typed_const(&mut bank, "set_simpl_stable_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let changed_atom =
            bool_binary_with_code(&mut bank, eqn_code, &changed_left, &changed_right);
        let stable_atom = bool_binary_with_code(&mut bank, eqn_code, &stable_left, &stable_right);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let changed_formula =
            bool_binary_with_code(&mut bank, or_code, &false_formula, &changed_atom);
        let mut changed = WrappedFormula::wt_formula_alloc(changed_formula);
        let changed_entry = changed.entry_id();
        changed.set_tptp_type(CP_TYPE_AXIOM);
        let stable = WrappedFormula::wt_formula_alloc(stable_atom.clone());
        let stable_entry = stable.entry_id();
        let mut set = FormulaSet::new();
        set.insert(changed);
        set.insert(stable);

        let result = set.simplify(&mut bank).unwrap();

        assert_eq!(result.formulas_changed, 1);
        assert_eq!(result.term_garbage_collections, 0);
        assert_eq!(result.terms_recovered_by_gc, 0);
        assert_eq!(result.formula_derivation_ops, vec![DC_FOF_SIMPLIFY]);
        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas[0].entry_id(), changed_entry);
        assert_eq!(formulas[0].query_tptp_type(), CP_TYPE_AXIOM);
        assert_eq!(formulas[0].formula(), &changed_atom);
        assert_eq!(formulas[1].entry_id(), stable_entry);
        assert_eq!(formulas[1].formula(), &stable_atom);
    }

    #[test]
    fn formula_set_simplify_with_gc_collects_after_node_growth() {
        let mut bank = test_bank();
        let dropped = typed_const(&mut bank, "set_simpl_gc_dropped");
        let a = typed_const(&mut bank, "set_simpl_gc_a");
        let b = typed_const(&mut bank, "set_simpl_gc_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_prop = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let equiv_code = bank.signature().equiv_code();
        let equiv_false = bool_binary_with_code(&mut bank, equiv_code, &atom, &false_prop);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(equiv_false));

        let result = set
            .simplify_with_garbage_collection(&mut bank, true)
            .unwrap();

        assert_eq!(result.formulas_changed, 1);
        assert!(result.term_garbage_collections >= 1);
        assert!(result.terms_recovered_by_gc >= 1);
        assert!(bank.find(&dropped).is_none());
        let simplified = set.iter().next().unwrap().formula().clone();
        assert_eq!(simplified.f_code(), neqn_code);
        assert!(bank.find(&simplified).is_some());
    }

    fn assert_answer_annotation_shape(bank: &TermBank, annotated: &Term, expected_body: &Term) {
        let qex_code = bank.signature().qex_code();
        assert_eq!(annotated.f_code(), qex_code);
        let first_var = annotated.argument(0).unwrap();
        let second_quantifier = annotated.argument(1).unwrap();
        assert_eq!(second_quantifier.f_code(), qex_code);
        let second_var = second_quantifier.argument(0).unwrap();
        let conjunction = second_quantifier.argument(1).unwrap();
        assert_eq!(conjunction.f_code(), bank.signature().and_code());
        assert_eq!(conjunction.argument(0).as_ref(), Some(expected_body));
        let answer_literal = conjunction.argument(1).unwrap();
        assert_eq!(answer_literal.f_code(), bank.signature().neqn_code());
        assert_eq!(answer_literal.argument(1).as_ref(), Some(bank.true_term()));
        let answer = answer_literal.argument(0).unwrap();
        assert_eq!(answer.f_code(), bank.signature().answer_code());
        let answer_payload = answer.argument(0).unwrap();
        assert_eq!(answer_payload.arity(), 2);
        assert_eq!(answer_payload.argument(0).as_ref(), Some(&first_var));
        assert_eq!(answer_payload.argument(1).as_ref(), Some(&second_var));
    }

    #[test]
    fn wrapped_formula_annotate_question_adds_answer_literal_to_leading_existentials() {
        let mut bank = test_bank();
        let first_var = typed_var(&bank, -401);
        let second_var = typed_var(&bank, -402);
        let left = typed_const(&mut bank, "wf_question_left");
        let right = typed_const(&mut bank, "wf_question_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let body = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let qex_code = bank.signature().qex_code();
        let inner = bool_binary_with_code(&mut bank, qex_code, &second_var, &body);
        let formula = bool_binary_with_code(&mut bank, qex_code, &first_var, &inner);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_QUESTION);

        assert!(wrapped.annotate_question(&mut bank, true, false).unwrap());

        assert_eq!(wrapped.query_tptp_type(), CP_TYPE_CONJECTURE);
        assert_answer_annotation_shape(&bank, wrapped.formula(), &body);
    }

    #[test]
    fn formula_set_preproc_conjectures_annotates_then_negates_in_order() {
        let mut bank = test_bank();
        let q_left = typed_const(&mut bank, "set_preproc_question_left");
        let q_right = typed_const(&mut bank, "set_preproc_question_right");
        let c_left = typed_const(&mut bank, "set_preproc_conj_left");
        let c_right = typed_const(&mut bank, "set_preproc_conj_right");
        let a_left = typed_const(&mut bank, "set_preproc_axiom_left");
        let a_right = typed_const(&mut bank, "set_preproc_axiom_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let question_formula = bool_binary_with_code(&mut bank, eqn_code, &q_left, &q_right);
        let conjecture_formula = bool_binary_with_code(&mut bank, eqn_code, &c_left, &c_right);
        let axiom_formula = bool_binary_with_code(&mut bank, eqn_code, &a_left, &a_right);
        let mut question = WrappedFormula::wt_formula_alloc(question_formula.clone());
        question.set_tptp_type(CP_TYPE_QUESTION);
        let mut conjecture = WrappedFormula::wt_formula_alloc(conjecture_formula.clone());
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut axiom = WrappedFormula::wt_formula_alloc(axiom_formula.clone());
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut set = FormulaSet::new();
        set.insert(question);
        set.insert(conjecture);
        set.insert(axiom);

        let result = set.preproc_conjectures(&mut bank, false, true).unwrap();

        assert_eq!(result.questions_annotated, 2);
        assert_eq!(result.conjectures_negated, 2);
        assert_eq!(
            result.formula_derivation_ops,
            vec![
                DC_ANNO_QUESTION,
                DC_NEGATE_CONJECTURE,
                DC_ANNO_QUESTION,
                DC_NEGATE_CONJECTURE,
            ]
        );
        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas[0].query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(formulas[0].formula().f_code(), bank.signature().not_code());
        assert_eq!(
            formulas[0].formula().argument(0).as_ref(),
            Some(&question_formula)
        );
        assert_eq!(formulas[1].query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
        assert_eq!(
            formulas[1].formula().argument(0).as_ref(),
            Some(&conjecture_formula)
        );
        assert_eq!(formulas[2].query_tptp_type(), CP_TYPE_AXIOM);
        assert_eq!(formulas[2].formula(), &axiom_formula);
    }

    #[test]
    fn wrapped_formula_replace_eqn_with_equiv_matches_c_complex_bool_shape() {
        let mut bank = test_bank();
        let left = c_complex_bool_shape(&mut bank, "wf_eq_to_eq_left");
        let right = c_complex_bool_shape(&mut bank, "wf_eq_to_eq_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &left, &right);
        let mut wrapped = WrappedFormula::wt_formula_alloc(equality);

        assert!(wrapped.replace_eqn_with_equiv(&mut bank).unwrap());

        assert_eq!(wrapped.formula().f_code(), bank.signature().equiv_code());
        assert_eq!(wrapped.formula().argument(0).as_ref(), Some(&left));
        assert_eq!(wrapped.formula().argument(1).as_ref(), Some(&right));
    }

    #[test]
    fn wrapped_formula_unroll_fool_expands_literals_without_counting_mapper_noops() {
        let mut bank = test_bank();
        let left = typed_const(&mut bank, "wf_fool_expand_left");
        let right = typed_const(&mut bank, "wf_fool_expand_right");
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let disequality = bool_binary_with_code(&mut bank, neqn_code, &left, &right);
        let mut wrapped = WrappedFormula::wt_formula_alloc(disequality);

        assert!(!wrapped.unroll_fool(&mut bank).unwrap());

        assert_eq!(wrapped.formula().f_code(), bank.signature().not_code());
        let equality = wrapped.formula().argument(0).unwrap();
        assert_eq!(equality.f_code(), bank.signature().eqn_code());
        assert_eq!(equality.argument(0).as_ref(), Some(&left));
        assert_eq!(equality.argument(1).as_ref(), Some(&right));
    }

    #[test]
    fn formula_set_unroll_fool_replaces_then_unrolls_in_order() {
        let mut bank = test_bank();
        let eq_left = c_complex_bool_shape(&mut bank, "set_fool_eq_left");
        let eq_right = c_complex_bool_shape(&mut bank, "set_fool_eq_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equality = bool_binary_with_code(&mut bank, eqn_code, &eq_left, &eq_right);
        let replacement_only = WrappedFormula::wt_formula_alloc(equality);

        let a = typed_const(&mut bank, "set_fool_a");
        let b = typed_const(&mut bank, "set_fool_b");
        let c = typed_const(&mut bank, "set_fool_c");
        let d = typed_const(&mut bank, "set_fool_d");
        let target = typed_const(&mut bank, "set_fool_target");
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let bool_arg = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let bool_type = bank.signature().type_bank().bool_type();
        let default_type = bank.signature().type_bank().default_type();
        let applied = typed_unary_with_types(
            &mut bank,
            "set_fool_fun",
            &bool_arg,
            &bool_type,
            &default_type,
        );
        let unroll_formula = bool_binary_with_code(&mut bank, eqn_code, &applied, &target);
        let unroll = WrappedFormula::wt_formula_alloc(unroll_formula);

        let stable_left = typed_const(&mut bank, "set_fool_stable_left");
        let stable_right = typed_const(&mut bank, "set_fool_stable_right");
        let stable_formula =
            bool_binary_with_code(&mut bank, eqn_code, &stable_left, &stable_right);
        let stable = WrappedFormula::wt_formula_alloc(stable_formula.clone());

        let mut set = FormulaSet::new();
        set.insert(replacement_only);
        set.insert(unroll);
        set.insert(stable);

        let result = set.unroll_fool(&mut bank).unwrap();

        assert_eq!(result.boolean_equalities_replaced, 1);
        assert_eq!(result.formulas_unrolled, 1);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_EQ_TO_EQ, DC_FOOL_UNROLL]
        );
        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(
            formulas[0].formula().f_code(),
            bank.signature().equiv_code()
        );
        assert_eq!(formulas[1].formula().f_code(), bank.signature().and_code());
        assert_eq!(formulas[2].formula(), &stable_formula);
    }

    #[test]
    fn formula_set_introduce_defs_archives_split_def_and_applies_definition() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "set_intro_def_first");
        let second = typed_const(&mut bank, "set_intro_def_second");
        let third = typed_const(&mut bank, "set_intro_def_third");
        let fourth = typed_const(&mut bank, "set_intro_def_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();

        let result = set.introduce_defs(&mut archive, &mut bank, 1).unwrap();

        assert_eq!(result.definitions_introduced, 1);
        assert_eq!(result.archived_definitions, 1);
        assert_eq!(result.active_definitions_inserted, 1);
        assert_eq!(result.formulas_rewritten, 1);
        assert_eq!(result.definition_applications, 1);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_SPLIT_EQUIV, DC_APPLY_DEF]
        );

        let formulas = set.iter().collect::<Vec<_>>();
        assert_eq!(formulas.len(), 2);
        assert_eq!(archive.cardinality(), 1);

        let rewritten = formulas[0].formula();
        assert_eq!(rewritten.f_code(), bank.signature().or_code());
        let rename_atom = rewritten.argument(0).unwrap();
        assert_eq!(rename_atom.f_code(), bank.signature().eqn_code());
        assert_eq!(rewritten.argument(1).as_ref(), Some(&tail));

        let active_definition = formulas[1].formula();
        assert_eq!(active_definition.f_code(), bank.signature().impl_code());
        assert_eq!(active_definition.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(active_definition.argument(1).as_ref(), Some(&expensive));

        let archived_definition = archive.iter().next().unwrap().formula();
        assert_eq!(archived_definition.f_code(), bank.signature().equiv_code());
        assert_eq!(archived_definition.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(archived_definition.argument(1).as_ref(), Some(&expensive));
    }

    #[test]
    fn formula_set_introduce_defs_keeps_zero_polarity_definition_as_equivalence() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "set_intro_zero_first");
        let second = typed_const(&mut bank, "set_intro_zero_second");
        let third = typed_const(&mut bank, "set_intro_zero_third");
        let fourth = typed_const(&mut bank, "set_intro_zero_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let or_code = bank.signature().or_code();
        let expensive = bool_binary_with_code(&mut bank, or_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let formula = bool_binary_with_code(&mut bank, equiv_code, &expensive, &tail);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();

        let result = set.introduce_defs(&mut archive, &mut bank, 1).unwrap();

        assert_eq!(result.definitions_introduced, 1);
        assert_eq!(
            result.formula_derivation_ops,
            vec![DC_INTRO_DEF, DC_FOF_QUOTE, DC_APPLY_DEF]
        );

        let formulas = set.iter().collect::<Vec<_>>();
        let rewritten = formulas[0].formula();
        let rename_atom = rewritten.argument(0).unwrap();
        assert_eq!(rename_atom.f_code(), bank.signature().eqn_code());
        let active_definition = formulas[1].formula();
        assert_eq!(active_definition.f_code(), bank.signature().equiv_code());
        assert_eq!(active_definition.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(active_definition.argument(1).as_ref(), Some(&expensive));
        assert_eq!(archive.cardinality(), 1);
    }

    #[test]
    fn wrapped_formula_cnf2_quotes_clause_wrappers_directly() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_cnf_clause_a");
        let b = typed_const(&mut bank, "wf_cnf_clause_b");
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn(&mut bank, &a, &b, true)]));
        let formula = tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_is_clause(true);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        let source = FormulaDerivationRef::new(wrapped.ident());
        let mut set = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = wrapped
            .cnf2_into(
                &mut bank,
                &mut set,
                &fresh_vars,
                100,
                false,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.clauses_generated, 1);
        assert_eq!(result.formula_derivation_ops, Vec::<i64>::new());
        let generated = set.iter().next().unwrap();
        assert_eq!(generated.query_tptp_type(), CP_TYPE_AXIOM);
        assert!(generated.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            generated.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_FOF_QUOTE),
                DerivationEntry::FormulaParent(source),
            ]
        );
    }

    #[test]
    fn wrapped_formula_cnf2_runs_formula_pipeline_and_extracts_clauses() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "wf_cnf_a");
        let b = typed_const(&mut bank, "wf_cnf_b");
        let c = typed_const(&mut bank, "wf_cnf_c");
        let d = typed_const(&mut bank, "wf_cnf_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_left = bool_binary_with_code(&mut bank, eqn_code, &b, &c);
        let right_right = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let right_conjunction =
            bool_binary_with_code(&mut bank, and_code, &right_left, &right_right);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &right_conjunction);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let source = FormulaDerivationRef::new(wrapped.ident());
        let mut set = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = wrapped
            .cnf2_into(
                &mut bank,
                &mut set,
                &fresh_vars,
                100,
                false,
                ProblemType::FirstOrder,
            )
            .unwrap();

        assert_eq!(result.clauses_generated, 2);
        assert!(result
            .formula_derivation_ops
            .contains(&DC_DIST_DISJUNCTIONS));
        assert_eq!(set.members(), 2);
        for clause in set.iter() {
            assert_eq!(clause.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
            assert_eq!(
                &clause.derivation().unwrap().as_slice()[..2],
                &[
                    DerivationEntry::Operation(DC_SPLIT_CONJUNCT),
                    DerivationEntry::FormulaParent(source),
                ]
            );
        }
    }

    fn count_formula_set_cnf_derivations(
        clauses: &ClauseSet,
        split_source: FormulaDerivationRef,
        quote_source: FormulaDerivationRef,
    ) -> (i32, i32) {
        let mut split_clauses = 0;
        let mut quoted_clauses = 0;
        for clause in clauses.iter() {
            let derivation = clause.derivation().unwrap().as_slice();
            let operation = match derivation.first() {
                Some(DerivationEntry::Operation(operation)) => *operation,
                _ => panic!("unexpected CNF clause derivation: {derivation:?}"),
            };
            let parent = match derivation.get(1) {
                Some(DerivationEntry::FormulaParent(parent)) => *parent,
                _ => panic!("unexpected CNF clause derivation: {derivation:?}"),
            };
            match operation {
                DC_SPLIT_CONJUNCT => {
                    split_clauses += 1;
                    assert_eq!(parent, split_source);
                    assert_eq!(clause.query_tptp_type(), CP_TYPE_NEG_CONJECTURE);
                }
                DC_FOF_QUOTE => {
                    quoted_clauses += 1;
                    assert_eq!(parent, quote_source);
                    assert_eq!(clause.query_tptp_type(), CP_TYPE_AXIOM);
                }
                _ => panic!("unexpected CNF clause derivation: {derivation:?}"),
            }
        }
        (split_clauses, quoted_clauses)
    }

    #[test]
    fn formula_set_cnf2_drains_inputs_and_archives_originals_then_cnf_copies() {
        let mut bank = test_bank();
        let atom_left = typed_const(&mut bank, "set_cnf_a");
        let atom_middle = typed_const(&mut bank, "set_cnf_b");
        let atom_right = typed_const(&mut bank, "set_cnf_c");
        let atom_tail = typed_const(&mut bank, "set_cnf_d");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left = bool_binary_with_code(&mut bank, eqn_code, &atom_left, &atom_middle);
        let right_left = bool_binary_with_code(&mut bank, eqn_code, &atom_middle, &atom_right);
        let right_right = bool_binary_with_code(&mut bank, eqn_code, &atom_right, &atom_tail);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let right_conjunction =
            bool_binary_with_code(&mut bank, and_code, &right_left, &right_right);
        let formula = bool_binary_with_code(&mut bank, or_code, &left, &right_conjunction);
        let original_formula = formula.clone();
        let mut first = WrappedFormula::wt_formula_alloc(formula);
        first.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        first.set_info(Some(ClauseInfo::new(Some("set_cnf_formula"), None, 1, 1)));
        let first_entry = first.entry_id();
        let first_source = FormulaDerivationRef::new(first.ident());

        let clause_left = typed_const(&mut bank, "set_cnf_e");
        let clause_right = typed_const(&mut bank, "set_cnf_f");
        let clause = Clause::alloc(EqnList::from_vec(vec![eqn(
            &mut bank,
            &clause_left,
            &clause_right,
            true,
        )]));
        let clause_formula =
            tformula_clause_encode(&mut bank, &clause, ProblemType::FirstOrder).unwrap();
        let original_clause_formula = clause_formula.clone();
        let mut second = WrappedFormula::wt_formula_alloc(clause_formula);
        second.set_is_clause(true);
        second.set_tptp_type(CP_TYPE_AXIOM);
        let second_entry = second.entry_id();
        let second_source = FormulaDerivationRef::new(second.ident());

        let mut formulas = FormulaSet::new();
        formulas.insert(first);
        formulas.insert(second);
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = formulas
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder),
            )
            .unwrap();

        assert!(formulas.is_empty());
        assert_eq!(archive.cardinality(), 4);
        assert_eq!(clauses.members(), 3);
        assert_eq!(result.clauses_generated, 3);
        assert_eq!(result.original_formulas_archived, 2);
        assert_eq!(result.cnf_formulas_archived, 2);
        assert_eq!(
            result.quoted_formula_sources,
            vec![first_source, second_source]
        );
        assert!(result
            .formula_derivation_ops
            .contains(&DC_DIST_DISJUNCTIONS));

        let archived = archive.iter().collect::<Vec<_>>();
        assert_eq!(archived[0].entry_id(), first_entry);
        assert_eq!(archived[0].formula(), &original_formula);
        assert_eq!(archived[1].ident(), first_source.ident());
        assert_ne!(archived[1].entry_id(), first_entry);
        assert_eq!(archived[1].info(), None);
        assert_ne!(archived[1].formula(), &original_formula);
        assert_eq!(archived[2].entry_id(), second_entry);
        assert_eq!(archived[2].formula(), &original_clause_formula);
        assert_eq!(archived[3].ident(), second_source.ident());
        assert!(archived[3].is_clause());

        assert_eq!(
            count_formula_set_cnf_derivations(&clauses, first_source, second_source),
            (2, 1)
        );
    }

    #[test]
    fn formula_set_cnf2_simplifies_before_archiving_inputs() {
        let mut bank = test_bank();
        let atom_left = typed_const(&mut bank, "set_cnf_simpl_left");
        let atom_right = typed_const(&mut bank, "set_cnf_simpl_right");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let atom = bool_binary_with_code(&mut bank, eqn_code, &atom_left, &atom_right);
        let truth = bank.true_term().clone();
        let neqn_code = bank.signature_mut().get_eqn_code(false);
        let false_formula = bool_binary_with_code(&mut bank, neqn_code, &truth, &truth);
        let or_code = bank.signature().or_code();
        let unsimplified = bool_binary_with_code(&mut bank, or_code, &false_formula, &atom);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(unsimplified.clone()));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.formulas_simplified, 1);
        assert!(result.formula_derivation_ops.contains(&DC_FOF_SIMPLIFY));
        assert_eq!(result.clauses_generated, 1);
        assert_eq!(clauses.members(), 1);
        let archived = archive.iter().collect::<Vec<_>>();
        assert_eq!(archived[0].formula(), &atom);
        assert_ne!(archived[0].formula(), &unsimplified);
    }

    #[test]
    fn formula_set_cnf2_unrolls_fool_before_archive_drain() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_cnf_fool_a");
        let b = typed_const(&mut bank, "set_cnf_fool_b");
        let c = typed_const(&mut bank, "set_cnf_fool_c");
        let d = typed_const(&mut bank, "set_cnf_fool_d");
        let target = typed_const(&mut bank, "set_cnf_fool_target");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &c, &d);
        let and_code = bank.signature().and_code();
        let bool_arg = bool_binary_with_code(&mut bank, and_code, &left_atom, &right_atom);
        let bool_type = bank.signature().type_bank().bool_type();
        let default_type = bank.signature().type_bank().default_type();
        let applied = typed_unary_with_types(
            &mut bank,
            "set_cnf_fool_fun",
            &bool_arg,
            &bool_type,
            &default_type,
        );
        let formula = bool_binary_with_code(&mut bank, eqn_code, &applied, &target);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, true, ProblemType::FirstOrder),
            )
            .unwrap();

        assert_eq!(result.formulas_fool_unrolled, 1);
        assert!(result.formula_derivation_ops.contains(&DC_FOOL_UNROLL));
        assert_eq!(archive.iter().next().unwrap().formula().f_code(), and_code);
        assert_eq!(clauses.members(), result.clauses_generated);
        assert!(result.clauses_generated > 0);
    }

    #[test]
    fn formula_set_cnf2_introduces_defs_before_archive_drain() {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "set_cnf_intro_first");
        let second = typed_const(&mut bank, "set_cnf_intro_second");
        let third = typed_const(&mut bank, "set_cnf_intro_third");
        let fourth = typed_const(&mut bank, "set_cnf_intro_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder).with_def_limit(1),
            )
            .unwrap();

        assert!(set.is_empty());
        assert_eq!(result.definitions_introduced, 1);
        assert_eq!(result.definition_formulas_archived, 1);
        assert_eq!(result.active_definition_formulas_inserted, 1);
        assert_eq!(result.formulas_rewritten_by_defs, 1);
        assert_eq!(result.definition_applications, 1);
        assert_eq!(result.original_formulas_archived, 2);
        assert_eq!(result.cnf_formulas_archived, 2);
        assert!(result.formula_derivation_ops.contains(&DC_INTRO_DEF));
        assert!(result.formula_derivation_ops.contains(&DC_SPLIT_EQUIV));
        assert!(result.formula_derivation_ops.contains(&DC_APPLY_DEF));
        assert_eq!(archive.cardinality(), 5);

        let archived = archive.iter().collect::<Vec<_>>();
        let neutral_definition = archived[0].formula();
        assert_eq!(neutral_definition.f_code(), bank.signature().equiv_code());
        let rename_atom = neutral_definition.argument(0).unwrap();
        let rewritten_original = archived[1].formula();
        assert_eq!(rewritten_original.f_code(), bank.signature().or_code());
        assert_eq!(rewritten_original.argument(0).as_ref(), Some(&rename_atom));
        assert_eq!(rewritten_original.argument(1).as_ref(), Some(&tail));
        assert_eq!(clauses.members(), result.clauses_generated);
        assert!(result.clauses_generated > 0);
    }

    #[test]
    fn formula_set_cnf2_collects_term_garbage_after_cnf_growth() {
        let mut bank = test_bank();
        let dropped = typed_const(&mut bank, "set_cnf_gc_dropped");
        let first = typed_const(&mut bank, "set_cnf_gc_first");
        let second = typed_const(&mut bank, "set_cnf_gc_second");
        let third = typed_const(&mut bank, "set_cnf_gc_third");
        let fourth = typed_const(&mut bank, "set_cnf_gc_fourth");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let left_atom = bool_binary_with_code(&mut bank, eqn_code, &first, &second);
        let right_atom = bool_binary_with_code(&mut bank, eqn_code, &third, &fourth);
        let equiv_code = bank.signature().equiv_code();
        let expensive = bool_binary_with_code(&mut bank, equiv_code, &left_atom, &right_atom);
        let tail = bool_binary_with_code(&mut bank, eqn_code, &first, &fourth);
        let or_code = bank.signature().or_code();
        let formula = bool_binary_with_code(&mut bank, or_code, &expensive, &tail);
        let mut set = FormulaSet::new();
        set.insert(WrappedFormula::wt_formula_alloc(formula));
        let mut archive = FormulaSet::new();
        let mut clauses = ClauseSet::new();
        let fresh_vars = VarBank::new(bank.signature().type_bank());

        let result = set
            .cnf2_into(
                &mut archive,
                &mut clauses,
                &mut bank,
                &fresh_vars,
                FormulaSetCnfOptions::new(100, false, ProblemType::FirstOrder).with_def_limit(1),
            )
            .unwrap();

        assert!(set.is_empty());
        assert!(result.term_garbage_collections >= 1);
        assert!(result.terms_recovered_by_gc >= 1);
        assert!(bank.find(&dropped).is_none());
        let archived_formula = archive.iter().next().unwrap().formula().clone();
        assert!(bank.find(&archived_formula).is_some());
    }

    #[test]
    fn formula_set_print_string_preserves_order_and_selected_format() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_print_a");
        let b = typed_const(&mut bank, "set_print_b");
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let first_formula = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let second_formula = bool_binary_with_code(&mut bank, eqn_code, &b, &a);
        let mut first = WrappedFormula::wt_formula_alloc(first_formula);
        first.set_tptp_type(CP_TYPE_AXIOM);
        first.set_prop(CP_INPUT_FORMULA);
        first.set_info(Some(ClauseInfo::new(Some("set_print_first"), None, 1, 1)));
        let mut second = WrappedFormula::wt_formula_alloc(second_formula);
        second.set_tptp_type(CP_TYPE_QUESTION);
        second.set_info(Some(ClauseInfo::new(Some("set_print_second"), None, 2, 1)));
        let mut set = FormulaSet::new();
        set.insert(first);
        set.insert(second);

        let set_tstp_output = set
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Tstp,
                true,
            )
            .unwrap();
        let set_legacy_output = set
            .print_string(
                &mut bank,
                true,
                ProblemType::FirstOrder,
                FormulaPrintFormat::Tptp,
                true,
            )
            .unwrap();

        let mut tstp_lines = set_tstp_output.lines();
        assert!(tstp_lines
            .next()
            .is_some_and(|line| line.starts_with("fof(set_print_first, axiom, ")));
        assert!(tstp_lines
            .next()
            .is_some_and(|line| line.starts_with("fof(set_print_second, question, ")));
        assert_eq!(tstp_lines.next(), None);
        assert!(set_legacy_output.starts_with("input_formula(set_print_first,axiom,"));
        assert!(set_legacy_output.contains("\ninput_formula(set_print_second,question,"));
        assert!(set_legacy_output.ends_with('\n'));
    }

    #[test]
    fn formula_set_pretty_print_tstp_declares_typed_symbols_first() {
        let mut bank = test_bank();
        let integer = bank.signature().type_bank().integer_type();
        let int_const = typed_const_with_type(&mut bank, "pretty_int", &integer);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &int_const, &int_const);
        let mut wrapped = WrappedFormula::wt_formula_alloc(formula);
        wrapped.set_tptp_type(CP_TYPE_AXIOM);
        wrapped.set_prop(CP_INPUT_FORMULA);
        wrapped.set_info(Some(ClauseInfo::new(Some("pretty_typed"), None, 1, 1)));
        let mut set = FormulaSet::new();
        set.insert(wrapped);

        let rendered = set
            .pretty_print_tstp_string(&mut bank, true, ProblemType::FirstOrder, true)
            .unwrap();

        let declaration = rendered
            .find("tff(decl_")
            .expect("typed pretty-print emits declarations");
        let formula = rendered
            .find("tff(pretty_typed, axiom, ")
            .expect("typed pretty-print emits formula after declarations");
        assert!(declaration < formula);
        assert!(rendered.contains("pretty_int: $int"));
    }

    #[test]
    fn formula_set_app_encode_preloads_declarations_and_skips_true_formula() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "set_app_a");
        let b = typed_const(&mut bank, "set_app_b");
        let f_a = typed_unary(&mut bank, "set_app_f", &a);
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let formula = bool_binary_with_code(&mut bank, eqn_code, &f_a, &b);
        let mut axiom = WrappedFormula::wt_formula_alloc(formula);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        axiom.set_prop(CP_INPUT_FORMULA);
        axiom.set_info(Some(ClauseInfo::new(Some("set_app_axiom"), None, 1, 1)));

        let conjecture_formula = bool_binary_with_code(&mut bank, eqn_code, &a, &b);
        let mut conjecture = WrappedFormula::wt_formula_alloc(conjecture_formula);
        conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        conjecture.set_info(Some(ClauseInfo::new(Some("set_app_neg_conj"), None, 2, 1)));

        let true_term = bank.true_term().clone();
        let true_formula = bool_binary_with_code(&mut bank, eqn_code, &true_term, &true_term);
        let mut skipped = WrappedFormula::wt_formula_alloc(true_formula);
        skipped.set_info(Some(ClauseInfo::new(Some("set_app_true"), None, 3, 1)));

        let mut set = FormulaSet::new();
        set.insert(axiom);
        set.insert(skipped);
        set.insert(conjecture);

        let rendered = set
            .app_encode_string(&mut bank, ProblemType::FirstOrder, true)
            .unwrap();

        let type_decl = rendered
            .find("tff(typedecl")
            .expect("type declarations are printed before formulas");
        let symbol_decl = rendered
            .find("tff(symboltypedecl")
            .expect("symbol declarations are printed before formulas");
        let axiom_line = rendered
            .find("tff(set_app_axiom, axiom, ")
            .expect("input axiom formula is rendered");
        assert!(type_decl < symbol_decl);
        assert!(symbol_decl < axiom_line);
        assert!(rendered.contains("tff(set_app_neg_conj, negated_conjecture, "));
        assert!(!rendered.contains("set_app_true"));
        assert_eq!(
            FormulaSet::new()
                .app_encode_string(&mut bank, ProblemType::FirstOrder, true)
                .unwrap(),
            ""
        );
    }

    #[test]
    fn formula_set_definition_statistics_matches_c_scan_shapes() {
        let mut bank = test_bank();
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let equiv_code = bank.signature().equiv_code();
        let or_code = bank.signature().or_code();

        let func_head = typed_const(&mut bank, "stats_function_head");
        let func_rhs = typed_const(&mut bank, "stats_function_rhs");
        let function_equation = bool_binary_with_code(&mut bank, eqn_code, &func_head, &func_rhs);
        let mut function_definition = WrappedFormula::wt_formula_alloc(function_equation);
        function_definition.set_prop(CP_IS_LAMBDA_DEF);

        let predicate_head = typed_predicate_const(&mut bank, "stats_predicate_head");
        let true_term = bank.true_term().clone();
        let predicate_equation =
            bool_binary_with_code(&mut bank, eqn_code, &predicate_head, &true_term);
        let true_formula = bank.true_term().clone();
        let predicate_equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &predicate_equation, &true_formula);
        let mut predicate_definition = WrappedFormula::wt_formula_alloc(predicate_equivalence);
        predicate_definition.set_prop(CP_IS_LAMBDA_DEF);

        let loose_head = typed_predicate_const(&mut bank, "stats_loose_predicate_head");
        let loose_rhs = typed_predicate_const(&mut bank, "stats_loose_predicate_rhs");
        let loose_equation = bool_binary_with_code(&mut bank, eqn_code, &loose_head, &loose_rhs);
        let true_formula = bank.true_term().clone();
        let loose_equivalence =
            bool_binary_with_code(&mut bank, equiv_code, &loose_equation, &true_formula);
        let mut loose_definition = WrappedFormula::wt_formula_alloc(loose_equivalence);
        loose_definition.set_prop(CP_IS_LAMBDA_DEF);
        assert_eq!(
            loose_definition.get_lambda_defined_symbol(bank.signature()),
            Some(loose_head.f_code())
        );

        let binder_type = bank.signature().type_bank().default_type();
        let lambda_body = typed_const(&mut bank, "stats_lambda_body");
        let lambda = close_with_db_var(&mut bank, &binder_type, &lambda_body).unwrap();
        assert_eq!(lambda.f_code(), SIG_DB_LAMBDA_CODE);
        let lambda_type = lambda.type_().expect("lambda is typed");
        let container_ret_type = bank.signature().type_bank().default_type();
        let lambda_container = typed_unary_with_types(
            &mut bank,
            "stats_lambda_container",
            &lambda,
            &lambda_type,
            &container_ret_type,
        );
        let lambda_rhs = typed_const(&mut bank, "stats_lambda_rhs");
        let lambda_literal =
            bool_binary_with_code(&mut bank, eqn_code, &lambda_container, &lambda_rhs);

        let app_head = typed_var(&bank, -101);
        let app_arg = typed_const(&mut bank, "stats_app_arg");
        let app_var = phony_app(&mut bank, &app_head, &app_arg);
        let app_literal = bool_binary_with_code(&mut bank, eqn_code, &app_var, &true_term);
        let normal_literal = bool_binary_with_code(&mut bank, eqn_code, &func_rhs, &func_rhs);
        let app_formula = bool_binary_with_code(&mut bank, or_code, &app_literal, &normal_literal);

        let mut orig = FormulaSet::new();
        orig.insert(function_definition);
        orig.insert(loose_definition);
        orig.insert(WrappedFormula::wt_formula_alloc(lambda_literal));
        let mut arch = FormulaSet::new();
        arch.insert(predicate_definition);
        arch.insert(WrappedFormula::wt_formula_alloc(app_formula));

        let stats = formula_set_definition_statistics(&orig, &arch, &bank);

        assert_eq!(stats.num_defs, 2);
        assert!((stats.percentage_form_defs - 0.5).abs() < f64::EPSILON);
        assert_eq!(stats.num_lams, 1);
        assert!(stats.has_app_var_lits);

        let empty = FormulaSet::new();
        assert_eq!(
            formula_set_definition_statistics(&empty, &empty, &bank),
            FormulaDefinitionStatistics::default()
        );
    }
}
