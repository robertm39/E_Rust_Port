use crate::basics::error::Diagnostic;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::fcvindexing::{
    fvi_param_init_anchors, fvi_param_init_specs, FvIndexInitTargetSets, FvIndexParams,
};
use crate::clauses::freqvectors::FvCollect;
use crate::terms::signature::{FunctionProperties, Signature};
use crate::terms::termbanks::TermBank;
use crate::terms::typebanks::TypeBank;

pub const WATCHLIST_INLINE_STRING: &str = "Use inline watchlist type";
pub const WATCHLIST_INLINE_QSTRING: &str = "'Use inline watchlist type'";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProofStateStatistics {
    pub status_reported: bool,
    pub answer_count: i64,
    pub processed_count: u64,
    pub proc_trivial_count: u64,
    pub proc_forward_subsumed_count: u64,
    pub proc_non_trivial_count: u64,
    pub other_redundant_count: u64,
    pub non_redundant_deleted: u64,
    pub backward_subsumed_count: u64,
    pub backward_rewritten_count: u64,
    pub backward_rewritten_lit_count: u64,
    pub rw_count: u64,
    pub generated_count: u64,
    pub aggressive_forward_subsumed_count: u64,
    pub generated_lit_count: u64,
    pub non_trivial_generated_count: u64,
    pub context_sr_count: u64,
    pub paramod_count: u64,
    pub factor_count: u64,
    pub neg_ext_count: u64,
    pub resolv_count: u64,
    pub disequ_deco_count: u64,
    pub satcheck_count: u64,
    pub satcheck_success: u64,
    pub satcheck_satisfiable: u64,
    pub satcheck_full_size: u64,
    pub satcheck_actual_size: u64,
    pub satcheck_core_size: u64,
    pub satcheck_preproc_time: f64,
    pub satcheck_encoding_time: f64,
    pub satcheck_solver_time: f64,
    pub satcheck_preproc_stime: f64,
    pub satcheck_encoding_stime: f64,
    pub satcheck_solver_stime: f64,
    pub filter_orphans_base: u64,
    pub forward_contract_base: u64,
    pub gc_count: u64,
    pub gc_used_count: u64,
}

#[derive(Clone, Debug)]
pub struct ProofState {
    terms: TermBank,
    original_symbols: usize,
    axioms: ClauseSet,
    ax_archive: ClauseSet,
    processed_pos_rules: ClauseSet,
    processed_pos_eqns: ClauseSet,
    processed_neg_units: ClauseSet,
    processed_non_units: ClauseSet,
    unprocessed: ClauseSet,
    tmp_store: ClauseSet,
    eval_store: ClauseSet,
    archive: ClauseSet,
    watchlist: Option<ClauseSet>,
    definition_store: ClauseSet,
    fvi_initialized: bool,
    fvi_cspec: Option<FvCollect>,
    def_store_cspec: Option<FvCollect>,
    state_is_complete: bool,
    has_interpreted_symbols: bool,
    statistics: ProofStateStatistics,
}

impl ProofState {
    /// Allocates the currently ported proof-state owner fields.
    ///
    /// Formula sets, global indices, demodulator trees, the temporary term bank,
    /// and SAT integration are added by later slices. The clause-set, FV-index,
    /// distinct-symbol, and statistic initialization mirrors C `ProofStateAlloc`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if internal signature or term-bank setup fails.
    pub fn new(free_symbol_props: FunctionProperties) -> Result<Self, Diagnostic> {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes()?;
        signature.remove_distinct_props(free_symbol_props);
        let terms = TermBank::new(signature)?;

        Ok(Self {
            terms,
            original_symbols: 0,
            axioms: ClauseSet::new(),
            ax_archive: ClauseSet::new(),
            processed_pos_rules: ClauseSet::new(),
            processed_pos_eqns: ClauseSet::new(),
            processed_neg_units: ClauseSet::new(),
            processed_non_units: ClauseSet::new(),
            unprocessed: ClauseSet::new(),
            tmp_store: ClauseSet::new(),
            eval_store: ClauseSet::new(),
            archive: ClauseSet::new(),
            watchlist: Some(ClauseSet::new()),
            definition_store: ClauseSet::new(),
            fvi_initialized: false,
            fvi_cspec: None,
            def_store_cspec: None,
            state_is_complete: true,
            has_interpreted_symbols: false,
            statistics: ProofStateStatistics::default(),
        })
    }

    #[must_use]
    pub const fn terms(&self) -> &TermBank {
        &self.terms
    }

    pub fn terms_mut(&mut self) -> &mut TermBank {
        &mut self.terms
    }

    #[must_use]
    pub const fn original_symbols(&self) -> usize {
        self.original_symbols
    }

    #[must_use]
    pub const fn axioms(&self) -> &ClauseSet {
        &self.axioms
    }

    pub fn axioms_mut(&mut self) -> &mut ClauseSet {
        &mut self.axioms
    }

    #[must_use]
    pub const fn ax_archive(&self) -> &ClauseSet {
        &self.ax_archive
    }

    pub fn ax_archive_mut(&mut self) -> &mut ClauseSet {
        &mut self.ax_archive
    }

    #[must_use]
    pub const fn processed_pos_rules(&self) -> &ClauseSet {
        &self.processed_pos_rules
    }

    pub fn processed_pos_rules_mut(&mut self) -> &mut ClauseSet {
        &mut self.processed_pos_rules
    }

    #[must_use]
    pub const fn processed_pos_eqns(&self) -> &ClauseSet {
        &self.processed_pos_eqns
    }

    pub fn processed_pos_eqns_mut(&mut self) -> &mut ClauseSet {
        &mut self.processed_pos_eqns
    }

    #[must_use]
    pub const fn processed_neg_units(&self) -> &ClauseSet {
        &self.processed_neg_units
    }

    pub fn processed_neg_units_mut(&mut self) -> &mut ClauseSet {
        &mut self.processed_neg_units
    }

    #[must_use]
    pub const fn processed_non_units(&self) -> &ClauseSet {
        &self.processed_non_units
    }

    pub fn processed_non_units_mut(&mut self) -> &mut ClauseSet {
        &mut self.processed_non_units
    }

    #[must_use]
    pub const fn unprocessed(&self) -> &ClauseSet {
        &self.unprocessed
    }

    pub fn unprocessed_mut(&mut self) -> &mut ClauseSet {
        &mut self.unprocessed
    }

    #[must_use]
    pub const fn tmp_store(&self) -> &ClauseSet {
        &self.tmp_store
    }

    pub fn tmp_store_mut(&mut self) -> &mut ClauseSet {
        &mut self.tmp_store
    }

    #[must_use]
    pub const fn eval_store(&self) -> &ClauseSet {
        &self.eval_store
    }

    pub fn eval_store_mut(&mut self) -> &mut ClauseSet {
        &mut self.eval_store
    }

    #[must_use]
    pub const fn archive(&self) -> &ClauseSet {
        &self.archive
    }

    pub fn archive_mut(&mut self) -> &mut ClauseSet {
        &mut self.archive
    }

    #[must_use]
    pub const fn watchlist(&self) -> Option<&ClauseSet> {
        self.watchlist.as_ref()
    }

    pub fn watchlist_mut(&mut self) -> Option<&mut ClauseSet> {
        self.watchlist.as_mut()
    }

    pub fn discard_watchlist(&mut self) -> Option<ClauseSet> {
        self.watchlist.take()
    }

    #[must_use]
    pub const fn definition_store(&self) -> &ClauseSet {
        &self.definition_store
    }

    pub fn definition_store_mut(&mut self) -> &mut ClauseSet {
        &mut self.definition_store
    }

    #[must_use]
    pub const fn fvi_initialized(&self) -> bool {
        self.fvi_initialized
    }

    #[must_use]
    pub const fn fvi_cspec(&self) -> Option<&FvCollect> {
        self.fvi_cspec.as_ref()
    }

    #[must_use]
    pub const fn def_store_cspec(&self) -> Option<&FvCollect> {
        self.def_store_cspec.as_ref()
    }

    #[must_use]
    pub const fn state_is_complete(&self) -> bool {
        self.state_is_complete
    }

    #[must_use]
    pub const fn has_interpreted_symbols(&self) -> bool {
        self.has_interpreted_symbols
    }

    #[must_use]
    pub const fn statistics(&self) -> &ProofStateStatistics {
        &self.statistics
    }

    pub fn statistics_mut(&mut self) -> &mut ProofStateStatistics {
        &mut self.statistics
    }

    #[must_use]
    pub fn processed_cardinality(&self) -> i64 {
        self.processed_pos_rules.members()
            + self.processed_pos_eqns.members()
            + self.processed_neg_units.members()
            + self.processed_non_units.members()
    }

    #[must_use]
    pub fn unprocessed_cardinality(&self) -> i64 {
        self.unprocessed.members()
    }

    #[must_use]
    pub fn cardinality(&self) -> i64 {
        self.processed_cardinality() + self.unprocessed_cardinality()
    }

    /// Counts clause axioms currently represented in Rust.
    ///
    /// C `ProofStateAxNo` also includes formula axioms; this returns the clause
    /// side until `FormulaSet` is part of the proof-state owner.
    #[must_use]
    pub fn axiom_count(&self) -> i64 {
        self.axioms.members()
    }

    #[must_use]
    pub fn is_untyped(&self) -> bool {
        self.processed_pos_rules.is_untyped()
            && self.processed_pos_eqns.is_untyped()
            && self.processed_neg_units.is_untyped()
            && self.processed_non_units.is_untyped()
            && self.unprocessed.is_untyped()
    }

    /// Clears the clause sets covered by C `ProofStateResetClauseSets`.
    ///
    /// The C helper does not clear `definition_store`, despite its comment
    /// saying all clause and formula sets are emptied. Rust preserves that until
    /// definition-store reset semantics are audited with callers.
    pub fn reset_clause_sets(&mut self) {
        self.axioms.clear();
        self.ax_archive.clear();
        self.processed_pos_rules.clear();
        self.processed_pos_eqns.clear();
        self.processed_neg_units.clear();
        self.processed_non_units.clear();
        self.unprocessed.clear();
        self.tmp_store.clear();
        self.eval_store.clear();
        self.archive.clear();
        if let Some(watchlist) = self.watchlist.as_mut() {
            watchlist.clear();
        }
    }

    /// Initializes and installs the FV-index anchors attached by C
    /// `fvi_param_init`.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic if the current signature cannot be represented in
    /// the feature-vector index layout.
    pub fn init_fvi_anchors(&mut self, params: &FvIndexParams) -> Result<(), Diagnostic> {
        let specs = fvi_param_init_specs(self.terms.signature(), params)?;
        let anchors =
            fvi_param_init_anchors(&self.axioms, &specs, params, self.watchlist.is_some());
        self.fvi_cspec = Some(specs.cspec().clone());
        self.def_store_cspec = Some(specs.def_store_cspec().clone());

        let Self {
            processed_non_units,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            watchlist,
            definition_store,
            ..
        } = self;
        anchors.install(FvIndexInitTargetSets {
            processed_non_units,
            processed_pos_rules,
            processed_pos_eqns,
            processed_neg_units,
            watchlist: watchlist.as_mut(),
            def_store: definition_store,
        });
        self.fvi_initialized = true;
        Ok(())
    }
}

pub fn proof_state_alloc(free_symbol_props: FunctionProperties) -> Result<ProofState, Diagnostic> {
    ProofState::new(free_symbol_props)
}

#[cfg(test)]
mod tests {
    use super::{proof_state_alloc, ProofState, ProofStateStatistics};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::fcvindexing::FvIndexParams;
    use crate::clauses::freqvectors::FvIndexType;
    use crate::clauses::proofstate::{WATCHLIST_INLINE_QSTRING, WATCHLIST_INLINE_STRING};
    use crate::terms::signature::{FP_DISTINCT_PROP, FP_IGNORE_PROPS};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    crate::terms::simpletypes::alloc_arrow_type(vec![type_.clone(), type_]),
                )
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bank.signature().type_bank().default_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause_from(literals: Vec<Eqn>, ident: i64) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_ident(ident);
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn simple_clause(state: &mut ProofState, stem: &str, ident: i64) -> Clause {
        let bank = state.terms_mut();
        let left = typed_const(bank, &format!("{stem}_left"));
        let right = typed_const(bank, &format!("{stem}_right"));
        clause_from(vec![literal(bank, &left, &right, true)], ident)
    }

    fn nontrivial_clause(state: &mut ProofState, stem: &str, ident: i64) -> Clause {
        let bank = state.terms_mut();
        let left = typed_const(bank, &format!("{stem}_left"));
        let right_const = typed_const(bank, &format!("{stem}_right"));
        let right = typed_unary(bank, &format!("{stem}_f"), &right_const);
        clause_from(vec![literal(bank, &left, &right, true)], ident)
    }

    #[test]
    fn proof_state_alloc_initializes_c_shape_clause_sets_and_flags() {
        let state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();

        assert_eq!(WATCHLIST_INLINE_STRING, "Use inline watchlist type");
        assert_eq!(WATCHLIST_INLINE_QSTRING, "'Use inline watchlist type'");
        assert_eq!(state.original_symbols(), 0);
        assert!(state.watchlist().is_some());
        assert_eq!(state.axioms().members(), 0);
        assert_eq!(state.ax_archive().members(), 0);
        assert_eq!(state.processed_pos_rules().members(), 0);
        assert_eq!(state.processed_pos_eqns().members(), 0);
        assert_eq!(state.processed_neg_units().members(), 0);
        assert_eq!(state.processed_non_units().members(), 0);
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.tmp_store().members(), 0);
        assert_eq!(state.eval_store().members(), 0);
        assert_eq!(state.archive().members(), 0);
        assert_eq!(state.definition_store().members(), 0);
        assert!(state.state_is_complete());
        assert!(!state.has_interpreted_symbols());
        assert!(!state.fvi_initialized());
        assert!(state.fvi_cspec().is_none());
        assert!(state.def_store_cspec().is_none());
        assert_eq!(state.statistics(), &ProofStateStatistics::default());
        assert!(state.terms().signature().distinct_code() > 0);
    }

    #[test]
    fn proof_state_alloc_applies_free_symbol_distinct_mask() {
        let default_state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        assert_eq!(
            default_state.terms().signature().distinct_props(),
            FP_DISTINCT_PROP
        );

        let free_state = proof_state_alloc(FP_DISTINCT_PROP).unwrap();
        assert_eq!(
            free_state.terms().signature().distinct_props(),
            FP_IGNORE_PROPS
        );
    }

    #[test]
    fn proof_state_cardinalities_follow_c_macros() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let rule = simple_clause(&mut state, "card_rule", 10);
        let equation = simple_clause(&mut state, "card_eqn", 11);
        let negative = simple_clause(&mut state, "card_neg", 12);
        let non_unit = simple_clause(&mut state, "card_nonunit", 13);
        let unprocessed = simple_clause(&mut state, "card_unproc", 14);
        let axiom = simple_clause(&mut state, "card_axiom", 15);

        state.processed_pos_rules_mut().insert(rule);
        state.processed_pos_eqns_mut().insert(equation);
        state.processed_neg_units_mut().insert(negative);
        state.processed_non_units_mut().insert(non_unit);
        state.unprocessed_mut().insert(unprocessed);
        state.axioms_mut().insert(axiom);

        assert_eq!(state.processed_cardinality(), 4);
        assert_eq!(state.unprocessed_cardinality(), 1);
        assert_eq!(state.cardinality(), 5);
        assert_eq!(state.axiom_count(), 1);
    }

    #[test]
    fn proof_state_is_untyped_checks_processed_and_unprocessed_only() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let processed = simple_clause(&mut state, "untyped_proc", 20);
        let axiom_only = simple_clause(&mut state, "untyped_axiom", 21);

        state.processed_non_units_mut().insert(processed);
        state.axioms_mut().insert(axiom_only);

        assert!(state.is_untyped());
    }

    #[test]
    fn proof_state_reset_clause_sets_preserves_definition_store_and_fvi_state() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let axiom = nontrivial_clause(&mut state, "reset_axiom", 30);
        let processed = nontrivial_clause(&mut state, "reset_proc", 31);
        let unprocessed = nontrivial_clause(&mut state, "reset_unproc", 32);
        let watch = nontrivial_clause(&mut state, "reset_watch", 33);
        let def = nontrivial_clause(&mut state, "reset_def", 34);
        let params = FvIndexParams::new(FvIndexType::AcFold, false, true, 9, 1);

        state.axioms_mut().insert(axiom);
        state.processed_non_units_mut().insert(processed);
        state.unprocessed_mut().insert(unprocessed);
        state.watchlist_mut().unwrap().insert(watch);
        state.definition_store_mut().insert(def);
        state.init_fvi_anchors(&params).unwrap();

        state.reset_clause_sets();

        assert!(state.fvi_initialized());
        assert_eq!(state.axioms().members(), 0);
        assert_eq!(state.processed_non_units().members(), 0);
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.watchlist().unwrap().members(), 0);
        assert_eq!(state.definition_store().members(), 1);
        assert_eq!(
            state
                .processed_non_units()
                .fv_anchor()
                .unwrap()
                .index()
                .clause_count(),
            0
        );
    }

    #[test]
    fn proof_state_init_fvi_anchors_installs_processed_watchlist_and_definition_store_anchors() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let axiom = nontrivial_clause(&mut state, "fvi_axiom", 40);
        let params = FvIndexParams::new(FvIndexType::AcFold, false, true, 9, 1);

        state.axioms_mut().insert(axiom);
        state.init_fvi_anchors(&params).unwrap();

        assert!(state.fvi_initialized());
        assert!(state.fvi_cspec().is_some());
        assert!(state.def_store_cspec().is_some());
        assert!(state.processed_non_units().fv_anchor().is_some());
        assert!(state.processed_pos_rules().fv_anchor().is_some());
        assert!(state.processed_pos_eqns().fv_anchor().is_some());
        assert!(state.processed_neg_units().fv_anchor().is_some());
        assert!(state.watchlist().unwrap().fv_anchor().is_some());
        assert!(state.definition_store().fv_anchor().is_some());
    }

    #[test]
    fn proof_state_init_fvi_anchors_omits_watchlist_after_discard() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let params = FvIndexParams::new(FvIndexType::AcFold, false, false, 9, 1);

        assert!(state.discard_watchlist().is_some());
        state.init_fvi_anchors(&params).unwrap();

        assert!(state.watchlist().is_none());
        assert!(state.processed_non_units().fv_anchor().is_some());
        assert!(state.definition_store().fv_anchor().is_some());
    }
}
