use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{
    CP_INITIAL, CP_IS_DEAD, CP_IS_IR_VICTIM, CP_IS_ORIENTED, CP_IS_PROCESSED, CP_LIMITED_RW,
    CP_NO_GENERATION, CP_SUBSUMES_WATCH,
};
use crate::clauses::clausefunc::{clause_remove_ac_resolved, clause_remove_superfluous_literals};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::condensation::condense;
use crate::clauses::context_sr::clause_contextual_simplify_reflect;
use crate::clauses::eqn_props::{EP_IS_PM_INTO_LIT, EP_IS_SELECTED};
use crate::clauses::fcvindexing::fv_index_pack_clause;
use crate::clauses::fcvindexing::FvIndexParams;
use crate::clauses::freqvectors::FvPackedClause;
use crate::clauses::global_indices::GlobalIndices;
use crate::clauses::neweval::PRIO_LARGEST_REASONABLE;
use crate::clauses::proofstate::ProofState;
use crate::clauses::rewrite::{clause_compute_li_normalform_plain, clause_local_rw};
use crate::clauses::subsumption::{
    clause_negative_simplify_reflect, clause_positive_simplify_reflect,
    clause_set_find_first_subsumed_clause_with_index, clause_set_find_subsumed_clauses_with_index,
    clause_set_subsumes_clause_with_index, clause_subsume_order_sort_lits,
    unit_clause_set_subsumes_clause,
};
use crate::clauses::tautologies::clause_is_tautology;
use crate::heuristics::axiomscan::clause_set_scan_ac;
use crate::heuristics::clausesetfeatures::SpecFeatureCell;
use crate::heuristics::hcb::{
    hcb_clause_evaluate, hcb_clause_set_reweight, AcHandling, HeuristicParmsCell, SplitClassType,
};
use crate::heuristics::hcbadmin::HcbAdmin;
use crate::heuristics::heuristic_lookup::get_heuristic_handle_with_context;
use crate::heuristics::litselection::{
    apply_ported_literal_selector_with_bank, UnsupportedLiteralSelection,
};
use crate::heuristics::to_autoselect::to_select_ordering;
use crate::heuristics::wfcbadmin::{WeightParseContext, WfcbAdmin};
use crate::inout::scanner::{Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::ho_csu::init_unif_limits;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::RewriteLevel;

pub const DEFAULT_WEIGHT_FUNCTIONS: &str = concat!(
    "\n",
    "weight21_ugg  = Clauseweight(PreferUnitGroundGoals,2,1,1)      \n",
    "rweight21_g   = Refinedweight(PreferGoals,    2,1,1.5,1.1,1) \n",
    "rweight11_g   = Refinedweight(PreferGoals,    1,1,1.5,1.1,1.1) \n",
    "rweight21_a   = Refinedweight(PreferNonGoals, 2,1,1.5,1.1,1.1) \n",
    "rweight21_ugg = Refinedweight(PreferUnitGroundGoals, 2,1,1.5,1.1,1.1) \n",
    "fifo_f        = FIFOWeight(ConstPrio)                      \n",
    "lifo_f        = LIFOWeight(ConstPrio)                      \n",
    "weight11_f    = Clauseweight(ConstPrio,1,1,1)              \n",
    "weight11_ugg  = Clauseweight(PreferUnitGroundGoals,1,1,1)  \n",
    "weight21_f    = Clauseweight(ConstPrio,2,1,1)              \n",
    "TSMRDefault   = TSMWeight(ConstPrio, 1, 1, 2, flat, E_KNOWLEDGE,",
    "100000,1.0,1.0,Flat,IndexIdentity,100000,-20,20,-2,-1,0,2)\n",
);

pub const DEFAULT_HEURISTICS: &str = concat!(
    "Weight     = (1*weight21_ugg)                       \n",
    "WeightC1   = (1*weight11_ugg)                       \n",
    "StandardWeight = (1*weight21_f)                     \n",
    "StandardPG = (5*weight21_f,1*fifo_f)                \n",
    "RWeight    = (1*rweight21_ugg)                      \n",
    "FIFO       = (1*fifo_f)                             \n",
    "LIFO       = (1*lifo_f)                             \n",
    "Default    = (3*rweight21_a, 1*rweight21_g)         \n",
    "Uniq       = (1*Uniqweight(ConstPrio))\n",
    "UseWatchlist = \n",
    "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),",
    " 10*Refinedweight(PreferNonGoals,2,1,2,2,2),",
    " 5*OrientLMaxWeight(PreferWatchlist,2,1,2,1,1),",
    " 1*FIFOWeight(PreferWatchlist))\n",
    "UseWatchlistPure=\n",
    "(1*Defaultweight(PreferWatchlist))\n",
    "UseWatchlistPG10=\n",
    "(10*Defaultweight(PreferWatchlist),\n",
    " 1*FIFOWeight(ConstPrio))\n",
    "UseWatchlistEvo=\n",
    "(1*ConjectureRelativeSymbolWeight(SimulateSOS,0.5, 100, 100,\n",
    "                                  100, 100, 1.5, 1.5, 1),\n",
    " 4*ConjectureRelativeSymbolWeight(PreferWatchlist,0.1, 100, \n",
    "                                  100, 100, 100, 1.5, 1.5, 1.5),\n",
    " 1*FIFOWeight(PreferProcessed),\n",
    " 1*ConjectureRelativeSymbolWeight(PreferWatchlist,0.5, 100, 100, \n",
    "                                  100, 100, 1.5, 1.5, 1),\n",
    " 4*Refinedweight(SimulateSOS,3,2,2,1.5,2))\n",
    "UseTSM1 = \n",
    "(10*Refinedweight(PreferGoals,1,2,2,2,0.5),",
    " 10*Refinedweight(PreferNonGoals,2,1,2,2,2),",
    " 5*TSMRDefault,",
    " 1*FIFOWeight(PreferWatchlist))\n",
    "UseTSM2 = \n",
    "(20*TSMRDefault,",
    " 5*OrientLMaxWeight(PreferWatchlist,2,1,2,1,1),",
    " 1*FIFOWeight(PreferWatchlist)).",
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SatSolverState {
    generation: u64,
    trace_generation_enabled: bool,
}

impl SatSolverState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            generation: 1,
            trace_generation_enabled: true,
        }
    }

    #[must_use]
    pub const fn generation(self) -> u64 {
        self.generation
    }

    #[must_use]
    pub const fn trace_generation_enabled(self) -> bool {
        self.trace_generation_enabled
    }

    pub fn reset(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.trace_generation_enabled = true;
    }
}

impl Default for SatSolverState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiteralSelectionOutcome {
    Inherited,
    SelectorApplied,
    SelectionSkipped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProofStateInitAxiomOutcome {
    pub initial_clauses: i64,
    pub sos_marked: i64,
    pub watchlist_matches: i64,
    pub watchlist_removed: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProofStateInitOutcome {
    pub watchlist_indexed: i64,
    pub initial_clauses: i64,
    pub sos_marked: i64,
    pub watchlist_matches: i64,
    pub watchlist_removed: i64,
    pub ac_handling_active: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProofStateWatchlistOutcome {
    pub subsumes_watch: bool,
    pub removed: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ForwardContractOptions {
    pub non_unit_subsumption: bool,
    pub context_sr: bool,
    pub condense_clause: bool,
    pub level: RewriteLevel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ForwardContractCounts {
    pub subsumed: u64,
    pub trivial: u64,
}

pub struct ProofControl {
    ocb: Option<OrderControlBlock>,
    active_hcb: Option<usize>,
    wfcbs: WfcbAdmin,
    hcbs: HcbAdmin,
    ac_handling_active: bool,
    heuristic_parms: HeuristicParmsCell,
    fvi_parms: FvIndexParams,
    problem_specs: SpecFeatureCell,
    solver: SatSolverState,
}

impl ProofControl {
    #[must_use]
    pub fn new() -> Self {
        Self {
            ocb: None,
            active_hcb: None,
            wfcbs: WfcbAdmin::new(),
            hcbs: HcbAdmin::new(),
            ac_handling_active: false,
            heuristic_parms: HeuristicParmsCell::default(),
            fvi_parms: FvIndexParams::default(),
            problem_specs: SpecFeatureCell::default(),
            solver: SatSolverState::new(),
        }
    }

    #[must_use]
    pub const fn ocb(&self) -> Option<&OrderControlBlock> {
        self.ocb.as_ref()
    }

    pub fn set_ocb(&mut self, ocb: OrderControlBlock) {
        self.ocb = Some(ocb);
    }

    #[must_use]
    pub fn take_ocb(&mut self) -> Option<OrderControlBlock> {
        self.ocb.take()
    }

    #[must_use]
    pub const fn active_hcb(&self) -> Option<usize> {
        self.active_hcb
    }

    pub const fn set_active_hcb(&mut self, active_hcb: Option<usize>) {
        self.active_hcb = active_hcb;
    }

    #[must_use]
    pub const fn wfcbs(&self) -> &WfcbAdmin {
        &self.wfcbs
    }

    pub const fn wfcbs_mut(&mut self) -> &mut WfcbAdmin {
        &mut self.wfcbs
    }

    #[must_use]
    pub const fn hcbs(&self) -> &HcbAdmin {
        &self.hcbs
    }

    pub const fn hcbs_mut(&mut self) -> &mut HcbAdmin {
        &mut self.hcbs
    }

    #[must_use]
    pub const fn ac_handling_active(&self) -> bool {
        self.ac_handling_active
    }

    pub const fn set_ac_handling_active(&mut self, active: bool) {
        self.ac_handling_active = active;
    }

    #[must_use]
    pub const fn heuristic_parms(&self) -> &HeuristicParmsCell {
        &self.heuristic_parms
    }

    pub const fn heuristic_parms_mut(&mut self) -> &mut HeuristicParmsCell {
        &mut self.heuristic_parms
    }

    pub fn set_heuristic_parms(&mut self, heuristic_parms: HeuristicParmsCell) {
        self.heuristic_parms = heuristic_parms;
    }

    #[must_use]
    pub const fn fvi_parms(&self) -> &FvIndexParams {
        &self.fvi_parms
    }

    pub fn set_fvi_parms(&mut self, fvi_parms: FvIndexParams) {
        self.fvi_parms = fvi_parms;
    }

    #[must_use]
    pub const fn problem_specs(&self) -> &SpecFeatureCell {
        &self.problem_specs
    }

    pub fn set_problem_specs(&mut self, problem_specs: SpecFeatureCell) {
        self.problem_specs = problem_specs;
    }

    #[must_use]
    pub const fn solver(&self) -> SatSolverState {
        self.solver
    }

    pub fn reset_sat_solver(&mut self) {
        self.solver.reset();
    }
}

impl Default for ProofControl {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn proof_control_alloc() -> ProofControl {
    ProofControl::new()
}

pub fn proof_control_reset_sat_solver(control: &mut ProofControl) {
    control.reset_sat_solver();
}

/// Initializes the currently ported proof-control state handled by C
/// `ProofControlInit`.
///
/// Proof-state-owned setup from C `ProofStateInit`, including FV-index anchor
/// creation and clause-set insertion, is kept outside this helper so
/// proof-control initialization remains separate from proof-state mutation.
///
/// # Errors
///
/// Returns diagnostics from ordering selection, built-in or user-supplied
/// weight/heuristic definition parsing, or active heuristic lookup.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible ProofControlInit bridge keeps the original inputs visible"
)]
pub fn proof_control_init(
    control: &mut ProofControl,
    bank: &mut TermBank,
    axioms: &mut ClauseSet,
    params: &mut HeuristicParmsCell,
    fvi_params: &FvIndexParams,
    wfcb_defs: &[String],
    hcb_defs: &mut Vec<String>,
    higher_order_problem: bool,
) -> Result<(), Diagnostic> {
    debug_assert!(control.ocb.is_none());
    debug_assert!(control.active_hcb.is_none());

    let ocb = to_select_ordering(bank, axioms, params, higher_order_problem)?;
    control.ocb = Some(ocb);
    proof_control_init_heuristics(control, axioms, params, fvi_params, wfcb_defs, hcb_defs)
}

/// Installs the heuristic definition state handled by C `ProofControlInit`.
///
/// This helper is available separately for tests and staged integration points
/// that already own an OCB.
///
/// # Errors
///
/// Returns a diagnostic when a built-in or user-supplied weight/heuristic
/// definition cannot be parsed, or when the requested active heuristic is
/// unknown.
pub fn proof_control_init_heuristics(
    control: &mut ProofControl,
    axioms: &ClauseSet,
    params: &mut HeuristicParmsCell,
    fvi_params: &FvIndexParams,
    wfcb_defs: &[String],
    hcb_defs: &mut Vec<String>,
) -> Result<(), Diagnostic> {
    debug_assert!(control.active_hcb.is_none());

    let context = WeightParseContext::new(axioms);
    install_default_weight_functions(control, context)?;
    for definition in wfcb_defs {
        install_option_weight_functions(control, definition, context)?;
    }

    install_default_heuristics(control, context)?;
    if let Some(heuristic_def) = params.heuristic_def.clone() {
        hcb_defs.push(heuristic_def);
    } else if let Some(heuristic_def) = hcb_defs.last() {
        params.heuristic_def = Some(heuristic_def.clone());
    }
    for definition in hcb_defs.iter() {
        install_option_heuristics(control, definition, context)?;
    }

    control.heuristic_parms = params.clone();
    control.active_hcb = Some(get_heuristic_handle_with_context(
        &params.heuristic_name,
        &mut control.hcbs,
        &mut control.wfcbs,
        context,
    )?);
    control.fvi_parms = fvi_params.clone();
    if control.heuristic_parms.split_clauses == SplitClassType::NONE {
        control.fvi_parms.set_symbol_slack(0);
    }
    *params = control.heuristic_parms.clone();
    init_unif_limits(&control.heuristic_parms);

    Ok(())
}

/// Initializes the currently ported proof-state indexing portion of C
/// `ProofStateInit`.
///
/// This covers the `fvi_param_init` call and watchlist local indexed rebuild.
/// Axiom reweighting, copying into `unprocessed`, watchlist checks, and global
/// index insertion remain with the later proof-process initialization slice.
///
/// # Errors
///
/// Returns a diagnostic if proof-control ordering is not initialized, or if
/// feature-vector anchor construction fails.
pub fn proof_state_init_indexing(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<i64, Diagnostic> {
    if control.ocb.is_none() {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ProofStateInit requires initialized proof-control ordering",
        ));
    }

    if !state.fvi_initialized() {
        state.init_fvi_anchors(control.fvi_parms())?;
    }
    let Some(ocb) = control.ocb.as_mut() else {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ProofStateInit requires initialized proof-control ordering",
        ));
    };
    Ok(state.init_watchlist(ocb))
}

/// Initializes the currently ported proof-state portions of C
/// `ProofStateInit`.
///
/// This covers the processed-set precondition, FV-index/watchlist prefix,
/// `Uniq` ordering of axioms, copying axioms into `unprocessed`, initial-clause
/// watchlist checks, active-HCB evaluation, `prefer_initial_clauses` priority
/// adjustment, SOS marking, and AC scanning. Proof-documentation/derivation
/// pushes and state-owned global-index storage remain pending.
///
/// # Errors
///
/// Returns diagnostics if proof-control ordering or active heuristic state is
/// missing, FV-index anchor construction fails, heuristic lookup fails, or an
/// axiom copy cannot be represented in the proof-state term bank.
pub fn proof_state_init(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<ProofStateInitOutcome, Diagnostic> {
    debug_assert!(state.processed_pos_rules().is_empty());
    debug_assert!(state.processed_pos_eqns().is_empty());
    debug_assert!(state.processed_neg_units().is_empty());
    debug_assert!(state.processed_non_units().is_empty());

    let watchlist_indexed = proof_state_init_indexing(state, control)?;
    let axiom_outcome = proof_state_init_axioms(state, control)?;
    let ac_handling_active = proof_state_init_ac_handling(state, control);
    Ok(ProofStateInitOutcome {
        watchlist_indexed,
        initial_clauses: axiom_outcome.initial_clauses,
        sos_marked: axiom_outcome.sos_marked,
        watchlist_matches: axiom_outcome.watchlist_matches,
        watchlist_removed: axiom_outcome.watchlist_removed,
        ac_handling_active,
    })
}

/// Runs C `ProofStateInit`, then initializes caller-owned global indices.
///
/// C stores these indices in `state->gindices`. The current Rust `ProofState`
/// owns its `TermBank` directly, while `GlobalIndices` borrows the signature, so
/// callers provide the index owner explicitly until proof-session ownership can
/// hold both without a self-reference.
///
/// # Errors
///
/// Returns diagnostics from [`proof_state_init`].
pub fn proof_state_init_with_global_indices<'sig>(
    state: &'sig mut ProofState,
    control: &mut ProofControl,
    indices: &mut GlobalIndices<'sig>,
    problem_type: ProblemType,
) -> Result<ProofStateInitOutcome, Diagnostic> {
    let outcome = proof_state_init(state, control)?;
    proof_state_init_global_indices(state, control, indices, problem_type);
    Ok(outcome)
}

/// Runs the axiom-queue portion of C `ProofStateInit` after indexing setup.
///
/// # Errors
///
/// Returns a diagnostic if the active HCB is missing, if `Uniq` lookup fails,
/// or if copying a source axiom into the state term bank fails.
pub fn proof_state_init_axioms(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<ProofStateInitAxiomOutcome, Diagnostic> {
    let active_hcb_handle = control.active_hcb.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ProofStateInit requires initialized proof-control heuristic",
        )
    })?;
    let context = WeightParseContext::new(state.axioms());
    let uniq_hcb_handle =
        get_heuristic_handle_with_context("Uniq", &mut control.hcbs, &mut control.wfcbs, context)?;

    {
        let ProofControl { hcbs, wfcbs, .. } = control;
        let uniq_hcb = hcbs
            .hcb(uniq_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("Uniq"))?;
        let (terms, axioms) = state.terms_and_axioms_mut();
        hcb_clause_set_reweight(uniq_hcb, wfcbs, terms, axioms);
    }

    let ordered_axioms = state.axioms().eval_order_cloned(0);
    let prefer_initial = control.heuristic_parms.prefer_initial_clauses;
    let static_watchlist = control.heuristic_parms.watchlist_is_static;
    let lambda_demod = control.heuristic_parms.lambda_demod;
    let use_tptp_sos = control.heuristic_parms.use_tptp_sos;
    let mut initial_clauses = 0;
    let mut watchlist_matches = 0;
    let mut watchlist_removed = 0;

    {
        let ProofControl { hcbs, wfcbs, .. } = control;
        let active_hcb = hcbs
            .hcb(active_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("active"))?;

        for source in ordered_axioms {
            let mut new = source.copy_to_bank(state.terms_mut())?;
            new.set_prop(CP_INITIAL);
            let watchlist_outcome =
                proof_state_check_watchlist(state, &mut new, static_watchlist, lambda_demod);
            if watchlist_outcome.subsumes_watch {
                watchlist_matches += 1;
            }
            watchlist_removed += watchlist_outcome.removed;

            hcb_clause_evaluate(active_hcb, wfcbs, state.terms(), &mut new);
            if prefer_initial {
                let Some(evaluations) = new.evaluations_mut() else {
                    return Err(Diagnostic::new(
                        ErrorCode::OTHER_ERROR,
                        "ProofStateInit HCB evaluation did not attach evaluations",
                    ));
                };
                evaluations.change_priority(-PRIO_LARGEST_REASONABLE);
            }
            state.unprocessed_mut().insert(new);
            initial_clauses += 1;
        }
    }

    let sos_marked = state.unprocessed_mut().mark_sos(use_tptp_sos);
    Ok(ProofStateInitAxiomOutcome {
        initial_clauses,
        sos_marked,
        watchlist_matches,
        watchlist_removed,
    })
}

/// Runs C `check_watchlist` against the proof-state watchlist.
///
/// The current Rust path updates the local watchlist FV index and archive.
/// Long-lived `wlindices` deletion and proof-documentation output are wired
/// with the later state-owned global-index/proof-output integration.
#[must_use]
pub fn proof_state_check_watchlist(
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    _lambda_demod: bool,
) -> ProofStateWatchlistOutcome {
    let (terms, watchlist, archive) = state.terms_watchlist_archive_mut();
    let Some(watchlist) = watchlist else {
        return ProofStateWatchlistOutcome::default();
    };

    clause.subsume_order_sort_literals(terms);
    clause.set_weight(clause.standard_weight());

    if static_watchlist {
        let subsumed = clause_set_find_first_subsumed_clause_with_index(
            watchlist,
            watchlist.fv_anchor(),
            clause,
            terms,
        );
        if subsumed.is_some() {
            clause.set_prop(CP_SUBSUMES_WATCH);
            return ProofStateWatchlistOutcome {
                subsumes_watch: true,
                removed: 0,
            };
        }
        return ProofStateWatchlistOutcome::default();
    }

    let removed = remove_watchlist_subsumed(watchlist, archive, clause, terms);
    if removed != 0 {
        clause.set_prop(CP_SUBSUMES_WATCH);
        return ProofStateWatchlistOutcome {
            subsumes_watch: true,
            removed,
        };
    }
    ProofStateWatchlistOutcome::default()
}

fn remove_watchlist_subsumed(
    watchlist: &mut ClauseSet,
    archive: &mut ClauseSet,
    subsumer: &Clause,
    terms: &TermBank,
) -> i64 {
    let mut stack = PStack::new();
    let expected_removed = clause_set_find_subsumed_clauses_with_index(
        watchlist,
        watchlist.fv_anchor(),
        subsumer,
        &mut stack,
        terms,
    );
    let ids = stack
        .as_slice()
        .iter()
        .map(|clause| clause.ident())
        .collect::<Vec<_>>();

    let mut removed = 0;
    for ident in ids {
        let Some(mut clause) = watchlist.extract_by_id(ident) else {
            continue;
        };
        clause.set_prop(CP_IS_DEAD);
        archive.insert(clause);
        removed += 1;
    }

    debug_assert_eq!(removed, expected_removed);
    removed
}

/// Moves all processed clauses back to `unprocessed`, matching C
/// `ProofStateResetProcessed`.
///
/// Each source clause is archived unchanged, while an evaluated flat copy is
/// queued in `unprocessed`.
///
/// # Errors
///
/// Returns a diagnostic if the active HCB is missing, copying into the
/// proof-state term bank fails, or active-HCB evaluation does not attach
/// evaluations before `prefer_initial_clauses` rewrites priorities.
pub fn proof_state_reset_processed(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<i64, Diagnostic> {
    let active_hcb_handle = control.active_hcb.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ProofStateResetProcessed requires initialized proof-control heuristic",
        )
    })?;
    let prefer_initial = control.heuristic_parms.prefer_initial_clauses;
    let mut reset = 0;

    {
        let ProofControl { hcbs, wfcbs, .. } = control;
        let active_hcb = hcbs
            .hcb(active_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("active"))?;
        let mut evaluate = |bank: &TermBank, clause: &mut Clause| {
            hcb_clause_evaluate(active_hcb, wfcbs, bank, clause);
        };

        reset += proof_state_reset_processed_set_by(
            state,
            prefer_initial,
            |state| state.processed_pos_rules_mut().extract_first(),
            &mut evaluate,
        )?;
        reset += proof_state_reset_processed_set_by(
            state,
            prefer_initial,
            |state| state.processed_pos_eqns_mut().extract_first(),
            &mut evaluate,
        )?;
        reset += proof_state_reset_processed_set_by(
            state,
            prefer_initial,
            |state| state.processed_neg_units_mut().extract_first(),
            &mut evaluate,
        )?;
        reset += proof_state_reset_processed_set_by(
            state,
            prefer_initial,
            |state| state.processed_non_units_mut().extract_first(),
            &mut evaluate,
        )?;
    }

    Ok(reset)
}

fn proof_state_reset_processed_set_by<E>(
    state: &mut ProofState,
    prefer_initial: bool,
    mut extract_first: impl FnMut(&mut ProofState) -> Option<Clause>,
    evaluate: &mut E,
) -> Result<i64, Diagnostic>
where
    E: FnMut(&TermBank, &mut Clause),
{
    let mut reset = 0;
    while let Some(handle) = extract_first(state) {
        proof_state_reset_processed_clause(state, handle, prefer_initial, evaluate)?;
        reset += 1;
    }
    Ok(reset)
}

fn proof_state_reset_processed_clause<E>(
    state: &mut ProofState,
    handle: Clause,
    prefer_initial: bool,
    evaluate: &mut E,
) -> Result<(), Diagnostic>
where
    E: FnMut(&TermBank, &mut Clause),
{
    let mut requeued = handle.flat_copy(state.terms_mut())?;
    state.archive_mut().insert(handle);
    evaluate(state.terms(), &mut requeued);
    requeued.del_prop(CP_IS_ORIENTED);

    if prefer_initial {
        let Some(evaluations) = requeued.evaluations_mut() else {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "ProofStateResetProcessed HCB evaluation did not attach evaluations",
            ));
        };
        evaluations.change_priority(-PRIO_LARGEST_REASONABLE);
    }

    state.unprocessed_mut().insert(requeued);
    Ok(())
}

/// Moves all processed clauses into `tmp_store`, matching C
/// `ProofStateMoveToTmpStore`.
///
/// This is the lightweight reset path: clauses are moved directly, not copied
/// or reevaluated.
#[must_use]
pub fn proof_state_move_to_tmp_store(state: &mut ProofState, _control: &ProofControl) -> i64 {
    let mut moved = 0;
    moved += proof_state_move_processed_set_to_tmp_by(state, |state| {
        state.processed_pos_rules_mut().extract_first()
    });
    moved += proof_state_move_processed_set_to_tmp_by(state, |state| {
        state.processed_pos_eqns_mut().extract_first()
    });
    moved += proof_state_move_processed_set_to_tmp_by(state, |state| {
        state.processed_neg_units_mut().extract_first()
    });
    moved += proof_state_move_processed_set_to_tmp_by(state, |state| {
        state.processed_non_units_mut().extract_first()
    });
    moved
}

fn proof_state_move_processed_set_to_tmp_by(
    state: &mut ProofState,
    mut extract_first: impl FnMut(&mut ProofState) -> Option<Clause>,
) -> i64 {
    let mut moved = 0;
    while let Some(mut handle) = extract_first(state) {
        handle.del_prop(CP_IS_ORIENTED);
        state.tmp_store_mut().insert(handle);
        moved += 1;
    }
    moved
}

/// Applies the currently ported modifying forward-inference prefix from C
/// `ForwardModifyClause`.
///
/// This covers the first-order/local mutation path: demodulation by the
/// processed positive-unit demodulator sets, superfluous literal removal,
/// optional AC-resolved literal cleanup, optional local rewriting, literal
/// orientation, optional condensation, triviality detection, and positive/negative
/// simplify-reflect against processed unit sets.
///
/// # Errors
///
/// Returns a diagnostic if proof-control ordering is missing, if a lower-level
/// term operation fails, or if the current problem is higher-order and reaches
/// higher-order-only normalization/pruning hooks that are not wired yet.
pub fn proof_state_forward_modify_clause(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
    _context_sr: bool,
    condense_clause: bool,
    level: RewriteLevel,
) -> Result<bool, Diagnostic> {
    if problem_type() == ProblemType::HigherOrder {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ForwardModifyClause higher-order normalization/pruning is not ported yet",
        ));
    }

    let prefer_general = control.heuristic_parms().prefer_general;
    let lambda_demod = control.heuristic_parms().lambda_demod;
    let local_rw = control.heuristic_parms().local_rw;
    let ac_handling_active = control.ac_handling_active();
    let ocb = control.ocb.as_mut().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ForwardModifyClause requires initialized proof-control ordering",
        )
    })?;

    let mut rw_steps = 0_i64;
    let trivial = {
        let (terms, processed_sets) = state.terms_and_processed_sets_mut();
        let demodulators = [processed_sets.pos_rules, processed_sets.pos_eqns];
        loop {
            let steps = clause_compute_li_normalform_plain(
                terms,
                ocb,
                clause,
                &demodulators,
                level,
                prefer_general,
                lambda_demod,
            )?;
            rw_steps += steps;

            let limited_rw = clause.query_prop(CP_LIMITED_RW);
            let _removed_lits = clause_remove_superfluous_literals(clause, terms);

            if ac_handling_active {
                let _ = clause_remove_ac_resolved(clause, terms);
            }

            if local_rw && clause_local_rw(ocb, terms, clause)? {
                debug_assert_ne!(problem_type(), ProblemType::HigherOrder);
            }

            clause.orient_literals(ocb, terms);

            if condense_clause && condense(clause, terms)? {
                clause.orient_literals(ocb, terms);
            }

            if clause.is_trivial(terms) {
                break true;
            }

            if clause.negative_literal_count() != 0 {
                let _ = clause_positive_simplify_reflect(processed_sets.pos_eqns, clause);
            }
            if clause.positive_literal_count() != 0 {
                let _ = clause_negative_simplify_reflect(processed_sets.neg_units, clause);
            }
            if clause.query_prop(CP_LIMITED_RW) == limited_rw {
                break false;
            }
        }
    };

    if rw_steps > 0 {
        state.statistics_mut().rw_count += u64::try_from(rw_steps).unwrap_or(u64::MAX);
    }
    Ok(trivial)
}

/// Tries C `ForwardSubsumption` against the processed clause sets.
///
/// The returned packed clause owns a Rust clone of the current candidate until
/// proof-state clause ownership can provide stable raw-pointer-shaped handles.
#[must_use]
pub fn proof_state_forward_subsumption(
    state: &ProofState,
    clause: &mut Clause,
    counts: &mut ForwardContractCounts,
    non_unit_subsumption: bool,
) -> Option<FvPackedClause> {
    clause.set_weight(clause.standard_weight());

    let mut subsumer_found = false;
    if clause.positive_literal_count() != 0 {
        subsumer_found =
            unit_clause_set_subsumes_clause(state.processed_pos_eqns(), clause).is_some();
    }
    if !subsumer_found && clause.negative_literal_count() != 0 {
        subsumer_found =
            unit_clause_set_subsumes_clause(state.processed_neg_units(), clause).is_some();
    }
    if !subsumer_found && clause.literal_number() > 1 && non_unit_subsumption {
        clause_subsume_order_sort_lits(clause, state.terms());
        subsumer_found = clause_set_subsumes_clause_with_index(
            state.processed_non_units(),
            state.processed_non_units().fv_anchor(),
            clause,
            state.terms(),
        )
        .is_some();
    }

    if subsumer_found {
        counts.subsumed += 1;
        return None;
    }

    Some(fv_index_pack_clause(
        clause.clone(),
        state.processed_non_units().fv_anchor(),
    ))
}

/// Applies the first-order/local body of C `forward_contract_keep`.
///
/// Higher-order-only hooks and proof-output side effects remain staged behind
/// explicit diagnostics or documentation until their owning modules are ported.
///
/// # Errors
///
/// Returns diagnostics from modifying contraction, tautology checking, literal
/// selection, or missing proof-control ordering.
pub fn proof_state_forward_contract_keep(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
    counts: &mut ForwardContractCounts,
    options: ForwardContractOptions,
) -> Result<Option<FvPackedClause>, Diagnostic> {
    if control.heuristic_parms().enable_given_forward_simpl {
        if proof_state_forward_modify_clause(
            state,
            control,
            clause,
            options.context_sr,
            options.condense_clause,
            options.level,
        )? {
            counts.trivial += 1;
            return Ok(None);
        }

        if clause.is_empty() {
            return Ok(Some(fv_index_pack_clause(clause.clone(), None)));
        }

        if control.ac_handling_active() && clause.is_ac_redundant(state.terms()) {
            let keep_orientable_unit = clause.is_unit()
                && control.heuristic_parms().ac_handling == AcHandling::KeepOrientable
                && clause
                    .literals()
                    .as_slice()
                    .first()
                    .is_some_and(crate::clauses::eqn::Eqn::is_oriented);
            let keep_unit = clause.is_unit()
                && matches!(control.heuristic_parms().ac_handling, AcHandling::KeepUnits);
            if keep_orientable_unit || keep_unit {
                clause.set_prop(CP_NO_GENERATION);
            } else {
                counts.trivial += 1;
                return Ok(None);
            }
        }

        if clause_is_tautology(state.terms_mut(), clause)? {
            counts.trivial += 1;
            return Ok(None);
        }

        debug_assert!(!clause.is_trivial(state.terms()));

        if problem_type() == ProblemType::HigherOrder {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "forward_contract_keep higher-order flex/naked-boolean hooks are not ported yet",
            ));
        }

        if proof_state_forward_subsumption(state, clause, counts, options.non_unit_subsumption)
            .is_none()
        {
            return Ok(None);
        }

        if options.context_sr && clause.literal_number() > 1 {
            let simplified = {
                let (terms, processed_sets) = state.terms_and_processed_sets_mut();
                clause_contextual_simplify_reflect(processed_sets.non_units, clause, terms)
            };
            state.statistics_mut().context_sr_count +=
                u64::try_from(simplified).unwrap_or(u64::MAX);
            clause_subsume_order_sort_lits(clause, state.terms());
        }
    } else if clause.is_empty() {
        return Ok(Some(fv_index_pack_clause(clause.clone(), None)));
    } else {
        clause.set_weight(clause.standard_weight());
    }

    clause.del_prop(CP_IS_ORIENTED);
    do_literal_selection_with_bank(control, state.terms(), clause)
        .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
    let ocb = control.ocb.as_mut().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "forward_contract_keep requires initialized proof-control ordering",
        )
    })?;
    clause.cond_mark_maximal_terms(ocb, state.terms());

    Ok(Some(fv_index_pack_clause(
        clause.clone(),
        state.processed_non_units().fv_anchor(),
    )))
}

/// Applies C `ForwardContractClause`, consuming redundant clauses.
///
/// # Errors
///
/// Returns diagnostics from [`proof_state_forward_contract_keep`].
pub fn proof_state_forward_contract_clause(
    state: &mut ProofState,
    control: &mut ProofControl,
    mut clause: Clause,
    options: ForwardContractOptions,
) -> Result<Option<FvPackedClause>, Diagnostic> {
    let mut counts = ForwardContractCounts::default();
    let result =
        proof_state_forward_contract_keep(state, control, &mut clause, &mut counts, options)?;
    state.statistics_mut().proc_forward_subsumed_count += counts.subsumed;
    state.statistics_mut().proc_trivial_count += counts.trivial;
    Ok(result)
}

/// Applies C `ForwardContractSet` over `set`.
///
/// The set is drained in order into a temporary owner and reconstructed on every
/// return path, preserving the C behavior where earlier clauses have already
/// been contracted/deleted and later clauses remain untouched when
/// `terminate_on_empty` returns an extracted empty clause.
///
/// # Errors
///
/// Returns diagnostics from [`proof_state_forward_contract_keep`].
pub fn proof_state_forward_contract_set(
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    non_unit_subsumption: bool,
    level: RewriteLevel,
    count_eliminated: &mut u64,
    terminate_on_empty: bool,
) -> Result<Option<Clause>, Diagnostic> {
    let mut rebuilt = ClauseSet::new();
    while let Some(mut clause) = set.extract_first() {
        let mut counts = ForwardContractCounts::default();
        let options = ForwardContractOptions {
            non_unit_subsumption,
            context_sr: false,
            condense_clause: false,
            level,
        };
        let contracted = match proof_state_forward_contract_keep(
            state,
            control,
            &mut clause,
            &mut counts,
            options,
        ) {
            Ok(contracted) => contracted,
            Err(err) => {
                rebuilt.insert(clause);
                restore_forward_contract_set(set, &mut rebuilt);
                return Err(err);
            }
        };
        *count_eliminated += counts.subsumed + counts.trivial;

        if contracted.is_some() {
            if terminate_on_empty && clause.is_empty() {
                restore_forward_contract_set(set, &mut rebuilt);
                return Ok(Some(clause));
            }
            rebuilt.insert(clause);
        }
    }

    set.insert_set(&mut rebuilt);
    Ok(None)
}

fn restore_forward_contract_set(set: &mut ClauseSet, rebuilt: &mut ClauseSet) {
    rebuilt.insert_set(set);
    set.insert_set(rebuilt);
}

/// Re-evaluates every clause in a set, matching C `ClauseSetReweight`.
///
/// # Errors
///
/// Returns a diagnostic if proof-control has no active HCB.
pub fn proof_control_clause_set_reweight(
    control: &mut ProofControl,
    terms: &TermBank,
    set: &mut ClauseSet,
) -> Result<(), Diagnostic> {
    let active_hcb_handle = control.active_hcb.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ClauseSetReweight requires initialized proof-control heuristic",
        )
    })?;
    let ProofControl { hcbs, wfcbs, .. } = control;
    let active_hcb = hcbs
        .hcb(active_hcb_handle)
        .ok_or_else(|| unknown_heuristic_handle("active"))?;
    hcb_clause_set_reweight(active_hcb, wfcbs, terms, set);
    Ok(())
}

/// Applies C `ForwardContractSetReweight`.
///
/// # Errors
///
/// Returns diagnostics from set contraction or HCB reweighting.
pub fn proof_state_forward_contract_set_reweight(
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    non_unit_subsumption: bool,
    level: RewriteLevel,
    count_eliminated: &mut u64,
) -> Result<Option<Clause>, Diagnostic> {
    let empty = proof_state_forward_contract_set(
        state,
        control,
        set,
        non_unit_subsumption,
        level,
        count_eliminated,
        true,
    )?;
    if empty.is_some() {
        return Ok(empty);
    }
    proof_control_clause_set_reweight(control, state.terms(), set)?;
    Ok(None)
}

/// Removes trivial clauses and re-evaluates the set, matching C's misspelled
/// `ClauseSetFilterReweigth`.
///
/// # Errors
///
/// Returns a diagnostic if HCB reweighting is not initialized.
pub fn proof_control_clause_set_filter_reweigth(
    control: &mut ProofControl,
    terms: &TermBank,
    set: &mut ClauseSet,
    count_eliminated: &mut u64,
) -> Result<(), Diagnostic> {
    *count_eliminated += i64_to_u64_saturating(set.filter_trivial(terms));
    proof_control_clause_set_reweight(control, terms, set)
}

/// Correctly spelled alias for [`proof_control_clause_set_filter_reweigth`].
///
/// # Errors
///
/// Returns a diagnostic if HCB reweighting is not initialized.
pub fn proof_control_clause_set_filter_reweight(
    control: &mut ProofControl,
    terms: &TermBank,
    set: &mut ClauseSet,
    count_eliminated: &mut u64,
) -> Result<(), Diagnostic> {
    proof_control_clause_set_filter_reweigth(control, terms, set, count_eliminated)
}

/// Applies C `ProofStateFilterUnprocessed` to the state-owned unprocessed set.
///
/// # Errors
///
/// Returns diagnostics from forward contraction.
pub fn proof_state_filter_unprocessed(
    state: &mut ProofState,
    control: &mut ProofControl,
    desc: &str,
) -> Result<Option<Clause>, Diagnostic> {
    let mut unprocessed = std::mem::take(state.unprocessed_mut());
    let result = proof_state_filter_unprocessed_set(state, control, &mut unprocessed, desc);
    *state.unprocessed_mut() = unprocessed;
    result
}

/// Applies C `ProofStateFilterUnprocessed` operations to a caller-provided set.
///
/// # Errors
///
/// Returns diagnostics from forward contraction.
pub fn proof_state_filter_unprocessed_set(
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    desc: &str,
) -> Result<Option<Clause>, Diagnostic> {
    for op in desc.bytes() {
        let empty = match op {
            b'u' => {
                let deleted = set.delete_non_units();
                state.statistics_mut().non_redundant_deleted += i64_to_u64_saturating(deleted);
                None
            }
            b'c' => {
                let deleted = set.delete_copies();
                state.statistics_mut().other_redundant_count += i64_to_u64_saturating(deleted);
                None
            }
            b'n' => proof_state_filter_contract_step(
                state,
                control,
                set,
                false,
                RewriteLevel::NoRewrite,
            )?,
            b'N' => proof_state_filter_contract_step(
                state,
                control,
                set,
                true,
                RewriteLevel::NoRewrite,
            )?,
            b'r' => proof_state_filter_contract_step(
                state,
                control,
                set,
                false,
                RewriteLevel::RuleRewrite,
            )?,
            b'R' => proof_state_filter_contract_step(
                state,
                control,
                set,
                true,
                RewriteLevel::RuleRewrite,
            )?,
            b'f' => proof_state_filter_contract_step(
                state,
                control,
                set,
                false,
                RewriteLevel::FullRewrite,
            )?,
            b'F' => proof_state_filter_contract_step(
                state,
                control,
                set,
                true,
                RewriteLevel::FullRewrite,
            )?,
            _ => None,
        };
        if empty.is_some() {
            return Ok(empty);
        }
    }
    Ok(None)
}

fn proof_state_filter_contract_step(
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    non_unit_subsumption: bool,
    level: RewriteLevel,
) -> Result<Option<Clause>, Diagnostic> {
    let mut count = 0;
    let empty = proof_state_forward_contract_set(
        state,
        control,
        set,
        non_unit_subsumption,
        level,
        &mut count,
        true,
    )?;
    state.statistics_mut().proc_trivial_count += count;
    Ok(empty)
}

fn i64_to_u64_saturating(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

/// Queues one generated non-trivial clause into `eval_store`, matching the
/// admission tail of C `insert_new_clauses`.
///
/// The C path reaches this point after generated-clause contraction and
/// replacement filters. It clears stale orientation state, runs literal
/// selection unless `select_on_proc_only` defers selection to processing time,
/// stamps `create_date` from `proc_non_trivial_count`, increments
/// `non_trivial_generated_count`, and queues the clause for later HCB
/// evaluation.
///
/// # Errors
///
/// Returns a diagnostic if the configured literal-selection strategy has not
/// been ported yet.
pub fn proof_state_queue_generated_clause_for_eval(
    state: &mut ProofState,
    control: &mut ProofControl,
    mut clause: Clause,
) -> Result<(), Diagnostic> {
    clause.del_prop(CP_IS_ORIENTED);
    if control.heuristic_parms().select_on_proc_only {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    } else {
        do_literal_selection_with_bank(control, state.terms(), &mut clause)
            .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
    }
    let create_date = i64::try_from(state.statistics().proc_non_trivial_count).unwrap_or(i64::MAX);
    clause.set_create_date(create_date);
    state.statistics_mut().non_trivial_generated_count += 1;
    state.eval_store_mut().insert(clause);
    Ok(())
}

/// Drains `tmp_store` through the currently ported local body of C
/// `insert_new_clauses`.
///
/// This covers generated counters, modifying forward contraction, watchlist
/// checks, empty-clause return, aggressive forward subsumption, eval-store
/// admission, HCB evaluation, and the final move to `unprocessed`. Destructive
/// equality resolution and controlled clause splitting are still separate
/// C-owned gates; when those options are enabled for pending generated clauses,
/// this helper reports an explicit diagnostic instead of silently skipping them.
///
/// # Errors
///
/// Returns a diagnostic from forward contraction, literal selection, HCB
/// evaluation, or from an enabled but not-yet-ported generated-clause gate.
pub fn proof_state_insert_new_clauses(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<Option<Clause>, Diagnostic> {
    if !state.tmp_store().is_empty() {
        ensure_insert_new_clauses_supported(control)?;
    }

    let generated_count = i64_to_u64_saturating(state.tmp_store().members());
    let generated_lit_count = i64_to_u64_saturating(state.tmp_store().literals());
    {
        let statistics = state.statistics_mut();
        statistics.generated_count += generated_count;
        statistics.generated_lit_count += generated_lit_count;
    }

    while let Some(mut clause) = state.tmp_store_mut().extract_first() {
        let context_sr = control.heuristic_parms().forward_context_sr_aggressive
            || (control.heuristic_parms().backward_context_sr
                && clause.query_prop(CP_IS_PROCESSED));
        let condense = control.heuristic_parms().condensing_aggressive;

        if clause.query_prop(CP_IS_IR_VICTIM) {
            debug_assert!(clause.query_prop(CP_LIMITED_RW));
            let _ = proof_state_forward_modify_clause(
                state,
                control,
                &mut clause,
                context_sr,
                condense,
                RewriteLevel::FullRewrite,
            )?;
            clause.del_prop(CP_IS_IR_VICTIM);
        }

        let level = control.heuristic_parms().forward_demod;
        let trivial = proof_state_forward_modify_clause(
            state,
            control,
            &mut clause,
            context_sr,
            condense,
            level,
        )?;
        if trivial || clause.is_trivial(state.terms()) {
            continue;
        }

        let static_watchlist = control.heuristic_parms().watchlist_is_static;
        let lambda_demod = control.heuristic_parms().lambda_demod;
        let _ = proof_state_check_watchlist(state, &mut clause, static_watchlist, lambda_demod);
        if clause.is_empty() {
            return Ok(Some(clause));
        }

        if control.heuristic_parms().forward_subsumption_aggressive {
            let mut counts = ForwardContractCounts::default();
            if proof_state_forward_subsumption(state, &mut clause, &mut counts, true).is_none() {
                state.statistics_mut().aggressive_forward_subsumed_count += counts.subsumed;
                continue;
            }
            state.statistics_mut().aggressive_forward_subsumed_count += counts.subsumed;
        }

        proof_state_queue_generated_clause_for_eval(state, control, clause)?;
    }

    proof_state_eval_clause_set(state, control)?;
    let _ = proof_state_move_eval_store_to_unprocessed(state);
    Ok(None)
}

fn ensure_insert_new_clauses_supported(control: &ProofControl) -> Result<(), Diagnostic> {
    let params = control.heuristic_parms();
    if params.er_aggressive && params.er_varlit_destructive {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "insert_new_clauses destructive equality-resolution gate is not ported yet",
        ));
    }
    if params.split_aggressive {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "insert_new_clauses aggressive controlled splitting gate is not ported yet",
        ));
    }
    Ok(())
}

/// Evaluates all clauses currently waiting in `eval_store`, matching C
/// `eval_clause_set`.
///
/// C mutates clauses in place after they have already been inserted into the
/// eval store. The Rust clause set owns evaluation indices, so this helper
/// extracts and reinserts each clause once after evaluation to keep those
/// indices synchronized while preserving set order.
///
/// # Errors
///
/// Returns a diagnostic if the eval store is non-empty and the active HCB is
/// missing.
pub fn proof_state_eval_clause_set(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<i64, Diagnostic> {
    let pending = state.eval_store().members();
    if pending == 0 {
        return Ok(0);
    }

    let active_hcb_handle = control.active_hcb.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "eval_clause_set requires initialized proof-control heuristic",
        )
    })?;

    {
        let ProofControl { hcbs, wfcbs, .. } = control;
        let active_hcb = hcbs
            .hcb(active_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("active"))?;

        for _ in 0..pending {
            let Some(mut clause) = state.eval_store_mut().extract_first() else {
                return Err(Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "eval_clause_set eval_store changed while evaluating clauses",
                ));
            };
            hcb_clause_evaluate(active_hcb, wfcbs, state.terms(), &mut clause);
            state.eval_store_mut().insert(clause);
        }
    }

    Ok(pending)
}

/// Moves evaluated clauses from `eval_store` to `unprocessed`.
///
/// This is the final queueing tail of C `insert_new_clauses` after
/// [`proof_state_eval_clause_set`] has attached evaluations. Proof-output
/// quoting is left for the later proof-documentation integration.
pub fn proof_state_move_eval_store_to_unprocessed(state: &mut ProofState) -> i64 {
    let mut moved = 0;
    while let Some(mut clause) = state.eval_store_mut().extract_first() {
        clause.del_prop(CP_IS_ORIENTED);
        state.unprocessed_mut().insert(clause);
        moved += 1;
    }
    moved
}

/// Runs the AC-axiom scan portion of C `ProofStateInit`.
///
/// The C code scans the initialized `unprocessed` set, not the source axiom
/// set, and skips mutation entirely when AC handling is disabled.
#[must_use]
pub fn proof_state_init_ac_handling(state: &mut ProofState, control: &mut ProofControl) -> bool {
    if control.heuristic_parms.ac_handling == AcHandling::None {
        return control.ac_handling_active;
    }

    let (terms, unprocessed) = state.terms_and_unprocessed_mut();
    let active = clause_set_scan_ac(terms.signature_mut(), unprocessed);
    control.ac_handling_active = active;
    active
}

/// Runs the global-index free/init tail of C `ProofStateInit`.
///
/// This mirrors `GlobalIndicesFreeIndices(&state->gindices)` followed by
/// `GlobalIndicesInit(...)`. The problem type is explicit instead of reading
/// C's process-global `problemType`.
pub fn proof_state_init_global_indices<'sig>(
    state: &'sig ProofState,
    control: &ProofControl,
    indices: &mut GlobalIndices<'sig>,
    problem_type: ProblemType,
) {
    let params = control.heuristic_parms();
    indices.init_for_problem(
        state.terms().signature(),
        params.rw_bw_index_type.as_str(),
        params.pm_from_index_type.as_str(),
        params.pm_into_index_type.as_str(),
        params.ext_rules_max_depth,
        problem_type,
    );
}

fn unknown_heuristic_handle(name: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        format!("ProofStateInit found missing {name} heuristic handle"),
    )
}

fn install_default_weight_functions(
    control: &mut ProofControl,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_internal_string(DEFAULT_WEIGHT_FUNCTIONS, true)?;
    control
        .wfcbs
        .weight_fun_def_list_parse_with_context(&mut scanner, context)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_option_weight_functions(
    control: &mut ProofControl,
    definition: &str,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_option_string(definition, true)?;
    control
        .wfcbs
        .weight_fun_def_list_parse_with_context(&mut scanner, context)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_default_heuristics(
    control: &mut ProofControl,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_internal_string(DEFAULT_HEURISTICS, true)?;
    control.hcbs.heuristic_def_list_parse_with_context(
        &mut scanner,
        &mut control.wfcbs,
        context,
    )?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_option_heuristics(
    control: &mut ProofControl,
    definition: &str,
    context: WeightParseContext<'_>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_option_string(definition, true)?;
    control.hcbs.heuristic_def_list_parse_with_context(
        &mut scanner,
        &mut control.wfcbs,
        context,
    )?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

#[must_use]
pub fn select_inherited_literal(clause: &mut Clause) -> bool {
    if clause.negative_literal_count() == 0 {
        return false;
    }
    let found = clause
        .literals()
        .as_slice()
        .iter()
        .any(|literal| literal.is_negative() && literal.query_prop(EP_IS_PM_INTO_LIT));
    if !found {
        return false;
    }
    for literal in clause.literals_mut().as_mut_slice() {
        if literal.query_prop(EP_IS_PM_INTO_LIT) {
            literal.set_prop(EP_IS_SELECTED);
        }
    }
    true
}

/// Runs the C `DoLiteralSelection` wrapper using a caller-supplied selector
/// body.
pub fn do_literal_selection_with_selector<S>(
    control: &mut ProofControl,
    clause: &mut Clause,
    mut selector: S,
) -> LiteralSelectionOutcome
where
    S: FnMut(Option<&mut OrderControlBlock>, &mut Clause),
{
    clear_literal_selection_state(clause);
    let parms = control.heuristic_parms();
    if should_try_inherited_selection(parms, clause) && select_inherited_literal(clause) {
        return LiteralSelectionOutcome::Inherited;
    }
    if literal_selection_conditions_hold(parms, clause) {
        debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
        selector(control.ocb.as_mut(), clause);
        LiteralSelectionOutcome::SelectorApplied
    } else {
        crate::heuristics::litselection::select_no_literals(control.ocb.as_mut(), clause);
        LiteralSelectionOutcome::SelectionSkipped
    }
}

/// Runs the C `DoLiteralSelection` wrapper using the literal-selection bodies
/// that have already been ported.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` if the configured selector body has
/// not been ported yet and the wrapper reaches the selector call.
pub fn do_literal_selection(
    control: &mut ProofControl,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, UnsupportedLiteralSelection> {
    do_literal_selection_impl(control, None, clause)
}

/// Runs the C `DoLiteralSelection` wrapper using ported selector bodies,
/// including selector bodies whose Rust implementation needs the term bank for
/// maximality marking.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` if the configured selector body has
/// not been ported yet and the wrapper reaches the selector call.
pub fn do_literal_selection_with_bank(
    control: &mut ProofControl,
    bank: &TermBank,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, UnsupportedLiteralSelection> {
    do_literal_selection_impl(control, Some(bank), clause)
}

fn do_literal_selection_impl(
    control: &mut ProofControl,
    bank: Option<&TermBank>,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, UnsupportedLiteralSelection> {
    clear_literal_selection_state(clause);
    let parms = control.heuristic_parms();
    if should_try_inherited_selection(parms, clause) && select_inherited_literal(clause) {
        return Ok(LiteralSelectionOutcome::Inherited);
    }
    if literal_selection_conditions_hold(parms, clause) {
        debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
        apply_ported_literal_selector_with_bank(
            control.heuristic_parms.selection_strategy.as_str(),
            control.ocb.as_mut(),
            bank,
            clause,
        )?;
        Ok(LiteralSelectionOutcome::SelectorApplied)
    } else {
        crate::heuristics::litselection::select_no_literals(control.ocb.as_mut(), clause);
        Ok(LiteralSelectionOutcome::SelectionSkipped)
    }
}

fn clear_literal_selection_state(clause: &mut Clause) {
    clause.literals_mut().del_prop(EP_IS_SELECTED);
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    clause.del_prop(CP_IS_ORIENTED);
    debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
}

fn should_try_inherited_selection(parms: &HeuristicParmsCell, clause: &Clause) -> bool {
    parms.inherit_paramod_lit
        || (parms.inherit_goal_pm_lit && clause.is_goal())
        || (parms.inherit_conj_pm_lit && clause.is_conjecture())
}

fn literal_selection_conditions_hold(parms: &HeuristicParmsCell, clause: &Clause) -> bool {
    clause.negative_literal_count() != 0
        && count_in_range(
            clause.positive_literal_count(),
            parms.pos_lit_sel_min,
            parms.pos_lit_sel_max,
        )
        && count_in_range(
            clause.negative_literal_count(),
            parms.neg_lit_sel_min,
            parms.neg_lit_sel_max,
        )
        && count_in_range(
            clause.literal_number(),
            parms.all_lit_sel_min,
            parms.all_lit_sel_max,
        )
        && (parms.weight_sel_min == 0 || parms.weight_sel_min <= clause.standard_weight())
}

fn count_in_range(count: usize, min: i64, max: i64) -> bool {
    let count = i64::try_from(count).unwrap_or(i64::MAX);
    count >= min && count <= max
}

#[cfg(test)]
mod tests {
    use super::{
        do_literal_selection, do_literal_selection_with_bank, do_literal_selection_with_selector,
        proof_control_alloc, proof_control_clause_set_filter_reweigth, proof_control_init,
        proof_control_init_heuristics, proof_control_reset_sat_solver, proof_state_eval_clause_set,
        proof_state_filter_unprocessed, proof_state_forward_contract_clause,
        proof_state_forward_contract_set, proof_state_forward_contract_set_reweight,
        proof_state_forward_modify_clause, proof_state_forward_subsumption, proof_state_init,
        proof_state_init_ac_handling, proof_state_init_global_indices, proof_state_init_indexing,
        proof_state_init_with_global_indices, proof_state_insert_new_clauses,
        proof_state_move_eval_store_to_unprocessed, proof_state_move_to_tmp_store,
        proof_state_queue_generated_clause_for_eval, proof_state_reset_processed,
        select_inherited_literal, ForwardContractCounts, ForwardContractOptions,
        LiteralSelectionOutcome, DEFAULT_HEURISTICS, DEFAULT_WEIGHT_FUNCTIONS,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::simple_stuff::ProblemType;
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{
        CP_INITIAL, CP_IS_DEAD, CP_IS_ORIENTED, CP_IS_PROCESSED, CP_IS_SOS, CP_LIMITED_RW,
        CP_SUBSUMES_WATCH, CP_TYPE_CONJECTURE,
    };
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{
        EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_PM_INTO_LIT, EP_IS_SELECTED, EP_MAX_IS_UP_TO_DATE,
    };
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::fcvindexing::FvIndexParams;
    use crate::clauses::freqvectors::{FvIndexType, FVINDEX_MAX_FEATURES_DEFAULT};
    use crate::clauses::global_indices::global_indices_null;
    use crate::clauses::neweval::{evals_alloc, PRIO_LARGEST_REASONABLE, PRIO_NORMAL};
    use crate::clauses::proofstate::{proof_state_alloc, ProofState};
    use crate::heuristics::hcb::{
        AcHandling, HeuristicParmsCell, SplitClassType, HCB_DEFAULT_HEURISTIC,
    };
    use crate::heuristics::litselection::{SELECT_NEGATIVE_LITERALS, SELECT_UNLESS_POS_MAX};
    use crate::heuristics::to_params::TermOrdering;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::{Signature, FP_COMMUTATIVE, FP_IGNORE_PROPS};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, RewriteLevel, Term};
    use crate::terms::typebanks::TypeBank;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

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

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn typed_binary_code(bank: &mut TermBank, name: &str) -> i64 {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(
                    f_code,
                    alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]),
                )
                .unwrap();
        }
        f_code
    }

    fn typed_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn commutativity_axiom(bank: &mut TermBank, name: &str, ident: i64) -> (Clause, i64) {
        let f_code = typed_binary_code(bank, name);
        let x = typed_var(bank, -2);
        let y = typed_var(bank, -4);
        let left = typed_binary_with_code(bank, f_code, &x, &y);
        let right = typed_binary_with_code(bank, f_code, &y, &x);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(bank, &left, &right, true)]));
        clause.set_ident(ident);
        (clause, f_code)
    }

    fn watchlist_subsumption_pair(
        bank: &mut TermBank,
        stem: &str,
        general_id: i64,
        watched_id: i64,
    ) -> (Clause, Clause) {
        let variable = typed_var(bank, -10);
        let witness = typed_const(bank, &format!("{stem}_witness"));
        let first = typed_const(bank, &format!("{stem}_first"));
        let second = typed_const(bank, &format!("{stem}_second"));
        let mut general = Clause::alloc(EqnList::from_vec(vec![
            literal(bank, &variable, &first, true),
            literal(bank, &variable, &second, true),
        ]));
        let mut watched = Clause::alloc(EqnList::from_vec(vec![
            literal(bank, &witness, &first, true),
            literal(bank, &witness, &second, true),
        ]));
        general.set_ident(general_id);
        general.set_weight(general.standard_weight());
        watched.set_ident(watched_id);
        watched.set_weight(watched.standard_weight());
        (general, watched)
    }

    fn mixed_clause() -> Clause {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "pc_a");
        let second = typed_const(&mut bank, "pc_b");
        Clause::alloc(EqnList::from_vec(vec![
            literal(&mut bank, &first, &second, true),
            literal(&mut bank, &second, &first, false),
        ]))
    }

    fn positive_clause() -> Clause {
        let mut bank = test_bank();
        let first = typed_const(&mut bank, "pc_pos_a");
        let second = typed_const(&mut bank, "pc_pos_b");
        Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &first, &second, true,
        )]))
    }

    fn negative_clause(bank: &mut TermBank) -> Clause {
        let first = typed_const(bank, "pc_neg_a");
        let second = typed_const(bank, "pc_neg_b");
        Clause::alloc(EqnList::from_vec(vec![literal(
            bank, &first, &second, false,
        )]))
    }

    fn unit_clause_with_id(bank: &mut TermBank, stem: &str, ident: i64) -> Clause {
        let left = typed_const(bank, &format!("{stem}_left"));
        let right = typed_const(bank, &format!("{stem}_right"));
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(bank, &left, &right, true)]));
        clause.set_ident(ident);
        clause
    }

    fn processed_unit_clause(bank: &mut TermBank, stem: &str, ident: i64) -> Clause {
        let mut clause = unit_clause_with_id(bank, stem, ident);
        clause.set_prop(CP_IS_PROCESSED | CP_IS_ORIENTED);
        clause
    }

    fn init_fifo_hcb(control: &mut super::ProofControl, state: &ProofState, name: &str) {
        let mut params = HeuristicParmsCell {
            heuristic_name: name.to_owned(),
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec![format!("{name}=(1*FIFOWeight(ConstPrio))")];
        proof_control_init_heuristics(
            control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    #[test]
    fn proof_control_alloc_initializes_c_shape() {
        let control = proof_control_alloc();

        assert!(control.ocb().is_none());
        assert!(control.active_hcb().is_none());
        assert!(control.wfcbs().is_empty());
        assert!(control.hcbs().is_empty());
        assert!(!control.ac_handling_active());
        assert_eq!(control.heuristic_parms(), &HeuristicParmsCell::default());
        assert_eq!(
            control.heuristic_parms().heuristic_name,
            HCB_DEFAULT_HEURISTIC
        );
        assert_eq!(control.fvi_parms().cspec().features(), FvIndexType::AcFold);
        assert!(!control.fvi_parms().use_perm_vectors());
        assert_eq!(
            control.fvi_parms().max_symbols(),
            FVINDEX_MAX_FEATURES_DEFAULT
        );
        assert_eq!(control.problem_specs().clauses, 0);
        assert_eq!(control.solver().generation(), 1);
        assert!(control.solver().trace_generation_enabled());
    }

    #[test]
    fn proof_control_reset_sat_solver_reinitializes_trace_state() {
        let mut control = proof_control_alloc();

        proof_control_reset_sat_solver(&mut control);

        assert_eq!(control.solver().generation(), 2);
        assert!(control.solver().trace_generation_enabled());
    }

    #[test]
    fn default_definition_strings_match_c_surface() {
        assert!(DEFAULT_WEIGHT_FUNCTIONS.starts_with('\n'));
        assert!(DEFAULT_WEIGHT_FUNCTIONS.contains("weight21_ugg  = Clauseweight"));
        assert!(DEFAULT_WEIGHT_FUNCTIONS.contains("TSMRDefault   = TSMWeight"));
        assert!(DEFAULT_HEURISTICS.contains("Default    = (3*rweight21_a, 1*rweight21_g)"));
        assert!(DEFAULT_HEURISTICS.contains("UseWatchlist = \n"));
        assert!(DEFAULT_HEURISTICS.ends_with(" 1*FIFOWeight(PreferWatchlist))."));
    }

    #[test]
    fn proof_control_init_installs_default_definitions_and_active_hcb() {
        let mut control = proof_control_alloc();
        let axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell::default();
        let fvi_params = FvIndexParams::new(FvIndexType::AcFold, true, true, 19, 7);
        let mut hcb_defs = Vec::new();

        proof_control_init_heuristics(
            &mut control,
            &axioms,
            &mut params,
            &fvi_params,
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(control.wfcbs().find_wfcb_handle("rweight21_a").is_some());
        assert!(control.wfcbs().find_wfcb_handle("TSMRDefault").is_some());
        let default_hcb = control
            .hcbs()
            .find_hcb_handle(HCB_DEFAULT_HEURISTIC)
            .unwrap_or_else(|| panic!("Default HCB should be installed"));
        assert_eq!(control.active_hcb(), Some(default_hcb));
        assert_eq!(control.fvi_parms().symbol_slack(), 0);
        assert_eq!(control.fvi_parms().max_symbols(), 19);
        assert!(control.fvi_parms().use_perm_vectors());
        assert!(control.fvi_parms().eliminate_uninformative());
        assert!(params.heuristic_def.is_none());
        assert!(hcb_defs.is_empty());
    }

    #[test]
    fn proof_control_init_preserves_symbol_slack_when_splitting_is_enabled() {
        let mut control = proof_control_alloc();
        let axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell {
            split_clauses: SplitClassType::ALL,
            ..HeuristicParmsCell::default()
        };
        let fvi_params = FvIndexParams::new(FvIndexType::AcFold, false, false, 23, 11);
        let mut hcb_defs = Vec::new();

        proof_control_init_heuristics(
            &mut control,
            &axioms,
            &mut params,
            &fvi_params,
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(control.fvi_parms().symbol_slack(), 11);
    }

    #[test]
    fn proof_state_init_indexing_installs_fvi_and_rebuilds_watchlist() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let watch = {
            let terms = state.terms_mut();
            negative_clause(terms)
        };
        state.watchlist_mut().unwrap().insert(watch);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.set_fvi_parms(FvIndexParams::new(FvIndexType::AcFold, false, true, 11, 2));

        let indexed = proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(indexed, 1);
        assert!(state.fvi_initialized());
        assert!(state.processed_non_units().fv_anchor().is_some());
        assert!(state.definition_store().fv_anchor().is_some());
        let watchlist = state.watchlist().unwrap();
        assert!(watchlist.fv_anchor().is_some());
        assert_eq!(watchlist.members(), 1);
    }

    #[test]
    fn proof_state_init_indexing_requires_initialized_ocb_before_mutation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();

        let error = proof_state_init_indexing(&mut state, &mut control).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(!state.fvi_initialized());
    }

    #[test]
    fn proof_state_init_copies_evaluated_axioms_to_unprocessed() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom_id, conjecture_id) = {
            let terms = state.terms_mut();
            let left = typed_const(terms, "pc_init_ax_a");
            let right = typed_const(terms, "pc_init_ax_b");
            let mut axiom =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &right, true)]));
            axiom.set_ident(4_001);

            let conj_left = typed_const(terms, "pc_init_conj_a");
            let conj_right = typed_const(terms, "pc_init_conj_b");
            let mut conjecture = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &conj_left,
                &conj_right,
                false,
            )]));
            conjecture.set_ident(4_002);
            conjecture.set_tptp_type(CP_TYPE_CONJECTURE);

            let axiom_id = axiom.ident();
            let conjecture_id = conjecture.ident();
            state.axioms_mut().insert(axiom);
            state.axioms_mut().insert(conjecture);
            (axiom_id, conjecture_id)
        };

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell {
            heuristic_name: "InitTest".to_owned(),
            prefer_initial_clauses: true,
            use_tptp_sos: true,
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec!["InitTest=(1*FIFOWeight(ConstPrio))".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let outcome = proof_state_init(&mut state, &mut control).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome.watchlist_indexed, 0);
        assert_eq!(outcome.initial_clauses, 2);
        assert_eq!(outcome.sos_marked, 1);
        assert_eq!(outcome.watchlist_matches, 0);
        assert_eq!(outcome.watchlist_removed, 0);
        assert!(!outcome.ac_handling_active);
        assert_eq!(state.axioms().members(), 2);
        assert_eq!(state.unprocessed().members(), 2);
        assert!(state.fvi_initialized());
        assert!(state
            .axioms()
            .iter()
            .all(|clause| clause.evaluations().is_some()));

        let copied_axiom = state.unprocessed().find_by_id(axiom_id).unwrap();
        assert!(copied_axiom.query_prop(CP_INITIAL));
        assert!(!copied_axiom.query_prop(CP_IS_SOS));
        let copied_conjecture = state.unprocessed().find_by_id(conjecture_id).unwrap();
        assert!(copied_conjecture.query_prop(CP_INITIAL | CP_IS_SOS));
        for clause in state.unprocessed().iter() {
            let evaluations = clause.evaluations().expect("copy is evaluated");
            assert_eq!(evaluations.eval_no(), 1);
            assert_eq!(
                evaluations.eval(0).priority(),
                PRIO_NORMAL - PRIO_LARGEST_REASONABLE
            );
        }
    }

    #[test]
    fn proof_state_init_static_watchlist_marks_matching_initial_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, watch) =
            watchlist_subsumption_pair(state.terms_mut(), "pc_static_watch", 4_020, 4_021);
        let axiom_id = axiom.ident();
        let watch_id = watch.ident();
        state.axioms_mut().insert(axiom);
        state.watchlist_mut().unwrap().insert(watch);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell {
            heuristic_name: "InitStaticWatch".to_owned(),
            watchlist_is_static: true,
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec!["InitStaticWatch=(1*FIFOWeight(ConstPrio))".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let outcome = proof_state_init(&mut state, &mut control).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome.watchlist_indexed, 1);
        assert_eq!(outcome.initial_clauses, 1);
        assert_eq!(outcome.watchlist_matches, 1);
        assert_eq!(outcome.watchlist_removed, 0);
        assert_eq!(state.watchlist().unwrap().members(), 1);
        assert!(state.watchlist().unwrap().find_by_id(watch_id).is_some());
        assert_eq!(state.archive().members(), 0);
        let copied = state.unprocessed().find_by_id(axiom_id).unwrap();
        assert!(copied.query_prop(CP_INITIAL | CP_SUBSUMES_WATCH));
        assert!(copied.is_subsume_ordered(state.terms()));
        assert_eq!(copied.weight(), copied.standard_weight());
    }

    #[test]
    fn proof_state_init_dynamic_watchlist_removes_matching_initial_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, watch) =
            watchlist_subsumption_pair(state.terms_mut(), "pc_dynamic_watch", 4_030, 4_031);
        let axiom_id = axiom.ident();
        let watch_id = watch.ident();
        state.axioms_mut().insert(axiom);
        state.watchlist_mut().unwrap().insert(watch);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell {
            heuristic_name: "InitDynamicWatch".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec!["InitDynamicWatch=(1*FIFOWeight(ConstPrio))".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let outcome = proof_state_init(&mut state, &mut control).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome.watchlist_indexed, 1);
        assert_eq!(outcome.initial_clauses, 1);
        assert_eq!(outcome.watchlist_matches, 1);
        assert_eq!(outcome.watchlist_removed, 1);
        assert_eq!(state.watchlist().unwrap().members(), 0);
        let archived = state.archive().find_by_id(watch_id).unwrap();
        assert!(archived.query_prop(CP_IS_DEAD));
        let copied = state.unprocessed().find_by_id(axiom_id).unwrap();
        assert!(copied.query_prop(CP_INITIAL | CP_SUBSUMES_WATCH));
    }

    #[test]
    fn proof_state_reset_processed_archives_originals_and_requeues_evaluated_copies() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (rule, equation, negative, non_unit) = {
            let terms = state.terms_mut();
            (
                processed_unit_clause(terms, "pc_reset_rule", 4_040),
                processed_unit_clause(terms, "pc_reset_equation", 4_041),
                processed_unit_clause(terms, "pc_reset_negative", 4_042),
                processed_unit_clause(terms, "pc_reset_non_unit", 4_043),
            )
        };
        let ids = [
            rule.ident(),
            equation.ident(),
            negative.ident(),
            non_unit.ident(),
        ];
        state.processed_pos_rules_mut().insert(rule);
        state.processed_pos_eqns_mut().insert(equation);
        state.processed_neg_units_mut().insert(negative);
        state.processed_non_units_mut().insert(non_unit);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell {
            heuristic_name: "ResetProcessedTest".to_owned(),
            prefer_initial_clauses: true,
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec!["ResetProcessedTest=(1*FIFOWeight(ConstPrio))".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let reset = proof_state_reset_processed(&mut state, &mut control).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(reset, 4);
        assert!(state.processed_pos_rules().is_empty());
        assert!(state.processed_pos_eqns().is_empty());
        assert!(state.processed_neg_units().is_empty());
        assert!(state.processed_non_units().is_empty());
        assert_eq!(state.archive().members(), 4);
        assert_eq!(state.unprocessed().members(), 4);
        for ident in ids {
            let archived = state.archive().find_by_id(ident).unwrap();
            assert!(archived.query_prop(CP_IS_PROCESSED | CP_IS_ORIENTED));
            assert!(archived.evaluations().is_none());

            let requeued = state.unprocessed().find_by_id(ident).unwrap();
            assert!(requeued.query_prop(CP_IS_PROCESSED));
            assert!(!requeued.query_prop(CP_IS_ORIENTED));
            let evaluations = requeued.evaluations().expect("requeued copy is evaluated");
            assert_eq!(evaluations.eval_no(), 1);
            assert_eq!(
                evaluations.eval(0).priority(),
                PRIO_NORMAL - PRIO_LARGEST_REASONABLE
            );
        }
    }

    #[test]
    fn proof_state_move_to_tmp_store_moves_originals_without_reevaluation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (rule, equation, negative, non_unit) = {
            let terms = state.terms_mut();
            (
                processed_unit_clause(terms, "pc_move_rule", 4_050),
                processed_unit_clause(terms, "pc_move_equation", 4_051),
                processed_unit_clause(terms, "pc_move_negative", 4_052),
                processed_unit_clause(terms, "pc_move_non_unit", 4_053),
            )
        };
        let ids = [
            rule.ident(),
            equation.ident(),
            negative.ident(),
            non_unit.ident(),
        ];
        let mut eval = evals_alloc(1);
        eval.eval_mut(0).set_priority(123);
        let mut rule = rule;
        rule.add_eval_cell(eval);
        state.processed_pos_rules_mut().insert(rule);
        state.processed_pos_eqns_mut().insert(equation);
        state.processed_neg_units_mut().insert(negative);
        state.processed_non_units_mut().insert(non_unit);

        let control = proof_control_alloc();
        let moved = proof_state_move_to_tmp_store(&mut state, &control);

        assert_eq!(moved, 4);
        assert!(state.processed_pos_rules().is_empty());
        assert!(state.processed_pos_eqns().is_empty());
        assert!(state.processed_neg_units().is_empty());
        assert!(state.processed_non_units().is_empty());
        assert_eq!(state.archive().members(), 0);
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.tmp_store().members(), 4);
        for ident in ids {
            let moved = state.tmp_store().find_by_id(ident).unwrap();
            assert!(moved.query_prop(CP_IS_PROCESSED));
            assert!(!moved.query_prop(CP_IS_ORIENTED));
        }
        let moved_rule = state.tmp_store().find_by_id(4_050).unwrap();
        assert_eq!(moved_rule.evaluations().unwrap().eval(0).priority(), 123);
    }

    #[test]
    fn proof_state_forward_modify_clause_rewrites_with_processed_demodulator() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, mut clause, target, replacement) = {
            let terms = state.terms_mut();
            let f_code = typed_binary_code(terms, "pc_forward_modify_f");
            let x = typed_var(terms, -2);
            let y = typed_var(terms, -4);
            let replacement = typed_const(terms, "pc_forward_modify_a");
            let first = typed_const(terms, "pc_forward_modify_b");
            let second = typed_const(terms, "pc_forward_modify_c");
            let rhs = typed_const(terms, "pc_forward_modify_d");
            let pattern = typed_binary_with_code(terms, f_code, &x, &y);
            let target = typed_binary_with_code(terms, f_code, &first, &second);
            let mut demod_lit = literal(terms, &pattern, &replacement, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_080);
            demodulator.set_date(SysDate::from_raw(5));
            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &target, &rhs, true)]));
            clause.set_ident(4_081);
            clause.set_prop(CP_INITIAL);
            (demodulator, clause, target, replacement)
        };
        state
            .processed_pos_rules_mut()
            .set_date(SysDate::from_raw(5));
        state.processed_pos_rules_mut().insert(demodulator);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));

        let trivial = proof_state_forward_modify_clause(
            &mut state,
            &mut control,
            &mut clause,
            false,
            false,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert_eq!(state.statistics().rw_count, 1);
        assert!(!clause.query_prop(CP_INITIAL));
        let literal = &clause.literals().as_slice()[0];
        assert_ne!(literal.left(), &target);
        assert_ne!(literal.right(), &target);
        assert!(literal.left() == &replacement || literal.right() == &replacement);
    }

    #[test]
    fn proof_state_forward_modify_clause_repeats_until_limited_rewrite_stabilizes() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, mut clause) = {
            let terms = state.terms_mut();
            let f_code = typed_binary_code(terms, "pc_forward_limited_f");
            let x = typed_var(terms, -2);
            let y = typed_var(terms, -4);
            let replacement = typed_const(terms, "pc_forward_limited_a");
            let first = typed_const(terms, "pc_forward_limited_b");
            let second = typed_const(terms, "pc_forward_limited_c");
            let rhs = typed_const(terms, "pc_forward_limited_d");
            let pattern = typed_binary_with_code(terms, f_code, &x, &y);
            let target = typed_binary_with_code(terms, f_code, &first, &second);
            let mut demod_lit = literal(terms, &pattern, &replacement, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_082);
            demodulator.set_date(SysDate::from_raw(5));
            let mut target_lit = literal(terms, &target, &rhs, true);
            target_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![target_lit]));
            clause.set_ident(4_083);
            clause.set_prop(CP_LIMITED_RW | CP_INITIAL);
            (demodulator, clause)
        };
        state
            .processed_pos_rules_mut()
            .set_date(SysDate::from_raw(5));
        state.processed_pos_rules_mut().insert(demodulator);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));

        let trivial = proof_state_forward_modify_clause(
            &mut state,
            &mut control,
            &mut clause,
            false,
            false,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert_eq!(state.statistics().rw_count, 1);
        assert!(!clause.query_prop(CP_LIMITED_RW));
        assert!(!clause.query_prop(CP_INITIAL));
    }

    #[test]
    fn proof_state_forward_subsumption_counts_processed_unit_subsumer() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (subsumer, mut clause) = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -2);
            let replacement = typed_const(terms, "pc_forward_subsumes_a");
            let instance = typed_const(terms, "pc_forward_subsumes_b");
            let mut subsumer = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &variable,
                &replacement,
                true,
            )]));
            subsumer.set_ident(4_084);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &instance,
                &replacement,
                true,
            )]));
            clause.set_ident(4_085);
            (subsumer, clause)
        };
        state.processed_pos_eqns_mut().insert(subsumer);
        let mut counts = ForwardContractCounts::default();

        let packed = proof_state_forward_subsumption(&state, &mut clause, &mut counts, false);

        assert!(packed.is_none());
        assert_eq!(counts.subsumed, 1);
    }

    #[test]
    fn proof_state_forward_contract_clause_returns_selected_marked_survivor() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let left = typed_const(terms, "pc_forward_contract_a");
            let right = typed_const(terms, "pc_forward_contract_b");
            let guard = typed_const(terms, "pc_forward_contract_c");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left, &right, true),
                literal(terms, &guard, &right, false),
            ]));
            clause.set_ident(4_086);
            clause.set_prop(CP_IS_ORIENTED);
            clause
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().selection_strategy = SELECT_NEGATIVE_LITERALS.to_owned();
        let options = ForwardContractOptions {
            non_unit_subsumption: true,
            context_sr: false,
            condense_clause: false,
            level: RewriteLevel::RuleRewrite,
        };

        let packed = proof_state_forward_contract_clause(&mut state, &mut control, clause, options)
            .unwrap_or_else(|err| panic!("{err}"))
            .expect("surviving clause should be packed");
        let survivor = packed.clause();

        assert_eq!(state.statistics().proc_forward_subsumed_count, 0);
        assert_eq!(state.statistics().proc_trivial_count, 0);
        assert!(survivor.query_prop(CP_IS_ORIENTED));
        assert_eq!(survivor.prop_lit_number(EP_IS_SELECTED), 1);
        assert!(survivor.literals().as_slice().iter().any(Eqn::is_maximal));
    }

    #[test]
    fn proof_state_forward_contract_clause_updates_subsumed_stat() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (subsumer, clause) = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -2);
            let replacement = typed_const(terms, "pc_forward_contract_subsumes_a");
            let instance = typed_const(terms, "pc_forward_contract_subsumes_b");
            let mut subsumer = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &variable,
                &replacement,
                true,
            )]));
            subsumer.set_ident(4_087);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &instance,
                &replacement,
                true,
            )]));
            clause.set_ident(4_088);
            (subsumer, clause)
        };
        state.processed_pos_eqns_mut().insert(subsumer);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let options = ForwardContractOptions {
            non_unit_subsumption: false,
            context_sr: false,
            condense_clause: false,
            level: RewriteLevel::RuleRewrite,
        };

        let packed = proof_state_forward_contract_clause(&mut state, &mut control, clause, options)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(packed.is_none());
        assert_eq!(state.statistics().proc_forward_subsumed_count, 1);
        assert_eq!(state.statistics().proc_trivial_count, 0);
    }

    #[test]
    fn proof_state_forward_contract_set_deletes_subsumed_and_counts_eliminated() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (processed_unit, redundant_clause, survivor) = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -2);
            let replacement = typed_const(terms, "pc_forward_set_subsumes_a");
            let instance = typed_const(terms, "pc_forward_set_subsumes_b");
            let other = typed_const(terms, "pc_forward_set_survives");
            let other_rhs = typed_const(terms, "pc_forward_set_survives_rhs");
            let mut processed_unit = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &variable,
                &replacement,
                true,
            )]));
            processed_unit.set_ident(4_089);
            let mut redundant_clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &instance,
                &replacement,
                true,
            )]));
            redundant_clause.set_ident(4_090);
            let mut survivor = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &other, &other_rhs, true,
            )]));
            survivor.set_ident(4_091);
            (processed_unit, redundant_clause, survivor)
        };
        state.processed_pos_eqns_mut().insert(processed_unit);
        let mut set = ClauseSet::new();
        set.insert(redundant_clause);
        set.insert(survivor);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut eliminated = 0;

        let empty = proof_state_forward_contract_set(
            &mut state,
            &mut control,
            &mut set,
            false,
            RewriteLevel::NoRewrite,
            &mut eliminated,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert_eq!(eliminated, 1);
        assert_eq!(set.members(), 1);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![4_091]
        );
    }

    #[test]
    fn proof_state_forward_contract_set_returns_empty_and_preserves_tail() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (survivor, tail) = {
            let terms = state.terms_mut();
            (
                unit_clause_with_id(terms, "pc_forward_set_before_empty", 4_092),
                unit_clause_with_id(terms, "pc_forward_set_after_empty", 4_094),
            )
        };
        let mut empty_clause = Clause::empty();
        empty_clause.set_ident(4_093);
        let mut set = ClauseSet::new();
        set.insert(survivor);
        set.insert(empty_clause);
        set.insert(tail);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut eliminated = 0;

        let empty = proof_state_forward_contract_set(
            &mut state,
            &mut control,
            &mut set,
            false,
            RewriteLevel::NoRewrite,
            &mut eliminated,
            true,
        )
        .unwrap_or_else(|err| panic!("{err}"))
        .expect("empty clause should terminate contraction");

        assert!(empty.is_empty());
        assert_eq!(empty.ident(), 4_093);
        assert_eq!(eliminated, 0);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![4_092, 4_094]
        );
    }

    #[test]
    fn proof_state_forward_contract_set_reweight_evaluates_survivors() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (first, second) = {
            let terms = state.terms_mut();
            (
                unit_clause_with_id(terms, "pc_forward_set_reweight_first", 4_095),
                unit_clause_with_id(terms, "pc_forward_set_reweight_second", 4_096),
            )
        };
        let mut set = ClauseSet::new();
        set.insert(first);
        set.insert(second);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ForwardSetReweightTest");
        let mut eliminated = 0;

        let empty = proof_state_forward_contract_set_reweight(
            &mut state,
            &mut control,
            &mut set,
            false,
            RewriteLevel::NoRewrite,
            &mut eliminated,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert_eq!(eliminated, 0);
        assert_eq!(set.members(), 2);
        for clause in set.iter() {
            let evaluations = clause
                .evaluations()
                .expect("reweighted forward-contract survivor is evaluated");
            assert_eq!(evaluations.eval_no(), 1);
            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
        }
        assert_eq!(set.eval_order_cloned(0).len(), 2);
    }

    #[test]
    fn proof_control_clause_set_filter_reweigth_removes_trivial_and_reweights() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (trivial, survivor) = {
            let terms = state.terms_mut();
            let same = typed_const(terms, "pc_filter_reweight_same");
            let survivor = unit_clause_with_id(terms, "pc_filter_reweight_survivor", 4_098);
            let mut trivial =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &same, &same, true)]));
            trivial.set_ident(4_097);
            (trivial, survivor)
        };
        let mut set = ClauseSet::new();
        set.insert(trivial);
        set.insert(survivor);
        let mut control = proof_control_alloc();
        init_fifo_hcb(&mut control, &state, "FilterReweightTest");
        let mut eliminated = 0;

        proof_control_clause_set_filter_reweigth(
            &mut control,
            state.terms(),
            &mut set,
            &mut eliminated,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(eliminated, 1);
        assert_eq!(set.members(), 1);
        let survivor = set.find_by_id(4_098).unwrap();
        assert!(survivor.evaluations().is_some());
    }

    #[test]
    fn proof_state_filter_unprocessed_contracts_and_restores_state_set() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (survivor, tail) = {
            let terms = state.terms_mut();
            (
                unit_clause_with_id(terms, "pc_filter_unprocessed_before", 4_099),
                unit_clause_with_id(terms, "pc_filter_unprocessed_after", 4_101),
            )
        };
        let mut empty_clause = Clause::empty();
        empty_clause.set_ident(4_100);
        state.unprocessed_mut().insert(survivor);
        state.unprocessed_mut().insert(empty_clause);
        state.unprocessed_mut().insert(tail);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));

        let empty = proof_state_filter_unprocessed(&mut state, &mut control, "n")
            .unwrap_or_else(|err| panic!("{err}"))
            .expect("filter descriptor should return an empty clause");

        assert_eq!(empty.ident(), 4_100);
        assert_eq!(state.statistics().proc_trivial_count, 0);
        assert_eq!(
            state
                .unprocessed()
                .iter()
                .map(Clause::ident)
                .collect::<Vec<_>>(),
            vec![4_099, 4_101]
        );
    }

    #[test]
    fn proof_state_eval_clause_set_evaluates_eval_store_and_preserves_order() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (first, second) = {
            let terms = state.terms_mut();
            (
                unit_clause_with_id(terms, "pc_eval_store_first", 4_060),
                unit_clause_with_id(terms, "pc_eval_store_second", 4_061),
            )
        };
        state.eval_store_mut().insert(first);
        state.eval_store_mut().insert(second);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell {
            heuristic_name: "EvalStoreTest".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec!["EvalStoreTest=(1*FIFOWeight(ConstPrio))".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let evaluated =
            proof_state_eval_clause_set(&mut state, &mut control).unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert_eq!(evaluated, 2);
        assert_eq!(state.eval_store().members(), 2);
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.tmp_store().members(), 0);
        assert_eq!(
            state
                .eval_store()
                .iter()
                .map(Clause::ident)
                .collect::<Vec<_>>(),
            vec![4_060, 4_061]
        );
        for clause in state.eval_store().iter() {
            let evaluations = clause
                .evaluations()
                .expect("eval-store clause is evaluated");
            assert_eq!(evaluations.eval_no(), 1);
            assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
            assert!(evaluations.object().is_some());
        }
        assert_eq!(
            state.eval_store().find_best(0).map(Clause::ident),
            Some(4_060)
        );
        assert_eq!(state.eval_store().eval_order_cloned(0).len(), 2);
    }

    #[test]
    fn proof_state_eval_clause_set_requires_active_hcb_for_nonempty_store() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = unit_clause_with_id(state.terms_mut(), "pc_eval_missing_hcb", 4_062);
        state.eval_store_mut().insert(clause);
        let mut control = proof_control_alloc();

        let error = proof_state_eval_clause_set(&mut state, &mut control).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        let clause = state.eval_store().find_by_id(4_062).unwrap();
        assert!(clause.evaluations().is_none());
    }

    #[test]
    fn proof_state_queue_generated_clause_for_eval_selects_and_stamps_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.statistics_mut().proc_non_trivial_count = 77;
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().selection_strategy = SELECT_NEGATIVE_LITERALS.to_owned();
        let mut clause = negative_clause(state.terms_mut());
        clause.set_ident(4_063);
        clause.set_prop(CP_IS_ORIENTED);
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_SELECTED);

        proof_state_queue_generated_clause_for_eval(&mut state, &mut control, clause)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(state.statistics().non_trivial_generated_count, 1);
        assert_eq!(state.eval_store().members(), 1);
        assert_eq!(state.unprocessed().members(), 0);
        let queued = state.eval_store().find_by_id(4_063).unwrap();
        assert_eq!(queued.create_date(), 77);
        assert!(!queued.query_prop(CP_IS_ORIENTED));
        assert_eq!(queued.prop_lit_number(EP_IS_SELECTED), 1);
        assert!(queued.evaluations().is_none());
    }

    #[test]
    fn proof_state_queue_generated_clause_for_eval_respects_select_on_proc_only() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.statistics_mut().proc_non_trivial_count = 88;
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().select_on_proc_only = true;
        control.heuristic_parms_mut().selection_strategy = SELECT_NEGATIVE_LITERALS.to_owned();
        let mut clause = negative_clause(state.terms_mut());
        clause.set_ident(4_064);
        clause.set_prop(CP_IS_ORIENTED);
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_SELECTED);

        proof_state_queue_generated_clause_for_eval(&mut state, &mut control, clause)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(state.statistics().non_trivial_generated_count, 1);
        let queued = state.eval_store().find_by_id(4_064).unwrap();
        assert_eq!(queued.create_date(), 88);
        assert!(!queued.query_prop(CP_IS_ORIENTED));
        assert_eq!(queued.prop_lit_number(EP_IS_SELECTED), 0);
    }

    #[test]
    fn proof_state_insert_new_clauses_routes_tmp_store_to_unprocessed() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.statistics_mut().proc_non_trivial_count = 91;
        let (first, second) = {
            let terms = state.terms_mut();
            let mut first = negative_clause(terms);
            first.set_ident(4_072);
            first.set_prop(CP_IS_ORIENTED);
            let second = unit_clause_with_id(terms, "pc_insert_new_second", 4_073);
            (first, second)
        };
        state.tmp_store_mut().insert(first);
        state.tmp_store_mut().insert(second);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewRouteTest");
        control.heuristic_parms_mut().selection_strategy = SELECT_NEGATIVE_LITERALS.to_owned();

        let empty = proof_state_insert_new_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(state.unprocessed().members(), 2);
        assert_eq!(state.statistics().generated_count, 2);
        assert_eq!(state.statistics().generated_lit_count, 2);
        assert_eq!(state.statistics().non_trivial_generated_count, 2);
        for ident in [4_072, 4_073] {
            let clause = state.unprocessed().find_by_id(ident).unwrap();
            assert_eq!(clause.create_date(), 91);
            assert!(!clause.query_prop(CP_IS_ORIENTED));
            assert!(clause.evaluations().is_some());
        }
        assert_eq!(
            state
                .unprocessed()
                .find_by_id(4_072)
                .unwrap()
                .prop_lit_number(EP_IS_SELECTED),
            1
        );
    }

    #[test]
    fn proof_state_insert_new_clauses_drops_trivial_generated_clauses() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (trivial, survivor) = {
            let terms = state.terms_mut();
            let same = typed_const(terms, "pc_insert_new_trivial_same");
            let mut trivial =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &same, &same, true)]));
            trivial.set_ident(4_074);
            let survivor = unit_clause_with_id(terms, "pc_insert_new_trivial_survivor", 4_075);
            (trivial, survivor)
        };
        state.tmp_store_mut().insert(trivial);
        state.tmp_store_mut().insert(survivor);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewTrivialTest");

        let empty = proof_state_insert_new_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(state.unprocessed().members(), 1);
        assert!(state.unprocessed().find_by_id(4_074).is_none());
        assert!(state.unprocessed().find_by_id(4_075).is_some());
        assert_eq!(state.statistics().generated_count, 2);
        assert_eq!(state.statistics().generated_lit_count, 2);
        assert_eq!(state.statistics().non_trivial_generated_count, 1);
        assert_eq!(state.statistics().proc_trivial_count, 0);
    }

    #[test]
    fn proof_state_insert_new_clauses_returns_empty_before_eval_drain() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (survivor, empty, tail) = {
            let terms = state.terms_mut();
            let survivor = unit_clause_with_id(terms, "pc_insert_new_before_empty", 4_076);
            let mut empty = Clause::empty();
            empty.set_ident(4_077);
            let tail = unit_clause_with_id(terms, "pc_insert_new_after_empty", 4_078);
            (survivor, empty, tail)
        };
        state.tmp_store_mut().insert(survivor);
        state.tmp_store_mut().insert(empty);
        state.tmp_store_mut().insert(tail);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewEmptyTest");

        let empty = proof_state_insert_new_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"))
            .expect("empty generated clause should be returned");

        assert_eq!(empty.ident(), 4_077);
        assert_eq!(state.tmp_store().members(), 1);
        assert!(state.tmp_store().find_by_id(4_078).is_some());
        assert_eq!(state.eval_store().members(), 1);
        assert!(state.eval_store().find_by_id(4_076).is_some());
        assert!(state.unprocessed().is_empty());
        assert_eq!(state.statistics().generated_count, 3);
        assert_eq!(state.statistics().generated_lit_count, 2);
        assert_eq!(state.statistics().non_trivial_generated_count, 1);
    }

    #[test]
    fn proof_state_insert_new_clauses_counts_aggressive_forward_subsumption() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (subsumer, candidate) = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -2);
            let replacement = typed_const(terms, "pc_insert_new_subsumes_a");
            let instance = typed_const(terms, "pc_insert_new_subsumes_b");
            let mut subsumer = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &variable,
                &replacement,
                false,
            )]));
            subsumer.set_ident(4_079);
            let mut candidate = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &instance,
                &replacement,
                false,
            )]));
            candidate.set_ident(4_080);
            (subsumer, candidate)
        };
        state.processed_neg_units_mut().insert(subsumer);
        state.tmp_store_mut().insert(candidate);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewSubsumedTest");
        control.heuristic_parms_mut().forward_subsumption_aggressive = true;

        let empty = proof_state_insert_new_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert!(state.unprocessed().is_empty());
        assert_eq!(state.statistics().generated_count, 1);
        assert_eq!(state.statistics().generated_lit_count, 1);
        assert_eq!(state.statistics().non_trivial_generated_count, 0);
        assert_eq!(state.statistics().aggressive_forward_subsumed_count, 1);
        assert_eq!(state.statistics().proc_forward_subsumed_count, 0);
    }

    #[test]
    fn proof_state_insert_new_clauses_rejects_unported_generated_clause_gates() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = unit_clause_with_id(state.terms_mut(), "pc_insert_new_unsupported", 4_081);
        state.tmp_store_mut().insert(clause);
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().er_aggressive = true;
        control.heuristic_parms_mut().er_varlit_destructive = true;

        let error = proof_state_insert_new_clauses(&mut state, &mut control).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(state.tmp_store().members(), 1);
        assert_eq!(state.statistics().generated_count, 0);
        assert_eq!(state.statistics().generated_lit_count, 0);
    }

    #[test]
    fn proof_state_move_eval_store_to_unprocessed_preserves_evaluations_and_indices() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut first, mut second) = {
            let terms = state.terms_mut();
            (
                unit_clause_with_id(terms, "pc_eval_move_first", 4_070),
                unit_clause_with_id(terms, "pc_eval_move_second", 4_071),
            )
        };
        first.set_prop(CP_IS_ORIENTED);
        second.set_prop(CP_IS_ORIENTED);
        let mut first_eval = evals_alloc(1);
        first_eval.eval_mut(0).set_priority(50);
        first.add_eval_cell(first_eval);
        let mut second_eval = evals_alloc(1);
        second_eval.eval_mut(0).set_priority(10);
        second.add_eval_cell(second_eval);
        state.eval_store_mut().insert(first);
        state.eval_store_mut().insert(second);

        let moved = proof_state_move_eval_store_to_unprocessed(&mut state);

        assert_eq!(moved, 2);
        assert!(state.eval_store().is_empty());
        assert_eq!(state.unprocessed().members(), 2);
        assert_eq!(
            state
                .unprocessed()
                .iter()
                .map(Clause::ident)
                .collect::<Vec<_>>(),
            vec![4_070, 4_071]
        );
        for ident in [4_070, 4_071] {
            let clause = state.unprocessed().find_by_id(ident).unwrap();
            assert!(!clause.query_prop(CP_IS_ORIENTED));
            assert!(clause.evaluations().is_some());
        }
        assert_eq!(
            state.unprocessed().find_best(0).map(Clause::ident),
            Some(4_071)
        );
    }

    #[test]
    fn proof_state_init_ac_handling_scans_initialized_unprocessed_set() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, f_code) = commutativity_axiom(state.terms_mut(), "pc_init_ac_f", 4_011);
        state.unprocessed_mut().insert(axiom);
        let mut control = proof_control_alloc();

        assert!(proof_state_init_ac_handling(&mut state, &mut control));

        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
    }

    #[test]
    fn proof_state_init_ac_handling_skips_scan_when_disabled() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, f_code) = commutativity_axiom(state.terms_mut(), "pc_init_no_ac_f", 4_012);
        state.unprocessed_mut().insert(axiom);
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().ac_handling = AcHandling::None;

        assert!(!proof_state_init_ac_handling(&mut state, &mut control));

        assert!(!control.ac_handling_active());
        assert!(!state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
    }

    #[test]
    fn proof_state_init_reports_ac_activation_after_axiom_queueing() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, f_code) = commutativity_axiom(state.terms_mut(), "pc_init_wrapped_ac_f", 4_013);
        state.axioms_mut().insert(axiom);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell::default();
        let mut hcb_defs = vec!["InitAcTest=(1*FIFOWeight(ConstPrio))".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let outcome = proof_state_init(&mut state, &mut control).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome.initial_clauses, 1);
        assert!(outcome.ac_handling_active);
        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
    }

    #[test]
    fn proof_state_init_global_indices_uses_control_index_parameters() {
        let state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().rw_bw_index_type = "FP1".to_owned();
        control.heuristic_parms_mut().pm_from_index_type = "NoIndex".to_owned();
        control.heuristic_parms_mut().pm_into_index_type = "FP7".to_owned();
        control.heuristic_parms_mut().ext_rules_max_depth = 3;
        let mut indices = global_indices_null();

        proof_state_init_global_indices(&state, &control, &mut indices, ProblemType::HigherOrder);

        assert!(indices.has_bw_rw_index());
        assert!(!indices.has_pm_from_index());
        assert!(indices.has_pm_into_index());
        assert!(indices.has_pm_negp_index());
        assert!(indices.has_ext_into_index());
        assert!(indices.has_ext_from_index());
        assert_eq!(indices.rw_bw_index_type(), "FP1");
        assert_eq!(indices.pm_from_index_type(), "NoIndex");
        assert_eq!(indices.pm_into_index_type(), "FP7");
        assert_eq!(indices.ext_rules_max_depth(), 3);
        assert_eq!(indices.problem_type(), ProblemType::HigherOrder);
    }

    #[test]
    fn proof_state_init_with_global_indices_runs_tail_after_state_init() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, _) = commutativity_axiom(state.terms_mut(), "pc_init_global_idx_f", 4_014);
        state.axioms_mut().insert(axiom);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell {
            rw_bw_index_type: "FP1".to_owned(),
            pm_from_index_type: "NoIndex".to_owned(),
            pm_into_index_type: "NoIndex".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec!["InitGlobalIdxTest=(1*FIFOWeight(ConstPrio))".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let mut indices = global_indices_null();
        let outcome = proof_state_init_with_global_indices(
            &mut state,
            &mut control,
            &mut indices,
            ProblemType::FirstOrder,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.initial_clauses, 1);
        assert!(indices.has_bw_rw_index());
        assert_eq!(indices.rw_bw_index_type(), "FP1");
        assert_eq!(indices.problem_type(), ProblemType::FirstOrder);
    }

    #[test]
    fn proof_control_init_uses_user_definition_stack_like_c() {
        let mut control = proof_control_alloc();
        let axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell {
            heuristic_name: "Alt".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let fvi_params = FvIndexParams::default();
        let wfcb_defs = vec!["custom = FIFOWeight(ConstPrio)".to_owned()];
        let mut hcb_defs = vec!["Alt=(1*custom)".to_owned()];

        proof_control_init_heuristics(
            &mut control,
            &axioms,
            &mut params,
            &fvi_params,
            &wfcb_defs,
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(control.wfcbs().find_wfcb_handle("custom").is_some());
        let alt_hcb = control
            .hcbs()
            .find_hcb_handle("Alt")
            .unwrap_or_else(|| panic!("user HCB should be installed"));
        assert_eq!(control.active_hcb(), Some(alt_hcb));
        assert_eq!(params.heuristic_def.as_deref(), Some("Alt=(1*custom)"));
        assert_eq!(hcb_defs, ["Alt=(1*custom)"]);
    }

    #[test]
    fn proof_control_init_pushes_direct_heuristic_def_like_c() {
        let mut control = proof_control_alloc();
        let axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell {
            heuristic_name: "Injected".to_owned(),
            heuristic_def: Some("Injected=(1*fifo_f)".to_owned()),
            ..HeuristicParmsCell::default()
        };
        let fvi_params = FvIndexParams::default();
        let mut hcb_defs = Vec::new();

        proof_control_init_heuristics(
            &mut control,
            &axioms,
            &mut params,
            &fvi_params,
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let injected_hcb = control
            .hcbs()
            .find_hcb_handle("Injected")
            .unwrap_or_else(|| panic!("direct HCB should be installed"));
        assert_eq!(control.active_hcb(), Some(injected_hcb));
        assert_eq!(hcb_defs, ["Injected=(1*fifo_f)"]);
        assert_eq!(params.heuristic_def.as_deref(), Some("Injected=(1*fifo_f)"));
    }

    #[test]
    fn proof_control_init_selects_ordering_then_initializes_heuristics() {
        let mut control = proof_control_alloc();
        let mut bank = test_bank();
        typed_const(&mut bank, "pc_init_order_a");
        let mut axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell::default();
        params.order_params.ordertype = TermOrdering::NoOrdering;
        let fvi_params = FvIndexParams::default();
        let mut hcb_defs = Vec::new();

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &fvi_params,
            &[],
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            control.ocb().map(|ocb| ocb.ordering_type),
            Some(TermOrdering::Kbo)
        );
        assert_eq!(
            control.active_hcb(),
            control.hcbs().find_hcb_handle(HCB_DEFAULT_HEURISTIC)
        );
        assert!(control.wfcbs().find_wfcb_handle("weight21_ugg").is_some());
    }

    #[test]
    fn inherited_literal_selection_requires_negative_pm_into_literal() {
        let mut clause = mixed_clause();
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_PM_INTO_LIT);
        assert!(!select_inherited_literal(&mut clause));
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

        clause.literals_mut().as_mut_slice()[1].set_prop(EP_IS_PM_INTO_LIT);
        assert!(select_inherited_literal(&mut clause));
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 2);
    }

    #[test]
    fn do_literal_selection_clears_state_and_applies_selector_when_enabled() {
        let mut control = proof_control_alloc();
        let mut clause = mixed_clause();
        clause.set_prop(CP_IS_ORIENTED);
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_SELECTED);
        let mut selector_calls = 0;

        let outcome =
            do_literal_selection_with_selector(&mut control, &mut clause, |_ocb, clause| {
                selector_calls += 1;
                assert!(!clause.query_prop(CP_IS_ORIENTED));
                assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
                clause.literals_mut().as_mut_slice()[1].set_prop(EP_IS_SELECTED);
            });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(selector_calls, 1);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 1);
    }

    #[test]
    fn do_literal_selection_inherited_path_bypasses_selector() {
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().inherit_paramod_lit = true;
        let mut clause = mixed_clause();
        clause.set_prop(CP_IS_ORIENTED);
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_PM_INTO_LIT);
        clause.literals_mut().as_mut_slice()[1].set_prop(EP_IS_PM_INTO_LIT);

        let outcome =
            do_literal_selection_with_selector(&mut control, &mut clause, |_ocb, _clause| {
                panic!("selector must not run after inherited selection");
            });

        assert_eq!(outcome, LiteralSelectionOutcome::Inherited);
        assert!(!clause.query_prop(CP_IS_ORIENTED));
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 2);
    }

    #[test]
    fn do_literal_selection_goal_and_conjecture_inheritance_follow_c_gate() {
        let mut goal_control = proof_control_alloc();
        goal_control.heuristic_parms_mut().inherit_goal_pm_lit = true;
        let mut goal = mixed_clause();
        goal.literals_mut().as_mut_slice()[1].set_prop(EP_IS_PM_INTO_LIT);
        assert_ne!(goal.negative_literal_count(), 0);
        goal.replace_literals(EqnList::from_vec(
            goal.literals()
                .as_slice()
                .iter()
                .filter(|literal| literal.is_negative())
                .cloned()
                .collect(),
        ));

        let outcome =
            do_literal_selection_with_selector(&mut goal_control, &mut goal, |_ocb, _clause| {
                panic!("goal inherited selection should bypass selector");
            });
        assert_eq!(outcome, LiteralSelectionOutcome::Inherited);

        let mut conjecture_control = proof_control_alloc();
        conjecture_control.heuristic_parms_mut().inherit_conj_pm_lit = true;
        let mut conjecture = mixed_clause();
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        conjecture.literals_mut().as_mut_slice()[1].set_prop(EP_IS_PM_INTO_LIT);

        let outcome = do_literal_selection_with_selector(
            &mut conjecture_control,
            &mut conjecture,
            |_ocb, _clause| {
                panic!("conjecture inherited selection should bypass selector");
            },
        );
        assert_eq!(outcome, LiteralSelectionOutcome::Inherited);
    }

    #[test]
    fn do_literal_selection_skips_selector_when_c_limits_fail() {
        let mut control = proof_control_alloc();
        let mut clause = positive_clause();
        clause.set_prop(CP_IS_ORIENTED);
        clause.literals_mut().as_mut_slice()[0].set_prop(EP_IS_SELECTED);

        let outcome =
            do_literal_selection_with_selector(&mut control, &mut clause, |_ocb, _clause| {
                panic!("selector must not run for clauses without negative literals");
            });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectionSkipped);
        assert!(!clause.query_prop(CP_IS_ORIENTED));
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    }

    #[test]
    fn do_literal_selection_named_noop_selectors_are_available() {
        let mut control = proof_control_alloc();
        let mut clause = mixed_clause();

        let outcome = do_literal_selection(&mut control, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);

        control.heuristic_parms_mut().selection_strategy = "NoGeneration".to_owned();
        let outcome = do_literal_selection(&mut control, &mut clause).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
    }

    #[test]
    fn do_literal_selection_reports_unported_strategy_only_if_reached() {
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().selection_strategy = "SelectUnlessUniqMax".to_owned();

        let mut positive = positive_clause();
        let outcome = do_literal_selection(&mut control, &mut positive).unwrap_or_else(|err| {
            panic!("{err}");
        });
        assert_eq!(outcome, LiteralSelectionOutcome::SelectionSkipped);

        let mut mixed = mixed_clause();
        let error = do_literal_selection(&mut control, &mut mixed).unwrap_err();
        assert_eq!(error.strategy(), "SelectUnlessUniqMax");
    }

    #[test]
    fn do_literal_selection_with_bank_applies_ordering_dependent_selector() {
        let mut bank = test_bank();
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().selection_strategy = SELECT_UNLESS_POS_MAX.to_owned();
        let mut clause = negative_clause(&mut bank);
        control.set_ocb(kbo_ocb(&bank));

        let outcome = do_literal_selection_with_bank(&mut control, &bank, &mut clause)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 1);
    }
}
