use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::{problem_type, ProblemType, ProverResult};
use crate::basics::sysdate::{SysDate, SysDateIncrement};
use crate::clauses::clause::{clause_print_format_string, clause_print_lop_format_string, Clause};
use crate::clauses::clause_props::{
    CP_INITIAL, CP_INPUT_FORMULA, CP_IS_DEAD, CP_IS_GLOBAL_INDEXED, CP_IS_IR_VICTIM,
    CP_IS_ORIENTED, CP_IS_PROCESSED, CP_IS_SOS, CP_LIMITED_RW, CP_NO_GENERATION, CP_SUBSUMES_WATCH,
    CP_TYPE_CONJECTURE, CP_WATCH_ONLY,
};
use crate::clauses::clausecpos::unpack_clause_pos;
use crate::clauses::clausefunc::{
    clause_archive, clause_archive_copy, clause_boolean_simplification,
    clause_eliminate_naked_boolean_variables, clause_is_orphaned_with, clause_normalize_equations,
    clause_prune_args, clause_recognize_injectivity, clause_remove_ac_resolved,
    clause_remove_ac_resolved_with_docs_and_axioms, clause_remove_superfluous_literals,
    clause_resolve_flex_clause, clause_set_delete_orphans_with, clause_set_recognize_choice,
    tformula_fcode_alloc, tformula_is_quantified_nl,
};
use crate::clauses::clausepos::ClausePos;
use crate::clauses::clausesets::{clause_set_list_get_max_date, ClauseSet};
use crate::clauses::condensation::{condense, condense_with_docs};
use crate::clauses::context_sr::{
    clause_contextual_simplify_reflect_with_bank,
    clause_contextual_simplify_reflect_with_docs_and_bank,
    clause_set_find_context_sr_clauses_with_bank,
};
use crate::clauses::derivation::{
    clause_push_derivation, clause_push_derivation_refs, clause_push_formula_derivation,
    derivation_entries, ClauseDerivationRef, DerivationEntry, DerivationParentRef,
    FormulaDerivationRef, DC_APPLY_DEF, DC_ARG_CONG, DC_CHOICE_AX, DC_CHOICE_INST, DC_CNF_EVAL_GC,
    DC_CNF_QUOTE, DC_DYNAMIC_CNF, DC_EVAL_ANSWERS, DC_EXT_EQ_FACT, DC_EXT_EQ_RES, DC_EXT_SUP,
    DC_FOF_QUOTE, DC_LEIBNIZ_ELIM, DC_NEG_EXT, DC_POS_EXT, DC_PRIM_ENUM, DC_SPLIT_EQUIV,
    DC_TRIGGER,
};
use crate::clauses::diseq_decomp::compute_dis_eq_decompositions;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqn_props::{EqnSide, PatEqnDirection, EP_IS_PM_INTO_LIT, EP_IS_SELECTED};
use crate::clauses::eqnlist::EqnList;
use crate::clauses::eqnresolution::{
    clause_er_normalize_var_with_fresh_vars, clause_er_normalize_var_with_fresh_vars_and_docs,
    compute_all_eqn_resolvents_with_fresh_vars,
    compute_all_eqn_resolvents_with_fresh_vars_and_docs, EQ_RES_ON_MAXIMAL_LITERALS_ONLY,
};
use crate::clauses::ext_index::{
    collect_ext_sup_from_pos, collect_ext_sup_into_pos, term_has_ext_eligible_subterm,
    type_ext_eligible,
};
use crate::clauses::factor::{
    compute_all_equality_factors_with_fresh_vars,
    compute_all_equality_factors_with_fresh_vars_and_docs,
};
use crate::clauses::fcvindexing::fv_index_pack_clause;
use crate::clauses::fcvindexing::FvIndexParams;
use crate::clauses::formulasets::{
    FormulaProofDocRenderOptions, FormulaSet, WrappedFormula, WrappedFormulaCnfDocContext,
};
use crate::clauses::freqvectors::FvPackedClause;
use crate::clauses::global_indices::GlobalIndices;
use crate::clauses::inferencedoc::{ClauseModificationInference, ProofDocSession};
use crate::clauses::neweval::{EvalObjectHandle, PRIO_LARGEST_REASONABLE};
use crate::clauses::paramodulation::{
    compute_all_paramodulants, compute_all_paramodulants_indexed,
    compute_all_paramodulants_indexed_with_docs, compute_all_paramodulants_with_docs,
    ParamodulationType as ClauseParamodulationType,
};
use crate::clauses::picosat::{PicoSat, PicoSatError};
use crate::clauses::proofstate::{ProofState, ProofStateGenerationContext};
use crate::clauses::rewrite::{
    clause_compute_li_normalform_plain, clause_compute_li_normalform_plain_with_docs,
    clause_local_rw,
};
use crate::clauses::rewrite::{find_rewritable_clauses, find_rewritable_clauses_indexed};
use crate::clauses::satinterface::{
    picosat_error_to_diagnostic, sat_check_proof_state_until_time_limit,
    sat_check_proof_state_with_picosat_until_time_limit, SatCheckReport,
};
use crate::clauses::splitting::{
    clause_split, ClauseSplitOutcome, ClauseSplitType as ClauseSplitMethod, SplitDefinitionStore,
};
use crate::clauses::subsumption::{
    clause_negative_simplify_reflect_with_bank,
    clause_negative_simplify_reflect_with_docs_and_bank,
    clause_positive_simplify_reflect_with_strong_and_bank,
    clause_positive_simplify_reflect_with_strong_and_docs_and_bank,
    clause_set_find_first_subsumed_clause_owned_with_bank,
    clause_set_find_subsumed_clauses_owned_with_bank, clause_set_subsumes_clause_owned,
    clause_set_subsumes_clause_owned_with_bank, clause_subsume_order_sort_lits,
    eqn_topsubsumes_termpair_with_bank, unit_clause_set_subsumes_clause,
    unit_clause_set_subsumes_clause_with_bank, unit_clause_set_subsumes_clause_with_strong,
};
use crate::clauses::subterm_index::SubtermIndex;
use crate::clauses::tautologies::clause_is_tautology;
use crate::heuristics::axiomscan::{clause_scan_ac, clause_set_scan_ac};
use crate::heuristics::clausesetfeatures::SpecFeatureCell;
use crate::heuristics::hcb::{
    hcb_clause_evaluate_with_bank, hcb_clause_set_delete_bad_clauses, hcb_clause_set_reweight,
    hcb_clause_set_reweight_with_bank, hcb_single_weight_clause_select_with,
    hcb_standard_clause_select_with, AcHandling, ExtInferenceType, GroundingStrategy,
    HcbSelectFunction, HeuristicParmsCell, ParamodulationType as HcbParamodulationType,
    PrimEnumMode, SplitClassType, SplitType,
};
use crate::heuristics::hcbadmin::HcbAdmin;
use crate::heuristics::heuristic_lookup::get_heuristic_handle_with_context;
use crate::heuristics::litselection::{
    apply_ported_literal_selector_with_bank, apply_ported_literal_selector_with_mut_bank,
    LiteralSelectionError, UnsupportedLiteralSelection, NO_GENERATION,
};
use crate::heuristics::to_autoselect::to_select_ordering;
use crate::heuristics::to_params::TermOrdering;
use crate::heuristics::wfcbadmin::{WeightParseContext, WfcbAdmin};
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::inout::signals::time_is_up;
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::ho_csu::init_unif_limits;
use crate::terms::lambda::{
    apply_terms, beta_normalize_db, close_with_db_var, close_with_type_prefix, lambda_normalize_db,
    post_cnf_encode_formulas, whnf_step,
};
use crate::terms::match_mgu::occur_check;
use crate::terms::replace::tb_term_pos_replace;
use crate::terms::simpletypes::{
    alloc_arrow_type, arrow_type_flattened, is_choice_type, type_get_max_arity, type_identity_cmp,
    type_is_predicate, Type,
};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_has_f_code, term_is_db_closed};
use crate::terms::termtypes::{DerefType, RewriteLevel, Term, TP_IS_REWRITABLE};
use crate::terms::termvars::VarBank;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt::{self, Write as _},
    hash::{BuildHasherDefault, Hasher},
    path::Path,
    time::Instant,
};

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

const IMMEDIATE_CLAUSIFICATION_RENAMING_LIMIT: i64 = 24;
const IMMEDIATE_CLAUSIFICATION_MINISCOPE_LIMIT: i64 = 100;
const TMPBANK_GC_LIMIT: i64 = 256;

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
pub enum SatSolverBackendKind {
    Internal,
    PicoSat,
}

enum SatSolverBackend {
    Internal,
    PicoSat(PicoSat),
}

impl SatSolverBackend {
    #[must_use]
    const fn kind(&self) -> SatSolverBackendKind {
        match self {
            Self::Internal => SatSolverBackendKind::Internal,
            Self::PicoSat(_) => SatSolverBackendKind::PicoSat,
        }
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CleanupUnprocessedOutcome {
    pub unsatisfiable: Option<Clause>,
    pub orphaned_deleted: i64,
    pub orphan_cleanup_triggered: bool,
    pub orphan_cleanup_deleted: i64,
    pub orphan_cleanup_remaining: i64,
    pub forward_contract_triggered: bool,
    pub forward_contract_deleted: u64,
    pub forward_contract_remaining: i64,
    pub delete_bad_triggered: bool,
    pub delete_bad_orphaned_deleted: i64,
    pub bad_deleted: i64,
    pub term_gc_recovered: i64,
}

/// Result of C `replacing_inferences`.
#[derive(Clone, Debug, PartialEq)]
pub enum ReplacingInferenceOutcome {
    /// No replacing inference fired, so the original clause remains selected.
    Survivor(Clause),
    /// The selected clause was consumed and any produced clauses were routed
    /// through `insert_new_clauses`.
    Replaced { empty: Option<Clause> },
}

/// Destination selected by C's processed-clause insertion tail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessedClauseClass {
    PositiveRule,
    PositiveEquation,
    NegativeUnit,
    NonUnit,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BackwardSimplificationOutcome {
    pub rewritten: u64,
    pub rewritten_literals: u64,
    pub subsumed: u64,
    pub unit_simplified: u64,
    pub context_sr: u64,
    pub tmp_store_marked: i64,
    pub min_rw_detected: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GenerateNewClausesOutcome {
    pub equality_factors: u64,
    pub equality_resolvents: u64,
    pub disequality_decompositions: u64,
    pub paramodulants: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessClauseReturnReason {
    EmptyClause,
    AnswerLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessClauseOutcome {
    NoClause,
    ContractedAway,
    Returned {
        clause: Clause,
        reason: ProcessClauseReturnReason,
    },
    Replaced {
        empty: Option<Clause>,
    },
    Processed {
        class: ProcessedClauseClass,
        answer_detected: bool,
        ac_activated: bool,
        watchlist: ProofStateWatchlistOutcome,
        backward: BackwardSimplificationOutcome,
        generation: GenerateNewClausesOutcome,
        generated_empty: Option<Clause>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaturateReturnReason {
    ProcessClause(ProcessClauseReturnReason),
    ReplacingInference,
    GeneratedClause,
    Cleanup,
    Filter,
    SatCheckPreprocessing,
    SatCheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaturateStopReason {
    TimeLimit,
    Saturated,
    StepLimit,
    ProcessedLimit,
    UnprocessedLimit,
    TotalLimit,
    GeneratedLimit,
    TermBankInsertionLimit,
    WatchlistEmpty,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SaturateOutcome {
    Returned {
        clause: Box<Clause>,
        reason: SaturateReturnReason,
        processed_steps: i64,
    },
    Stopped {
        reason: SaturateStopReason,
        processed_steps: i64,
    },
}

impl SaturateOutcome {
    #[must_use]
    pub const fn processed_steps(&self) -> i64 {
        match self {
            Self::Returned {
                processed_steps, ..
            }
            | Self::Stopped {
                processed_steps, ..
            } => *processed_steps,
        }
    }
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
    sat_solver_backend: SatSolverBackend,
    record_gc_selection: bool,
    strong_unit_forward_subsumption: bool,
}

/// A C level-two proof-control administration event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HeuristicAdminEvent {
    /// A weight function was added under this name.
    WeightFunction(String),
    /// A heuristic was added under this name.
    Heuristic(String),
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
            sat_solver_backend: SatSolverBackend::Internal,
            record_gc_selection: false,
            strong_unit_forward_subsumption: false,
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

    #[must_use]
    pub const fn sat_solver_backend_kind(&self) -> SatSolverBackendKind {
        self.sat_solver_backend.kind()
    }

    pub fn install_picosat_solver(&mut self, path: &Path) -> Result<Option<String>, PicoSatError> {
        let solver = PicoSat::open(path)?;
        let version = solver.version();
        self.solver = SatSolverState::new();
        self.sat_solver_backend = SatSolverBackend::PicoSat(solver);
        Ok(version)
    }

    pub fn clear_picosat_solver(&mut self) {
        self.solver = SatSolverState::new();
        self.sat_solver_backend = SatSolverBackend::Internal;
    }

    pub fn reset_sat_solver(&mut self) -> Result<(), PicoSatError> {
        if let SatSolverBackend::PicoSat(solver) = &mut self.sat_solver_backend {
            solver.reset()?;
        }
        self.solver.reset();
        Ok(())
    }

    #[must_use]
    pub const fn record_gc_selection(&self) -> bool {
        self.record_gc_selection
    }

    pub const fn set_record_gc_selection(&mut self, record: bool) {
        self.record_gc_selection = record;
    }

    #[must_use]
    pub const fn strong_unit_forward_subsumption(&self) -> bool {
        self.strong_unit_forward_subsumption
    }

    pub const fn set_strong_unit_forward_subsumption(&mut self, enabled: bool) {
        self.strong_unit_forward_subsumption = enabled;
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

pub fn proof_control_reset_sat_solver(control: &mut ProofControl) -> Result<(), PicoSatError> {
    control.reset_sat_solver()
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
    let context = WeightParseContext::new_with_signature(axioms, bank.signature());
    proof_control_init_heuristics_with_context(
        control, params, fvi_params, wfcb_defs, hcb_defs, context, None,
    )
}

/// Initializes proof control with a formula-aware axiom context.
///
/// # Errors
///
/// Returns diagnostics from ordering selection, built-in or user-supplied
/// weight/heuristic definition parsing, or active heuristic lookup.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible ProofControlInit bridge keeps the original inputs visible"
)]
pub fn proof_control_init_with_formula_axioms(
    control: &mut ProofControl,
    bank: &mut TermBank,
    axioms: &mut ClauseSet,
    formula_axioms: &FormulaSet,
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
    let context = WeightParseContext::new_with_formulas_and_signature(
        axioms,
        formula_axioms,
        bank.signature(),
    );
    proof_control_init_heuristics_with_context(
        control, params, fvi_params, wfcb_defs, hcb_defs, context, None,
    )
}

/// Initializes formula-aware proof control and returns administration events in C order.
///
/// # Errors
///
/// Returns diagnostics from ordering selection, built-in or user-supplied
/// weight/heuristic definition parsing, or active heuristic lookup.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible ProofControlInit bridge keeps the original inputs visible"
)]
pub fn proof_control_init_with_formula_axioms_and_events(
    control: &mut ProofControl,
    bank: &mut TermBank,
    axioms: &mut ClauseSet,
    formula_axioms: &FormulaSet,
    params: &mut HeuristicParmsCell,
    fvi_params: &FvIndexParams,
    wfcb_defs: &[String],
    hcb_defs: &mut Vec<String>,
    higher_order_problem: bool,
) -> Result<Vec<HeuristicAdminEvent>, Diagnostic> {
    debug_assert!(control.ocb.is_none());
    debug_assert!(control.active_hcb.is_none());

    let ocb = to_select_ordering(bank, axioms, params, higher_order_problem)?;
    control.ocb = Some(ocb);
    let context = WeightParseContext::new_with_formulas_and_signature(
        axioms,
        formula_axioms,
        bank.signature(),
    );
    let mut events = Vec::new();
    proof_control_init_heuristics_with_context(
        control,
        params,
        fvi_params,
        wfcb_defs,
        hcb_defs,
        context,
        Some(&mut events),
    )?;
    Ok(events)
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
    proof_control_init_heuristics_with_context(
        control, params, fvi_params, wfcb_defs, hcb_defs, context, None,
    )
}

pub fn proof_control_init_heuristics_with_formula_axioms(
    control: &mut ProofControl,
    axioms: &ClauseSet,
    formula_axioms: &FormulaSet,
    params: &mut HeuristicParmsCell,
    fvi_params: &FvIndexParams,
    wfcb_defs: &[String],
    hcb_defs: &mut Vec<String>,
) -> Result<(), Diagnostic> {
    debug_assert!(control.active_hcb.is_none());

    let context = WeightParseContext::new_with_formulas(axioms, formula_axioms);
    proof_control_init_heuristics_with_context(
        control, params, fvi_params, wfcb_defs, hcb_defs, context, None,
    )
}

fn proof_control_init_heuristics_with_context(
    control: &mut ProofControl,
    params: &mut HeuristicParmsCell,
    fvi_params: &FvIndexParams,
    wfcb_defs: &[String],
    hcb_defs: &mut Vec<String>,
    context: WeightParseContext<'_>,
    mut events: Option<&mut Vec<HeuristicAdminEvent>>,
) -> Result<(), Diagnostic> {
    install_default_weight_functions(control, context, events.as_deref_mut())?;
    for definition in wfcb_defs {
        install_option_weight_functions(control, definition, context, events.as_deref_mut())?;
    }

    install_default_heuristics(control, context, events.as_deref_mut())?;
    if let Some(heuristic_def) = params.heuristic_def.clone() {
        hcb_defs.push(heuristic_def);
    } else if let Some(heuristic_def) = hcb_defs.last() {
        params.heuristic_def = Some(heuristic_def.clone());
    }
    for definition in hcb_defs.iter() {
        install_option_heuristics(control, definition, context, events.as_deref_mut())?;
    }

    control.heuristic_parms = params.clone();
    let weight_start = control.wfcbs.len();
    let heuristic_start = control.hcbs.len();
    control.active_hcb = Some(get_heuristic_handle_with_context(
        &params.heuristic_name,
        &mut control.hcbs,
        &mut control.wfcbs,
        context,
    )?);
    record_admin_additions(control, weight_start, heuristic_start, events);
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
    state.init_watchlist(ocb)
}

/// Initializes the currently ported proof-state portions of C
/// `ProofStateInit`.
///
/// This covers the processed-set precondition, FV-index/watchlist prefix,
/// `Uniq` ordering of axioms, copying axioms into `unprocessed`, initial-clause
/// watchlist checks, active-HCB evaluation, `prefer_initial_clauses` priority
/// adjustment, SOS marking, and AC scanning. Use the documentation wrappers for
/// represented proof-output side effects; state-owned global-index storage
/// remains pending.
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
    proof_state_init_impl::<String, _>(state, control, None, None, |state, control| {
        Ok(proof_state_init_ac_handling(state, control))
    })
}

/// Initializes the ported proof-state portions of C `ProofStateInit` while
/// emitting represented initial-clause `eval` proof-documentation quotes.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_init`], plus any
/// proof-documentation write diagnostic.
pub fn proof_state_init_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<ProofStateInitOutcome, Diagnostic> {
    proof_state_init_impl(
        state,
        control,
        Some((output, session)),
        None,
        |state, control| Ok(proof_state_init_ac_handling(state, control)),
    )
}

/// Initializes the ported proof-state portions of C `ProofStateInit` while
/// emitting represented initial-clause `eval` proof-documentation quotes and
/// rendering represented `OutputLevel` text from AC scanning.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_init_with_docs`], plus any
/// output diagnostic from AC scan/status rendering.
pub fn proof_state_init_with_docs_and_output(
    output: &mut (impl fmt::Write + std::io::Write),
    session: &mut ProofDocSession,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<ProofStateInitOutcome, Diagnostic> {
    proof_state_write_init_banner(output, output_level)?;
    debug_assert!(state.processed_pos_rules().is_empty());
    debug_assert!(state.processed_pos_eqns().is_empty());
    debug_assert!(state.processed_neg_units().is_empty());
    debug_assert!(state.processed_non_units().is_empty());

    let _ = proof_state_recognize_choice_axioms(state, control)?;
    let watchlist_indexed = proof_state_init_indexing(state, control)?;
    let axiom_outcome = {
        let mut doc_context = Some((&mut *output, session));
        let mut output_context = None;
        proof_state_init_axioms_impl(state, control, &mut doc_context, &mut output_context)?
    };
    let ac_handling_active =
        proof_state_init_ac_handling_with_output(output, output_level, state, control)?;
    Ok(ProofStateInitOutcome {
        watchlist_indexed,
        initial_clauses: axiom_outcome.initial_clauses,
        sos_marked: axiom_outcome.sos_marked,
        watchlist_matches: axiom_outcome.watchlist_matches,
        watchlist_removed: axiom_outcome.watchlist_removed,
        ac_handling_active,
    })
}

/// Initializes the ported proof-state portions of C `ProofStateInit` while
/// rendering represented `OutputLevel` text.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_init`], plus any output
/// diagnostic from watchlist or AC scan/status rendering.
pub fn proof_state_init_with_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<ProofStateInitOutcome, Diagnostic> {
    let output = output as &mut dyn std::io::Write;
    proof_state_write_init_banner(output, output_level)?;
    let mut output_context = Some((output, output_level));
    debug_assert!(state.processed_pos_rules().is_empty());
    debug_assert!(state.processed_pos_eqns().is_empty());
    debug_assert!(state.processed_neg_units().is_empty());
    debug_assert!(state.processed_non_units().is_empty());

    let _ = proof_state_recognize_choice_axioms(state, control)?;
    let watchlist_indexed = proof_state_init_indexing(state, control)?;
    let axiom_outcome = {
        let mut doc_context = None;
        proof_state_init_axioms_impl::<String>(
            state,
            control,
            &mut doc_context,
            &mut output_context,
        )?
    };
    let Some((output, output_level)) = output_context.as_mut() else {
        unreachable!("proof-state output context should remain installed");
    };
    let ac_handling_active =
        proof_state_init_ac_handling_with_output(&mut **output, *output_level, state, control)?;
    Ok(ProofStateInitOutcome {
        watchlist_indexed,
        initial_clauses: axiom_outcome.initial_clauses,
        sos_marked: axiom_outcome.sos_marked,
        watchlist_matches: axiom_outcome.watchlist_matches,
        watchlist_removed: axiom_outcome.watchlist_removed,
        ac_handling_active,
    })
}

fn proof_state_init_impl<W, A>(
    state: &mut ProofState,
    control: &mut ProofControl,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
    mut output_context: Option<(&mut dyn std::io::Write, i64)>,
    mut ac_scan: A,
) -> Result<ProofStateInitOutcome, Diagnostic>
where
    W: fmt::Write,
    A: FnMut(&mut ProofState, &mut ProofControl) -> Result<bool, Diagnostic>,
{
    debug_assert!(state.processed_pos_rules().is_empty());
    debug_assert!(state.processed_pos_eqns().is_empty());
    debug_assert!(state.processed_neg_units().is_empty());
    debug_assert!(state.processed_non_units().is_empty());

    let _ = proof_state_recognize_choice_axioms(state, control)?;
    let watchlist_indexed = proof_state_init_indexing(state, control)?;
    let axiom_outcome =
        proof_state_init_axioms_impl(state, control, &mut doc_context, &mut output_context)?;
    let ac_handling_active = ac_scan(state, control)?;
    Ok(ProofStateInitOutcome {
        watchlist_indexed,
        initial_clauses: axiom_outcome.initial_clauses,
        sos_marked: axiom_outcome.sos_marked,
        watchlist_matches: axiom_outcome.watchlist_matches,
        watchlist_removed: axiom_outcome.watchlist_removed,
        ac_handling_active,
    })
}

/// Runs C `ProofStateInit`, then initializes the proof state's global indices.
///
/// # Errors
///
/// Returns diagnostics from [`proof_state_init`].
pub fn proof_state_init_with_global_indices(
    state: &mut ProofState,
    control: &mut ProofControl,
    problem_type: ProblemType,
) -> Result<ProofStateInitOutcome, Diagnostic> {
    let outcome = proof_state_init(state, control)?;
    proof_state_init_global_indices(state, control, problem_type);
    Ok(outcome)
}

/// Runs C `ProofStateInit` with proof-documentation quotes, then initializes
/// the proof state's global indices.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_init_with_docs`].
pub fn proof_state_init_with_global_indices_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    problem_type: ProblemType,
) -> Result<ProofStateInitOutcome, Diagnostic> {
    let outcome = proof_state_init_with_docs(output, session, state, control)?;
    proof_state_init_global_indices(state, control, problem_type);
    Ok(outcome)
}

/// Inserts the initialized proof-state watchlist into its global indices.
///
/// C stores this owner in `state->wlindices` and calls
/// `GlobalIndicesInsertClauseSet` at the tail of `ProofStateInitWatchlist`.
pub fn proof_state_insert_watchlist_global_indices(
    state: &mut ProofState,
    lambda_demod: bool,
) -> i64 {
    state.with_watchlist_indices(|state, indices| {
        proof_state_insert_watchlist_global_indices_into(state, indices, lambda_demod)
    })
}

/// Inserts the initialized watchlist into an explicitly supplied index owner.
///
/// This lower-level variant supports isolated index tests and callers that are
/// constructing a proof state in stages.
pub fn proof_state_insert_watchlist_global_indices_into(
    state: &mut ProofState,
    indices: &mut GlobalIndices,
    lambda_demod: bool,
) -> i64 {
    let (terms, watchlist) = state.terms_and_watchlist_mut();
    let Some(watchlist) = watchlist else {
        return 0;
    };
    indices.insert_clause_set(watchlist, terms, lambda_demod)
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
    let mut doc_context = None;
    let mut output_context = None;
    proof_state_init_axioms_impl::<String>(state, control, &mut doc_context, &mut output_context)
}

pub fn proof_state_recognize_choice_axioms(
    state: &mut ProofState,
    control: &ProofControl,
) -> Result<i64, Diagnostic> {
    if problem_type() != ProblemType::HigherOrder
        || control.heuristic_parms().inst_choice_max_depth < 0
    {
        return Ok(0);
    }

    let (terms, axioms, choice_opcodes) = state.terms_axioms_choice_opcodes_mut();
    clause_set_recognize_choice(terms, axioms, choice_opcodes)
}

/// Runs the axiom-queue portion of C `ProofStateInit` while emitting
/// represented initial-clause `eval` proof-documentation quotes.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_init_axioms`], plus any
/// proof-documentation write diagnostic.
pub fn proof_state_init_axioms_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<ProofStateInitAxiomOutcome, Diagnostic> {
    let mut doc_context = Some((output, session));
    let mut output_context = None;
    proof_state_init_axioms_impl(state, control, &mut doc_context, &mut output_context)
}

fn proof_state_init_axioms_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
    output_context: &mut Option<(&mut dyn std::io::Write, i64)>,
) -> Result<ProofStateInitAxiomOutcome, Diagnostic> {
    let active_hcb_handle = control.active_hcb.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ProofStateInit requires initialized proof-control heuristic",
        )
    })?;
    let context = WeightParseContext::new_with_formulas_and_signature(
        state.axioms(),
        state.f_axioms(),
        state.terms().signature(),
    );
    let uniq_hcb_handle =
        get_heuristic_handle_with_context("Uniq", &mut control.hcbs, &mut control.wfcbs, context)?;

    {
        let ProofControl {
            hcbs, wfcbs, ocb, ..
        } = control;
        let uniq_hcb = hcbs
            .hcb(uniq_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("Uniq"))?;
        let ocb = ocb.as_mut().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "ProofStateInit requires initialized proof-control ordering",
            )
        })?;
        let (terms, axioms) = state.terms_and_axioms_mut();
        hcb_clause_set_reweight_with_bank(uniq_hcb, wfcbs, ocb, terms, axioms)?;
    }

    let ordered_axioms = state.axioms().eval_order_objects(0);
    let prefer_initial = control.heuristic_parms.prefer_initial_clauses;
    let static_watchlist = control.heuristic_parms.watchlist_is_static;
    let lambda_demod = control.heuristic_parms.lambda_demod;
    let use_tptp_sos = control.heuristic_parms.use_tptp_sos;
    let record_gc_selection = control.record_gc_selection();
    let mut initial_clauses = 0;
    let mut watchlist_matches = 0;
    let mut watchlist_removed = 0;

    state.unprocessed_mut().reserve_exact(ordered_axioms.len());

    {
        let ProofControl {
            hcbs, wfcbs, ocb, ..
        } = control;
        let active_hcb = hcbs
            .hcb(active_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("active"))?;
        let ocb = ocb.as_mut().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "ProofStateInit requires initialized proof-control ordering",
            )
        })?;

        for source_object in ordered_axioms {
            let (mut new, source_ref) = proof_state_copy_evaluated_axiom(state, source_object)?;
            new.refresh_derivation_generation();
            new.set_prop(CP_INITIAL);
            let watchlist_outcome = proof_state_check_watchlist_maybe_output(
                state,
                &mut new,
                static_watchlist,
                lambda_demod,
                None,
                doc_context,
                output_context.as_mut(),
            )?;
            if watchlist_outcome.subsumes_watch {
                watchlist_matches += 1;
            }
            watchlist_removed += watchlist_outcome.removed;

            hcb_clause_evaluate_with_bank(active_hcb, wfcbs, ocb, state.terms_mut(), &mut new)?;
            new.del_prop(CP_INPUT_FORMULA);
            if let Some((output, session)) = doc_context.as_mut() {
                session.doc_clause_quote(output, state.terms(), 6, &mut new, Some("eval"), None)?;
            }
            clause_push_derivation_refs(&mut new, DC_CNF_QUOTE, Some(source_ref), None);
            if record_gc_selection {
                clause_push_derivation(&mut new, DC_CNF_EVAL_GC, None, None);
            }
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

fn proof_state_copy_evaluated_axiom(
    state: &mut ProofState,
    source_object: EvalObjectHandle,
) -> Result<(Clause, ClauseDerivationRef), Diagnostic> {
    let (terms, axioms) = state.terms_and_axioms_mut();
    let source = axioms.find_by_eval_object(source_object).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ProofStateInit axiom evaluation index references a missing clause",
        )
    })?;
    Ok((
        source.copy_to_bank(terms)?,
        ClauseDerivationRef::from(source),
    ))
}

/// Runs C `check_watchlist` against the proof-state watchlist.
///
/// The current Rust path updates the local watchlist FV index and archive.
/// Long-lived `wlindices` deletion is wired with the later state-owned
/// global-index integration. Use [`proof_state_check_watchlist_with_docs`] for
/// represented proof-documentation quote side effects, or
/// [`proof_state_check_watchlist_with_output`] for C's `OutputLevel` text only.
///
/// # Panics
///
/// Panics if the internal non-documenting path reports a proof-documentation
/// diagnostic, which would indicate a bug because no proof-doc writer is
/// installed.
#[must_use]
pub fn proof_state_check_watchlist(
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    lambda_demod: bool,
) -> ProofStateWatchlistOutcome {
    let mut doc_context = None;
    let mut output_context = None;
    proof_state_check_watchlist_impl::<String>(
        state,
        clause,
        static_watchlist,
        lambda_demod,
        None,
        &mut doc_context,
        output_context.as_mut(),
    )
    .unwrap_or_else(|err| panic!("plain watchlist check unexpectedly failed: {err}"))
}

/// Runs C `check_watchlist` while emitting represented proof-documentation
/// quotes for dynamic watchlist extraction.
///
/// # Errors
///
/// Returns any proof-documentation write diagnostic.
pub fn proof_state_check_watchlist_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    lambda_demod: bool,
) -> Result<ProofStateWatchlistOutcome, Diagnostic> {
    let mut doc_context = Some((output, session));
    let mut output_context = None;
    proof_state_check_watchlist_impl(
        state,
        clause,
        static_watchlist,
        lambda_demod,
        None,
        &mut doc_context,
        output_context.as_mut(),
    )
}

/// Runs C `check_watchlist` while maintaining explicitly supplied watchlist global
/// indices.
///
/// # Panics
///
/// Panics if the internal non-documenting path reports a proof-documentation
/// diagnostic, which would indicate a bug because no proof-doc writer is
/// installed.
#[must_use]
pub fn proof_state_check_watchlist_with_global_indices(
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    lambda_demod: bool,
    watchlist_indices: &mut GlobalIndices,
) -> ProofStateWatchlistOutcome {
    let mut doc_context = None;
    let mut output_context = None;
    proof_state_check_watchlist_impl::<String>(
        state,
        clause,
        static_watchlist,
        lambda_demod,
        Some(watchlist_indices),
        &mut doc_context,
        output_context.as_mut(),
    )
    .unwrap_or_else(|err| panic!("indexed watchlist check unexpectedly failed: {err}"))
}

/// Runs C `check_watchlist` with proof docs while maintaining explicitly supplied
/// watchlist global indices.
///
/// # Errors
///
/// Returns any proof-documentation write diagnostic.
pub fn proof_state_check_watchlist_with_global_indices_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    lambda_demod: bool,
    watchlist_indices: &mut GlobalIndices,
) -> Result<ProofStateWatchlistOutcome, Diagnostic> {
    let mut doc_context = Some((output, session));
    let mut output_context = None;
    proof_state_check_watchlist_impl(
        state,
        clause,
        static_watchlist,
        lambda_demod,
        Some(watchlist_indices),
        &mut doc_context,
        output_context.as_mut(),
    )
}

/// Runs C `check_watchlist` while rendering only C's `OutputLevel` text.
///
/// # Errors
///
/// Returns a diagnostic if the output sink fails while printing the dynamic
/// watchlist-reduction message.
pub fn proof_state_check_watchlist_with_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    lambda_demod: bool,
) -> Result<ProofStateWatchlistOutcome, Diagnostic> {
    let mut doc_context = None;
    let output = output as &mut dyn std::io::Write;
    let mut output_context = Some((output, output_level));
    proof_state_check_watchlist_impl::<String>(
        state,
        clause,
        static_watchlist,
        lambda_demod,
        None,
        &mut doc_context,
        output_context.as_mut(),
    )
}

fn proof_state_check_watchlist_with_optional_indices_and_output(
    output: &mut dyn std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    lambda_demod: bool,
    watchlist_indices: Option<&mut GlobalIndices>,
) -> Result<ProofStateWatchlistOutcome, Diagnostic> {
    let mut doc_context = None;
    let mut output_context = (output, output_level);
    proof_state_check_watchlist_impl::<String>(
        state,
        clause,
        static_watchlist,
        lambda_demod,
        watchlist_indices,
        &mut doc_context,
        Some(&mut output_context),
    )
}

fn proof_state_check_watchlist_maybe_output<W: fmt::Write>(
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    lambda_demod: bool,
    watchlist_indices: Option<&mut GlobalIndices>,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
    output_context: Option<&mut (&mut dyn std::io::Write, i64)>,
) -> Result<ProofStateWatchlistOutcome, Diagnostic> {
    proof_state_check_watchlist_impl(
        state,
        clause,
        static_watchlist,
        lambda_demod,
        watchlist_indices,
        doc_context,
        output_context,
    )
}

fn proof_state_check_watchlist_impl<W: fmt::Write>(
    state: &mut ProofState,
    clause: &mut Clause,
    static_watchlist: bool,
    lambda_demod: bool,
    watchlist_indices: Option<&mut GlobalIndices>,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
    mut output_context: Option<&mut (&mut dyn std::io::Write, i64)>,
) -> Result<ProofStateWatchlistOutcome, Diagnostic> {
    let (terms, watchlist, archive) = state.terms_watchlist_archive_mut();
    let Some(watchlist) = watchlist else {
        return Ok(ProofStateWatchlistOutcome::default());
    };

    clause.subsume_order_sort_literals(terms);
    clause.set_weight(clause.standard_weight());

    if static_watchlist {
        let subsumed =
            clause_set_find_first_subsumed_clause_owned_with_bank(watchlist, clause, terms)?;
        if subsumed.is_some() {
            clause.set_prop(CP_SUBSUMES_WATCH);
            return Ok(ProofStateWatchlistOutcome {
                subsumes_watch: true,
                removed: 0,
            });
        }
        return Ok(ProofStateWatchlistOutcome::default());
    }

    let removed = remove_watchlist_subsumed(
        watchlist,
        archive,
        clause,
        terms,
        lambda_demod,
        watchlist_indices,
        doc_context,
    )?;
    if removed != 0 {
        clause.set_prop(CP_SUBSUMES_WATCH);
        if let Some((output, session)) = doc_context.as_mut() {
            if session.output_level == 1 {
                writeln!(
                    output,
                    "{DEFAULT_COMCHAR_RAW} Watchlist reduced by {removed} clause{}",
                    if removed == 1 { "" } else { "s" }
                )
                .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
            }
            session.doc_clause_quote(
                output,
                terms,
                6,
                clause,
                Some("extract_subsumed_watched"),
                None,
            )?;
        } else if let Some((output, output_level)) = output_context.as_mut() {
            proof_state_write_watchlist_reduction(&mut **output, *output_level, removed)?;
        }
        return Ok(ProofStateWatchlistOutcome {
            subsumes_watch: true,
            removed,
        });
    }
    Ok(ProofStateWatchlistOutcome::default())
}

fn proof_state_write_watchlist_reduction(
    output: &mut (impl std::io::Write + ?Sized),
    output_level: i64,
    removed: i64,
) -> Result<(), Diagnostic> {
    if output_level != 1 {
        return Ok(());
    }
    let line = format!(
        "{DEFAULT_COMCHAR_RAW} Watchlist reduced by {removed} clause{}\n",
        if removed == 1 { "" } else { "s" }
    );
    std::io::Write::write_all(output, line.as_bytes())
        .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))
}

fn proof_state_write_init_banner(
    output: &mut (impl std::io::Write + ?Sized),
    output_level: i64,
) -> Result<(), Diagnostic> {
    if output_level < 1 {
        return Ok(());
    }
    let line = format!("{DEFAULT_COMCHAR_RAW} Initializing proof state\n");
    std::io::Write::write_all(output, line.as_bytes())
        .map_err(|error| proof_control_io_error(&error))
}

/// Runs the local owned-watchlist body of C `simplify_watchlist`.
///
/// This uses a plain scan of the owned watchlist, archives each rewritable
/// watched original as dead, normalizes the quoted flat copy with the processed
/// demodulator sets, minimizes/AC-cleans it, marks maximal terms, and reinserts
/// it through the watchlist FV index. Long-lived `wlindices` deletion/insertion
/// remains later integration work.
///
/// # Errors
///
/// Returns diagnostics from backward-rewrite matching, archive copies, or
/// leftmost-innermost normalization.
pub fn proof_state_simplify_watchlist(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
) -> Result<i64, Diagnostic> {
    proof_state_simplify_watchlist_impl::<String>(state, control, clause, None, None)
}

/// Runs C `simplify_watchlist` while emitting represented proof docs.
///
/// This wires rewrite documentation from watched-clause normalization and the
/// `inf_minimize` modification quote emitted after superfluous literal removal.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_simplify_watchlist`], plus any
/// proof-documentation write diagnostic.
pub fn proof_state_simplify_watchlist_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
) -> Result<i64, Diagnostic> {
    proof_state_simplify_watchlist_impl(state, control, clause, None, Some((output, session)))
}

/// Runs C `simplify_watchlist` while maintaining explicitly supplied watchlist global
/// indices.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_simplify_watchlist`].
pub fn proof_state_simplify_watchlist_with_global_indices(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    watchlist_indices: &mut GlobalIndices,
) -> Result<i64, Diagnostic> {
    proof_state_simplify_watchlist_impl::<String>(
        state,
        control,
        clause,
        Some(watchlist_indices),
        None,
    )
}

/// Runs C `simplify_watchlist` with proof docs while maintaining explicitly supplied
/// watchlist global indices.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_simplify_watchlist_with_docs`].
pub fn proof_state_simplify_watchlist_with_global_indices_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    watchlist_indices: &mut GlobalIndices,
) -> Result<i64, Diagnostic> {
    proof_state_simplify_watchlist_impl(
        state,
        control,
        clause,
        Some(watchlist_indices),
        Some((output, session)),
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "C-compatible watchlist simplification keeps removal, normalization, and reinsertion together"
)]
fn proof_state_simplify_watchlist_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    mut watchlist_indices: Option<&mut GlobalIndices>,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    if !clause.is_demodulator() || state.watchlist().is_none_or(ClauseSet::is_empty) {
        return Ok(0);
    }
    let ac_axiom_parents = state.ac_axiom_parent_refs();

    let ids = {
        let ocb = control.ocb.as_mut().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "simplify_watchlist requires initialized proof-control ordering",
            )
        })?;
        let (terms, watchlist, _) = state.terms_watchlist_archive_mut();
        let Some(watchlist) = watchlist else {
            return Ok(0);
        };
        let bw_rw_index = watchlist_indices
            .as_ref()
            .and_then(|indices| indices.bw_rw_index());
        let (_found, ids) =
            rewritable_ids_in_watchlist(terms, ocb, watchlist, bw_rw_index, clause, clause.date())?;
        ids
    };

    let mut tmp_set = ClauseSet::new();
    for id in ids.into_iter().rev() {
        let Some(watchlist) = state.watchlist_mut() else {
            break;
        };
        let Some(mut watched) = watchlist.extract_by_id(id) else {
            continue;
        };
        if watched.query_prop(CP_IS_GLOBAL_INDEXED) {
            if let Some(indices) = watchlist_indices.as_deref_mut() {
                indices.delete_clause(
                    &mut watched,
                    state.terms(),
                    control.heuristic_parms().lambda_demod,
                );
            }
        }
        let requeued = proof_state_archive_simplified_clause(state, watched)?;
        tmp_set.insert(requeued);
    }

    let mut simplified = 0;
    while let Some(mut handle) = tmp_set.extract_first() {
        let forward_demod = control.heuristic_parms().forward_demod;
        let prefer_general = control.heuristic_parms().prefer_general;
        let lambda_demod = control.heuristic_parms().lambda_demod;
        let rw_delta = {
            let ocb = control.ocb.as_mut().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "simplify_watchlist normalization requires initialized proof-control ordering",
                )
            })?;
            let (terms, processed_sets) = state.terms_and_processed_sets_mut();
            let demodulators: [&ClauseSet; 2] =
                [&*processed_sets.pos_rules, &*processed_sets.pos_eqns];
            match doc_context.as_mut() {
                Some((output, session)) => clause_compute_li_normalform_plain_with_docs(
                    output,
                    session,
                    terms,
                    ocb,
                    &mut handle,
                    &demodulators,
                    forward_demod,
                    prefer_general,
                    lambda_demod,
                )?,
                None => clause_compute_li_normalform_plain(
                    terms,
                    ocb,
                    &mut handle,
                    &demodulators,
                    forward_demod,
                    prefer_general,
                    lambda_demod,
                )?,
            }
        };
        state.statistics_mut().rw_count += i64_to_u64_saturating(rw_delta);

        let removed_lits = clause_remove_superfluous_literals(&mut handle, state.terms());
        if removed_lits != 0 {
            if let Some((output, session)) = doc_context.as_mut() {
                session.doc_clause_modification(
                    output,
                    state.terms(),
                    &mut handle,
                    ClauseModificationInference::Minimize,
                    None,
                    None,
                )?;
            }
        }
        if control.ac_handling_active() {
            match doc_context.as_mut() {
                Some((output, session)) => {
                    clause_remove_ac_resolved_with_docs_and_axioms(
                        output,
                        session,
                        &mut handle,
                        state.terms(),
                        &ac_axiom_parents,
                    )?;
                }
                None => {
                    clause_remove_ac_resolved(&mut handle, state.terms());
                }
            }
        }
        handle.set_weight(handle.standard_weight());
        {
            let ocb = control.ocb.as_mut().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "simplify_watchlist maximal marking requires initialized proof-control ordering",
                )
            })?;
            handle.mark_maximal_terms(ocb, state.terms());
        }
        let (terms, watchlist, _) = state.terms_watchlist_archive_mut();
        if let Some(watchlist) = watchlist {
            if let Some(indices) = watchlist_indices.as_deref_mut() {
                indices.insert_clause(&mut handle, terms, control.heuristic_parms().lambda_demod);
            }
            watchlist.indexed_insert_clause_owned_with_bank(handle, terms)?;
            simplified += 1;
        }
    }

    Ok(simplified)
}

fn remove_watchlist_subsumed<W: fmt::Write>(
    watchlist: &mut ClauseSet,
    archive: &mut ClauseSet,
    subsumer: &Clause,
    terms: &mut TermBank,
    lambda_demod: bool,
    mut watchlist_indices: Option<&mut GlobalIndices>,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let mut stack = PStack::new();
    let expected_removed =
        clause_set_find_subsumed_clauses_owned_with_bank(watchlist, subsumer, &mut stack, terms)?;
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
        if clause.query_prop(CP_IS_GLOBAL_INDEXED) {
            if let Some(indices) = watchlist_indices.as_deref_mut() {
                indices.delete_clause(&mut clause, terms, lambda_demod);
            }
        }
        if let Some((output, session)) = doc_context.as_mut() {
            let comment = if clause.query_prop(CP_WATCH_ONLY) {
                "extract_wl_subsumed"
            } else {
                "subsumed"
            };
            session.doc_clause_quote(
                output,
                terms,
                6,
                &mut clause,
                Some(comment),
                Some(subsumer),
            )?;
        }
        clause.set_prop(CP_IS_DEAD);
        archive.insert(clause);
        removed += 1;
    }

    debug_assert_eq!(removed, expected_removed);
    Ok(removed)
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
    proof_state_reset_processed_impl::<String>(state, control, None, None)
}

/// Moves all processed clauses back to `unprocessed` while emitting represented
/// proof-documentation quotes for the evaluated requeued copies.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_reset_processed`], plus any
/// proof-documentation write diagnostic.
pub fn proof_state_reset_processed_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<i64, Diagnostic> {
    proof_state_reset_processed_impl(state, control, None, Some((output, session)))
}

/// Moves all processed clauses back to `unprocessed`, deleting any
/// explicitly supplied global-index entries before the clauses move.
///
/// This matches the indexed C `ProofStateResetProcessed` path where
/// `GlobalIndicesDeleteClause` runs while the processed-set clause still has
/// its original address-shaped identity.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_reset_processed`].
pub fn proof_state_reset_processed_with_global_indices(
    state: &mut ProofState,
    control: &mut ProofControl,
    indices: &mut GlobalIndices,
) -> Result<i64, Diagnostic> {
    proof_state_reset_processed_impl::<String>(state, control, Some(indices), None)
}

/// Moves all processed clauses back to `unprocessed`, deleting explicitly supplied
/// global-index entries and emitting represented proof-documentation quotes.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_reset_processed_with_docs`].
pub fn proof_state_reset_processed_with_global_indices_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    indices: &mut GlobalIndices,
) -> Result<i64, Diagnostic> {
    proof_state_reset_processed_impl(state, control, Some(indices), Some((output, session)))
}

fn proof_state_reset_processed_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    mut indices: Option<&mut GlobalIndices>,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic> {
    let active_hcb_handle = control.active_hcb.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ProofStateResetProcessed requires initialized proof-control heuristic",
        )
    })?;
    let reset_options = ResetProcessedOptions {
        prefer_initial: control.heuristic_parms.prefer_initial_clauses,
        record_gc_selection: control.record_gc_selection(),
        lambda_demod: control.heuristic_parms.lambda_demod,
    };
    let mut reset = 0;

    {
        let ProofControl {
            hcbs, wfcbs, ocb, ..
        } = control;
        let active_hcb = hcbs
            .hcb(active_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("active"))?;
        let ocb = ocb.as_mut().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "ProofStateResetProcessed requires initialized proof-control ordering",
            )
        })?;
        let mut evaluate = |bank: &mut TermBank, clause: &mut Clause| {
            hcb_clause_evaluate_with_bank(active_hcb, wfcbs, ocb, bank, clause)
        };

        reset += proof_state_reset_processed_set_by(
            state,
            ProcessedSetSlot::PosRules,
            reset_options,
            &mut evaluate,
            indices.as_deref_mut(),
            &mut doc_context,
        )?;
        reset += proof_state_reset_processed_set_by(
            state,
            ProcessedSetSlot::PosEqns,
            reset_options,
            &mut evaluate,
            indices.as_deref_mut(),
            &mut doc_context,
        )?;
        reset += proof_state_reset_processed_set_by(
            state,
            ProcessedSetSlot::NegUnits,
            reset_options,
            &mut evaluate,
            indices.as_deref_mut(),
            &mut doc_context,
        )?;
        reset += proof_state_reset_processed_set_by(
            state,
            ProcessedSetSlot::NonUnits,
            reset_options,
            &mut evaluate,
            indices,
            &mut doc_context,
        )?;
    }

    Ok(reset)
}

#[derive(Clone, Copy)]
struct ResetProcessedOptions {
    prefer_initial: bool,
    record_gc_selection: bool,
    lambda_demod: bool,
}

fn proof_state_reset_processed_set_by<E, W: fmt::Write>(
    state: &mut ProofState,
    slot: ProcessedSetSlot,
    options: ResetProcessedOptions,
    evaluate: &mut E,
    mut indices: Option<&mut GlobalIndices>,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<i64, Diagnostic>
where
    E: FnMut(&mut TermBank, &mut Clause) -> Result<(), Diagnostic>,
{
    let mut reset = 0;
    while !processed_set_by_slot(state, slot).is_empty() {
        if let Some(indices) = indices.as_deref_mut() {
            proof_state_delete_first_global_indexed_clause_from_slot(
                state,
                slot,
                indices,
                options.lambda_demod,
            );
        }
        let Some(handle) = processed_set_mut_by_slot(state, slot).extract_first() else {
            continue;
        };
        proof_state_reset_processed_clause(state, handle, options, evaluate, doc_context)?;
        reset += 1;
    }
    Ok(reset)
}

fn proof_state_reset_processed_clause<E, W: fmt::Write>(
    state: &mut ProofState,
    mut handle: Clause,
    options: ResetProcessedOptions,
    evaluate: &mut E,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<(), Diagnostic>
where
    E: FnMut(&mut TermBank, &mut Clause) -> Result<(), Diagnostic>,
{
    if options.record_gc_selection {
        clause_push_derivation(&mut handle, DC_CNF_EVAL_GC, None, None);
    }
    let mut requeued = {
        let (terms, archive) = state.terms_and_archive_mut();
        clause_archive(archive, handle, terms)?
    };
    evaluate(state.terms_mut(), &mut requeued)?;
    requeued.del_prop(CP_IS_ORIENTED);
    requeued.del_prop(CP_INPUT_FORMULA);
    if let Some((output, session)) = doc_context.as_mut() {
        session.doc_clause_quote(
            output,
            state.terms(),
            6,
            &mut requeued,
            Some("move_eval"),
            None,
        )?;
    }

    if options.prefer_initial {
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
    proof_state_move_to_tmp_store_impl(state, None, false)
}

/// Moves processed clauses into `tmp_store`, deleting explicitly supplied global-index
/// entries before each original clause moves out of its processed set.
#[must_use]
pub fn proof_state_move_to_tmp_store_with_global_indices(
    state: &mut ProofState,
    control: &ProofControl,
    indices: &mut GlobalIndices,
) -> i64 {
    proof_state_move_to_tmp_store_impl(state, Some(indices), control.heuristic_parms().lambda_demod)
}

fn proof_state_move_to_tmp_store_impl(
    state: &mut ProofState,
    mut indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
) -> i64 {
    let mut moved = 0;
    moved += proof_state_move_processed_set_to_tmp_by(
        state,
        ProcessedSetSlot::PosRules,
        indices.as_deref_mut(),
        lambda_demod,
    );
    moved += proof_state_move_processed_set_to_tmp_by(
        state,
        ProcessedSetSlot::PosEqns,
        indices.as_deref_mut(),
        lambda_demod,
    );
    moved += proof_state_move_processed_set_to_tmp_by(
        state,
        ProcessedSetSlot::NegUnits,
        indices.as_deref_mut(),
        lambda_demod,
    );
    moved += proof_state_move_processed_set_to_tmp_by(
        state,
        ProcessedSetSlot::NonUnits,
        indices,
        lambda_demod,
    );
    moved
}

fn proof_state_move_processed_set_to_tmp_by(
    state: &mut ProofState,
    slot: ProcessedSetSlot,
    mut indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
) -> i64 {
    let mut moved = 0;
    while !processed_set_by_slot(state, slot).is_empty() {
        if let Some(indices) = indices.as_deref_mut() {
            proof_state_delete_first_global_indexed_clause_from_slot(
                state,
                slot,
                indices,
                lambda_demod,
            );
        }
        let Some(mut handle) = processed_set_mut_by_slot(state, slot).extract_first() else {
            continue;
        };
        handle.del_prop(CP_IS_ORIENTED);
        state.tmp_store_mut().insert(handle);
        moved += 1;
    }
    moved
}

/// Applies the currently ported modifying forward-inference prefix from C
/// `ForwardModifyClause`.
///
/// This covers the currently ported mutation path: demodulation by the processed
/// positive-unit demodulator sets, superfluous literal removal, optional
/// AC-resolved literal cleanup, optional local rewriting, literal orientation,
/// optional condensation, triviality detection, and positive/negative
/// simplify-reflect against processed unit sets. Higher-order runs preserve the
/// optimized C executable's explicit-ordering surface, including classic KBO,
/// legacy LPO/copy, LPO4/copy, and KBO6.
///
/// # Errors
///
/// Returns a diagnostic if proof-control ordering is missing, if a lower-level
/// term operation fails, or if the current higher-order problem requests a
/// non-empty ordering outside C's concrete ordering surface.
pub fn proof_state_forward_modify_clause(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
    _context_sr: bool,
    condense_clause: bool,
    level: RewriteLevel,
) -> Result<bool, Diagnostic> {
    proof_state_forward_modify_clause_impl::<String>(
        state,
        control,
        clause,
        condense_clause,
        level,
        problem_type(),
        None,
    )
}

/// Applies C `ForwardModifyClause` while emitting represented proof docs.
///
/// This wires represented rewrite, minimization, condensation, and
/// simplify-reflect modification documentation through an explicit session.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_forward_modify_clause`], plus
/// any proof-documentation write diagnostic.
#[expect(
    clippy::too_many_arguments,
    reason = "Proof-control proof-documentation wrapper keeps output/session state explicit"
)]
pub fn proof_state_forward_modify_clause_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
    _context_sr: bool,
    condense_clause: bool,
    level: RewriteLevel,
) -> Result<bool, Diagnostic> {
    proof_state_forward_modify_clause_impl(
        state,
        control,
        clause,
        condense_clause,
        level,
        problem_type(),
        Some((output, session)),
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "C-compatible ForwardModifyClause staging keeps mutation order visible"
)]
fn proof_state_forward_modify_clause_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
    condense_clause: bool,
    level: RewriteLevel,
    problem_type: ProblemType,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<bool, Diagnostic> {
    let prefer_general = control.heuristic_parms().prefer_general;
    let lambda_demod = control.heuristic_parms().lambda_demod;
    let local_rw = control.heuristic_parms().local_rw;
    let prune_args = control.heuristic_parms().prune_args;
    let higher_order = problem_type == ProblemType::HigherOrder;
    let ac_handling_active = control.ac_handling_active();
    let ac_axiom_parents = state.ac_axiom_parent_refs();
    let strong_unit_forward_subsumption = control.strong_unit_forward_subsumption();
    let ocb = control.ocb.as_mut().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ForwardModifyClause requires initialized proof-control ordering",
        )
    })?;
    let mut rw_steps = 0_i64;
    let trivial = {
        let (terms, processed_sets) = state.terms_and_processed_sets_mut();
        let demodulators: [&ClauseSet; 2] = [&*processed_sets.pos_rules, &*processed_sets.pos_eqns];
        forward_modify_check_higher_order_ordering(higher_order, ocb, clause, &demodulators)?;
        loop {
            forward_modify_normalize_if_higher_order(higher_order, clause, terms);
            let steps = match doc_context.as_mut() {
                Some((output, session)) => clause_compute_li_normalform_plain_with_docs(
                    output,
                    session,
                    terms,
                    ocb,
                    clause,
                    &demodulators,
                    level,
                    prefer_general,
                    lambda_demod,
                )?,
                None => clause_compute_li_normalform_plain(
                    terms,
                    ocb,
                    clause,
                    &demodulators,
                    level,
                    prefer_general,
                    lambda_demod,
                )?,
            };
            rw_steps += steps;
            forward_modify_normalize_if_higher_order(higher_order, clause, terms);

            let limited_rw = clause.query_prop(CP_LIMITED_RW);
            let removed_lits = clause_remove_superfluous_literals(clause, terms);
            if removed_lits != 0 {
                if let Some((output, session)) = doc_context.as_mut() {
                    session.doc_clause_modification(
                        output,
                        terms,
                        clause,
                        ClauseModificationInference::Minimize,
                        None,
                        None,
                    )?;
                }
            }

            if ac_handling_active {
                forward_modify_remove_ac_resolved(
                    terms,
                    clause,
                    &ac_axiom_parents,
                    &mut doc_context,
                )?;
            }

            if local_rw && clause_local_rw(ocb, terms, clause)? {
                forward_modify_normalize_if_higher_order(higher_order, clause, terms);
            }

            clause.orient_literals_with_bank(ocb, terms)?;

            if forward_modify_condense(terms, clause, condense_clause, &mut doc_context)? {
                clause.orient_literals_with_bank(ocb, terms)?;
            }

            if clause.is_trivial(terms) {
                break true;
            }

            if higher_order && prune_args {
                let _ = clause_prune_args(clause, terms)?;
            }
            forward_modify_normalize_if_higher_order(higher_order, clause, terms);

            forward_modify_positive_simplify_reflect(
                terms,
                processed_sets.pos_eqns,
                clause,
                strong_unit_forward_subsumption,
                &mut doc_context,
            )?;
            forward_modify_negative_simplify_reflect(
                terms,
                processed_sets.neg_units,
                clause,
                &mut doc_context,
            )?;
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

fn forward_modify_check_higher_order_ordering(
    higher_order: bool,
    ocb: &OrderControlBlock,
    _clause: &Clause,
    _demodulators: &[&ClauseSet],
) -> Result<(), Diagnostic> {
    if !higher_order || ocb.ordering_type == TermOrdering::Empty {
        return Ok(());
    }

    if matches!(
        ocb.ordering_type,
        TermOrdering::Kbo
            | TermOrdering::Kbo6
            | TermOrdering::Lpo
            | TermOrdering::LpoCopy
            | TermOrdering::Lpo4
            | TermOrdering::Lpo4Copy
    ) {
        return Ok(());
    }

    Err(Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "ForwardModifyClause higher-order term ordering is not ported yet",
    ))
}

fn forward_modify_normalize_if_higher_order(
    higher_order: bool,
    clause: &mut Clause,
    terms: &TermBank,
) {
    if higher_order {
        let _ = clause_normalize_equations(clause, terms);
    }
}

fn forward_modify_condense<W: fmt::Write>(
    terms: &mut TermBank,
    clause: &mut Clause,
    condense_clause: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<bool, Diagnostic> {
    if !condense_clause {
        return Ok(false);
    }
    match doc_context.as_mut() {
        Some((output, session)) => condense_with_docs(output, session, clause, terms),
        None => condense(clause, terms),
    }
}

fn forward_modify_remove_ac_resolved<W: fmt::Write>(
    terms: &TermBank,
    clause: &mut Clause,
    ac_axioms: &[ClauseDerivationRef],
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<(), Diagnostic> {
    match doc_context.as_mut() {
        Some((output, session)) => {
            clause_remove_ac_resolved_with_docs_and_axioms(
                output, session, clause, terms, ac_axioms,
            )?;
        }
        None => {
            clause_remove_ac_resolved(clause, terms);
        }
    }
    Ok(())
}

fn forward_modify_positive_simplify_reflect<W: fmt::Write>(
    terms: &mut TermBank,
    units: &ClauseSet,
    clause: &mut Clause,
    strong_unit_forward_subsumption: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<(), Diagnostic> {
    if clause.negative_literal_count() == 0 {
        return Ok(());
    }
    match doc_context.as_mut() {
        Some((output, session)) => {
            let _ = clause_positive_simplify_reflect_with_strong_and_docs_and_bank(
                output,
                session,
                terms,
                units,
                clause,
                strong_unit_forward_subsumption,
            )?;
        }
        None => {
            let _ = clause_positive_simplify_reflect_with_strong_and_bank(
                terms,
                units,
                clause,
                strong_unit_forward_subsumption,
            )?;
        }
    }
    Ok(())
}

fn forward_modify_negative_simplify_reflect<W: fmt::Write>(
    terms: &mut TermBank,
    units: &ClauseSet,
    clause: &mut Clause,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<(), Diagnostic> {
    if clause.positive_literal_count() == 0 {
        return Ok(());
    }
    match doc_context.as_mut() {
        Some((output, session)) => {
            let _ = clause_negative_simplify_reflect_with_docs_and_bank(
                output, session, terms, units, clause,
            )?;
        }
        None => {
            let _ = clause_negative_simplify_reflect_with_bank(terms, units, clause)?;
        }
    }
    Ok(())
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
    proof_state_forward_subsumption_with_strong(state, clause, counts, non_unit_subsumption, false)
}

/// Tries C `ForwardSubsumption` while honoring C's global
/// `StrongUnitForwardSubsumption` switch through an explicit session flag.
#[must_use]
pub fn proof_state_forward_subsumption_with_strong(
    state: &ProofState,
    clause: &mut Clause,
    counts: &mut ForwardContractCounts,
    non_unit_subsumption: bool,
    strong_unit_forward_subsumption: bool,
) -> Option<FvPackedClause> {
    clause.set_weight(clause.standard_weight());

    let mut subsumer_found = false;
    if clause.positive_literal_count() != 0 {
        subsumer_found = unit_clause_set_subsumes_clause_with_strong(
            state.processed_pos_eqns(),
            clause,
            strong_unit_forward_subsumption,
        )
        .is_some();
    }
    if !subsumer_found && clause.negative_literal_count() != 0 {
        subsumer_found =
            unit_clause_set_subsumes_clause(state.processed_neg_units(), clause).is_some();
    }
    if !subsumer_found && clause.literal_number() > 1 && non_unit_subsumption {
        clause_subsume_order_sort_lits(clause, state.terms());
        subsumer_found =
            clause_set_subsumes_clause_owned(state.processed_non_units(), clause, state.terms())
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

/// Bank-aware C `ForwardSubsumption` for proof-search paths that may contain
/// higher-order clauses.
///
/// # Errors
///
/// Returns diagnostics from complete higher-order matching or normalization.
pub fn proof_state_forward_subsumption_with_bank(
    state: &mut ProofState,
    clause: &mut Clause,
    counts: &mut ForwardContractCounts,
    non_unit_subsumption: bool,
    strong_unit_forward_subsumption: bool,
) -> Result<Option<FvPackedClause>, Diagnostic> {
    clause.set_weight(clause.standard_weight());
    let (terms, processed_sets) = state.terms_and_processed_sets_mut();

    let mut subsumer_found = false;
    if clause.positive_literal_count() != 0 {
        subsumer_found = unit_clause_set_subsumes_clause_with_bank(
            terms,
            processed_sets.pos_eqns,
            clause,
            strong_unit_forward_subsumption,
        )?
        .is_some();
    }
    if !subsumer_found && clause.negative_literal_count() != 0 {
        subsumer_found = unit_clause_set_subsumes_clause_with_bank(
            terms,
            processed_sets.neg_units,
            clause,
            false,
        )?
        .is_some();
    }
    if !subsumer_found && clause.literal_number() > 1 && non_unit_subsumption {
        clause_subsume_order_sort_lits(clause, terms);
        subsumer_found =
            clause_set_subsumes_clause_owned_with_bank(processed_sets.non_units, clause, terms)?
                .is_some();
    }

    if subsumer_found {
        counts.subsumed += 1;
        return Ok(None);
    }

    Ok(Some(fv_index_pack_clause(
        clause.clone(),
        processed_sets.non_units.fv_anchor(),
    )))
}

fn proof_state_forward_subsumption_with_control(
    state: &mut ProofState,
    control: &ProofControl,
    clause: &mut Clause,
    counts: &mut ForwardContractCounts,
    non_unit_subsumption: bool,
) -> Result<Option<FvPackedClause>, Diagnostic> {
    proof_state_forward_subsumption_with_bank(
        state,
        clause,
        counts,
        non_unit_subsumption,
        control.strong_unit_forward_subsumption(),
    )
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
    proof_state_forward_contract_keep_impl::<String>(state, control, clause, counts, options, None)
}

/// Applies C `forward_contract_keep` while emitting represented proof docs.
///
/// This currently threads the represented `ForwardModifyClause` rewrite docs
/// and contextual simplify-reflect modification docs through a shared proof-doc
/// session. Other forward-contraction proof-output side effects remain at their
/// existing renderer boundaries.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_forward_contract_keep`], plus
/// any proof-documentation write diagnostic.
pub fn proof_state_forward_contract_keep_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
    counts: &mut ForwardContractCounts,
    options: ForwardContractOptions,
) -> Result<Option<FvPackedClause>, Diagnostic> {
    proof_state_forward_contract_keep_impl(
        state,
        control,
        clause,
        counts,
        options,
        Some((output, session)),
    )
}

fn proof_state_forward_modify_clause_maybe_docs<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
    options: ForwardContractOptions,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<bool, Diagnostic> {
    match doc_context.as_mut() {
        Some((output, session)) => proof_state_forward_modify_clause_with_docs(
            output,
            session,
            state,
            control,
            clause,
            options.context_sr,
            options.condense_clause,
            options.level,
        ),
        None => proof_state_forward_modify_clause(
            state,
            control,
            clause,
            options.context_sr,
            options.condense_clause,
            options.level,
        ),
    }
}

fn proof_state_contextual_simplify_reflect_maybe_docs<W: fmt::Write>(
    state: &mut ProofState,
    clause: &mut Clause,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<usize, Diagnostic> {
    let (terms, processed_sets) = state.terms_and_processed_sets_mut();
    match doc_context.as_mut() {
        Some((output, session)) => clause_contextual_simplify_reflect_with_docs_and_bank(
            output,
            session,
            processed_sets.non_units,
            clause,
            terms,
        ),
        None => {
            clause_contextual_simplify_reflect_with_bank(processed_sets.non_units, clause, terms)
        }
    }
}

fn proof_state_forward_contract_keep_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
    counts: &mut ForwardContractCounts,
    options: ForwardContractOptions,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<Option<FvPackedClause>, Diagnostic> {
    if control.heuristic_parms().enable_given_forward_simpl {
        let forward_trivial = proof_state_forward_modify_clause_maybe_docs(
            state,
            control,
            clause,
            options,
            &mut doc_context,
        )?;
        if forward_trivial {
            counts.trivial += 1;
            return Ok(None);
        }

        if clause_boolean_simplification(clause, state.terms_mut())? {
            counts.trivial += 1;
            return Ok(None);
        }

        if clause.is_empty() {
            return Ok(Some(fv_index_pack_clause(clause.clone(), None)));
        }
        if problem_type() == ProblemType::HigherOrder
            && clause_resolve_flex_clause(clause, state.terms())
        {
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

        if clause_is_tautology(state.tmp_terms_mut(), clause)? {
            counts.trivial += 1;
            return Ok(None);
        }

        debug_assert!(!clause.is_trivial(state.terms()));

        if problem_type() == ProblemType::HigherOrder
            && clause_eliminate_naked_boolean_variables(clause, state.terms_mut())?
        {
            counts.trivial += 1;
            return Ok(None);
        }

        if proof_state_forward_subsumption_with_control(
            state,
            control,
            clause,
            counts,
            options.non_unit_subsumption,
        )?
        .is_none()
        {
            return Ok(None);
        }

        if options.context_sr && clause.literal_number() > 1 {
            let simplified = proof_state_contextual_simplify_reflect_maybe_docs(
                state,
                clause,
                &mut doc_context,
            )?;
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
    {
        let terms = state.terms_mut();
        do_literal_selection_with_bank(control, terms, clause)
            .map_err(literal_selection_error_to_diagnostic)?;
    }
    let ocb = control.ocb.as_mut().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "forward_contract_keep requires initialized proof-control ordering",
        )
    })?;
    {
        let terms = state.terms_mut();
        clause.cond_mark_maximal_terms_with_bank(ocb, terms)?;
    }

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

/// Applies C `ForwardContractClause` while emitting represented proof docs.
///
/// # Errors
///
/// Returns diagnostics from [`proof_state_forward_contract_keep_with_docs`].
pub fn proof_state_forward_contract_clause_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    mut clause: Clause,
    options: ForwardContractOptions,
) -> Result<Option<FvPackedClause>, Diagnostic> {
    let mut counts = ForwardContractCounts::default();
    let result = proof_state_forward_contract_keep_with_docs(
        output,
        session,
        state,
        control,
        &mut clause,
        &mut counts,
        options,
    )?;
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
    proof_state_forward_contract_set_impl::<String>(
        state,
        control,
        set,
        non_unit_subsumption,
        level,
        count_eliminated,
        terminate_on_empty,
        None,
    )
}

/// Applies C `ForwardContractSet` while emitting represented proof docs.
///
/// # Errors
///
/// Returns diagnostics from [`proof_state_forward_contract_keep_with_docs`].
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible set contraction keeps the proof-documentation session explicit"
)]
pub fn proof_state_forward_contract_set_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    non_unit_subsumption: bool,
    level: RewriteLevel,
    count_eliminated: &mut u64,
    terminate_on_empty: bool,
) -> Result<Option<Clause>, Diagnostic> {
    proof_state_forward_contract_set_impl(
        state,
        control,
        set,
        non_unit_subsumption,
        level,
        count_eliminated,
        terminate_on_empty,
        Some((output, session)),
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible set contraction keeps control flags and optional docs together"
)]
fn proof_state_forward_contract_set_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    non_unit_subsumption: bool,
    level: RewriteLevel,
    count_eliminated: &mut u64,
    terminate_on_empty: bool,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
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
        let contracted_result = match doc_context.as_mut() {
            Some((output, session)) => proof_state_forward_contract_keep_with_docs(
                output,
                session,
                state,
                control,
                &mut clause,
                &mut counts,
                options,
            ),
            None => {
                proof_state_forward_contract_keep(state, control, &mut clause, &mut counts, options)
            }
        };
        let contracted = match contracted_result {
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

/// Re-evaluates every clause in a set with mutable term-bank ordering context.
///
/// This is the bank-backed counterpart of C `ClauseSetReweight` used by
/// proof-control paths that may run higher-order ordering-aware WFCBs.
///
/// # Errors
///
/// Returns a diagnostic if proof-control has no active HCB, no initialized
/// ordering control block, or if bank-backed ordering preparation fails.
pub fn proof_control_clause_set_reweight_with_bank(
    control: &mut ProofControl,
    terms: &mut TermBank,
    set: &mut ClauseSet,
) -> Result<(), Diagnostic> {
    let active_hcb_handle = control.active_hcb.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ClauseSetReweight requires initialized proof-control heuristic",
        )
    })?;
    let ProofControl {
        hcbs, wfcbs, ocb, ..
    } = control;
    let active_hcb = hcbs
        .hcb(active_hcb_handle)
        .ok_or_else(|| unknown_heuristic_handle("active"))?;
    let ocb = ocb.as_mut().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ClauseSetReweight requires initialized proof-control ordering",
        )
    })?;
    hcb_clause_set_reweight_with_bank(active_hcb, wfcbs, ocb, terms, set)
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
    proof_control_clause_set_reweight_with_bank(control, state.terms_mut(), set)?;
    Ok(None)
}

/// Applies C `ForwardContractSetReweight` while emitting represented proof docs.
///
/// # Errors
///
/// Returns diagnostics from documenting set contraction or HCB reweighting.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible set contraction keeps the proof-documentation session explicit"
)]
pub fn proof_state_forward_contract_set_reweight_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    non_unit_subsumption: bool,
    level: RewriteLevel,
    count_eliminated: &mut u64,
) -> Result<Option<Clause>, Diagnostic> {
    let empty = proof_state_forward_contract_set_with_docs(
        output,
        session,
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
    proof_control_clause_set_reweight_with_bank(control, state.terms_mut(), set)?;
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

/// Bank-backed counterpart of [`proof_control_clause_set_filter_reweigth`].
///
/// # Errors
///
/// Returns a diagnostic if HCB reweighting or ordering is not initialized, or
/// if bank-backed ordering preparation fails.
pub fn proof_control_clause_set_filter_reweigth_with_bank(
    control: &mut ProofControl,
    terms: &mut TermBank,
    set: &mut ClauseSet,
    count_eliminated: &mut u64,
) -> Result<(), Diagnostic> {
    *count_eliminated += i64_to_u64_saturating(set.filter_trivial(terms));
    proof_control_clause_set_reweight_with_bank(control, terms, set)
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

/// Correctly spelled alias for
/// [`proof_control_clause_set_filter_reweigth_with_bank`].
///
/// # Errors
///
/// Returns a diagnostic if HCB reweighting or ordering is not initialized, or
/// if bank-backed ordering preparation fails.
pub fn proof_control_clause_set_filter_reweight_with_bank(
    control: &mut ProofControl,
    terms: &mut TermBank,
    set: &mut ClauseSet,
    count_eliminated: &mut u64,
) -> Result<(), Diagnostic> {
    proof_control_clause_set_filter_reweigth_with_bank(control, terms, set, count_eliminated)
}

/// Returns a Rust-side estimate for C `ProofStateStorage`.
///
/// This keeps the same selected proof-state domains as C and uses the
/// constant-memory `ClauseSetStorage`/`TBStorage` estimates exposed by the
/// Rust owners.
#[must_use]
pub fn proof_state_storage_estimate(state: &ProofState) -> i64 {
    [
        state.unprocessed().storage_estimate(),
        state.processed_pos_rules().storage_estimate(),
        state.processed_pos_eqns().storage_estimate(),
        state.processed_neg_units().storage_estimate(),
        state.processed_non_units().storage_estimate(),
        state.archive().storage_estimate(),
        state.terms().storage_estimate(),
    ]
    .into_iter()
    .fold(0_i64, i64::saturating_add)
}

#[derive(Clone, Debug, Default)]
struct ClauseRefHasher(u64);

impl Hasher for ClauseRefHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0_u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.write_u64(u64::from_ne_bytes(word));
        }
    }

    fn write_u64(&mut self, value: u64) {
        const FX_SEED: u64 = 0x517c_c1b7_2722_0a95;
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(FX_SEED);
    }

    fn write_i64(&mut self, value: i64) {
        self.write_u64(u64::from_ne_bytes(value.to_ne_bytes()));
    }
}

type ClauseRefSet = HashSet<ClauseDerivationRef, BuildHasherDefault<ClauseRefHasher>>;

#[derive(Clone, Debug, Default)]
struct ParentLivenessSnapshot {
    live: ClauseRefSet,
}

impl ParentLivenessSnapshot {
    fn from_state(state: &ProofState) -> Self {
        let capacity = state.axioms().len()
            + state.ax_archive().len()
            + state.processed_pos_rules().len()
            + state.processed_pos_eqns().len()
            + state.processed_neg_units().len()
            + state.processed_non_units().len()
            + state.unprocessed().len()
            + state.tmp_store().len()
            + state.eval_store().len()
            + state.archive().len()
            + state.definition_store().len()
            + state.watchlist().map_or(0, ClauseSet::len);
        let mut snapshot = Self {
            live: ClauseRefSet::with_capacity_and_hasher(capacity, BuildHasherDefault::default()),
        };
        snapshot.collect_set(state.axioms());
        snapshot.collect_set(state.ax_archive());
        snapshot.collect_set(state.processed_pos_rules());
        snapshot.collect_set(state.processed_pos_eqns());
        snapshot.collect_set(state.processed_neg_units());
        snapshot.collect_set(state.processed_non_units());
        snapshot.collect_set(state.unprocessed());
        snapshot.collect_set(state.tmp_store());
        snapshot.collect_set(state.eval_store());
        snapshot.collect_set(state.archive());
        snapshot.collect_set(state.definition_store());
        if let Some(watchlist) = state.watchlist() {
            snapshot.collect_set(watchlist);
        }
        snapshot
    }

    fn collect_set(&mut self, set: &ClauseSet) {
        for clause in set.iter() {
            if !clause.query_prop(CP_IS_DEAD) {
                self.live.insert(ClauseDerivationRef::from(clause));
            }
        }
    }

    fn parent_is_dead(&self, parent: DerivationParentRef) -> bool {
        match parent {
            DerivationParentRef::Clause(parent) => !self.live.contains(&parent),
            DerivationParentRef::Demodulator(_) | DerivationParentRef::Formula(_) => false,
        }
    }
}

fn selection_parent_is_dead(state: &ProofState, parent: DerivationParentRef) -> bool {
    let DerivationParentRef::Clause(parent) = parent else {
        return false;
    };
    let is_live_parent = |clause: &Clause| {
        !clause.query_prop(CP_IS_DEAD) && ClauseDerivationRef::from(clause) == parent
    };
    let indexed_parent_sets = [
        state.processed_pos_rules(),
        state.processed_pos_eqns(),
        state.processed_neg_units(),
        state.processed_non_units(),
    ];
    if indexed_parent_sets.into_iter().any(|set| {
        set.find_indexed_by_id(parent.ident())
            .is_some_and(|clause| is_live_parent(clause) || set.iter().any(&is_live_parent))
    }) {
        return false;
    }
    let stable_unindexed_parent_sets = [
        state.axioms(),
        state.ax_archive(),
        state.archive(),
        state.definition_store(),
    ];
    !stable_unindexed_parent_sets
        .into_iter()
        .chain(state.watchlist())
        .any(|set| set.iter().any(&is_live_parent))
}

/// Applies the currently ported local effects of C
/// `cleanup_unprocessed_clauses`.
///
/// This preserves the C gate order: orphan deletion, special forward
/// contraction/reweighting, then delete-bad under the storage limit. The
/// orphan check is supplied by the caller for tests and alternate owners; the
/// default proof-state wrapper supplies a compact parent-liveness snapshot.
///
/// # Errors
///
/// Returns diagnostics from forward contraction, HCB reweighting, or missing
/// active-HCB state in the delete-bad branch.
pub fn proof_state_cleanup_unprocessed_clauses_with(
    state: &mut ProofState,
    control: &mut ProofControl,
    current_storage: i64,
    mut parent_is_dead: impl FnMut(DerivationParentRef) -> bool,
) -> Result<CleanupUnprocessedOutcome, Diagnostic> {
    proof_state_cleanup_unprocessed_clauses_impl::<String>(
        state,
        control,
        |_| current_storage,
        |state| clause_set_delete_orphans_with(state.unprocessed_mut(), &mut parent_is_dead),
        None,
    )
}

fn proof_state_cleanup_unprocessed_clauses_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    mut current_storage: impl FnMut(&ProofState) -> i64,
    mut delete_orphans: impl FnMut(&mut ProofState) -> i64,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<CleanupUnprocessedOutcome, Diagnostic> {
    let mut outcome = CleanupUnprocessedOutcome::default();
    let back_simplified = state
        .statistics()
        .backward_subsumed_count
        .saturating_add(state.statistics().backward_rewritten_count);
    let orphan_delta = back_simplified.saturating_sub(state.statistics().filter_orphans_base);

    if unsigned_delta_exceeds_limit(orphan_delta, control.heuristic_parms().filter_orphans_limit) {
        let deleted = delete_orphans(state);
        outcome.orphan_cleanup_triggered = true;
        outcome.orphan_cleanup_deleted = deleted;
        outcome.orphan_cleanup_remaining = state.unprocessed().members();
        outcome.orphaned_deleted += deleted;
        state.statistics_mut().other_redundant_count += i64_to_u64_saturating(deleted);
        state.statistics_mut().filter_orphans_base = back_simplified;
    }

    let processed_delta = state
        .statistics()
        .processed_count
        .saturating_sub(state.statistics().forward_contract_base);
    if unsigned_delta_exceeds_limit(
        processed_delta,
        control.heuristic_parms().forward_contract_limit,
    ) {
        let mut count_eliminated = 0;
        let mut unprocessed = std::mem::take(state.unprocessed_mut());
        let contract_result = match doc_context.as_mut() {
            Some((output, session)) => proof_state_forward_contract_set_with_docs(
                output,
                session,
                state,
                control,
                &mut unprocessed,
                false,
                RewriteLevel::FullRewrite,
                &mut count_eliminated,
                true,
            ),
            None => proof_state_forward_contract_set(
                state,
                control,
                &mut unprocessed,
                false,
                RewriteLevel::FullRewrite,
                &mut count_eliminated,
                true,
            ),
        };
        let unsatisfiable = match contract_result {
            Ok(unsatisfiable) => unsatisfiable,
            Err(err) => {
                *state.unprocessed_mut() = unprocessed;
                return Err(err);
            }
        };
        *state.unprocessed_mut() = unprocessed;
        outcome.forward_contract_triggered = true;
        outcome.forward_contract_deleted = count_eliminated;
        outcome.forward_contract_remaining = state.unprocessed().members();
        state.statistics_mut().other_redundant_count += count_eliminated;

        if let Some(empty) = unsatisfiable {
            outcome.unsatisfiable = Some(empty);
            return Ok(outcome);
        }

        let processed_count = state.statistics().processed_count;
        state.statistics_mut().forward_contract_base = processed_count;
        let mut unprocessed = std::mem::take(state.unprocessed_mut());
        let reweight_result = proof_control_clause_set_reweight_with_bank(
            control,
            state.terms_mut(),
            &mut unprocessed,
        );
        *state.unprocessed_mut() = unprocessed;
        reweight_result?;
    }

    if current_storage(state) > control.heuristic_parms().delete_bad_limit {
        let target_size = state.unprocessed().members() / 2;
        let orphan_count = delete_orphans(state);
        outcome.delete_bad_triggered = true;
        outcome.delete_bad_orphaned_deleted = orphan_count;
        outcome.orphaned_deleted += orphan_count;
        state.statistics_mut().non_redundant_deleted += i64_to_u64_saturating(orphan_count);

        let active_hcb_handle = control.active_hcb.ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "cleanup_unprocessed_clauses delete-bad requires initialized proof-control heuristic",
            )
        })?;
        let active_hcb = control
            .hcbs
            .hcb(active_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("active"))?;
        let bad_deleted =
            hcb_clause_set_delete_bad_clauses(active_hcb, state.unprocessed_mut(), target_size);
        outcome.bad_deleted = bad_deleted;
        if bad_deleted != 0 {
            state.set_state_is_complete(false);
        }
        outcome.term_gc_recovered = state.collect_term_garbage();
    }

    Ok(outcome)
}

/// Applies [`proof_state_cleanup_unprocessed_clauses_with`] using the current
/// storage estimate and a proof-state snapshot of compact parent liveness.
///
/// # Errors
///
/// Returns diagnostics from the underlying cleanup helper.
pub fn proof_state_cleanup_unprocessed_clauses(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<CleanupUnprocessedOutcome, Diagnostic> {
    proof_state_cleanup_unprocessed_clauses_impl::<String>(
        state,
        control,
        proof_state_storage_estimate,
        |state| {
            let parent_liveness = ParentLivenessSnapshot::from_state(state);
            clause_set_delete_orphans_with(state.unprocessed_mut(), |parent| {
                parent_liveness.parent_is_dead(parent)
            })
        },
        None,
    )
}

/// Applies C `cleanup_unprocessed_clauses` while emitting represented proof docs.
///
/// # Errors
///
/// Returns diagnostics from documenting forward contraction or later cleanup stages.
pub fn proof_state_cleanup_unprocessed_clauses_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<CleanupUnprocessedOutcome, Diagnostic> {
    proof_state_cleanup_unprocessed_clauses_impl(
        state,
        control,
        proof_state_storage_estimate,
        |state| {
            let parent_liveness = ParentLivenessSnapshot::from_state(state);
            clause_set_delete_orphans_with(state.unprocessed_mut(), |parent| {
                parent_liveness.parent_is_dead(parent)
            })
        },
        Some((output, session)),
    )
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
    proof_state_filter_unprocessed_impl::<String>(state, control, desc, None)
}

/// Applies C `ProofStateFilterUnprocessed` while emitting represented proof docs.
///
/// # Errors
///
/// Returns diagnostics from documenting forward contraction.
pub fn proof_state_filter_unprocessed_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    desc: &str,
) -> Result<Option<Clause>, Diagnostic> {
    proof_state_filter_unprocessed_impl(state, control, desc, Some((output, session)))
}

fn proof_state_filter_unprocessed_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    desc: &str,
    doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<Option<Clause>, Diagnostic> {
    let mut unprocessed = std::mem::take(state.unprocessed_mut());
    let result = proof_state_filter_unprocessed_set_impl(
        state,
        control,
        &mut unprocessed,
        desc,
        doc_context,
    );
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
    proof_state_filter_unprocessed_set_impl::<String>(state, control, set, desc, None)
}

fn proof_state_filter_unprocessed_set_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    desc: &str,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
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
                &mut doc_context,
            )?,
            b'N' => proof_state_filter_contract_step(
                state,
                control,
                set,
                true,
                RewriteLevel::NoRewrite,
                &mut doc_context,
            )?,
            b'r' => proof_state_filter_contract_step(
                state,
                control,
                set,
                false,
                RewriteLevel::RuleRewrite,
                &mut doc_context,
            )?,
            b'R' => proof_state_filter_contract_step(
                state,
                control,
                set,
                true,
                RewriteLevel::RuleRewrite,
                &mut doc_context,
            )?,
            b'f' => proof_state_filter_contract_step(
                state,
                control,
                set,
                false,
                RewriteLevel::FullRewrite,
                &mut doc_context,
            )?,
            b'F' => proof_state_filter_contract_step(
                state,
                control,
                set,
                true,
                RewriteLevel::FullRewrite,
                &mut doc_context,
            )?,
            _ => None,
        };
        if empty.is_some() {
            return Ok(empty);
        }
    }
    Ok(None)
}

fn proof_state_filter_contract_step<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    set: &mut ClauseSet,
    non_unit_subsumption: bool,
    level: RewriteLevel,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<Option<Clause>, Diagnostic> {
    let mut count = 0;
    let empty = match doc_context.as_mut() {
        Some((output, session)) => proof_state_forward_contract_set_with_docs(
            output,
            session,
            state,
            control,
            set,
            non_unit_subsumption,
            level,
            &mut count,
            true,
        )?,
        None => proof_state_forward_contract_set(
            state,
            control,
            set,
            non_unit_subsumption,
            level,
            &mut count,
            true,
        )?,
    };
    state.statistics_mut().proc_trivial_count += count;
    Ok(empty)
}

fn i64_to_u64_saturating(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

fn unsigned_delta_exceeds_limit(delta: u64, limit: i64) -> bool {
    limit >= 0 && delta > u64::try_from(limit).unwrap_or(u64::MAX)
}

fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
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
/// Returns a diagnostic if the configured literal-selection strategy needs
/// ordering context that has not been initialized.
pub fn proof_state_queue_generated_clause_for_eval(
    state: &mut ProofState,
    control: &mut ProofControl,
    mut clause: Clause,
) -> Result<(), Diagnostic> {
    clause.del_prop(CP_IS_ORIENTED);
    if control.heuristic_parms().select_on_proc_only {
        clause.literals_mut().del_prop(EP_IS_SELECTED);
    } else {
        let terms = state.terms_mut();
        do_literal_selection_with_bank(control, terms, &mut clause)
            .map_err(literal_selection_error_to_diagnostic)?;
    }
    let create_date = i64::try_from(state.statistics().proc_non_trivial_count).unwrap_or(i64::MAX);
    clause.set_create_date(create_date);
    state.statistics_mut().non_trivial_generated_count += 1;
    if control.record_gc_selection() {
        clause_push_derivation(&mut clause, DC_CNF_EVAL_GC, None, None);
    }
    state.eval_store_mut().insert(clause);
    Ok(())
}

fn proof_state_aggressive_forward_subsumed(
    state: &mut ProofState,
    control: &ProofControl,
    clause: &mut Clause,
) -> Result<bool, Diagnostic> {
    if !control.heuristic_parms().forward_subsumption_aggressive {
        return Ok(false);
    }

    let mut counts = ForwardContractCounts::default();
    let subsumed =
        proof_state_forward_subsumption_with_control(state, control, clause, &mut counts, true)?
            .is_none();
    state.statistics_mut().aggressive_forward_subsumed_count += counts.subsumed;
    Ok(subsumed)
}

fn proof_state_clause_er_normalize_var_maybe_docs<W: fmt::Write>(
    state: &mut ProofState,
    clause: Clause,
    strong: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<(Clause, i64), Diagnostic> {
    let fresh_vars = state.fresh_vars().clone();
    if let Some((output, session)) = doc_context.as_mut() {
        clause_er_normalize_var_with_fresh_vars_and_docs(
            &mut **output,
            session,
            state.terms_mut(),
            clause,
            strong,
            &fresh_vars,
        )
    } else {
        clause_er_normalize_var_with_fresh_vars(state.terms_mut(), clause, strong, &fresh_vars)
    }
}

/// Drains `tmp_store` through the currently ported local body of C
/// `insert_new_clauses`.
///
/// This covers generated counters, modifying forward contraction, watchlist
/// checks, empty-clause return, aggressive forward subsumption, eval-store
/// admission, HCB evaluation, and the final move to `unprocessed`. Destructive
/// equality resolution is available for the first-order destructive
/// variable-literal path, with opt-in proof-documentation output, and controlled
/// clause splitting is available for fresh definitions plus clause-level reused
/// definitions. Arity-zero split-definition formula archive parents are
/// represented on split clauses so opt-in wrappers can emit split proof docs;
/// executable-wide proof-documentation session ownership remains separate.
///
/// # Errors
///
/// Returns a diagnostic from forward contraction, literal selection, HCB
/// evaluation, or split-definition lookup/allocation.
pub fn proof_state_insert_new_clauses(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<Option<Clause>, Diagnostic> {
    proof_state_insert_new_clauses_impl::<String>(state, control, None, None)
}

/// Drains `tmp_store` through C `insert_new_clauses` while emitting represented
/// proof-documentation steps for the already-ported generated-clause branches.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_insert_new_clauses`], plus any
/// proof-documentation write diagnostic.
pub fn proof_state_insert_new_clauses_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<Option<Clause>, Diagnostic> {
    proof_state_insert_new_clauses_impl(state, control, Some((output, session)), None)
}

/// Drains `tmp_store` through C `insert_new_clauses` while rendering only C's
/// `OutputLevel` text.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_insert_new_clauses`], plus any
/// output diagnostic from dynamic watchlist-reduction rendering.
pub fn proof_state_insert_new_clauses_with_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<Option<Clause>, Diagnostic> {
    let output = output as &mut dyn std::io::Write;
    proof_state_insert_new_clauses_impl::<String>(
        state,
        control,
        None,
        Some((output, output_level)),
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "C-compatible generated-clause admission keeps the mutation gates in source order"
)]
fn proof_state_insert_new_clauses_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
    mut output_context: Option<(&mut dyn std::io::Write, i64)>,
) -> Result<Option<Clause>, Diagnostic> {
    proof_state_record_tmp_store_generated_snapshot(state);

    while let Some(mut clause) = state.tmp_store_mut().extract_first() {
        let context_sr = control.heuristic_parms().forward_context_sr_aggressive
            || (control.heuristic_parms().backward_context_sr
                && clause.query_prop(CP_IS_PROCESSED));
        let condense = control.heuristic_parms().condensing_aggressive;

        if clause.query_prop(CP_IS_IR_VICTIM) {
            debug_assert!(clause.query_prop(CP_LIMITED_RW));
            let _ = proof_state_forward_modify_clause_maybe_docs(
                state,
                control,
                &mut clause,
                insert_new_forward_modify_options(context_sr, condense, RewriteLevel::FullRewrite),
                &mut doc_context,
            )?;
            clause.del_prop(CP_IS_IR_VICTIM);
        }

        let trivial = proof_state_forward_modify_clause_maybe_docs(
            state,
            control,
            &mut clause,
            insert_new_forward_modify_options(
                context_sr,
                condense,
                control.heuristic_parms().forward_demod,
            ),
            &mut doc_context,
        )?;
        if trivial || clause.is_trivial(state.terms()) {
            continue;
        }

        let static_watchlist = control.heuristic_parms().watchlist_is_static;
        let lambda_demod = control.heuristic_parms().lambda_demod;
        let _ = proof_state_check_watchlist_maybe_output(
            state,
            &mut clause,
            static_watchlist,
            lambda_demod,
            None,
            &mut doc_context,
            output_context.as_mut(),
        )?;
        if clause.is_empty() {
            return Ok(Some(clause));
        }

        if proof_state_aggressive_forward_subsumed(state, control, &mut clause)? {
            continue;
        }

        if control.heuristic_parms().er_aggressive
            && control.heuristic_parms().er_varlit_destructive
        {
            let strong = control.heuristic_parms().er_strong_destructive;
            let (normalized, clause_count) = proof_state_clause_er_normalize_var_maybe_docs(
                state,
                clause,
                strong,
                &mut doc_context,
            )?;
            clause = normalized;
            if clause_count != 0 {
                let count = i64_to_u64_saturating(clause_count);
                let statistics = state.statistics_mut();
                statistics.other_redundant_count += count;
                statistics.resolv_count += count;
                statistics.generated_count += count;
                state.tmp_store_mut().insert(clause);
                continue;
            }
        }

        if control.heuristic_parms().split_aggressive
            && controlled_split_class_matches(&clause, control.heuristic_parms().split_clauses)
        {
            let split_method = clause_split_method(control.heuristic_parms().split_method);
            match proof_state_split_clause(
                state,
                clause,
                split_method,
                control.heuristic_parms().split_fresh_defs,
            )? {
                ClauseSplitOutcome::Unsplit(unsplit) => {
                    clause = *unsplit;
                }
                ClauseSplitOutcome::Split(mut clauses, split_count) => {
                    if let Some((output, session)) = doc_context.as_mut() {
                        proof_state_document_split_clauses(
                            &mut **output,
                            session,
                            state,
                            &mut clauses,
                        )?;
                    }
                    let count = usize_to_u64_saturating(split_count);
                    for split_clause in clauses {
                        state.tmp_store_mut().insert(split_clause);
                    }
                    state.statistics_mut().generated_count += count;
                    continue;
                }
            }
        }

        proof_state_queue_generated_clause_for_eval(state, control, clause)?;
    }

    proof_state_eval_clause_set(state, control)?;
    if let Some((output, session)) = doc_context.as_mut() {
        proof_state_move_eval_store_to_unprocessed_with_docs(output, session, state)?;
    } else {
        let _ = proof_state_move_eval_store_to_unprocessed(state);
    }
    Ok(None)
}

fn proof_state_record_tmp_store_generated_snapshot(state: &mut ProofState) {
    let generated_count = i64_to_u64_saturating(state.tmp_store().members());
    let generated_lit_count = i64_to_u64_saturating(state.tmp_store().literals());
    let statistics = state.statistics_mut();
    statistics.generated_count += generated_count;
    statistics.generated_lit_count += generated_lit_count;
}

fn insert_new_forward_modify_options(
    context_sr: bool,
    condense_clause: bool,
    level: RewriteLevel,
) -> ForwardContractOptions {
    ForwardContractOptions {
        non_unit_subsumption: false,
        context_sr,
        condense_clause,
        level,
    }
}

/// Applies C `replacing_inferences` to one already packed selected clause.
///
/// The current port covers the higher-order immediate-clausification branch,
/// the first-order destructive equality-resolution branch, and fresh/reused
/// controlled-splitting branches. If a branch replaces the selected clause, the
/// produced clauses are routed through [`proof_state_insert_new_clauses`]
/// immediately, matching the C helper.
///
/// # Errors
///
/// Returns diagnostics from immediate clausification and its formula proof
/// documentation, destructive equality resolution, controlled splitting, or
/// generated-clause insertion.
pub fn proof_state_replacing_inferences(
    state: &mut ProofState,
    control: &mut ProofControl,
    packed: FvPackedClause,
) -> Result<ReplacingInferenceOutcome, Diagnostic> {
    proof_state_replacing_inferences_impl::<String>(state, control, packed, None)
}

/// Applies C `replacing_inferences` while emitting represented proof
/// documentation for already-ported replacement branches.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_replacing_inferences`], plus
/// any proof-documentation write diagnostic.
pub fn proof_state_replacing_inferences_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    packed: FvPackedClause,
) -> Result<ReplacingInferenceOutcome, Diagnostic> {
    proof_state_replacing_inferences_impl(state, control, packed, Some((output, session)))
}

fn proof_state_replacing_inferences_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    packed: FvPackedClause,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<ReplacingInferenceOutcome, Diagnostic> {
    let mut clause = packed.into_clause();

    if problem_type() == ProblemType::HigherOrder
        && clause_needs_immediate_clausification(&clause, state.terms())
    {
        let empty = if let Some((output, session)) = doc_context.as_mut() {
            proof_state_immediate_clausification_with_docs(
                &mut **output,
                session,
                state,
                clause,
                control.heuristic_parms().fool_unroll,
            )?;
            proof_state_insert_new_clauses_with_docs(&mut **output, session, state, control)?
        } else {
            proof_state_immediate_clausification(
                state,
                clause,
                control.heuristic_parms().fool_unroll,
            )?;
            proof_state_insert_new_clauses(state, control)?
        };
        return Ok(ReplacingInferenceOutcome::Replaced { empty });
    }

    if control.heuristic_parms().er_varlit_destructive {
        let strong = control.heuristic_parms().er_strong_destructive;
        let (normalized, clause_count) = proof_state_clause_er_normalize_var_maybe_docs(
            state,
            clause,
            strong,
            &mut doc_context,
        )?;
        clause = normalized;
        if clause_count != 0 {
            let count = i64_to_u64_saturating(clause_count);
            let statistics = state.statistics_mut();
            statistics.other_redundant_count += count;
            statistics.resolv_count += count;
            state.tmp_store_mut().insert(clause);
            let empty = if let Some((output, session)) = doc_context.as_mut() {
                proof_state_insert_new_clauses_with_docs(&mut **output, session, state, control)?
            } else {
                proof_state_insert_new_clauses(state, control)?
            };
            return Ok(ReplacingInferenceOutcome::Replaced { empty });
        }
    }

    let split_class = control.heuristic_parms().split_clauses;
    if controlled_split_class_matches(&clause, split_class) {
        let split_method = clause_split_method(control.heuristic_parms().split_method);
        match proof_state_split_clause(
            state,
            clause,
            split_method,
            control.heuristic_parms().split_fresh_defs,
        )? {
            ClauseSplitOutcome::Unsplit(unsplit) => {
                clause = *unsplit;
            }
            ClauseSplitOutcome::Split(mut clauses, _) => {
                if let Some((output, session)) = doc_context.as_mut() {
                    proof_state_document_split_clauses(
                        &mut **output,
                        session,
                        state,
                        &mut clauses,
                    )?;
                }
                for split_clause in clauses {
                    state.tmp_store_mut().insert(split_clause);
                }
                let empty = if let Some((output, session)) = doc_context.as_mut() {
                    proof_state_insert_new_clauses_with_docs(
                        &mut **output,
                        session,
                        state,
                        control,
                    )?
                } else {
                    proof_state_insert_new_clauses(state, control)?
                };
                return Ok(ReplacingInferenceOutcome::Replaced { empty });
            }
        }
    }

    Ok(ReplacingInferenceOutcome::Survivor(clause))
}

fn clause_needs_immediate_clausification(clause: &Clause, terms: &TermBank) -> bool {
    clause
        .literals()
        .as_slice()
        .iter()
        .any(|literal| literal.is_clausifiable(terms))
}

fn proof_state_immediate_clausification(
    state: &mut ProofState,
    clause: Clause,
    fool_unroll: bool,
) -> Result<(), Diagnostic> {
    proof_state_immediate_clausification_impl::<String>(state, clause, fool_unroll, None)
}

fn proof_state_immediate_clausification_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    clause: Clause,
    fool_unroll: bool,
) -> Result<(), Diagnostic> {
    proof_state_immediate_clausification_impl(state, clause, fool_unroll, Some((output, session)))
}

fn proof_state_immediate_clausification_impl<W: fmt::Write>(
    state: &mut ProofState,
    clause: Clause,
    fool_unroll: bool,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<(), Diagnostic> {
    state.terms().vars().set_v_counts_to_used();

    let wrapped = WrappedFormula::of_clause(state.terms_mut(), &clause, ProblemType::HigherOrder)?;
    let mut work_set = FormulaSet::new();
    work_set.insert(wrapped);
    let mut formula_archive = FormulaSet::new();

    if fool_unroll {
        work_set.unroll_fool(state.terms_mut())?;
    }
    if let Some((output, session)) = doc_context.as_mut() {
        let full_terms = session.step_options.full_terms;
        work_set.simplify_with_docs(
            &mut **output,
            state.terms_mut(),
            session,
            full_terms,
            ProblemType::HigherOrder,
        )?;
        work_set.introduce_defs_with_docs(
            &mut formula_archive,
            &mut **output,
            state.terms_mut(),
            session,
            FormulaProofDocRenderOptions::new(full_terms, ProblemType::HigherOrder),
            IMMEDIATE_CLAUSIFICATION_RENAMING_LIMIT,
        )?;
    } else {
        work_set.simplify(state.terms_mut())?;
        work_set.introduce_defs(
            &mut formula_archive,
            state.terms_mut(),
            IMMEDIATE_CLAUSIFICATION_RENAMING_LIMIT,
        )?;
    }

    let mut results = ClauseSet::new();
    let fresh_vars = state.fresh_vars().clone();
    while let Some(mut formula) = work_set.extract_first() {
        if let Some((output, session)) = doc_context.as_mut() {
            let render_options = FormulaProofDocRenderOptions::new(
                session.step_options.full_terms,
                ProblemType::HigherOrder,
            );
            let mut context =
                WrappedFormulaCnfDocContext::new(&mut **output, session, render_options);
            formula.cnf2_into_with_docs(
                &mut context,
                state.terms_mut(),
                &mut results,
                &fresh_vars,
                IMMEDIATE_CLAUSIFICATION_MINISCOPE_LIMIT,
                fool_unroll,
            )?;
        } else {
            formula.cnf2_into(
                state.terms_mut(),
                &mut results,
                &fresh_vars,
                IMMEDIATE_CLAUSIFICATION_MINISCOPE_LIMIT,
                fool_unroll,
                ProblemType::HigherOrder,
            )?;
        }
    }

    while let Some(mut result) = results.extract_first() {
        lambda_normalize_clause_terms(state.terms_mut(), &mut result)?;
        result.set_derivation(None);
        set_ho_generation_proof_object(&mut result, &clause, None, DC_DYNAMIC_CNF, 0);
        result.set_weight(result.standard_weight());
        state.tmp_store_mut().insert(result);
    }
    state.archive_mut().insert(clause);
    Ok(())
}

fn lambda_normalize_clause_terms(
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<(), Diagnostic> {
    for literal in clause.literals_mut().as_mut_slice() {
        let left = literal.left().clone();
        let right = literal.right().clone();
        let normalized_left = lambda_normalize_db(bank, &left)?;
        let normalized_right = lambda_normalize_db(bank, &right)?;
        literal.set_left_raw(normalized_left);
        literal.set_right_raw(normalized_right);
    }
    clause.set_weight(clause.standard_weight());
    Ok(())
}

/// Normalizes and inserts a surviving selected clause into a processed set.
///
/// This ports the local processed-set insertion tail of C `ProcessClause`:
/// normalize variables, stamp the clause date, set `CPLimitedRW`, classify the
/// clause as a positive rule/equation, negative unit, or non-unit, and insert it
/// through the processed set's FV index when present. Global-index insertion,
/// watchlist simplification, and generation remain separate proof-session
/// responsibilities.
///
/// # Errors
///
/// Returns diagnostics from variable normalization.
pub fn proof_state_insert_processed_clause(
    state: &mut ProofState,
    mut clause: Clause,
    clause_date: SysDate,
) -> Result<ProcessedClauseClass, Diagnostic> {
    let fresh_vars = state.fresh_vars().clone();
    clause.normalize_vars(state.terms_mut(), &fresh_vars)?;
    proof_state_insert_normalized_processed_clause(state, clause, clause_date)
}

fn proof_state_insert_normalized_processed_clause(
    state: &mut ProofState,
    mut clause: Clause,
    clause_date: SysDate,
) -> Result<ProcessedClauseClass, Diagnostic> {
    clause.set_date(clause_date);
    clause.set_prop(CP_LIMITED_RW);
    clause.set_weight(clause.standard_weight());

    let class = if clause.is_demodulator() {
        debug_assert_eq!(clause.negative_literal_count(), 0);
        let is_rule = clause
            .literals()
            .as_slice()
            .first()
            .is_some_and(crate::clauses::eqn::Eqn::is_oriented);
        if is_rule {
            if let Some(literal) = clause.literals().as_slice().first() {
                literal.left().set_prop(TP_IS_REWRITABLE);
            }
            ProcessedClauseClass::PositiveRule
        } else {
            ProcessedClauseClass::PositiveEquation
        }
    } else if clause.is_unit() {
        debug_assert_eq!(clause.negative_literal_count(), 1);
        ProcessedClauseClass::NegativeUnit
    } else {
        ProcessedClauseClass::NonUnit
    };

    let (terms, processed_sets) = state.terms_and_processed_sets_mut();
    match class {
        ProcessedClauseClass::PositiveRule => {
            processed_sets.pos_rules.set_date(clause_date);
            processed_sets
                .pos_rules
                .indexed_insert_clause_owned_with_bank(clause, terms)?;
        }
        ProcessedClauseClass::PositiveEquation => {
            processed_sets.pos_eqns.set_date(clause_date);
            processed_sets
                .pos_eqns
                .indexed_insert_clause_owned_with_bank(clause, terms)?;
        }
        ProcessedClauseClass::NegativeUnit => {
            processed_sets
                .neg_units
                .indexed_insert_clause_owned_with_bank(clause, terms)?;
        }
        ProcessedClauseClass::NonUnit => {
            processed_sets
                .non_units
                .indexed_insert_clause_owned_with_bank(clause, terms)?;
        }
    }

    Ok(class)
}

/// Selects and extracts the next unprocessed clause with the active HCB.
///
/// This is the `control->hcb->hcb_select(control->hcb, state->unprocessed)`
/// prefix of C `ProcessClause`.
///
/// # Errors
///
/// Returns a diagnostic if proof-control has no active heuristic control block.
pub fn proof_state_select_unprocessed_clause(
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<Option<Clause>, Diagnostic> {
    let active_hcb_handle = control.active_hcb.ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "ProcessClause selection requires initialized proof-control heuristic",
        )
    })?;
    let hcb = control
        .hcbs_mut()
        .hcb_mut(active_hcb_handle)
        .ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "active proof-control heuristic handle is invalid",
            )
        })?;
    // Generated children wait in tmp/eval/unprocessed. Detaching unprocessed
    // lets the orphan check inspect only stable source, processed, and archive owners.
    let mut unprocessed = std::mem::take(state.unprocessed_mut());
    let selected = match hcb.hcb_select() {
        HcbSelectFunction::StandardClauseSelect => {
            hcb_standard_clause_select_with(hcb, &mut unprocessed, |clause| {
                clause_is_orphaned_with(clause, |parent| selection_parent_is_dead(state, parent))
            })
        }
        HcbSelectFunction::SingleWeightClauseSelect => {
            hcb_single_weight_clause_select_with(hcb, &mut unprocessed, |clause| {
                clause_is_orphaned_with(clause, |parent| selection_parent_is_dead(state, parent))
            })
        }
    };
    *state.unprocessed_mut() = unprocessed;
    Ok(selected)
}

/// Runs the currently ported backward-simplification tail of C `ProcessClause`.
///
/// This covers plain backward rewriting, backward subsumption, unit
/// back-simplification, backward contextual simplify-reflect, and the final
/// `CPIsIRVictim` marking over `tmp_store`. Use
/// [`proof_state_backward_simplify_with_docs`] for represented
/// backward-subsumption and simplified-clause movement quotes. C comments in
/// the removal helpers mention child killing, but the current bodies archive
/// or requeue the affected clauses without traversing child links.
///
/// # Errors
///
/// Returns diagnostics from backward rewrite matching or clause archive copies.
pub fn proof_state_backward_simplify(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    clause_date: &mut SysDate,
) -> Result<BackwardSimplificationOutcome, Diagnostic> {
    proof_state_backward_simplify_impl::<String>(state, control, clause, clause_date, None, None)
}

/// Runs the currently ported backward-simplification tail while emitting
/// represented proof-documentation quotes for backward-subsumed clauses and
/// simplified clauses moved through `tmp_store`.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_backward_simplify`], plus any
/// proof-documentation write diagnostic.
pub fn proof_state_backward_simplify_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    clause_date: &mut SysDate,
) -> Result<BackwardSimplificationOutcome, Diagnostic> {
    proof_state_backward_simplify_impl(
        state,
        control,
        clause,
        clause_date,
        None,
        Some((output, session)),
    )
}

/// Runs backward simplification while maintaining explicitly supplied global indices.
///
/// C deletes processed clauses from `state->gindices` before moving rewritten
/// or unit/context-simplified clauses into `tmp_store`, or before archiving
/// backward-subsumed clauses. This variant preserves that order for the
/// explicit Rust index owner.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_backward_simplify`].
pub fn proof_state_backward_simplify_with_global_indices(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    clause_date: &mut SysDate,
    indices: &mut GlobalIndices,
) -> Result<BackwardSimplificationOutcome, Diagnostic> {
    proof_state_backward_simplify_impl::<String>(
        state,
        control,
        clause,
        clause_date,
        Some(indices),
        None,
    )
}

/// Runs backward simplification with explicitly supplied global indices while emitting
/// represented proof-documentation quotes for backward-subsumed clauses and
/// simplified clauses moved through `tmp_store`.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_backward_simplify_with_global_indices`], plus any
/// proof-documentation write diagnostic.
pub fn proof_state_backward_simplify_with_global_indices_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    clause_date: &mut SysDate,
    indices: &mut GlobalIndices,
) -> Result<BackwardSimplificationOutcome, Diagnostic> {
    proof_state_backward_simplify_impl(
        state,
        control,
        clause,
        clause_date,
        Some(indices),
        Some((output, session)),
    )
}

fn proof_state_backward_simplify_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    clause_date: &mut SysDate,
    mut indices: Option<&mut GlobalIndices>,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<BackwardSimplificationOutcome, Diagnostic> {
    let _timer =
        crate::basics::perf_counters::start(crate::basics::perf_counters::PerfCounter::BwrwTimer);
    let mut outcome = BackwardSimplificationOutcome::default();

    let old_lit_count = state.tmp_store().literals();
    let old_clause_count = state.tmp_store().members();
    let lambda_demod = control.heuristic_parms().lambda_demod;
    outcome.min_rw_detected = proof_state_eliminate_backward_rewritten_clauses(
        state,
        control,
        clause,
        clause_date,
        indices.as_deref_mut(),
        &mut doc_context,
    )?;
    let rewritten_lits = state.tmp_store().literals() - old_lit_count;
    let rewritten = state.tmp_store().members() - old_clause_count;
    outcome.rewritten_literals = i64_to_u64_saturating(rewritten_lits);
    outcome.rewritten = i64_to_u64_saturating(rewritten);
    {
        let statistics = state.statistics_mut();
        statistics.backward_rewritten_lit_count += outcome.rewritten_literals;
        statistics.backward_rewritten_count += outcome.rewritten;
    }

    outcome.subsumed = proof_state_eliminate_backward_subsumed_clauses(
        state,
        clause,
        indices.as_deref_mut(),
        lambda_demod,
        &mut doc_context,
    )?;
    state.statistics_mut().backward_subsumed_count += outcome.subsumed;
    outcome.unit_simplified = proof_state_eliminate_unit_simplified_clauses(
        state,
        clause,
        indices.as_deref_mut(),
        lambda_demod,
        &mut doc_context,
    )?;
    outcome.context_sr = proof_state_eliminate_context_sr_clauses(
        state,
        control,
        clause,
        indices,
        lambda_demod,
        &mut doc_context,
    )?;

    outcome.tmp_store_marked = state.tmp_store().members();
    state.tmp_store_mut().set_prop(CP_IS_IR_VICTIM);
    Ok(outcome)
}

/// Runs the currently ported generators from C `generate_new_clauses`.
///
/// The available slice covers higher-order argument congruence plus first-order
/// equality factoring, equality resolution, disequality decomposition, and
/// unindexed plain/simultaneous paramodulation in the same order as the C
/// helper. The remaining higher-order generators and state-owned indexed
/// paramodulation remain explicit staging diagnostics.
///
/// # Errors
///
/// Returns diagnostics from generator helpers, missing ordering state, or an
/// unported generation branch that would be reached in C.
pub fn proof_state_generate_new_clauses(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
) -> Result<GenerateNewClausesOutcome, Diagnostic> {
    proof_state_generate_new_clauses_impl::<String>(
        state,
        control,
        clause,
        problem_type(),
        None,
        None,
    )
}

/// Runs the ported first-order selected-clause generators while emitting
/// represented proof-documentation output for generated equality factors,
/// equality resolvents, and paramodulants.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_generate_new_clauses`], plus
/// any proof-documentation write diagnostic.
pub fn proof_state_generate_new_clauses_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
) -> Result<GenerateNewClausesOutcome, Diagnostic> {
    proof_state_generate_new_clauses_impl(
        state,
        control,
        clause,
        problem_type(),
        None,
        Some((output, session)),
    )
}

/// Runs the ported selected-clause generators with explicitly supplied global
/// indices.
///
/// This mirrors C's indexed paramodulation branch when `indices` has
/// paramodulation indexes initialized. `ProofState` owns the production index;
/// this lower-level entry point keeps the borrow explicit so the inference core
/// can also be tested with an isolated index.
///
/// # Errors
///
/// Returns diagnostics from generator helpers, missing ordering state, or an
/// unported generation branch that would be reached in C.
pub fn proof_state_generate_new_clauses_with_global_indices(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    indices: &GlobalIndices,
) -> Result<GenerateNewClausesOutcome, Diagnostic> {
    proof_state_generate_new_clauses_impl::<String>(
        state,
        control,
        clause,
        problem_type(),
        Some(indices),
        None,
    )
}

/// Runs the ported first-order selected-clause generators with explicitly supplied
/// global indices while emitting represented proof-documentation output for
/// generated equality factors, equality resolvents, and paramodulants.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_generate_new_clauses_with_global_indices`], plus any
/// proof-documentation write diagnostic.
pub fn proof_state_generate_new_clauses_with_global_indices_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    indices: &GlobalIndices,
) -> Result<GenerateNewClausesOutcome, Diagnostic> {
    proof_state_generate_new_clauses_impl(
        state,
        control,
        clause,
        problem_type(),
        Some(indices),
        Some((output, session)),
    )
}

#[derive(Clone, Debug)]
struct InductionAbstractionBucket {
    type_: Type,
    pairs: Vec<(Term, Clause)>,
}

#[derive(Clone, Debug, Default)]
struct InductionAbstractionStore {
    buckets: Vec<InductionAbstractionBucket>,
}

impl InductionAbstractionStore {
    fn add(&mut self, abstraction: Term, clause: &Clause) {
        let type_ = abstraction
            .type_()
            .expect("induction abstraction must be typed");
        let bucket = if let Some(bucket) = self
            .buckets
            .iter_mut()
            .find(|bucket| type_identity_cmp(&bucket.type_, &type_) == 0)
        {
            bucket
        } else {
            self.buckets.push(InductionAbstractionBucket {
                type_: type_.clone(),
                pairs: Vec::new(),
            });
            self.buckets
                .last_mut()
                .expect("new abstraction bucket was just inserted")
        };

        if bucket
            .pairs
            .iter()
            .any(|(seen, _clause)| seen == &abstraction)
        {
            return;
        }
        bucket.pairs.push((abstraction, clause.clone()));
    }

    fn hits(&self, type_: &Type) -> Option<&[(Term, Clause)]> {
        self.buckets
            .iter()
            .find(|bucket| type_identity_cmp(&bucket.type_, type_) == 0)
            .map(|bucket| bucket.pairs.as_slice())
    }
}

/// Applies the currently ported C `PreinstantiateInduction` preprocessing step.
///
/// The helper collects induction abstractions from archived conjecture formulas
/// and single-literal conjecture clauses, then instantiates every active clause
/// variable whose type matches a collected abstraction. Generated clauses are
/// inserted after the scan in stack-pop order, matching C's `PStack` drain.
///
/// # Errors
///
/// Returns diagnostics from formula encoding, term-bank insertion,
/// beta-normalization, literal allocation, or Boolean simplification.
pub fn preinstantiate_induction(state: &mut ProofState) -> Result<i64, Diagnostic> {
    let formula_archive = state.f_ax_archive().iter().cloned().collect::<Vec<_>>();
    let (bank, clauses, archive) = state.terms_axioms_archive_mut();
    preinstantiate_induction_sets(bank, &formula_archive, clauses, archive)
}

fn preinstantiate_induction_sets(
    bank: &mut TermBank,
    formula_archive: &[WrappedFormula],
    clauses: &mut ClauseSet,
    archive: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    bank.vars().set_v_counts_to_used();
    let mut store = InductionAbstractionStore::default();

    for formula in formula_archive {
        if formula.query_tptp_type() == CP_TYPE_CONJECTURE {
            store_induction_abstraction_form(bank, formula, archive, &mut store)?;
        }
    }
    for clause in clauses.iter() {
        if clause.is_conjecture() && clause.literal_number() == 1 {
            store_induction_abstraction_clause(bank, clause, &mut store)?;
        }
    }

    let mut generated = Vec::new();
    for clause in clauses.iter() {
        let mut vars = BTreeMap::new();
        let _ = clause.collect_variables(&mut vars);
        for var in vars.values() {
            instantiate_induction_abstractions(bank, var, clause, &store, &mut generated)?;
        }
    }

    let count = i64::try_from(generated.len()).unwrap_or(i64::MAX);
    while let Some(clause) = generated.pop() {
        clauses.insert(clause);
    }
    Ok(count)
}

fn store_induction_abstraction_form(
    bank: &mut TermBank,
    formula: &WrappedFormula,
    archive: &mut ClauseSet,
    store: &mut InductionAbstractionStore,
) -> Result<(), Diagnostic> {
    if !tformula_is_quantified_nl(bank, formula.formula()) || formula.formula().arity() != 2 {
        return Ok(());
    }

    let encoded = post_cnf_encode_formulas(bank, formula.formula())?;
    let true_term = bank.true_term().clone();
    let literal = Eqn::alloc(encoded.clone(), true_term, bank, true)?;
    let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
    clause_push_formula_derivation(
        &mut clause,
        DC_FOF_QUOTE,
        Some(formula.derivation_ref()),
        None,
    );
    archive.insert(clause.clone());

    let mut quantified = encoded;
    while tformula_is_quantified_nl(bank, &quantified) && quantified.arity() == 1 {
        let lambda = quantified
            .argument(0)
            .expect("encoded quantified formula must have a lambda argument");
        store.add(lambda.clone(), &clause);
        let binder_type = lambda
            .argument(0)
            .and_then(|binder| binder.type_())
            .expect("encoded quantified lambda binder must be typed");
        let fresh_var = bank.vars().get_fresh_var(&binder_type);
        let applied = bank.term_apply_arg(&lambda, &fresh_var);
        let applied = bank.term_top_insert(applied)?;
        quantified = whnf_step(bank, &applied)?;
    }

    Ok(())
}

fn store_induction_abstraction_clause(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut InductionAbstractionStore,
) -> Result<(), Diagnostic> {
    debug_assert_eq!(clause.literal_number(), 1);
    let literal = clause
        .literals()
        .as_slice()
        .first()
        .expect("single-literal clause must have a literal");

    if literal.left().f_code() <= bank.signature().internal_symbols() {
        return Ok(());
    }

    let terms = [literal.left().clone(), literal.right().clone()];
    for term_index in 0..2 {
        let term = &terms[term_index];
        let other = &terms[1 - term_index];
        if term.f_code() == other.f_code() {
            for argument in term.argument_clones().into_iter().flatten() {
                if term_is_db_closed(&argument) && term_contains_subterm(other, &argument) {
                    let abstraction = abstract_induction_arg(
                        bank,
                        term,
                        other,
                        &argument,
                        !literal.is_positive(),
                    )?;
                    store.add(abstraction, clause);
                }
            }
        } else {
            debug_assert!(term_is_db_closed(term));
            debug_assert!(term_is_db_closed(other));
            if term_contains_subterm(other, term) {
                let abstraction =
                    abstract_induction_arg(bank, term, other, term, !literal.is_positive())?;
                store.add(abstraction, clause);
            }
        }
    }

    Ok(())
}

fn instantiate_induction_abstractions(
    bank: &mut TermBank,
    var: &Term,
    orig_clause: &Clause,
    store: &InductionAbstractionStore,
    generated: &mut Vec<Clause>,
) -> Result<(), Diagnostic> {
    let Some(var_type) = var.type_() else {
        return Ok(());
    };
    let Some(hits) = store.hits(&var_type) else {
        return Ok(());
    };

    assert!(
        var.binding().is_none(),
        "induction preinstantiation variable must be unbound"
    );
    for (target, other_clause) in hits {
        debug_assert_eq!(var.type_(), target.type_());
        var.set_binding(Some(target.clone()));
        let result = (|| {
            let mut new_literals = Vec::with_capacity(orig_clause.literal_number());
            for literal in orig_clause.literals().as_slice() {
                new_literals.push(literal.copy_instantiated_ho(bank)?);
            }
            let mut new_literals = EqnList::from_vec(new_literals);
            beta_normalize_eqn_list(bank, &mut new_literals)?;
            let _ = new_literals.remove_resolved(bank);
            let _ = new_literals.remove_duplicates(bank);

            let mut new_clause = Clause::alloc(new_literals);
            let _ = clause_normalize_equations(&mut new_clause, bank);
            set_ho_generation_proof_object(
                &mut new_clause,
                orig_clause,
                Some(other_clause),
                DC_TRIGGER,
                1,
            );
            let _ = clause_boolean_simplification(&mut new_clause, bank)?;
            generated.push(new_clause);
            Ok(())
        })();
        var.set_binding(None);
        result?;
    }
    Ok(())
}

fn abstract_induction_arg(
    bank: &mut TermBank,
    lhs: &Term,
    rhs: &Term,
    arg: &Term,
    sign: bool,
) -> Result<Term, Diagnostic> {
    let mut refreshed_vars = Vec::new();
    let result = (|| {
        let lhs_abs = do_induction_abstract(lhs, arg, bank, 0, &mut refreshed_vars)?;
        let rhs_abs = do_induction_abstract(rhs, arg, bank, 0, &mut refreshed_vars)?;
        let matrix =
            Eqn::terms_tb_term_encode(bank, &lhs_abs, &rhs_abs, sign, PatEqnDirection::Normal)?;
        let arg_type = arg
            .type_()
            .expect("induction abstraction argument must be typed");
        close_with_db_var(bank, &arg_type, &matrix)
    })();
    for var in refreshed_vars {
        var.set_binding(None);
    }
    result
}

fn do_induction_abstract(
    term: &Term,
    arg: &Term,
    bank: &mut TermBank,
    depth: i64,
    refreshed_vars: &mut Vec<Term>,
) -> Result<Term, Diagnostic> {
    if term == arg {
        let arg_type = arg
            .type_()
            .expect("induction abstraction argument must be typed");
        return Ok(bank.request_db_var(&arg_type, depth));
    }
    if term.is_lambda() {
        let old_matrix = term
            .argument(1)
            .expect("lambda term must have a matrix argument");
        let new_matrix = do_induction_abstract(&old_matrix, arg, bank, depth + 1, refreshed_vars)?;
        if new_matrix == old_matrix {
            return Ok(term.clone());
        }
        let binder_type = term
            .argument(0)
            .and_then(|binder| binder.type_())
            .expect("lambda binder must be typed");
        return close_with_db_var(bank, &binder_type, &new_matrix);
    }
    if term.is_free_var() {
        if let Some(binding) = term.binding() {
            return Ok(binding);
        }
        let var_type = term
            .type_()
            .expect("induction abstraction free variable must be typed");
        let fresh_var = bank.vars().get_fresh_var(&var_type);
        term.set_binding(Some(fresh_var.clone()));
        refreshed_vars.push(term.clone());
        return Ok(fresh_var);
    }
    if term.arity() == 0 {
        return Ok(term.clone());
    }

    let new_term = Term::top_copy_without_args(term);
    let mut changed = false;
    for (index, old_arg) in term.argument_clones().into_iter().enumerate() {
        let old_arg = old_arg.expect("induction abstraction argument must be initialized");
        let new_arg = do_induction_abstract(&old_arg, arg, bank, depth, refreshed_vars)?;
        changed |= new_arg != old_arg;
        new_term.set_argument(index, new_arg);
    }

    if changed {
        bank.term_top_insert(new_term)
    } else {
        Ok(term.clone())
    }
}

fn term_contains_subterm(term: &Term, needle: &Term) -> bool {
    term == needle
        || term
            .argument_clones()
            .into_iter()
            .flatten()
            .any(|argument| term_contains_subterm(&argument, needle))
}

fn compute_ho_inferences(
    state: &mut ProofState,
    control: &ProofControl,
    renamed_clause: &Clause,
    clause: &Clause,
    problem_type: ProblemType,
    indices: Option<&GlobalIndices>,
) -> Result<i64, Diagnostic> {
    if problem_type != ProblemType::HigherOrder {
        return Ok(0);
    }

    let parms = control.heuristic_parms();
    let ext_rule_indices = ext_rule_indices(parms, indices)?;

    let mut generated = 0;
    let mut neg_ext_count = 0;
    {
        let (terms, generation) = state.terms_and_generation_context_mut();
        if parms.arg_cong != ExtInferenceType::NoLits {
            generated += compute_arg_cong(terms, clause, generation.tmp_store, parms.arg_cong)?;
        }
        if parms.neg_ext != ExtInferenceType::NoLits {
            let neg_ext = compute_neg_ext(terms, clause, generation.tmp_store, parms.neg_ext)?;
            generated += neg_ext;
            neg_ext_count = neg_ext;
        }
        if parms.neg_ext != ExtInferenceType::NoLits {
            generated += compute_pos_ext(terms, clause, generation.tmp_store, parms.pos_ext)?;
        }
        if parms.inverse_recognition {
            generated += compute_inverse_recognition(terms, clause, generation.tmp_store)?;
        }
        if let Some(indices) = ext_rule_indices {
            generated += compute_ext_sup(
                terms,
                renamed_clause,
                clause,
                generation.tmp_store,
                indices,
                parms.ext_rules_max_depth,
            )?;
            generated += compute_ext_eq_res(
                terms,
                clause,
                generation.tmp_store,
                parms.ext_rules_max_depth,
            )?;
            generated += compute_ext_eq_fact(
                terms,
                clause,
                generation.tmp_store,
                parms.ext_rules_max_depth,
            )?;
        }
        if parms.elim_leibniz_max_depth >= 0 {
            generated += compute_leibniz_elimination(
                terms,
                clause,
                generation.tmp_store,
                parms.elim_leibniz_max_depth,
            )?;
        }
        if parms.prim_enum_max_depth >= 0 {
            generated += compute_primitive_enumeration(
                terms,
                clause,
                generation.tmp_store,
                parms.prim_enum_mode,
                parms.prim_enum_max_depth,
            )?;
        }
        if parms.inst_choice_max_depth >= 0 {
            generated += instantiate_choice_clauses(
                terms,
                renamed_clause,
                clause,
                generation.tmp_store,
                generation.archive,
                generation.choice_opcodes,
                parms.inst_choice_max_depth,
            )?;
        }
    }
    state.statistics_mut().neg_ext_count += u64::try_from(neg_ext_count).unwrap_or(u64::MAX);
    Ok(generated)
}

fn ext_rule_indices<'indices>(
    parms: &HeuristicParmsCell,
    indices: Option<&'indices GlobalIndices>,
) -> Result<Option<&'indices GlobalIndices>, Diagnostic> {
    if parms.ext_rules_max_depth >= 0 {
        let indices = indices.ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "higher-order extensional superposition generation requires explicitly supplied ExtSup indexes",
            )
        })?;
        if !indices.has_ext_into_index() || !indices.has_ext_from_index() {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "higher-order extensional superposition generation requires explicitly supplied ExtSup indexes",
            ));
        }
        return Ok(Some(indices));
    }
    Ok(None)
}

/// Computes C `ComputeExtSup` over explicitly supplied extension indexes.
///
/// `renamed_clause` must be a disjoint variable copy of `orig_clause` with the
/// same identifier and proof metrics, matching the C `tmp_copy` argument.
///
/// # Errors
///
/// Returns diagnostics from term replacement, instantiated insertion,
/// optimized literal copying, beta normalization, and literal allocation.
pub fn compute_ext_sup(
    bank: &mut TermBank,
    renamed_clause: &Clause,
    orig_clause: &Clause,
    store: &mut ClauseSet,
    indices: &GlobalIndices,
    limit: i32,
) -> Result<i64, Diagnostic> {
    if orig_clause.proof_depth() > i64::from(limit) {
        return Ok(0);
    }

    let mut generated = 0;
    generated += compute_ext_sup_from(bank, renamed_clause, orig_clause, store, indices)?;
    generated += compute_ext_sup_into(bank, renamed_clause, orig_clause, store, indices)?;
    Ok(generated)
}

fn compute_ext_sup_from(
    bank: &mut TermBank,
    renamed_clause: &Clause,
    orig_clause: &Clause,
    store: &mut ClauseSet,
    indices: &GlobalIndices,
) -> Result<i64, Diagnostic> {
    let mut positions = Vec::new();
    collect_ext_sup_from_pos(renamed_clause, &mut positions);
    let mut generated = 0;
    for entry in positions.iter().rev() {
        let from_pos = unpack_clause_pos(entry.pos(), renamed_clause.clone());
        let Some(into_partners) = indices.find_ext_into_symbol(entry.f_code()) else {
            continue;
        };
        for partner in into_partners.entries() {
            for into_cpos in partner.positions() {
                let into_pos = unpack_clause_pos(*into_cpos, partner.clause().clone());
                generated += make_ext_sup(
                    bank,
                    &from_pos,
                    &into_pos,
                    store,
                    orig_clause,
                    ExtSupSelectedRole::From,
                )?;
            }
        }
    }
    Ok(generated)
}

fn compute_ext_sup_into(
    bank: &mut TermBank,
    renamed_clause: &Clause,
    orig_clause: &Clause,
    store: &mut ClauseSet,
    indices: &GlobalIndices,
) -> Result<i64, Diagnostic> {
    let mut positions = Vec::new();
    collect_ext_sup_into_pos(renamed_clause, &mut positions);
    let mut generated = 0;
    for entry in positions.iter().rev() {
        let into_pos = unpack_clause_pos(entry.pos(), renamed_clause.clone());
        let Some(from_partners) = indices.find_ext_from_symbol(entry.f_code()) else {
            continue;
        };
        for partner in from_partners.entries() {
            for from_cpos in partner.positions() {
                let from_pos = unpack_clause_pos(*from_cpos, partner.clause().clone());
                generated += make_ext_sup(
                    bank,
                    &from_pos,
                    &into_pos,
                    store,
                    orig_clause,
                    ExtSupSelectedRole::Into,
                )?;
            }
        }
    }
    Ok(generated)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExtSupSelectedRole {
    From,
    Into,
}

fn make_ext_sup(
    bank: &mut TermBank,
    from_pos: &ClausePos,
    into_pos: &ClausePos,
    store: &mut ClauseSet,
    orig_clause: &Clause,
    selected_role: ExtSupSelectedRole,
) -> Result<i64, Diagnostic> {
    if ext_sup_positive_top_duplicate(from_pos, into_pos) {
        return Ok(0);
    }

    let from_t = from_pos
        .get_subterm()
        .expect("ExtSup from position must select a subterm");
    let into_t = into_pos
        .get_subterm()
        .expect("ExtSup into position must select a subterm");
    let mut disagreements = Vec::new();
    if !find_ext_disagreements(bank, &from_t, &into_t, &mut disagreements) {
        return Ok(0);
    }

    let from_clause = from_pos
        .clause()
        .expect("ExtSup from position must be backed by a clause");
    let into_clause = into_pos
        .clause()
        .expect("ExtSup into position must be backed by a clause");
    let from_index = from_pos
        .literal_index()
        .expect("ExtSup from position must select a literal");
    let into_index = into_pos
        .literal_index()
        .expect("ExtSup into position must select a literal");
    let into_literal = into_pos
        .literal()
        .expect("ExtSup into position must select a literal");

    let freshvars = fresh_var_bank_for_ext_sup_clauses(bank, from_clause, into_clause);
    let mut subst = Substitution::new();
    let result = (|| {
        let _ = from_clause.literals().subst_norm(&mut subst, &freshvars);
        let _ = into_clause.literals().subst_norm(&mut subst, &freshvars);

        let mut new_literals = ext_disagreement_literals_instantiated(bank, &disagreements)?;
        let from_rhs = from_pos
            .get_other_side()
            .expect("ExtSup from position must select an opposite side");
        let into_rhs = into_pos
            .get_other_side()
            .expect("ExtSup into position must select an opposite side");
        let new_lhs = tb_term_pos_replace(
            bank,
            &from_rhs,
            into_pos.term_pos(),
            DerefType::Always,
            0,
            Some(&into_t),
        )?;
        let new_rhs = bank.insert_opt(&into_rhs, DerefType::Always)?;

        let into_copy = into_clause
            .literals()
            .copy_opt_except_index(Some(into_index), bank)?;
        let from_copy = from_clause
            .literals()
            .copy_opt_except_index(Some(from_index), bank)?;
        new_literals.append(into_copy);
        new_literals.append(from_copy);
        new_literals.append(EqnList::from_vec(vec![Eqn::alloc(
            new_lhs,
            new_rhs,
            bank,
            into_literal.is_positive(),
        )?]));
        new_literals.remove_resolved(bank);
        new_literals.remove_duplicates(bank);
        beta_normalize_eqn_list(bank, &mut new_literals)?;

        let mut new_clause = Clause::alloc(new_literals);
        new_clause.set_proof_size(into_clause.proof_size() + from_clause.proof_size() + 1);
        new_clause.set_proof_depth(into_clause.proof_depth().max(from_clause.proof_depth()) + 1);
        new_clause.set_prop(into_clause.give_props(CP_IS_SOS) | from_clause.give_props(CP_IS_SOS));
        let (parent1, parent2) = match selected_role {
            ExtSupSelectedRole::From => (into_clause, orig_clause),
            ExtSupSelectedRole::Into => (orig_clause, from_clause),
        };
        clause_push_derivation(&mut new_clause, DC_EXT_SUP, Some(parent1), Some(parent2));
        store.insert(new_clause);
        Ok(1)
    })();
    subst.backtrack();
    result
}

fn ext_sup_positive_top_duplicate(from_pos: &ClausePos, into_pos: &ClausePos) -> bool {
    let from_literal = from_pos
        .literal()
        .expect("ExtSup from position must select a literal");
    let into_literal = into_pos
        .literal()
        .expect("ExtSup into position must select a literal");
    if !from_literal.is_positive() || !into_literal.is_positive() || !into_pos.is_top() {
        return false;
    }
    let from_other = from_pos
        .get_other_side()
        .expect("ExtSup from position must select an opposite side");
    let into_other = into_pos
        .get_other_side()
        .expect("ExtSup into position must select an opposite side");
    from_other == into_other
        || from_pos
            .clause()
            .expect("ExtSup from position must be backed by a clause")
            .ident()
            < into_pos
                .clause()
                .expect("ExtSup into position must be backed by a clause")
                .ident()
}

fn ext_disagreement_literals_instantiated(
    bank: &mut TermBank,
    disagreements: &[(Term, Term)],
) -> Result<EqnList, Diagnostic> {
    let mut literals = Vec::with_capacity(disagreements.len());
    for (left, right) in disagreements {
        let lhs = bank.insert_instantiated_for_problem(right, ProblemType::HigherOrder)?;
        let rhs = bank.insert_instantiated_for_problem(left, ProblemType::HigherOrder)?;
        literals.push(Eqn::alloc(lhs, rhs, bank, false)?);
    }
    Ok(EqnList::from_vec(literals))
}

/// Computes the local `ComputeExtEqRes` part of C higher-order extension rules.
///
/// # Errors
///
/// Returns diagnostics from optimized literal copying, beta normalization, and
/// literal allocation.
pub fn compute_ext_eq_res(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    limit: i32,
) -> Result<i64, Diagnostic> {
    if clause.proof_depth() > i64::from(limit) {
        return Ok(0);
    }

    let mut generated = 0;
    for (literal_index, literal) in clause.literals().as_slice().iter().enumerate() {
        let literal_matches = literal.is_negative()
            && literal.is_equ_lit(bank)
            && !literal.left().type_().is_some_and(|type_| type_.is_arrow())
            && literal.left().f_code() == literal.right().f_code()
            && !literal.left().is_phony_app()
            && !literal.left().is_db_var()
            && !literal.right().is_db_var()
            && term_has_ext_eligible_subterm(literal.left())
            && term_has_ext_eligible_subterm(literal.right());
        if !literal_matches {
            continue;
        }
        generated += make_ext_eq_res(bank, clause, literal_index, literal, store)?;
    }
    Ok(generated)
}

/// Computes the local `ComputeExtEqFact` part of C higher-order extension rules.
///
/// # Errors
///
/// Returns diagnostics from optimized literal copying, beta normalization, and
/// literal allocation.
pub fn compute_ext_eq_fact(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    limit: i32,
) -> Result<i64, Diagnostic> {
    if clause.proof_depth() > i64::from(limit) {
        return Ok(0);
    }

    let positions = ext_eq_fact_positions(clause);
    let mut generated = 0;
    for (main_index, main_pos) in positions.iter().enumerate() {
        for partner_pos in &positions[main_index + 1..] {
            if partner_pos.literal_index <= main_pos.literal_index {
                continue;
            }
            generated += make_ext_eq_fact(bank, clause, main_pos, partner_pos, store)?;
        }
    }
    Ok(generated)
}

fn make_ext_eq_res(
    bank: &mut TermBank,
    clause: &Clause,
    literal_index: usize,
    literal: &Eqn,
    store: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    let mut disagreements = Vec::new();
    if !find_ext_disagreements(bank, literal.left(), literal.right(), &mut disagreements) {
        return Ok(0);
    }

    let mut new_literals = ext_disagreement_literals(bank, &disagreements)?;
    let rest = clause
        .literals()
        .copy_opt_except_index(Some(literal_index), bank)?;
    new_literals.append(rest);
    new_literals.remove_resolved(bank);
    new_literals.remove_duplicates(bank);
    beta_normalize_eqn_list(bank, &mut new_literals)?;

    let mut new_clause = Clause::alloc(new_literals);
    set_ho_generation_proof_object(&mut new_clause, clause, None, DC_EXT_EQ_RES, 1);
    store.insert(new_clause);
    Ok(1)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExtEqFactPosition {
    literal_index: usize,
    side: EqnSide,
}

fn ext_eq_fact_positions(clause: &Clause) -> Vec<ExtEqFactPosition> {
    let mut positions = Vec::new();
    for (literal_index, literal) in clause.literals().as_slice().iter().enumerate() {
        if !literal.is_positive() {
            continue;
        }
        if term_has_ext_eligible_subterm(literal.left()) {
            positions.push(ExtEqFactPosition {
                literal_index,
                side: EqnSide::LeftSide,
            });
        }
        if term_has_ext_eligible_subterm(literal.right()) {
            positions.push(ExtEqFactPosition {
                literal_index,
                side: EqnSide::RightSide,
            });
        }
    }
    positions
}

fn make_ext_eq_fact(
    bank: &mut TermBank,
    clause: &Clause,
    main_pos: &ExtEqFactPosition,
    partner_pos: &ExtEqFactPosition,
    store: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    let main_literal = &clause.literals().as_slice()[main_pos.literal_index];
    let partner_literal = &clause.literals().as_slice()[partner_pos.literal_index];
    let main_term = ext_eq_fact_side(main_literal, main_pos.side);
    let partner_term = ext_eq_fact_side(partner_literal, partner_pos.side);

    let mut disagreements = Vec::new();
    if !find_ext_disagreements(bank, &main_term, &partner_term, &mut disagreements) {
        return Ok(0);
    }

    let mut new_literals = ext_disagreement_literals(bank, &disagreements)?;
    let main_other = ext_eq_fact_other_side(main_literal, main_pos.side);
    let partner_other = ext_eq_fact_other_side(partner_literal, partner_pos.side);
    new_literals.append(EqnList::from_vec(vec![Eqn::alloc(
        main_other,
        partner_other,
        bank,
        false,
    )?]));
    let rest = clause
        .literals()
        .copy_opt_except_index(Some(main_pos.literal_index), bank)?;
    new_literals.append(rest);
    new_literals.remove_resolved(bank);
    new_literals.remove_duplicates(bank);
    beta_normalize_eqn_list(bank, &mut new_literals)?;

    let mut new_clause = Clause::alloc(new_literals);
    new_clause.set_proof_size(clause.proof_size() + 1);
    new_clause.set_proof_depth(clause.proof_depth() + 1);
    new_clause.set_prop(clause.give_props(CP_IS_SOS));
    clause_push_derivation(&mut new_clause, DC_EXT_EQ_FACT, Some(clause), None);
    store.insert(new_clause);
    Ok(1)
}

fn ext_eq_fact_side(literal: &Eqn, side: EqnSide) -> Term {
    match side {
        EqnSide::LeftSide => literal.left().clone(),
        EqnSide::RightSide => literal.right().clone(),
        EqnSide::NoSide | EqnSide::BothSides => {
            panic!("ExtEqFact position must select exactly one side")
        }
    }
}

fn ext_eq_fact_other_side(literal: &Eqn, side: EqnSide) -> Term {
    match side {
        EqnSide::LeftSide => literal.right().clone(),
        EqnSide::RightSide => literal.left().clone(),
        EqnSide::NoSide | EqnSide::BothSides => {
            panic!("ExtEqFact position must select exactly one side")
        }
    }
}

fn ext_disagreement_literals(
    bank: &mut TermBank,
    disagreements: &[(Term, Term)],
) -> Result<EqnList, Diagnostic> {
    let mut literals = Vec::with_capacity(disagreements.len());
    for (left, right) in disagreements {
        literals.push(Eqn::alloc(right.clone(), left.clone(), bank, false)?);
    }
    Ok(EqnList::from_vec(literals))
}

fn find_ext_disagreements(
    bank: &TermBank,
    left: &Term,
    right: &Term,
    disagreements: &mut Vec<(Term, Term)>,
) -> bool {
    if left.type_() != right.type_() || left == right {
        return false;
    }

    let start_len = disagreements.len();
    let mut tasks = vec![(left.clone(), right.clone())];
    let mut exists_eligible = false;

    while let Some((task_left, task_right)) = tasks.pop() {
        if task_left == task_right {
            continue;
        }
        if same_ext_disagreement_head_allows_descent(bank, &task_left, &task_right) {
            debug_assert_eq!(
                task_left.arity(),
                task_right.arity(),
                "same extension-disagreement head must have matching arity"
            );
            for index in 0..task_left.arity() {
                let left_arg = task_left
                    .argument(index)
                    .unwrap_or_else(|| panic!("left disagreement argument {index} is missing"));
                let right_arg = task_right
                    .argument(index)
                    .unwrap_or_else(|| panic!("right disagreement argument {index} is missing"));
                debug_assert_eq!(
                    left_arg.type_(),
                    right_arg.type_(),
                    "matching disagreement arguments must have matching types"
                );
                tasks.push((left_arg, right_arg));
            }
        } else {
            exists_eligible |= type_ext_eligible(&task_right)
                && !task_left.is_free_var()
                && !task_right.is_free_var();
            disagreements.push((task_left, task_right));
        }
    }

    if !exists_eligible {
        disagreements.truncate(start_len);
    }
    exists_eligible
}

fn same_ext_disagreement_head_allows_descent(bank: &TermBank, left: &Term, right: &Term) -> bool {
    if left.is_phony_app() || right.is_phony_app() || left.is_lambda() || right.is_lambda() {
        return false;
    }
    if left.f_code() != right.f_code() {
        return false;
    }
    let is_polymorphic = left.f_code() > 0 && bank.signature().is_polymorphic(left.f_code());
    !is_polymorphic
        || left.arity() == 0
        || left.argument(0).and_then(|arg| arg.type_())
            == right.argument(0).and_then(|arg| arg.type_())
}

fn compute_inverse_recognition(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    let Some(inverse_definition) = clause_recognize_injectivity(bank, clause)? else {
        return Ok(0);
    };
    store.insert(inverse_definition);
    Ok(1)
}

fn compute_arg_cong(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    mode: ExtInferenceType,
) -> Result<i64, Diagnostic> {
    let mut generated = 0;
    let freshvars = fresh_var_bank_for_arg_cong_clause(bank, clause);
    for (literal_index, literal) in clause.literals().as_slice().iter().enumerate() {
        let Some(literal_type) = literal.left().type_() else {
            continue;
        };
        let needed_args = type_get_max_arity(&literal_type);
        let literal_matches = literal.is_positive()
            && needed_args > 0
            && (matches!(mode, ExtInferenceType::AllLits)
                || matches!(mode, ExtInferenceType::MaxLits) && literal.is_maximal());
        if !literal_matches {
            continue;
        }

        let mut fresh_args = Vec::with_capacity(needed_args);
        for arg_index in 0..needed_args {
            let fresh_arg = freshvars.get_fresh_var(&literal_type.args()[arg_index]);
            let fresh_arg = bank.insert(&fresh_arg, DerefType::Never)?;
            fresh_args.push(fresh_arg);

            let new_left = apply_terms(bank, literal.left(), &fresh_args)?;
            let new_right = apply_terms(bank, literal.right(), &fresh_args)?;
            debug_assert_eq!(new_left.type_(), new_right.type_());
            let new_literal = Eqn::alloc(new_left, new_right, bank, true)?;
            let mut new_literals = clause
                .literals()
                .copy_except_index(Some(literal_index), bank)?;
            new_literals.insert_first(new_literal);
            beta_normalize_eqn_list(bank, &mut new_literals)?;

            let mut new_clause = Clause::alloc(new_literals);
            set_ho_generation_proof_object(&mut new_clause, clause, None, DC_ARG_CONG, 0);
            store.insert(new_clause);
            generated += 1;
        }
    }
    Ok(generated)
}

fn compute_neg_ext(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    mode: ExtInferenceType,
) -> Result<i64, Diagnostic> {
    let mut generated = 0;
    for (literal_index, literal) in clause.literals().as_slice().iter().enumerate() {
        let Some(literal_type) = literal.left().type_() else {
            continue;
        };
        let needed_args = type_get_max_arity(&literal_type);
        let literal_matches = literal.is_negative()
            && needed_args > 0
            && (matches!(mode, ExtInferenceType::AllLits)
                || matches!(mode, ExtInferenceType::MaxLits) && literal.is_maximal());
        if !literal_matches {
            continue;
        }

        let mut variables = BTreeMap::new();
        let _ = literal.collect_variables(&mut variables);
        let variables = variables.into_values().collect::<Vec<_>>();
        let mut new_left = literal.left().clone();
        let mut new_right = literal.right().clone();
        for arg_type in literal_type.args().iter().take(needed_args) {
            let skolem = bank.alloc_new_skolem(&variables, Some(arg_type))?;
            let applied_left = bank.term_apply_arg(&new_left, &skolem);
            new_left = bank.term_top_insert(applied_left)?;
            let applied_right = bank.term_apply_arg(&new_right, &skolem);
            new_right = bank.term_top_insert(applied_right)?;

            let left = bank.insert_no_props(&new_left, DerefType::Always)?;
            let right = bank.insert_no_props(&new_right, DerefType::Always)?;
            let new_literal = Eqn::alloc(left, right, bank, false)?;
            let mut new_literals = clause
                .literals()
                .copy_except_index(Some(literal_index), bank)?;
            new_literals.insert_first(new_literal);
            beta_normalize_eqn_list(bank, &mut new_literals)?;

            let mut new_clause = Clause::alloc(new_literals);
            set_ho_generation_proof_object(&mut new_clause, clause, None, DC_NEG_EXT, 0);
            store.insert(new_clause);
            generated += 1;
        }
    }
    Ok(generated)
}

fn compute_pos_ext(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    mode: ExtInferenceType,
) -> Result<i64, Diagnostic> {
    let mut generated = 0;
    for (literal_index, literal) in clause.literals().as_slice().iter().enumerate() {
        let literal_matches = literal.is_positive()
            && literal.is_equ_lit(bank)
            && (matches!(mode, ExtInferenceType::AllLits)
                || matches!(mode, ExtInferenceType::MaxLits) && literal.is_strictly_maximal());
        if !literal_matches {
            continue;
        }

        let mut left = literal.left().clone();
        let mut right = literal.right().clone();
        while let Some(shared_tail_var) = pos_ext_shared_trailing_free_var(&left, &right) {
            if pos_ext_tail_occurs_elsewhere(
                clause,
                literal_index,
                &left,
                &right,
                shared_tail_var.f_code(),
            ) {
                break;
            }

            left = term_drop_last_arg(bank, &left);
            right = term_drop_last_arg(bank, &right);
            if !left.is_free_var() {
                left = bank.term_top_insert(left)?;
            }
            if !right.is_free_var() {
                right = bank.term_top_insert(right)?;
            }

            let new_literal = Eqn::alloc(left.clone(), right.clone(), bank, true)?;
            let mut new_literals = clause
                .literals()
                .copy_except_index(Some(literal_index), bank)?;
            new_literals.insert_first(new_literal);

            let mut new_clause = Clause::alloc(new_literals);
            set_ho_generation_proof_object(&mut new_clause, clause, None, DC_POS_EXT, 0);
            store.insert(new_clause);
            generated += 1;
        }
    }
    Ok(generated)
}

fn pos_ext_shared_trailing_free_var(left: &Term, right: &Term) -> Option<Term> {
    if left.arity() == 0 || right.arity() == 0 || left.is_lambda() || right.is_lambda() {
        return None;
    }
    let left_tail = left.argument(left.arity() - 1)?;
    let right_tail = right.argument(right.arity() - 1)?;
    (left_tail == right_tail && left_tail.is_free_var()).then_some(left_tail)
}

fn pos_ext_tail_occurs_elsewhere(
    clause: &Clause,
    literal_index: usize,
    left: &Term,
    right: &Term,
    var_code: i64,
) -> bool {
    let occurs_in_left_prefix = (0..left.arity().saturating_sub(1)).any(|index| {
        left.argument(index)
            .is_some_and(|argument| term_has_f_code(&argument, var_code))
    });
    let occurs_in_right_prefix = (0..right.arity().saturating_sub(1)).any(|index| {
        right
            .argument(index)
            .is_some_and(|argument| term_has_f_code(&argument, var_code))
    });
    occurs_in_left_prefix
        || occurs_in_right_prefix
        || clause
            .literals()
            .as_slice()
            .iter()
            .enumerate()
            .any(|(index, literal)| {
                index != literal_index
                    && (term_has_f_code(literal.left(), var_code)
                        || term_has_f_code(literal.right(), var_code))
            })
}

fn term_drop_last_arg(bank: &mut TermBank, term: &Term) -> Term {
    assert!(
        term.arity() > 0,
        "term_drop_last_arg expects an applied term"
    );

    let term_type = term.type_().expect("applied term must have a type");
    let tail_type = term
        .argument(term.arity() - 1)
        .and_then(|tail| tail.type_())
        .expect("trailing argument must have a type");
    let mut result_type_args = Vec::with_capacity(type_get_max_arity(&term_type) + 2);
    result_type_args.push(tail_type);
    if term_type.is_arrow() {
        result_type_args.extend(term_type.args().iter().cloned());
    } else {
        result_type_args.push(term_type);
    }
    let result_type = bank
        .signature_mut()
        .type_bank_mut()
        .insert_type_shared(alloc_arrow_type(result_type_args));

    if term.is_phony_app() && term.arity() == 2 {
        let head = term
            .argument(0)
            .expect("phony application must have a head argument");
        debug_assert_eq!(head.type_(), Some(result_type));
        head
    } else {
        let prefix = Term::top_alloc(term.f_code(), term.arity() - 1);
        prefix.set_type(Some(result_type));
        for index in 0..term.arity() - 1 {
            prefix.set_argument(
                index,
                term.argument(index)
                    .expect("prefix argument must be initialized"),
            );
        }
        prefix
    }
}

fn compute_leibniz_elimination(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    limit: i32,
) -> Result<i64, Diagnostic> {
    if clause.proof_depth() > i64::from(limit) {
        return Ok(0);
    }

    let mut positive_vars = BTreeSet::new();
    let mut negative_vars = BTreeSet::new();
    for literal in clause.literals().as_slice() {
        if let Some(var) = leibniz_literal_head_var(literal, bank) {
            if literal.is_positive() {
                positive_vars.insert(var.f_code());
            } else {
                negative_vars.insert(var.f_code());
            }
        }
    }

    let mut generated = 0;
    for (literal_index, literal) in clause.literals().as_slice().iter().enumerate() {
        let Some(var) = leibniz_literal_head_var(literal, bank) else {
            continue;
        };
        let found_opposite = if literal.is_positive() {
            negative_vars.contains(&var.f_code())
        } else {
            positive_vars.contains(&var.f_code())
        };
        if !found_opposite {
            continue;
        }

        let applied = literal.left();
        assert!(
            applied.is_applied_free_var(),
            "Leibniz literal must have an applied free-variable left side"
        );
        for arg_index in 1..applied.arity() {
            let arg = applied.argument(arg_index).unwrap_or_else(|| {
                panic!("applied variable argument {arg_index} must be initialized")
            });
            if occur_check(&arg, &var) {
                continue;
            }

            let binding = leibniz_binding_for_arg(bank, literal, applied, arg_index, &arg)?;
            make_leibniz_instance(bank, clause, literal_index, &var, binding, store)?;
            generated += 1;
        }
    }

    Ok(generated)
}

fn leibniz_literal_head_var(literal: &Eqn, bank: &TermBank) -> Option<Term> {
    if literal.is_equ_lit(bank) || !literal.left().is_applied_free_var() {
        return None;
    }
    assert_eq!(
        literal.right(),
        bank.true_term(),
        "Leibniz predicate literal must use $true as right side"
    );
    literal.left().argument(0)
}

fn leibniz_binding_for_arg(
    bank: &mut TermBank,
    literal: &Eqn,
    applied: &Term,
    arg_index: usize,
    arg: &Term,
) -> Result<Term, Diagnostic> {
    let arg_type = arg
        .type_()
        .expect("Leibniz applied-variable argument must have a type");
    let db_index =
        i64::try_from(applied.arity() - arg_index - 1).expect("Leibniz DB index fits in FunCode");
    let db_var = bank.request_db_var(&arg_type, db_index);
    let encoded = Eqn::terms_tb_term_encode(
        bank,
        &db_var,
        arg,
        !literal.is_positive(),
        PatEqnDirection::Normal,
    )?;
    let binder_types = (1..applied.arity())
        .map(|index| {
            applied
                .argument(index)
                .and_then(|argument| argument.type_())
                .unwrap_or_else(|| panic!("Leibniz binder argument {index} must have a type"))
        })
        .collect::<Vec<_>>();
    close_with_type_prefix(bank, &binder_types, &encoded)
}

fn make_leibniz_instance(
    bank: &mut TermBank,
    clause: &Clause,
    literal_index: usize,
    var: &Term,
    binding: Term,
    store: &mut ClauseSet,
) -> Result<(), Diagnostic> {
    assert!(
        var.binding().is_none(),
        "Leibniz predicate variable must be unbound"
    );
    assert_eq!(
        var.type_(),
        binding.type_(),
        "Leibniz binding type must match predicate variable"
    );

    var.set_binding(Some(binding));
    let result = (|| {
        let mut new_literals = Vec::with_capacity(clause.literal_number().saturating_sub(1));
        for (index, literal) in clause.literals().as_slice().iter().enumerate() {
            if index != literal_index {
                new_literals.push(literal.copy_instantiated_ho(bank)?);
            }
        }
        let mut new_literals = EqnList::from_vec(new_literals);
        beta_normalize_eqn_list(bank, &mut new_literals)?;
        let _ = new_literals.remove_resolved(bank);
        let _ = new_literals.remove_duplicates(bank);

        let mut new_clause = Clause::alloc(new_literals);
        let _ = clause_normalize_equations(&mut new_clause, bank);
        set_ho_generation_proof_object(&mut new_clause, clause, None, DC_LEIBNIZ_ELIM, 1);
        store.insert(new_clause);
        Ok(())
    })();
    var.set_binding(None);
    result
}

fn compute_primitive_enumeration(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    mode: PrimEnumMode,
    limit: i32,
) -> Result<i64, Diagnostic> {
    if clause.proof_depth() > i64::from(limit) {
        return Ok(0);
    }

    bank.vars().set_v_counts_to_used();
    bank.vars().set_fresh_count_to_used();
    let mut generated = 0;
    let mut processed_vars = BTreeSet::new();
    for literal in clause.literals().as_slice() {
        if !literal.left().type_().is_some_and(|type_| type_.is_bool()) {
            continue;
        }
        for term in [literal.left(), literal.right()] {
            if !term.is_applied_free_var() {
                continue;
            }
            let var = term
                .argument(0)
                .expect("applied primitive-enumeration variable must have a head");
            if processed_vars.insert(var.f_code()) {
                generated += prim_enum_var(bank, clause, store, mode, term)?;
            }
        }
    }

    Ok(generated)
}

#[expect(
    clippy::too_many_lines,
    reason = "mirrors C prim_enum_var mode dispatch for compatibility"
)]
fn prim_enum_var(
    bank: &mut TermBank,
    clause: &Clause,
    store: &mut ClauseSet,
    mode: PrimEnumMode,
    app_var: &Term,
) -> Result<i64, Diagnostic> {
    assert!(
        app_var.is_applied_free_var(),
        "primitive enumeration expects an applied free variable"
    );
    assert!(
        app_var.type_().is_some_and(|type_| type_.is_bool()),
        "primitive enumeration expects a Boolean application"
    );

    let head = app_var
        .argument(0)
        .expect("applied primitive-enumeration variable must have a head");
    let sig = bank.signature();
    let not_code = sig.not_code();
    let and_code = sig.and_code();
    let or_code = sig.or_code();
    let eqn_code = sig.eqn_code();
    let neqn_code = sig.neqn_code();
    let equiv_code = sig.equiv_code();
    let xor_code = sig.xor_code();
    let qall_code = sig.qall_code();
    let qex_code = sig.qex_code();

    let mut generated = 0;
    if matches!(mode, PrimEnumMode::Neg | PrimEnumMode::Full) {
        let pattern = fresh_pattern(bank, app_var)?;
        let matrix = tformula_fcode_alloc(bank, not_code, pattern, None)?;
        let target = close_for_appvar(bank, app_var, &matrix)?;
        make_prim_enum_instance(bank, clause, &head, target, store)?;
        generated += 1;
    }
    if matches!(mode, PrimEnumMode::And | PrimEnumMode::Full) {
        let left = fresh_pattern(bank, app_var)?;
        let right = fresh_pattern(bank, app_var)?;
        let matrix = tformula_fcode_alloc(bank, and_code, left, Some(right))?;
        let target = close_for_appvar(bank, app_var, &matrix)?;
        make_prim_enum_instance(bank, clause, &head, target, store)?;
        generated += 1;
    }
    if matches!(mode, PrimEnumMode::Or | PrimEnumMode::Full) {
        let left = fresh_pattern(bank, app_var)?;
        let right = fresh_pattern(bank, app_var)?;
        let matrix = tformula_fcode_alloc(bank, or_code, left, Some(right))?;
        let target = close_for_appvar(bank, app_var, &matrix)?;
        make_prim_enum_instance(bank, clause, &head, target, store)?;
        generated += 1;
    }
    if matches!(mode, PrimEnumMode::Eq | PrimEnumMode::Full) {
        for return_type in primitive_enum_return_types(app_var) {
            let code = if return_type.is_bool() {
                equiv_code
            } else {
                eqn_code
            };
            let left = fresh_pattern_w_ty(bank, app_var, &return_type)?;
            let right = fresh_pattern_w_ty(bank, app_var, &return_type)?;
            let matrix = tformula_fcode_alloc(bank, code, left, Some(right))?;
            let target = close_for_appvar(bank, app_var, &matrix)?;
            make_prim_enum_instance(bank, clause, &head, target, store)?;
            generated += 1;
        }
    }

    let target = close_for_appvar(bank, app_var, &bank.true_term().clone())?;
    make_prim_enum_instance(bank, clause, &head, target, store)?;
    let target = close_for_appvar(bank, app_var, &bank.false_term().clone())?;
    make_prim_enum_instance(bank, clause, &head, target, store)?;
    generated += 2;

    if mode == PrimEnumMode::Pragmatic {
        for first in 1..app_var.arity() {
            for second in first + 1..app_var.arity() {
                let first_arg = app_var
                    .argument(first)
                    .expect("primitive-enumeration first argument must be initialized");
                let second_arg = app_var
                    .argument(second)
                    .expect("primitive-enumeration second argument must be initialized");
                let first_type = first_arg
                    .type_()
                    .expect("primitive-enumeration first argument must have a type");
                let second_type = second_arg
                    .type_()
                    .expect("primitive-enumeration second argument must have a type");
                if first_type != second_type {
                    continue;
                }
                let first_db = bank.request_db_var(
                    &first_type,
                    i64::try_from(app_var.arity() - first - 1)
                        .expect("primitive-enumeration DB index fits in FunCode"),
                );
                let second_db = bank.request_db_var(
                    &first_type,
                    i64::try_from(app_var.arity() - second - 1)
                        .expect("primitive-enumeration DB index fits in FunCode"),
                );
                let pos_code = if first_type.is_bool() {
                    equiv_code
                } else {
                    eqn_code
                };
                let neg_code = if first_type.is_bool() {
                    xor_code
                } else {
                    neqn_code
                };
                let matrix = tformula_fcode_alloc(
                    bank,
                    pos_code,
                    first_db.clone(),
                    Some(second_db.clone()),
                )?;
                let target = close_for_appvar(bank, app_var, &matrix)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                let matrix = tformula_fcode_alloc(
                    bank,
                    neg_code,
                    first_db.clone(),
                    Some(second_db.clone()),
                )?;
                let target = close_for_appvar(bank, app_var, &matrix)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                generated += 2;

                if type_is_predicate(&first_type) {
                    let first_projection = apply_pattern_vars(bank, &first_db, app_var)?;
                    let second_projection = apply_pattern_vars(bank, &second_db, app_var)?;
                    let matrix = tformula_fcode_alloc(
                        bank,
                        and_code,
                        first_projection.clone(),
                        Some(second_projection.clone()),
                    )?;
                    let target = close_for_appvar(bank, app_var, &matrix)?;
                    make_prim_enum_instance(bank, clause, &head, target, store)?;
                    let matrix = tformula_fcode_alloc(
                        bank,
                        or_code,
                        first_projection,
                        Some(second_projection),
                    )?;
                    let target = close_for_appvar(bank, app_var, &matrix)?;
                    make_prim_enum_instance(bank, clause, &head, target, store)?;
                    generated += 2;
                }
            }
        }
    }

    if matches!(mode, PrimEnumMode::LogSymbol | PrimEnumMode::Pragmatic) {
        let var_type = head
            .type_()
            .expect("primitive-enumeration head variable must have a type");
        if var_type.is_arrow() {
            if var_type.arity() == 2 && var_type.args()[0].is_bool() && var_type.args()[1].is_bool()
            {
                let target = logical_symbol_as_term(bank, not_code, &var_type)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                generated += 1;
            } else if var_type.arity() == 3
                && var_type.args()[0].is_bool()
                && var_type.args()[1].is_bool()
                && var_type.args()[2].is_bool()
            {
                let target = logical_symbol_as_term(bank, and_code, &var_type)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                let target = logical_symbol_as_term(bank, or_code, &var_type)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                generated += 2;
            }

            if var_type.arity() == 3
                && var_type.args()[0] == var_type.args()[1]
                && var_type.args()[2].is_bool()
            {
                let pos_code = if var_type.args()[0].is_bool() {
                    equiv_code
                } else {
                    eqn_code
                };
                let neg_code = if var_type.args()[0].is_bool() {
                    xor_code
                } else {
                    neqn_code
                };
                let target = logical_symbol_as_term(bank, pos_code, &var_type)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                let target = logical_symbol_as_term(bank, neg_code, &var_type)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                generated += 2;
            }

            if var_type.arity() == 2
                && var_type.args()[1].is_bool()
                && var_type.args()[0].is_arrow()
                && var_type.args()[0].arity() == 2
                && var_type.args()[0].args()[1].is_bool()
            {
                let predicate_type = var_type.args()[0].clone();
                let db_var = bank.request_db_var(&predicate_type, 0);
                let matrix = quantified_matrix(bank, qall_code, &db_var)?;
                let target = close_with_db_var(bank, &predicate_type, &matrix)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                let matrix = quantified_matrix(bank, qex_code, &db_var)?;
                let target = close_with_db_var(bank, &predicate_type, &matrix)?;
                make_prim_enum_instance(bank, clause, &head, target, store)?;
                generated += 2;
            }
        }
    }

    Ok(generated)
}

fn fresh_pattern(bank: &mut TermBank, app_var: &Term) -> Result<Term, Diagnostic> {
    let return_type = app_var
        .type_()
        .expect("primitive-enumeration application must have a type");
    fresh_pattern_w_ty(bank, app_var, &return_type)
}

fn fresh_pattern_w_ty(
    bank: &mut TermBank,
    app_var: &Term,
    return_type: &Type,
) -> Result<Term, Diagnostic> {
    assert!(
        app_var.is_applied_free_var(),
        "fresh primitive pattern expects an applied free variable"
    );
    let arg_types = appvar_arg_types(app_var);
    let fresh_type = bank
        .signature_mut()
        .type_bank_mut()
        .insert_type_shared(arrow_type_flattened(&arg_types, return_type));
    let fresh_var = bank.vars().get_fresh_var(&fresh_type);
    let fresh_var = bank.insert(&fresh_var, DerefType::Never)?;

    let applied = Term::top_copy_without_args(app_var);
    applied.set_type(Some(return_type.clone()));
    applied.set_argument(0, fresh_var);
    for index in 1..app_var.arity() {
        let arg = app_var
            .argument(index)
            .expect("primitive-enumeration application argument must be initialized");
        let arg_type = arg
            .type_()
            .expect("primitive-enumeration application argument must have a type");
        let db_index = i64::try_from(app_var.arity() - index - 1)
            .expect("primitive-enumeration DB index fits in FunCode");
        applied.set_argument(index, bank.request_db_var(&arg_type, db_index));
    }

    bank.term_top_insert(applied)
}

fn close_for_appvar(
    bank: &mut TermBank,
    app_var: &Term,
    matrix: &Term,
) -> Result<Term, Diagnostic> {
    let arg_types = appvar_arg_types(app_var);
    close_with_type_prefix(bank, &arg_types, matrix)
}

fn apply_pattern_vars(
    bank: &mut TermBank,
    head: &Term,
    app_var: &Term,
) -> Result<Term, Diagnostic> {
    assert!(
        app_var.is_applied_free_var(),
        "primitive projection expects an applied free variable"
    );
    let head_type = head
        .type_()
        .expect("primitive projection head must have a type");
    if !head_type.is_arrow() {
        return Ok(head.clone());
    }

    let arg_types = appvar_arg_types(app_var);
    let mut db_args = Vec::with_capacity(app_var.arity().saturating_sub(1));
    for index in 1..app_var.arity() {
        let arg = app_var
            .argument(index)
            .expect("primitive projection application argument must be initialized");
        let arg_type = arg
            .type_()
            .expect("primitive projection application argument must have a type");
        let db_index = i64::try_from(app_var.arity() - index - 1)
            .expect("primitive projection DB index fits in FunCode");
        db_args.push(bank.request_db_var(&arg_type, db_index));
    }

    let mut head_args = Vec::with_capacity(head_type.arity().saturating_sub(1));
    for target_type in head_type.args().iter().take(head_type.arity() - 1) {
        let fresh_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(arrow_type_flattened(&arg_types, target_type));
        let fresh_var = bank.vars().get_fresh_var(&fresh_type);
        let fresh_var = bank.insert(&fresh_var, DerefType::Never)?;
        head_args.push(apply_terms(bank, &fresh_var, &db_args)?);
    }

    apply_terms(bank, head, &head_args)
}

fn appvar_arg_types(app_var: &Term) -> Vec<Type> {
    (1..app_var.arity())
        .map(|index| {
            app_var
                .argument(index)
                .and_then(|arg| arg.type_())
                .unwrap_or_else(|| {
                    panic!("primitive-enumeration argument {index} must be initialized and typed")
                })
        })
        .collect()
}

fn primitive_enum_return_types(app_var: &Term) -> Vec<Type> {
    let mut return_types = Vec::new();
    for index in 1..app_var.arity() {
        let argument_type = app_var
            .argument(index)
            .and_then(|arg| arg.type_())
            .unwrap_or_else(|| {
                panic!("primitive-enumeration argument {index} must be initialized and typed")
            });
        if !return_types.iter().any(|seen| seen == &argument_type) {
            return_types.push(argument_type);
        }
    }
    return_types.sort_by(|left, right| type_identity_cmp(left, right).cmp(&0));
    return_types
}

fn logical_symbol_as_term(
    bank: &mut TermBank,
    f_code: i64,
    type_: &Type,
) -> Result<Term, Diagnostic> {
    let term = Term::top_alloc(f_code, 0);
    term.set_type(Some(type_.clone()));
    bank.term_top_insert(term)
}

fn quantified_matrix(
    bank: &mut TermBank,
    f_code: i64,
    predicate: &Term,
) -> Result<Term, Diagnostic> {
    let term = Term::top_alloc(f_code, 1);
    term.set_type(Some(bank.signature().type_bank().bool_type()));
    term.set_argument(0, predicate.clone());
    bank.term_top_insert(term)
}

fn make_prim_enum_instance(
    bank: &mut TermBank,
    clause: &Clause,
    var: &Term,
    target: Term,
    store: &mut ClauseSet,
) -> Result<(), Diagnostic> {
    assert!(
        var.binding().is_none(),
        "primitive-enumeration variable must be unbound"
    );
    assert_eq!(
        var.type_(),
        target.type_(),
        "primitive-enumeration target type must match variable type"
    );

    var.set_binding(Some(target));
    let result = (|| {
        let mut new_literals = Vec::with_capacity(clause.literal_number());
        for literal in clause.literals().as_slice() {
            new_literals.push(literal.copy_instantiated_ho(bank)?);
        }
        let mut new_literals = EqnList::from_vec(new_literals);
        beta_normalize_eqn_list(bank, &mut new_literals)?;
        let _ = new_literals.remove_resolved(bank);
        let _ = new_literals.remove_duplicates(bank);

        let mut new_clause = Clause::alloc(new_literals);
        let _ = clause_normalize_equations(&mut new_clause, bank);
        set_ho_generation_proof_object(&mut new_clause, clause, None, DC_PRIM_ENUM, 1);
        let _ = clause_boolean_simplification(&mut new_clause, bank)?;
        store.insert(new_clause);
        Ok(())
    })();
    var.set_binding(None);
    result
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ChoiceTrigger {
    Defined {
        choice_code: i64,
        predicate: Term,
    },
    AppliedVariable {
        choice_variable: Term,
        predicate: Term,
    },
}

fn instantiate_choice_clauses(
    bank: &mut TermBank,
    renamed_clause: &Clause,
    clause: &Clause,
    store: &mut ClauseSet,
    archive: &mut ClauseSet,
    choice_symbols: &mut BTreeMap<i64, Clause>,
    limit: i32,
) -> Result<i64, Diagnostic> {
    if clause.proof_depth() > i64::from(limit) {
        return Ok(0);
    }

    let mut generated = 0;
    let mut triggers = Vec::new();
    for literal in renamed_clause.literals().as_slice() {
        debug_assert!(triggers.is_empty());
        find_choice_triggers(choice_symbols, &mut triggers, literal.left());
        find_choice_triggers(choice_symbols, &mut triggers, literal.right());
        while let Some(trigger) = triggers.pop() {
            match trigger {
                ChoiceTrigger::Defined {
                    choice_code,
                    predicate,
                } => {
                    generated += instantiate_choice(
                        bank,
                        store,
                        choice_symbols,
                        clause,
                        choice_code,
                        &predicate,
                    )?;
                }
                ChoiceTrigger::AppliedVariable {
                    choice_variable,
                    predicate,
                } => {
                    let choice_type = choice_variable
                        .type_()
                        .expect("choice variable trigger must have a type");
                    let mut choice_codes =
                        choice_codes_for_type(bank, choice_symbols, &choice_type);
                    if choice_codes.is_empty() {
                        choice_codes.push(make_new_choice(
                            bank,
                            archive,
                            choice_symbols,
                            &choice_type,
                        )?);
                    }
                    while let Some(choice_code) = choice_codes.pop() {
                        generated += instantiate_choice(
                            bank,
                            store,
                            choice_symbols,
                            clause,
                            choice_code,
                            &predicate,
                        )?;
                    }
                }
            }
        }
    }

    Ok(generated)
}

fn find_choice_triggers(
    choice_symbols: &BTreeMap<i64, Clause>,
    triggers: &mut Vec<ChoiceTrigger>,
    term: &Term,
) {
    if term.is_db_var() || term.is_lambda() {
        return;
    }

    if term.arity() == 1
        && choice_symbols.contains_key(&term.f_code())
        && term
            .argument(0)
            .is_some_and(|argument| !argument.is_free_var())
    {
        triggers.push(ChoiceTrigger::Defined {
            choice_code: term.f_code(),
            predicate: term
                .argument(0)
                .expect("defined choice trigger must have an argument"),
        });
    } else if term.is_applied_free_var() && term.arity() == 2 {
        let choice_variable = term
            .argument(0)
            .expect("applied choice-variable trigger must have a head");
        if choice_variable
            .type_()
            .is_some_and(|type_| is_choice_type(&type_))
        {
            triggers.push(ChoiceTrigger::AppliedVariable {
                choice_variable,
                predicate: term
                    .argument(1)
                    .expect("applied choice-variable trigger must have a predicate"),
            });
        }
    } else if !term.is_free_var() && term.arity() != 0 {
        for argument in term.argument_clones() {
            let argument = argument.expect("choice trigger scan term argument is uninitialized");
            find_choice_triggers(choice_symbols, triggers, &argument);
        }
    }
}

fn choice_codes_for_type(
    bank: &TermBank,
    choice_symbols: &BTreeMap<i64, Clause>,
    choice_type: &Type,
) -> Vec<i64> {
    choice_symbols
        .keys()
        .copied()
        .filter(|choice_code| {
            bank.signature()
                .get_type(*choice_code)
                .is_some_and(|candidate| candidate == choice_type)
        })
        .collect()
}

fn make_new_choice(
    bank: &mut TermBank,
    archive: &mut ClauseSet,
    choice_symbols: &mut BTreeMap<i64, Clause>,
    choice_type: &Type,
) -> Result<i64, Diagnostic> {
    assert!(
        is_choice_type(choice_type),
        "fresh choice symbols require a choice type"
    );

    let choice_const = bank.alloc_new_skolem(&[], Some(choice_type))?;
    let predicate_type = choice_type
        .args()
        .first()
        .expect("choice type must have a predicate argument")
        .clone();
    assert!(
        predicate_type.is_arrow() && predicate_type.arity() == 2,
        "choice predicate argument must be unary"
    );
    let witness_type = predicate_type.args()[0].clone();

    let predicate_var = bank.vars().get_fresh_var(&predicate_type);
    let predicate_var = bank.insert(&predicate_var, DerefType::Never)?;
    let choice_applied = apply_terms(bank, &choice_const, std::slice::from_ref(&predicate_var))?;
    let positive_atom = apply_terms(bank, &predicate_var, std::slice::from_ref(&choice_applied))?;

    let witness_var = bank.vars().get_fresh_var(&witness_type);
    let witness_var = bank.insert(&witness_var, DerefType::Never)?;
    let negative_atom = apply_terms(bank, &predicate_var, std::slice::from_ref(&witness_var))?;

    let true_term = bank.true_term().clone();
    let negative_literal = Eqn::alloc(negative_atom, true_term.clone(), bank, false)?;
    let positive_literal = Eqn::alloc(positive_atom, true_term, bank, true)?;
    let mut choice_axiom =
        Clause::alloc(EqnList::from_vec(vec![negative_literal, positive_literal]));
    clause_push_derivation(&mut choice_axiom, DC_CHOICE_AX, None, None);

    let choice_code = choice_const.f_code();
    assert!(
        !choice_symbols.contains_key(&choice_code),
        "fresh choice symbol must not already be registered"
    );
    archive.insert(choice_axiom.clone());
    choice_symbols.insert(choice_code, choice_axiom);
    Ok(choice_code)
}

fn instantiate_choice(
    bank: &mut TermBank,
    store: &mut ClauseSet,
    choice_symbols: &BTreeMap<i64, Clause>,
    clause: &Clause,
    choice_code: i64,
    predicate: &Term,
) -> Result<i64, Diagnostic> {
    let Some(choice_definition) = choice_symbols.get(&choice_code).cloned() else {
        return Ok(0);
    };
    make_choice_instance(
        bank,
        store,
        clause,
        &choice_definition,
        choice_code,
        predicate,
    )?;

    let negated_predicate = negated_eta_expanded_predicate(bank, predicate)?;
    make_choice_instance(
        bank,
        store,
        clause,
        &choice_definition,
        choice_code,
        &negated_predicate,
    )?;
    Ok(2)
}

fn make_choice_instance(
    bank: &mut TermBank,
    store: &mut ClauseSet,
    clause: &Clause,
    choice_definition: &Clause,
    choice_code: i64,
    predicate: &Term,
) -> Result<(), Diagnostic> {
    let predicate_type = predicate.type_().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "choice-instantiation trigger predicate must be typed",
        )
    })?;
    assert!(
        type_is_predicate(&predicate_type),
        "choice-instantiation trigger must be a predicate"
    );
    assert!(
        choice_symbols_match_type(bank, choice_code, &predicate_type),
        "choice symbol argument type must match trigger predicate"
    );

    let choice_var = choice_definition_predicate_var(choice_definition);
    assert!(
        choice_var.binding().is_none(),
        "choice definition predicate variable must be unbound"
    );
    assert_eq!(
        choice_var.type_(),
        Some(predicate_type),
        "choice definition predicate type must match trigger predicate"
    );

    choice_var.set_binding(Some(predicate.clone()));
    let result = (|| {
        let mut new_literals = Vec::with_capacity(choice_definition.literal_number());
        for literal in choice_definition.literals().as_slice() {
            new_literals.push(literal.copy_instantiated_ho(bank)?);
        }
        let mut new_literals = EqnList::from_vec(new_literals);
        beta_normalize_eqn_list(bank, &mut new_literals)?;
        let _ = new_literals.remove_resolved(bank);
        let _ = new_literals.remove_duplicates(bank);

        let mut new_clause = Clause::alloc(new_literals);
        let _ = clause_normalize_equations(&mut new_clause, bank);
        set_ho_generation_proof_object(
            &mut new_clause,
            clause,
            Some(choice_definition),
            DC_CHOICE_INST,
            1,
        );
        let _ = clause_boolean_simplification(&mut new_clause, bank)?;
        store.insert(new_clause);
        Ok(())
    })();
    choice_var.set_binding(None);
    result
}

fn choice_symbols_match_type(bank: &TermBank, choice_code: i64, predicate_type: &Type) -> bool {
    let Some(choice_type) = bank.signature().get_type(choice_code) else {
        return false;
    };
    choice_type
        .args()
        .first()
        .is_some_and(|choice_predicate_type| choice_predicate_type == predicate_type)
}

fn choice_definition_predicate_var(choice_definition: &Clause) -> Term {
    let negative_literal = choice_definition
        .literals()
        .as_slice()
        .iter()
        .find(|literal| literal.is_negative())
        .expect("choice definition must have a negative literal");
    let left = negative_literal.left();
    assert!(
        left.is_applied_free_var(),
        "choice definition negative literal must be an applied variable"
    );
    left.argument(0)
        .expect("choice definition predicate variable must be initialized")
}

fn negated_eta_expanded_predicate(
    bank: &mut TermBank,
    predicate: &Term,
) -> Result<Term, Diagnostic> {
    let predicate_type = predicate.type_().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "choice-instantiation negated trigger must be typed",
        )
    })?;
    assert!(
        predicate_type.is_arrow() && type_is_predicate(&predicate_type),
        "choice-instantiation negated trigger must be an arrow predicate"
    );

    let argument_types = predicate_type.args()[..predicate_type.arity() - 1].to_vec();
    let mut db_args = Vec::with_capacity(argument_types.len());
    for (index, argument_type) in argument_types.iter().enumerate() {
        let db_index = i64::try_from(argument_types.len() - index - 1)
            .expect("choice eta-expansion DB index fits in FunCode");
        db_args.push(bank.request_db_var(argument_type, db_index));
    }
    let applied = apply_terms(bank, predicate, &db_args)?;
    let not_code = bank.signature().not_code();
    let negated = tformula_fcode_alloc(bank, not_code, applied, None)?;
    close_with_type_prefix(bank, &argument_types, &negated)
}

fn fresh_var_bank_for_arg_cong_clause(bank: &TermBank, clause: &Clause) -> VarBank {
    let mut variables: BTreeMap<usize, Term> = BTreeMap::new();
    let _ = clause.collect_variables(&mut variables);
    VarBank::fresh_normalization_bank(
        bank.signature().type_bank(),
        bank.vars(),
        variables.values(),
    )
}

fn fresh_var_bank_for_ext_sup_clauses(bank: &TermBank, first: &Clause, second: &Clause) -> VarBank {
    let mut variables: BTreeMap<usize, Term> = BTreeMap::new();
    let _ = first.collect_variables(&mut variables);
    let _ = second.collect_variables(&mut variables);
    VarBank::fresh_normalization_bank(
        bank.signature().type_bank(),
        bank.vars(),
        variables.values(),
    )
}

fn beta_normalize_eqn_list(bank: &mut TermBank, literals: &mut EqnList) -> Result<(), Diagnostic> {
    for literal in literals.as_mut_slice() {
        let left = beta_normalize_db(bank, literal.left())?;
        let right = beta_normalize_db(bank, literal.right())?;
        literal.set_left_raw(left);
        literal.set_right_raw(right);
    }
    Ok(())
}

fn set_ho_generation_proof_object(
    new_clause: &mut Clause,
    orig_clause: &Clause,
    parent2: Option<&Clause>,
    derivation_code: i64,
    depth_incr: i64,
) {
    let parent2_depth = parent2.map_or(0, Clause::proof_depth);
    let parent2_size = parent2.map_or(0, Clause::proof_size);
    new_clause.set_proof_depth(orig_clause.proof_depth().max(parent2_depth) + depth_incr);
    new_clause.set_proof_size(orig_clause.proof_size() + parent2_size + 1);
    new_clause.set_tptp_type(orig_clause.query_tptp_type());
    new_clause.set_prop(orig_clause.give_props(CP_IS_SOS));
    clause_push_derivation(new_clause, derivation_code, Some(orig_clause), parent2);
}

fn proof_state_generate_new_clauses_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    problem_type: ProblemType,
    indices: Option<&GlobalIndices>,
    doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<GenerateNewClausesOutcome, Diagnostic> {
    let mut renamed_clause = clause.copy_disjoint(state.terms_mut())?;
    renamed_clause.set_ident(clause.ident());
    proof_state_generate_new_clauses_with_disjoint_copy_impl(
        state,
        control,
        &renamed_clause,
        clause,
        problem_type,
        indices,
        doc_context,
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "C-compatible selected-clause generation keeps generator order and optional proof docs together"
)]
fn proof_state_generate_new_clauses_with_disjoint_copy_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    renamed_clause: &Clause,
    clause: &Clause,
    problem_type: ProblemType,
    indices: Option<&GlobalIndices>,
    mut doc_context: Option<(&mut W, &mut ProofDocSession)>,
) -> Result<GenerateNewClausesOutcome, Diagnostic> {
    state.terms().vars().set_v_counts_to_used();
    let _ = compute_ho_inferences(
        state,
        control,
        renamed_clause,
        clause,
        problem_type,
        indices,
    )?;
    let enable_eq_factoring = control.heuristic_parms().enable_eq_factoring;
    let enable_neg_unit_paramod = control.heuristic_parms().enable_neg_unit_paramod;
    let diseq_decomposition = control.heuristic_parms().diseq_decomposition;
    let diseq_decomp_maxarity = control.heuristic_parms().diseq_decomp_maxarity;
    let should_paramodulate = proof_state_generation_runs_paramodulation(control, clause);
    let pm_type = if should_paramodulate {
        Some(proof_state_unindexed_paramodulation_type(
            control.heuristic_parms().pm_type,
        ))
    } else {
        None
    };
    let source_for_paramod = should_paramodulate.then_some(renamed_clause);

    let result = (|| {
        let mut outcome = GenerateNewClausesOutcome::default();
        let needs_ocb = enable_eq_factoring || should_paramodulate;
        let mut ocb = if needs_ocb {
            Some(control.ocb.as_mut().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "selected-clause generation requires initialized proof-control ordering",
                )
            })?)
        } else {
            None
        };
        let (terms, mut generation) = state.terms_and_generation_context_mut();

        if enable_eq_factoring {
            let Some(ocb) = ocb.as_mut() else {
                return Err(Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "selected-clause generation requires initialized proof-control ordering",
                ));
            };
            let count = if let Some((output, session)) = doc_context.as_mut() {
                compute_all_equality_factors_with_fresh_vars_and_docs(
                    &mut **output,
                    session,
                    terms,
                    ocb,
                    clause,
                    generation.tmp_store,
                    generation.fresh_vars,
                )?
            } else {
                compute_all_equality_factors_with_fresh_vars(
                    terms,
                    ocb,
                    clause,
                    generation.tmp_store,
                    generation.fresh_vars,
                )?
            };
            outcome.equality_factors = i64_to_u64_saturating(count);
        }

        let count = if let Some((output, session)) = doc_context.as_mut() {
            compute_all_eqn_resolvents_with_fresh_vars_and_docs(
                &mut **output,
                session,
                terms,
                clause,
                generation.tmp_store,
                EQ_RES_ON_MAXIMAL_LITERALS_ONLY,
                generation.fresh_vars,
            )?
        } else {
            compute_all_eqn_resolvents_with_fresh_vars(
                terms,
                clause,
                generation.tmp_store,
                EQ_RES_ON_MAXIMAL_LITERALS_ONLY,
                generation.fresh_vars,
            )?
        };
        outcome.equality_resolvents = i64_to_u64_saturating(count);

        let count = compute_dis_eq_decompositions(
            terms,
            clause,
            generation.tmp_store,
            diseq_decomposition,
            diseq_decomp_maxarity,
        )?;
        outcome.disequality_decompositions = i64_to_u64_saturating(count);

        if let (Some(pm_type), Some(source_for_paramod)) = (pm_type, source_for_paramod.as_ref()) {
            let Some(ocb) = ocb.as_mut() else {
                return Err(Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "selected-clause generation requires initialized proof-control ordering",
                ));
            };
            let _timer = crate::basics::perf_counters::start(
                crate::basics::perf_counters::PerfCounter::ParamodTimer,
            );
            outcome.paramodulants = compute_selected_paramodulants(
                terms,
                ocb,
                source_for_paramod,
                clause,
                &mut generation,
                enable_neg_unit_paramod,
                pm_type,
                indices,
                &mut doc_context,
            )?;
        }

        Ok(outcome)
    })();

    let outcome = result?;
    let statistics = state.statistics_mut();
    statistics.factor_count += outcome.equality_factors;
    statistics.resolv_count += outcome.equality_resolvents;
    statistics.disequ_deco_count += outcome.disequality_decompositions;
    statistics.paramod_count += outcome.paramodulants;
    Ok(outcome)
}

fn proof_state_generation_runs_paramodulation(control: &ProofControl, clause: &Clause) -> bool {
    !clause.query_prop(CP_NO_GENERATION)
        && (control.heuristic_parms().enable_neg_unit_paramod
            || !clause.is_unit()
            || !clause.is_negative())
}

fn proof_state_unindexed_paramodulation_type(
    pm_type: HcbParamodulationType,
) -> ClauseParamodulationType {
    match pm_type {
        HcbParamodulationType::Plain => ClauseParamodulationType::Plain,
        HcbParamodulationType::Sim => ClauseParamodulationType::Simultaneous,
        HcbParamodulationType::OrientedSim => ClauseParamodulationType::OrientedSimultaneous,
        HcbParamodulationType::DecreasingSim => ClauseParamodulationType::DecreasingSimultaneous,
        HcbParamodulationType::SizeDecreasingSim => {
            ClauseParamodulationType::SizeDecreasingSimultaneous
        }
        HcbParamodulationType::SuperSim => ClauseParamodulationType::SuperSimultaneous,
        HcbParamodulationType::OrientedSuperSim => {
            ClauseParamodulationType::OrientedSuperSimultaneous
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "selected-clause generator keeps term bank, store context, and optional global indexes explicit"
)]
fn compute_selected_paramodulants(
    terms: &mut TermBank,
    ocb: &mut OrderControlBlock,
    source_for_paramod: &Clause,
    parent_alias: &Clause,
    generation: &mut ProofStateGenerationContext<'_>,
    enable_neg_unit_paramod: bool,
    pm_type: ClauseParamodulationType,
    indices: Option<&GlobalIndices>,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    if let Some((into_index, negp_index, from_index)) =
        indices.and_then(GlobalIndices::pm_paramodulation_indexes)
    {
        let count = if let Some((output, session)) = doc_context.as_mut() {
            compute_all_paramodulants_indexed_with_docs(
                &mut **output,
                session,
                terms,
                ocb,
                source_for_paramod,
                parent_alias,
                into_index,
                negp_index,
                from_index,
                generation.tmp_store,
                pm_type,
            )?
        } else {
            compute_all_paramodulants_indexed(
                terms,
                ocb,
                source_for_paramod,
                parent_alias,
                into_index,
                negp_index,
                from_index,
                generation.tmp_store,
                pm_type,
            )?
        };
        Ok(i64_to_u64_saturating(count))
    } else {
        compute_unindexed_selected_paramodulants(
            terms,
            ocb,
            source_for_paramod,
            parent_alias,
            generation,
            enable_neg_unit_paramod,
            pm_type,
            doc_context,
        )
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "selected-clause paramodulation keeps source, partner stores, strategy gates, and optional docs explicit"
)]
fn compute_unindexed_selected_paramodulants(
    terms: &mut TermBank,
    ocb: &mut OrderControlBlock,
    source_for_paramod: &Clause,
    parent_alias: &Clause,
    generation: &mut ProofStateGenerationContext<'_>,
    enable_neg_unit_paramod: bool,
    pm_type: ClauseParamodulationType,
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    let tmp_store = &mut *generation.tmp_store;
    let mut count = compute_all_paramodulants_maybe_docs(
        doc_context,
        terms,
        ocb,
        source_for_paramod,
        parent_alias,
        generation.processed_pos_rules,
        tmp_store,
        pm_type,
    )?;
    count += compute_all_paramodulants_maybe_docs(
        doc_context,
        terms,
        ocb,
        source_for_paramod,
        parent_alias,
        generation.processed_pos_eqns,
        tmp_store,
        pm_type,
    )?;
    if enable_neg_unit_paramod && !parent_alias.is_negative() {
        count += compute_all_paramodulants_maybe_docs(
            doc_context,
            terms,
            ocb,
            source_for_paramod,
            parent_alias,
            generation.processed_neg_units,
            tmp_store,
            pm_type,
        )?;
    }
    count += compute_all_paramodulants_maybe_docs(
        doc_context,
        terms,
        ocb,
        source_for_paramod,
        parent_alias,
        generation.processed_non_units,
        tmp_store,
        pm_type,
    )?;
    Ok(i64_to_u64_saturating(count))
}

#[expect(
    clippy::too_many_arguments,
    reason = "selected-clause paramodulation keeps source, parent alias, partner set, and optional docs explicit"
)]
fn compute_all_paramodulants_maybe_docs(
    doc_context: &mut Option<(&mut impl fmt::Write, &mut ProofDocSession)>,
    terms: &mut TermBank,
    ocb: &mut OrderControlBlock,
    source_for_paramod: &Clause,
    parent_alias: &Clause,
    with_set: &ClauseSet,
    tmp_store: &mut ClauseSet,
    pm_type: ClauseParamodulationType,
) -> Result<i64, Diagnostic> {
    if let Some((output, session)) = doc_context.as_mut() {
        compute_all_paramodulants_with_docs(
            &mut **output,
            session,
            terms,
            ocb,
            source_for_paramod,
            parent_alias,
            with_set,
            tmp_store,
            pm_type,
        )
    } else {
        compute_all_paramodulants(
            terms,
            ocb,
            source_for_paramod,
            parent_alias,
            with_set,
            tmp_store,
            pm_type,
        )
    }
}

/// Processes one selected clause through the currently ported C `ProcessClause`.
///
/// The wrapper can run C `NoGeneration` without selected-clause generation, and
/// otherwise delegates to the staged generator helper. Backward simplification
/// can still put simplified processed clauses into `tmp_store`, and those are
/// routed through the existing `insert_new_clauses` path together with any
/// generated clauses.
///
/// # Errors
///
/// Returns diagnostics from selection, contraction, answer-literal evaluation,
/// replacement inferences, backward simplification, processed insertion, or
/// generated-clause reinsertion.
pub fn proof_state_process_clause(
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    proof_state_process_clause_impl::<String>(state, control, answer_limit, None, None, None, None)
}

/// Processes one selected clause while emitting represented C proof output.
///
/// This includes C's selected-clause `check_ac_status` `OutputLevel` text,
/// `OutputLevel` 1 selected-clause text, the target-level 6 `new_given`
/// proof-documentation quote, plus represented documentation from the final
/// `insert_new_clauses` tail. The plain helper remains output-free for callers
/// that do not own a proof-output stream yet.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_process_clause`], plus any
/// output or proof-documentation write diagnostic.
pub fn proof_state_process_clause_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    proof_state_process_clause_impl(
        state,
        control,
        answer_limit,
        None,
        None,
        Some((output, session, output_level)),
        None,
    )
}

/// Processes one selected clause while rendering only C's `OutputLevel` text.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_process_clause`], plus any
/// output diagnostic from dynamic AC or watchlist-reduction rendering.
pub fn proof_state_process_clause_with_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    let output = output as &mut dyn std::io::Write;
    proof_state_process_clause_impl::<String>(
        state,
        control,
        answer_limit,
        None,
        None,
        None,
        Some((output, output_level, IoFormat::Lop)),
    )
}

/// Processes one selected clause using explicitly supplied global indices.
///
/// This mirrors the C `ProcessClause` tail that inserts the survivor into
/// `state->gindices` before watchlist simplification and selected-clause
/// generation. Production saturation supplies the index owned by `ProofState`.
///
/// # Errors
///
/// Returns diagnostics from selection, contraction, answer-literal evaluation,
/// replacement inferences, backward simplification, processed insertion, global
/// indexed generation, or generated-clause reinsertion.
pub fn proof_state_process_clause_with_global_indices(
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
    indices: &mut GlobalIndices,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    proof_state_process_clause_impl::<String>(
        state,
        control,
        answer_limit,
        Some(indices),
        None,
        None,
        None,
    )
}

/// Processes one selected clause using explicitly supplied global and watchlist
/// global indices.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_process_clause_with_global_indices`].
pub fn proof_state_process_clause_with_global_and_watchlist_indices(
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
    indices: &mut GlobalIndices,
    watchlist_indices: &mut GlobalIndices,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    proof_state_process_clause_impl::<String>(
        state,
        control,
        answer_limit,
        Some(indices),
        Some(watchlist_indices),
        None,
        None,
    )
}

/// Processes one selected clause using explicitly supplied global indices while
/// rendering only C's `OutputLevel` text.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_process_clause_with_global_indices`], plus any output
/// diagnostic from dynamic AC or watchlist-reduction rendering.
pub fn proof_state_process_clause_with_global_indices_and_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
    indices: &mut GlobalIndices,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    let output = output as &mut dyn std::io::Write;
    proof_state_process_clause_impl::<String>(
        state,
        control,
        answer_limit,
        Some(indices),
        None,
        None,
        Some((output, output_level, IoFormat::Lop)),
    )
}

/// Processes one selected clause using explicitly supplied global and watchlist
/// global indices while rendering only C's `OutputLevel` text.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_process_clause_with_global_indices_and_output`].
pub fn proof_state_process_clause_with_global_and_watchlist_indices_and_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
    indices: &mut GlobalIndices,
    watchlist_indices: &mut GlobalIndices,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    let output = output as &mut dyn std::io::Write;
    proof_state_process_clause_impl::<String>(
        state,
        control,
        answer_limit,
        Some(indices),
        Some(watchlist_indices),
        None,
        Some((output, output_level, IoFormat::Lop)),
    )
}

/// Processes one selected clause with explicitly supplied global indices while
/// emitting represented C `document_processing` output.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_process_clause_with_global_indices`], plus any output or
/// proof-documentation write diagnostic.
pub fn proof_state_process_clause_with_global_indices_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
    indices: &mut GlobalIndices,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    proof_state_process_clause_impl(
        state,
        control,
        answer_limit,
        Some(indices),
        None,
        Some((output, session, output_level)),
        None,
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "C-compatible ProcessClause staging keeps the selected-clause phases in order"
)]
fn proof_state_process_clause_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
    mut indices: Option<&mut GlobalIndices>,
    mut watchlist_indices: Option<&mut GlobalIndices>,
    mut doc_context: Option<(&mut W, &mut ProofDocSession, i64)>,
    mut output_context: Option<(&mut dyn std::io::Write, i64, IoFormat)>,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    let Some(mut clause) = proof_state_select_unprocessed_clause(state, control)? else {
        return Ok(ProcessClauseOutcome::NoClause);
    };
    if let Some((output, _session, output_level)) = doc_context.as_mut() {
        if *output_level == 1 {
            fmt::Write::write_str(&mut **output, DEFAULT_COMCHAR_RAW)
                .map_err(proof_control_write_error)?;
        }
    } else if let Some((output, output_level, _output_format)) = output_context.as_mut() {
        if *output_level == 1 {
            std::io::Write::write_all(&mut **output, DEFAULT_COMCHAR_RAW.as_bytes())
                .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
        }
    }
    clause.remove_evaluations();
    clause.set_prop(CP_IS_PROCESSED);
    state.statistics_mut().processed_count += 1;
    debug_assert!(!clause.query_prop(CP_IS_IR_VICTIM));

    let archived_ref = if control.record_gc_selection() {
        let (terms, archive) = state.terms_and_archive_mut();
        Some(clause_archive_copy(archive, &mut clause, terms)?)
    } else {
        None
    };

    let options = ForwardContractOptions {
        non_unit_subsumption: true,
        context_sr: control.heuristic_parms().forward_context_sr,
        condense_clause: control.heuristic_parms().condensing,
        level: RewriteLevel::FullRewrite,
    };
    let packed = if let Some((output, session, _output_level)) = doc_context.as_mut() {
        proof_state_forward_contract_clause_with_docs(
            &mut **output,
            session,
            state,
            control,
            clause,
            options,
        )?
    } else {
        proof_state_forward_contract_clause(state, control, clause, options)?
    };
    let Some(mut packed) = packed else {
        if let Some(archived_ref) = archived_ref {
            let _ = state.archive_mut().delete_by_id(archived_ref.ident());
        }
        return Ok(ProcessClauseOutcome::ContractedAway);
    };

    let answer_detected = if packed.clause().is_sem_false() {
        state.statistics_mut().answer_count += 1;
        state.record_answer_clause(packed.clause());
        true
    } else {
        false
    };
    if answer_detected
        && (packed.clause().is_empty() || state.statistics().answer_count >= answer_limit)
    {
        let reason = if packed.clause().is_empty() {
            ProcessClauseReturnReason::EmptyClause
        } else {
            ProcessClauseReturnReason::AnswerLimit
        };
        let mut clause = packed.into_clause();
        if clause.evaluate_answer_literals(state.terms()) != 0 {
            clause_push_derivation(&mut clause, DC_EVAL_ANSWERS, None, None);
        }
        state.push_extract_root(clause.clone());
        return Ok(ProcessClauseOutcome::Returned { clause, reason });
    }

    debug_assert_eq!(packed.clause().weight(), packed.clause().standard_weight());
    let ac_activated = if let Some((output, _session, output_level)) = doc_context.as_mut() {
        proof_state_check_ac_status_with_fmt_output(
            &mut **output,
            *output_level,
            state,
            control,
            packed.clause_mut(),
        )?
    } else if let Some((output, output_level, _output_format)) = output_context.as_mut() {
        proof_state_check_ac_status_with_output(
            &mut **output,
            *output_level,
            state,
            control,
            packed.clause_mut(),
        )?
    } else {
        proof_state_check_ac_status(state, control, packed.clause_mut())
    };
    if let Some((output, session, output_level)) = doc_context.as_mut() {
        proof_state_document_processing_with_docs(
            &mut **output,
            session,
            *output_level,
            state,
            packed.clause_mut(),
        )?;
    } else if let Some((output, output_level, output_format)) = output_context.as_mut() {
        proof_state_document_processing_with_output(
            &mut **output,
            *output_level,
            *output_format,
            state,
            packed.clause(),
        )?;
    }
    state.statistics_mut().proc_non_trivial_count += 1;

    let replacing = if let Some((output, session, _output_level)) = doc_context.as_mut() {
        proof_state_replacing_inferences_with_docs(&mut **output, session, state, control, packed)?
    } else {
        proof_state_replacing_inferences(state, control, packed)?
    };

    let mut clause = match replacing {
        ReplacingInferenceOutcome::Survivor(clause) => clause,
        ReplacingInferenceOutcome::Replaced { empty } => {
            if let Some(empty) = empty.as_ref() {
                state.push_extract_root(empty.clone());
            }
            return Ok(ProcessClauseOutcome::Replaced { empty });
        }
    };

    let static_watchlist = control.heuristic_parms().watchlist_is_static;
    let lambda_demod = control.heuristic_parms().lambda_demod;
    let watchlist = if let Some((output, session, _output_level)) = doc_context.as_mut() {
        match watchlist_indices.as_deref_mut() {
            Some(indices) => proof_state_check_watchlist_with_global_indices_and_docs(
                &mut **output,
                session,
                state,
                &mut clause,
                static_watchlist,
                lambda_demod,
                indices,
            )?,
            None => proof_state_check_watchlist_with_docs(
                &mut **output,
                session,
                state,
                &mut clause,
                static_watchlist,
                lambda_demod,
            )?,
        }
    } else if let Some((output, output_level, _output_format)) = output_context.as_mut() {
        proof_state_check_watchlist_with_optional_indices_and_output(
            &mut **output,
            *output_level,
            state,
            &mut clause,
            static_watchlist,
            lambda_demod,
            watchlist_indices.as_deref_mut(),
        )?
    } else {
        match watchlist_indices.as_deref_mut() {
            Some(indices) => proof_state_check_watchlist_with_global_indices(
                state,
                &mut clause,
                static_watchlist,
                lambda_demod,
                indices,
            ),
            None => proof_state_check_watchlist(state, &mut clause, static_watchlist, lambda_demod),
        }
    };

    let mut clause_date = proof_state_demodulator_date(state, RewriteLevel::FullRewrite);
    let backward = if let Some(indices) = indices.as_deref_mut() {
        if let Some((output, session, _output_level)) = doc_context.as_mut() {
            proof_state_backward_simplify_with_global_indices_and_docs(
                &mut **output,
                session,
                state,
                control,
                &clause,
                &mut clause_date,
                indices,
            )?
        } else {
            proof_state_backward_simplify_with_global_indices(
                state,
                control,
                &clause,
                &mut clause_date,
                indices,
            )?
        }
    } else if let Some((output, session, _output_level)) = doc_context.as_mut() {
        proof_state_backward_simplify_with_docs(
            &mut **output,
            session,
            state,
            control,
            &clause,
            &mut clause_date,
        )?
    } else {
        proof_state_backward_simplify(state, control, &clause, &mut clause_date)?
    };

    let fresh_vars = state.fresh_vars().clone();
    clause.normalize_vars(state.terms_mut(), &fresh_vars)?;
    let mut renamed_clause = clause.copy_disjoint(state.terms_mut())?;
    renamed_clause.set_ident(clause.ident());

    let processed_ident = clause.ident();
    let class = proof_state_insert_normalized_processed_clause(state, clause, clause_date)?;
    if let Some(indices) = indices.as_deref_mut() {
        proof_state_global_index_processed_clause(
            state,
            indices,
            class,
            processed_ident,
            control.heuristic_parms().lambda_demod,
        )?;
    }
    if answer_detected {
        let root = proof_state_processed_clause_by_class(state, class, processed_ident)
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "processed answer clause disappeared before extraction-root recording",
                )
            })?;
        state.push_extract_root(root);
    }
    if control.heuristic_parms().watchlist_simplify {
        let processed_clause =
            proof_state_processed_clause_by_class(state, class, processed_ident).cloned();
        if let Some(processed_clause) = processed_clause {
            if let Some((output, session, _output_level)) = doc_context.as_mut() {
                let _simplified = match watchlist_indices.as_deref_mut() {
                    Some(indices) => proof_state_simplify_watchlist_with_global_indices_and_docs(
                        &mut **output,
                        session,
                        state,
                        control,
                        &processed_clause,
                        indices,
                    )?,
                    None => proof_state_simplify_watchlist_with_docs(
                        &mut **output,
                        session,
                        state,
                        control,
                        &processed_clause,
                    )?,
                };
            } else {
                let _simplified = match watchlist_indices {
                    Some(indices) => proof_state_simplify_watchlist_with_global_indices(
                        state,
                        control,
                        &processed_clause,
                        indices,
                    )?,
                    None => proof_state_simplify_watchlist(state, control, &processed_clause)?,
                };
            }
        }
    }

    let generation = if control.heuristic_parms().selection_strategy == NO_GENERATION {
        GenerateNewClausesOutcome::default()
    } else {
        let processed_clause = proof_state_processed_clause_by_class(state, class, processed_ident)
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "processed clause disappeared before selected-clause generation",
                )
            })?;
        if let Some((output, session, _output_level)) = doc_context.as_mut() {
            proof_state_generate_new_clauses_with_disjoint_copy_impl(
                state,
                control,
                &renamed_clause,
                &processed_clause,
                problem_type(),
                indices.as_deref(),
                Some((&mut **output, session)),
            )?
        } else {
            proof_state_generate_new_clauses_with_disjoint_copy_impl::<String>(
                state,
                control,
                &renamed_clause,
                &processed_clause,
                problem_type(),
                indices.as_deref(),
                None,
            )?
        }
    };
    drop(renamed_clause);

    if state.tmp_terms().non_var_term_nodes() > TMPBANK_GC_LIMIT {
        let _ = state.tmp_terms_mut().gc_sweep();
    }

    if control.heuristic_parms().detsort_tmpset {
        proof_state_sort_tmp_store_by_struct_weight(state);
    }
    let generated_empty = if let Some((output, session, _output_level)) = doc_context.as_mut() {
        proof_state_insert_new_clauses_with_docs(&mut **output, session, state, control)?
    } else if let Some((output, output_level, _output_format)) = output_context.as_mut() {
        proof_state_insert_new_clauses_impl::<String>(
            state,
            control,
            None,
            Some((&mut **output, *output_level)),
        )?
    } else {
        proof_state_insert_new_clauses(state, control)?
    };
    if let Some(empty) = generated_empty.as_ref() {
        state.push_extract_root(empty.clone());
    }

    Ok(ProcessClauseOutcome::Processed {
        class,
        answer_detected,
        ac_activated,
        watchlist,
        backward,
        generation,
        generated_empty,
    })
}

fn proof_state_check_ac_status_with_fmt_output(
    output: &mut impl fmt::Write,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
) -> Result<bool, Diagnostic> {
    let mut rendered = Vec::new();
    let activated = proof_state_check_ac_status_with_output(
        &mut rendered,
        output_level,
        state,
        control,
        clause,
    )?;
    if !rendered.is_empty() {
        let rendered = std::str::from_utf8(&rendered)
            .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
        fmt::Write::write_str(output, rendered).map_err(proof_control_write_error)?;
    }
    Ok(activated)
}

fn proof_state_document_processing_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    output_level: i64,
    state: &ProofState,
    clause: &mut Clause,
) -> Result<(), Diagnostic> {
    if output_level == 0 {
        return Ok(());
    }
    if output_level == 1 {
        fmt::Write::write_str(output, "\n").map_err(proof_control_write_error)?;
        fmt::Write::write_str(output, DEFAULT_COMCHAR_RAW).map_err(proof_control_write_error)?;
        fmt::Write::write_str(
            output,
            &clause_print_lop_format_string(state.terms(), clause, true),
        )
        .map_err(proof_control_write_error)?;
        fmt::Write::write_str(output, "\n").map_err(proof_control_write_error)?;
    }
    session.doc_clause_quote(output, state.terms(), 6, clause, Some("new_given"), None)?;
    Ok(())
}

fn proof_state_document_processing_with_output(
    output: &mut (impl std::io::Write + ?Sized),
    output_level: i64,
    output_format: IoFormat,
    state: &ProofState,
    clause: &Clause,
) -> Result<(), Diagnostic> {
    if output_level != 1 {
        return Ok(());
    }
    std::io::Write::write_all(output, b"\n")
        .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
    std::io::Write::write_all(output, DEFAULT_COMCHAR_RAW.as_bytes())
        .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
    let rendered =
        clause_print_format_string(state.terms(), clause, true, output_format, problem_type())?;
    std::io::Write::write_all(output, rendered.as_bytes())
        .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
    std::io::Write::write_all(output, b"\n")
        .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))
}

fn proof_control_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "failed to write proof-control output",
    )
}

fn proof_state_global_index_processed_clause(
    state: &mut ProofState,
    indices: &mut GlobalIndices,
    class: ProcessedClauseClass,
    ident: i64,
    lambda_demod: bool,
) -> Result<(), Diagnostic> {
    let (terms, sets) = state.terms_and_processed_sets_mut();
    let clause = match class {
        ProcessedClauseClass::PositiveRule => sets.pos_rules.find_by_id_mut(ident),
        ProcessedClauseClass::PositiveEquation => sets.pos_eqns.find_by_id_mut(ident),
        ProcessedClauseClass::NegativeUnit => sets.neg_units.find_by_id_mut(ident),
        ProcessedClauseClass::NonUnit => sets.non_units.find_by_id_mut(ident),
    }
    .ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "processed clause disappeared before global-index insertion",
        )
    })?;
    indices.insert_clause(clause, terms, lambda_demod);
    Ok(())
}

fn proof_state_processed_clause_by_class(
    state: &ProofState,
    class: ProcessedClauseClass,
    ident: i64,
) -> Option<&Clause> {
    match class {
        ProcessedClauseClass::PositiveRule => state.processed_pos_rules().find_by_id(ident),
        ProcessedClauseClass::PositiveEquation => state.processed_pos_eqns().find_by_id(ident),
        ProcessedClauseClass::NegativeUnit => state.processed_neg_units().find_by_id(ident),
        ProcessedClauseClass::NonUnit => state.processed_non_units().find_by_id(ident),
    }
}

/// Runs the currently ported C `Saturate` loop.
///
/// This preserves the C loop gate order, delegates each iteration to
/// [`proof_state_process_clause`], runs `cleanup_unprocessed_clauses` after
/// non-returning clauses, and stops when the local limit checks fail. The
/// default path uses the ported unindexed selected-clause generators; the
/// explicitly supplied global-index variant uses the indexed branch when PM indexes
/// are available. The SAT-check branch uses the ported pseudo-ground
/// propositional import and internal solver when enabled and due.
///
/// # Errors
///
/// Returns diagnostics from clause processing, cleanup, or SAT-check import.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible Saturate bridge keeps the original limit arguments visible"
)]
pub fn proof_state_saturate(
    state: &mut ProofState,
    control: &mut ProofControl,
    step_limit: i64,
    proc_limit: i64,
    unproc_limit: i64,
    total_limit: i64,
    generated_limit: i64,
    tb_insert_limit: i64,
    answer_limit: i64,
) -> Result<SaturateOutcome, Diagnostic> {
    proof_state_saturate_impl::<String>(
        state,
        control,
        step_limit,
        proc_limit,
        unproc_limit,
        total_limit,
        generated_limit,
        tb_insert_limit,
        answer_limit,
        None,
        None,
        None,
        None,
    )
}

/// Runs the ported C `Saturate` loop while rendering only C's `OutputLevel`
/// text from selected-clause processing.
///
/// # Errors
///
/// Returns the same diagnostics as [`proof_state_saturate`], plus any output
/// diagnostic from dynamic AC or watchlist-reduction rendering.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible Saturate bridge keeps the original limit arguments visible"
)]
pub fn proof_state_saturate_with_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    step_limit: i64,
    proc_limit: i64,
    unproc_limit: i64,
    total_limit: i64,
    generated_limit: i64,
    tb_insert_limit: i64,
    answer_limit: i64,
) -> Result<SaturateOutcome, Diagnostic> {
    let output = output as &mut dyn std::io::Write;
    proof_state_saturate_impl::<String>(
        state,
        control,
        step_limit,
        proc_limit,
        unproc_limit,
        total_limit,
        generated_limit,
        tb_insert_limit,
        answer_limit,
        None,
        Some((output, output_level, IoFormat::Lop)),
        None,
        None,
    )
}

/// Runs the ported C `Saturate` loop using explicitly supplied global indices.
///
/// This mirrors the `ProcessClause` path where C inserts each processed
/// survivor into `state->gindices` and then uses indexed selected-clause
/// generation when paramodulation indexes are available. Production saturation
/// supplies the index owned by `ProofState`.
///
/// # Errors
///
/// Returns diagnostics from indexed clause processing, cleanup, or SAT-check
/// import.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible Saturate bridge keeps the original limit arguments visible"
)]
pub fn proof_state_saturate_with_global_indices(
    state: &mut ProofState,
    control: &mut ProofControl,
    step_limit: i64,
    proc_limit: i64,
    unproc_limit: i64,
    total_limit: i64,
    generated_limit: i64,
    tb_insert_limit: i64,
    answer_limit: i64,
    indices: &mut GlobalIndices,
) -> Result<SaturateOutcome, Diagnostic> {
    proof_state_saturate_impl::<String>(
        state,
        control,
        step_limit,
        proc_limit,
        unproc_limit,
        total_limit,
        generated_limit,
        tb_insert_limit,
        answer_limit,
        Some(indices),
        None,
        None,
        None,
    )
}

/// Runs the ported C `Saturate` loop using explicitly supplied global indices while
/// rendering only C's `OutputLevel` text from selected-clause processing.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_saturate_with_global_indices`], plus any output diagnostic
/// from dynamic AC or watchlist-reduction rendering.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible Saturate bridge keeps the original limit arguments visible"
)]
pub fn proof_state_saturate_with_global_indices_and_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    step_limit: i64,
    proc_limit: i64,
    unproc_limit: i64,
    total_limit: i64,
    generated_limit: i64,
    tb_insert_limit: i64,
    answer_limit: i64,
    indices: &mut GlobalIndices,
) -> Result<SaturateOutcome, Diagnostic> {
    let output = output as &mut dyn std::io::Write;
    proof_state_saturate_impl::<String>(
        state,
        control,
        step_limit,
        proc_limit,
        unproc_limit,
        total_limit,
        generated_limit,
        tb_insert_limit,
        answer_limit,
        Some(indices),
        Some((output, output_level, IoFormat::Lop)),
        None,
        None,
    )
}

/// Runs the ported C `Saturate` loop using explicitly supplied global and
/// watchlist global indices while rendering only C's `OutputLevel` text from
/// selected-clause processing.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_saturate_with_global_indices_and_output`].
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible Saturate bridge keeps the original limit arguments visible"
)]
pub fn proof_state_saturate_with_global_and_watchlist_indices_and_output(
    output: &mut impl std::io::Write,
    output_level: i64,
    output_format: IoFormat,
    state: &mut ProofState,
    control: &mut ProofControl,
    step_limit: i64,
    proc_limit: i64,
    unproc_limit: i64,
    total_limit: i64,
    generated_limit: i64,
    tb_insert_limit: i64,
    answer_limit: i64,
    indices: &mut GlobalIndices,
    watchlist_indices: &mut GlobalIndices,
) -> Result<SaturateOutcome, Diagnostic> {
    let output = output as &mut dyn std::io::Write;
    proof_state_saturate_impl::<String>(
        state,
        control,
        step_limit,
        proc_limit,
        unproc_limit,
        total_limit,
        generated_limit,
        tb_insert_limit,
        answer_limit,
        Some(indices),
        Some((output, output_level, output_format)),
        Some(watchlist_indices),
        None,
    )
}

/// Runs the ported C `Saturate` loop using explicitly supplied global and
/// watchlist global indices while emitting represented C
/// `document_processing` and generated-clause proof-documentation output.
///
/// # Errors
///
/// Returns the same diagnostics as
/// [`proof_state_saturate_with_global_and_watchlist_indices_and_output`], plus
/// any proof-documentation write diagnostic.
#[expect(
    clippy::too_many_arguments,
    reason = "C-compatible Saturate bridge keeps the original limit arguments visible"
)]
pub fn proof_state_saturate_with_global_and_watchlist_indices_and_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    step_limit: i64,
    proc_limit: i64,
    unproc_limit: i64,
    total_limit: i64,
    generated_limit: i64,
    tb_insert_limit: i64,
    answer_limit: i64,
    indices: &mut GlobalIndices,
    watchlist_indices: &mut GlobalIndices,
) -> Result<SaturateOutcome, Diagnostic> {
    proof_state_saturate_impl(
        state,
        control,
        step_limit,
        proc_limit,
        unproc_limit,
        total_limit,
        generated_limit,
        tb_insert_limit,
        answer_limit,
        Some(indices),
        None,
        Some(watchlist_indices),
        Some((output, session, output_level)),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SatCheckThresholds {
    size: i64,
    step: i64,
    ttinsert: i64,
}

impl SatCheckThresholds {
    const fn new(params: &HeuristicParmsCell) -> Self {
        Self {
            size: params.sat_check_size_limit,
            step: params.sat_check_step_limit,
            ttinsert: params.sat_check_ttinsert_limit,
        }
    }

    fn advance_after(self, trigger: SatCheckTrigger, params: &HeuristicParmsCell) -> Self {
        match trigger {
            SatCheckTrigger::Size { cardinality } => {
                let mut next = self.size;
                if params.sat_check_size_limit > 0 {
                    while next <= cardinality {
                        next = next.saturating_add(params.sat_check_size_limit);
                    }
                }
                Self { size: next, ..self }
            }
            SatCheckTrigger::Step => Self {
                step: self.step.saturating_add(params.sat_check_step_limit),
                ..self
            },
            SatCheckTrigger::TermBankInsertions => Self {
                ttinsert: self.ttinsert.saturating_mul(2),
                ..self
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SatCheckTrigger {
    Size { cardinality: i64 },
    Step,
    TermBankInsertions,
}

struct SatCheckRefutation {
    clause: Clause,
    solver_reported: bool,
}

impl SatCheckRefutation {
    fn into_saturate_outcome(
        self,
        state: &mut ProofState,
        processed_steps: i64,
    ) -> SaturateOutcome {
        let reason = if self.solver_reported {
            SaturateReturnReason::SatCheck
        } else {
            SaturateReturnReason::SatCheckPreprocessing
        };
        proof_state_saturate_return_with_extract_root(state, self.clause, reason, processed_steps)
    }
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "C-compatible Saturate bridge keeps the original limits and main-loop gate order visible"
)]
fn proof_state_saturate_impl<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    step_limit: i64,
    proc_limit: i64,
    unproc_limit: i64,
    total_limit: i64,
    generated_limit: i64,
    tb_insert_limit: i64,
    answer_limit: i64,
    mut indices: Option<&mut GlobalIndices>,
    mut output_context: Option<(&mut dyn std::io::Write, i64, IoFormat)>,
    mut watchlist_indices: Option<&mut GlobalIndices>,
    mut doc_context: Option<(&mut W, &mut ProofDocSession, i64)>,
) -> Result<SaturateOutcome, Diagnostic> {
    let mut processed_steps = 0_i64;
    let mut sat_check_thresholds = SatCheckThresholds::new(control.heuristic_parms());

    loop {
        if let Some(reason) = proof_state_saturate_stop_reason(
            state,
            step_limit,
            proc_limit,
            unproc_limit,
            total_limit,
            generated_limit,
            tb_insert_limit,
            processed_steps,
        ) {
            return Ok(SaturateOutcome::Stopped {
                reason,
                processed_steps,
            });
        }

        processed_steps = processed_steps.saturating_add(1);
        let process_outcome = if let Some((output, session, output_level)) = doc_context.as_mut() {
            proof_state_process_clause_impl(
                state,
                control,
                answer_limit,
                indices.as_deref_mut(),
                watchlist_indices.as_deref_mut(),
                Some((&mut **output, &mut **session, *output_level)),
                None,
            )
        } else {
            proof_state_process_clause_for_saturate(
                state,
                control,
                answer_limit,
                indices.as_deref_mut(),
                watchlist_indices.as_deref_mut(),
                output_context.as_mut(),
            )
        }?;
        match process_outcome {
            ProcessClauseOutcome::NoClause => {
                return Ok(SaturateOutcome::Stopped {
                    reason: SaturateStopReason::Saturated,
                    processed_steps,
                });
            }
            ProcessClauseOutcome::ContractedAway => {}
            ProcessClauseOutcome::Returned { clause, reason } => {
                return Ok(SaturateOutcome::Returned {
                    clause: Box::new(clause),
                    reason: SaturateReturnReason::ProcessClause(reason),
                    processed_steps,
                });
            }
            ProcessClauseOutcome::Replaced { empty } => {
                if let Some(clause) = empty {
                    return Ok(SaturateOutcome::Returned {
                        clause: Box::new(clause),
                        reason: SaturateReturnReason::ReplacingInference,
                        processed_steps,
                    });
                }
            }
            ProcessClauseOutcome::Processed {
                generated_empty, ..
            } => {
                if let Some(clause) = generated_empty {
                    return Ok(SaturateOutcome::Returned {
                        clause: Box::new(clause),
                        reason: SaturateReturnReason::GeneratedClause,
                        processed_steps,
                    });
                }
            }
        }

        let cleanup = if let Some((output, session, _output_level)) = doc_context.as_mut() {
            proof_state_cleanup_unprocessed_clauses_with_docs(
                &mut **output,
                session,
                state,
                control,
            )?
        } else {
            proof_state_cleanup_unprocessed_clauses(state, control)?
        };
        if let Some((output, _session, output_level)) = doc_context.as_mut() {
            write_cleanup_unprocessed_fmt_output(&mut **output, *output_level, &cleanup)?;
        } else if let Some((output, output_level, _output_format)) = output_context.as_mut() {
            write_cleanup_unprocessed_output(&mut **output, *output_level, &cleanup)?;
        }
        if let Some(clause) = cleanup.unsatisfiable {
            return Ok(proof_state_saturate_return_with_extract_root(
                state,
                clause,
                SaturateReturnReason::Cleanup,
                processed_steps,
            ));
        }

        if let Some(refutation) = proof_state_saturate_sat_check_gate(
            state,
            control,
            &mut sat_check_thresholds,
            &mut doc_context,
        )? {
            return Ok(refutation.into_saturate_outcome(state, processed_steps));
        }
    }
}

fn write_cleanup_unprocessed_output(
    output: &mut dyn std::io::Write,
    output_level: i64,
    outcome: &CleanupUnprocessedOutcome,
) -> Result<(), Diagnostic> {
    let rendered = cleanup_unprocessed_output_string(output_level, outcome);
    std::io::Write::write_all(output, rendered.as_bytes())
        .map_err(|error| proof_control_io_error(&error))
}

fn write_cleanup_unprocessed_fmt_output(
    output: &mut impl fmt::Write,
    output_level: i64,
    outcome: &CleanupUnprocessedOutcome,
) -> Result<(), Diagnostic> {
    let rendered = cleanup_unprocessed_output_string(output_level, outcome);
    fmt::Write::write_str(output, &rendered).map_err(proof_control_write_error)
}

fn cleanup_unprocessed_output_string(
    output_level: i64,
    outcome: &CleanupUnprocessedOutcome,
) -> String {
    if output_level == 0 {
        return String::new();
    }

    let mut rendered = String::new();
    if outcome.orphan_cleanup_triggered {
        let _ = writeln!(
            &mut rendered,
            "{DEFAULT_COMCHAR_RAW} Deleted {} orphaned clauses (remaining: {})",
            outcome.orphan_cleanup_deleted, outcome.orphan_cleanup_remaining
        );
    }
    if outcome.forward_contract_triggered {
        let _ = writeln!(
            &mut rendered,
            "{DEFAULT_COMCHAR_RAW} Special forward-contraction deletes {} clauses(remaining: {}) ",
            outcome.forward_contract_deleted, outcome.forward_contract_remaining
        );
        if outcome.unsatisfiable.is_none() && output_level >= 1 {
            let _ = writeln!(
                &mut rendered,
                "{DEFAULT_COMCHAR_RAW} Reweighting unprocessed clauses..."
            );
        }
    }
    if outcome.delete_bad_triggered {
        let _ = writeln!(
            &mut rendered,
            "{DEFAULT_COMCHAR_RAW} Deleted {} orphaned clauses and {} bad clauses (prover may be incomplete now)",
            outcome.delete_bad_orphaned_deleted, outcome.bad_deleted
        );
    }
    rendered
}

fn proof_control_io_error(error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, error.to_string())
}

fn proof_state_process_clause_for_saturate(
    state: &mut ProofState,
    control: &mut ProofControl,
    answer_limit: i64,
    indices: Option<&mut GlobalIndices>,
    watchlist_indices: Option<&mut GlobalIndices>,
    output_context: Option<&mut (&mut dyn std::io::Write, i64, IoFormat)>,
) -> Result<ProcessClauseOutcome, Diagnostic> {
    match (indices, watchlist_indices, output_context) {
        (Some(indices), Some(watchlist_indices), Some((output, output_level, output_format))) => {
            proof_state_process_clause_impl::<String>(
                state,
                control,
                answer_limit,
                Some(indices),
                Some(watchlist_indices),
                None,
                Some((&mut **output, *output_level, *output_format)),
            )
        }
        (Some(indices), Some(watchlist_indices), None) => {
            proof_state_process_clause_with_global_and_watchlist_indices(
                state,
                control,
                answer_limit,
                indices,
                watchlist_indices,
            )
        }
        (Some(indices), None, Some((output, output_level, output_format))) => {
            proof_state_process_clause_impl::<String>(
                state,
                control,
                answer_limit,
                Some(indices),
                None,
                None,
                Some((&mut **output, *output_level, *output_format)),
            )
        }
        (Some(indices), None, None) => {
            proof_state_process_clause_with_global_indices(state, control, answer_limit, indices)
        }
        (None, Some(watchlist_indices), Some((output, output_level, output_format))) => {
            proof_state_process_clause_impl::<String>(
                state,
                control,
                answer_limit,
                None,
                Some(watchlist_indices),
                None,
                Some((&mut **output, *output_level, *output_format)),
            )
        }
        (None, Some(watchlist_indices), None) => proof_state_process_clause_impl::<String>(
            state,
            control,
            answer_limit,
            None,
            Some(watchlist_indices),
            None,
            None,
        ),
        (None, None, Some((output, output_level, output_format))) => {
            proof_state_process_clause_impl::<String>(
                state,
                control,
                answer_limit,
                None,
                None,
                None,
                Some((&mut **output, *output_level, *output_format)),
            )
        }
        (None, None, None) => proof_state_process_clause(state, control, answer_limit),
    }
}

fn proof_state_saturate_return_with_extract_root(
    state: &mut ProofState,
    clause: Clause,
    reason: SaturateReturnReason,
    processed_steps: i64,
) -> SaturateOutcome {
    state.push_extract_root(clause.clone());
    SaturateOutcome::Returned {
        clause: Box::new(clause),
        reason,
        processed_steps,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "Matches the C Saturate while-condition layout"
)]
fn proof_state_saturate_stop_reason(
    state: &ProofState,
    step_limit: i64,
    proc_limit: i64,
    unproc_limit: i64,
    total_limit: i64,
    generated_limit: i64,
    tb_insert_limit: i64,
    processed_steps: i64,
) -> Option<SaturateStopReason> {
    if time_is_up() {
        return Some(SaturateStopReason::TimeLimit);
    }
    if state.unprocessed().is_empty() {
        return Some(SaturateStopReason::Saturated);
    }
    if step_limit <= processed_steps {
        return Some(SaturateStopReason::StepLimit);
    }
    if proc_limit <= state.processed_cardinality() {
        return Some(SaturateStopReason::ProcessedLimit);
    }
    if unproc_limit <= state.unprocessed_cardinality() {
        return Some(SaturateStopReason::UnprocessedLimit);
    }
    if total_limit <= state.cardinality() {
        return Some(SaturateStopReason::TotalLimit);
    }
    if !c_signed_long_gt_unsigned_long(generated_limit, proof_state_generated_limit_counter(state))
    {
        return Some(SaturateStopReason::GeneratedLimit);
    }
    if !c_signed_long_gt_unsigned_long(tb_insert_limit, state.terms().insertions()) {
        return Some(SaturateStopReason::TermBankInsertionLimit);
    }
    if state.watchlist_active() && state.watchlist().is_some_and(ClauseSet::is_empty) {
        return Some(SaturateStopReason::WatchlistEmpty);
    }
    None
}

fn proof_state_generated_limit_counter(state: &ProofState) -> u64 {
    state
        .statistics()
        .generated_count
        .wrapping_sub(state.statistics().backward_rewritten_count)
}

fn c_signed_long_gt_unsigned_long(left: i64, right: u64) -> bool {
    i64_as_c_unsigned_long(left) > right
}

fn c_unsigned_long_ge_signed_long(left: u64, right: i64) -> bool {
    left >= i64_as_c_unsigned_long(right)
}

fn i64_as_c_unsigned_long(value: i64) -> u64 {
    u64::from_ne_bytes(value.to_ne_bytes())
}

fn proof_state_saturate_sat_check_gate<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    thresholds: &mut SatCheckThresholds,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession, i64)>,
) -> Result<Option<SatCheckRefutation>, Diagnostic> {
    let params = control.heuristic_parms();
    if params.sat_check_grounding == GroundingStrategy::NoGrounding {
        return Ok(None);
    }

    let cardinality = state.cardinality();
    let trigger = if cardinality >= thresholds.size {
        Some(SatCheckTrigger::Size { cardinality })
    } else if c_unsigned_long_ge_signed_long(
        state.statistics().proc_non_trivial_count,
        thresholds.step,
    ) {
        Some(SatCheckTrigger::Step)
    } else if c_unsigned_long_ge_signed_long(state.terms().insertions(), thresholds.ttinsert) {
        Some(SatCheckTrigger::TermBankInsertions)
    } else {
        None
    };

    let Some(trigger) = trigger else {
        return Ok(None);
    };

    let empty = proof_state_sat_check(state, control, doc_context)?;
    *thresholds = thresholds.advance_after(trigger, control.heuristic_parms());
    Ok(empty)
}

fn proof_state_sat_check<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession, i64)>,
) -> Result<Option<SatCheckRefutation>, Diagnostic> {
    let sat_check_normalize = control.heuristic_parms().sat_check_normalize;
    let sat_check_grounding = control.heuristic_parms().sat_check_grounding;
    let sat_check_normconst = control.heuristic_parms().sat_check_normconst;
    let sat_check_decision_limit = control.heuristic_parms().sat_check_decision_limit;
    let preproc_start = Instant::now();
    let mut preproc_time = 0.0;
    if sat_check_normalize {
        let mut eliminated = 0_u64;
        let mut unprocessed = std::mem::take(state.unprocessed_mut());
        let contraction_result = match doc_context.as_mut() {
            Some((output, session, _output_level)) => {
                proof_state_forward_contract_set_reweight_with_docs(
                    &mut **output,
                    session,
                    state,
                    control,
                    &mut unprocessed,
                    false,
                    RewriteLevel::FullRewrite,
                    &mut eliminated,
                )
            }
            None => proof_state_forward_contract_set_reweight(
                state,
                control,
                &mut unprocessed,
                false,
                RewriteLevel::FullRewrite,
                &mut eliminated,
            ),
        };
        let empty = match contraction_result {
            Ok(empty) => empty,
            Err(err) => {
                *state.unprocessed_mut() = unprocessed;
                return Err(err);
            }
        };
        *state.unprocessed_mut() = unprocessed;
        state.statistics_mut().proc_trivial_count = state
            .statistics()
            .proc_trivial_count
            .saturating_add(eliminated);
        preproc_time = preproc_start.elapsed().as_secs_f64();
        if let Some(clause) = empty {
            return Ok(Some(SatCheckRefutation {
                clause,
                solver_reported: false,
            }));
        }
    }

    let report = match &mut control.sat_solver_backend {
        SatSolverBackend::Internal => sat_check_proof_state_until_time_limit(
            state,
            sat_check_grounding,
            sat_check_normconst,
            sat_check_decision_limit,
        )?,
        SatSolverBackend::PicoSat(solver) => sat_check_proof_state_with_picosat_until_time_limit(
            state,
            sat_check_grounding,
            sat_check_normconst,
            sat_check_decision_limit,
            solver,
        )?,
    };
    let Some(report) = report else {
        return Ok(None);
    };
    control
        .reset_sat_solver()
        .map_err(|error| picosat_error_to_diagnostic(&error))?;
    apply_sat_check_report(state, preproc_time, &report);
    Ok(report.empty.map(|clause| SatCheckRefutation {
        clause,
        solver_reported: true,
    }))
}

fn apply_sat_check_report(state: &mut ProofState, preproc_time: f64, report: &SatCheckReport) {
    let statistics = state.statistics_mut();
    statistics.satcheck_count = statistics.satcheck_count.saturating_add(1);
    statistics.satcheck_preproc_time += preproc_time;
    statistics.satcheck_encoding_time += report.encoding_time;
    statistics.satcheck_solver_time += report.solver_time;
    match report.result {
        ProverResult::Unsatisfiable => {
            statistics.satcheck_success = statistics.satcheck_success.saturating_add(1);
            statistics.satcheck_full_size = report.full_size;
            statistics.satcheck_actual_size = report.actual_size;
            statistics.satcheck_core_size = report.core_size;
            statistics.satcheck_preproc_stime += preproc_time;
            statistics.satcheck_encoding_stime += report.encoding_time;
            statistics.satcheck_solver_stime += report.solver_time;
        }
        ProverResult::Satisfiable => {
            statistics.satcheck_satisfiable = statistics.satcheck_satisfiable.saturating_add(1);
        }
        _ => {}
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProcessedSetSlot {
    PosRules,
    PosEqns,
    NegUnits,
    NonUnits,
}

const PROCESSED_SET_SLOTS: [ProcessedSetSlot; 4] = [
    ProcessedSetSlot::PosRules,
    ProcessedSetSlot::PosEqns,
    ProcessedSetSlot::NegUnits,
    ProcessedSetSlot::NonUnits,
];

fn proof_state_demodulator_date(state: &ProofState, level: RewriteLevel) -> SysDate {
    let demodulators = [state.processed_pos_rules(), state.processed_pos_eqns()];
    clause_set_list_get_max_date(&demodulators, rewrite_level_set_count(level))
}

fn rewrite_level_set_count(level: RewriteLevel) -> usize {
    match level {
        RewriteLevel::NoRewrite => 0,
        RewriteLevel::RuleRewrite => 1,
        RewriteLevel::FullRewrite => 2,
    }
}

fn proof_state_eliminate_backward_rewritten_clauses<W: fmt::Write>(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &Clause,
    clause_date: &mut SysDate,
    mut indices: Option<&mut GlobalIndices>,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<bool, Diagnostic> {
    if !clause.is_demodulator() {
        return Ok(false);
    }
    match clause_date.increment() {
        SysDateIncrement::Advanced => {}
        SysDateIncrement::CAssertionWouldFail => {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "backward rewrite date increment would violate C SysDate assertion",
            ));
        }
        SysDateIncrement::Overflow => {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "backward rewrite date increment overflowed",
            ));
        }
    }

    let lambda_demod = control.heuristic_parms().lambda_demod;
    let indexed_rewritable = match indices.as_deref().and_then(|indices| indices.bw_rw_index()) {
        Some(index) => {
            let ocb = control.ocb.as_mut().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "backward rewriting requires initialized proof-control ordering",
                )
            })?;
            Some(rewritable_ids_in_index(
                state.terms_mut(),
                ocb,
                index,
                clause,
                *clause_date,
            )?)
        }
        None => None,
    };

    let min_rw = if let Some((found, ids)) = indexed_rewritable {
        move_simplified_ids_from_processed_sets(
            state,
            ids,
            indices.as_deref_mut(),
            lambda_demod,
            doc_context,
        )?;
        found
    } else {
        let mut found_any = false;
        for slot in PROCESSED_SET_SLOTS {
            let (found, ids) = {
                let ocb = control.ocb.as_mut().ok_or_else(|| {
                    Diagnostic::new(
                        ErrorCode::OTHER_ERROR,
                        "backward rewriting requires initialized proof-control ordering",
                    )
                })?;
                let (terms, processed_sets) = state.terms_and_processed_sets_mut();
                let set = processed_set_from_bundle(&processed_sets, slot);
                rewritable_ids_in_set(terms, ocb, set, clause, *clause_date)?
            };
            found_any = found_any || found;
            move_simplified_ids_from_slot(
                state,
                slot,
                ids,
                indices.as_deref_mut(),
                lambda_demod,
                doc_context,
            )?;
        }
        found_any
    };

    if control.heuristic_parms().detsort_bw_rw {
        proof_state_sort_tmp_store_by_struct_weight(state);
    }
    Ok(min_rw)
}

fn rewritable_ids_in_set(
    terms: &mut TermBank,
    ocb: &mut OrderControlBlock,
    set: &ClauseSet,
    clause: &Clause,
    clause_date: SysDate,
) -> Result<(bool, Vec<i64>), Diagnostic> {
    let mut rewritable = Vec::new();
    let found = find_rewritable_clauses(terms, ocb, set, &mut rewritable, clause, clause_date)?;
    let ids = rewritable.iter().map(|clause| clause.ident()).collect();
    Ok((found, ids))
}

fn rewritable_ids_in_index(
    terms: &mut TermBank,
    ocb: &mut OrderControlBlock,
    index: &SubtermIndex,
    clause: &Clause,
    clause_date: SysDate,
) -> Result<(bool, Vec<i64>), Diagnostic> {
    let mut rewritable = Vec::new();
    let found =
        find_rewritable_clauses_indexed(terms, ocb, index, &mut rewritable, clause, clause_date)?;
    let ids = rewritable.iter().map(|clause| clause.ident()).collect();
    Ok((found != 0, ids))
}

fn rewritable_ids_in_watchlist(
    terms: &mut TermBank,
    ocb: &mut OrderControlBlock,
    set: &ClauseSet,
    bw_rw_index: Option<&SubtermIndex>,
    clause: &Clause,
    clause_date: SysDate,
) -> Result<(bool, Vec<i64>), Diagnostic> {
    let Some(index) = bw_rw_index else {
        return rewritable_ids_in_set(terms, ocb, set, clause, clause_date);
    };
    rewritable_ids_in_index(terms, ocb, index, clause, clause_date)
}

fn proof_state_eliminate_backward_subsumed_clauses<W: fmt::Write>(
    state: &mut ProofState,
    subsumer: &Clause,
    mut indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    let mut removed = 0;
    if subsumer.is_unit() {
        if subsumer.positive_literal_count() != 0 {
            if !subsumer.is_rw_rule() {
                removed += remove_subsumed_ids_from_slot(
                    state,
                    ProcessedSetSlot::PosRules,
                    subsumer,
                    indices.as_deref_mut(),
                    lambda_demod,
                    doc_context,
                )?;
                removed += remove_subsumed_ids_from_slot(
                    state,
                    ProcessedSetSlot::PosEqns,
                    subsumer,
                    indices.as_deref_mut(),
                    lambda_demod,
                    doc_context,
                )?;
            }
            removed += remove_subsumed_ids_from_slot(
                state,
                ProcessedSetSlot::NonUnits,
                subsumer,
                indices.as_deref_mut(),
                lambda_demod,
                doc_context,
            )?;
        } else {
            removed += remove_subsumed_ids_from_slot(
                state,
                ProcessedSetSlot::NegUnits,
                subsumer,
                indices.as_deref_mut(),
                lambda_demod,
                doc_context,
            )?;
            removed += remove_subsumed_ids_from_slot(
                state,
                ProcessedSetSlot::NonUnits,
                subsumer,
                indices.as_deref_mut(),
                lambda_demod,
                doc_context,
            )?;
        }
    } else {
        removed += remove_subsumed_ids_from_slot(
            state,
            ProcessedSetSlot::NonUnits,
            subsumer,
            indices,
            lambda_demod,
            doc_context,
        )?;
    }
    Ok(removed)
}

fn remove_subsumed_ids_from_slot<W: fmt::Write>(
    state: &mut ProofState,
    slot: ProcessedSetSlot,
    subsumer: &Clause,
    mut indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    let ids = {
        let (terms, sets) = state.terms_and_processed_sets_mut();
        let set = processed_set_from_bundle(&sets, slot);
        subsumed_ids_in_set(set, subsumer, terms)?
    };
    let mut removed = 0;
    for id in ids.into_iter().rev() {
        if let Some(indices) = indices.as_deref_mut() {
            proof_state_delete_global_indexed_clause_by_id_from_slot(
                state,
                slot,
                id,
                indices,
                lambda_demod,
            );
        }
        let Some(clause) = processed_set_mut_by_slot(state, slot).extract_by_id(id) else {
            continue;
        };
        let mut clause = clause;
        if let Some((output, session)) = doc_context.as_mut() {
            let comment = if clause.query_prop(CP_WATCH_ONLY) {
                "extract_wl_subsumed"
            } else {
                "subsumed"
            };
            session.doc_clause_quote(
                &mut **output,
                state.terms(),
                6,
                &mut clause,
                Some(comment),
                Some(subsumer),
            )?;
        }
        proof_state_archive_dead_clause(state, clause);
        removed += 1;
    }
    Ok(removed)
}

fn subsumed_ids_in_set(
    set: &ClauseSet,
    subsumer: &Clause,
    terms: &mut TermBank,
) -> Result<Vec<i64>, Diagnostic> {
    let mut matched_clauses = PStack::new();
    let _ = clause_set_find_subsumed_clauses_owned_with_bank(
        set,
        subsumer,
        &mut matched_clauses,
        terms,
    )?;
    Ok(matched_clauses
        .as_slice()
        .iter()
        .map(|clause| clause.ident())
        .collect())
}

fn proof_state_eliminate_unit_simplified_clauses<W: fmt::Write>(
    state: &mut ProofState,
    simplifier: &Clause,
    mut indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    if simplifier.is_rw_rule() || !simplifier.is_unit() {
        return Ok(0);
    }

    let mut moved = move_unit_simplified_from_slot(
        state,
        ProcessedSetSlot::NonUnits,
        simplifier,
        indices.as_deref_mut(),
        lambda_demod,
        doc_context,
    )?;
    if simplifier.is_positive() {
        moved += move_unit_simplified_from_slot(
            state,
            ProcessedSetSlot::NegUnits,
            simplifier,
            indices.as_deref_mut(),
            lambda_demod,
            doc_context,
        )?;
    } else {
        moved += move_unit_simplified_from_slot(
            state,
            ProcessedSetSlot::PosRules,
            simplifier,
            indices.as_deref_mut(),
            lambda_demod,
            doc_context,
        )?;
        moved += move_unit_simplified_from_slot(
            state,
            ProcessedSetSlot::PosEqns,
            simplifier,
            indices,
            lambda_demod,
            doc_context,
        )?;
    }
    Ok(moved)
}

fn move_unit_simplified_from_slot<W: fmt::Write>(
    state: &mut ProofState,
    slot: ProcessedSetSlot,
    simplifier: &Clause,
    indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    let ids = {
        let (terms, sets) = state.terms_and_processed_sets_mut();
        let set = processed_set_from_bundle(&sets, slot);
        let mut ids = Vec::new();
        for clause in set.iter() {
            if clause_unit_simplify_test(clause, simplifier, terms)? {
                ids.push(clause.ident());
            }
        }
        ids
    };
    move_simplified_ids_from_slot(state, slot, ids, indices, lambda_demod, doc_context)
}

fn clause_unit_simplify_test(
    clause: &Clause,
    simplifier: &Clause,
    bank: &mut TermBank,
) -> Result<bool, Diagnostic> {
    debug_assert!(simplifier.is_unit());
    let simplifier_literal = simplifier
        .literals()
        .as_slice()
        .first()
        .expect("unit simplifier must have one literal");
    debug_assert!(simplifier_literal.is_negative() || !simplifier_literal.is_oriented());

    let simplifier_positive = simplifier_literal.is_positive();
    if simplifier_positive == clause.is_positive() {
        return Ok(false);
    }

    for literal in clause.literals().as_slice() {
        if simplifier_positive != literal.is_positive()
            && eqn_topsubsumes_termpair_with_bank(
                bank,
                simplifier_literal,
                literal.left(),
                literal.right(),
            )?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn proof_state_eliminate_context_sr_clauses<W: fmt::Write>(
    state: &mut ProofState,
    control: &ProofControl,
    simplifier: &Clause,
    indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    if !control.heuristic_parms().backward_context_sr {
        return Ok(0);
    }

    let ids = {
        let (terms, processed_sets) = state.terms_and_processed_sets_mut();
        let mut clauses = PStack::new();
        let count = clause_set_find_context_sr_clauses_with_bank(
            processed_sets.non_units,
            &mut simplifier.clone(),
            &mut clauses,
            terms,
        )?;
        if count == 0 {
            Vec::new()
        } else {
            clauses
                .as_slice()
                .iter()
                .map(|clause| clause.ident())
                .collect()
        }
    };

    move_simplified_ids_from_slot(
        state,
        ProcessedSetSlot::NonUnits,
        ids,
        indices,
        lambda_demod,
        doc_context,
    )
}

fn move_simplified_ids_from_slot<W: fmt::Write>(
    state: &mut ProofState,
    slot: ProcessedSetSlot,
    ids: Vec<i64>,
    mut indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    let mut moved = 0;
    for id in ids.into_iter().rev() {
        moved += move_simplified_id_from_slot(
            state,
            slot,
            id,
            indices.as_deref_mut(),
            lambda_demod,
            doc_context,
        )?;
    }
    Ok(moved)
}

fn move_simplified_ids_from_processed_sets<W: fmt::Write>(
    state: &mut ProofState,
    ids: Vec<i64>,
    mut indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    let mut moved = 0;
    for id in ids.into_iter().rev() {
        let slot = PROCESSED_SET_SLOTS
            .into_iter()
            .find(|slot| processed_set_by_slot(state, *slot).find_by_id(id).is_some());
        let Some(slot) = slot else {
            continue;
        };
        moved += move_simplified_id_from_slot(
            state,
            slot,
            id,
            indices.as_deref_mut(),
            lambda_demod,
            doc_context,
        )?;
    }
    Ok(moved)
}

fn move_simplified_id_from_slot<W: fmt::Write>(
    state: &mut ProofState,
    slot: ProcessedSetSlot,
    id: i64,
    indices: Option<&mut GlobalIndices>,
    lambda_demod: bool,
    doc_context: &mut Option<(&mut W, &mut ProofDocSession)>,
) -> Result<u64, Diagnostic> {
    if let Some(indices) = indices {
        proof_state_delete_global_indexed_clause_by_id_from_slot(
            state,
            slot,
            id,
            indices,
            lambda_demod,
        );
    }
    let Some(mut clause) = processed_set_mut_by_slot(state, slot).extract_by_id(id) else {
        return Ok(0);
    };
    if let Some((output, session)) = doc_context.as_mut() {
        session.doc_clause_quote(
            &mut **output,
            state.terms(),
            6,
            &mut clause,
            Some("simplifiable"),
            None,
        )?;
    }
    proof_state_move_simplified_clause_to_tmp(state, clause)?;
    Ok(1)
}

fn proof_state_move_simplified_clause_to_tmp(
    state: &mut ProofState,
    clause: Clause,
) -> Result<(), Diagnostic> {
    let requeued = proof_state_archive_simplified_clause(state, clause)?;
    state.tmp_store_mut().insert(requeued);
    Ok(())
}

fn proof_state_archive_simplified_clause(
    state: &mut ProofState,
    mut clause: Clause,
) -> Result<Clause, Diagnostic> {
    let mut requeued = clause.flat_copy(state.terms_mut())?;
    requeued.refresh_derivation_generation();
    clause_push_derivation(&mut requeued, DC_CNF_QUOTE, Some(&clause), None);
    clause.set_prop(CP_IS_DEAD);
    state.archive_mut().insert(clause);
    Ok(requeued)
}

fn proof_state_archive_dead_clause(state: &mut ProofState, mut clause: Clause) {
    clause.set_prop(CP_IS_DEAD);
    state.archive_mut().insert(clause);
}

fn proof_state_sort_tmp_store_by_struct_weight(state: &mut ProofState) {
    let mut tmp_store = std::mem::take(state.tmp_store_mut());
    tmp_store.sort_by(|left, right| left.cmp_by_struct_weight(right, state.terms()).cmp(&0));
    *state.tmp_store_mut() = tmp_store;
}

fn processed_set_from_bundle<'a>(
    sets: &'a crate::clauses::proofstate::ProofStateProcessedSets<'a>,
    slot: ProcessedSetSlot,
) -> &'a ClauseSet {
    match slot {
        ProcessedSetSlot::PosRules => sets.pos_rules,
        ProcessedSetSlot::PosEqns => sets.pos_eqns,
        ProcessedSetSlot::NegUnits => sets.neg_units,
        ProcessedSetSlot::NonUnits => sets.non_units,
    }
}

fn processed_set_mut_from_bundle(
    sets: crate::clauses::proofstate::ProofStateProcessedSets<'_>,
    slot: ProcessedSetSlot,
) -> &mut ClauseSet {
    match slot {
        ProcessedSetSlot::PosRules => sets.pos_rules,
        ProcessedSetSlot::PosEqns => sets.pos_eqns,
        ProcessedSetSlot::NegUnits => sets.neg_units,
        ProcessedSetSlot::NonUnits => sets.non_units,
    }
}

fn processed_set_by_slot(state: &ProofState, slot: ProcessedSetSlot) -> &ClauseSet {
    match slot {
        ProcessedSetSlot::PosRules => state.processed_pos_rules(),
        ProcessedSetSlot::PosEqns => state.processed_pos_eqns(),
        ProcessedSetSlot::NegUnits => state.processed_neg_units(),
        ProcessedSetSlot::NonUnits => state.processed_non_units(),
    }
}

fn processed_set_mut_by_slot(state: &mut ProofState, slot: ProcessedSetSlot) -> &mut ClauseSet {
    match slot {
        ProcessedSetSlot::PosRules => state.processed_pos_rules_mut(),
        ProcessedSetSlot::PosEqns => state.processed_pos_eqns_mut(),
        ProcessedSetSlot::NegUnits => state.processed_neg_units_mut(),
        ProcessedSetSlot::NonUnits => state.processed_non_units_mut(),
    }
}

fn proof_state_delete_first_global_indexed_clause_from_slot(
    state: &mut ProofState,
    slot: ProcessedSetSlot,
    indices: &mut GlobalIndices,
    lambda_demod: bool,
) {
    let (terms, sets) = state.terms_and_processed_sets_mut();
    let set = processed_set_mut_from_bundle(sets, slot);
    if let Some(clause) = set.iter_mut().next() {
        proof_state_delete_global_indexed_clause(indices, terms, clause, lambda_demod);
    }
}

fn proof_state_delete_global_indexed_clause_by_id_from_slot(
    state: &mut ProofState,
    slot: ProcessedSetSlot,
    ident: i64,
    indices: &mut GlobalIndices,
    lambda_demod: bool,
) {
    let (terms, sets) = state.terms_and_processed_sets_mut();
    let set = processed_set_mut_from_bundle(sets, slot);
    if let Some(clause) = set.find_by_id_mut(ident) {
        proof_state_delete_global_indexed_clause(indices, terms, clause, lambda_demod);
    }
}

fn proof_state_delete_global_indexed_clause(
    indices: &mut GlobalIndices,
    bank: &TermBank,
    clause: &mut Clause,
    lambda_demod: bool,
) {
    if clause.query_prop(CP_IS_GLOBAL_INDEXED) {
        indices.delete_clause(clause, bank, lambda_demod);
    }
}

fn clause_split_method(method: SplitType) -> ClauseSplitMethod {
    match method {
        SplitType::GroundNone => ClauseSplitMethod::GroundNone,
        SplitType::GroundOne => ClauseSplitMethod::GroundOne,
        SplitType::GroundFull => ClauseSplitMethod::GroundFull,
    }
}

fn controlled_split_class_matches(clause: &Clause, which: SplitClassType) -> bool {
    which != SplitClassType::NONE
        && ((clause.is_horn() && which.contains(SplitClassType::HORN))
            || (!clause.is_horn() && which.contains(SplitClassType::NON_HORN))
            || (clause.is_negative() && which.contains(SplitClassType::NEGATIVE))
            || (clause.is_positive() && which.contains(SplitClassType::POSITIVE))
            || (clause.is_mixed() && which.contains(SplitClassType::MIXED)))
}

fn proof_state_split_clause(
    state: &mut ProofState,
    clause: Clause,
    method: ClauseSplitMethod,
    fresh_defs: bool,
) -> Result<ClauseSplitOutcome, Diagnostic> {
    let (terms, definitions, predicates, formula_parents, archive) =
        state.terms_and_definition_store_mut();
    let mut store = SplitDefinitionStore::with_formula_archive(
        definitions,
        predicates,
        formula_parents,
        archive,
    );
    clause_split(terms, Some(&mut store), clause, method, fresh_defs)
}

fn formula_parents_for_clause_operation(
    clause: &Clause,
    operation: i64,
) -> Vec<FormulaDerivationRef> {
    derivation_entries(clause)
        .windows(2)
        .filter_map(|entry| match entry {
            [DerivationEntry::Operation(found), DerivationEntry::FormulaParent(parent)]
                if *found == operation =>
            {
                Some(*parent)
            }
            _ => None,
        })
        .collect()
}

fn split_definition_formula_by_ident(
    state: &ProofState,
    ident: i64,
) -> Result<WrappedFormula, Diagnostic> {
    state
        .definition_formula_archive()
        .iter()
        .find(|formula| formula.ident() == ident)
        .cloned()
        .ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                format!("split proof documentation is missing definition formula {ident}"),
            )
        })
}

fn remap_split_formula_parent_refs(
    clause: &mut Clause,
    old_parent: FormulaDerivationRef,
    new_parent: FormulaDerivationRef,
) {
    if old_parent == new_parent {
        return;
    }

    let Some(mut derivation) = clause.take_derivation() else {
        return;
    };
    for entry in derivation.as_mut_slice() {
        if *entry == DerivationEntry::FormulaParent(old_parent) {
            *entry = DerivationEntry::FormulaParent(new_parent);
        }
    }
    clause.set_derivation(Some(derivation));
}

fn update_split_definition_formula_ref(
    state: &mut ProofState,
    old_parent: FormulaDerivationRef,
    new_parent: FormulaDerivationRef,
    new_properties: crate::clauses::clause_props::FormulaProperties,
) -> Result<(), Diagnostic> {
    if old_parent == new_parent {
        if let Some(formula) = state
            .definition_formula_archive_mut()
            .iter_mut()
            .find(|formula| formula.ident() == old_parent.ident())
        {
            formula.set_properties(new_properties);
        }
        return Ok(());
    }

    let Some(formula) = state
        .definition_formula_archive_mut()
        .iter_mut()
        .find(|formula| formula.ident() == old_parent.ident())
    else {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            format!(
                "split proof documentation cannot update missing definition formula {}",
                old_parent.ident()
            ),
        ));
    };
    formula.set_ident(new_parent.ident());
    formula.set_properties(new_properties);

    for parent in state.definition_formula_assocs_mut().values_mut() {
        if *parent == old_parent {
            *parent = new_parent;
        }
    }
    Ok(())
}

fn document_split_definition_formula<W: fmt::Write>(
    output: &mut W,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    parent: FormulaDerivationRef,
    remapped_parents: &mut BTreeMap<i64, FormulaDerivationRef>,
) -> Result<FormulaDerivationRef, Diagnostic> {
    if let Some(parent) = remapped_parents.get(&parent.ident()).copied() {
        return Ok(parent);
    }

    let formula = split_definition_formula_by_ident(state, parent.ident())?;
    let (new_parent, new_properties) = {
        let rendered = formula.proof_doc_formula_body_string(
            state.terms_mut(),
            session.step_options.full_terms,
            session.problem_type,
        )?;
        let mut view = formula.proof_doc_view(&rendered);
        session.doc_intro_split_def(output, &mut view)?;
        (
            FormulaDerivationRef::new_with_source(view.ident(), parent.source()),
            view.properties(),
        )
    };

    update_split_definition_formula_ref(state, parent, new_parent, new_properties)?;
    remapped_parents.insert(parent.ident(), new_parent);
    Ok(new_parent)
}

fn remapped_split_parent(
    parent: FormulaDerivationRef,
    remapped_parents: &BTreeMap<i64, FormulaDerivationRef>,
) -> FormulaDerivationRef {
    remapped_parents
        .get(&parent.ident())
        .copied()
        .unwrap_or(parent)
}

fn proof_state_document_split_clauses<W: fmt::Write>(
    output: &mut W,
    session: &mut ProofDocSession,
    state: &mut ProofState,
    clauses: &mut [Clause],
) -> Result<(), Diagnostic> {
    let mut remapped_parents = BTreeMap::new();

    for clause in clauses.iter_mut() {
        let parents = formula_parents_for_clause_operation(clause, DC_SPLIT_EQUIV);
        let Some(original_parent) = parents.first().copied() else {
            continue;
        };
        let parent = document_split_definition_formula(
            output,
            session,
            state,
            original_parent,
            &mut remapped_parents,
        )?;
        remap_split_formula_parent_refs(clause, original_parent, parent);
        let formula = split_definition_formula_by_ident(state, parent.ident())?;
        let rendered = formula.proof_doc_formula_body_string(
            state.terms_mut(),
            session.step_options.full_terms,
            session.problem_type,
        )?;
        let parent_view = formula.proof_doc_view(&rendered);
        session.doc_intro_split_def_rest(output, state.terms(), clause, &parent_view, None)?;
    }

    for clause in clauses.iter_mut() {
        let original_id = clause.ident();
        let parents = formula_parents_for_clause_operation(clause, DC_APPLY_DEF);
        if parents.is_empty() {
            continue;
        }

        let mut def_ids = Vec::with_capacity(parents.len());
        for parent in parents {
            let mapped = remapped_split_parent(parent, &remapped_parents);
            remap_split_formula_parent_refs(clause, parent, mapped);
            def_ids.push(mapped.ident());
        }
        session.doc_clause_apply_defs(
            output,
            state.terms(),
            clause,
            original_id,
            &def_ids,
            None,
        )?;
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
        let ProofControl {
            hcbs, wfcbs, ocb, ..
        } = control;
        let active_hcb = hcbs
            .hcb(active_hcb_handle)
            .ok_or_else(|| unknown_heuristic_handle("active"))?;
        let ocb = ocb.as_mut().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "eval_clause_set requires initialized proof-control ordering",
            )
        })?;

        for _ in 0..pending {
            let Some(mut clause) = state.eval_store_mut().extract_first() else {
                return Err(Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "eval_clause_set eval_store changed while evaluating clauses",
                ));
            };
            let evaluation = hcb_clause_evaluate_with_bank(
                active_hcb,
                wfcbs,
                ocb,
                state.terms_mut(),
                &mut clause,
            );
            if let Err(err) = evaluation {
                state.eval_store_mut().insert(clause);
                return Err(err);
            }
            state.eval_store_mut().insert(clause);
        }
    }

    Ok(pending)
}

/// Moves evaluated clauses from `eval_store` to `unprocessed`.
///
/// This is the final queueing tail of C `insert_new_clauses` after
/// [`proof_state_eval_clause_set`] has attached evaluations. Use
/// [`proof_state_move_eval_store_to_unprocessed_with_docs`] for the represented
/// proof-documentation `eval` quote side effect.
pub fn proof_state_move_eval_store_to_unprocessed(state: &mut ProofState) -> i64 {
    let mut moved = 0;
    while let Some(mut clause) = state.eval_store_mut().extract_first() {
        clause.del_prop(CP_IS_ORIENTED);
        state.unprocessed_mut().insert(clause);
        moved += 1;
    }
    moved
}

/// Moves evaluated clauses from `eval_store` to `unprocessed` while emitting
/// the represented C `DocClauseQuoteDefault(6, handle, "eval")` step.
///
/// # Errors
///
/// Returns any proof-documentation write diagnostic.
pub fn proof_state_move_eval_store_to_unprocessed_with_docs(
    output: &mut impl fmt::Write,
    session: &mut ProofDocSession,
    state: &mut ProofState,
) -> Result<i64, Diagnostic> {
    let mut moved = 0;
    while let Some(mut clause) = state.eval_store_mut().extract_first() {
        clause.del_prop(CP_IS_ORIENTED);
        session.doc_clause_quote(output, state.terms(), 6, &mut clause, Some("eval"), None)?;
        state.unprocessed_mut().insert(clause);
        moved += 1;
    }
    Ok(moved)
}

/// Runs C `check_ac_status` for one newly processed clause.
///
/// Returns true when this call newly activates AC handling. Use
/// [`proof_state_check_ac_status_with_output`] when the C `OutputLevel`
/// side effect should be rendered too.
#[must_use]
pub fn proof_state_check_ac_status(
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
) -> bool {
    if control.heuristic_parms().ac_handling == AcHandling::None {
        return false;
    }

    let detected_commutativity = clause_scan_ac(state.terms_mut().signature_mut(), clause);
    if detected_commutativity && !control.ac_handling_active() {
        control.set_ac_handling_active(true);
        return true;
    }
    false
}

/// Runs C `check_ac_status` and renders the represented `OutputLevel` output.
///
/// # Errors
///
/// Returns a diagnostic if the output sink fails while printing the signature
/// AC status or activation line.
pub fn proof_state_check_ac_status_with_output(
    output: &mut (impl std::io::Write + ?Sized),
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
    clause: &mut Clause,
) -> Result<bool, Diagnostic> {
    let activated = proof_state_check_ac_status(state, control, clause);
    if activated && output_level != 0 {
        state
            .terms()
            .signature()
            .print_ac_status(output)
            .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
        std::io::Write::write_all(output, DEFAULT_COMCHAR_RAW.as_bytes())
            .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
        std::io::Write::write_all(output, b" AC handling enabled dynamically\n")
            .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
    }
    Ok(activated)
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

/// Runs the AC-axiom scan portion of C `ProofStateInit` and renders its
/// represented `OutputLevel` output.
///
/// # Errors
///
/// Returns a diagnostic if the output sink fails while printing the scan
/// banner, signature AC status, or activation line.
pub fn proof_state_init_ac_handling_with_output(
    output: &mut (impl std::io::Write + ?Sized),
    output_level: i64,
    state: &mut ProofState,
    control: &mut ProofControl,
) -> Result<bool, Diagnostic> {
    let ac_handling_enabled = control.heuristic_parms().ac_handling != AcHandling::None;
    if ac_handling_enabled && output_level != 0 {
        std::io::Write::write_all(output, DEFAULT_COMCHAR_RAW.as_bytes())
            .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
        std::io::Write::write_all(output, b" Scanning for AC axioms\n")
            .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
    }

    let active = proof_state_init_ac_handling(state, control);
    if ac_handling_enabled && output_level != 0 {
        state
            .terms()
            .signature()
            .print_ac_status(output)
            .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
        if active {
            std::io::Write::write_all(output, DEFAULT_COMCHAR_RAW.as_bytes())
                .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
            std::io::Write::write_all(output, b" AC handling enabled\n")
                .map_err(|err| Diagnostic::new(ErrorCode::OTHER_ERROR, err.to_string()))?;
        }
    }
    Ok(active)
}

/// Runs the global-index free/init tail of C `ProofStateInit`.
///
/// This mirrors `GlobalIndicesFreeIndices(&state->gindices)` followed by
/// `GlobalIndicesInit(...)`. The problem type is explicit instead of reading
/// C's process-global `problemType`.
pub fn proof_state_init_global_indices(
    state: &mut ProofState,
    control: &ProofControl,
    problem_type: ProblemType,
) {
    let params = control.heuristic_parms();
    state.global_indices_mut().init_for_problem(
        params.rw_bw_index_type.as_str(),
        params.pm_from_index_type.as_str(),
        params.pm_into_index_type.as_str(),
        params.ext_rules_max_depth,
        problem_type,
    );
}

/// Initializes the proof state's watchlist indices with rewriting only.
///
/// This mirrors C `ProofStateAlloc`, which configures `state->wlindices` with
/// the backward-rewrite index type and disables paramodulation indexes.
pub fn proof_state_init_watchlist_global_indices(
    state: &mut ProofState,
    control: &ProofControl,
    problem_type: ProblemType,
) {
    let params = control.heuristic_parms();
    state.watchlist_indices_mut().init_for_problem(
        params.rw_bw_index_type.as_str(),
        "NoIndex",
        "NoIndex",
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
    events: Option<&mut Vec<HeuristicAdminEvent>>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_internal_string(DEFAULT_WEIGHT_FUNCTIONS, true)?;
    parse_weight_function_definitions(control, &mut scanner, context, events)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_option_weight_functions(
    control: &mut ProofControl,
    definition: &str,
    context: WeightParseContext<'_>,
    events: Option<&mut Vec<HeuristicAdminEvent>>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_option_string(definition, true)?;
    parse_weight_function_definitions(control, &mut scanner, context, events)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_default_heuristics(
    control: &mut ProofControl,
    context: WeightParseContext<'_>,
    events: Option<&mut Vec<HeuristicAdminEvent>>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_internal_string(DEFAULT_HEURISTICS, true)?;
    parse_heuristic_definitions(control, &mut scanner, context, events)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn install_option_heuristics(
    control: &mut ProofControl,
    definition: &str,
    context: WeightParseContext<'_>,
    events: Option<&mut Vec<HeuristicAdminEvent>>,
) -> Result<(), Diagnostic> {
    let mut scanner = Scanner::from_option_string(definition, true)?;
    parse_heuristic_definitions(control, &mut scanner, context, events)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

fn parse_weight_function_definitions(
    control: &mut ProofControl,
    scanner: &mut Scanner,
    context: WeightParseContext<'_>,
    mut events: Option<&mut Vec<HeuristicAdminEvent>>,
) -> Result<(), Diagnostic> {
    while scanner.test_tok(TokenType::IDENTIFIER)
        && scanner
            .look_token(1)
            .kind()
            .intersects(TokenType::EQUAL_SIGN | TokenType::OPEN_BRACKET)
    {
        let name = control
            .wfcbs
            .weight_fun_def_parse_with_context(scanner, context)?;
        if let Some(events) = events.as_deref_mut() {
            events.push(HeuristicAdminEvent::WeightFunction(name));
        }
    }
    Ok(())
}

fn parse_heuristic_definitions(
    control: &mut ProofControl,
    scanner: &mut Scanner,
    context: WeightParseContext<'_>,
    mut events: Option<&mut Vec<HeuristicAdminEvent>>,
) -> Result<(), Diagnostic> {
    while (scanner.test_tok(TokenType::IDENTIFIER)
        && scanner
            .look_token(1)
            .kind()
            .intersects(TokenType::EQUAL_SIGN))
        || scanner.test_tok(TokenType::OPEN_BRACKET)
    {
        let weight_start = control.wfcbs.len();
        let heuristic_start = control.hcbs.len();
        control
            .hcbs
            .heuristic_def_parse_with_context(scanner, &mut control.wfcbs, context)?;
        record_admin_additions(
            control,
            weight_start,
            heuristic_start,
            events.as_deref_mut(),
        );
    }
    Ok(())
}

fn record_admin_additions(
    control: &ProofControl,
    weight_start: usize,
    heuristic_start: usize,
    events: Option<&mut Vec<HeuristicAdminEvent>>,
) {
    let Some(events) = events else {
        return;
    };
    for index in weight_start..control.wfcbs.len() {
        if let Some(name) = control.wfcbs.name(index) {
            events.push(HeuristicAdminEvent::WeightFunction(name.to_owned()));
        }
    }
    for index in heuristic_start..control.hcbs.len() {
        if let Some(name) = control.hcbs.name(index) {
            events.push(HeuristicAdminEvent::Heuristic(name.to_owned()));
        }
    }
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
/// through the bankless selector entry point.
///
/// # Errors
///
/// Returns `UnsupportedLiteralSelection` if the configured selector is unknown,
/// or if the wrapper reaches a selector that requires a term bank.
pub fn do_literal_selection(
    control: &mut ProofControl,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, UnsupportedLiteralSelection> {
    do_literal_selection_impl(control, None, clause)
}

/// Runs the C `DoLiteralSelection` wrapper using ported selector bodies and a
/// mutable term bank for selector bodies that need bank-backed ordering
/// preparation during maximality marking.
///
/// # Errors
///
/// Returns [`LiteralSelectionError`] if the configured selector is unknown,
/// required selector context is missing, or bank-backed ordering preparation
/// fails.
pub fn do_literal_selection_with_bank(
    control: &mut ProofControl,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, LiteralSelectionError> {
    do_literal_selection_with_mut_bank_impl(control, bank, clause)
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

fn do_literal_selection_with_mut_bank_impl(
    control: &mut ProofControl,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<LiteralSelectionOutcome, LiteralSelectionError> {
    clear_literal_selection_state(clause);
    let parms = control.heuristic_parms();
    if should_try_inherited_selection(parms, clause) && select_inherited_literal(clause) {
        return Ok(LiteralSelectionOutcome::Inherited);
    }
    if literal_selection_conditions_hold(parms, clause) {
        debug_assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 0);
        apply_ported_literal_selector_with_mut_bank(
            control.heuristic_parms.selection_strategy.as_str(),
            control.ocb.as_mut(),
            Some(bank),
            clause,
        )?;
        Ok(LiteralSelectionOutcome::SelectorApplied)
    } else {
        crate::heuristics::litselection::select_no_literals(control.ocb.as_mut(), clause);
        Ok(LiteralSelectionOutcome::SelectionSkipped)
    }
}

fn literal_selection_error_to_diagnostic(error: LiteralSelectionError) -> Diagnostic {
    match error {
        LiteralSelectionError::Unsupported(error) => {
            Diagnostic::new(ErrorCode::OTHER_ERROR, error.to_string())
        }
        LiteralSelectionError::Ordering(error) => error,
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
        apply_terms, close_with_db_var, compute_ext_eq_fact, compute_ext_eq_res, compute_ext_sup,
        do_literal_selection, do_literal_selection_with_bank, do_literal_selection_with_selector,
        preinstantiate_induction, proof_control_alloc,
        proof_control_clause_set_filter_reweigth_with_bank,
        proof_control_clause_set_reweight_with_bank, proof_control_init,
        proof_control_init_heuristics, proof_control_init_heuristics_with_formula_axioms,
        proof_control_reset_sat_solver, proof_state_archive_simplified_clause,
        proof_state_backward_simplify_with_global_indices, proof_state_check_ac_status,
        proof_state_check_ac_status_with_output, proof_state_check_watchlist_with_docs,
        proof_state_check_watchlist_with_global_indices, proof_state_check_watchlist_with_output,
        proof_state_cleanup_unprocessed_clauses, proof_state_cleanup_unprocessed_clauses_with,
        proof_state_cleanup_unprocessed_clauses_with_docs, proof_state_eval_clause_set,
        proof_state_filter_unprocessed, proof_state_forward_contract_clause,
        proof_state_forward_contract_clause_with_docs, proof_state_forward_contract_set,
        proof_state_forward_contract_set_reweight,
        proof_state_forward_contract_set_reweight_with_docs,
        proof_state_forward_contract_set_with_docs, proof_state_forward_modify_clause,
        proof_state_forward_modify_clause_impl, proof_state_forward_modify_clause_with_docs,
        proof_state_forward_subsumption, proof_state_forward_subsumption_with_bank,
        proof_state_forward_subsumption_with_strong, proof_state_generate_new_clauses,
        proof_state_generate_new_clauses_impl, proof_state_generate_new_clauses_with_docs,
        proof_state_generate_new_clauses_with_global_indices,
        proof_state_generate_new_clauses_with_global_indices_and_docs,
        proof_state_immediate_clausification, proof_state_immediate_clausification_with_docs,
        proof_state_init, proof_state_init_ac_handling, proof_state_init_ac_handling_with_output,
        proof_state_init_global_indices, proof_state_init_indexing,
        proof_state_init_watchlist_global_indices, proof_state_init_with_docs,
        proof_state_init_with_global_indices, proof_state_init_with_output,
        proof_state_insert_new_clauses, proof_state_insert_new_clauses_with_docs,
        proof_state_insert_new_clauses_with_output, proof_state_insert_processed_clause,
        proof_state_insert_watchlist_global_indices_into,
        proof_state_move_eval_store_to_unprocessed,
        proof_state_move_eval_store_to_unprocessed_with_docs, proof_state_move_to_tmp_store,
        proof_state_move_to_tmp_store_with_global_indices, proof_state_process_clause,
        proof_state_process_clause_with_docs, proof_state_process_clause_with_global_indices,
        proof_state_process_clause_with_output, proof_state_queue_generated_clause_for_eval,
        proof_state_recognize_choice_axioms, proof_state_replacing_inferences,
        proof_state_replacing_inferences_with_docs, proof_state_reset_processed,
        proof_state_reset_processed_with_docs, proof_state_reset_processed_with_global_indices,
        proof_state_saturate, proof_state_saturate_with_global_and_watchlist_indices_and_docs,
        proof_state_saturate_with_global_indices, proof_state_saturate_with_output,
        proof_state_select_unprocessed_clause, proof_state_simplify_watchlist,
        proof_state_simplify_watchlist_with_docs,
        proof_state_simplify_watchlist_with_global_indices, proof_state_storage_estimate,
        select_inherited_literal, selection_parent_is_dead, write_cleanup_unprocessed_output,
        BackwardSimplificationOutcome, CleanupUnprocessedOutcome, ForwardContractCounts,
        ForwardContractOptions, GenerateNewClausesOutcome, LiteralSelectionOutcome,
        ParentLivenessSnapshot, ProcessClauseOutcome, ProcessClauseReturnReason,
        ProcessedClauseClass, ProofStateWatchlistOutcome, ReplacingInferenceOutcome,
        SatSolverBackendKind, SaturateOutcome, SaturateReturnReason, SaturateStopReason,
        DEFAULT_HEURISTICS, DEFAULT_WEIGHT_FUNCTIONS,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::simple_stuff::{
        reset_jkiss_for_tests, reset_problem_type, set_problem_type, ProblemType,
    };
    use crate::basics::sysdate::SysDate;
    use crate::clauses::clause::{clause_print_lop_format_string, Clause};
    use crate::clauses::clause_props::{
        CP_INITIAL, CP_INPUT_FORMULA, CP_IS_DEAD, CP_IS_GLOBAL_INDEXED, CP_IS_ORIENTED,
        CP_IS_PROCESSED, CP_IS_PURE_INJECTIVITY, CP_IS_SOS, CP_IS_S_INDEXED, CP_LIMITED_RW,
        CP_NO_GENERATION, CP_SUBSUMES_WATCH, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE,
        CP_TYPE_NEG_CONJECTURE, CP_WATCH_ONLY,
    };
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::derivation::{
        clause_push_derivation, ClauseDerivationRef, DerivationEntry, DerivationParentRef,
        DC_AC_RES, DC_ARG_CONG, DC_CHOICE_AX, DC_CHOICE_INST, DC_CNF_EVAL_GC, DC_CNF_QUOTE,
        DC_CONDENSE, DC_CONTEXT_SR, DC_DES_EQ_RES, DC_DYNAMIC_CNF, DC_EXT_EQ_FACT, DC_EXT_EQ_RES,
        DC_EXT_SUP, DC_INV_REC, DC_LEIBNIZ_ELIM, DC_LOCAL_REWRITE, DC_NEG_EXT, DC_NORMALIZE,
        DC_ORDERED_FACTOR, DC_POS_EXT, DC_PRIM_ENUM, DC_PRUNE_ARG, DC_SR, DC_TRIGGER,
    };
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{
        EP_IS_MAXIMAL, EP_IS_ORIENTED, EP_IS_PM_INTO_LIT, EP_IS_SELECTED, EP_IS_SPLIT_LIT,
        EP_MAX_IS_UP_TO_DATE,
    };
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::fcvindexing::{fv_index_pack_clause, FvIndexParams};
    use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
    use crate::clauses::freqvectors::{FvIndexType, FVINDEX_MAX_FEATURES_DEFAULT};
    use crate::clauses::global_indices::GlobalIndices;
    use crate::clauses::inferencedoc::{ProofDocOutputFormat, ProofDocSession};
    use crate::clauses::neweval::{evals_alloc, PRIO_LARGEST_REASONABLE, PRIO_NORMAL};
    use crate::clauses::picosat::PicoSatError;
    use crate::clauses::proofstate::{proof_state_alloc, ProofState, WatchlistSource};
    use crate::clauses::subsumption::clause_subsume_order_sort_lits;
    use crate::heuristics::hcb::{
        hcb_clause_evaluate_with_bank, AcHandling, ExtInferenceType, GroundingStrategy,
        HeuristicParmsCell, ParamodulationType as HcbParamodulationType, PrimEnumMode,
        SplitClassType, SplitType, HCB_DEFAULT_HEURISTIC,
    };
    use crate::heuristics::litselection::{
        NO_GENERATION, SELECT_NEGATIVE_LITERALS, SELECT_UNLESS_POS_MAX,
    };
    use crate::heuristics::to_params::TermOrdering;
    use crate::inout::scanner::IoFormat;
    use crate::inout::signals::{configure_time_limits, RLIM_INFINITY_COMPAT};
    use crate::learn::numfeatures::FEATURE_NUMBER;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::{
        Signature, FP_ASSOCIATIVE, FP_COMMUTATIVE, FP_DEF_PRED, FP_IGNORE_PROPS,
    };
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, RewriteLevel, Term, TP_IS_REWRITABLE};
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;
    use std::{collections::BTreeMap, path::Path};

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

    fn typed_arrow_const(bank: &mut TermBank, name: &str, arg_count: usize) -> Term {
        let i_type = bank.signature().type_bank().default_type();
        let mut type_args = vec![i_type.clone(); arg_count];
        type_args.push(i_type);
        let symbol_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(type_args));
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, symbol_type)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn typed_var(bank: &TermBank, f_code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn unary_predicate_var(bank: &mut TermBank, f_code: i64) -> Term {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let type_ = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type, bool_type]));
        bank.vars().var_assert_alloc(f_code, &type_)
    }

    fn unary_predicate_const(bank: &mut TermBank, name: &str) -> Term {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let type_ = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type, bool_type]));
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, type_)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn predicate_argument_binary_const(bank: &mut TermBank, name: &str) -> Term {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), bool_type]));
        let symbol_type =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    predicate_type,
                    arg_type.clone(),
                    arg_type,
                ]));
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, symbol_type)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn choice_const(bank: &mut TermBank, name: &str) -> Term {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), bool_type]));
        let choice_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![predicate_type, arg_type]));
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, choice_type)
                .unwrap();
        }
        bank.create_const_term(f_code).unwrap()
    }

    fn choice_axiom(bank: &mut TermBank, name: &str, p_code: i64, x_code: i64) -> (Clause, i64) {
        let predicate = unary_predicate_var(bank, p_code);
        let witness = typed_var(bank, x_code);
        let choice = choice_const(bank, name);
        let choice_applied = apply_terms(bank, &choice, std::slice::from_ref(&predicate)).unwrap();
        let negative_atom = apply_terms(bank, &predicate, std::slice::from_ref(&witness)).unwrap();
        let positive_atom =
            apply_terms(bank, &predicate, std::slice::from_ref(&choice_applied)).unwrap();
        let true_term = bank.true_term().clone();
        let clause = Clause::alloc(EqnList::from_vec(vec![
            Eqn::alloc(negative_atom, true_term.clone(), bank, false).unwrap(),
            Eqn::alloc(positive_atom, true_term, bank, true).unwrap(),
        ]));
        (clause, choice.f_code())
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

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_]))
                .unwrap();
        }
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bank.signature().type_bank().default_type()));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn unary_predicate_code(bank: &mut TermBank, name: &str) -> i64 {
        let arg_type = bank.signature().type_bank().default_type();
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        if bank.signature().get_type(f_code).is_none() {
            bank.signature_mut()
                .declare_type(f_code, alloc_arrow_type(vec![arg_type, bool_type]))
                .unwrap();
        }
        f_code
    }

    fn unary_predicate(bank: &mut TermBank, f_code: i64, arg: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(bool_type));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never).unwrap()
    }

    fn bool_binary_with_code(bank: &mut TermBank, f_code: i64, left: &Term, right: &Term) -> Term {
        let bool_type = bank.signature().type_bank().bool_type();
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(bool_type));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.term_top_insert(term).unwrap()
    }

    struct TimeLimitsReset;

    impl Drop for TimeLimitsReset {
        fn drop(&mut self) {
            configure_time_limits(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        }
    }

    struct ProblemTypeReset;

    impl Drop for ProblemTypeReset {
        fn drop(&mut self) {
            reset_problem_type();
        }
    }

    #[must_use]
    fn configure_time_limits_for_test(
        hard_limit: u64,
        soft_limit: u64,
        schedule_limit: u64,
    ) -> TimeLimitsReset {
        configure_time_limits(hard_limit, soft_limit, schedule_limit);
        TimeLimitsReset
    }

    fn set_problem_type_for_test(problem_type: ProblemType) -> ProblemTypeReset {
        reset_problem_type();
        set_problem_type(problem_type).unwrap_or_else(|err| panic!("{err}"));
        ProblemTypeReset
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn immediate_definition_clause(bank: &mut TermBank, ident: i64) -> Clause {
        let eqn_code = bank.signature_mut().get_eqn_code(true);
        let and_code = bank.signature().and_code();
        let or_code = bank.signature().or_code();
        let mut atoms = Vec::new();
        for index in 0..10 {
            let left = typed_const(bank, &format!("immediate_def_{ident}_left_{index}"));
            let right = typed_const(bank, &format!("immediate_def_{ident}_right_{index}"));
            atoms.push(bool_binary_with_code(bank, eqn_code, &left, &right));
        }
        let mut conjunctions = Vec::new();
        let mut atoms = atoms.into_iter();
        while let Some(left) = atoms.next() {
            let right = atoms.next().expect("fixture has an even atom count");
            conjunctions.push(bool_binary_with_code(bank, and_code, &left, &right));
        }
        let mut expensive = conjunctions.remove(0);
        for conjunction in conjunctions {
            expensive = bool_binary_with_code(bank, or_code, &expensive, &conjunction);
        }
        let tail_left = typed_const(bank, &format!("immediate_def_{ident}_tail_left"));
        let tail_right = typed_const(bank, &format!("immediate_def_{ident}_tail_right"));
        let tail = bool_binary_with_code(bank, eqn_code, &tail_left, &tail_right);
        let formula = bool_binary_with_code(bank, or_code, &expensive, &tail);
        let truth = bank.true_term().clone();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
            bank, &formula, &truth, true,
        )]));
        clause.set_ident(ident);
        clause
    }

    fn derivation_contains_operation(clause: &Clause, operation: i64) -> bool {
        clause.derivation().is_some_and(|derivation| {
            derivation
                .as_slice()
                .contains(&DerivationEntry::Operation(operation))
        })
    }

    fn derivation_contains_parent(clause: &Clause, parent_ident: i64) -> bool {
        clause.derivation().is_some_and(|derivation| {
            derivation
                .as_slice()
                .contains(&DerivationEntry::ClauseParent(ClauseDerivationRef::new(
                    parent_ident,
                    0,
                )))
        })
    }

    #[test]
    fn preinstantiate_induction_generates_trigger_instance_from_conjecture_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (source, target, source_ident, target_ident) = {
            let bank = state.terms_mut();
            let a = typed_const(bank, "pi_clause_a");
            let b = typed_const(bank, "pi_clause_b");
            let f_a = typed_unary(bank, "pi_clause_f", &a);
            let p_code = unary_predicate_code(bank, "pi_clause_p");
            let p_a = unary_predicate(bank, p_code, &a);
            let p_f_a = unary_predicate(bank, p_code, &f_a);
            let mut source =
                Clause::alloc(EqnList::from_vec(vec![literal(bank, &p_a, &p_f_a, true)]));
            source.set_tptp_type(CP_TYPE_CONJECTURE);
            source.set_ident(71_001);

            let predicate = unary_predicate_var(bank, -42);
            let predicate_b = apply_terms(bank, &predicate, std::slice::from_ref(&b)).unwrap();
            let true_term = bank.true_term().clone();
            let mut target = Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
                predicate_b,
                true_term,
                bank,
                true,
            )
            .unwrap()]));
            target.set_ident(71_002);
            (source, target, 71_001, 71_002)
        };
        state.axioms_mut().insert(source);
        state.axioms_mut().insert(target);

        let generated = preinstantiate_induction(&mut state).unwrap();

        assert_eq!(generated, 1);
        let generated_clause = state
            .axioms()
            .iter()
            .find(|clause| derivation_contains_operation(clause, DC_TRIGGER))
            .expect("trigger instance should be inserted into active axioms");
        assert!(derivation_contains_parent(generated_clause, target_ident));
        assert!(derivation_contains_parent(generated_clause, source_ident));
    }

    #[test]
    fn preinstantiate_induction_uses_archived_quantified_formula_triggers() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (formula, target, target_ident) = {
            let bank = state.terms_mut();
            let b = typed_const(bank, "pi_formula_b");
            let x = typed_var(bank, -44);
            let p_code = unary_predicate_code(bank, "pi_formula_p");
            let p_x = unary_predicate(bank, p_code, &x);
            let qall_code = bank.signature().qall_code();
            let quantified = super::tformula_fcode_alloc(bank, qall_code, x, Some(p_x)).unwrap();
            let mut formula = WrappedFormula::wt_formula_alloc(quantified);
            formula.set_tptp_type(CP_TYPE_CONJECTURE);
            formula.set_ident(72_001);

            let predicate = unary_predicate_var(bank, -46);
            let predicate_b = apply_terms(bank, &predicate, std::slice::from_ref(&b)).unwrap();
            let true_term = bank.true_term().clone();
            let mut target = Clause::alloc(EqnList::from_vec(vec![Eqn::alloc(
                predicate_b,
                true_term,
                bank,
                true,
            )
            .unwrap()]));
            target.set_ident(72_002);
            (formula, target, 72_002)
        };
        state.f_ax_archive_mut().insert(formula);
        state.axioms_mut().insert(target);

        let generated = preinstantiate_induction(&mut state).unwrap();

        assert_eq!(generated, 1);
        assert_eq!(state.archive().members(), 1);
        let generated_clause = state
            .axioms()
            .iter()
            .find(|clause| derivation_contains_operation(clause, DC_TRIGGER))
            .expect("formula trigger instance should be inserted into active axioms");
        assert!(derivation_contains_parent(generated_clause, target_ident));
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
        signed_unit_clause_with_id(bank, stem, ident, true)
    }

    fn signed_unit_clause_with_id(
        bank: &mut TermBank,
        stem: &str,
        ident: i64,
        positive: bool,
    ) -> Clause {
        let left = typed_const(bank, &format!("{stem}_left"));
        let right = typed_const(bank, &format!("{stem}_right"));
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
            bank, &left, &right, positive,
        )]));
        clause.set_ident(ident);
        clause
    }

    fn answer_clause_with_id(bank: &mut TermBank, stem: &str, ident: i64) -> Clause {
        let witness = typed_const(bank, &format!("{stem}_witness"));
        let answer_code = bank.signature().answer_code();
        let answer = unary_predicate(bank, answer_code, &witness);
        let truth = bank.true_term().clone();
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
            bank, &answer, &truth, true,
        )]));
        clause.set_ident(ident);
        clause
    }

    fn processed_unit_clause(bank: &mut TermBank, stem: &str, ident: i64) -> Clause {
        let mut clause = unit_clause_with_id(bank, stem, ident);
        clause.set_prop(CP_IS_PROCESSED | CP_IS_ORIENTED);
        clause
    }

    fn processed_indexed_unit_clause(
        bank: &mut TermBank,
        stem: &str,
        ident: i64,
    ) -> (Clause, Term) {
        let left = typed_const(bank, &format!("{stem}_left"));
        let right = typed_const(bank, &format!("{stem}_right"));
        let mut literal = literal(bank, &left, &right, true);
        literal.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
        clause.set_ident(ident);
        clause.set_prop(CP_IS_PROCESSED | CP_IS_ORIENTED);
        (clause, left)
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

    fn init_process_clause_control(control: &mut super::ProofControl, state: &ProofState) {
        init_fifo_hcb(control, state, "ProcessClauseTest");
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().selection_strategy = NO_GENERATION.to_owned();
    }

    fn queue_unprocessed_for_process(
        state: &mut ProofState,
        control: &mut super::ProofControl,
        clause: Clause,
    ) {
        state.unprocessed_mut().insert(clause);
        let mut unprocessed = std::mem::take(state.unprocessed_mut());
        proof_control_clause_set_reweight_with_bank(control, state.terms_mut(), &mut unprocessed)
            .unwrap_or_else(|err| panic!("{err}"));
        *state.unprocessed_mut() = unprocessed;
    }

    fn kbo_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn kbo6_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn empty_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Empty,
            false,
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
        assert_eq!(
            control.sat_solver_backend_kind(),
            SatSolverBackendKind::Internal
        );
    }

    #[test]
    fn proof_control_reset_sat_solver_reinitializes_trace_state() {
        let mut control = proof_control_alloc();

        proof_control_reset_sat_solver(&mut control).unwrap();

        assert_eq!(control.solver().generation(), 2);
        assert!(control.solver().trace_generation_enabled());
    }

    #[test]
    fn proof_control_keeps_internal_backend_after_missing_picosat_install() {
        let mut control = proof_control_alloc();

        let error = control
            .install_picosat_solver(Path::new("missing-picosat-for-proof-control-test.dll"))
            .unwrap_err();

        assert!(matches!(error, PicoSatError::LoadLibrary { .. }));
        assert_eq!(
            control.sat_solver_backend_kind(),
            SatSolverBackendKind::Internal
        );
        assert_eq!(control.solver().generation(), 1);
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
    fn proof_state_insert_watchlist_global_indices_indexes_rebuilt_watchlist() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut watch = {
            let terms = state.terms_mut();
            negative_clause(terms)
        };
        watch.set_ident(4_010);
        state.watchlist_mut().unwrap().insert(watch);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let indexed = proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| {
            panic!("{err}");
        });
        let mut indices = GlobalIndices::new_for_problem(
            "FP1",
            "NoIndex",
            "NoIndex",
            -1,
            ProblemType::FirstOrder,
        );

        let globally_indexed = proof_state_insert_watchlist_global_indices_into(
            &mut state,
            &mut indices,
            control.heuristic_parms().lambda_demod,
        );

        assert_eq!(indexed, 1);
        assert_eq!(globally_indexed, 1);
        let watch = state.watchlist().unwrap().find_by_id(4_010).unwrap();
        assert!(watch.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices
            .find_bw_rw_occurrence(watch.literals().as_slice()[0].left())
            .is_some());
    }

    #[test]
    fn proof_state_check_watchlist_with_global_indices_deletes_removed_watch() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut subsumer, watched, watched_term) = {
            let terms = state.terms_mut();
            let (subsumer, watched) =
                watchlist_subsumption_pair(terms, "pc_wl_gidx_remove", 4_011, 4_012);
            let watched_term = watched.literals().as_slice()[0].left().clone();
            (subsumer, watched, watched_term)
        };
        state.watchlist_mut().unwrap().insert(watched);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| panic!("{err}"));
        let mut indices = GlobalIndices::new_for_problem(
            "FP1",
            "NoIndex",
            "NoIndex",
            -1,
            ProblemType::FirstOrder,
        );
        assert_eq!(
            proof_state_insert_watchlist_global_indices_into(
                &mut state,
                &mut indices,
                control.heuristic_parms().lambda_demod,
            ),
            1
        );
        assert!(indices.find_bw_rw_occurrence(&watched_term).is_some());

        let outcome = proof_state_check_watchlist_with_global_indices(
            &mut state,
            &mut subsumer,
            false,
            control.heuristic_parms().lambda_demod,
            &mut indices,
        );

        assert_eq!(
            outcome,
            ProofStateWatchlistOutcome {
                subsumes_watch: true,
                removed: 1,
            }
        );
        assert!(subsumer.query_prop(CP_SUBSUMES_WATCH));
        assert_eq!(state.watchlist().unwrap().members(), 0);
        let archived = state.archive().find_by_id(4_012).unwrap();
        assert!(archived.query_prop(CP_IS_DEAD));
        assert!(!archived.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_bw_rw_occurrence(&watched_term).is_none());
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
    fn proof_state_recognize_choice_axioms_uses_ho_and_depth_gates() {
        let _guard = global_state_lock();
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let choice_code = {
            let terms = state.terms_mut();
            let (choice_clause, choice_code) =
                choice_axiom(terms, "pc_init_choice_axiom", -30, -32);
            state.axioms_mut().insert(choice_clause);
            choice_code
        };
        let mut control = proof_control_alloc();

        let fo_reset = set_problem_type_for_test(ProblemType::FirstOrder);
        control.heuristic_parms_mut().inst_choice_max_depth = 0;
        assert_eq!(
            proof_state_recognize_choice_axioms(&mut state, &control).unwrap(),
            0
        );
        assert!(state.choice_opcodes().is_empty());
        drop(fo_reset);

        let _ho = set_problem_type_for_test(ProblemType::HigherOrder);
        control.heuristic_parms_mut().inst_choice_max_depth = -1;
        assert_eq!(
            proof_state_recognize_choice_axioms(&mut state, &control).unwrap(),
            0
        );
        assert!(state.choice_opcodes().is_empty());

        control.heuristic_parms_mut().inst_choice_max_depth = 0;
        assert_eq!(
            proof_state_recognize_choice_axioms(&mut state, &control).unwrap(),
            1
        );
        assert!(state.choice_opcodes().contains_key(&choice_code));
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
            axiom.set_prop(CP_INPUT_FORMULA);

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
        assert!(!copied_axiom.query_prop(CP_INPUT_FORMULA));
        assert!(!copied_axiom.query_prop(CP_IS_SOS));
        assert_eq!(
            copied_axiom
                .derivation()
                .map(crate::basics::pstacks::PStack::as_slice),
            Some(
                &[
                    DerivationEntry::Operation(DC_CNF_QUOTE),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::new(axiom_id, 0)),
                ][..]
            )
        );
        let copied_conjecture = state.unprocessed().find_by_id(conjecture_id).unwrap();
        assert!(copied_conjecture.query_prop(CP_INITIAL | CP_IS_SOS));
        assert_eq!(
            copied_conjecture
                .derivation()
                .map(crate::basics::pstacks::PStack::as_slice),
            Some(
                &[
                    DerivationEntry::Operation(DC_CNF_QUOTE),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::new(conjecture_id, 0)),
                ][..]
            )
        );
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
    fn proof_state_init_records_eval_gc_when_gc_selection_is_enabled() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let axiom = unit_clause_with_id(state.terms_mut(), "pc_init_gc", 4_018);
        let axiom_id = axiom.ident();
        state.axioms_mut().insert(axiom);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InitGcSelection");
        control.set_record_gc_selection(true);

        let outcome = proof_state_init(&mut state, &mut control).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(outcome.initial_clauses, 1);
        let copied = state.unprocessed().find_by_id(axiom_id).unwrap();
        assert_eq!(
            copied
                .derivation()
                .map(crate::basics::pstacks::PStack::as_slice),
            Some(
                &[
                    DerivationEntry::Operation(DC_CNF_QUOTE),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::new(axiom_id, 0)),
                    DerivationEntry::Operation(DC_CNF_EVAL_GC),
                ][..]
            )
        );
    }

    #[test]
    fn proof_state_init_with_docs_emits_initial_eval_quote() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut axiom = unit_clause_with_id(state.terms_mut(), "pc_init_doc_eval", 4_019);
        axiom.set_prop(CP_INPUT_FORMULA);
        let axiom_id = axiom.ident();
        state.axioms_mut().insert(axiom);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InitDocEval");
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let outcome =
            proof_state_init_with_docs(&mut rendered, &mut session, &mut state, &mut control)
                .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.initial_clauses, 1);
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("1 : :"));
        assert!(rendered.contains("4019 : 'eval'"));
        let copied = state.unprocessed().find_by_id(1).unwrap();
        assert!(copied.query_prop(CP_INITIAL));
        assert!(!copied.query_prop(CP_INPUT_FORMULA));
        assert_eq!(
            copied.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_CNF_QUOTE),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(axiom_id, 0)),
            ]
        );
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
    fn proof_state_init_with_output_reports_dynamic_watchlist_reduction() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, watch) =
            watchlist_subsumption_pair(state.terms_mut(), "pc_init_watch_output", 4_032, 4_033);
        state.axioms_mut().insert(axiom);
        state.watchlist_mut().unwrap().insert(watch);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell {
            heuristic_name: "InitWatchOutput".to_owned(),
            ac_handling: AcHandling::None,
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec!["InitWatchOutput=(1*FIFOWeight(ConstPrio))".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let mut output = Vec::new();

        let outcome = proof_state_init_with_output(&mut output, 1, &mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.watchlist_removed, 1);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Initializing proof state\n% Watchlist reduced by 1 clause\n"
        );
        assert_eq!(state.watchlist().unwrap().members(), 0);
    }

    #[test]
    fn proof_state_init_banner_obeys_output_level() {
        let mut output = Vec::new();

        super::proof_state_write_init_banner(&mut output, 0).unwrap();
        assert!(output.is_empty());

        super::proof_state_write_init_banner(&mut output, 1).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Initializing proof state\n"
        );
    }

    #[test]
    fn proof_state_check_watchlist_with_docs_quotes_dynamic_extraction() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut subsumer, mut watched) =
            watchlist_subsumption_pair(state.terms_mut(), "pc_doc_watch", 4_032, 4_033);
        subsumer.set_prop(CP_INPUT_FORMULA);
        watched.set_prop(CP_INPUT_FORMULA | CP_WATCH_ONLY);
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| panic!("{err}"));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let outcome = proof_state_check_watchlist_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut subsumer,
            false,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            ProofStateWatchlistOutcome {
                subsumes_watch: true,
                removed: 1,
            }
        );
        assert_eq!(session.id_source.current_ident(), 2);
        assert!(rendered.contains("4033 : 'extract_wl_subsumed(4032)'"));
        assert!(rendered.contains("4032 : 'extract_subsumed_watched'"));
        assert_eq!(state.watchlist().unwrap().members(), 0);
        let archived = state.archive().find_by_id(1).unwrap();
        assert!(archived.query_prop(CP_IS_DEAD | CP_WATCH_ONLY));
        assert!(!archived.query_prop(CP_INPUT_FORMULA));
        assert_eq!(subsumer.ident(), 2);
        assert!(subsumer.query_prop(CP_SUBSUMES_WATCH));
        assert!(!subsumer.query_prop(CP_INPUT_FORMULA));
    }

    #[test]
    fn proof_state_check_watchlist_with_docs_reports_output_level_one_reduction() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut subsumer, watched) =
            watchlist_subsumption_pair(state.terms_mut(), "pc_doc_watch_level_one", 4_034, 4_035);
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| panic!("{err}"));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let outcome = proof_state_check_watchlist_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut subsumer,
            false,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            ProofStateWatchlistOutcome {
                subsumes_watch: true,
                removed: 1,
            }
        );
        assert_eq!(session.id_source.current_ident(), 0);
        assert_eq!(rendered, "% Watchlist reduced by 1 clause\n");
        assert_eq!(state.watchlist().unwrap().members(), 0);
        let archived = state.archive().find_by_id(4_035).unwrap();
        assert!(archived.query_prop(CP_IS_DEAD));
        assert_eq!(subsumer.ident(), 4_034);
        assert!(subsumer.query_prop(CP_SUBSUMES_WATCH));
    }

    #[test]
    fn proof_state_check_watchlist_with_output_reports_output_level_one_reduction() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut subsumer, watched) = watchlist_subsumption_pair(
            state.terms_mut(),
            "pc_watch_output_level_one",
            4_036,
            4_037,
        );
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| panic!("{err}"));
        let mut output = Vec::new();

        let outcome = proof_state_check_watchlist_with_output(
            &mut output,
            1,
            &mut state,
            &mut subsumer,
            false,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            ProofStateWatchlistOutcome {
                subsumes_watch: true,
                removed: 1,
            }
        );
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Watchlist reduced by 1 clause\n"
        );
        assert_eq!(state.watchlist().unwrap().members(), 0);
        assert_eq!(subsumer.ident(), 4_036);
        assert!(subsumer.query_prop(CP_SUBSUMES_WATCH));
    }

    #[test]
    fn proof_state_reset_processed_archives_originals_and_requeues_evaluated_copies() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut rule, equation, negative, non_unit) = {
            let terms = state.terms_mut();
            (
                processed_unit_clause(terms, "pc_reset_rule", 4_040),
                processed_unit_clause(terms, "pc_reset_equation", 4_041),
                processed_unit_clause(terms, "pc_reset_negative", 4_042),
                processed_unit_clause(terms, "pc_reset_non_unit", 4_043),
            )
        };
        rule.set_prop(CP_INPUT_FORMULA);
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
        control.set_record_gc_selection(true);
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
            assert_eq!(
                archived
                    .derivation()
                    .map(crate::basics::pstacks::PStack::as_slice),
                Some(&[DerivationEntry::Operation(DC_CNF_EVAL_GC)][..])
            );

            let requeued = state.unprocessed().find_by_id(ident).unwrap();
            assert!(requeued.query_prop(CP_IS_PROCESSED));
            assert!(!requeued.query_prop(CP_IS_ORIENTED));
            assert!(!requeued.query_prop(CP_INPUT_FORMULA));
            assert_eq!(
                requeued
                    .derivation()
                    .map(crate::basics::pstacks::PStack::as_slice),
                Some(
                    &[
                        DerivationEntry::Operation(DC_CNF_QUOTE),
                        DerivationEntry::ClauseParent(ClauseDerivationRef::new(ident, 0)),
                    ][..]
                )
            );
            let evaluations = requeued.evaluations().expect("requeued copy is evaluated");
            assert_eq!(evaluations.eval_no(), 1);
            assert_eq!(
                evaluations.eval(0).priority(),
                PRIO_NORMAL - PRIO_LARGEST_REASONABLE
            );
        }
    }

    #[test]
    fn proof_state_reset_processed_with_docs_emits_move_eval_quote() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = processed_unit_clause(state.terms_mut(), "pc_reset_doc_quote", 4_044);
        clause.set_prop(CP_INPUT_FORMULA);
        state.processed_pos_rules_mut().insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ResetProcessedDocTest");
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let reset = proof_state_reset_processed_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(reset, 1);
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("1 : :"));
        assert!(rendered.contains("4044 : 'move_eval'"));
        let archived = state.archive().find_by_id(4_044).unwrap();
        assert!(archived.query_prop(CP_INPUT_FORMULA));
        let requeued = state.unprocessed().find_by_id(1).unwrap();
        assert!(!requeued.query_prop(CP_INPUT_FORMULA | CP_IS_ORIENTED));
        assert_eq!(
            requeued.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_CNF_QUOTE),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_044, 0)),
            ]
        );
    }

    #[test]
    fn proof_state_reset_processed_with_global_indices_deletes_entries_before_requeue() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (clause, indexed_term) =
            processed_indexed_unit_clause(state.terms_mut(), "pc_reset_global_idx", 4_044);
        state.processed_pos_rules_mut().insert(clause);
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        {
            let (terms, sets) = state.terms_and_processed_sets_mut();
            let clause = sets.pos_rules.find_by_id_mut(4_044).unwrap();
            indices.insert_clause(clause, terms, false);
        }
        assert!(indices.find_pm_from_occurrence(&indexed_term).is_some());
        assert!(state
            .processed_pos_rules()
            .find_by_id(4_044)
            .unwrap()
            .query_prop(CP_IS_GLOBAL_INDEXED));

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ResetProcessedGlobalIdxTest");
        let reset =
            proof_state_reset_processed_with_global_indices(&mut state, &mut control, &mut indices)
                .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(reset, 1);
        assert!(indices.find_pm_from_occurrence(&indexed_term).is_none());
        assert!(state.processed_pos_rules().is_empty());
        assert_eq!(state.archive().members(), 1);
        let archived = state.archive().find_by_id(4_044).unwrap();
        assert!(!archived.query_prop(CP_IS_GLOBAL_INDEXED));
        let requeued = state.unprocessed().find_by_id(4_044).unwrap();
        assert!(!requeued.query_prop(CP_IS_GLOBAL_INDEXED));
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
    fn proof_state_move_to_tmp_store_with_global_indices_deletes_entries_before_move() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (clause, indexed_term) =
            processed_indexed_unit_clause(state.terms_mut(), "pc_move_global_idx", 4_054);
        state.processed_pos_eqns_mut().insert(clause);
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        {
            let (terms, sets) = state.terms_and_processed_sets_mut();
            let clause = sets.pos_eqns.find_by_id_mut(4_054).unwrap();
            indices.insert_clause(clause, terms, false);
        }
        assert!(indices.find_pm_from_occurrence(&indexed_term).is_some());

        let control = proof_control_alloc();
        let moved =
            proof_state_move_to_tmp_store_with_global_indices(&mut state, &control, &mut indices);

        assert_eq!(moved, 1);
        assert!(indices.find_pm_from_occurrence(&indexed_term).is_none());
        assert!(state.processed_pos_eqns().is_empty());
        let moved = state.tmp_store().find_by_id(4_054).unwrap();
        assert!(moved.query_prop(CP_IS_PROCESSED));
        assert!(!moved.query_prop(CP_IS_ORIENTED | CP_IS_GLOBAL_INDEXED));
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
    fn proof_state_forward_modify_clause_honors_local_rewrite_option() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut clause, rewritten_left, rewritten_right) = {
            let terms = state.terms_mut();
            let a = typed_const(terms, "pc_local_rw_a");
            let c = typed_const(terms, "pc_local_rw_c");
            let f_a = typed_unary(terms, "pc_local_rw_f", &a);
            let g_f_a = typed_unary(terms, "pc_local_rw_g", &f_a);
            let g_a = typed_unary(terms, "pc_local_rw_g", &a);
            let rule = literal(terms, &f_a, &a, false);
            let target = literal(terms, &g_f_a, &c, true);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![rule, target]));
            clause.set_ident(4_082);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
            (clause, g_a, c)
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().local_rw = true;
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let trivial = proof_state_forward_modify_clause_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            &mut clause,
            false,
            false,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.literals().as_slice().iter().any(|literal| {
            literal.is_positive()
                && literal.left() == &rewritten_left
                && literal.right() == &rewritten_right
        }));
        assert!(derivation_contains_operation(&clause, DC_LOCAL_REWRITE));
        assert_eq!(clause.ident(), 4_082);
        assert_eq!(session.id_source.current_ident(), 0);
        assert!(rendered.is_empty());
    }

    #[test]
    fn proof_state_forward_modify_clause_higher_order_normalizes_encoded_equality() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut clause, left, right) = {
            let terms = state.terms_mut();
            let left = typed_const(terms, "pc_ho_norm_a");
            let right = typed_const(terms, "pc_ho_norm_b");
            let truth = terms.true_term().clone();
            let eqn_code = terms.signature().eqn_code();
            let encoded = bool_binary_with_code(terms, eqn_code, &left, &right);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &encoded, &truth, true,
            )]));
            clause.set_ident(4_082);
            (clause, left, right)
        };
        let mut control = proof_control_alloc();
        control.set_ocb(empty_ocb(state.terms()));

        let trivial = proof_state_forward_modify_clause_impl::<String>(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        let literal = &clause.literals().as_slice()[0];
        assert!(literal.is_equ_lit(state.terms()));
        assert!(
            (literal.left() == &left && literal.right() == &right)
                || (literal.left() == &right && literal.right() == &left)
        );
        assert!(clause
            .derivation()
            .unwrap()
            .as_slice()
            .iter()
            .any(|entry| matches!(entry, DerivationEntry::Operation(DC_NORMALIZE))));
    }

    #[test]
    fn proof_state_forward_modify_clause_higher_order_first_order_subset_uses_ordering() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let left = typed_const(terms, "pc_ho_order_a");
            let right = typed_const(terms, "pc_ho_order_b");
            Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &right, true)]))
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo6_ocb(state.terms()));

        let trivial = proof_state_forward_modify_clause_impl::<String>(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.literals().as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn proof_state_forward_modify_clause_higher_order_lpo_first_order_subset_uses_ordering() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let arg = typed_const(terms, "pc_ho_order_lpo_a");
            let left = typed_unary(terms, "pc_ho_order_lpo_f", &arg);
            Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &arg, true)]))
        };
        let mut control = proof_control_alloc();
        control.set_ocb(OrderControlBlock::alloc(
            TermOrdering::Lpo,
            true,
            state.terms().signature(),
            HoOrderKind::LfhoOrder,
        ));

        let trivial = proof_state_forward_modify_clause_impl::<String>(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.literals().as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn proof_state_forward_modify_clause_ho_lambda_order_fo_subset_uses_ordering() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let left = typed_const(terms, "pc_ho_order_lambda_fo_a");
            let right = typed_const(terms, "pc_ho_order_lambda_fo_b");
            Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &right, true)]))
        };
        let mut control = proof_control_alloc();
        control.set_ocb(OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            state.terms().signature(),
            HoOrderKind::LambdaOrder,
        ));

        let trivial = proof_state_forward_modify_clause_impl::<String>(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.literals().as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn proof_state_forward_modify_clause_higher_order_lpo_surface_matches_release() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let predicate = unary_predicate_var(terms, -4_092);
            let arg = typed_const(terms, "pc_ho_order_lpo_app_arg");
            let applied = apply_terms(terms, &predicate, std::slice::from_ref(&arg))
                .unwrap_or_else(|err| panic!("{err}"));
            let truth = terms.true_term().clone();
            Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &applied, &truth, true,
            )]))
        };
        let mut control = proof_control_alloc();
        control.set_ocb(OrderControlBlock::alloc(
            TermOrdering::Lpo,
            true,
            state.terms().signature(),
            HoOrderKind::LfhoOrder,
        ));

        let trivial = proof_state_forward_modify_clause_impl::<String>(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.literals().as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn proof_state_forward_modify_clause_higher_order_lpo4_ignores_kbo_ho_order_kind() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let predicate = unary_predicate_var(terms, -4_093);
            let arg = typed_const(terms, "pc_ho_order_lpo4_app_arg");
            let applied = apply_terms(terms, &predicate, std::slice::from_ref(&arg))
                .unwrap_or_else(|err| panic!("{err}"));
            let rhs_predicate = unary_predicate_const(terms, "pc_ho_order_lpo4_rhs_pred");
            let right = apply_terms(terms, &rhs_predicate, std::slice::from_ref(&arg))
                .unwrap_or_else(|err| panic!("{err}"));
            Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &applied, &right, true,
            )]))
        };
        let mut control = proof_control_alloc();
        control.set_ocb(OrderControlBlock::alloc(
            TermOrdering::Lpo4,
            true,
            state.terms().signature(),
            HoOrderKind::LambdaOrder,
        ));

        let trivial = proof_state_forward_modify_clause_impl::<String>(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.literals().as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn proof_state_forward_modify_clause_higher_order_lfho_applied_var_ordering_runs() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let predicate = unary_predicate_var(terms, -4_091);
            let arg = typed_const(terms, "pc_ho_order_app_arg");
            let applied = apply_terms(terms, &predicate, std::slice::from_ref(&arg))
                .unwrap_or_else(|err| panic!("{err}"));
            let truth = terms.true_term().clone();
            Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &applied, &truth, true,
            )]))
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo6_ocb(state.terms()));

        let trivial = proof_state_forward_modify_clause_impl::<String>(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.literals().as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn proof_state_forward_modify_clause_higher_order_lambda_order_surface_runs() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let binder_type = terms.signature().type_bank().bool_type();
            let db0 = terms.request_db_var(&binder_type, 0);
            let lambda =
                close_with_db_var(terms, &binder_type, &db0).unwrap_or_else(|err| panic!("{err}"));
            let predicate = unary_predicate_const(terms, "pc_ho_order_lambda_pred");
            let arg = typed_const(terms, "pc_ho_order_lambda_app_arg");
            let atom = apply_terms(terms, &predicate, std::slice::from_ref(&arg))
                .unwrap_or_else(|err| panic!("{err}"));
            let applied = apply_terms(terms, &lambda, std::slice::from_ref(&atom))
                .unwrap_or_else(|err| panic!("{err}"));
            let truth = terms.true_term().clone();
            Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &applied, &truth, true,
            )]))
        };
        let mut control = proof_control_alloc();
        control.set_ocb(OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            state.terms().signature(),
            HoOrderKind::LambdaOrder,
        ));

        let trivial = proof_state_forward_modify_clause_impl::<String>(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.literals().as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn proof_state_forward_modify_clause_higher_order_prunes_constant_argument() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut clause, function, x, y) = {
            let terms = state.terms_mut();
            let type_ = terms.signature().type_bank().default_type();
            let arrow = terms
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![type_.clone(), type_.clone(), type_]));
            let function = terms.vars().var_assert_alloc(-4_094, &arrow);
            let constant = typed_const(terms, "pc_ho_prune_constant");
            let x = typed_var(terms, -4_096);
            let y = typed_var(terms, -4_098);
            let first = apply_terms(terms, &function, &[constant.clone(), x.clone()])
                .unwrap_or_else(|err| panic!("{err}"));
            let second = apply_terms(terms, &function, &[constant, y.clone()])
                .unwrap_or_else(|err| panic!("{err}"));
            let first_rhs = typed_const(terms, "pc_ho_prune_first_rhs");
            let second_rhs = typed_const(terms, "pc_ho_prune_second_rhs");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &first, &first_rhs, true),
                literal(terms, &second, &second_rhs, true),
            ]));
            clause.set_ident(4_083);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
            (clause, function, x, y)
        };
        let mut control = proof_control_alloc();
        control.set_ocb(empty_ocb(state.terms()));
        control.heuristic_parms_mut().prune_args = true;
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::HigherOrder);
        let mut rendered = String::new();

        let trivial = proof_state_forward_modify_clause_impl(
            &mut state,
            &mut control,
            &mut clause,
            false,
            RewriteLevel::RuleRewrite,
            ProblemType::HigherOrder,
            Some((&mut rendered, &mut session)),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        let first_left = clause.literals().as_slice()[0].left();
        let second_left = clause.literals().as_slice()[1].left();
        assert!(first_left.is_applied_free_var());
        assert!(second_left.is_applied_free_var());
        assert_eq!(first_left.arity(), 2);
        assert_eq!(second_left.arity(), 2);
        assert_ne!(first_left.argument(0).as_ref(), Some(&function));
        assert_eq!(first_left.argument(1).as_ref(), Some(&x));
        assert_eq!(second_left.argument(1).as_ref(), Some(&y));
        assert!(derivation_contains_operation(&clause, DC_PRUNE_ARG));
        assert_eq!(clause.ident(), 4_083);
        assert_eq!(session.id_source.current_ident(), 0);
        assert!(rendered.is_empty());
    }

    #[test]
    fn proof_state_forward_modify_clause_with_docs_emits_rewrite_steps_at_level_four() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, mut clause) = {
            let terms = state.terms_mut();
            let f_code = typed_binary_code(terms, "pc_forward_doc_f");
            let x = typed_var(terms, -2);
            let y = typed_var(terms, -4);
            let replacement = typed_const(terms, "pc_forward_doc_a");
            let first = typed_const(terms, "pc_forward_doc_b");
            let second = typed_const(terms, "pc_forward_doc_c");
            let rhs = typed_const(terms, "pc_forward_doc_d");
            let pattern = typed_binary_with_code(terms, f_code, &x, &y);
            let target = typed_binary_with_code(terms, f_code, &first, &second);
            let mut demod_lit = literal(terms, &pattern, &replacement, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_084);
            demodulator.set_date(SysDate::from_raw(5));
            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &target, &rhs, true)]));
            clause.set_ident(4_085);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
            (demodulator, clause)
        };
        state
            .processed_pos_rules_mut()
            .set_date(SysDate::from_raw(5));
        state.processed_pos_rules_mut().insert(demodulator);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 4, ProblemType::FirstOrder);
        session.pcl_shell_level = 1;
        let mut rendered = String::new();

        let trivial = proof_state_forward_modify_clause_with_docs(
            &mut rendered,
            &mut session,
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
        assert_eq!(clause.ident(), 1);
        assert!(!clause.query_prop(CP_INITIAL | CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 1);
        assert_eq!(rendered, "     1 : : : rw(4085,4084)\n");
    }

    #[test]
    fn proof_state_forward_modify_clause_with_docs_records_ac_resolution() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut first_parent = Clause::empty();
        first_parent.set_ident(70);
        first_parent.refresh_derivation_generation();
        let first_parent_ref = ClauseDerivationRef::from(&first_parent);
        let mut second_parent = Clause::empty();
        second_parent.set_ident(71);
        second_parent.refresh_derivation_generation();
        let second_parent_ref = ClauseDerivationRef::from(&second_parent);
        state
            .terms_mut()
            .signature_mut()
            .push_ac_axiom(first_parent_ref);
        state
            .terms_mut()
            .signature_mut()
            .push_ac_axiom(second_parent_ref);
        first_parent.set_ident(7);
        second_parent.set_ident(8);
        state.axioms_mut().insert(first_parent);
        state.archive_mut().insert(second_parent);
        let mut clause = {
            let terms = state.terms_mut();
            let f_code = typed_binary_code(terms, "pc_forward_ac_f");
            terms
                .signature_mut()
                .set_func_prop(f_code, FP_ASSOCIATIVE | FP_COMMUTATIVE);
            let first = typed_const(terms, "pc_forward_ac_a");
            let second = typed_const(terms, "pc_forward_ac_b");
            let left = typed_binary_with_code(terms, f_code, &first, &second);
            let right = typed_binary_with_code(terms, f_code, &second, &first);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &left, &right, false,
            )]));
            clause.set_ident(4_088);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
            clause
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.set_ac_handling_active(true);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Tstp, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let trivial = proof_state_forward_modify_clause_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            &mut clause,
            false,
            false,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert!(clause.is_empty());
        assert_eq!(clause.ident(), 1);
        assert!(!clause.query_prop(CP_INITIAL | CP_INPUT_FORMULA));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_AC_RES),
                DerivationEntry::NumericArg(2),
            ]
        );
        assert_eq!(
            rendered,
            "cnf(c_0_1, plain, ($false),inference(ar,[status(thm)],[c_0_4088,c_0_7,c_0_8])).\n"
        );
    }

    #[test]
    fn proof_state_forward_modify_clause_with_docs_preserves_output_level_four_gate() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, mut clause) = {
            let terms = state.terms_mut();
            let f_code = typed_binary_code(terms, "pc_forward_doc_gate_f");
            let x = typed_var(terms, -2);
            let y = typed_var(terms, -4);
            let replacement = typed_const(terms, "pc_forward_doc_gate_a");
            let first = typed_const(terms, "pc_forward_doc_gate_b");
            let second = typed_const(terms, "pc_forward_doc_gate_c");
            let rhs = typed_const(terms, "pc_forward_doc_gate_d");
            let pattern = typed_binary_with_code(terms, f_code, &x, &y);
            let target = typed_binary_with_code(terms, f_code, &first, &second);
            let mut demod_lit = literal(terms, &pattern, &replacement, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_086);
            demodulator.set_date(SysDate::from_raw(5));
            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &target, &rhs, true)]));
            clause.set_ident(4_087);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
            (demodulator, clause)
        };
        state
            .processed_pos_rules_mut()
            .set_date(SysDate::from_raw(5));
        state.processed_pos_rules_mut().insert(demodulator);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 3, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let trivial = proof_state_forward_modify_clause_with_docs(
            &mut rendered,
            &mut session,
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
        assert_eq!(clause.ident(), 4_087);
        assert!(rendered.is_empty());
        assert!(!clause.query_prop(CP_INITIAL));
        assert!(clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 0);
    }

    #[test]
    fn proof_state_forward_modify_clause_with_docs_emits_minimize_step() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let first = typed_const(terms, "pc_forward_doc_min_a");
            let second = typed_const(terms, "pc_forward_doc_min_b");
            let positive = literal(terms, &first, &second, true);
            let duplicate = literal(terms, &second, &first, true);
            let false_literal = literal(terms, &first, &first, false);
            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![positive, duplicate, false_literal]));
            clause.set_ident(4_088);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW);
            clause
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let trivial = proof_state_forward_modify_clause_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            &mut clause,
            false,
            false,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert_eq!(clause.ident(), 1);
        assert_eq!(clause.literal_number(), 1);
        assert!(!clause.is_any_prop_set(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW));
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.starts_with("     1 :"));
        assert!(rendered.contains("cn(4088)"));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_NORMALIZE)]
        );
    }

    #[test]
    fn proof_state_forward_modify_clause_with_docs_emits_condense_step() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -2);
            let constant = typed_const(terms, "pc_forward_doc_condense_a");
            let instance = typed_const(terms, "pc_forward_doc_condense_b");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &variable, &constant, true),
                literal(terms, &instance, &constant, true),
            ]));
            clause.set_ident(4_089);
            clause.set_prop(CP_INPUT_FORMULA);
            clause
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let trivial = proof_state_forward_modify_clause_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            &mut clause,
            false,
            true,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert_eq!(clause.ident(), 1);
        assert_eq!(clause.literal_number(), 1);
        assert!(!clause.query_prop(CP_INPUT_FORMULA));
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("condense(4089)"));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[DerivationEntry::Operation(DC_CONDENSE)]
        );
    }

    #[test]
    fn proof_state_forward_modify_clause_with_docs_emits_simplify_reflect_step() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (simplifier, mut clause) = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -2);
            let constant = typed_const(terms, "pc_forward_doc_sr_a");
            let witness = typed_const(terms, "pc_forward_doc_sr_b");
            let kept = typed_const(terms, "pc_forward_doc_sr_c");
            let mut simplifier = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &variable, &constant, true,
            )]));
            simplifier.set_ident(4_090);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &witness, &constant, false),
                literal(terms, &kept, &constant, true),
            ]));
            clause.set_ident(4_091);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW);
            (simplifier, clause)
        };
        state.processed_pos_eqns_mut().insert(simplifier);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let trivial = proof_state_forward_modify_clause_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            &mut clause,
            false,
            false,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!trivial);
        assert_eq!(clause.ident(), 1);
        assert_eq!(clause.literal_number(), 1);
        assert!(!clause.is_any_prop_set(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW));
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("sr(4091,4090)"));
        assert_eq!(
            clause.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_090, 0)),
            ]
        );
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
    fn proof_state_forward_modify_clause_honors_strong_unit_forward_subsumption() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (positive_unit, positive_id, mut default_target, mut strong_target) = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -10);
            let constant = typed_const(terms, "pc_strong_sr_a");
            let left_other = typed_const(terms, "pc_strong_sr_b");
            let right_other = typed_const(terms, "pc_strong_sr_c");
            let right_match = typed_const(terms, "pc_strong_sr_d");
            let kept_left = typed_const(terms, "pc_strong_sr_e");
            let kept_right = typed_const(terms, "pc_strong_sr_g");
            let f_code = typed_binary_code(terms, "pc_strong_sr_f");
            let left = typed_binary_with_code(terms, f_code, &left_other, &right_other);
            let right = typed_binary_with_code(terms, f_code, &constant, &right_match);
            let mut positive_unit = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &variable,
                &right_match,
                true,
            )]));
            positive_unit.set_ident(4_102);
            let positive_id = positive_unit.ident();
            let default_target = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left, &right, false),
                literal(terms, &kept_left, &kept_right, true),
            ]));
            let strong_target = default_target.clone();
            (positive_unit, positive_id, default_target, strong_target)
        };
        state.processed_pos_eqns_mut().insert(positive_unit);

        let mut default_control = proof_control_alloc();
        default_control.set_ocb(kbo_ocb(state.terms()));
        let default_trivial = proof_state_forward_modify_clause(
            &mut state,
            &mut default_control,
            &mut default_target,
            false,
            false,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!default_trivial);
        assert_eq!(default_target.negative_literal_count(), 1);
        assert!(default_target.derivation().is_none());

        let mut strong_control = proof_control_alloc();
        strong_control.set_ocb(kbo_ocb(state.terms()));
        strong_control.set_strong_unit_forward_subsumption(true);
        let strong_trivial = proof_state_forward_modify_clause(
            &mut state,
            &mut strong_control,
            &mut strong_target,
            false,
            false,
            RewriteLevel::RuleRewrite,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!strong_trivial);
        assert_eq!(strong_target.negative_literal_count(), 0);
        assert_eq!(strong_target.positive_literal_count(), 1);
        assert_eq!(
            strong_target.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(positive_id, 0)),
            ]
        );
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
    fn proof_state_banked_forward_subsumption_matches_higher_order_unit() {
        let _global_state = global_state_lock();
        set_problem_type(ProblemType::HigherOrder).unwrap();
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (subsumer, mut clause, flex) = {
            let terms = state.terms_mut();
            let individual = terms.signature().type_bank().default_type();
            let unary = terms
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![individual.clone(), individual]));
            let flex = terms.vars().get_fresh_var(&unary);
            let argument = typed_const(terms, "pc_ho_subsumption_argument");
            let rhs = typed_const(terms, "pc_ho_subsumption_rhs");
            let rigid = typed_arrow_const(terms, "pc_ho_subsumption_rigid", 1);
            let flex_application =
                apply_terms(terms, &flex, std::slice::from_ref(&argument)).unwrap();
            let rigid_application =
                apply_terms(terms, &rigid, std::slice::from_ref(&argument)).unwrap();
            let matcher = typed_unary(terms, "pc_ho_subsumption_outer", &flex_application);
            let target = typed_unary(terms, "pc_ho_subsumption_outer", &rigid_application);
            let subsumer = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &matcher, &rhs, true,
            )]));
            let clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &target, &rhs, true)]));
            (subsumer, clause, flex)
        };
        state.processed_pos_eqns_mut().insert(subsumer);

        let mut legacy_counts = ForwardContractCounts::default();
        assert!(proof_state_forward_subsumption(
            &state,
            &mut clause.clone(),
            &mut legacy_counts,
            false,
        )
        .is_some());

        let mut counts = ForwardContractCounts::default();
        let packed = proof_state_forward_subsumption_with_bank(
            &mut state,
            &mut clause,
            &mut counts,
            false,
            false,
        )
        .unwrap();

        assert!(packed.is_none());
        assert_eq!(counts.subsumed, 1);
        assert!(flex.binding().is_none());
    }

    #[test]
    fn proof_state_forward_subsumption_honors_strong_unit_forward_subsumption() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (subsumer, mut default_clause, mut strong_clause) = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -10);
            let constant = typed_const(terms, "pc_strong_subsumption_a");
            let left_other = typed_const(terms, "pc_strong_subsumption_b");
            let right_other = typed_const(terms, "pc_strong_subsumption_c");
            let right_match = typed_const(terms, "pc_strong_subsumption_d");
            let f_code = typed_binary_code(terms, "pc_strong_subsumption_f");
            let left = typed_binary_with_code(terms, f_code, &left_other, &right_other);
            let right = typed_binary_with_code(terms, f_code, &constant, &right_match);
            let mut subsumer = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &variable,
                &right_match,
                true,
            )]));
            subsumer.set_ident(4_103);
            let default_clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &right, true)]));
            let strong_clause = default_clause.clone();
            (subsumer, default_clause, strong_clause)
        };
        state.processed_pos_eqns_mut().insert(subsumer);

        let mut default_counts = ForwardContractCounts::default();
        let packed = proof_state_forward_subsumption(
            &state,
            &mut default_clause,
            &mut default_counts,
            false,
        );

        assert!(packed.is_some());
        assert_eq!(default_counts.subsumed, 0);

        let mut strong_counts = ForwardContractCounts::default();
        let packed = proof_state_forward_subsumption_with_strong(
            &state,
            &mut strong_clause,
            &mut strong_counts,
            false,
            true,
        );

        assert!(packed.is_none());
        assert_eq!(strong_counts.subsumed, 1);
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
        let permanent_terms_before = state.terms().in_count();
        let temporary_terms_before = state.tmp_terms().in_count();

        let packed = proof_state_forward_contract_clause(&mut state, &mut control, clause, options)
            .unwrap_or_else(|err| panic!("{err}"))
            .expect("surviving clause should be packed");
        let survivor = packed.clause();

        assert_eq!(state.statistics().proc_forward_subsumed_count, 0);
        assert_eq!(state.statistics().proc_trivial_count, 0);
        assert_eq!(state.terms().in_count(), permanent_terms_before);
        assert!(state.tmp_terms().in_count() > temporary_terms_before);
        assert!(survivor.query_prop(CP_IS_ORIENTED));
        assert_eq!(survivor.prop_lit_number(EP_IS_SELECTED), 1);
        assert!(survivor.literals().as_slice().iter().any(Eqn::is_maximal));
    }

    #[test]
    fn proof_state_forward_contract_clause_counts_boolean_simplified_tautology() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let variable = terms
                .vars()
                .var_assert_alloc(-2, &terms.signature().type_bank().bool_type());
            let truth = terms.true_term().clone();
            let or_code = terms.signature().or_code();
            let disjunction = bool_binary_with_code(terms, or_code, &variable, &truth);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &disjunction,
                &truth,
                true,
            )]));
            clause.set_ident(40_861);
            clause
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let options = ForwardContractOptions {
            non_unit_subsumption: true,
            context_sr: false,
            condense_clause: false,
            level: RewriteLevel::RuleRewrite,
        };

        let packed = proof_state_forward_contract_clause(&mut state, &mut control, clause, options)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(packed.is_none());
        assert_eq!(state.statistics().proc_trivial_count, 1);
        assert_eq!(state.statistics().proc_forward_subsumed_count, 0);
    }

    #[test]
    fn proof_state_forward_contract_clause_with_docs_emits_context_sr_step() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut subsumer, clause) = {
            let terms = state.terms_mut();
            let p_code = unary_predicate_code(terms, "pc_forward_contract_doc_p");
            let q_code = unary_predicate_code(terms, "pc_forward_contract_doc_q");
            let arg = typed_const(terms, "pc_forward_contract_doc_a");
            let p_atom = unary_predicate(terms, p_code, &arg);
            let q_atom = unary_predicate(terms, q_code, &arg);
            let truth = terms.true_term().clone();
            let mut subsumer = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &p_atom, &truth, true),
                literal(terms, &q_atom, &truth, false),
            ]));
            subsumer.set_ident(4_087);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &p_atom, &truth, true),
                literal(terms, &q_atom, &truth, true),
            ]));
            clause.set_ident(4_088);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
            (subsumer, clause)
        };
        clause_subsume_order_sort_lits(&mut subsumer, state.terms());
        subsumer.set_weight(subsumer.standard_weight());
        state.processed_non_units_mut().insert(subsumer);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let options = ForwardContractOptions {
            non_unit_subsumption: false,
            context_sr: true,
            condense_clause: false,
            level: RewriteLevel::RuleRewrite,
        };
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let packed = proof_state_forward_contract_clause_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            clause,
            options,
        )
        .unwrap_or_else(|err| panic!("{err}"))
        .expect("contextually simplified clause should survive");
        let survivor = packed.clause();

        assert_eq!(state.statistics().context_sr_count, 1);
        assert_eq!(state.statistics().proc_forward_subsumed_count, 0);
        assert_eq!(state.statistics().proc_trivial_count, 0);
        assert_eq!(session.id_source.current_ident(), 1);
        assert_eq!(survivor.ident(), 1);
        assert_eq!(survivor.literal_number(), 1);
        assert!(!survivor.is_any_prop_set(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW));
        assert!(rendered.contains("csr(4088,4087)"));
        assert!(survivor.query_prop(CP_IS_ORIENTED));
        assert_eq!(
            survivor.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_CONTEXT_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_087, 0)),
            ]
        );
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
    fn proof_state_forward_contract_set_with_docs_emits_modification_step() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let first = typed_const(terms, "pc_forward_set_doc_a");
            let second = typed_const(terms, "pc_forward_set_doc_b");
            let positive = literal(terms, &first, &second, true);
            let duplicate = literal(terms, &second, &first, true);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![positive, duplicate]));
            clause.set_ident(4_092);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW);
            clause
        };
        let mut set = ClauseSet::new();
        set.insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();
        let mut eliminated = 0;

        let empty = proof_state_forward_contract_set_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            &mut set,
            false,
            RewriteLevel::RuleRewrite,
            &mut eliminated,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert_eq!(eliminated, 0);
        assert_eq!(set.members(), 1);
        let survivor = set.iter().next().expect("contracted clause should survive");
        assert_eq!(survivor.ident(), 1);
        assert_eq!(survivor.literal_number(), 1);
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("cn(4092)"));
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
    fn proof_state_forward_contract_set_reweight_with_docs_keeps_modified_survivor_evaluated() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let first = typed_const(terms, "pc_forward_set_reweight_doc_a");
            let second = typed_const(terms, "pc_forward_set_reweight_doc_b");
            let positive = literal(terms, &first, &second, true);
            let duplicate = literal(terms, &second, &first, true);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![positive, duplicate]));
            clause.set_ident(4_097);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW);
            clause
        };
        let mut set = ClauseSet::new();
        set.insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ForwardSetReweightDocsTest");
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();
        let mut eliminated = 0;

        let empty = proof_state_forward_contract_set_reweight_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            &mut set,
            false,
            RewriteLevel::RuleRewrite,
            &mut eliminated,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert_eq!(eliminated, 0);
        assert_eq!(set.members(), 1);
        let survivor = set.iter().next().expect("contracted clause should survive");
        assert_eq!(survivor.ident(), 1);
        assert_eq!(survivor.literal_number(), 1);
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("cn(4097)"), "{rendered}");
        let evaluations = survivor
            .evaluations()
            .expect("documented contraction survivor should be reweighted");
        assert_eq!(evaluations.eval_no(), 1);
        assert_eq!(evaluations.eval(0).priority(), PRIO_NORMAL);
        assert_eq!(set.eval_order_cloned(0).len(), 1);
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
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "FilterReweightTest");
        let mut eliminated = 0;

        proof_control_clause_set_filter_reweigth_with_bank(
            &mut control,
            state.terms_mut(),
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
    fn proof_control_clause_set_reweight_with_bank_handles_lambda_order_refined_weight() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let binder_type = terms.signature().type_bank().bool_type();
            let db0 = terms.request_db_var(&binder_type, 0);
            let lambda =
                close_with_db_var(terms, &binder_type, &db0).unwrap_or_else(|err| panic!("{err}"));
            let predicate = unary_predicate_const(terms, "pc_reweight_lambda_pred");
            let arg = typed_const(terms, "pc_reweight_lambda_arg");
            let atom = apply_terms(terms, &predicate, std::slice::from_ref(&arg))
                .unwrap_or_else(|err| panic!("{err}"));
            let applied = apply_terms(terms, &lambda, std::slice::from_ref(&atom))
                .unwrap_or_else(|err| panic!("{err}"));
            let truth = terms.true_term().clone();
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &applied, &truth, true,
            )]));
            clause.set_ident(4_099);
            clause
        };
        let mut set = ClauseSet::new();
        set.insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(OrderControlBlock::alloc(
            TermOrdering::Kbo6,
            true,
            state.terms().signature(),
            HoOrderKind::LambdaOrder,
        ));
        let mut params = HeuristicParmsCell {
            heuristic_name: "LambdaRefinedReweightTest".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let mut hcb_defs = vec![
            "LambdaRefinedReweightTest=(1*Refinedweight(ConstPrio,2,1,1.0,1.0,1.0))".to_owned(),
        ];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        proof_control_clause_set_reweight_with_bank(&mut control, state.terms_mut(), &mut set)
            .unwrap_or_else(|err| panic!("{err}"));

        let evaluated = set.find_by_id(4_099).unwrap();
        let evaluations = evaluated
            .evaluations()
            .expect("banked proof-control reweight attaches evaluations");
        assert_eq!(evaluations.eval_no(), 1);
        assert!(evaluated.literals().as_slice()[0].query_prop(EP_MAX_IS_UP_TO_DATE));
    }

    #[test]
    fn proof_state_cleanup_unprocessed_deletes_orphans_after_back_simplification_limit() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut parent = Clause::empty();
        parent.set_ident(4_110);
        let mut orphan = Clause::empty();
        orphan.set_ident(4_111);
        clause_push_derivation(&mut orphan, DC_ORDERED_FACTOR, Some(&parent), None);
        let survivor = unit_clause_with_id(state.terms_mut(), "pc_cleanup_orphan_survivor", 4_112);
        state.unprocessed_mut().insert(orphan);
        state.unprocessed_mut().insert(survivor);
        state.statistics_mut().backward_subsumed_count = 2;

        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().filter_orphans_limit = 0;

        let outcome =
            proof_state_cleanup_unprocessed_clauses_with(&mut state, &mut control, 0, |parent| {
                matches!(
                    parent,
                    DerivationParentRef::Clause(parent)
                        if parent == ClauseDerivationRef::new(4_110, 0)
                )
            })
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(outcome.unsatisfiable.is_none());
        assert!(outcome.orphan_cleanup_triggered);
        assert_eq!(outcome.orphan_cleanup_deleted, 1);
        assert_eq!(outcome.orphan_cleanup_remaining, 1);
        assert_eq!(outcome.orphaned_deleted, 1);
        assert_eq!(outcome.forward_contract_deleted, 0);
        assert_eq!(state.unprocessed().members(), 1);
        assert!(state.unprocessed().find_by_id(4_111).is_none());
        assert_eq!(state.statistics().other_redundant_count, 1);
        assert_eq!(state.statistics().filter_orphans_base, 2);
    }

    #[test]
    fn compact_parent_liveness_distinguishes_same_id_generations() {
        let parent = ClauseDerivationRef::new_with_generation(4_117, 0, 41);
        let stale_alias = ClauseDerivationRef::new_with_generation(4_117, 0, 42);
        let mut snapshot = ParentLivenessSnapshot::default();
        snapshot.live.insert(parent);

        assert!(!snapshot.parent_is_dead(DerivationParentRef::Clause(parent)));
        assert!(snapshot.parent_is_dead(DerivationParentRef::Clause(stale_alias)));
    }

    #[test]
    fn selection_parent_liveness_scans_only_stable_parent_owners() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut live_parent = Clause::empty();
        live_parent.set_ident(4_120);
        let live_ref = ClauseDerivationRef::from(&live_parent);
        state.processed_non_units_mut().insert(live_parent);
        assert_eq!(
            state.processed_non_units().find_indexed_by_id(4_120),
            state.processed_non_units().find_by_id(4_120)
        );

        let mut dead_parent = Clause::empty();
        dead_parent.set_ident(4_121);
        dead_parent.refresh_derivation_generation();
        dead_parent.set_prop(CP_IS_DEAD);
        let dead_ref = ClauseDerivationRef::from(&dead_parent);
        state.archive_mut().insert(dead_parent);

        let mut waiting_child = Clause::empty();
        waiting_child.set_ident(4_122);
        let waiting_ref = ClauseDerivationRef::from(&waiting_child);
        state.unprocessed_mut().insert(waiting_child);

        assert!(!selection_parent_is_dead(
            &state,
            DerivationParentRef::Clause(live_ref)
        ));
        assert!(selection_parent_is_dead(
            &state,
            DerivationParentRef::Clause(dead_ref)
        ));
        let mut live_alias = Clause::empty();
        live_alias.set_ident(4_121);
        live_alias.refresh_derivation_generation();
        let live_alias_ref = ClauseDerivationRef::from(&live_alias);
        assert_ne!(dead_ref, live_alias_ref);
        state.processed_pos_eqns_mut().insert(live_alias);
        assert!(selection_parent_is_dead(
            &state,
            DerivationParentRef::Clause(dead_ref)
        ));
        assert!(!selection_parent_is_dead(
            &state,
            DerivationParentRef::Clause(live_alias_ref)
        ));
        assert!(selection_parent_is_dead(
            &state,
            DerivationParentRef::Clause(waiting_ref)
        ));
    }

    #[test]
    fn proof_state_cleanup_unprocessed_default_deletes_archived_dead_parent_orphans() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut parent = Clause::empty();
        parent.set_ident(4_117);
        let mut orphan = Clause::empty();
        orphan.set_ident(4_118);
        clause_push_derivation(&mut orphan, DC_ORDERED_FACTOR, Some(&parent), None);
        parent.set_prop(CP_IS_DEAD);
        state.archive_mut().insert(parent);
        let survivor = unit_clause_with_id(state.terms_mut(), "pc_cleanup_default_survivor", 4_119);
        state.unprocessed_mut().insert(orphan);
        state.unprocessed_mut().insert(survivor);
        state.statistics_mut().backward_rewritten_count = 1;

        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().filter_orphans_limit = 0;

        let outcome = proof_state_cleanup_unprocessed_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.orphaned_deleted, 1);
        assert!(outcome.orphan_cleanup_triggered);
        assert_eq!(outcome.orphan_cleanup_deleted, 1);
        assert_eq!(outcome.orphan_cleanup_remaining, 1);
        assert_eq!(state.unprocessed().members(), 1);
        assert!(state.unprocessed().find_by_id(4_118).is_none());
        assert!(state.unprocessed().find_by_id(4_119).is_some());
        assert_eq!(state.statistics().other_redundant_count, 1);
    }

    #[test]
    fn cleanup_default_measures_storage_after_orphan_deletion() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut parent = Clause::empty();
        parent.set_ident(4_123);
        let mut orphan = Clause::empty();
        orphan.set_ident(4_124);
        clause_push_derivation(&mut orphan, DC_ORDERED_FACTOR, Some(&parent), None);
        parent.set_prop(CP_IS_DEAD);
        state.archive_mut().insert(parent);
        let survivor = unit_clause_with_id(state.terms_mut(), "pc_cleanup_order_survivor", 4_125);
        state.unprocessed_mut().insert(orphan);
        state.unprocessed_mut().insert(survivor);
        state.statistics_mut().backward_rewritten_count = 1;

        let storage_before_cleanup = proof_state_storage_estimate(&state);
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().filter_orphans_limit = 0;
        control.heuristic_parms_mut().delete_bad_limit = storage_before_cleanup - 1;

        let outcome = proof_state_cleanup_unprocessed_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));
        let storage_after_cleanup = proof_state_storage_estimate(&state);

        assert!(outcome.orphan_cleanup_triggered);
        assert_eq!(outcome.orphan_cleanup_deleted, 1);
        assert!(!outcome.delete_bad_triggered);
        assert!(storage_after_cleanup < storage_before_cleanup);
        assert!(storage_after_cleanup <= control.heuristic_parms().delete_bad_limit);
    }

    #[test]
    fn proof_state_storage_estimate_aggregates_c_storage_domains() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let unprocessed = unit_clause_with_id(state.terms_mut(), "pc_storage_unprocessed", 4_120);
        let archive = unit_clause_with_id(state.terms_mut(), "pc_storage_archive", 4_121);
        state.unprocessed_mut().insert(unprocessed);
        state.archive_mut().insert(archive);

        let expected = [
            state.unprocessed().storage_estimate(),
            state.processed_pos_rules().storage_estimate(),
            state.processed_pos_eqns().storage_estimate(),
            state.processed_neg_units().storage_estimate(),
            state.processed_non_units().storage_estimate(),
            state.archive().storage_estimate(),
            state.terms().storage_estimate(),
        ]
        .into_iter()
        .fold(0_i64, i64::saturating_add);

        assert_eq!(proof_state_storage_estimate(&state), expected);
        assert!(proof_state_storage_estimate(&state) > state.terms().non_var_term_nodes());
    }

    #[test]
    fn proof_state_cleanup_unprocessed_forward_contracts_and_reweights() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (trivial, survivor) = {
            let terms = state.terms_mut();
            let same = typed_const(terms, "pc_cleanup_forward_same");
            let mut trivial =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &same, &same, true)]));
            trivial.set_ident(4_113);
            let survivor = unit_clause_with_id(terms, "pc_cleanup_forward_survivor", 4_114);
            (trivial, survivor)
        };
        state.unprocessed_mut().insert(trivial);
        state.unprocessed_mut().insert(survivor);
        state.statistics_mut().processed_count = 3;

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "CleanupForwardContractTest");
        control.heuristic_parms_mut().forward_contract_limit = 0;

        let outcome =
            proof_state_cleanup_unprocessed_clauses_with(&mut state, &mut control, 0, |_| false)
                .unwrap_or_else(|err| panic!("{err}"));

        assert!(outcome.unsatisfiable.is_none());
        assert!(outcome.forward_contract_triggered);
        assert_eq!(outcome.forward_contract_deleted, 1);
        assert_eq!(outcome.forward_contract_remaining, 1);
        assert_eq!(state.unprocessed().members(), 1);
        let survivor = state.unprocessed().find_by_id(4_114).unwrap();
        assert!(survivor.evaluations().is_some());
        assert_eq!(state.statistics().other_redundant_count, 1);
        assert_eq!(state.statistics().forward_contract_base, 3);
    }

    #[test]
    fn proof_state_cleanup_unprocessed_with_docs_records_preprocessing_refutation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let same = typed_const(terms, "pc_cleanup_forward_doc_same");
            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &same, &same, false)]));
            clause.set_ident(4_115);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW);
            clause
        };
        state.unprocessed_mut().insert(clause);
        state.statistics_mut().processed_count = 1;

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().forward_contract_limit = 0;
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let outcome = proof_state_cleanup_unprocessed_clauses_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let empty = outcome
            .unsatisfiable
            .expect("documenting cleanup should return the minimized empty clause");
        assert!(empty.is_empty());
        assert_eq!(empty.ident(), 1);
        assert!(outcome.forward_contract_triggered);
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("cn(4115)"), "{rendered}");
    }

    #[test]
    fn proof_state_cleanup_unprocessed_delete_bad_keeps_best_half_and_marks_incomplete() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (first, second) = {
            let terms = state.terms_mut();
            (
                unit_clause_with_id(terms, "pc_cleanup_bad_first", 4_115),
                unit_clause_with_id(terms, "pc_cleanup_bad_second", 4_116),
            )
        };
        state.unprocessed_mut().insert(first);
        state.unprocessed_mut().insert(second);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "CleanupDeleteBadTest");
        {
            let mut unprocessed = std::mem::take(state.unprocessed_mut());
            proof_control_clause_set_reweight_with_bank(
                &mut control,
                state.terms_mut(),
                &mut unprocessed,
            )
            .unwrap_or_else(|err| panic!("{err}"));
            *state.unprocessed_mut() = unprocessed;
        }
        control.heuristic_parms_mut().delete_bad_limit = 0;
        let current_storage = proof_state_storage_estimate(&state).max(1);

        let outcome = proof_state_cleanup_unprocessed_clauses_with(
            &mut state,
            &mut control,
            current_storage,
            |_| false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.bad_deleted, 1);
        assert!(outcome.delete_bad_triggered);
        assert_eq!(outcome.delete_bad_orphaned_deleted, 0);
        assert_eq!(state.unprocessed().members(), 1);
        assert!(!state.state_is_complete());
        assert_eq!(state.statistics().non_redundant_deleted, 0);
        assert!(outcome.term_gc_recovered >= 0);
    }

    #[test]
    fn cleanup_unprocessed_output_renders_c_messages() {
        let mut output = Vec::new();
        let outcome = CleanupUnprocessedOutcome {
            orphan_cleanup_triggered: true,
            orphan_cleanup_deleted: 2,
            orphan_cleanup_remaining: 5,
            forward_contract_triggered: true,
            forward_contract_deleted: 3,
            forward_contract_remaining: 2,
            delete_bad_triggered: true,
            delete_bad_orphaned_deleted: 1,
            bad_deleted: 4,
            ..CleanupUnprocessedOutcome::default()
        };

        write_cleanup_unprocessed_output(&mut output, 1, &outcome)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Deleted 2 orphaned clauses (remaining: 5)\n\
% Special forward-contraction deletes 3 clauses(remaining: 2) \n\
% Reweighting unprocessed clauses...\n\
% Deleted 1 orphaned clauses and 4 bad clauses (prover may be incomplete now)\n"
        );
    }

    #[test]
    fn cleanup_unprocessed_output_suppresses_reweight_after_empty_clause() {
        let mut output = Vec::new();
        let outcome = CleanupUnprocessedOutcome {
            unsatisfiable: Some(Clause::empty()),
            forward_contract_triggered: true,
            forward_contract_deleted: 1,
            forward_contract_remaining: 0,
            ..CleanupUnprocessedOutcome::default()
        };

        write_cleanup_unprocessed_output(&mut output, 1, &outcome)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Special forward-contraction deletes 1 clauses(remaining: 0) \n"
        );
    }

    #[test]
    fn cleanup_unprocessed_output_is_quiet_at_output_level_zero() {
        let mut output = Vec::new();
        let outcome = CleanupUnprocessedOutcome {
            orphan_cleanup_triggered: true,
            orphan_cleanup_deleted: 2,
            orphan_cleanup_remaining: 5,
            forward_contract_triggered: true,
            forward_contract_deleted: 3,
            forward_contract_remaining: 2,
            delete_bad_triggered: true,
            delete_bad_orphaned_deleted: 1,
            bad_deleted: 4,
            ..CleanupUnprocessedOutcome::default()
        };

        write_cleanup_unprocessed_output(&mut output, 0, &outcome)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(output.is_empty());
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
    fn proof_control_random_weight_drives_live_eval_and_selection_with_c_sequence() {
        let _guard = global_state_lock();
        reset_jkiss_for_tests();
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (first, second) = {
            let terms = state.terms_mut();
            (
                unit_clause_with_id(terms, "pc_random_eval_first", 4_063),
                unit_clause_with_id(terms, "pc_random_eval_second", 4_064),
            )
        };
        state.eval_store_mut().insert(first);
        state.eval_store_mut().insert(second);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut params = HeuristicParmsCell {
            heuristic_name: "RandomEvalStoreTest".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let wfcb_defs = vec!["random_eval=RandomWeight(ConstPrio,1000,0,0,11,13,17)".to_owned()];
        let mut hcb_defs = vec!["RandomEvalStoreTest=(1*random_eval)".to_owned()];
        proof_control_init_heuristics(
            &mut control,
            state.axioms(),
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            proof_state_eval_clause_set(&mut state, &mut control)
                .unwrap_or_else(|err| panic!("{err}")),
            2
        );
        let first_weight = state
            .eval_store()
            .find_by_id(4_063)
            .and_then(Clause::evaluations)
            .expect("first random-weight evaluation")
            .eval(0)
            .heuristic();
        let second_weight = state
            .eval_store()
            .find_by_id(4_064)
            .and_then(Clause::evaluations)
            .expect("second random-weight evaluation")
            .eval(0)
            .heuristic();

        assert_eq!(first_weight.to_bits(), 1_124_233_471);
        assert_eq!(second_weight.to_bits(), 1_142_390_271);
        assert_eq!(
            state.eval_store().find_best(0).map(Clause::ident),
            Some(4_063)
        );

        assert_eq!(proof_state_move_eval_store_to_unprocessed(&mut state), 2);
        let selected = proof_state_select_unprocessed_clause(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"))
            .expect("random-weight HCB should select a clause");
        assert_eq!(selected.ident(), 4_063);
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
    fn proof_state_move_eval_store_to_unprocessed_with_docs_emits_eval_quote() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut clause = unit_clause_with_id(state.terms_mut(), "pc_eval_store_doc", 4_065);
        clause.set_prop(CP_INPUT_FORMULA);
        clause.set_prop(CP_IS_ORIENTED);
        state.eval_store_mut().insert(clause);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let moved = proof_state_move_eval_store_to_unprocessed_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(moved, 1);
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("1 : :"));
        assert!(rendered.contains("4065 : 'eval'"));
        assert!(state.eval_store().is_empty());
        let moved_clause = state.unprocessed().find_by_id(1).unwrap();
        assert!(!moved_clause.query_prop(CP_INPUT_FORMULA | CP_IS_ORIENTED));
    }

    #[test]
    fn proof_state_queue_generated_clause_for_eval_selects_and_stamps_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.statistics_mut().proc_non_trivial_count = 77;
        let mut control = proof_control_alloc();
        control.set_record_gc_selection(true);
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
        assert_eq!(
            queued
                .derivation()
                .map(crate::basics::pstacks::PStack::as_slice),
            Some(&[DerivationEntry::Operation(DC_CNF_EVAL_GC)][..])
        );
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
    fn proof_state_insert_new_clauses_with_docs_emits_eval_quote() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.statistics_mut().proc_non_trivial_count = 92;
        let mut clause = unit_clause_with_id(state.terms_mut(), "pc_insert_new_doc_eval", 4_076);
        clause.set_prop(CP_INPUT_FORMULA);
        clause.set_prop(CP_IS_ORIENTED);
        state.tmp_store_mut().insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewDocEval");
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);
        let mut rendered = String::new();

        let empty = proof_state_insert_new_clauses_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(rendered.contains("1 : :"));
        assert!(rendered.contains("4076 : 'eval'"));
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        let queued = state.unprocessed().find_by_id(1).unwrap();
        assert_eq!(queued.create_date(), 92);
        assert!(!queued.query_prop(CP_INPUT_FORMULA | CP_IS_ORIENTED));
        assert!(queued.evaluations().is_some());
        assert_eq!(state.statistics().generated_count, 1);
        assert_eq!(state.statistics().non_trivial_generated_count, 1);
    }

    #[test]
    fn proof_state_insert_new_clauses_with_output_reports_dynamic_watchlist_reduction() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state.statistics_mut().proc_non_trivial_count = 93;
        let (generated, watched) =
            watchlist_subsumption_pair(state.terms_mut(), "pc_insert_watch_output", 4_077, 4_078);
        state.tmp_store_mut().insert(generated);
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewWatchOutput");
        control.heuristic_parms_mut().watchlist_is_static = false;
        proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| panic!("{err}"));
        let mut output = Vec::new();

        let empty =
            proof_state_insert_new_clauses_with_output(&mut output, 1, &mut state, &mut control)
                .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Watchlist reduced by 1 clause\n"
        );
        assert_eq!(state.watchlist().unwrap().members(), 0);
        assert_eq!(state.archive().members(), 1);
        assert_eq!(state.unprocessed().members(), 1);
        let queued = state.unprocessed().find_by_id(4_077).unwrap();
        assert!(queued.query_prop(CP_SUBSUMES_WATCH));
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
    fn proof_state_insert_new_clauses_requeues_destructive_er_result() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (clause, rhs) = {
            let terms = state.terms_mut();
            let x = typed_var(terms, -30);
            let y = typed_var(terms, -32);
            let rhs = typed_const(terms, "pc_insert_new_er_rhs");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &x, &rhs, true),
                literal(terms, &x, &y, false),
            ]));
            clause.set_ident(4_081);
            (clause, rhs)
        };
        state.tmp_store_mut().insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewDestructiveErTest");
        control.heuristic_parms_mut().er_aggressive = true;
        control.heuristic_parms_mut().er_varlit_destructive = true;

        let empty = proof_state_insert_new_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(state.unprocessed().members(), 1);
        assert_eq!(state.statistics().generated_count, 2);
        assert_eq!(state.statistics().generated_lit_count, 2);
        assert_eq!(state.statistics().other_redundant_count, 1);
        assert_eq!(state.statistics().resolv_count, 1);
        assert_eq!(state.statistics().non_trivial_generated_count, 1);
        let queued = state.unprocessed().find_by_id(4_081).unwrap();
        assert_eq!(queued.proof_depth(), 1);
        assert_eq!(queued.proof_size(), 1);
        assert_eq!(queued.literal_number(), 1);
        let literal = &queued.literals().as_slice()[0];
        assert!(literal.is_positive());
        assert!(literal.left().is_free_var());
        assert_eq!(literal.right(), &rhs);
    }

    #[test]
    fn proof_state_insert_new_clauses_with_docs_quotes_destructive_er() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (clause, rhs) = {
            let terms = state.terms_mut();
            let x = typed_var(terms, -34);
            let y = typed_var(terms, -36);
            let rhs = typed_const(terms, "pc_insert_new_doc_er_rhs");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &x, &rhs, true),
                literal(terms, &x, &y, false),
            ]));
            clause.set_ident(4_082);
            (clause, rhs)
        };
        state.tmp_store_mut().insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewDocDestructiveErTest");
        control.heuristic_parms_mut().er_aggressive = true;
        control.heuristic_parms_mut().er_varlit_destructive = true;
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let empty = proof_state_insert_new_clauses_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(output.contains(" : er(4082)\n"));
        assert!(state.unprocessed().find_by_id(4_082).is_none());
        let queued = state.unprocessed().find_by_id(1).unwrap();
        assert_eq!(queued.literal_number(), 1);
        assert_eq!(queued.proof_depth(), 1);
        assert_eq!(queued.proof_size(), 1);
        assert_eq!(queued.literals().as_slice()[0].right(), &rhs);
    }

    #[test]
    fn proof_state_insert_new_clauses_requeues_fresh_split_result() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let left_var = typed_var(terms, -40);
            let right_var = typed_var(terms, -42);
            let left_const = typed_const(terms, "pc_insert_new_split_left");
            let right_const = typed_const(terms, "pc_insert_new_split_right");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left_var, &left_const, true),
                literal(terms, &right_var, &right_const, true),
            ]));
            clause.set_ident(4_082);
            clause
        };
        state.tmp_store_mut().insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewFreshSplitTest");
        control.heuristic_parms_mut().split_aggressive = true;
        control.heuristic_parms_mut().split_clauses = SplitClassType::ALL;
        control.heuristic_parms_mut().split_method = SplitType::GroundFull;

        let empty = proof_state_insert_new_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(state.definition_store().members(), 0);
        assert_eq!(state.definition_assocs().len(), 0);
        assert_eq!(state.definition_formula_assocs().len(), 0);
        assert_eq!(state.definition_formula_archive().cardinality(), 2);
        assert_eq!(state.f_archive().cardinality(), 0);
        assert_eq!(state.unprocessed().members(), 3);
        assert_eq!(state.statistics().generated_count, 4);
        assert_eq!(state.statistics().generated_lit_count, 2);
        assert_eq!(state.statistics().non_trivial_generated_count, 3);

        let residual = state.unprocessed().find_by_id(4_082).unwrap();
        assert_eq!(residual.literal_number(), 2);
        assert!(residual.literals().as_slice().iter().all(Eqn::is_negative));
        assert!(residual
            .literals()
            .as_slice()
            .iter()
            .all(|literal| literal.query_prop(EP_IS_SPLIT_LIT)));
        let split_literal_count = state
            .unprocessed()
            .iter()
            .flat_map(|clause| clause.literals().as_slice())
            .filter(|literal| literal.query_prop(EP_IS_SPLIT_LIT))
            .count();
        assert_eq!(split_literal_count, 4);
    }

    #[test]
    fn proof_state_insert_new_clauses_with_docs_documents_fresh_split_result() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let left_var = typed_var(terms, -41);
            let right_var = typed_var(terms, -43);
            let left_const = typed_const(terms, "pc_insert_doc_split_left");
            let right_const = typed_const(terms, "pc_insert_doc_split_right");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left_var, &left_const, true),
                literal(terms, &right_var, &right_const, true),
            ]));
            clause.set_ident(4_082);
            clause
        };
        state.tmp_store_mut().insert(clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewDocFreshSplitTest");
        control.heuristic_parms_mut().split_aggressive = true;
        control.heuristic_parms_mut().split_clauses = SplitClassType::ALL;
        control.heuristic_parms_mut().split_method = SplitType::GroundFull;
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let empty = proof_state_insert_new_clauses_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert_eq!(session.id_source.current_ident(), 5);
        assert_eq!(output.matches(" : introduced : 'split'\n").count(), 2);
        assert!(output.contains(" : split_equiv(1)\n"));
        assert!(output.contains(" : split_equiv(3)\n"));
        assert!(output.contains(" : apply_def(apply_def(4082,1),3) : 'split'\n"));
        assert_eq!(
            state
                .definition_formula_archive()
                .iter()
                .map(WrappedFormula::ident)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert!(state.unprocessed().find_by_id(4_082).is_none());
        assert!(state.unprocessed().find_by_id(5).is_some());
    }

    #[test]
    fn proof_state_insert_new_clauses_reuses_split_definitions() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (first_clause, second_clause) = {
            let terms = state.terms_mut();
            let first_const = typed_const(terms, "pc_insert_new_reuse_first");
            let second_const = typed_const(terms, "pc_insert_new_reuse_second");
            let mut first_clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &typed_var(terms, -44), &first_const, true),
                literal(terms, &typed_var(terms, -46), &second_const, true),
            ]));
            first_clause.set_ident(4_083);
            let mut second_clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &typed_var(terms, -48), &first_const, true),
                literal(terms, &typed_var(terms, -50), &second_const, true),
            ]));
            second_clause.set_ident(4_084);
            (first_clause, second_clause)
        };
        state.tmp_store_mut().insert(first_clause);
        state.tmp_store_mut().insert(second_clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewReuseSplitTest");
        control.heuristic_parms_mut().split_aggressive = true;
        control.heuristic_parms_mut().split_clauses = SplitClassType::ALL;
        control.heuristic_parms_mut().split_method = SplitType::GroundFull;
        control.heuristic_parms_mut().split_fresh_defs = false;

        let empty = proof_state_insert_new_clauses(&mut state, &mut control)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(state.definition_store().members(), 2);
        assert_eq!(state.definition_assocs().len(), 2);
        assert_eq!(state.definition_formula_assocs().len(), 2);
        assert_eq!(state.definition_formula_archive().cardinality(), 2);
        assert_eq!(state.f_archive().cardinality(), 0);
        assert_eq!(state.unprocessed().members(), 4);
        assert_eq!(state.statistics().generated_count, 8);
        assert_eq!(state.statistics().generated_lit_count, 4);
        assert_eq!(state.statistics().non_trivial_generated_count, 4);
    }

    #[test]
    fn proof_state_insert_new_clauses_with_docs_reuses_split_definition_ids() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (first_clause, second_clause) = {
            let terms = state.terms_mut();
            let first_const = typed_const(terms, "pc_insert_doc_reuse_first");
            let second_const = typed_const(terms, "pc_insert_doc_reuse_second");
            let mut first_clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &typed_var(terms, -45), &first_const, true),
                literal(terms, &typed_var(terms, -47), &second_const, true),
            ]));
            first_clause.set_ident(4_083);
            let mut second_clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &typed_var(terms, -49), &first_const, true),
                literal(terms, &typed_var(terms, -51), &second_const, true),
            ]));
            second_clause.set_ident(4_084);
            (first_clause, second_clause)
        };
        state.tmp_store_mut().insert(first_clause);
        state.tmp_store_mut().insert(second_clause);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "InsertNewDocReuseSplitTest");
        control.heuristic_parms_mut().split_aggressive = true;
        control.heuristic_parms_mut().split_clauses = SplitClassType::ALL;
        control.heuristic_parms_mut().split_method = SplitType::GroundFull;
        control.heuristic_parms_mut().split_fresh_defs = false;
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let empty = proof_state_insert_new_clauses_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(empty.is_none());
        assert_eq!(session.id_source.current_ident(), 6);
        assert_eq!(output.matches(" : introduced : 'split'\n").count(), 2);
        assert_eq!(output.matches(" : split_equiv(").count(), 2);
        assert!(output.contains(" : apply_def(apply_def(4083,1),3) : 'split'\n"));
        assert!(output.contains(" : apply_def(apply_def(4084,1),3) : 'split'\n"));
        let mut parent_ids = state
            .definition_formula_assocs()
            .values()
            .map(|parent| parent.ident())
            .collect::<Vec<_>>();
        parent_ids.sort_unstable();
        assert_eq!(parent_ids, vec![1, 3]);
        assert_eq!(
            state
                .definition_formula_archive()
                .iter()
                .map(WrappedFormula::ident)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert!(state.unprocessed().find_by_id(4_084).is_none());
        assert!(state.unprocessed().find_by_id(6).is_some());
    }

    #[test]
    fn proof_state_replacing_inferences_returns_surviving_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = unit_clause_with_id(state.terms_mut(), "pc_replacing_survivor", 4_084);
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();

        let outcome = proof_state_replacing_inferences(&mut state, &mut control, packed)
            .unwrap_or_else(|err| panic!("{err}"));

        let ReplacingInferenceOutcome::Survivor(survivor) = outcome else {
            panic!("clause should survive without replacement");
        };
        assert_eq!(survivor.ident(), 4_084);
        assert!(state.tmp_store().is_empty());
        assert!(state.unprocessed().is_empty());
        assert_eq!(state.statistics().generated_count, 0);
        assert_eq!(state.statistics().other_redundant_count, 0);
    }

    #[test]
    fn proof_state_replacing_inferences_higher_order_non_clausifiable_survives() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = unit_clause_with_id(state.terms_mut(), "pc_replacing_ho_survivor", 4_084);
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();

        let outcome = proof_state_replacing_inferences(&mut state, &mut control, packed)
            .unwrap_or_else(|err| panic!("{err}"));

        let ReplacingInferenceOutcome::Survivor(survivor) = outcome else {
            panic!("non-clausifiable higher-order clause should fall through");
        };
        assert_eq!(survivor.ident(), 4_084);
        assert!(state.tmp_store().is_empty());
        assert!(state.unprocessed().is_empty());
    }

    #[test]
    fn proof_state_replacing_inferences_higher_order_clausifiable_requeues_dynamic_cnf() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let truth = terms.true_term().clone();
            let falsity = terms.false_term().clone();
            let equiv_code = terms.signature().equiv_code();
            let encoded = bool_binary_with_code(terms, equiv_code, &truth, &falsity);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &encoded, &truth, true,
            )]));
            clause.set_ident(4_085);
            clause
        };
        assert!(clause.literals().as_slice()[0].is_clausifiable(state.terms()));
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();
        control.set_ocb(empty_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ReplacingImmediateCnfTest");

        let outcome = proof_state_replacing_inferences(&mut state, &mut control, packed)
            .unwrap_or_else(|err| panic!("{err}"));

        let ReplacingInferenceOutcome::Replaced { empty } = outcome else {
            panic!("clausifiable higher-order literal should replace the selected clause");
        };
        let empty = empty.expect("dynamic CNF of $true <=> $false should derive empty clause");
        assert_eq!(empty.literal_number(), 0);
        assert!(derivation_contains_operation(&empty, DC_DYNAMIC_CNF));
        assert!(derivation_contains_parent(&empty, 4_085));
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert!(state.unprocessed().is_empty());
        assert!(state.archive().find_by_id(4_085).is_some());
        assert_eq!(state.statistics().generated_count, 2);
        assert_eq!(state.statistics().generated_lit_count, 4);
    }

    #[test]
    fn proof_state_replacing_inferences_with_docs_documents_dynamic_cnf() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let truth = terms.true_term().clone();
            let falsity = terms.false_term().clone();
            let equiv_code = terms.signature().equiv_code();
            let encoded = bool_binary_with_code(terms, equiv_code, &truth, &falsity);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &encoded, &truth, true,
            )]));
            clause.set_ident(4_086);
            clause
        };
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();
        control.set_ocb(empty_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ReplacingImmediateCnfDocTest");
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::HigherOrder);

        let outcome = proof_state_replacing_inferences_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
            packed,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ReplacingInferenceOutcome::Replaced { empty } = outcome else {
            panic!("clausifiable higher-order literal should replace the selected clause");
        };
        let empty = empty.expect("documented dynamic CNF should derive an empty clause");
        assert!(output.contains("split_conjunct("));
        assert!(session.id_source.current_ident() > 0);
        assert!(derivation_contains_operation(&empty, DC_DYNAMIC_CNF));
        assert!(derivation_contains_parent(&empty, 4_086));
        assert!(state.archive().find_by_id(4_086).is_some());
    }

    #[test]
    fn proof_state_immediate_clausification_introduces_expensive_definition() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = immediate_definition_clause(state.terms_mut(), 4_086);
        assert!(clause.literals().as_slice()[0].is_clausifiable(state.terms()));
        let symbol_count_before = state.terms().signature().f_count();

        proof_state_immediate_clausification(&mut state, clause, false)
            .unwrap_or_else(|err| panic!("{err}"));

        let signature = state.terms().signature();
        assert!((symbol_count_before + 1..=signature.f_count())
            .any(|f_code| signature.query_prop(f_code, FP_DEF_PRED)));
        assert!(!state.tmp_store().is_empty());
        for generated in state.tmp_store().iter() {
            assert!(derivation_contains_operation(generated, DC_DYNAMIC_CNF));
            assert!(derivation_contains_parent(generated, 4_086));
        }
        assert!(state.archive().find_by_id(4_086).is_some());
    }

    #[test]
    fn proof_state_immediate_clausification_with_docs_keeps_formula_parent_chain() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = immediate_definition_clause(state.terms_mut(), 4_087);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::HigherOrder);

        proof_state_immediate_clausification_with_docs(
            &mut output,
            &mut session,
            &mut state,
            clause,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let intro_pos = output.find(" : introduced\n").unwrap();
        let split_pos = output.find(" : split_equiv(").unwrap();
        let apply_pos = output.find(" : apply_def(").unwrap();
        let clause_pos = output.rfind("split_conjunct(").unwrap();
        assert!(intro_pos < split_pos);
        assert!(split_pos < apply_pos);
        assert!(apply_pos < clause_pos);
        assert_eq!(
            i64::try_from(output.matches("split_conjunct(").count()).unwrap(),
            state.tmp_store().members()
        );
        assert!(state.tmp_store().iter().all(|generated| {
            generated.ident() > 0
                && derivation_contains_operation(generated, DC_DYNAMIC_CNF)
                && derivation_contains_parent(generated, 4_087)
        }));
        assert!(state.archive().find_by_id(4_087).is_some());
        assert!(session.id_source.current_ident() > 0);
    }

    #[test]
    fn proof_state_replacing_inferences_requeues_destructive_er_result() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (clause, rhs) = {
            let terms = state.terms_mut();
            let x = typed_var(terms, -50);
            let y = typed_var(terms, -52);
            let rhs = typed_const(terms, "pc_replacing_er_rhs");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &x, &rhs, true),
                literal(terms, &x, &y, false),
            ]));
            clause.set_ident(4_085);
            (clause, rhs)
        };
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ReplacingDestructiveErTest");
        control.heuristic_parms_mut().er_varlit_destructive = true;

        let outcome = proof_state_replacing_inferences(&mut state, &mut control, packed)
            .unwrap_or_else(|err| panic!("{err}"));

        let ReplacingInferenceOutcome::Replaced { empty } = outcome else {
            panic!("destructive equality resolution should replace the clause");
        };
        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(state.unprocessed().members(), 1);
        assert_eq!(state.statistics().generated_count, 1);
        assert_eq!(state.statistics().generated_lit_count, 1);
        assert_eq!(state.statistics().other_redundant_count, 1);
        assert_eq!(state.statistics().resolv_count, 1);
        assert_eq!(state.statistics().non_trivial_generated_count, 1);
        let queued = state.unprocessed().find_by_id(4_085).unwrap();
        assert_eq!(queued.literal_number(), 1);
        assert_eq!(queued.proof_depth(), 1);
        assert_eq!(queued.proof_size(), 1);
        assert_eq!(queued.literals().as_slice()[0].right(), &rhs);
    }

    #[test]
    fn proof_state_replacing_inferences_with_docs_quotes_destructive_er() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (clause, rhs) = {
            let terms = state.terms_mut();
            let x = typed_var(terms, -54);
            let y = typed_var(terms, -56);
            let rhs = typed_const(terms, "pc_replacing_doc_er_rhs");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &x, &rhs, true),
                literal(terms, &x, &y, false),
            ]));
            clause.set_ident(4_086);
            (clause, rhs)
        };
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ReplacingDocDestructiveErTest");
        control.heuristic_parms_mut().er_varlit_destructive = true;
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_replacing_inferences_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
            packed,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, ReplacingInferenceOutcome::Replaced { empty: None });
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(session.id_source.current_ident(), 1);
        assert!(output.contains(" : er(4086)\n"));
        assert!(state.unprocessed().find_by_id(4_086).is_none());
        let queued = state.unprocessed().find_by_id(1).unwrap();
        assert_eq!(queued.literal_number(), 1);
        assert_eq!(queued.literals().as_slice()[0].right(), &rhs);
    }

    #[test]
    fn proof_state_replacing_inferences_requeues_fresh_split_result() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let left_var = typed_var(terms, -60);
            let right_var = typed_var(terms, -62);
            let left_const = typed_const(terms, "pc_replacing_split_left");
            let right_const = typed_const(terms, "pc_replacing_split_right");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left_var, &left_const, true),
                literal(terms, &right_var, &right_const, true),
            ]));
            clause.set_ident(4_086);
            clause
        };
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ReplacingFreshSplitTest");
        control.heuristic_parms_mut().split_clauses = SplitClassType::ALL;
        control.heuristic_parms_mut().split_method = SplitType::GroundFull;

        let outcome = proof_state_replacing_inferences(&mut state, &mut control, packed)
            .unwrap_or_else(|err| panic!("{err}"));

        let ReplacingInferenceOutcome::Replaced { empty } = outcome else {
            panic!("controlled splitting should replace the clause");
        };
        assert!(empty.is_none());
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(state.definition_store().members(), 0);
        assert_eq!(state.definition_assocs().len(), 0);
        assert_eq!(state.definition_formula_assocs().len(), 0);
        assert_eq!(state.definition_formula_archive().cardinality(), 2);
        assert_eq!(state.f_archive().cardinality(), 0);
        assert_eq!(state.unprocessed().members(), 3);
        assert_eq!(state.statistics().generated_count, 3);
        assert_eq!(state.statistics().generated_lit_count, 6);
        assert_eq!(state.statistics().non_trivial_generated_count, 3);

        let residual = state.unprocessed().find_by_id(4_086).unwrap();
        assert_eq!(residual.literal_number(), 2);
        assert!(residual.literals().as_slice().iter().all(Eqn::is_negative));
        assert!(residual
            .literals()
            .as_slice()
            .iter()
            .all(|literal| literal.query_prop(EP_IS_SPLIT_LIT)));
    }

    #[test]
    fn proof_state_replacing_inferences_with_docs_documents_fresh_split_result() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let left_var = typed_var(terms, -61);
            let right_var = typed_var(terms, -63);
            let left_const = typed_const(terms, "pc_replacing_doc_split_left");
            let right_const = typed_const(terms, "pc_replacing_doc_split_right");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left_var, &left_const, true),
                literal(terms, &right_var, &right_const, true),
            ]));
            clause.set_ident(4_086);
            clause
        };
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ReplacingDocFreshSplitTest");
        control.heuristic_parms_mut().split_clauses = SplitClassType::ALL;
        control.heuristic_parms_mut().split_method = SplitType::GroundFull;
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_replacing_inferences_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
            packed,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, ReplacingInferenceOutcome::Replaced { empty: None });
        assert_eq!(session.id_source.current_ident(), 5);
        assert_eq!(output.matches(" : introduced : 'split'\n").count(), 2);
        assert!(output.contains(" : split_equiv(1)\n"));
        assert!(output.contains(" : split_equiv(3)\n"));
        assert!(output.contains(" : apply_def(apply_def(4086,1),3) : 'split'\n"));
        assert_eq!(
            state
                .definition_formula_archive()
                .iter()
                .map(WrappedFormula::ident)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert!(state.unprocessed().find_by_id(4_086).is_none());
        assert!(state.unprocessed().find_by_id(5).is_some());
    }

    #[test]
    fn proof_state_replacing_inferences_reuses_split_definitions() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (first_clause, second_clause) = {
            let terms = state.terms_mut();
            let first_const = typed_const(terms, "pc_replacing_reuse_first");
            let second_const = typed_const(terms, "pc_replacing_reuse_second");
            let mut first_clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &typed_var(terms, -64), &first_const, true),
                literal(terms, &typed_var(terms, -66), &second_const, true),
            ]));
            first_clause.set_ident(4_087);
            let mut second_clause = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &typed_var(terms, -68), &first_const, true),
                literal(terms, &typed_var(terms, -70), &second_const, true),
            ]));
            second_clause.set_ident(4_088);
            (first_clause, second_clause)
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        init_fifo_hcb(&mut control, &state, "ReplacingReuseSplitTest");
        control.heuristic_parms_mut().split_clauses = SplitClassType::ALL;
        control.heuristic_parms_mut().split_method = SplitType::GroundFull;
        control.heuristic_parms_mut().split_fresh_defs = false;

        let first = proof_state_replacing_inferences(
            &mut state,
            &mut control,
            fv_index_pack_clause(first_clause, None),
        )
        .unwrap_or_else(|err| panic!("{err}"));
        let second = proof_state_replacing_inferences(
            &mut state,
            &mut control,
            fv_index_pack_clause(second_clause, None),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(first, ReplacingInferenceOutcome::Replaced { empty: None });
        assert_eq!(second, ReplacingInferenceOutcome::Replaced { empty: None });
        assert!(state.tmp_store().is_empty());
        assert!(state.eval_store().is_empty());
        assert_eq!(state.definition_store().members(), 2);
        assert_eq!(state.definition_assocs().len(), 2);
        assert_eq!(state.definition_formula_assocs().len(), 2);
        assert_eq!(state.definition_formula_archive().cardinality(), 2);
        assert_eq!(state.f_archive().cardinality(), 0);
        assert_eq!(state.unprocessed().members(), 4);
        assert_eq!(state.statistics().generated_count, 4);
        assert_eq!(state.statistics().generated_lit_count, 8);
        assert_eq!(state.statistics().non_trivial_generated_count, 4);
    }

    #[test]
    fn proof_state_process_clause_does_not_resplit_split_children() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let x = typed_var(terms, -72);
            let y = typed_var(terms, -74);
            let p_code = unary_predicate_code(terms, "pc_process_split_child_p");
            let q_code = unary_predicate_code(terms, "pc_process_split_child_q");
            let p_x = unary_predicate(terms, p_code, &x);
            let q_y = unary_predicate(terms, q_code, &y);
            let true_term = terms.true_term().clone();
            let first = Eqn::alloc(p_x, true_term.clone(), terms, true).unwrap();
            let second = Eqn::alloc(q_y, true_term, terms, true).unwrap();
            let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
            clause.set_ident(4_089);
            clause
        };
        let packed = fv_index_pack_clause(clause, None);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        control.heuristic_parms_mut().split_clauses = SplitClassType::ALL;
        control.heuristic_parms_mut().split_method = SplitType::GroundFull;

        let outcome = proof_state_replacing_inferences(&mut state, &mut control, packed)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, ReplacingInferenceOutcome::Replaced { empty: None });
        assert_eq!(state.unprocessed().members(), 3);
        let generated_count = state.statistics().generated_count;
        for _ in 0..3 {
            let outcome = proof_state_process_clause(&mut state, &mut control, 1)
                .unwrap_or_else(|err| panic!("{err}"));
            assert!(
                matches!(outcome, ProcessClauseOutcome::Processed { .. }),
                "split child should be processed without another split: {outcome:?}"
            );
        }
        assert_eq!(state.unprocessed().members(), 0);
        assert_eq!(state.statistics().generated_count, generated_count);
    }

    #[test]
    fn proof_state_process_clause_returns_no_clause_for_empty_unprocessed() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, ProcessClauseOutcome::NoClause);
        assert_eq!(state.statistics().processed_count, 0);
    }

    #[test]
    fn proof_state_process_clause_skips_orphaned_best_unprocessed_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut parent = Clause::empty();
        parent.set_ident(4_144);
        let mut orphan = unit_clause_with_id(state.terms_mut(), "pc_process_orphan", 4_145);
        clause_push_derivation(&mut orphan, DC_ORDERED_FACTOR, Some(&parent), None);
        parent.set_prop(CP_IS_DEAD);
        state.archive_mut().insert(parent);
        let survivor = unit_clause_with_id(state.terms_mut(), "pc_process_after_orphan", 4_146);

        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, orphan);
        queue_unprocessed_for_process(&mut state, &mut control, survivor);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(matches!(outcome, ProcessClauseOutcome::Processed { .. }));
        assert!(state.unprocessed().find_by_id(4_145).is_none());
        assert!(state.unprocessed().find_by_id(4_146).is_none());
        assert!(state.processed_pos_eqns().find_by_id(4_146).is_some());
        assert_eq!(state.processed_cardinality(), 1);
    }

    #[test]
    fn proof_state_process_clause_allows_plain_paramodulation_generation_without_partners() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_fifo_hcb(
            &mut control,
            &state,
            "ProcessClausePlainParamodNoPartnersTest",
        );
        control.set_ocb(kbo_ocb(state.terms()));
        let clause = unit_clause_with_id(state.terms_mut(), "pc_process_paramod_needed", 4_143);
        queue_unprocessed_for_process(&mut state, &mut control, clause);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed { generation, .. } = outcome else {
            panic!("plain paramodulation should not reject without partner clauses");
        };
        assert_eq!(generation.paramodulants, 0);
        assert_eq!(state.statistics().paramod_count, 0);
    }

    #[test]
    fn proof_state_process_clause_copies_normalized_clause_disjointly_without_generation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -2);
            let skolem = typed_unary(terms, "pc_process_disjoint_skolem", &variable);
            let predicate_code = unary_predicate_code(terms, "pc_process_disjoint_predicate");
            let predicate = unary_predicate(terms, predicate_code, &variable);
            let truth = terms.true_term().clone();
            let first = literal(terms, &skolem, &variable, false);
            let second = literal(terms, &predicate, &truth, false);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
            clause.set_ident(4_147);
            clause
        };
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, clause);
        let terms_before = state.terms().in_count();

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(matches!(
            outcome,
            ProcessClauseOutcome::Processed {
                class: ProcessedClauseClass::NonUnit,
                ..
            }
        ));
        assert_eq!(state.terms().in_count(), terms_before + 2);
    }

    #[test]
    fn proof_state_process_clause_with_docs_prints_output_level_one_given_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let clause = unit_clause_with_id(state.terms_mut(), "pc_process_doc_l1", 4_152);
        let expected_clause = clause_print_lop_format_string(state.terms(), &clause, true);
        queue_unprocessed_for_process(&mut state, &mut control, clause);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 1, ProblemType::FirstOrder);

        let outcome = proof_state_process_clause_with_docs(
            &mut output,
            &mut session,
            1,
            &mut state,
            &mut control,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed { class, .. } = outcome else {
            panic!("selected clause should be processed");
        };
        assert_eq!(output, format!("%\n%{expected_clause}\n"));
        assert!(match class {
            ProcessedClauseClass::PositiveRule => state.processed_pos_rules().find_by_id(4_152),
            ProcessedClauseClass::PositiveEquation => state.processed_pos_eqns().find_by_id(4_152),
            ProcessedClauseClass::NegativeUnit => state.processed_neg_units().find_by_id(4_152),
            ProcessedClauseClass::NonUnit => state.processed_non_units().find_by_id(4_152),
        }
        .is_some());
    }

    #[test]
    fn proof_state_process_clause_with_output_prints_output_level_one_given_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let clause = unit_clause_with_id(state.terms_mut(), "pc_process_output_l1", 4_153);
        let expected_clause = clause_print_lop_format_string(state.terms(), &clause, true);
        queue_unprocessed_for_process(&mut state, &mut control, clause);
        let mut output = Vec::new();

        let outcome =
            proof_state_process_clause_with_output(&mut output, 1, &mut state, &mut control, 1)
                .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed { class, .. } = outcome else {
            panic!("selected clause should be processed");
        };
        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("%\n%{expected_clause}\n")
        );
        assert!(match class {
            ProcessedClauseClass::PositiveRule => state.processed_pos_rules().find_by_id(4_153),
            ProcessedClauseClass::PositiveEquation => state.processed_pos_eqns().find_by_id(4_153),
            ProcessedClauseClass::NegativeUnit => state.processed_neg_units().find_by_id(4_153),
            ProcessedClauseClass::NonUnit => state.processed_non_units().find_by_id(4_153),
        }
        .is_some());
    }

    #[test]
    fn proof_state_process_clause_with_docs_emits_new_given_quote_at_level_six() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let clause = unit_clause_with_id(state.terms_mut(), "pc_process_doc_quote", 4_153);
        queue_unprocessed_for_process(&mut state, &mut control, clause);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);

        let outcome = proof_state_process_clause_with_docs(
            &mut output,
            &mut session,
            6,
            &mut state,
            &mut control,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed { class, .. } = outcome else {
            panic!("selected clause should be processed");
        };
        assert!(!output.starts_with('%'));
        assert!(output.contains(" : 4153 : 'new_given'\n"));
        assert!(match class {
            ProcessedClauseClass::PositiveRule => state.processed_pos_rules().find_by_id(1),
            ProcessedClauseClass::PositiveEquation => state.processed_pos_eqns().find_by_id(1),
            ProcessedClauseClass::NegativeUnit => state.processed_neg_units().find_by_id(1),
            ProcessedClauseClass::NonUnit => state.processed_non_units().find_by_id(1),
        }
        .is_some());
        assert!(state.processed_pos_rules().find_by_id(4_153).is_none());
        assert!(state.processed_pos_eqns().find_by_id(4_153).is_none());
        assert!(state.processed_neg_units().find_by_id(4_153).is_none());
        assert!(state.processed_non_units().find_by_id(4_153).is_none());
    }

    #[test]
    fn proof_state_process_clause_with_docs_emits_forward_context_sr_modification() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut subsumer, selected) = {
            let terms = state.terms_mut();
            let p_code = unary_predicate_code(terms, "pc_process_context_doc_p");
            let q_code = unary_predicate_code(terms, "pc_process_context_doc_q");
            let arg = typed_const(terms, "pc_process_context_doc_a");
            let p_atom = unary_predicate(terms, p_code, &arg);
            let q_atom = unary_predicate(terms, q_code, &arg);
            let truth = terms.true_term().clone();
            let mut subsumer = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &p_atom, &truth, true),
                literal(terms, &q_atom, &truth, false),
            ]));
            subsumer.set_ident(4_154);
            subsumer.set_prop(CP_IS_PROCESSED | CP_LIMITED_RW);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &p_atom, &truth, true),
                literal(terms, &q_atom, &truth, true),
            ]));
            selected.set_ident(4_155);
            selected.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
            (subsumer, selected)
        };
        clause_subsume_order_sort_lits(&mut subsumer, state.terms());
        subsumer.set_weight(subsumer.standard_weight());
        state.processed_non_units_mut().insert(subsumer);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        control.heuristic_parms_mut().forward_context_sr = true;
        queue_unprocessed_for_process(&mut state, &mut control, selected);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);

        let outcome = proof_state_process_clause_with_docs(
            &mut output,
            &mut session,
            6,
            &mut state,
            &mut control,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed { class, .. } = outcome else {
            panic!("contextually simplified clause should be processed");
        };
        let csr_position = output.find("csr(4155,4154)").unwrap();
        let new_given_position = output.find(" : 1 : 'new_given'\n").unwrap();
        assert!(csr_position < new_given_position);
        assert_eq!(state.statistics().context_sr_count, 1);
        assert_eq!(session.id_source.current_ident(), 4);
        let processed = match class {
            ProcessedClauseClass::PositiveRule => state.processed_pos_rules().find_by_id(2),
            ProcessedClauseClass::PositiveEquation => state.processed_pos_eqns().find_by_id(2),
            ProcessedClauseClass::NegativeUnit => state.processed_neg_units().find_by_id(2),
            ProcessedClauseClass::NonUnit => state.processed_non_units().find_by_id(2),
        }
        .expect("documented survivor should retain the latest proof id");
        assert_eq!(processed.literal_number(), 1);
        assert_eq!(
            processed.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_CONTEXT_SR),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_154, 0)),
            ]
        );
    }

    #[test]
    fn proof_state_process_clause_with_docs_reports_dynamic_ac_activation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (clause, f_code) =
            commutativity_axiom(state.terms_mut(), "pc_process_dynamic_ac_f", 4_162);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, clause);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);

        let outcome = proof_state_process_clause_with_docs(
            &mut output,
            &mut session,
            6,
            &mut state,
            &mut control,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed {
            class,
            ac_activated,
            ..
        } = outcome
        else {
            panic!("selected commutativity axiom should survive processing");
        };
        assert!(ac_activated);
        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
        assert!(output.contains(
            "% pc_process_dynamic_ac_f is commutative\n% AC handling enabled dynamically\n"
        ));
        assert!(output.contains(" : 4162 : 'new_given'\n"));
        let processed_ref = match class {
            ProcessedClauseClass::PositiveRule => state.processed_pos_rules().find_by_id(1),
            ProcessedClauseClass::PositiveEquation => state.processed_pos_eqns().find_by_id(1),
            ProcessedClauseClass::NegativeUnit => state.processed_neg_units().find_by_id(1),
            ProcessedClauseClass::NonUnit => state.processed_non_units().find_by_id(1),
        }
        .map(ClauseDerivationRef::from)
        .expect("processed set owns the renumbered AC parent");
        let signature_ref = state.terms().signature().ac_axioms()[0];
        assert_ne!(signature_ref.generation(), 0);
        assert_eq!(signature_ref, processed_ref);
        assert_eq!(state.ac_axiom_parent_refs(), vec![processed_ref]);
        assert_eq!(
            state
                .proof_clause_by_derivation_ref(signature_ref)
                .map(ClauseDerivationRef::from),
            Some(processed_ref)
        );
    }

    #[test]
    fn proof_state_process_clause_with_global_indices_generates_indexed_paramodulants() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, mut indexed_partner) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_process_idx_pm_source");
            let replacement = typed_const(terms, "pc_process_idx_pm_replacement");
            let rhs = typed_const(terms, "pc_process_idx_pm_rhs");
            let f_source = typed_unary(terms, "pc_process_idx_pm_f", &source);
            let mut selected_lit = literal(terms, &source, &replacement, true);
            let mut partner_lit = literal(terms, &f_source, &rhs, true);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_151);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            (selected, partner)
        };
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut indexed_partner, state.terms(), false);
        let mut control = proof_control_alloc();
        init_fifo_hcb(&mut control, &state, "ProcessClauseIndexedParamodTest");
        control.set_ocb(kbo_ocb(state.terms()));
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_process_clause_with_global_indices(
            &mut state,
            &mut control,
            1,
            &mut indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed { generation, .. } = outcome else {
            panic!("indexed paramodulation should process the selected clause");
        };
        assert_eq!(generation.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.unprocessed().members(), 1);
    }

    #[test]
    fn proof_state_generate_new_clauses_computes_negative_unit_equality_resolution() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -70);
            let constant = typed_const(terms, "pc_generate_eq_res_const");
            let mut literal = literal(terms, &variable, &constant, false);
            literal.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
            clause.set_ident(4_144);
            clause
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().enable_neg_unit_paramod = false;

        let outcome = proof_state_generate_new_clauses(&mut state, &mut control, &clause)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            GenerateNewClausesOutcome {
                equality_factors: 0,
                equality_resolvents: 1,
                disequality_decompositions: 0,
                paramodulants: 0,
            }
        );
        assert_eq!(state.statistics().resolv_count, 1);
        assert_eq!(state.tmp_store().members(), 1);
        assert!(state.tmp_store().iter().next().unwrap().is_empty());
    }

    #[test]
    fn proof_state_generate_new_clauses_with_docs_quotes_equality_resolution() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -74);
            let constant = typed_const(terms, "pc_generate_doc_er_const");
            let mut literal = literal(terms, &variable, &constant, false);
            literal.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
            clause.set_ident(4_146);
            clause
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().enable_neg_unit_paramod = false;
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_generate_new_clauses_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
            &clause,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.equality_resolvents, 1);
        assert_eq!(state.statistics().resolv_count, 1);
        assert!(output.contains(" : er(4146)\n"));
        assert_eq!(state.tmp_store().members(), 1);
        assert_eq!(state.tmp_store().iter().next().unwrap().ident(), 1);
    }

    #[test]
    fn proof_state_generate_new_clauses_with_docs_quotes_equality_factor() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -72);
            let left_remainder = typed_const(terms, "pc_generate_doc_ef_a");
            let right_remainder = typed_const(terms, "pc_generate_doc_ef_c");
            let instance = typed_const(terms, "pc_generate_doc_ef_b");
            let f_of_var = typed_unary(terms, "pc_generate_doc_ef_f", &variable);
            let f_of_instance = typed_unary(terms, "pc_generate_doc_ef_f", &instance);
            let mut first = literal(terms, &f_of_var, &left_remainder, true);
            let second = literal(terms, &f_of_instance, &right_remainder, true);
            first.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
            clause.set_ident(4_145);
            clause
        };
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().enable_eq_factoring = true;
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_generate_new_clauses_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
            &clause,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.equality_factors, 1);
        assert_eq!(state.statistics().factor_count, 1);
        assert!(output.contains(" : ef(4145)\n"));
        assert_eq!(state.tmp_store().members(), 1);
        assert_eq!(state.tmp_store().iter().next().unwrap().ident(), 1);
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_arg_cong_generates_prefix_applications() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (left_code, right_code, existing_var_code, clause) = {
            let terms = state.terms_mut();
            let left = typed_arrow_const(terms, "pc_generate_arg_cong_left", 2);
            let right = typed_arrow_const(terms, "pc_generate_arg_cong_right", 2);
            let existing_var = typed_var(terms, -2);
            let existing_const = typed_const(terms, "pc_generate_arg_cong_existing_const");
            let mut arg_cong_literal = literal(terms, &left, &right, true);
            let residual = literal(terms, &existing_var, &existing_const, true);
            arg_cong_literal.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![arg_cong_literal, residual]));
            clause.set_ident(4_170);
            clause.set_proof_depth(4);
            clause.set_proof_size(9);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (left.f_code(), right.f_code(), existing_var.f_code(), clause)
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::AllLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.statistics().factor_count, 0);
        assert_eq!(state.statistics().resolv_count, 0);
        assert_eq!(state.statistics().paramod_count, 0);
        assert_eq!(state.tmp_store().members(), 2);
        let generated = state.tmp_store().iter().collect::<Vec<_>>();
        let first_literal = &generated[0].literals().as_slice()[0];
        let second_literal = &generated[1].literals().as_slice()[0];
        assert!(first_literal.is_positive());
        assert!(second_literal.is_positive());
        assert_eq!(first_literal.left().f_code(), left_code);
        assert_eq!(first_literal.right().f_code(), right_code);
        assert_eq!(first_literal.left().arity(), 1);
        assert_eq!(first_literal.right().arity(), 1);
        assert_eq!(second_literal.left().f_code(), left_code);
        assert_eq!(second_literal.right().f_code(), right_code);
        assert_eq!(second_literal.left().arity(), 2);
        assert_eq!(second_literal.right().arity(), 2);
        assert_eq!(
            first_literal.left().argument(0),
            first_literal.right().argument(0)
        );
        assert_ne!(
            first_literal.left().argument(0).unwrap().f_code(),
            existing_var_code
        );
        assert_eq!(
            second_literal.left().argument(0),
            second_literal.right().argument(0)
        );
        assert_eq!(
            second_literal.left().argument(1),
            second_literal.right().argument(1)
        );
        assert_eq!(
            first_literal.left().argument(0),
            second_literal.left().argument(0)
        );

        for generated_clause in generated {
            assert_eq!(generated_clause.proof_depth(), 4);
            assert_eq!(generated_clause.proof_size(), 10);
            assert!(generated_clause.query_prop(CP_IS_SOS));
            assert!(!generated_clause.query_prop(CP_NO_GENERATION));
            assert_eq!(
                generated_clause.derivation().unwrap().as_slice(),
                &[
                    DerivationEntry::Operation(DC_ARG_CONG),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_170, 0)),
                ],
            );
        }
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_arg_cong_respects_max_filter() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let maximal_left_code;
        let nonmax_left_code;
        let clause = {
            let terms = state.terms_mut();
            let maximal_left = typed_arrow_const(terms, "pc_generate_arg_cong_max_left", 1);
            let maximal_right = typed_arrow_const(terms, "pc_generate_arg_cong_max_right", 1);
            let nonmax_left = typed_arrow_const(terms, "pc_generate_arg_cong_nonmax_left", 1);
            let nonmax_right = typed_arrow_const(terms, "pc_generate_arg_cong_nonmax_right", 1);
            maximal_left_code = maximal_left.f_code();
            nonmax_left_code = nonmax_left.f_code();
            let mut maximal = literal(terms, &maximal_left, &maximal_right, true);
            let nonmax = literal(terms, &nonmax_left, &nonmax_right, true);
            maximal.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![maximal, nonmax]));
            clause.set_ident(4_171);
            clause.set_prop(CP_NO_GENERATION);
            clause
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::MaxLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 2);
        assert_eq!(
            generated.literals().as_slice()[0].left().f_code(),
            maximal_left_code
        );
        assert_eq!(
            generated.literals().as_slice()[1].left().f_code(),
            nonmax_left_code
        );
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_neg_ext_generates_skolem_applications() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (left_code, right_code, clause) = {
            let terms = state.terms_mut();
            let left = typed_arrow_const(terms, "pc_generate_neg_ext_left", 2);
            let right = typed_arrow_const(terms, "pc_generate_neg_ext_right", 2);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &left, &right, false,
            )]));
            clause.set_ident(4_172);
            clause.set_proof_depth(5);
            clause.set_proof_size(11);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (left.f_code(), right.f_code(), clause)
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().neg_ext = ExtInferenceType::AllLits;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.statistics().neg_ext_count, 2);
        assert_eq!(state.tmp_store().members(), 2);
        let generated = state.tmp_store().iter().collect::<Vec<_>>();
        let first_literal = &generated[0].literals().as_slice()[0];
        let second_literal = &generated[1].literals().as_slice()[0];
        assert!(first_literal.is_negative());
        assert!(second_literal.is_negative());
        assert_eq!(first_literal.left().f_code(), left_code);
        assert_eq!(first_literal.right().f_code(), right_code);
        assert_eq!(first_literal.left().arity(), 1);
        assert_eq!(first_literal.right().arity(), 1);
        assert_eq!(second_literal.left().f_code(), left_code);
        assert_eq!(second_literal.right().f_code(), right_code);
        assert_eq!(second_literal.left().arity(), 2);
        assert_eq!(second_literal.right().arity(), 2);
        assert_eq!(
            first_literal.left().argument(0),
            first_literal.right().argument(0)
        );
        assert_eq!(
            second_literal.left().argument(0),
            second_literal.right().argument(0)
        );
        assert_eq!(
            second_literal.left().argument(1),
            second_literal.right().argument(1)
        );
        assert_ne!(
            first_literal.left().argument(0),
            second_literal.left().argument(1)
        );

        for generated_clause in generated {
            assert_eq!(generated_clause.proof_depth(), 5);
            assert_eq!(generated_clause.proof_size(), 12);
            assert!(generated_clause.query_prop(CP_IS_SOS));
            assert!(!generated_clause.query_prop(CP_NO_GENERATION));
            assert_eq!(
                generated_clause.derivation().unwrap().as_slice(),
                &[
                    DerivationEntry::Operation(DC_NEG_EXT),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_172, 0)),
                ],
            );
        }
    }

    fn assert_generated_pos_ext_clause(generated_clause: &Clause) {
        assert_eq!(generated_clause.proof_depth(), 6);
        assert_eq!(generated_clause.proof_size(), 13);
        assert!(generated_clause.query_prop(CP_IS_SOS));
        assert!(!generated_clause.query_prop(CP_NO_GENERATION));
        assert_eq!(
            generated_clause.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_POS_EXT),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_173, 0)),
            ],
        );
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_pos_ext_generates_prefix_equalities() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (left_code, right_code, left_const_code, right_const_code, shared_var_code, clause) = {
            let terms = state.terms_mut();
            let left_head = typed_arrow_const(terms, "pc_generate_pos_ext_left", 3);
            let right_head = typed_arrow_const(terms, "pc_generate_pos_ext_right", 3);
            let left_const = typed_const(terms, "pc_generate_pos_ext_a");
            let right_const = typed_const(terms, "pc_generate_pos_ext_b");
            let shared_first = typed_var(terms, -2);
            let shared_last = typed_var(terms, -4);

            let applied_left = terms.term_apply_arg(&left_head, &left_const);
            let left_prefix = terms.term_top_insert(applied_left).unwrap();
            let applied_left = terms.term_apply_arg(&left_prefix, &shared_first);
            let left_prefix = terms.term_top_insert(applied_left).unwrap();
            let applied_left = terms.term_apply_arg(&left_prefix, &shared_last);
            let left = terms.term_top_insert(applied_left).unwrap();

            let applied_right = terms.term_apply_arg(&right_head, &right_const);
            let right_prefix = terms.term_top_insert(applied_right).unwrap();
            let applied_right = terms.term_apply_arg(&right_prefix, &shared_first);
            let right_prefix = terms.term_top_insert(applied_right).unwrap();
            let applied_right = terms.term_apply_arg(&right_prefix, &shared_last);
            let right = terms.term_top_insert(applied_right).unwrap();

            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &right, true)]));
            clause.set_ident(4_173);
            clause.set_proof_depth(6);
            clause.set_proof_size(12);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (
                left_head.f_code(),
                right_head.f_code(),
                left_const.f_code(),
                right_const.f_code(),
                shared_first.f_code(),
                clause,
            )
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().neg_ext = ExtInferenceType::AllLits;
        control.heuristic_parms_mut().pos_ext = ExtInferenceType::AllLits;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 2);
        let generated = state.tmp_store().iter().collect::<Vec<_>>();
        let first_literal = &generated[0].literals().as_slice()[0];
        let second_literal = &generated[1].literals().as_slice()[0];
        assert!(first_literal.is_positive());
        assert!(second_literal.is_positive());
        assert_eq!(first_literal.left().f_code(), left_code);
        assert_eq!(first_literal.right().f_code(), right_code);
        assert_eq!(first_literal.left().arity(), 2);
        assert_eq!(first_literal.right().arity(), 2);
        assert_eq!(
            first_literal.left().argument(0).unwrap().f_code(),
            left_const_code
        );
        assert_eq!(
            first_literal.right().argument(0).unwrap().f_code(),
            right_const_code
        );
        assert_eq!(
            first_literal.left().argument(1).unwrap().f_code(),
            shared_var_code
        );
        assert_eq!(
            first_literal.right().argument(1).unwrap().f_code(),
            shared_var_code
        );
        assert_eq!(second_literal.left().f_code(), left_code);
        assert_eq!(second_literal.right().f_code(), right_code);
        assert_eq!(second_literal.left().arity(), 1);
        assert_eq!(second_literal.right().arity(), 1);
        assert_eq!(
            second_literal.left().argument(0).unwrap().f_code(),
            left_const_code
        );
        assert_eq!(
            second_literal.right().argument(0).unwrap().f_code(),
            right_const_code
        );

        for generated_clause in generated {
            assert_generated_pos_ext_clause(generated_clause);
        }
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_pos_ext_alone_preserves_c_noop_gate() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let left_head = typed_arrow_const(terms, "pc_generate_pos_ext_noop_left", 1);
            let right_head = typed_arrow_const(terms, "pc_generate_pos_ext_noop_right", 1);
            let shared = typed_var(terms, -2);
            let applied_left = terms.term_apply_arg(&left_head, &shared);
            let left = terms.term_top_insert(applied_left).unwrap();
            let applied_right = terms.term_apply_arg(&right_head, &shared);
            let right = terms.term_top_insert(applied_right).unwrap();
            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &right, true)]));
            clause.set_prop(CP_NO_GENERATION);
            clause
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().pos_ext = ExtInferenceType::AllLits;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 0);
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_pos_ext_off_suppresses_c_gate() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let left_head = typed_arrow_const(terms, "pc_generate_pos_ext_off_left", 1);
            let right_head = typed_arrow_const(terms, "pc_generate_pos_ext_off_right", 1);
            let shared = typed_var(terms, -2);
            let applied_left = terms.term_apply_arg(&left_head, &shared);
            let left = terms.term_top_insert(applied_left).unwrap();
            let applied_right = terms.term_apply_arg(&right_head, &shared);
            let right = terms.term_top_insert(applied_right).unwrap();
            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &right, true)]));
            clause.set_prop(CP_NO_GENERATION);
            clause
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().neg_ext = ExtInferenceType::AllLits;
        control.heuristic_parms_mut().pos_ext = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.statistics().neg_ext_count, 0);
        assert_eq!(state.tmp_store().members(), 0);
    }

    #[test]
    fn compute_ext_eq_res_generates_disagreement_conditions() {
        let mut bank = test_bank();
        let (p_code, q_code, clause) = {
            let head = predicate_argument_binary_const(&mut bank, "pc_ext_eqres_h");
            let p = unary_predicate_const(&mut bank, "pc_ext_eqres_p");
            let q = unary_predicate_const(&mut bank, "pc_ext_eqres_q");
            let arg = typed_const(&mut bank, "pc_ext_eqres_a");
            let left = apply_terms(&mut bank, &head, &[p.clone(), arg.clone()]).unwrap();
            let right = apply_terms(&mut bank, &head, &[q.clone(), arg]).unwrap();
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                &mut bank, &left, &right, false,
            )]));
            clause.set_ident(4_183);
            clause.set_proof_depth(2);
            clause.set_proof_size(5);
            clause.set_tptp_type(CP_TYPE_AXIOM);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (p.f_code(), q.f_code(), clause)
        };
        let mut store = ClauseSet::new();

        assert_eq!(
            compute_ext_eq_res(&mut bank, &clause, &mut store, 2).unwrap(),
            1
        );

        let generated = store.iter().next().expect("ExtEqRes should generate");
        assert_eq!(generated.literal_number(), 1);
        assert_eq!(generated.negative_literal_count(), 1);
        assert_eq!(generated.proof_depth(), 3);
        assert_eq!(generated.proof_size(), 6);
        assert_eq!(generated.query_tptp_type(), CP_TYPE_AXIOM);
        assert!(generated.query_prop(CP_IS_SOS));
        assert!(!generated.query_prop(CP_NO_GENERATION));
        assert!(derivation_contains_operation(generated, DC_EXT_EQ_RES));
        assert!(derivation_contains_parent(generated, 4_183));

        let condition = &generated.literals().as_slice()[0];
        assert!(condition.is_negative());
        assert_eq!(condition.left().f_code(), q_code);
        assert_eq!(condition.right().f_code(), p_code);

        assert_eq!(
            compute_ext_eq_res(&mut bank, &clause, &mut ClauseSet::new(), 1).unwrap(),
            0
        );
    }

    #[test]
    fn compute_ext_eq_fact_generates_partner_condition_and_keeps_partner_literal() {
        let mut bank = test_bank();
        let (p_code, q_code, u_code, v_code, clause) = {
            let head = predicate_argument_binary_const(&mut bank, "pc_ext_eqfact_h");
            let p = unary_predicate_const(&mut bank, "pc_ext_eqfact_p");
            let q = unary_predicate_const(&mut bank, "pc_ext_eqfact_q");
            let arg = typed_const(&mut bank, "pc_ext_eqfact_a");
            let u = typed_const(&mut bank, "pc_ext_eqfact_u");
            let v = typed_const(&mut bank, "pc_ext_eqfact_v");
            let left = apply_terms(&mut bank, &head, &[p.clone(), arg.clone()]).unwrap();
            let right = apply_terms(&mut bank, &head, &[q.clone(), arg]).unwrap();
            let first = literal(&mut bank, &left, &u, true);
            let second = literal(&mut bank, &right, &v, true);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![first, second]));
            clause.set_ident(4_184);
            clause.set_proof_depth(4);
            clause.set_proof_size(9);
            clause.set_tptp_type(CP_TYPE_CONJECTURE);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (p.f_code(), q.f_code(), u.f_code(), v.f_code(), clause)
        };
        let mut store = ClauseSet::new();

        assert_eq!(
            compute_ext_eq_fact(&mut bank, &clause, &mut store, 4).unwrap(),
            1
        );

        let generated = store.iter().next().expect("ExtEqFact should generate");
        assert_eq!(generated.literal_number(), 3);
        assert_eq!(generated.positive_literal_count(), 1);
        assert_eq!(generated.negative_literal_count(), 2);
        assert_eq!(generated.proof_depth(), 5);
        assert_eq!(generated.proof_size(), 10);
        assert_ne!(generated.query_tptp_type(), CP_TYPE_CONJECTURE);
        assert!(generated.query_prop(CP_IS_SOS));
        assert!(!generated.query_prop(CP_NO_GENERATION));
        assert!(derivation_contains_operation(generated, DC_EXT_EQ_FACT));
        assert!(derivation_contains_parent(generated, 4_184));

        let negative_literals = generated
            .literals()
            .as_slice()
            .iter()
            .filter(|literal| literal.is_negative())
            .collect::<Vec<_>>();
        assert!(
            negative_literals
                .iter()
                .any(|literal| literal.left().f_code() == q_code
                    && literal.right().f_code() == p_code)
        );
        assert!(
            negative_literals
                .iter()
                .any(|literal| literal.left().f_code() == u_code
                    && literal.right().f_code() == v_code)
        );
        let positive = generated
            .literals()
            .as_slice()
            .iter()
            .find(|literal| literal.is_positive())
            .expect("partner literal should remain");
        assert_eq!(positive.right().f_code(), v_code);

        assert_eq!(
            compute_ext_eq_fact(&mut bank, &clause, &mut ClauseSet::new(), 3).unwrap(),
            0
        );
    }

    #[test]
    fn compute_ext_sup_generates_indexed_condition_and_replacement_literal() {
        let mut bank = test_bank();
        let (p_code, q_code, u_code, v_code, selected, mut partner) = {
            let head = predicate_argument_binary_const(&mut bank, "pc_ext_sup_h");
            let p = unary_predicate_const(&mut bank, "pc_ext_sup_p");
            let q = unary_predicate_const(&mut bank, "pc_ext_sup_q");
            let arg = typed_const(&mut bank, "pc_ext_sup_a");
            let u = typed_const(&mut bank, "pc_ext_sup_u");
            let v = typed_const(&mut bank, "pc_ext_sup_v");
            let selected_left = apply_terms(&mut bank, &head, &[p.clone(), arg.clone()]).unwrap();
            let partner_left = apply_terms(&mut bank, &head, &[q.clone(), arg]).unwrap();

            let mut selected = Clause::alloc(EqnList::from_vec(vec![literal(
                &mut bank,
                &selected_left,
                &u,
                true,
            )]));
            selected.set_ident(4_186);
            selected.set_proof_depth(3);
            selected.set_proof_size(7);
            selected.set_tptp_type(CP_TYPE_CONJECTURE);
            selected.set_prop(CP_IS_SOS | CP_NO_GENERATION);

            let mut partner = Clause::alloc(EqnList::from_vec(vec![literal(
                &mut bank,
                &partner_left,
                &v,
                true,
            )]));
            partner.set_ident(4_185);
            partner.set_proof_depth(2);
            partner.set_proof_size(5);
            partner.set_tptp_type(CP_TYPE_AXIOM);

            (
                p.f_code(),
                q.f_code(),
                u.f_code(),
                v.f_code(),
                selected,
                partner,
            )
        };
        let renamed = selected.copy_disjoint(&mut bank).unwrap();
        let mut indices = GlobalIndices::new_for_problem(
            "NoIndex",
            "NoIndex",
            "NoIndex",
            3,
            ProblemType::HigherOrder,
        );
        indices.insert_clause(&mut partner, &bank, false);
        let mut store = ClauseSet::new();

        assert_eq!(
            compute_ext_sup(&mut bank, &renamed, &selected, &mut store, &indices, 3).unwrap(),
            1
        );

        let generated = store.iter().next().expect("ExtSup should generate");
        assert_eq!(generated.literal_number(), 2);
        assert_eq!(generated.positive_literal_count(), 1);
        assert_eq!(generated.negative_literal_count(), 1);
        assert_eq!(generated.proof_depth(), 4);
        assert_eq!(generated.proof_size(), 13);
        assert_ne!(generated.query_tptp_type(), CP_TYPE_CONJECTURE);
        assert!(generated.query_prop(CP_IS_SOS));
        assert!(!generated.query_prop(CP_NO_GENERATION));
        assert!(derivation_contains_operation(generated, DC_EXT_SUP));
        assert!(derivation_contains_parent(generated, 4_185));
        assert!(derivation_contains_parent(generated, 4_186));

        let positive = generated
            .literals()
            .as_slice()
            .iter()
            .find(|literal| literal.is_positive())
            .expect("ExtSup should keep replacement literal");
        assert_eq!(positive.left().f_code(), u_code);
        assert_eq!(positive.right().f_code(), v_code);

        let condition = generated
            .literals()
            .as_slice()
            .iter()
            .find(|literal| literal.is_negative())
            .expect("ExtSup should emit disagreement condition");
        assert_eq!(condition.left().f_code(), q_code);
        assert_eq!(condition.right().f_code(), p_code);

        assert_eq!(
            compute_ext_sup(
                &mut bank,
                &renamed,
                &selected,
                &mut ClauseSet::new(),
                &indices,
                2,
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn proof_state_generate_new_clauses_with_global_indices_runs_ext_rules() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, mut partner, u_code, v_code) = {
            let terms = state.terms_mut();
            let head = predicate_argument_binary_const(terms, "pc_generate_ext_sup_h");
            let p = unary_predicate_const(terms, "pc_generate_ext_sup_p");
            let q = unary_predicate_const(terms, "pc_generate_ext_sup_q");
            let arg = typed_const(terms, "pc_generate_ext_sup_a");
            let u = typed_const(terms, "pc_generate_ext_sup_u");
            let v = typed_const(terms, "pc_generate_ext_sup_v");
            let selected_left = apply_terms(terms, &head, &[p, arg.clone()]).unwrap();
            let partner_left = apply_terms(terms, &head, &[q, arg]).unwrap();

            let mut selected = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &selected_left,
                &u,
                true,
            )]));
            selected.set_ident(4_188);
            selected.set_proof_depth(1);
            selected.set_proof_size(2);

            let mut partner = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &partner_left,
                &v,
                true,
            )]));
            partner.set_ident(4_187);
            partner.set_proof_depth(1);
            partner.set_proof_size(3);
            (selected, partner, u.f_code(), v.f_code())
        };
        let mut indices = GlobalIndices::new_for_problem(
            "NoIndex",
            "NoIndex",
            "NoIndex",
            4,
            ProblemType::HigherOrder,
        );
        indices.insert_clause(&mut partner, state.terms(), false);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().ext_rules_max_depth = 4;

        let outcome = proof_state_generate_new_clauses_with_global_indices(
            &mut state,
            &mut control,
            &selected,
            &indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert!(derivation_contains_operation(generated, DC_EXT_SUP));
        assert!(generated
            .literals()
            .as_slice()
            .iter()
            .any(|literal| literal.is_positive()
                && literal.left().f_code() == u_code
                && literal.right().f_code() == v_code));
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_leibniz_generates_equality_instances() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (predicate_var, a_code, b_code, clause) = {
            let terms = state.terms_mut();
            let predicate_var = unary_predicate_var(terms, -8);
            let a = typed_const(terms, "pc_generate_leibniz_a");
            let b = typed_const(terms, "pc_generate_leibniz_b");
            let applied_a = apply_terms(terms, &predicate_var, std::slice::from_ref(&a)).unwrap();
            let applied_b = apply_terms(terms, &predicate_var, std::slice::from_ref(&b)).unwrap();
            let true_term = terms.true_term().clone();
            let pos = Eqn::alloc(applied_a, true_term.clone(), terms, true).unwrap();
            let neg = Eqn::alloc(applied_b, true_term, terms, false).unwrap();
            let mut clause = Clause::alloc(EqnList::from_vec(vec![pos, neg]));
            clause.set_ident(4_174);
            clause.set_proof_depth(2);
            clause.set_proof_size(5);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (predicate_var, a.f_code(), b.f_code(), clause)
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().elim_leibniz_max_depth = 2;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 2);
        assert!(predicate_var.binding().is_none());
        let generated = state.tmp_store().iter().collect::<Vec<_>>();
        let first_literal = &generated[0].literals().as_slice()[0];
        let second_literal = &generated[1].literals().as_slice()[0];
        assert!(first_literal.is_positive());
        assert!(second_literal.is_positive());
        assert_eq!(first_literal.left().f_code(), b_code);
        assert_eq!(first_literal.right().f_code(), a_code);
        assert_eq!(second_literal.left().f_code(), a_code);
        assert_eq!(second_literal.right().f_code(), b_code);

        for generated_clause in generated {
            assert_eq!(generated_clause.proof_depth(), 3);
            assert_eq!(generated_clause.proof_size(), 6);
            assert!(generated_clause.query_prop(CP_IS_SOS));
            assert!(!generated_clause.query_prop(CP_NO_GENERATION));
            assert_eq!(
                generated_clause.derivation().unwrap().as_slice(),
                &[
                    DerivationEntry::Operation(DC_NORMALIZE),
                    DerivationEntry::Operation(DC_LEIBNIZ_ELIM),
                    DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_174, 0)),
                ],
            );
        }
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_leibniz_respects_depth_limit() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let predicate_var = unary_predicate_var(terms, -10);
            let a = typed_const(terms, "pc_generate_leibniz_depth_a");
            let b = typed_const(terms, "pc_generate_leibniz_depth_b");
            let applied_a = apply_terms(terms, &predicate_var, std::slice::from_ref(&a)).unwrap();
            let applied_b = apply_terms(terms, &predicate_var, std::slice::from_ref(&b)).unwrap();
            let true_term = terms.true_term().clone();
            let pos = Eqn::alloc(applied_a, true_term.clone(), terms, true).unwrap();
            let neg = Eqn::alloc(applied_b, true_term, terms, false).unwrap();
            let mut clause = Clause::alloc(EqnList::from_vec(vec![pos, neg]));
            clause.set_proof_depth(3);
            clause.set_prop(CP_NO_GENERATION);
            clause
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().elim_leibniz_max_depth = 2;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 0);
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_inverse_recognizes_definition() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (f_code, x_code, z_code, clause) = {
            let terms = state.terms_mut();
            let f_code = typed_binary_code(terms, "pc_generate_inverse_f");
            let x = typed_var(terms, -18);
            let y = typed_var(terms, -20);
            let z = typed_var(terms, -22);
            let left = typed_binary_with_code(terms, f_code, &x, &z);
            let right = typed_binary_with_code(terms, f_code, &y, &z);
            let negative = literal(terms, &left, &right, false);
            let positive = literal(terms, &x, &y, true);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![negative, positive]));
            clause.set_ident(4_178);
            clause.set_proof_depth(4);
            clause.set_proof_size(7);
            clause.set_tptp_type(CP_TYPE_AXIOM);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (f_code, x.f_code(), z.f_code(), clause)
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().inverse_recognition = true;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.positive_literal_count(), 1);
        assert_eq!(generated.negative_literal_count(), 0);
        assert_eq!(generated.proof_depth(), 5);
        assert_eq!(generated.proof_size(), 8);
        assert_eq!(generated.query_tptp_type(), CP_TYPE_AXIOM);
        assert!(generated.query_prop(CP_IS_PURE_INJECTIVITY));
        assert!(generated.query_prop(CP_IS_SOS));
        assert!(!generated.query_prop(CP_NO_GENERATION));
        assert_eq!(
            generated.derivation().unwrap().as_slice(),
            &[
                DerivationEntry::Operation(DC_INV_REC),
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(4_178, 0)),
            ],
        );

        let inverse_literal = &generated.literals().as_slice()[0];
        assert!(inverse_literal.is_positive());
        assert_eq!(inverse_literal.right().f_code(), x_code);
        let inverse_term = inverse_literal.left();
        assert_eq!(inverse_term.arity(), 2);
        assert_eq!(
            inverse_term.argument(0).map(|term| term.f_code()),
            Some(z_code)
        );
        assert_eq!(
            inverse_term
                .argument(1)
                .and_then(|argument| argument.argument(0))
                .map(|term| term.f_code()),
            Some(x_code),
        );
        assert_eq!(
            inverse_term.argument(1).map(|term| term.f_code()),
            Some(f_code)
        );
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_inverse_ignores_non_definition() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let x = typed_var(terms, -24);
            let y = typed_var(terms, -26);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(terms, &x, &y, true)]));
            clause.set_ident(4_179);
            clause.set_prop(CP_NO_GENERATION);
            clause
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().inverse_recognition = true;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 0);
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_choice_instantiates_defined_trigger() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (choice_definition, choice_code, clause) = {
            let terms = state.terms_mut();
            let (choice_definition, choice_code) =
                choice_axiom(terms, "pc_generate_choice_defined", -40, -42);
            let choice = terms.create_const_term(choice_code).unwrap();
            let predicate = unary_predicate_const(terms, "pc_generate_choice_pred");
            let choice_application =
                apply_terms(terms, &choice, std::slice::from_ref(&predicate)).unwrap();
            let witness = typed_const(terms, "pc_generate_choice_witness");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &choice_application,
                &witness,
                true,
            )]));
            clause.set_ident(4_181);
            clause.set_proof_depth(1);
            clause.set_proof_size(4);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (choice_definition, choice_code, clause)
        };
        state.axioms_mut().insert(choice_definition);
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().inst_choice_max_depth = 1;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        assert_eq!(
            proof_state_recognize_choice_axioms(&mut state, &control).unwrap(),
            1
        );
        let choice_parent = state
            .choice_opcodes()
            .get(&choice_code)
            .map(Clause::ident)
            .expect("choice definition should be recorded");
        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 2);
        for generated_clause in state.tmp_store().iter() {
            assert_eq!(generated_clause.proof_depth(), 2);
            assert!(generated_clause.query_prop(CP_IS_SOS));
            assert!(!generated_clause.query_prop(CP_NO_GENERATION));
            assert!(derivation_contains_operation(
                generated_clause,
                DC_CHOICE_INST
            ));
            assert!(derivation_contains_parent(generated_clause, 4_181));
            assert!(derivation_contains_parent(generated_clause, choice_parent));
        }
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_choice_variable_creates_choice_axiom() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (existing_witness_var_code, clause) = {
            let terms = state.terms_mut();
            let arg_type = terms.signature().type_bank().default_type();
            let bool_type = terms.signature().type_bank().bool_type();
            let predicate_type = terms
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![arg_type.clone(), bool_type]));
            let choice_type = terms
                .signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![predicate_type, arg_type.clone()]));
            let choice_variable = terms.vars().var_assert_alloc(-44, &choice_type);
            let existing_witness_var = terms.vars().var_assert_alloc(-46, &arg_type);
            let predicate = unary_predicate_const(terms, "pc_generate_choice_var_pred");
            let choice_application =
                apply_terms(terms, &choice_variable, std::slice::from_ref(&predicate)).unwrap();
            let witness = typed_const(terms, "pc_generate_choice_var_witness");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms,
                &choice_application,
                &witness,
                true,
            )]));
            clause.set_ident(4_182);
            clause.set_proof_depth(0);
            clause.set_proof_size(3);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (existing_witness_var.f_code(), clause)
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().inst_choice_max_depth = 0;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.archive().members(), 1);
        assert_eq!(state.choice_opcodes().len(), 1);
        let choice_parent = state
            .choice_opcodes()
            .values()
            .next()
            .map(Clause::ident)
            .expect("fresh choice axiom should be recorded");
        assert!(state.archive().find_by_id(choice_parent).is_some());
        let choice_axiom = state.choice_opcodes().values().next().unwrap();
        assert!(derivation_contains_operation(choice_axiom, DC_CHOICE_AX));
        let mut choice_axiom_vars = BTreeMap::new();
        let _ = choice_axiom.collect_variables(&mut choice_axiom_vars);
        assert!(
            choice_axiom_vars
                .values()
                .all(|var| var.f_code() != existing_witness_var_code),
            "fresh choice axiom variables must not reuse existing input variables"
        );
        assert_eq!(state.tmp_store().members(), 2);
        for generated_clause in state.tmp_store().iter() {
            assert_eq!(generated_clause.proof_depth(), 1);
            assert!(generated_clause.query_prop(CP_IS_SOS));
            assert!(!generated_clause.query_prop(CP_NO_GENERATION));
            assert!(derivation_contains_operation(
                generated_clause,
                DC_CHOICE_INST
            ));
            assert!(derivation_contains_parent(generated_clause, 4_182));
            assert!(derivation_contains_parent(generated_clause, choice_parent));
        }
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_primitive_enum_neg_generates_instances() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (predicate_var, a_code, clause) = {
            let terms = state.terms_mut();
            let predicate_var = unary_predicate_var(terms, -12);
            let a = typed_const(terms, "pc_generate_prim_enum_a");
            let atom = apply_terms(terms, &predicate_var, std::slice::from_ref(&a)).unwrap();
            let true_term = terms.true_term().clone();
            let literal = Eqn::alloc(atom, true_term, terms, true).unwrap();
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
            clause.set_ident(4_176);
            clause.set_proof_depth(1);
            clause.set_proof_size(4);
            clause.set_prop(CP_IS_SOS | CP_NO_GENERATION);
            (predicate_var, a.f_code(), clause)
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().prim_enum_mode = PrimEnumMode::Neg;
        control.heuristic_parms_mut().prim_enum_max_depth = 1;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 3);
        assert!(predicate_var.binding().is_none());
        let generated = state.tmp_store().iter().collect::<Vec<_>>();
        for generated_clause in &generated {
            assert_eq!(generated_clause.proof_depth(), 2);
            assert_eq!(generated_clause.proof_size(), 5);
            assert!(generated_clause.query_prop(CP_IS_SOS));
            assert!(!generated_clause.query_prop(CP_NO_GENERATION));
            assert!(derivation_contains_operation(
                generated_clause,
                DC_PRIM_ENUM
            ));
            assert!(derivation_contains_parent(generated_clause, 4_176));
        }

        let fresh_literal = generated
            .iter()
            .flat_map(|generated_clause| generated_clause.literals().as_slice())
            .find(|literal| {
                literal.left().is_applied_free_var()
                    && literal
                        .left()
                        .argument(1)
                        .is_some_and(|argument| argument.f_code() == a_code)
            })
            .unwrap_or_else(|| panic!("primitive enumeration should create a fresh pattern"));
        assert!(fresh_literal.is_negative());
        let fresh_head = fresh_literal
            .left()
            .argument(0)
            .expect("fresh pattern application must have a head");
        assert!(fresh_head.is_free_var());
        assert_ne!(fresh_head.f_code(), predicate_var.f_code());
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_primitive_enum_processes_head_once() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let predicate_var = unary_predicate_var(terms, -14);
            let a = typed_const(terms, "pc_generate_prim_enum_once_a");
            let b = typed_const(terms, "pc_generate_prim_enum_once_b");
            let first_atom = apply_terms(terms, &predicate_var, std::slice::from_ref(&a)).unwrap();
            let second_atom = apply_terms(terms, &predicate_var, std::slice::from_ref(&b)).unwrap();
            let true_term = terms.true_term().clone();
            let first_literal = Eqn::alloc(first_atom, true_term.clone(), terms, true).unwrap();
            let second_literal = Eqn::alloc(second_atom, true_term, terms, true).unwrap();
            let mut clause = Clause::alloc(EqnList::from_vec(vec![first_literal, second_literal]));
            clause.set_ident(4_177);
            clause.set_prop(CP_NO_GENERATION);
            clause
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().prim_enum_mode = PrimEnumMode::Eq;
        control.heuristic_parms_mut().prim_enum_max_depth = 0;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 3);
        assert!(state
            .tmp_store()
            .iter()
            .all(|generated_clause| derivation_contains_operation(generated_clause, DC_PRIM_ENUM)));
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_primitive_enum_respects_depth_limit() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let clause = {
            let terms = state.terms_mut();
            let predicate_var = unary_predicate_var(terms, -16);
            let a = typed_const(terms, "pc_generate_prim_enum_depth_a");
            let atom = apply_terms(terms, &predicate_var, std::slice::from_ref(&a)).unwrap();
            let true_term = terms.true_term().clone();
            let literal = Eqn::alloc(atom, true_term, terms, true).unwrap();
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
            clause.set_proof_depth(2);
            clause.set_prop(CP_NO_GENERATION);
            clause
        };
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().prim_enum_mode = PrimEnumMode::Neg;
        control.heuristic_parms_mut().prim_enum_max_depth = 1;
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses_impl::<String>(
            &mut state,
            &mut control,
            &clause,
            ProblemType::HigherOrder,
            None,
            None,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome, GenerateNewClausesOutcome::default());
        assert_eq!(state.tmp_store().members(), 0);
    }

    #[test]
    fn proof_state_generate_new_clauses_computes_plain_paramodulation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, partner, replacement, rhs) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_generate_pm_source");
            let replacement = typed_const(terms, "pc_generate_pm_replacement");
            let rhs = typed_const(terms, "pc_generate_pm_rhs");
            let f_of_source = typed_unary(terms, "pc_generate_pm_f", &source);
            let mut partner_lit = literal(terms, &source, &replacement, true);
            let mut selected_lit = literal(terms, &f_of_source, &rhs, true);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_146);
            (selected, partner, replacement, rhs)
        };
        state.processed_pos_eqns_mut().insert(partner);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));

        let outcome = proof_state_generate_new_clauses(&mut state, &mut control, &selected)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 1);
        assert_eq!(generated.literals().as_slice()[0].right(), &rhs);
        assert!(generated.literals().as_slice()[0]
            .left()
            .argument(0)
            .is_some_and(|arg| arg == replacement));
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_first_order_subset_paramodulates() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, partner, replacement, rhs) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_generate_ho_pm_source");
            let replacement = typed_const(terms, "pc_generate_ho_pm_replacement");
            let rhs = typed_const(terms, "pc_generate_ho_pm_rhs");
            let f_of_source = typed_unary(terms, "pc_generate_ho_pm_f", &source);
            let mut partner_lit = literal(terms, &source, &replacement, true);
            let mut selected_lit = literal(terms, &f_of_source, &rhs, true);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_153);
            (selected, partner, replacement, rhs)
        };
        state.processed_pos_eqns_mut().insert(partner);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo6_ocb(state.terms()));
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;

        let outcome = proof_state_generate_new_clauses(&mut state, &mut control, &selected)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 1);
        assert_eq!(generated.literals().as_slice()[0].right(), &rhs);
        assert!(generated.literals().as_slice()[0]
            .left()
            .argument(0)
            .is_some_and(|arg| arg == replacement));
    }

    #[test]
    fn proof_state_generate_new_clauses_higher_order_carries_unrelated_applied_var_literal() {
        let _guard = global_state_lock();
        let _problem_type = set_problem_type_for_test(ProblemType::HigherOrder);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, partner, replacement, rhs, truth) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_generate_ho_pm_diag_source");
            let replacement = typed_const(terms, "pc_generate_ho_pm_diag_replacement");
            let rhs = typed_const(terms, "pc_generate_ho_pm_diag_rhs");
            let f_of_source = typed_unary(terms, "pc_generate_ho_pm_diag_f", &source);
            let predicate = unary_predicate_var(terms, -4_230);
            let arg = typed_const(terms, "pc_generate_ho_pm_diag_arg");
            let applied = apply_terms(terms, &predicate, std::slice::from_ref(&arg))
                .unwrap_or_else(|err| panic!("{err}"));
            let truth = terms.true_term().clone();

            let mut partner_lit = literal(terms, &source, &replacement, true);
            let mut selected_lit = literal(terms, &f_of_source, &rhs, true);
            let applied_lit = literal(terms, &applied, &truth, true);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit, applied_lit]));
            selected.set_ident(4_154);
            (selected, partner, replacement, rhs, truth)
        };
        state.processed_pos_eqns_mut().insert(partner);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo6_ocb(state.terms()));
        control.heuristic_parms_mut().arg_cong = ExtInferenceType::NoLits;
        control.heuristic_parms_mut().enable_eq_factoring = false;

        let outcome = proof_state_generate_new_clauses(&mut state, &mut control, &selected)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 2);
        assert!(generated.literals().as_slice().iter().any(|literal| {
            literal.right() == &rhs
                && literal
                    .left()
                    .argument(0)
                    .is_some_and(|arg| arg == replacement)
        }));
        assert!(generated
            .literals()
            .as_slice()
            .iter()
            .any(|literal| { literal.right() == &truth && literal.left().is_applied_free_var() }));
    }

    #[test]
    fn proof_state_generate_new_clauses_with_docs_quotes_paramodulation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, partner) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_generate_doc_pm_source");
            let replacement = typed_const(terms, "pc_generate_doc_pm_replacement");
            let rhs = typed_const(terms, "pc_generate_doc_pm_rhs");
            let f_of_source = typed_unary(terms, "pc_generate_doc_pm_f", &source);
            let mut partner_lit = literal(terms, &source, &replacement, true);
            let mut selected_lit = literal(terms, &f_of_source, &rhs, true);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let mut partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            partner.set_ident(4_147);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_146);
            (selected, partner)
        };
        state.processed_pos_eqns_mut().insert(partner);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_generate_new_clauses_with_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
            &selected,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert!(output.contains(" : pm(4146,4147)\n"));
        assert_eq!(state.tmp_store().members(), 1);
        assert_eq!(state.tmp_store().iter().next().unwrap().ident(), 1);
    }

    #[test]
    fn proof_state_generate_new_clauses_paramodulates_predicate_fact_into_rule() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected_fact, processed_rule, mortal_socrates, truth) = {
            let terms = state.terms_mut();
            let socrates = typed_const(terms, "pc_pm_socrates");
            let x = typed_var(terms, -2);
            let human_code = unary_predicate_code(terms, "pc_pm_human");
            let mortal_code = unary_predicate_code(terms, "pc_pm_mortal");
            let human_socrates = unary_predicate(terms, human_code, &socrates);
            let human_x = unary_predicate(terms, human_code, &x);
            let mortal_x = unary_predicate(terms, mortal_code, &x);
            let mortal_socrates = unary_predicate(terms, mortal_code, &socrates);
            let truth = terms.true_term().clone();

            let mut fact_lit = literal(terms, &human_socrates, &truth, true);
            fact_lit.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let selected_fact = Clause::alloc(EqnList::from_vec(vec![fact_lit]));

            let mut rule_head = literal(terms, &mortal_x, &truth, true);
            let mut rule_tail = literal(terms, &human_x, &truth, false);
            rule_head.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            rule_tail.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let processed_rule = Clause::alloc(EqnList::from_vec(vec![rule_head, rule_tail]));

            (selected_fact, processed_rule, mortal_socrates, truth)
        };
        state.processed_non_units_mut().insert(processed_rule);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));

        let outcome = proof_state_generate_new_clauses(&mut state, &mut control, &selected_fact)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 1);
        let literal = &generated.literals().as_slice()[0];
        assert!(literal.is_positive());
        assert_eq!(literal.left(), &mortal_socrates);
        assert_eq!(literal.right(), &truth);
    }

    #[test]
    fn proof_state_generate_new_clauses_with_global_indices_uses_indexed_paramodulation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, mut indexed_partner, replacement, rhs) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_generate_idx_pm_source");
            let replacement = typed_const(terms, "pc_generate_idx_pm_replacement");
            let rhs = typed_const(terms, "pc_generate_idx_pm_rhs");
            let f_source = typed_unary(terms, "pc_generate_idx_pm_f", &source);
            let mut selected_lit = literal(terms, &source, &replacement, true);
            let mut partner_lit = literal(terms, &f_source, &rhs, true);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_150);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            (selected, partner, replacement, rhs)
        };
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut indexed_partner, state.terms(), false);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));

        let outcome = proof_state_generate_new_clauses_with_global_indices(
            &mut state,
            &mut control,
            &selected,
            &indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 1);
        let literal = &generated.literals().as_slice()[0];
        assert_eq!(literal.right(), &rhs);
        assert!(literal
            .left()
            .argument(0)
            .is_some_and(|arg| arg == replacement));
    }

    #[test]
    fn proof_state_generate_new_clauses_with_global_indices_and_docs_quotes_paramodulation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, mut indexed_partner) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_generate_idx_doc_pm_source");
            let replacement = typed_const(terms, "pc_generate_idx_doc_pm_replacement");
            let rhs = typed_const(terms, "pc_generate_idx_doc_pm_rhs");
            let f_source = typed_unary(terms, "pc_generate_idx_doc_pm_f", &source);
            let mut selected_lit = literal(terms, &source, &replacement, true);
            let mut partner_lit = literal(terms, &f_source, &rhs, true);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_151);
            let mut partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            partner.set_ident(4_152);
            (selected, partner)
        };
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut indexed_partner, state.terms(), false);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_generate_new_clauses_with_global_indices_and_docs(
            &mut output,
            &mut session,
            &mut state,
            &mut control,
            &selected,
            &indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert!(output.contains(" : pm(4152,4151)\n"));
        assert_eq!(state.tmp_store().members(), 1);
        assert_eq!(state.tmp_store().iter().next().unwrap().ident(), 1);
    }

    #[test]
    fn proof_state_generate_new_clauses_computes_simultaneous_paramodulation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, partner, f_replacement, g_replacement) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_generate_sim_source");
            let replacement = typed_const(terms, "pc_generate_sim_replacement");
            let f_source = typed_unary(terms, "pc_generate_sim_f", &source);
            let g_source = typed_unary(terms, "pc_generate_sim_g", &source);
            let f_replacement = typed_unary(terms, "pc_generate_sim_f", &replacement);
            let g_replacement = typed_unary(terms, "pc_generate_sim_g", &replacement);
            let mut partner_lit = literal(terms, &source, &replacement, true);
            let mut selected_lit = literal(terms, &f_source, &g_source, true);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_147);
            (selected, partner, f_replacement, g_replacement)
        };
        state.processed_pos_eqns_mut().insert(partner);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().pm_type = HcbParamodulationType::Sim;

        let outcome = proof_state_generate_new_clauses(&mut state, &mut control, &selected)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 1);
        let literal = &generated.literals().as_slice()[0];
        assert_eq!(literal.left(), &f_replacement);
        assert_eq!(literal.right(), &g_replacement);
    }

    #[test]
    fn proof_state_generate_new_clauses_computes_size_decreasing_simultaneous_paramodulation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, partner, h_replacement, k_replacement) = {
            let terms = state.terms_mut();
            let source_arg = typed_const(terms, "pc_generate_size_sim_source_arg");
            let replacement = typed_const(terms, "pc_generate_size_sim_replacement");
            let f_source = typed_unary(terms, "pc_generate_size_sim_f", &source_arg);
            let h_source = typed_unary(terms, "pc_generate_size_sim_h", &f_source);
            let k_source = typed_unary(terms, "pc_generate_size_sim_k", &f_source);
            let h_replacement = typed_unary(terms, "pc_generate_size_sim_h", &replacement);
            let k_replacement = typed_unary(terms, "pc_generate_size_sim_k", &replacement);
            let mut partner_lit = literal(terms, &f_source, &replacement, true);
            let mut selected_lit = literal(terms, &h_source, &k_source, true);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_148);
            (selected, partner, h_replacement, k_replacement)
        };
        state.processed_pos_eqns_mut().insert(partner);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().pm_type = HcbParamodulationType::SizeDecreasingSim;

        let outcome = proof_state_generate_new_clauses(&mut state, &mut control, &selected)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 1);
        let literal = &generated.literals().as_slice()[0];
        assert_eq!(literal.left(), &h_replacement);
        assert_eq!(literal.right(), &k_replacement);
    }

    #[test]
    fn proof_state_generate_new_clauses_computes_super_simultaneous_paramodulation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, partner, h_replacement, k_replacement) = {
            let terms = state.terms_mut();
            let source_arg = typed_const(terms, "pc_generate_super_source_arg");
            let replacement = typed_const(terms, "pc_generate_super_replacement");
            let variable = typed_var(terms, -72);
            let f_source = typed_unary(terms, "pc_generate_super_f", &source_arg);
            let f_variable = typed_unary(terms, "pc_generate_super_f", &variable);
            let h_variable = typed_unary(terms, "pc_generate_super_h", &f_variable);
            let k_source = typed_unary(terms, "pc_generate_super_k", &f_source);
            let h_replacement = typed_unary(terms, "pc_generate_super_h", &replacement);
            let k_replacement = typed_unary(terms, "pc_generate_super_k", &replacement);
            let mut partner_lit = literal(terms, &f_source, &replacement, true);
            let mut selected_lit = literal(terms, &h_variable, &k_source, true);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_149);
            (selected, partner, h_replacement, k_replacement)
        };
        state.processed_pos_eqns_mut().insert(partner);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().pm_type = HcbParamodulationType::SuperSim;

        let outcome = proof_state_generate_new_clauses(&mut state, &mut control, &selected)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.paramodulants, 1);
        assert_eq!(state.tmp_store().members(), 1);
        let generated = state.tmp_store().iter().next().unwrap();
        assert_eq!(generated.literal_number(), 1);
        let literal = &generated.literals().as_slice()[0];
        assert_eq!(literal.left(), &h_replacement);
        assert_eq!(literal.right(), &k_replacement);
    }

    #[test]
    fn proof_state_process_clause_allows_generation_when_paramodulation_is_skipped() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let selected = {
            let terms = state.terms_mut();
            let left = typed_const(terms, "pc_process_gen_skip_left");
            let right = typed_const(terms, "pc_process_gen_skip_right");
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &left, &right, false,
            )]));
            clause.set_ident(4_145);
            clause
        };
        let mut control = proof_control_alloc();
        init_fifo_hcb(&mut control, &state, "ProcessClauseGenerateEqResTest");
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().enable_neg_unit_paramod = false;
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed {
            class,
            generation,
            generated_empty,
            ..
        } = outcome
        else {
            panic!("negative unit should process with generation enabled");
        };
        assert_eq!(class, ProcessedClauseClass::NegativeUnit);
        assert_eq!(generation, GenerateNewClausesOutcome::default());
        assert!(generated_empty.is_none());
        assert_eq!(state.statistics().resolv_count, 0);
        assert_eq!(state.statistics().generated_count, 0);
    }

    #[test]
    fn proof_state_process_clause_records_answer_limit_extract_root() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let selected = answer_clause_with_id(state.terms_mut(), "pc_process_answer_limit", 4_160);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Returned { clause, reason } = outcome else {
            panic!("first answer should hit the configured answer limit");
        };
        assert_eq!(reason, ProcessClauseReturnReason::AnswerLimit);
        assert!(clause.is_empty());
        assert_eq!(state.extract_roots(), std::slice::from_ref(&clause));
        assert_eq!(state.statistics().answer_count, 1);
        assert_eq!(state.answer_outputs().len(), 2);
    }

    #[test]
    fn proof_state_process_clause_records_non_returning_answer_extract_root() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let selected =
            answer_clause_with_id(state.terms_mut(), "pc_process_answer_continue", 4_161);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_process_clause(&mut state, &mut control, 2)
            .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed {
            answer_detected,
            generated_empty,
            ..
        } = outcome
        else {
            panic!("answer below the limit should keep processing");
        };
        assert!(answer_detected);
        assert!(generated_empty.is_none());
        assert_eq!(state.extract_roots().len(), 1);
        let root = &state.extract_roots()[0];
        assert_eq!(root.ident(), 4_161);
        assert!(root.is_sem_false());
        assert!(!root.is_empty());
        assert_eq!(state.statistics().answer_count, 1);
        assert_eq!(state.processed_cardinality(), 1);
    }

    #[test]
    fn proof_state_process_clause_records_replacement_empty_extract_root() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let selected = {
            let terms = state.terms_mut();
            let left = typed_var(terms, -76);
            let right = typed_var(terms, -78);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &left, &right, false,
            )]));
            clause.set_ident(4_162);
            clause
        };
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        control.heuristic_parms_mut().er_varlit_destructive = true;
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Replaced { empty } = outcome else {
            panic!("destructive equality resolution should replace the selected clause");
        };
        let empty = empty.expect("destructive equality resolution should derive empty clause");
        assert!(empty.is_empty());
        assert!(derivation_contains_operation(&empty, DC_DES_EQ_RES));
        assert_eq!(state.extract_roots(), std::slice::from_ref(&empty));
    }

    #[test]
    fn proof_state_process_clause_records_generated_empty_extract_root() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let selected = {
            let terms = state.terms_mut();
            let variable = typed_var(terms, -78);
            let constant = typed_const(terms, "pc_process_generated_empty_const");
            let mut literal = literal(terms, &variable, &constant, false);
            literal.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
            clause.set_ident(4_163);
            clause
        };
        let mut control = proof_control_alloc();
        init_fifo_hcb(&mut control, &state, "ProcessClauseGeneratedRootTest");
        control.set_ocb(kbo_ocb(state.terms()));
        control.heuristic_parms_mut().enable_neg_unit_paramod = false;
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed {
            generated_empty, ..
        } = outcome
        else {
            panic!("negative unit should process and generate an empty resolvent");
        };
        let empty = generated_empty.expect("equality resolution should generate empty clause");
        assert!(empty.is_empty());
        assert_eq!(state.statistics().resolv_count, 1);
        assert_eq!(state.extract_roots(), std::slice::from_ref(&empty));
    }

    #[test]
    fn proof_state_saturate_return_with_extract_root_records_root() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut empty = Clause::empty();
        empty.set_ident(4_164);

        let outcome = super::proof_state_saturate_return_with_extract_root(
            &mut state,
            empty,
            SaturateReturnReason::GeneratedClause,
            3,
        );

        let SaturateOutcome::Returned {
            clause,
            reason,
            processed_steps,
        } = outcome
        else {
            panic!("helper should return a proof-success outcome");
        };
        assert_eq!(reason, SaturateReturnReason::GeneratedClause);
        assert_eq!(processed_steps, 3);
        assert_eq!(state.extract_roots(), std::slice::from_ref(clause.as_ref()));
    }

    #[test]
    fn proof_state_saturate_processes_until_unprocessed_empty() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let first = unit_clause_with_id(state.terms_mut(), "pc_saturate_first", 4_137);
        let second = unit_clause_with_id(state.terms_mut(), "pc_saturate_second", 4_138);
        queue_unprocessed_for_process(&mut state, &mut control, first);
        queue_unprocessed_for_process(&mut state, &mut control, second);

        let outcome = proof_state_saturate(
            &mut state,
            &mut control,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::Saturated,
                processed_steps: 2,
            }
        );
        assert_eq!(state.statistics().processed_count, 2);
        assert!(state.unprocessed().is_empty());
        assert_eq!(state.processed_cardinality(), 2);
    }

    #[test]
    fn proof_state_saturate_stops_on_cooperative_time_limit() {
        let _guard = global_state_lock();
        let _time_limits = configure_time_limits_for_test(0, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let clause = unit_clause_with_id(state.terms_mut(), "pc_saturate_time_limit", 4_151);
        queue_unprocessed_for_process(&mut state, &mut control, clause);

        let outcome = proof_state_saturate(
            &mut state,
            &mut control,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::TimeLimit,
                processed_steps: 0,
            }
        );
        assert_eq!(state.statistics().processed_count, 0);
        assert_eq!(state.unprocessed().members(), 1);
    }

    #[test]
    fn proof_state_saturate_closes_variable_predicate_horn_chain() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_fifo_hcb(&mut control, &state, "SaturatePredicateHornTest");
        control.set_ocb(kbo_ocb(state.terms()));
        let (rule, fact, goal) = {
            let terms = state.terms_mut();
            let socrates = typed_const(terms, "sat_pm_socrates");
            let x = typed_var(terms, -2);
            let human_code = unary_predicate_code(terms, "sat_pm_human");
            let mortal_code = unary_predicate_code(terms, "sat_pm_mortal");
            let human_socrates = unary_predicate(terms, human_code, &socrates);
            let human_x = unary_predicate(terms, human_code, &x);
            let mortal_x = unary_predicate(terms, mortal_code, &x);
            let mortal_socrates = unary_predicate(terms, mortal_code, &socrates);
            let truth = terms.true_term().clone();

            let mut rule_head = literal(terms, &mortal_x, &truth, true);
            let mut rule_tail = literal(terms, &human_x, &truth, false);
            rule_head.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            rule_tail.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut rule = Clause::alloc(EqnList::from_vec(vec![rule_head, rule_tail]));
            rule.set_ident(4_152);

            let mut fact_lit = literal(terms, &human_socrates, &truth, true);
            fact_lit.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut fact = Clause::alloc(EqnList::from_vec(vec![fact_lit]));
            fact.set_ident(4_153);

            let mut goal_lit = literal(terms, &mortal_socrates, &truth, false);
            goal_lit.set_prop(EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut goal = Clause::alloc(EqnList::from_vec(vec![goal_lit]));
            goal.set_ident(4_154);

            (rule, fact, goal)
        };
        queue_unprocessed_for_process(&mut state, &mut control, rule);
        queue_unprocessed_for_process(&mut state, &mut control, fact);
        queue_unprocessed_for_process(&mut state, &mut control, goal);

        let outcome = proof_state_saturate(
            &mut state,
            &mut control,
            20,
            20,
            50,
            100,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let SaturateOutcome::Returned { clause, reason, .. } = outcome else {
            panic!("predicate Horn chain should close");
        };
        assert!(clause.is_empty());
        assert_eq!(
            reason,
            SaturateReturnReason::ProcessClause(ProcessClauseReturnReason::EmptyClause)
        );
        assert_eq!(state.extract_roots(), std::slice::from_ref(clause.as_ref()));
    }

    #[test]
    fn proof_state_saturate_with_global_indices_uses_indexed_paramodulation() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, mut indexed_partner, source) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_saturate_idx_pm_source");
            let replacement = typed_const(terms, "pc_saturate_idx_pm_replacement");
            let rhs = typed_const(terms, "pc_saturate_idx_pm_rhs");
            let f_source = typed_unary(terms, "pc_saturate_idx_pm_f", &source);
            let mut selected_lit = literal(terms, &source, &replacement, true);
            let mut partner_lit = literal(terms, &f_source, &rhs, true);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_155);
            let partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            (selected, partner, source)
        };
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut indexed_partner, state.terms(), false);
        let mut control = proof_control_alloc();
        init_fifo_hcb(&mut control, &state, "SaturateIndexedParamodTest");
        control.set_ocb(kbo_ocb(state.terms()));
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_saturate_with_global_indices(
            &mut state,
            &mut control,
            1,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
            &mut indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::StepLimit,
                processed_steps: 1,
            }
        );
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.processed_cardinality(), 1);
        assert_eq!(state.unprocessed().members(), 1);
        assert!(indices.find_pm_from_occurrence(&source).is_some());
    }

    #[test]
    fn proof_state_saturate_with_docs_quotes_indexed_paramodulation() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, mut indexed_partner, source) = {
            let terms = state.terms_mut();
            let source = typed_const(terms, "pc_saturate_doc_idx_pm_source");
            let replacement = typed_const(terms, "pc_saturate_doc_idx_pm_replacement");
            let rhs = typed_const(terms, "pc_saturate_doc_idx_pm_rhs");
            let f_source = typed_unary(terms, "pc_saturate_doc_idx_pm_f", &source);
            let mut selected_lit = literal(terms, &source, &replacement, true);
            let mut partner_lit = literal(terms, &f_source, &rhs, true);
            selected_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            partner_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED | EP_MAX_IS_UP_TO_DATE);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![selected_lit]));
            selected.set_ident(4_155);
            let mut partner = Clause::alloc(EqnList::from_vec(vec![partner_lit]));
            partner.set_ident(4_156);
            (selected, partner, source)
        };
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        indices.insert_clause(&mut indexed_partner, state.terms(), false);
        let mut watchlist_indices = GlobalIndices::new("NoIndex", "NoIndex", "NoIndex", 0);
        let mut control = proof_control_alloc();
        init_fifo_hcb(&mut control, &state, "SaturateIndexedDocsTest");
        control.set_ocb(kbo_ocb(state.terms()));
        queue_unprocessed_for_process(&mut state, &mut control, selected);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_saturate_with_global_and_watchlist_indices_and_docs(
            &mut output,
            &mut session,
            2,
            &mut state,
            &mut control,
            1,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
            &mut indices,
            &mut watchlist_indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::StepLimit,
                processed_steps: 1,
            }
        );
        assert_eq!(state.statistics().paramod_count, 1);
        assert_eq!(state.processed_cardinality(), 1);
        assert_eq!(state.unprocessed().members(), 1);
        assert!(indices.find_pm_from_occurrence(&source).is_some());
        assert!(output.contains(" : pm(4156,4155)\n"), "{output}");
    }

    #[test]
    fn proof_state_saturate_with_docs_records_cleanup_contraction_before_status() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let selected =
            unit_clause_with_id(state.terms_mut(), "pc_saturate_cleanup_doc_given", 4_157);
        let cleanup_clause = {
            let terms = state.terms_mut();
            let same = typed_const(terms, "pc_saturate_cleanup_doc_same");
            let mut clause =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &same, &same, false)]));
            clause.set_ident(4_158);
            clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA | CP_LIMITED_RW);
            clause
        };
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, selected);
        queue_unprocessed_for_process(&mut state, &mut control, cleanup_clause);
        control.heuristic_parms_mut().forward_contract_limit = 0;
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        let mut watchlist_indices = GlobalIndices::new("NoIndex", "NoIndex", "NoIndex", 0);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_saturate_with_global_and_watchlist_indices_and_docs(
            &mut output,
            &mut session,
            2,
            &mut state,
            &mut control,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
            &mut indices,
            &mut watchlist_indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let SaturateOutcome::Returned {
            clause,
            reason,
            processed_steps,
        } = outcome
        else {
            panic!("maintenance cleanup should return its documented empty clause");
        };
        assert!(clause.is_empty());
        assert_eq!(clause.ident(), 1);
        assert_eq!(reason, SaturateReturnReason::Cleanup);
        assert_eq!(processed_steps, 1);
        assert_eq!(session.id_source.current_ident(), 1);
        let contraction = output
            .find("cn(4158)")
            .unwrap_or_else(|| panic!("missing cleanup contraction in {output}"));
        let status = output
            .find("Special forward-contraction")
            .unwrap_or_else(|| panic!("missing cleanup status in {output}"));
        assert!(contraction < status, "{output}");
    }

    #[test]
    fn proof_state_saturate_with_output_reports_dynamic_ac_activation() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (clause, f_code) =
            commutativity_axiom(state.terms_mut(), "pc_saturate_dynamic_ac_f", 4_156);
        let expected_clause = clause_print_lop_format_string(state.terms(), &clause, true);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        control.heuristic_parms_mut().selection_strategy = NO_GENERATION.to_owned();
        queue_unprocessed_for_process(&mut state, &mut control, clause);
        let mut output = Vec::new();

        let outcome = proof_state_saturate_with_output(
            &mut output,
            1,
            &mut state,
            &mut control,
            1,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::Saturated,
                processed_steps: 1,
            }
        );
        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!(
                "%% pc_saturate_dynamic_ac_f is commutative\n% AC handling enabled dynamically\n\n%{expected_clause}\n"
            )
        );
    }

    #[test]
    fn proof_state_saturate_with_output_reports_dynamic_watchlist_reduction() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, watched) =
            watchlist_subsumption_pair(state.terms_mut(), "pc_saturate_watch_output", 4_157, 4_158);
        let expected_clause = clause_print_lop_format_string(state.terms(), &selected, true);
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        control.heuristic_parms_mut().ac_handling = AcHandling::None;
        control.heuristic_parms_mut().selection_strategy = NO_GENERATION.to_owned();
        control.heuristic_parms_mut().watchlist_is_static = false;
        proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| panic!("{err}"));
        queue_unprocessed_for_process(&mut state, &mut control, selected);
        let mut output = Vec::new();

        let outcome = proof_state_saturate_with_output(
            &mut output,
            1,
            &mut state,
            &mut control,
            1,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::Saturated,
                processed_steps: 1,
            }
        );
        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("%\n%{expected_clause}\n% Watchlist reduced by 1 clause\n")
        );
        assert_eq!(state.watchlist().unwrap().members(), 0);
    }

    #[test]
    fn proof_state_saturate_stops_at_step_limit_after_iteration() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let first = unit_clause_with_id(state.terms_mut(), "pc_saturate_step_first", 4_139);
        let second = unit_clause_with_id(state.terms_mut(), "pc_saturate_step_second", 4_140);
        queue_unprocessed_for_process(&mut state, &mut control, first);
        queue_unprocessed_for_process(&mut state, &mut control, second);

        let outcome = proof_state_saturate(
            &mut state,
            &mut control,
            1,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::StepLimit,
                processed_steps: 1,
            }
        );
        assert_eq!(state.statistics().processed_count, 1);
        assert_eq!(state.unprocessed().members(), 1);
    }

    #[test]
    fn proof_state_saturate_stops_on_active_empty_watchlist_only() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        state
            .load_watchlist(WatchlistSource::Inline, IoFormat::Lop)
            .unwrap_or_else(|err| panic!("{err}"));
        assert!(state.watchlist_active());
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let clause = unit_clause_with_id(state.terms_mut(), "pc_saturate_watch", 4_141);
        queue_unprocessed_for_process(&mut state, &mut control, clause);

        let outcome = proof_state_saturate(
            &mut state,
            &mut control,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::WatchlistEmpty,
                processed_steps: 0,
            }
        );
        assert_eq!(state.statistics().processed_count, 0);
        assert_eq!(state.unprocessed().members(), 1);
    }

    #[test]
    fn proof_state_saturate_runs_due_sat_check_and_records_model() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let clause = unit_clause_with_id(state.terms_mut(), "pc_saturate_satcheck", 4_142);
        queue_unprocessed_for_process(&mut state, &mut control, clause);
        control.heuristic_parms_mut().sat_check_grounding = GroundingStrategy::GlobalMin;
        control.heuristic_parms_mut().sat_check_step_limit = 1;

        let outcome = proof_state_saturate(
            &mut state,
            &mut control,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::Saturated,
                processed_steps: 1,
            }
        );
        assert_eq!(state.statistics().satcheck_count, 1);
        assert_eq!(state.statistics().satcheck_satisfiable, 1);
        assert_eq!(state.statistics().satcheck_success, 0);
        assert_eq!(state.statistics().satcheck_full_size, 0);
        assert_eq!(state.statistics().satcheck_actual_size, 0);
        assert_eq!(control.solver().generation(), 2);
    }

    #[test]
    fn proof_state_saturate_sat_check_refutes_opposite_pseudo_ground_units() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let positive = signed_unit_clause_with_id(
            state.terms_mut(),
            "pc_saturate_satcheck_unsat",
            4_143,
            true,
        );
        let negative = signed_unit_clause_with_id(
            state.terms_mut(),
            "pc_saturate_satcheck_unsat",
            4_144,
            false,
        );
        queue_unprocessed_for_process(&mut state, &mut control, positive);
        queue_unprocessed_for_process(&mut state, &mut control, negative);
        control.heuristic_parms_mut().sat_check_grounding = GroundingStrategy::GlobalMin;
        control.heuristic_parms_mut().sat_check_step_limit = 1;

        let outcome = proof_state_saturate(
            &mut state,
            &mut control,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let SaturateOutcome::Returned {
            clause,
            reason,
            processed_steps,
        } = outcome
        else {
            panic!("SATCheck should return an empty proof clause");
        };
        assert!(clause.is_empty());
        assert_eq!(state.extract_roots(), std::slice::from_ref(clause.as_ref()));
        assert_eq!(reason, SaturateReturnReason::SatCheck);
        assert_eq!(processed_steps, 1);
        assert_eq!(state.statistics().satcheck_count, 1);
        assert_eq!(state.statistics().satcheck_satisfiable, 0);
        assert_eq!(state.statistics().satcheck_success, 1);
        assert_eq!(state.statistics().satcheck_full_size, 2);
        assert_eq!(state.statistics().satcheck_actual_size, 2);
        assert_eq!(state.statistics().satcheck_core_size, 2);
        assert_eq!(control.solver().generation(), 2);
    }

    #[test]
    fn proof_state_saturate_distinguishes_sat_check_preprocessing_refutation() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let positive = signed_unit_clause_with_id(
            state.terms_mut(),
            "pc_saturate_satcheck_preprocessing",
            4_145,
            true,
        );
        let negative = signed_unit_clause_with_id(
            state.terms_mut(),
            "pc_saturate_satcheck_preprocessing",
            4_146,
            false,
        );
        queue_unprocessed_for_process(&mut state, &mut control, positive);
        queue_unprocessed_for_process(&mut state, &mut control, negative);
        control.heuristic_parms_mut().sat_check_grounding = GroundingStrategy::GlobalMin;
        control.heuristic_parms_mut().sat_check_step_limit = 1;
        control.heuristic_parms_mut().sat_check_normalize = true;

        let outcome = proof_state_saturate(
            &mut state,
            &mut control,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let SaturateOutcome::Returned {
            clause,
            reason,
            processed_steps,
        } = outcome
        else {
            panic!("SATCheck preprocessing should return an empty proof clause");
        };
        assert!(clause.is_empty());
        assert_eq!(state.extract_roots(), std::slice::from_ref(clause.as_ref()));
        assert_eq!(reason, SaturateReturnReason::SatCheckPreprocessing);
        assert_eq!(processed_steps, 1);
        assert_eq!(state.statistics().satcheck_count, 0);
        assert_eq!(state.statistics().satcheck_success, 0);
        assert_eq!(control.solver().generation(), 1);
    }

    #[test]
    fn proof_state_saturate_with_docs_records_sat_check_normalization_refutation() {
        let _guard = global_state_lock();
        let _time_limits =
            configure_time_limits_for_test(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let positive = signed_unit_clause_with_id(
            state.terms_mut(),
            "pc_saturate_satcheck_doc_preprocessing",
            4_147,
            true,
        );
        let negative = signed_unit_clause_with_id(
            state.terms_mut(),
            "pc_saturate_satcheck_doc_preprocessing",
            4_148,
            false,
        );
        queue_unprocessed_for_process(&mut state, &mut control, positive);
        queue_unprocessed_for_process(&mut state, &mut control, negative);
        control.heuristic_parms_mut().sat_check_grounding = GroundingStrategy::GlobalMin;
        control.heuristic_parms_mut().sat_check_step_limit = 1;
        control.heuristic_parms_mut().sat_check_normalize = true;
        let mut indices = GlobalIndices::new("NoIndex", "FP1", "FP1", 0);
        let mut watchlist_indices = GlobalIndices::new("NoIndex", "NoIndex", "NoIndex", 0);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 2, ProblemType::FirstOrder);

        let outcome = proof_state_saturate_with_global_and_watchlist_indices_and_docs(
            &mut output,
            &mut session,
            2,
            &mut state,
            &mut control,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            1,
            &mut indices,
            &mut watchlist_indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let SaturateOutcome::Returned {
            clause,
            reason,
            processed_steps,
        } = outcome
        else {
            panic!("SATCheck normalization should return its documented empty clause");
        };
        assert!(clause.is_empty());
        assert_eq!(clause.ident(), 1, "{output}");
        assert_eq!(reason, SaturateReturnReason::SatCheckPreprocessing);
        assert_eq!(processed_steps, 1);
        assert_eq!(session.id_source.current_ident(), 1, "{output}");
        assert!(output.contains("cn(4148)"), "{output}");
        assert_eq!(state.statistics().satcheck_count, 0);
        assert_eq!(state.statistics().satcheck_success, 0);
        assert_eq!(control.solver().generation(), 1);
    }

    #[test]
    fn proof_state_process_clause_selects_and_inserts_processed_survivor() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        let clause = unit_clause_with_id(state.terms_mut(), "pc_process_survivor", 4_130);
        queue_unprocessed_for_process(&mut state, &mut control, clause);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed {
            class,
            answer_detected,
            ac_activated,
            watchlist,
            backward,
            generation,
            generated_empty,
        } = outcome
        else {
            panic!("selected survivor should be inserted into a processed set");
        };
        assert!(matches!(
            class,
            ProcessedClauseClass::PositiveRule | ProcessedClauseClass::PositiveEquation
        ));
        assert!(!answer_detected);
        assert!(!ac_activated);
        assert_eq!(watchlist, ProofStateWatchlistOutcome::default());
        assert_eq!(backward, BackwardSimplificationOutcome::default());
        assert_eq!(generation, GenerateNewClausesOutcome::default());
        assert!(generated_empty.is_none());
        assert_eq!(state.statistics().processed_count, 1);
        assert_eq!(state.statistics().proc_non_trivial_count, 1);
        assert!(state.unprocessed().is_empty());
        assert_eq!(
            state.processed_pos_rules().members() + state.processed_pos_eqns().members(),
            1
        );
    }

    #[test]
    fn proof_state_process_clause_backward_subsumes_processed_non_unit() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, subsumed) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_process_back_sub_target");
            let witness = typed_const(terms, "pc_process_back_sub_witness");
            let guard_left = typed_const(terms, "pc_process_back_sub_guard_left");
            let guard_right = typed_const(terms, "pc_process_back_sub_guard_right");
            let mut selected = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &witness, &target, true,
            )]));
            selected.set_ident(4_131);
            let mut subsumed = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &witness, &target, true),
                literal(terms, &guard_left, &guard_right, true),
            ]));
            subsumed.set_ident(4_132);
            subsumed.set_weight(subsumed.standard_weight());
            (selected, subsumed)
        };
        state.processed_non_units_mut().insert(subsumed);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed {
            class,
            backward,
            generated_empty,
            ..
        } = outcome
        else {
            panic!("selected non-unit should survive processing");
        };
        assert!(matches!(
            class,
            ProcessedClauseClass::PositiveRule | ProcessedClauseClass::PositiveEquation
        ));
        assert_eq!(backward.subsumed, 1);
        assert_eq!(state.statistics().backward_subsumed_count, 1);
        assert!(generated_empty.is_none());
        assert!(state.processed_non_units().find_by_id(4_132).is_none());
        assert!(
            state.processed_pos_rules().find_by_id(4_131).is_some()
                || state.processed_pos_eqns().find_by_id(4_131).is_some()
        );
        let archived = state.archive().find_by_id(4_132).unwrap();
        assert!(archived.query_prop(CP_IS_DEAD));
    }

    #[test]
    fn backward_simplification_uses_global_rewrite_index_across_processed_sets() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (
            demodulator,
            mut pos_rule,
            mut pos_eqn,
            mut neg_unit,
            mut non_unit,
            sentinel,
            compound,
        ) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_back_index_target");
            let replacement = typed_const(terms, "pc_back_index_replacement");
            let compound = typed_unary(terms, "pc_back_index_f", &target);
            let mut demod_lit = literal(terms, &compound, &replacement, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_170);
            demodulator.set_weight(demodulator.standard_weight());

            let make_candidate = |terms: &mut TermBank, suffix: &str, ident: i64| {
                let right = typed_const(terms, &format!("pc_back_index_{suffix}"));
                let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
                    terms, &compound, &right, true,
                )]));
                clause.set_ident(ident);
                clause.set_prop(CP_IS_PROCESSED | CP_LIMITED_RW);
                clause.set_weight(clause.standard_weight());
                clause
            };

            (
                demodulator,
                make_candidate(terms, "pos_rule", 4_171),
                make_candidate(terms, "pos_eqn", 4_172),
                make_candidate(terms, "neg_unit", 4_173),
                make_candidate(terms, "non_unit", 4_174),
                make_candidate(terms, "unindexed", 4_175),
                compound,
            )
        };
        assert!(demodulator.is_demodulator());

        let mut indices = GlobalIndices::new_for_problem(
            "FP1",
            "NoIndex",
            "NoIndex",
            -1,
            ProblemType::FirstOrder,
        );
        indices.insert_clause(&mut pos_rule, state.terms(), false);
        indices.insert_clause(&mut pos_eqn, state.terms(), false);
        indices.insert_clause(&mut neg_unit, state.terms(), false);
        indices.insert_clause(&mut non_unit, state.terms(), false);
        state.processed_pos_rules_mut().insert(pos_rule);
        state.processed_pos_eqns_mut().insert(pos_eqn);
        state.processed_neg_units_mut().insert(neg_unit);
        state.processed_non_units_mut().insert(non_unit);
        state.processed_non_units_mut().insert(sentinel);

        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut clause_date = SysDate::from_raw(12);
        let outcome = proof_state_backward_simplify_with_global_indices(
            &mut state,
            &mut control,
            &demodulator,
            &mut clause_date,
            &mut indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(clause_date, SysDate::from_raw(13));
        assert_eq!(
            outcome,
            BackwardSimplificationOutcome {
                rewritten: 4,
                rewritten_literals: 4,
                subsumed: 0,
                unit_simplified: 0,
                context_sr: 0,
                tmp_store_marked: 4,
                min_rw_detected: true,
            }
        );
        assert!(state.processed_pos_rules().is_empty());
        assert!(state.processed_pos_eqns().is_empty());
        assert!(state.processed_neg_units().is_empty());
        assert_eq!(state.processed_non_units().members(), 1);
        assert!(state.processed_non_units().find_by_id(4_175).is_some());
        assert_eq!(state.tmp_store().members(), 4);
        assert_eq!(state.archive().members(), 4);
        for ident in 4_171..=4_174 {
            let archived = state.archive().find_by_id(ident).unwrap();
            assert!(archived.query_prop(CP_IS_DEAD));
            assert!(!archived.query_prop(CP_IS_GLOBAL_INDEXED));
        }
        assert!(indices.find_bw_rw_occurrence(&compound).is_none());
    }

    #[test]
    fn proof_state_process_clause_with_docs_quotes_backward_subsumed_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, subsumed) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_process_back_doc_sub_target");
            let witness = typed_const(terms, "pc_process_back_doc_sub_witness");
            let guard_left = typed_const(terms, "pc_process_back_doc_sub_guard_left");
            let guard_right = typed_const(terms, "pc_process_back_doc_sub_guard_right");
            let mut selected = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &witness, &target, true,
            )]));
            selected.set_ident(4_154);
            let mut subsumed = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &witness, &target, true),
                literal(terms, &guard_left, &guard_right, true),
            ]));
            subsumed.set_ident(4_155);
            subsumed.set_weight(subsumed.standard_weight());
            (selected, subsumed)
        };
        state.processed_non_units_mut().insert(subsumed);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, selected);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);

        let outcome = proof_state_process_clause_with_docs(
            &mut output,
            &mut session,
            6,
            &mut state,
            &mut control,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed { backward, .. } = outcome else {
            panic!("selected non-unit should survive processing");
        };
        assert_eq!(backward.subsumed, 1);
        assert!(output.contains(" : 4154 : 'new_given'\n"));
        assert!(output.contains(" : 4155 : 'subsumed(1)'\n"));
        assert!(state.archive().find_by_id(2).is_some());
        assert!(state.archive().find_by_id(4_155).is_none());
    }

    #[test]
    fn proof_state_process_clause_with_docs_quotes_unit_simplified_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, simplified) = {
            let terms = state.terms_mut();
            let left = typed_const(terms, "pc_process_unit_doc_left");
            let right = typed_const(terms, "pc_process_unit_doc_right");
            let guard_left = typed_const(terms, "pc_process_unit_doc_guard_left");
            let guard_right = typed_const(terms, "pc_process_unit_doc_guard_right");
            let mut selected = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &left, &right, false,
            )]));
            selected.set_ident(4_156);
            let mut simplified = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left, &right, true),
                literal(terms, &guard_left, &guard_right, true),
            ]));
            simplified.set_ident(4_157);
            simplified.set_weight(simplified.standard_weight());
            simplified.set_prop(CP_IS_PROCESSED | CP_LIMITED_RW);
            (selected, simplified)
        };
        state.processed_non_units_mut().insert(simplified);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, selected);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);

        let outcome = proof_state_process_clause_with_docs(
            &mut output,
            &mut session,
            6,
            &mut state,
            &mut control,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed { backward, .. } = outcome else {
            panic!("selected negative unit should survive processing");
        };
        assert_eq!(backward.subsumed, 0);
        assert_eq!(backward.unit_simplified, 1);
        assert_eq!(backward.tmp_store_marked, 1);
        assert!(output.contains(" : 4156 : 'new_given'\n"));
        assert!(output.contains(" : 4157 : 'simplifiable'\n"));
        assert!(output.contains(" : 3 : 'eval'\n"));
        assert!(state.processed_non_units().find_by_id(4_157).is_none());
        assert!(state.archive().find_by_id(2).is_some());
        assert!(state.tmp_store().is_empty());
    }

    #[test]
    fn proof_state_process_clause_with_docs_quotes_dynamic_watchlist_extraction() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, watched) = {
            let terms = state.terms_mut();
            let left = typed_const(terms, "pc_process_watch_doc_left");
            let right = typed_const(terms, "pc_process_watch_doc_right");
            let guard_left = typed_const(terms, "pc_process_watch_doc_guard_left");
            let guard_right = typed_const(terms, "pc_process_watch_doc_guard_right");
            let mut selected =
                Clause::alloc(EqnList::from_vec(vec![literal(terms, &left, &right, true)]));
            selected.set_ident(4_160);
            let mut watched = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left, &right, true),
                literal(terms, &guard_left, &guard_right, true),
            ]));
            watched.set_ident(4_161);
            watched.set_prop(CP_INPUT_FORMULA | CP_WATCH_ONLY);
            watched.set_weight(watched.standard_weight());
            (selected, watched)
        };
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        control.heuristic_parms_mut().watchlist_is_static = false;
        queue_unprocessed_for_process(&mut state, &mut control, selected);
        let mut output = String::new();
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 6, ProblemType::FirstOrder);

        let outcome = proof_state_process_clause_with_docs(
            &mut output,
            &mut session,
            6,
            &mut state,
            &mut control,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ProcessClauseOutcome::Processed {
            class, watchlist, ..
        } = outcome
        else {
            panic!("selected clause should survive processing");
        };
        assert_eq!(
            watchlist,
            ProofStateWatchlistOutcome {
                subsumes_watch: true,
                removed: 1,
            }
        );
        assert_eq!(session.id_source.current_ident(), 3);
        assert!(output.contains(" : 4160 : 'new_given'\n"));
        assert!(output.contains(" : 4161 : 'extract_wl_subsumed(1)'\n"));
        assert!(output.contains(" : 1 : 'extract_subsumed_watched'\n"));
        assert_eq!(state.watchlist().unwrap().members(), 0);
        let archived = state.archive().find_by_id(2).unwrap();
        assert!(archived.query_prop(CP_IS_DEAD | CP_WATCH_ONLY));
        assert!(!archived.query_prop(CP_INPUT_FORMULA));
        assert!(match class {
            ProcessedClauseClass::PositiveRule => state.processed_pos_rules().find_by_id(3),
            ProcessedClauseClass::PositiveEquation => state.processed_pos_eqns().find_by_id(3),
            ProcessedClauseClass::NegativeUnit => state.processed_neg_units().find_by_id(3),
            ProcessedClauseClass::NonUnit => state.processed_non_units().find_by_id(3),
        }
        .is_some());
    }

    #[test]
    fn proof_state_simplify_watchlist_rewrites_and_reinserts_watched_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, watched, target, other) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_watch_simpl_target");
            let other = typed_const(terms, "pc_watch_simpl_other");
            let compound = typed_unary(terms, "pc_watch_simpl_f", &target);
            let mut demod_lit = literal(terms, &compound, &target, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_133);
            demodulator.set_date(SysDate::from_raw(7));
            demodulator.set_weight(demodulator.standard_weight());
            let mut watched = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &compound, &other, true,
            )]));
            watched.set_ident(4_134);
            watched.set_weight(watched.standard_weight());
            (demodulator, watched, target, other)
        };
        state.processed_pos_rules_mut().insert(demodulator.clone());
        state.processed_pos_rules_mut().set_date(demodulator.date());
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));

        let simplified = proof_state_simplify_watchlist(&mut state, &mut control, &demodulator)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(simplified, 1);
        assert_eq!(state.watchlist().unwrap().members(), 1);
        assert_eq!(state.archive().members(), 1);
        assert!(state
            .archive()
            .find_by_id(4_134)
            .unwrap()
            .query_prop(CP_IS_DEAD));
        let simplified = state.watchlist().unwrap().find_by_id(4_134).unwrap();
        let literal = &simplified.literals().as_slice()[0];
        assert_eq!(literal.left(), &target);
        assert_eq!(literal.right(), &other);
        assert!(simplified.query_prop(CP_IS_ORIENTED));
        assert!(state.statistics().rw_count >= 1);
    }

    #[test]
    fn proof_state_simplify_watchlist_with_global_indices_reindexes_watched_clause() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, watched, compound, target, other) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_watch_simpl_gidx_target");
            let other = typed_const(terms, "pc_watch_simpl_gidx_other");
            let compound = typed_unary(terms, "pc_watch_simpl_gidx_f", &target);
            let mut demod_lit = literal(terms, &compound, &target, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_135);
            demodulator.set_date(SysDate::from_raw(8));
            demodulator.set_weight(demodulator.standard_weight());
            let mut watched = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &compound, &other, true,
            )]));
            watched.set_ident(4_136);
            watched.set_weight(watched.standard_weight());
            (demodulator, watched, compound, target, other)
        };
        state.processed_pos_rules_mut().insert(demodulator.clone());
        state.processed_pos_rules_mut().set_date(demodulator.date());
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut indices = GlobalIndices::new_for_problem(
            "FP1",
            "NoIndex",
            "NoIndex",
            -1,
            ProblemType::FirstOrder,
        );
        assert_eq!(
            proof_state_insert_watchlist_global_indices_into(
                &mut state,
                &mut indices,
                control.heuristic_parms().lambda_demod,
            ),
            1
        );
        assert!(indices.find_bw_rw_occurrence(&compound).is_some());

        let simplified = proof_state_simplify_watchlist_with_global_indices(
            &mut state,
            &mut control,
            &demodulator,
            &mut indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(simplified, 1);
        assert!(indices.find_bw_rw_occurrence(&compound).is_none());
        assert!(indices.find_bw_rw_occurrence(&target).is_some());
        let archived = state.archive().find_by_id(4_136).unwrap();
        assert!(archived.query_prop(CP_IS_DEAD));
        assert!(!archived.query_prop(CP_IS_GLOBAL_INDEXED));
        let simplified = state.watchlist().unwrap().find_by_id(4_136).unwrap();
        assert!(simplified.query_prop(CP_IS_GLOBAL_INDEXED));
        let literal = &simplified.literals().as_slice()[0];
        assert_eq!(literal.left(), &target);
        assert_eq!(literal.right(), &other);
    }

    #[test]
    fn proof_state_simplify_watchlist_with_global_indices_uses_indexed_candidates() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, watched, compound, target, other) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_watch_simpl_indexed_target");
            let other = typed_const(terms, "pc_watch_simpl_indexed_other");
            let compound = typed_unary(terms, "pc_watch_simpl_indexed_f", &target);
            let mut demod_lit = literal(terms, &compound, &target, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_139);
            demodulator.set_date(SysDate::from_raw(10));
            demodulator.set_weight(demodulator.standard_weight());
            let mut watched = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &compound, &other, true,
            )]));
            watched.set_ident(4_140);
            watched.set_weight(watched.standard_weight());
            (demodulator, watched, compound, target, other)
        };
        state.processed_pos_rules_mut().insert(demodulator.clone());
        state.processed_pos_rules_mut().set_date(demodulator.date());
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut indices = GlobalIndices::new_for_problem(
            "FP1",
            "NoIndex",
            "NoIndex",
            -1,
            ProblemType::FirstOrder,
        );
        assert!(indices.has_bw_rw_index());
        assert!(indices.find_bw_rw_occurrence(&compound).is_none());

        let simplified = proof_state_simplify_watchlist_with_global_indices(
            &mut state,
            &mut control,
            &demodulator,
            &mut indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(simplified, 0);
        assert_eq!(state.archive().members(), 0);
        let watched = state.watchlist().unwrap().find_by_id(4_140).unwrap();
        let literal = &watched.literals().as_slice()[0];
        assert_eq!(literal.left(), &compound);
        assert_eq!(literal.right(), &other);
        assert!(!watched.query_prop(CP_IS_GLOBAL_INDEXED));
        assert!(indices.find_bw_rw_occurrence(&target).is_none());
    }

    #[test]
    fn proof_state_simplify_watchlist_with_global_indices_scans_without_backward_index() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, watched, target, other) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_watch_simpl_noindex_target");
            let other = typed_const(terms, "pc_watch_simpl_noindex_other");
            let compound = typed_unary(terms, "pc_watch_simpl_noindex_f", &target);
            let mut demod_lit = literal(terms, &compound, &target, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_141);
            demodulator.set_date(SysDate::from_raw(11));
            demodulator.set_weight(demodulator.standard_weight());
            let mut watched = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &compound, &other, true,
            )]));
            watched.set_ident(4_142);
            watched.set_weight(watched.standard_weight());
            (demodulator, watched, target, other)
        };
        state.processed_pos_rules_mut().insert(demodulator.clone());
        state.processed_pos_rules_mut().set_date(demodulator.date());
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut indices = GlobalIndices::new_for_problem(
            "NoIndex",
            "NoIndex",
            "NoIndex",
            -1,
            ProblemType::FirstOrder,
        );
        assert!(!indices.has_bw_rw_index());

        let simplified = proof_state_simplify_watchlist_with_global_indices(
            &mut state,
            &mut control,
            &demodulator,
            &mut indices,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(simplified, 1);
        let archived = state.archive().find_by_id(4_142).unwrap();
        assert!(archived.query_prop(CP_IS_DEAD));
        let simplified = state.watchlist().unwrap().find_by_id(4_142).unwrap();
        assert!(simplified.query_prop(CP_IS_GLOBAL_INDEXED));
        let literal = &simplified.literals().as_slice()[0];
        assert_eq!(literal.left(), &target);
        assert_eq!(literal.right(), &other);
    }

    #[test]
    fn proof_state_simplify_watchlist_with_docs_emits_rewrite_and_minimize_steps() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (demodulator, watched, target, other) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_watch_simpl_doc_target");
            let other = typed_const(terms, "pc_watch_simpl_doc_other");
            let compound = typed_unary(terms, "pc_watch_simpl_doc_f", &target);
            let mut demod_lit = literal(terms, &compound, &target, true);
            demod_lit.set_prop(EP_IS_ORIENTED | EP_IS_MAXIMAL | EP_MAX_IS_UP_TO_DATE);
            let mut demodulator = Clause::alloc(EqnList::from_vec(vec![demod_lit]));
            demodulator.set_ident(4_137);
            demodulator.set_date(SysDate::from_raw(9));
            demodulator.set_weight(demodulator.standard_weight());
            let mut watched = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &compound, &other, true),
                literal(terms, &other, &target, true),
                literal(terms, &target, &target, false),
            ]));
            watched.set_ident(4_138);
            watched.set_prop(CP_INPUT_FORMULA);
            watched.set_weight(watched.standard_weight());
            (demodulator, watched, target, other)
        };
        state.processed_pos_rules_mut().insert(demodulator.clone());
        state.processed_pos_rules_mut().set_date(demodulator.date());
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 4, ProblemType::FirstOrder);
        session.pcl_shell_level = 1;
        let mut rendered = String::new();

        let simplified = proof_state_simplify_watchlist_with_docs(
            &mut rendered,
            &mut session,
            &mut state,
            &mut control,
            &demodulator,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(simplified, 1);
        assert_eq!(session.id_source.current_ident(), 2);
        assert!(rendered.contains("rw(4138,4137)"));
        assert!(rendered.contains("cn(1)"));
        assert_eq!(state.watchlist().unwrap().members(), 1);
        assert!(state
            .archive()
            .find_by_id(4_138)
            .unwrap()
            .query_prop(CP_IS_DEAD));
        let simplified = state.watchlist().unwrap().find_by_id(2).unwrap();
        assert_eq!(simplified.literal_number(), 1);
        assert!(!simplified.query_prop(CP_INPUT_FORMULA));
        let literal = &simplified.literals().as_slice()[0];
        let kept_expected_equality = (literal.left() == &target && literal.right() == &other)
            || (literal.left() == &other && literal.right() == &target);
        assert!(kept_expected_equality);
        assert!(state.statistics().rw_count >= 1);
    }

    #[test]
    fn proof_state_process_clause_simplifies_nonempty_watchlist() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, watched, target, other) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_process_watch_simpl_target");
            let other = typed_const(terms, "pc_process_watch_simpl_other");
            let compound = typed_unary(terms, "pc_process_watch_simpl_f", &target);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &compound, &target, true,
            )]));
            selected.set_ident(4_135);
            let mut watched = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &compound, &other, true,
            )]));
            watched.set_ident(4_136);
            watched.set_weight(watched.standard_weight());
            (selected, watched, target, other)
        };
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, selected);

        let outcome = proof_state_process_clause(&mut state, &mut control, 1)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(matches!(outcome, ProcessClauseOutcome::Processed { .. }));
        assert_eq!(state.watchlist().unwrap().members(), 1);
        assert_eq!(state.archive().members(), 1);
        assert!(state
            .archive()
            .find_by_id(4_136)
            .unwrap()
            .query_prop(CP_IS_DEAD));
        let simplified = state.watchlist().unwrap().find_by_id(4_136).unwrap();
        let literal = &simplified.literals().as_slice()[0];
        assert_eq!(literal.left(), &target);
        assert_eq!(literal.right(), &other);
        assert!(state.statistics().rw_count >= 1);
    }

    #[test]
    fn proof_state_process_clause_with_docs_threads_watchlist_simplification_docs() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (selected, watched, target, other) = {
            let terms = state.terms_mut();
            let target = typed_const(terms, "pc_process_watch_simpl_doc_target");
            let other = typed_const(terms, "pc_process_watch_simpl_doc_other");
            let compound = typed_unary(terms, "pc_process_watch_simpl_doc_f", &target);
            let mut selected = Clause::alloc(EqnList::from_vec(vec![literal(
                terms, &compound, &target, true,
            )]));
            selected.set_ident(4_158);
            let mut watched = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &compound, &other, true),
                literal(terms, &other, &target, true),
                literal(terms, &target, &target, false),
            ]));
            watched.set_ident(4_159);
            watched.set_prop(CP_INPUT_FORMULA);
            watched.set_weight(watched.standard_weight());
            (selected, watched, target, other)
        };
        state.watchlist_mut().unwrap().insert(watched);
        let mut control = proof_control_alloc();
        init_process_clause_control(&mut control, &state);
        queue_unprocessed_for_process(&mut state, &mut control, selected);
        let mut session =
            ProofDocSession::new(ProofDocOutputFormat::Pcl, 4, ProblemType::FirstOrder);
        session.pcl_shell_level = 1;
        let mut output = String::new();

        let outcome = proof_state_process_clause_with_docs(
            &mut output,
            &mut session,
            4,
            &mut state,
            &mut control,
            1,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(matches!(outcome, ProcessClauseOutcome::Processed { .. }));
        assert_eq!(session.id_source.current_ident(), 2);
        assert!(output.contains("rw(4159,4158)"));
        assert!(output.contains("cn(1)"));
        assert_eq!(state.watchlist().unwrap().members(), 1);
        assert!(state
            .archive()
            .find_by_id(4_159)
            .unwrap()
            .query_prop(CP_IS_DEAD));
        let simplified = state.watchlist().unwrap().find_by_id(2).unwrap();
        assert_eq!(simplified.literal_number(), 1);
        assert!(!simplified.query_prop(CP_INPUT_FORMULA));
        let literal = &simplified.literals().as_slice()[0];
        let kept_expected_equality = (literal.left() == &target && literal.right() == &other)
            || (literal.left() == &other && literal.right() == &target);
        assert!(kept_expected_equality);
        assert!(state.statistics().rw_count >= 1);
    }

    #[test]
    fn proof_state_insert_processed_clause_indexes_oriented_rule() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        control.set_ocb(kbo_ocb(state.terms()));
        proof_state_init_indexing(&mut state, &mut control).unwrap_or_else(|err| panic!("{err}"));
        let date = SysDate::from_raw(17);
        let (clause, lhs) = {
            let terms = state.terms_mut();
            let lhs = typed_const(terms, "pc_processed_rule_lhs");
            let rhs = typed_const(terms, "pc_processed_rule_rhs");
            let mut literal = literal(terms, &lhs, &rhs, true);
            literal.set_prop(EP_IS_ORIENTED);
            let mut clause = Clause::alloc(EqnList::from_vec(vec![literal]));
            clause.set_ident(4_088);
            (clause, lhs)
        };

        let class = proof_state_insert_processed_clause(&mut state, clause, date)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(class, ProcessedClauseClass::PositiveRule);
        assert!(lhs.query_prop(TP_IS_REWRITABLE));
        assert_eq!(state.processed_pos_rules().members(), 1);
        assert_eq!(state.processed_pos_rules().date(), date);
        let stored = state.processed_pos_rules().find_by_id(4_088).unwrap();
        assert_eq!(stored.date(), date);
        assert!(stored.query_prop(CP_LIMITED_RW));
        assert!(stored.query_prop(CP_IS_S_INDEXED));
        assert_eq!(stored.weight(), stored.standard_weight());
        assert!(state.processed_pos_eqns().is_empty());
        assert!(state.processed_neg_units().is_empty());
        assert!(state.processed_non_units().is_empty());
    }

    #[test]
    fn proof_state_insert_processed_clause_classifies_non_rule_sets() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let date = SysDate::from_raw(23);
        let (positive_equation, negative_unit, non_unit) = {
            let terms = state.terms_mut();
            let positive_equation =
                unit_clause_with_id(terms, "pc_processed_positive_equation", 4_089);
            let mut negative_unit = negative_clause(terms);
            negative_unit.set_ident(4_090);
            let left = typed_const(terms, "pc_processed_non_unit_left");
            let right = typed_const(terms, "pc_processed_non_unit_right");
            let guard = typed_const(terms, "pc_processed_non_unit_guard");
            let mut non_unit = Clause::alloc(EqnList::from_vec(vec![
                literal(terms, &left, &right, true),
                literal(terms, &guard, &right, false),
            ]));
            non_unit.set_ident(4_091);
            (positive_equation, negative_unit, non_unit)
        };

        let positive_class =
            proof_state_insert_processed_clause(&mut state, positive_equation, date)
                .unwrap_or_else(|err| panic!("{err}"));
        let negative_class = proof_state_insert_processed_clause(&mut state, negative_unit, date)
            .unwrap_or_else(|err| panic!("{err}"));
        let non_unit_class = proof_state_insert_processed_clause(&mut state, non_unit, date)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(positive_class, ProcessedClauseClass::PositiveEquation);
        assert_eq!(negative_class, ProcessedClauseClass::NegativeUnit);
        assert_eq!(non_unit_class, ProcessedClauseClass::NonUnit);
        assert_eq!(state.processed_pos_eqns().members(), 1);
        assert_eq!(state.processed_pos_eqns().date(), date);
        assert_eq!(state.processed_neg_units().members(), 1);
        assert_eq!(state.processed_non_units().members(), 1);
        assert!(state.processed_pos_rules().is_empty());
        for ident in [4_089, 4_090, 4_091] {
            let found = state
                .processed_pos_eqns()
                .find_by_id(ident)
                .or_else(|| state.processed_neg_units().find_by_id(ident))
                .or_else(|| state.processed_non_units().find_by_id(ident))
                .unwrap();
            assert_eq!(found.date(), date);
            assert!(found.query_prop(CP_LIMITED_RW));
            assert_eq!(found.weight(), found.standard_weight());
        }
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
    fn proof_state_check_ac_status_enables_ac_dynamically() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut clause, f_code) = commutativity_axiom(state.terms_mut(), "pc_dynamic_ac_f", 4_092);
        let mut control = proof_control_alloc();

        let activated = proof_state_check_ac_status(&mut state, &mut control, &mut clause);
        let already_active = proof_state_check_ac_status(&mut state, &mut control, &mut clause);

        assert!(activated);
        assert!(!already_active);
        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
    }

    #[test]
    fn signature_ac_parent_survives_renumber_and_archive_requeue() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut clause, _) =
            commutativity_axiom(state.terms_mut(), "pc_dynamic_ac_owner_f", 4_096);
        let mut control = proof_control_alloc();

        assert!(proof_state_check_ac_status(
            &mut state,
            &mut control,
            &mut clause
        ));
        let signature_ref = state.terms().signature().ac_axioms()[0];
        assert_ne!(signature_ref.generation(), 0);

        clause.set_ident(1);
        let current_ref = ClauseDerivationRef::from(&clause);
        state.processed_pos_eqns_mut().insert(clause);

        assert_eq!(signature_ref, current_ref);
        assert_eq!(state.ac_axiom_parent_refs(), vec![current_ref]);
        assert_eq!(
            state
                .proof_clause_by_derivation_ref(signature_ref)
                .map(ClauseDerivationRef::from),
            Some(current_ref)
        );

        let archived = state
            .processed_pos_eqns_mut()
            .extract_by_id(1)
            .expect("state still owns the processed AC parent");
        let requeued = proof_state_archive_simplified_clause(&mut state, archived)
            .expect("AC parent can be archived and requeued");
        let requeued_ref = ClauseDerivationRef::from(&requeued);
        state.tmp_store_mut().insert(requeued);

        assert_ne!(signature_ref, requeued_ref);
        assert!(state
            .archive()
            .find_by_derivation_ref(signature_ref)
            .is_some());
        assert!(state
            .tmp_store()
            .find_by_derivation_ref(requeued_ref)
            .is_some());
        assert_eq!(state.ac_axiom_parent_refs(), vec![current_ref]);
    }

    #[test]
    fn proof_state_check_ac_status_with_output_reports_dynamic_activation_once() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut clause, f_code) =
            commutativity_axiom(state.terms_mut(), "pc_dynamic_ac_print_f", 4_094);
        let mut control = proof_control_alloc();
        let mut output = Vec::new();

        let activated = proof_state_check_ac_status_with_output(
            &mut output,
            1,
            &mut state,
            &mut control,
            &mut clause,
        )
        .unwrap();
        let already_active = proof_state_check_ac_status_with_output(
            &mut output,
            1,
            &mut state,
            &mut control,
            &mut clause,
        )
        .unwrap();

        assert!(activated);
        assert!(!already_active);
        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% pc_dynamic_ac_print_f is commutative\n% AC handling enabled dynamically\n"
        );
    }

    #[test]
    fn proof_state_check_ac_status_with_output_obeys_zero_output_level() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut clause, f_code) =
            commutativity_axiom(state.terms_mut(), "pc_dynamic_ac_quiet_f", 4_095);
        let mut control = proof_control_alloc();
        let mut output = Vec::new();

        let activated = proof_state_check_ac_status_with_output(
            &mut output,
            0,
            &mut state,
            &mut control,
            &mut clause,
        )
        .unwrap();

        assert!(activated);
        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
        assert!(output.is_empty());
    }

    #[test]
    fn proof_state_check_ac_status_skips_scan_when_disabled() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (mut clause, f_code) =
            commutativity_axiom(state.terms_mut(), "pc_dynamic_no_ac_f", 4_093);
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().ac_handling = AcHandling::None;

        let activated = proof_state_check_ac_status(&mut state, &mut control, &mut clause);

        assert!(!activated);
        assert!(!control.ac_handling_active());
        assert!(!state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
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
    fn proof_state_init_ac_handling_with_output_reports_scan_and_activation() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, f_code) = commutativity_axiom(state.terms_mut(), "pc_init_ac_print_f", 4_015);
        state.unprocessed_mut().insert(axiom);
        let mut control = proof_control_alloc();
        let mut output = Vec::new();

        let active =
            proof_state_init_ac_handling_with_output(&mut output, 1, &mut state, &mut control)
                .unwrap();

        assert!(active);
        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "% Scanning for AC axioms\n% pc_init_ac_print_f is commutative\n% AC handling enabled\n"
        );
    }

    #[test]
    fn proof_state_init_ac_handling_with_output_obeys_zero_output_level() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, f_code) = commutativity_axiom(state.terms_mut(), "pc_init_ac_quiet_f", 4_016);
        state.unprocessed_mut().insert(axiom);
        let mut control = proof_control_alloc();
        let mut output = Vec::new();

        let active =
            proof_state_init_ac_handling_with_output(&mut output, 0, &mut state, &mut control)
                .unwrap();

        assert!(active);
        assert!(control.ac_handling_active());
        assert!(state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
        assert!(output.is_empty());
    }

    #[test]
    fn proof_state_init_ac_handling_with_output_skips_output_when_disabled() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let (axiom, f_code) =
            commutativity_axiom(state.terms_mut(), "pc_init_ac_disabled_print_f", 4_017);
        state.unprocessed_mut().insert(axiom);
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().ac_handling = AcHandling::None;
        let mut output = Vec::new();

        let active =
            proof_state_init_ac_handling_with_output(&mut output, 1, &mut state, &mut control)
                .unwrap();

        assert!(!active);
        assert!(!control.ac_handling_active());
        assert!(!state.terms().signature().query_prop(f_code, FP_COMMUTATIVE));
        assert!(output.is_empty());
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
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().rw_bw_index_type = "FP1".to_owned();
        control.heuristic_parms_mut().pm_from_index_type = "NoIndex".to_owned();
        control.heuristic_parms_mut().pm_into_index_type = "FP7".to_owned();
        control.heuristic_parms_mut().ext_rules_max_depth = 3;
        proof_state_init_global_indices(&mut state, &control, ProblemType::HigherOrder);
        let indices = state.global_indices();
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
    fn proof_state_init_watchlist_indices_enables_only_rewriting() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().rw_bw_index_type = "FP1".to_owned();
        control.heuristic_parms_mut().pm_from_index_type = "FP6".to_owned();
        control.heuristic_parms_mut().pm_into_index_type = "FP7".to_owned();
        control.heuristic_parms_mut().ext_rules_max_depth = 3;

        proof_state_init_watchlist_global_indices(&mut state, &control, ProblemType::HigherOrder);

        let indices = state.watchlist_indices();
        assert!(indices.has_bw_rw_index());
        assert!(!indices.has_pm_from_index());
        assert!(!indices.has_pm_into_index());
        assert!(!indices.has_pm_negp_index());
        assert!(indices.has_ext_into_index());
        assert!(indices.has_ext_from_index());
        assert_eq!(indices.rw_bw_index_type(), "FP1");
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

        let outcome =
            proof_state_init_with_global_indices(&mut state, &mut control, ProblemType::FirstOrder)
                .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(outcome.initial_clauses, 1);
        let indices = state.global_indices();
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
    fn proof_control_installs_varweight_with_active_owner_context() {
        let mut bank = test_bank();
        let left_base = typed_const(&mut bank, "pc_varweight_left");
        let right_base = typed_const(&mut bank, "pc_varweight_right");
        let left = typed_unary(&mut bank, "pc_varweight_f", &left_base);
        let right = typed_unary(&mut bank, "pc_varweight_g", &right_base);
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &left, &right, true,
        )]));
        let mut axioms = ClauseSet::new();
        let mut control = proof_control_alloc();
        let mut params = HeuristicParmsCell {
            heuristic_name: "VarOwnerSearch".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let wfcb_defs = vec!["var_owner=Depthweight(ConstPrio,2,1,3.0,5.0,7.0,11.0)".to_owned()];
        let mut hcb_defs = vec!["VarOwnerSearch=(1*var_owner)".to_owned()];

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!clause.query_prop(CP_IS_ORIENTED));
        assert!(!clause.literals().as_slice()[0].is_maximal());
        let active_hcb_handle = control.active_hcb.expect("varweight HCB should be active");
        let super::ProofControl {
            hcbs, wfcbs, ocb, ..
        } = &mut control;
        let hcb = hcbs
            .hcb(active_hcb_handle)
            .expect("varweight HCB should be installed");
        let ocb = ocb.as_mut().expect("proof ordering should be installed");

        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut clause)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(clause.query_prop(CP_IS_ORIENTED));
        assert!(clause.literals().as_slice()[0].is_maximal());
        assert_eq!(
            clause
                .evaluations()
                .expect("HCB should attach one varweight evaluation")
                .eval_no(),
            1
        );
    }

    #[test]
    fn proof_control_installs_funweights_with_active_owner_context() {
        let mut bank = test_bank();
        let left_base = typed_const(&mut bank, "pc_funweight_left");
        let right_base = typed_const(&mut bank, "pc_funweight_right");
        let left = typed_unary(&mut bank, "pc_funweight_f", &left_base);
        let right = typed_unary(&mut bank, "pc_funweight_g", &right_base);
        let mut first_clause = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &left, &right, true,
        )]));
        let mut second_clause = first_clause.clone();
        let mut axioms = ClauseSet::new();
        let mut control = proof_control_alloc();
        let mut params = HeuristicParmsCell {
            heuristic_name: "FunOwnerSearch".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let wfcb_defs = vec![
            "fun_owner=FunWeight(ConstPrio,2,1,3.0,5.0,7.0,pc_funweight_f:10,pc_funweight_left:20)"
                .to_owned(),
            "offset_owner=SymOffsetWeight(ConstPrio,2,1,3.0,5.0,7.0,pc_funweight_f:10,pc_funweight_left:-3)"
                .to_owned(),
        ];
        let mut hcb_defs = vec!["FunOwnerSearch=(1*fun_owner,1*offset_owner)".to_owned()];

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!first_clause.query_prop(CP_IS_ORIENTED));
        assert!(!second_clause.query_prop(CP_IS_ORIENTED));
        let active_hcb_handle = control.active_hcb.expect("funweight HCB should be active");
        let super::ProofControl {
            hcbs, wfcbs, ocb, ..
        } = &mut control;
        let hcb = hcbs
            .hcb(active_hcb_handle)
            .expect("funweight HCB should be installed");
        let ocb = ocb.as_mut().expect("proof ordering should be installed");

        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut first_clause)
            .unwrap_or_else(|err| panic!("{err}"));
        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut second_clause)
            .unwrap_or_else(|err| panic!("{err}"));

        for clause in [&first_clause, &second_clause] {
            assert!(clause.query_prop(CP_IS_ORIENTED));
            assert!(clause.literals().as_slice()[0].is_maximal());
            assert_eq!(
                clause
                    .evaluations()
                    .expect("HCB should attach both funweight evaluations")
                    .eval_no(),
                2
            );
        }
        let first_evaluations = first_clause
            .evaluations()
            .expect("first clause should retain evaluations");
        let second_evaluations = second_clause
            .evaluations()
            .expect("second clause should retain evaluations");
        for index in 0..2 {
            assert_eq!(
                first_evaluations.eval(index).heuristic().to_bits(),
                second_evaluations.eval(index).heuristic().to_bits()
            );
        }
    }

    fn assert_conjecture_term_weights_with_active_owner_context(rel_terms: i32) {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "pc_termweight_a");
        let b = typed_const(&mut bank, "pc_termweight_b");
        let conjecture_term = typed_unary(&mut bank, "pc_termweight_f", &a);
        let target_term = typed_unary(&mut bank, "pc_termweight_g", &a);
        let mut conjecture = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank,
            &conjecture_term,
            &a,
            false,
        )]));
        conjecture.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut first_clause = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank,
            &target_term,
            &b,
            true,
        )]));
        let mut second_clause = first_clause.clone();
        let mut axioms = ClauseSet::from_clauses([conjecture]);
        let mut control = proof_control_alloc();
        let mut params = HeuristicParmsCell {
            heuristic_name: "TermOwnerSearch".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let wfcb_defs = vec![
            format!(
                "relative=ConjectureRelativeTermWeight(ConstPrio,0,{rel_terms},2.0,10,3,20,1,0,1.0,1.0,1.0)"
            ),
            format!(
                "prefix=ConjectureTermPrefixWeight(ConstPrio,0,{rel_terms},0.5,5.0,0,1.0,1.0,1.0)"
            ),
            format!(
                "tfidf=ConjectureTermTfIdfWeight(ConstPrio,0,{rel_terms},0,1.0,0,1.0,1.0,1.0)"
            ),
            format!(
                "lev=ConjectureLevDistanceWeight(ConstPrio,0,{rel_terms},1,1,5,0,1.0,1.0,1.0)"
            ),
            format!(
                "tree=ConjectureTreeDistanceWeight(ConstPrio,0,{rel_terms},1,1,5,0,1.0,1.0,1.0)"
            ),
            format!(
                "struc=ConjectureStrucDistanceWeight(ConstPrio,0,{rel_terms},5.0,10.0,2.0,3.0,0,1.0,1.0,1.0)"
            ),
        ];
        let mut hcb_defs =
            vec!["TermOwnerSearch=(1*relative,1*prefix,1*tfidf,1*lev,1*tree,1*struc)".to_owned()];

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!first_clause.query_prop(CP_IS_ORIENTED));
        assert!(!second_clause.query_prop(CP_IS_ORIENTED));
        let active_hcb_handle = control
            .active_hcb
            .expect("conjecture-term HCB should be active");
        let super::ProofControl {
            hcbs, wfcbs, ocb, ..
        } = &mut control;
        let hcb = hcbs
            .hcb(active_hcb_handle)
            .expect("conjecture-term HCB should be installed");
        let ocb = ocb.as_mut().expect("proof ordering should be installed");

        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut first_clause)
            .unwrap_or_else(|err| panic!("{err}"));
        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut second_clause)
            .unwrap_or_else(|err| panic!("{err}"));

        for clause in [&first_clause, &second_clause] {
            assert!(clause.query_prop(CP_IS_ORIENTED));
            assert!(clause.literals().as_slice()[0].is_maximal());
            assert_eq!(
                clause
                    .evaluations()
                    .expect("HCB should attach all six term-weight evaluations")
                    .eval_no(),
                6
            );
        }
        let first_evaluations = first_clause
            .evaluations()
            .expect("first clause should retain evaluations");
        let second_evaluations = second_clause
            .evaluations()
            .expect("second clause should retain evaluations");
        for index in 0..6 {
            assert_eq!(
                first_evaluations.eval(index).heuristic().to_bits(),
                second_evaluations.eval(index).heuristic().to_bits()
            );
        }
    }

    #[test]
    fn proof_control_installs_conjecture_term_weights_with_active_owner_context() {
        assert_conjecture_term_weights_with_active_owner_context(0);
    }

    #[test]
    fn proof_control_installs_all_related_conjecture_term_sets() {
        for rel_terms in 1..=3 {
            assert_conjecture_term_weights_with_active_owner_context(rel_terms);
        }
    }

    #[test]
    fn tfidf_problem_scores_use_the_parsed_frequency_factor() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "tfidf_a");
        let b = typed_const(&mut bank, "tfidf_b");
        let c = typed_const(&mut bank, "tfidf_c");
        let g_b = typed_unary(&mut bank, "tfidf_g", &b);
        let g_c = typed_unary(&mut bank, "tfidf_g", &c);
        let f_code = typed_binary_code(&mut bank, "tfidf_f");
        let f_a_g_b = typed_binary_with_code(&mut bank, f_code, &a, &g_b);
        let f_a_g_c = typed_binary_with_code(&mut bank, f_code, &a, &g_c);
        let h_c = typed_unary(&mut bank, "tfidf_h", &c);
        let q_code = unary_predicate_code(&mut bank, "tfidf_q");
        let r_code = unary_predicate_code(&mut bank, "tfidf_r");
        let p_code = unary_predicate_code(&mut bank, "tfidf_p");
        let nested_atom = unary_predicate(&mut bank, q_code, &f_a_g_b);
        let flat_atom = unary_predicate(&mut bank, r_code, &h_c);
        let goal_atom = unary_predicate(&mut bank, p_code, &f_a_g_c);
        let truth = bank.true_term().clone();
        let nested = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank,
            &nested_atom,
            &truth,
            true,
        )]));
        let flat = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &flat_atom, &truth, true,
        )]));
        let mut goal = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &goal_atom, &truth, false,
        )]));
        goal.set_tptp_type(CP_TYPE_NEG_CONJECTURE);
        let mut axioms = ClauseSet::from_clauses([nested.clone(), flat.clone(), goal.clone()]);
        let mut control = proof_control_alloc();
        let mut params = HeuristicParmsCell {
            heuristic_name: "TfIdfParsedFactor".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let wfcb_defs =
            vec!["tfidf=ConjectureTermTfIdfWeight(ConstPrio,0,0,0,1.0,0,1.0,1.0,1.0)".to_owned()];
        let mut hcb_defs = vec!["TfIdfParsedFactor=(1*tfidf)".to_owned()];

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let active_hcb_handle = control.active_hcb.expect("TF-IDF HCB should be active");
        let super::ProofControl {
            hcbs, wfcbs, ocb, ..
        } = &mut control;
        let hcb = hcbs
            .hcb(active_hcb_handle)
            .expect("TF-IDF HCB should exist");
        let ocb = ocb.as_mut().expect("proof ordering should be installed");
        let mut clauses = [nested, flat, goal];
        for clause in &mut clauses {
            hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, clause)
                .unwrap_or_else(|err| panic!("{err}"));
        }
        let scores = clauses.map(|clause| {
            clause
                .evaluations()
                .expect("TF-IDF should attach an evaluation")
                .eval(0)
                .heuristic()
        });
        assert_eq!(
            scores.map(f32::to_bits),
            [0x3F_B7_AB_66, 0x3F_B7_AB_66, 0x3F_3A_AE_08]
        );
    }

    #[test]
    fn proof_control_installs_diversity_and_orient_weights_with_active_owner_context() {
        let mut bank = test_bank();
        let left_base = typed_const(&mut bank, "pc_diversity_left");
        let right_base = typed_const(&mut bank, "pc_diversity_right");
        let left = typed_unary(&mut bank, "pc_diversity_f", &left_base);
        let right = typed_unary(&mut bank, "pc_diversity_g", &right_base);
        let mut first_clause = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &left, &right, true,
        )]));
        let mut second_clause = first_clause.clone();
        let mut axioms = ClauseSet::new();
        let mut control = proof_control_alloc();
        let mut params = HeuristicParmsCell {
            heuristic_name: "DiversityOrientOwnerSearch".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let wfcb_defs = vec![
            "diversity=Diversityweight(ConstPrio,2,3,1.0,1.0,1.0,10.0,1.0,20.0,2.0)".to_owned(),
            "orient=Orientweight(ConstPrio,2,1,7.0,5.0,3.0)".to_owned(),
            "lmax=OrientLMaxWeight(ConstPrio,2,1,7.0,5.0,3.0)".to_owned(),
        ];
        let mut hcb_defs =
            vec!["DiversityOrientOwnerSearch=(1*diversity,1*orient,1*lmax)".to_owned()];

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!first_clause.query_prop(CP_IS_ORIENTED));
        assert!(!second_clause.query_prop(CP_IS_ORIENTED));
        let active_hcb_handle = control
            .active_hcb
            .expect("diversity/orient HCB should be active");
        let super::ProofControl {
            hcbs, wfcbs, ocb, ..
        } = &mut control;
        let hcb = hcbs
            .hcb(active_hcb_handle)
            .expect("diversity/orient HCB should be installed");
        let ocb = ocb.as_mut().expect("proof ordering should be installed");

        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut first_clause)
            .unwrap_or_else(|err| panic!("{err}"));
        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut second_clause)
            .unwrap_or_else(|err| panic!("{err}"));

        for clause in [&first_clause, &second_clause] {
            assert!(clause.query_prop(CP_IS_ORIENTED));
            assert!(clause.literals().as_slice()[0].is_maximal());
            assert_eq!(
                clause
                    .evaluations()
                    .expect("HCB should attach all three diversity/orient evaluations")
                    .eval_no(),
                3
            );
        }
        let first_evaluations = first_clause
            .evaluations()
            .expect("first clause should retain evaluations");
        let second_evaluations = second_clause
            .evaluations()
            .expect("second clause should retain evaluations");
        for index in 0..3 {
            assert_eq!(
                first_evaluations.eval(index).heuristic().to_bits(),
                second_evaluations.eval(index).heuristic().to_bits()
            );
        }
    }

    #[test]
    fn proof_control_installs_dag_weights_with_exact_owner_split() {
        let mut bank = test_bank();
        let left_base = typed_const(&mut bank, "pc_dag_left");
        let right_base = typed_const(&mut bank, "pc_dag_right");
        let left = typed_unary(&mut bank, "pc_dag_f", &left_base);
        let right = typed_unary(&mut bank, "pc_dag_g", &right_base);
        let mut first_clause = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &left, &right, true,
        )]));
        let mut second_clause = first_clause.clone();
        let mut axioms = ClauseSet::new();
        let mut control = proof_control_alloc();
        let mut params = HeuristicParmsCell {
            heuristic_name: "DagOwnerSearch".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let wfcb_defs = vec![
            "rdag=RDAGweight(ConstPrio,10,3,1,5.0,2.0,7.0,4.0)".to_owned(),
            "dag=DAGweight(ConstPrio,2,1,3.0,1,true,false,false,true,false,false,false)".to_owned(),
            "rdag2=RDAGweight2(ConstPrio,10,3,1,4.0,2.0)".to_owned(),
            "rdag3=RDAGweight3(ConstPrio,2,1,13,17,1,3.0,5.0,7.0,11.0)".to_owned(),
        ];
        let mut hcb_defs = vec!["DagOwnerSearch=(1*rdag,1*dag,1*rdag2,1*rdag3)".to_owned()];

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(!first_clause.query_prop(CP_IS_ORIENTED));
        assert!(!second_clause.query_prop(CP_IS_ORIENTED));
        let active_hcb_handle = control.active_hcb.expect("DAG HCB should be active");
        let super::ProofControl {
            hcbs, wfcbs, ocb, ..
        } = &mut control;
        let hcb = hcbs
            .hcb(active_hcb_handle)
            .expect("DAG HCB should be installed");
        let ocb = ocb.as_mut().expect("proof ordering should be installed");

        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut first_clause)
            .unwrap_or_else(|err| panic!("{err}"));
        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut second_clause)
            .unwrap_or_else(|err| panic!("{err}"));

        for clause in [&first_clause, &second_clause] {
            assert!(clause.query_prop(CP_IS_ORIENTED));
            assert!(clause.literals().as_slice()[0].is_maximal());
            assert_eq!(
                clause
                    .evaluations()
                    .expect("HCB should attach all four DAG evaluations")
                    .eval_no(),
                4
            );
        }
        let first_evaluations = first_clause
            .evaluations()
            .expect("first clause should retain evaluations");
        let second_evaluations = second_clause
            .evaluations()
            .expect("second clause should retain evaluations");
        for index in 0..4 {
            assert_eq!(
                first_evaluations.eval(index).heuristic().to_bits(),
                second_evaluations.eval(index).heuristic().to_bits()
            );
        }
    }

    #[test]
    fn proof_control_installs_tsm_with_shared_proof_state_bank() {
        let kb_dir = proof_control_tsm_kb_dir();
        write_proof_control_tsm_kb(&kb_dir);
        let kb_arg = kb_dir.to_string_lossy().replace('\\', "/");
        let mut bank = test_bank();
        let target = typed_const(&mut bank, "pc_tsm_target");
        let mut clause = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &target, &target, true,
        )]));
        let mut axioms = ClauseSet::from_clauses([clause.clone()]);
        let mut control = proof_control_alloc();
        let mut params = HeuristicParmsCell {
            heuristic_name: "LearnedSearch".to_owned(),
            ..HeuristicParmsCell::default()
        };
        let wfcb_defs = vec![format!(
            "learned=TSMWeight(ConstPrio,2,3,0.5,rec,{kb_arg},1,1.0,1.0,Flat,IndexArity,0,1,0,0,0,0,0)"
        )];
        let mut hcb_defs = vec!["LearnedSearch=(1*learned)".to_owned()];

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(bank.signature().find_f_code("pc_tsm_pattern_sym"), 0);
        let active_hcb_handle = control
            .active_hcb
            .expect("learned heuristic should be active");
        let super::ProofControl {
            hcbs, wfcbs, ocb, ..
        } = &mut control;
        let hcb = hcbs
            .hcb(active_hcb_handle)
            .expect("learned heuristic should be installed");
        let ocb = ocb.as_mut().expect("proof ordering should be installed");
        hcb_clause_evaluate_with_bank(hcb, wfcbs, ocb, &mut bank, &mut clause)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(clause.evaluations().is_some());
        assert_ne!(bank.signature().find_f_code("pc_tsm_pattern_sym"), 0);

        std::fs::remove_dir_all(&kb_dir).expect("remove proof-control TSM KB");
    }

    #[test]
    fn proof_control_weight_context_preserves_formula_relevance_levels() {
        let mut bank = test_bank();
        let a = typed_const(&mut bank, "pc_formula_rel_a");
        let b = typed_const(&mut bank, "pc_formula_rel_b");
        let f_a = typed_unary(&mut bank, "pc_formula_rel_f", &a);
        let g_b = typed_unary(&mut bank, "pc_formula_rel_g", &b);
        let f_g_b = typed_unary(&mut bank, "pc_formula_rel_f", &g_b);
        let mut conjecture = WrappedFormula::wt_formula_alloc(f_a.clone());
        conjecture.set_tptp_type(CP_TYPE_CONJECTURE);
        let mut axiom = WrappedFormula::wt_formula_alloc(f_g_b);
        axiom.set_tptp_type(CP_TYPE_AXIOM);
        let mut formulas = FormulaSet::new();
        formulas.insert(conjecture);
        formulas.insert(axiom);
        let target = Clause::alloc(EqnList::from_vec(vec![literal(
            &mut bank, &f_a, &g_b, true,
        )]));
        let axioms = ClauseSet::new();
        let mut control = proof_control_alloc();
        let mut params = HeuristicParmsCell::default();
        let wfcb_defs = vec![
            "formula_rel=RelevanceLevelWeight(ConstPrio,0.0,1.0,0.0,10,2,3,5,7,1.0,1.0,1.0)"
                .to_owned(),
        ];
        let mut hcb_defs = Vec::new();

        proof_control_init_heuristics_with_formula_axioms(
            &mut control,
            &axioms,
            &formulas,
            &mut params,
            &FvIndexParams::default(),
            &wfcb_defs,
            &mut hcb_defs,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let actual = control
            .wfcbs_mut()
            .find_wfcb_mut("formula_rel")
            .expect("formula relevance WFCB should be installed")
            .compute_eval(&bank, &target);
        assert_eq!(actual.to_bits(), 15.0_f64.to_bits());
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
    fn proof_control_init_owns_higher_order_lambda_ocb_like_c() {
        let mut control = proof_control_alloc();
        let mut bank = test_bank();
        typed_const(&mut bank, "pc_lambda_order_a");
        let expected_sig_size = bank.signature().f_count();
        let mut axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell::default();
        params.order_params.ordertype = TermOrdering::Kbo6;
        params.order_params.ho_order_kind = HoOrderKind::LambdaOrder;
        params.order_params.lam_w = 30;
        params.order_params.db_w = 12;
        let mut hcb_defs = Vec::new();

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
            true,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ocb = control.ocb().expect("proof control should own an OCB");
        assert_eq!(ocb.ordering_type, TermOrdering::Kbo6);
        assert_eq!(ocb.ho_order_kind, HoOrderKind::LambdaOrder);
        assert_eq!(ocb.sig_size, expected_sig_size);
        assert_eq!(ocb.vb_size, 1);
        assert!(ocb.ho_vb.is_empty());
        assert_eq!(ocb.lam_weight, 30);
        assert_eq!(ocb.db_weight, 12);
    }

    #[test]
    fn proof_control_init_preserves_explicit_classic_kbo_for_higher_order_problem() {
        let mut control = proof_control_alloc();
        let mut bank = test_bank();
        typed_const(&mut bank, "pc_classic_kbo_ho_a");
        let mut axioms = ClauseSet::new();
        let mut params = HeuristicParmsCell::default();
        params.order_params.ordertype = TermOrdering::Kbo;
        let mut hcb_defs = Vec::new();

        proof_control_init(
            &mut control,
            &mut bank,
            &mut axioms,
            &mut params,
            &FvIndexParams::default(),
            &[],
            &mut hcb_defs,
            true,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let ocb = control
            .ocb()
            .expect("proof control should own the requested classic KBO");
        assert_eq!(ocb.ordering_type, TermOrdering::Kbo);
        assert!(ocb.weights.is_some());
        assert!(ocb.prec_weights.is_some());
        assert!(ocb.precedence.is_none());
    }

    fn proof_control_tsm_kb_dir() -> std::path::PathBuf {
        std::path::PathBuf::from("target")
            .join("e-rust-port-tests")
            .join(format!("proof-control-tsm-{}", std::process::id()))
    }

    fn write_proof_control_tsm_kb(kb_dir: &Path) {
        if kb_dir.exists() {
            std::fs::remove_dir_all(kb_dir).expect("remove stale proof-control TSM KB");
        }
        std::fs::create_dir_all(kb_dir).expect("create proof-control TSM KB");
        std::fs::write(
            kb_dir.join("clausepatterns"),
            "pc_tsm_pattern_sym : 1:(1,1,0,0,0,0,0).",
        )
        .expect("write proof-control TSM clause patterns");
        std::fs::write(kb_dir.join("signature"), "pc_tsm_pattern_sym:0")
            .expect("write proof-control TSM signature");
        let mut features = String::from("PA: () FA: () (0");
        for _ in 1..FEATURE_NUMBER {
            features.push_str(", 0");
        }
        features.push(')');
        std::fs::write(kb_dir.join("problems"), format!("1: \"only\" {features}"))
            .expect("write proof-control TSM problems");
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
    fn do_literal_selection_bankless_reports_missing_context_only_if_reached() {
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
        assert!(error.to_string().contains("unavailable"));
    }

    #[test]
    fn do_literal_selection_with_bank_applies_ordering_dependent_selector() {
        let mut bank = test_bank();
        let mut control = proof_control_alloc();
        control.heuristic_parms_mut().selection_strategy = SELECT_UNLESS_POS_MAX.to_owned();
        let mut clause = negative_clause(&mut bank);
        control.set_ocb(kbo_ocb(&bank));

        let outcome = do_literal_selection_with_bank(&mut control, &mut bank, &mut clause)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert_eq!(outcome, LiteralSelectionOutcome::SelectorApplied);
        assert_eq!(clause.prop_lit_number(EP_IS_SELECTED), 1);
    }
}
