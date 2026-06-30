use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{check_option_letter_string, Diagnostic, ErrorCode};
#[cfg(all(windows, not(test)))]
use crate::basics::os_wrapper::set_hard_cpu_limit;
#[cfg(not(test))]
use crate::basics::os_wrapper::set_memory_limit;
#[cfg(target_os = "linux")]
use crate::basics::os_wrapper::RLIMIT_DATA_COMPAT;
use crate::basics::os_wrapper::{
    current_resource_usage, format_resource_usage, get_core_number, get_system_phys_memory,
    resource_limit_error_message, RLimResult, RLimitOutcome,
};
#[cfg(all(target_os = "linux", not(test)))]
use crate::basics::os_wrapper::{
    set_soft_rlimit_with_error, RLIMIT_CORE_COMPAT, RLIMIT_CPU_COMPAT,
};
use crate::basics::partial_orderings::HoOrderKind;
use crate::basics::pstacks::PStack;
use crate::basics::simple_stuff::{
    problem_type, reset_problem_type, set_problem_type, ProblemType,
};
use crate::basics::stringtrees::StrTree;
use crate::basics::verbose::set_verbose_level;
use crate::clauses::bce::eliminate_blocked_clauses_with_output;
use crate::clauses::clause::{
    clause_parse, clause_print_lop_format_string_with_options,
    clause_print_tptp_format_string_with_options, clause_write_tstp_with_type_suffixes, Clause,
};
use crate::clauses::clause_props::{
    clause_type_from_identifier, FormulaProperties, CP_IGNORE_PROPS, CP_INITIAL, CP_INPUT_FORMULA,
    CP_IS_LAMBDA_DEF, CP_SUBSUMES_WATCH, CP_TYPE_AXIOM, CP_TYPE_CONJECTURE, CP_TYPE_HYPOTHESIS,
    CP_TYPE_LEMMA, CP_TYPE_NEG_CONJECTURE, CP_TYPE_QUESTION, CP_TYPE_WATCH_CLAUSE,
};
use crate::clauses::clauseinfo::{source_info_pcl_string, source_info_tstp_string, ClauseInfo};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::derivation::{
    demodulator_clause_refs, deriv_stack_pcl_string_with_ac_axioms,
    deriv_stack_tstp_string_with_ac_axioms, op_has_arg1, op_has_arg2, op_has_cnf_arg1,
    op_has_cnf_arg2, ClauseDerivationRef, DerivationEntry,
};
use crate::clauses::eqn::{eqn_fof_parse, eqn_write_app_encode, Eqn, EqnPrintOptions};
use crate::clauses::eqnlist::EqnList;
use crate::clauses::f_generality::GenDistrib;
use crate::clauses::fcvindexing::FvIndexParams;
use crate::clauses::freqvectors::FvIndexType;
use crate::clauses::gd_transformation::clause_set_gd_transform;
use crate::clauses::global_indices::GlobalIndices;
use crate::clauses::inferencedoc::{
    pcl_print_end, pcl_print_start, ClauseCreationInference, ClauseCreationParents,
    PclStepPrintOptions, ProofDocIdSource, ProofDocOutputFormat, ProofDocSession,
};
use crate::clauses::proofstate::{
    proof_state_alloc, ProofObjectAnalysis, ProofObjectGraph, ProofState, RawFormulaFeatures,
    WatchlistSource as ProofStateWatchlistSource,
};
use crate::clauses::relevance::clause_set_relevance_prune;
use crate::clauses::sine::{
    select_axioms_clause_sets, select_threshold_clause_sets, ClauseSineParams,
};
use crate::clauses::unfold_defs::{clause_set_preprocess, clause_set_unfold_eq_def_normalize};
use crate::heuristics::axfilter::{sine_get_filter, AxFilter, AxFilterType};
use crate::heuristics::clausesetfeatures::{
    create_default_spec_limits, proof_state_print_selective_string, spec_features_add_eval,
    spec_features_compute_clause_set_without_choice, spec_type_string, SpecFeatureCell, SpecLimits,
};
use crate::heuristics::hcb::{self, heuristic_parms_parse_into, HeuristicParmsCell};
use crate::heuristics::litselection::NO_GENERATION;
use crate::heuristics::new_autoschedule::{
    get_heuristic_with_name, get_preprocessing_schedule, get_search_schedule,
    heuristic_parms_strategy_print_string, initialize_placeholder_search_schedule,
    schedule_times_init_multi_core, strategies_print_predefined_string, ScheduleCell, DEFAULT_MASK,
    DEFAULT_SCHED_TIME_LIMIT,
};
use crate::heuristics::proofcontrol::{
    proof_control_init, proof_state_filter_unprocessed, proof_state_init_with_ac_output,
    proof_state_reset_processed_with_global_indices, proof_state_saturate_with_global_indices,
    ProofControl, SaturateOutcome, SaturateReturnReason, SaturateStopReason,
};
use crate::heuristics::rawspecfeatures::{
    raw_spec_features_classify, raw_spec_features_compute, RawSpecFeatureCell, RAW_DEFAULT_MASK,
};
use crate::heuristics::to_params::{self, OrderParmsCell};
use crate::inout::basicparser::parse_skip_parenthesized_expr;
use crate::inout::commandline::{
    get_bool_arg, get_int_arg, get_int_arg_check_range, print_options, CommandLineState, ParsedOpt,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{
    token_pos_rep, IoFormat, Scanner, TokenType, EMPTY_INCLUDE_SELECTOR_SENTINEL,
};
use crate::inout::signals::{
    configure_time_limits, e_signal_setup, SignalOutcome, RLIM_INFINITY_COMPAT, SIGXCPU_COMPAT,
};
use crate::orderings::cto_lpo::set_lpo_recursion_depth_limit;
use crate::prover::options::{EProverOption, EPROVER_OPTIONS};
use crate::prover::version::{self, E_NICKNAME, PROGRAM_NAME, VERSION};
use crate::terms::signature::{
    FunctionProperties, Signature, FP_IGNORE_PROPS, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT,
    FP_IS_RATIONAL, SIG_ITE_CODE, SIG_LET_CODE,
};
use crate::terms::simpletypes::{type_app_encoded_name, Type};
use crate::terms::subst::Substitution;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_app_encode, term_standard_weight};
use crate::terms::termtypes::{term_identity_id, RewriteLevel, Term};
use crate::terms::typebanks::TypeBank;

const MEGA: u64 = 1_048_576;
const C_INT_MAX: i64 = i32::MAX as i64;
const DEFAULT_CLASSIFICATION_TIMEOUT_PERCENTAGE: i64 = 2;
const DEFAULT_DELETE_BAD_LIMIT: i64 = i64::MAX;
const DEFAULT_EQDEF_INCRLIMIT: i64 = 20;
const DEFAULT_EQDEF_MAXCLAUSES: i64 = 20_000;
const DEFAULT_FORMULA_DEF_LIMIT: i64 = 24;
const DEFAULT_HEURISTIC_NAME: &str = "Default";
const FOF_LOGICAL_SYMBOL_WEIGHT: i64 = 2;
const FOF_QUANTIFIER_BINDER_WEIGHT: i64 = 8;
const SINE_AUTO_MASK: &str = "-aaaaaaa";
const SINE_AUTO_CLASS_LEN: usize = SINE_AUTO_MASK.len();
const TRAINING_PRINT_POS: i64 = 1;
const TRAINING_PRINT_NEG: i64 = 2;
const SINE_AUTO_CLASSES: &[(&str, Option<&str>)] = &[
    ("-LMSSMLL", None),
    ("-MMSSMSL", None),
    ("-LMLSMSL", Some("gf200_h_gu_R03_F100_L20000")),
    ("-MMSSMSM", None),
    ("-MSSMLSM", None),
    ("-LMLSLLL", Some("gf120_h_gu_RUU_F100_L01000")),
    ("-MMSMMSM", None),
    ("-LMMSMLL", Some("gf120_gu_R02_F100_L20000")),
    ("-LLLLMLL", Some("gf500_h_gu_R04_F100_L20000")),
    ("-MSLMMSL", None),
    ("-MSLSMSL", Some("gf500_h_gu_R04_F100_L20000")),
    ("-SSSSLSM", None),
    ("-SSSSLSL", None),
    ("-SSSSMSL", None),
    ("-SSSSMSM", None),
    ("-SSSSMSS", None),
    ("-SMSSMSM", Some("gf200_h_gu_R03_F100_L20000")),
    ("-LSMSMSL", Some("gf200_h_gu_RUU_F100_L20000")),
    ("-SSSSLSS", None),
    ("-LLLSMSL", Some("gf600_h_gu_R05_F100_L20000")),
    ("-MSSSMML", Some("gf120_h_gu_R02_F100_L20000")),
    ("-MSSSMSS", Some("gf600_h_gu_R05_F100_L20000")),
    ("-LMLMMLL", Some("gf120_h_gu_RUU_F100_L00500")),
    ("-LMLMLLL", Some("gf120_h_gu_R02_F100_L20000")),
    ("-LSLSMSL", Some("gf500_h_gu_R04_F100_L20000")),
    ("-MMSSMLL", Some("gf120_h_gu_RUU_F100_L01000")),
    ("-MSSSMSL", Some("gf600_h_gu_R05_F100_L20000")),
    ("-MSSSMSM", None),
    ("-LSSSMSM", None),
    ("-MSMSMSL", None),
    ("-LMLLMSL", None),
    ("-MSSSLSM", None),
    ("-SSSSMLL", None),
    ("-LLLMLLL", Some("gf120_h_gu_RUU_F100_L01000")),
    ("-LMMMMLL", Some("gf200_h_gu_R03_F100_L20000")),
    ("-MSSSMLL", Some("gf120_h_gu_RUU_F100_L00500")),
    ("-MMMSMLL", Some("gf120_h_gu_RUU_F100_L00500")),
];
const DEFAULT_LAMBDA_WEIGHT: i64 = 20;
const DEFAULT_DB_WEIGHT: i64 = 10;
const DEFAULT_LPO_RECURSION_LIMIT: i64 = 1_000;
const LPO_RECURSION_WARNING_LIMIT: i64 = 20_000;
const LPO_RECURSION_LIMIT_WARNING: &str = "Using very large values for --lpo-recursion-limit may lead to stack overflows and segmentation faults.";
const DEFAULT_MAX_UNIFIERS: i64 = 4;

struct IoRunGuard;

impl Drop for IoRunGuard {
    fn drop(&mut self) {
        exit_io();
    }
}

struct ProblemTypeRunGuard;

impl ProblemTypeRunGuard {
    fn new() -> Self {
        reset_problem_type();
        Self
    }
}

impl Drop for ProblemTypeRunGuard {
    fn drop(&mut self) {
        reset_problem_type();
    }
}
const DEFAULT_MAX_UNIF_STEPS: i64 = 256;
const DEFAULT_MINISCOPE_LIMIT: i64 = 1_048_576;
const DEFAULT_OUTPUT_DESCRIPTOR: &str = "eigEIG";
const DEFAULT_SYMBOL_OCCURRENCES: i64 = 512;
const DEFAULT_FILTER_DESCRIPTOR: &str = "Fc";
const NO_HIGHER_ORDER_DEPTH: i64 = -1;
const THF_REQUIRES_HOL_MESSAGE: &str =
    "To support HOL reasoning, recompile E using './configure --enable-ho && make rebuild'";
const TSTP_FORMULA_FREE_VARIABLES_MESSAGE: &str =
    "Formula has free variables (check parentheses and quantifier precedence)";
const WATCHLIST_INLINE_STRING: &str = "Use inline watchlist type";
const WATCHLIST_INLINE_QSTRING: &str = "'Use inline watchlist type'";

const GROUNDING_STRATEGY_NAMES: &[&str] = &[
    "NoGrounding",
    "PseudoVar",
    "FirstConst",
    "ConjMinMinFreq",
    "ConjMaxMinFreq",
    "ConjMinMaxFreq",
    "ConjMaxMaxFreq",
    "GlobalMax",
    "GlobalMin",
];

const FP_INDEX_NAMES: &[&str] = &[
    "FP0", "FPfp", "FP1", "FP2", "FP3D", "FP3W", "FP4D", "FP4W", "FP4M", "FP5M", "FP6M", "FP7",
    "FP7M", "FP4X2_2", "FP3DFlex", "NPDT", "NoIndex",
];

const PRECEDENCE_GENERATION_METHODS: &[&str] = &[
    "none",
    "unary_first",
    "unary_freq",
    "arity",
    "invarity",
    "const_max",
    "const_min",
    "freq",
    "invfreq",
    "invconjfreq",
    "invfreqconjmax",
    "invfreqconjmin",
    "invfreqconstmin",
    "invfreqhack",
    "typefreq",
    "invtypefreq",
    "combfreq",
    "invcombfreq",
    "arrayopt",
    "orient_axioms",
];

const WEIGHT_GENERATION_METHODS: &[&str] = &[
    "none",
    "firstmaximal0",
    "arity",
    "aritymax0",
    "modarity",
    "modaritymax0",
    "aritysquared",
    "aritysquaredmax0",
    "invarity",
    "invaritymax0",
    "invaritysquared",
    "invaritysquaredmax0",
    "precedence",
    "invprecedence",
    "precrank5",
    "precrank10",
    "precrank20",
    "freqcount",
    "invfreqcount",
    "freqrank",
    "invfreqrank",
    "invconjfreqrank",
    "freqranksquare",
    "invfreqranksquare",
    "invmodfreqrank",
    "invmodfreqrankmax0",
    "typefreqrank",
    "typefreqcount",
    "invtypefreqrank",
    "invtypefreqcount",
    "combfreqrank",
    "combfreqcount",
    "invcombfreqrank",
    "invcombfreqcount",
    "constant",
];

const LITERAL_SELECTION_STRATEGIES: &[&str] = &[
    "NoSelection",
    "NoGeneration",
    "SelectNegativeLiterals",
    "PSelectNegativeLiterals",
    "SelectPureVarNegLiterals",
    "PSelectPureVarNegLiterals",
    "SelectLargestNegLit",
    "PSelectLargestNegLit",
    "SelectSmallestNegLit",
    "PSelectSmallestNegLit",
    "SelectLargestOrientable",
    "PSelectLargestOrientable",
    "MSelectLargestOrientable",
    "SelectSmallestOrientable",
    "PSelectSmallestOrientable",
    "MSelectSmallestOrientable",
    "SelectDiffNegLit",
    "PSelectDiffNegLit",
    "SelectGroundNegLit",
    "PSelectGroundNegLit",
    "SelectOptimalLit",
    "PSelectOptimalLit",
    "SelectMinOptimalLit",
    "PSelectMinOptimalLit",
    "SelectMinOptimalNoTypePred",
    "PSelectMinOptimalNoTypePred",
    "SelectMinOptimalNoXTypePred",
    "PSelectMinOptimalNoXTypePred",
    "SelectMinOptimalNoRXTypePred",
    "PSelectMinOptimalNoRXTypePred",
    "SelectCondOptimalLit",
    "PSelectCondOptimalLit",
    "SelectAllCondOptimalLit",
    "PSelectAllCondOptimalLit",
    "SelectOptimalRestrDepth2",
    "PSelectOptimalRestrDepth2",
    "SelectOptimalRestrPDepth2",
    "PSelectOptimalRestrPDepth2",
    "SelectOptimalRestrNDepth2",
    "PSelectOptimalRestrNDepth2",
    "SelectNonRROptimalLit",
    "PSelectNonRROptimalLit",
    "SelectNonStrongRROptimalLit",
    "PSelectNonStrongRROptimalLit",
    "SelectAntiRROptimalLit",
    "PSelectAntiRROptimalLit",
    "SelectNonAntiRROptimalLit",
    "PSelectNonAntiRROptimalLit",
    "SelectStrongRRNonRROptimalLit",
    "PSelectStrongRRNonRROptimalLit",
    "SelectUnlessUniqMax",
    "PSelectUnlessUniqMax",
    "SelectUnlessPosMax",
    "PSelectUnlessPosMax",
    "SelectUnlessUniqPosMax",
    "PSelectUnlessUniqPosMax",
    "SelectUnlessUniqMaxPos",
    "PSelectUnlessUniqMaxPos",
    "SelectComplex",
    "PSelectComplex",
    "SelectComplexExceptRRHorn",
    "PSelectComplexExceptRRHorn",
    "SelectLComplex",
    "PSelectLComplex",
    "SelectMaxLComplex",
    "PSelectMaxLComplex",
    "SelectMaxLComplexNoTypePred",
    "PSelectMaxLComplexNoTypePred",
    "SelectMaxLComplexNoXTypePred",
    "PSelectMaxLComplexNoXTypePred",
    "SelectComplexPreferNEQ",
    "PSelectComplexPreferNEQ",
    "SelectComplexPreferEQ",
    "PSelectComplexPreferEQ",
    "SelectComplexExceptUniqMaxHorn",
    "PSelectComplexExceptUniqMaxHorn",
    "MSelectComplexExceptUniqMaxHorn",
    "SelectNewComplex",
    "PSelectNewComplex",
    "SelectNewComplexExceptUniqMaxHorn",
    "PSelectNewComplexExceptUniqMaxHorn",
    "SelectMinInfpos",
    "PSelectMinInfpos",
    "HSelectMinInfpos",
    "GSelectMinInfpos",
    "SelectMinInfposNoTypePred",
    "PSelectMinInfposNoTypePred",
    "SelectMin2Infpos",
    "PSelectMin2Infpos",
    "SelectComplexExceptUniqMaxPosHorn",
    "PSelectComplexExceptUniqMaxPosHorn",
    "SelectUnlessUniqMaxSmallestOrientable",
    "PSelectUnlessUniqMaxSmallestOrientable",
    "SelectDivLits",
    "SelectDivPreferIntoLits",
    "SelectMaxLComplexG",
    "SelectMaxLComplexAvoidPosPred",
    "SelectMaxLComplexAPPNTNp",
    "SelectMaxLComplexAPPNoType",
    "SelectMaxLComplexAvoidPosUPred",
    "SelectComplexG",
    "SelectComplexAHP",
    "PSelectComplexAHP",
    "SelectNewComplexAHP",
    "PSelectNewComplexAHP",
    "SelectComplexAHPExceptRRHorn",
    "PSelectComplexAHPExceptRRHorn",
    "SelectNewComplexAHPExceptRRHorn",
    "PSelectNewComplexAHPExceptRRHorn",
    "SelectNewComplexAHPExceptUniqMaxHorn",
    "PSelectNewComplexAHPExceptUniqMaxHorn",
    "SelectNewComplexAHPNS",
    "SelectVGNonCR",
    "SelectCQArEqLast",
    "SelectCQArEqFirst",
    "SelectCQIArEqLast",
    "SelectCQIArEqFirst",
    "SelectCQAr",
    "SelectCQIAr",
    "SelectCQArNpEqFirst",
    "SelectCQIArNpEqFirst",
    "SelectGrCQArEqFirst",
    "SelectCQGrArEqFirst",
    "SelectCQArNTEqFirst",
    "SelectCQIArNTEqFirst",
    "SelectCQArNTNpEqFirst",
    "SelectCQIArNTNpEqFirst",
    "SelectCQArNXTEqFirst",
    "SelectCQIArNXTEqFirst",
    "SelectCQArNTNp",
    "SelectCQIArNTNp",
    "SelectCQArNT",
    "SelectCQIArNT",
    "SelectCQArNp",
    "SelectCQIArNp",
    "SelectCQArNpEqFirstUnlessPDom",
    "SelectCQArNTEqFirstUnlessPDom",
    "SelectCQPrecW",
    "SelectCQIPrecW",
    "SelectCQPrecWNTNp",
    "SelectCQIPrecWNTNp",
    "SelectMaxLComplexAvoidAppVar",
    "SelectMaxLComplexStronglyAvoidAppVar",
    "SelectMaxLComplexPreferAppVar",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EProverAction {
    Help,
    Version,
    Run(Box<EProverConfig>),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum DocOutputFormat {
    #[default]
    NoFormat = 0,
    Lop = 1,
    Pcl = 2,
    Tstp = 3,
    Tptp = 4,
    Xml = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EquationPrintConfig {
    pub use_infix: bool,
    pub full_equational_rep: bool,
    pub print_oriented: bool,
}

impl Default for EquationPrintConfig {
    fn default() -> Self {
        Self {
            use_infix: true,
            full_equational_rep: false,
            print_oriented: false,
        }
    }
}

impl EquationPrintConfig {
    #[must_use]
    pub const fn into_eqn_print_options(self, output_format: IoFormat) -> EqnPrintOptions {
        EqnPrintOptions {
            output_format,
            use_infix: self.use_infix,
            full_equational_rep: self.full_equational_rep,
            print_oriented: self.print_oriented,
            higher_order_parentheses: false,
            print_types: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PclOutputConfig {
    pub full_terms: bool,
    pub compact: bool,
    pub shell_level: i64,
}

impl Default for PclOutputConfig {
    fn default() -> Self {
        Self {
            full_terms: true,
            compact: false,
            shell_level: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum AcHandling {
    None = 0,
    #[default]
    DiscardAll = 1,
    KeepUnits = 2,
    KeepOrientable = 3,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GoalDefinitionConfig {
    pub positive: bool,
    pub negative: bool,
    pub subterms: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FoolUnroll {
    #[default]
    Enabled,
    Disabled,
}

impl From<bool> for FoolUnroll {
    fn from(value: bool) -> Self {
        if value {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BceConfig {
    pub enabled: bool,
    pub max_occs: i64,
}

impl Default for BceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_occs: DEFAULT_SYMBOL_OCCURRENCES,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PredicateEliminationFlags {
    bits: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PredicateEliminationFlag {
    RecognizeGates = 1 << 0,
    ForceMuDecrease = 1 << 1,
    IgnoreConjectureSymbols = 1 << 2,
}

impl PredicateEliminationFlags {
    pub fn set(&mut self, flag: PredicateEliminationFlag, value: bool) {
        if value {
            self.bits |= flag as u8;
        } else {
            self.bits &= !(flag as u8);
        }
    }

    #[must_use]
    pub const fn contains(self, flag: PredicateEliminationFlag) -> bool {
        (self.bits & flag as u8) != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PredicateEliminationConfig {
    pub enabled: bool,
    pub max_occs: i64,
    pub tolerance: i64,
    pub flags: PredicateEliminationFlags,
}

impl Default for PredicateEliminationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_occs: DEFAULT_SYMBOL_OCCURRENCES,
            tolerance: 0,
            flags: PredicateEliminationFlags::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreprocessingConfig {
    pub no_preprocessing: bool,
    pub eqdef_maxclauses: i64,
    pub eqdef_incrlimit: i64,
    pub formula_def_limit: i64,
    pub miniscope_limit: i64,
    pub classification_timeout_percentage: i64,
    pub fool_unroll: FoolUnroll,
    pub bce: BceConfig,
    pub predicate_elimination: PredicateEliminationConfig,
    pub goal_definitions: GoalDefinitionConfig,
    pub relevance_prune_level: i64,
    pub presat_interreduction: bool,
    pub ac_handling: AcHandling,
    pub ac_res_aggressive: bool,
}

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            no_preprocessing: false,
            eqdef_maxclauses: DEFAULT_EQDEF_MAXCLAUSES,
            eqdef_incrlimit: DEFAULT_EQDEF_INCRLIMIT,
            formula_def_limit: DEFAULT_FORMULA_DEF_LIMIT,
            miniscope_limit: DEFAULT_MINISCOPE_LIMIT,
            classification_timeout_percentage: DEFAULT_CLASSIFICATION_TIMEOUT_PERCENTAGE,
            fool_unroll: FoolUnroll::Enabled,
            bce: BceConfig::default(),
            predicate_elimination: PredicateEliminationConfig::default(),
            goal_definitions: GoalDefinitionConfig::default(),
            relevance_prune_level: 0,
            presat_interreduction: false,
            ac_handling: AcHandling::DiscardAll,
            ac_res_aggressive: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum ParamodulationType {
    #[default]
    Plain = 0,
    Sim = 1,
    OrientedSim = 2,
    SuperSim = 3,
    OrientedSuperSim = 4,
    DecreasingSim = 5,
    SizeDecreasingSim = 6,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum GroundingStrategy {
    #[default]
    NoGrounding = 0,
    PseudoVar = 1,
    FirstConst = 2,
    ConjMinMinFreq = 3,
    ConjMaxMinFreq = 4,
    ConjMinMaxFreq = 5,
    ConjMaxMaxFreq = 6,
    GlobalMax = 7,
    GlobalMin = 8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum FvIndexFeatureType {
    NoFeatures = 0,
    AcFeatures = 1,
    SsFeatures = 2,
    AllFeatures = 3,
    BillFeatures = 4,
    BillPlusFeatures = 5,
    #[default]
    AcFold = 6,
    AcStagger = 7,
    CollectFeatures = 8,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum ExtInferenceType {
    AllLits = 0,
    MaxLits = 1,
    #[default]
    NoLits = 2,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum TermOrdering {
    NoOrdering = 0,
    Optimize = 1,
    Kbo = 2,
    #[default]
    Kbo6 = 3,
    Lpo = 4,
    LpoCopy = 5,
    Lpo4 = 6,
    Lpo4Copy = 7,
    Rpo = 8,
    Empty = 9,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum LiteralComparison {
    None = 0,
    #[default]
    Normal = 1,
    TfoEqMax = 2,
    TfoEqMin = 3,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PrecedenceModifierConfig {
    pub conjecture_only: i64,
    pub conjecture_axiom: i64,
    pub axiom_only: i64,
    pub skolem: i64,
    pub defpred: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TermOrderingConfig {
    pub ordering: TermOrdering,
    pub weight_generation: String,
    pub precedence_generation: String,
    pub precedence_modifiers: PrecedenceModifierConfig,
    pub weight_overrides: Option<String>,
    pub constant_weight: i64,
    pub precedence: Option<String>,
    pub lpo_recursion_limit: i64,
    pub lpo_recursion_limit_changed: bool,
    pub literal_comparison: LiteralComparison,
    pub lambda_weight: i64,
    pub db_weight: i64,
    pub ho_order_kind: HoOrderKind,
    pub rewrite_strong_rhs_inst: bool,
}

impl Default for TermOrderingConfig {
    fn default() -> Self {
        Self {
            ordering: TermOrdering::Kbo6,
            weight_generation: "none".to_owned(),
            precedence_generation: "none".to_owned(),
            precedence_modifiers: PrecedenceModifierConfig::default(),
            weight_overrides: None,
            constant_weight: 0,
            precedence: None,
            lpo_recursion_limit: DEFAULT_LPO_RECURSION_LIMIT,
            lpo_recursion_limit_changed: false,
            literal_comparison: LiteralComparison::Normal,
            lambda_weight: DEFAULT_LAMBDA_WEIGHT,
            db_weight: DEFAULT_DB_WEIGHT,
            ho_order_kind: HoOrderKind::LfhoOrder,
            rewrite_strong_rhs_inst: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeuristicConfig {
    pub name: String,
    pub weight_function_definitions: Vec<String>,
    pub heuristic_definitions: Vec<String>,
    pub prefer_initial_clauses: bool,
    pub filter_orphans_limit: i64,
    pub forward_contract_limit: i64,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self {
            name: DEFAULT_HEURISTIC_NAME.to_owned(),
            weight_function_definitions: Vec::new(),
            heuristic_definitions: Vec::new(),
            prefer_initial_clauses: false,
            filter_orphans_limit: i64::MAX,
            forward_contract_limit: i64::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LiteralSelectionLimits {
    pub positive_min: i64,
    pub positive_max: i64,
    pub negative_min: i64,
    pub negative_max: i64,
    pub all_min: i64,
    pub all_max: i64,
    pub weight_min: i64,
}

impl Default for LiteralSelectionLimits {
    fn default() -> Self {
        Self {
            positive_min: 0,
            positive_max: i64::MAX,
            negative_min: 0,
            negative_max: i64::MAX,
            all_min: 0,
            all_max: i64::MAX,
            weight_min: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ParamodLiteralInheritanceConfig {
    pub any_clause: bool,
    pub goal_clause: bool,
    pub conjecture_clause: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiteralSelectionConfig {
    pub strategy: String,
    pub limits: LiteralSelectionLimits,
    pub select_on_processing_only: bool,
    pub inherit_paramod_literals: ParamodLiteralInheritanceConfig,
}

impl Default for LiteralSelectionConfig {
    fn default() -> Self {
        Self {
            strategy: "NoSelection".to_owned(),
            limits: LiteralSelectionLimits::default(),
            select_on_processing_only: false,
            inherit_paramod_literals: ParamodLiteralInheritanceConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CondensingConfig {
    pub enabled: bool,
    pub aggressive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DemodulationConfig {
    pub forward_demod: RewriteLevel,
    pub lambda_demod: bool,
    pub prefer_general: bool,
}

impl Default for DemodulationConfig {
    fn default() -> Self {
        Self {
            forward_demod: RewriteLevel::FullRewrite,
            lambda_demod: false,
            prefer_general: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ContextSimplificationConfig {
    pub forward: bool,
    pub forward_aggressive: bool,
    pub backward: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EqualityResolutionConfig {
    pub destructive: bool,
    pub strong_destructive: bool,
    pub aggressive: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SubsumptionConfig {
    pub forward_aggressive: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HigherOrderInferenceConfig {
    pub arg_cong: ExtInferenceType,
    pub neg_ext: ExtInferenceType,
    pub pos_ext: ExtInferenceType,
    pub ext_rules_max_depth: i64,
    pub inverse_recognition: bool,
    pub replace_inj_defs: bool,
}

impl Default for HigherOrderInferenceConfig {
    fn default() -> Self {
        Self {
            arg_cong: ExtInferenceType::AllLits,
            neg_ext: ExtInferenceType::NoLits,
            pos_ext: ExtInferenceType::NoLits,
            ext_rules_max_depth: NO_HIGHER_ORDER_DEPTH,
            inverse_recognition: false,
            replace_inj_defs: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EtaNormalization {
    #[default]
    Reduce,
    Expand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HigherOrderPreprocessingConfig {
    pub eta_normalization: EtaNormalization,
    pub lambda_to_forall: bool,
    pub unroll_only_formulas: bool,
    pub elim_leibniz_max_depth: i64,
    pub inst_choice_max_depth: i64,
    pub preinstantiate_induction: bool,
}

impl Default for HigherOrderPreprocessingConfig {
    fn default() -> Self {
        Self {
            eta_normalization: EtaNormalization::Reduce,
            lambda_to_forall: true,
            unroll_only_formulas: true,
            elim_leibniz_max_depth: NO_HIGHER_ORDER_DEPTH,
            inst_choice_max_depth: NO_HIGHER_ORDER_DEPTH,
            preinstantiate_induction: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum PrimEnumMode {
    Neg = 0,
    And = 1,
    Or = 2,
    Eq = 3,
    #[default]
    Pragmatic = 4,
    Full = 5,
    LogSymbol = 6,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrimitiveEnumerationConfig {
    pub mode: PrimEnumMode,
    pub max_depth: i64,
}

impl Default for PrimitiveEnumerationConfig {
    fn default() -> Self {
        Self {
            mode: PrimEnumMode::Pragmatic,
            max_depth: NO_HIGHER_ORDER_DEPTH,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HigherOrderSearchConfig {
    pub primitive_enumeration: PrimitiveEnumerationConfig,
    pub local_rw: bool,
    pub prune_args: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum UnificationMode {
    #[default]
    Single = 0,
    Multi = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HigherOrderUnificationConfig {
    pub func_proj_limit: i64,
    pub imit_limit: i64,
    pub ident_limit: i64,
    pub elim_limit: i64,
    pub mode: UnificationMode,
    pub pattern_oracle: bool,
    pub fixpoint_oracle: bool,
    pub max_unifiers: i64,
    pub max_unif_steps: i64,
}

impl Default for HigherOrderUnificationConfig {
    fn default() -> Self {
        Self {
            func_proj_limit: 0,
            imit_limit: 0,
            ident_limit: 0,
            elim_limit: 0,
            mode: UnificationMode::Single,
            pattern_oracle: true,
            fixpoint_oracle: true,
            max_unifiers: DEFAULT_MAX_UNIFIERS,
            max_unif_steps: DEFAULT_MAX_UNIF_STEPS,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InferenceConfig {
    pub enable_eq_factoring: bool,
    pub enable_neg_unit_paramod: bool,
    pub enable_given_forward_simplification: bool,
    pub paramodulation: ParamodulationType,
    pub condensing: CondensingConfig,
    pub demodulation: DemodulationConfig,
    pub context_simplification: ContextSimplificationConfig,
    pub equality_resolution: EqualityResolutionConfig,
    pub subsumption: SubsumptionConfig,
    pub higher_order: HigherOrderInferenceConfig,
    pub higher_order_preprocessing: HigherOrderPreprocessingConfig,
    pub higher_order_search: HigherOrderSearchConfig,
    pub higher_order_unification: HigherOrderUnificationConfig,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            enable_eq_factoring: true,
            enable_neg_unit_paramod: true,
            enable_given_forward_simplification: true,
            paramodulation: ParamodulationType::Plain,
            condensing: CondensingConfig::default(),
            demodulation: DemodulationConfig::default(),
            context_simplification: ContextSimplificationConfig::default(),
            equality_resolution: EqualityResolutionConfig::default(),
            subsumption: SubsumptionConfig::default(),
            higher_order: HigherOrderInferenceConfig::default(),
            higher_order_preprocessing: HigherOrderPreprocessingConfig::default(),
            higher_order_search: HigherOrderSearchConfig::default(),
            higher_order_unification: HigherOrderUnificationConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompletenessConfig {
    pub inference_system_complete: bool,
    pub assume_inference_system_complete: bool,
    pub incomplete: bool,
}

impl Default for CompletenessConfig {
    fn default() -> Self {
        Self {
            inference_system_complete: true,
            assume_inference_system_complete: false,
            incomplete: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SplittingConfig {
    pub classes: i64,
    pub method: i64,
    pub aggressive: bool,
    pub fresh_defs: bool,
    pub diseq_decomposition: i64,
    pub diseq_decomp_maxarity: i64,
}

impl Default for SplittingConfig {
    fn default() -> Self {
        Self {
            classes: 0,
            method: 0,
            aggressive: false,
            fresh_defs: true,
            diseq_decomposition: 0,
            diseq_decomp_maxarity: i64::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SearchSupportConfig {
    pub use_tptp_sos: bool,
    pub lift_lambdas: bool,
    pub strong_unit_forward_subsumption: bool,
}

impl Default for SearchSupportConfig {
    fn default() -> Self {
        Self {
            use_tptp_sos: false,
            lift_lambdas: true,
            strong_unit_forward_subsumption: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SatCheckConfig {
    pub grounding: GroundingStrategy,
    pub step_limit: i64,
    pub size_limit: i64,
    pub ttinsert_limit: i64,
    pub normconst: bool,
    pub normalize: bool,
    pub decision_limit: i64,
}

impl Default for SatCheckConfig {
    fn default() -> Self {
        Self {
            grounding: GroundingStrategy::NoGrounding,
            step_limit: i64::MAX,
            size_limit: i64::MAX,
            ttinsert_limit: i64::MAX,
            normconst: false,
            normalize: false,
            decision_limit: 10_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WatchlistSource {
    Inline,
    File(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchlistConfig {
    pub source: Option<WatchlistSource>,
    pub simplify: bool,
    pub is_static: bool,
}

impl Default for WatchlistConfig {
    fn default() -> Self {
        Self {
            source: None,
            simplify: true,
            is_static: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureVectorIndexConfig {
    pub feature_type: FvIndexFeatureType,
    pub use_perm_vectors: bool,
    pub eliminate_uninformative: bool,
    pub max_symbols: i64,
    pub symbol_slack: i64,
}

impl Default for FeatureVectorIndexConfig {
    fn default() -> Self {
        Self {
            feature_type: FvIndexFeatureType::AcFold,
            use_perm_vectors: false,
            eliminate_uninformative: false,
            max_symbols: 17,
            symbol_slack: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FingerprintIndexConfig {
    pub rw_bw_index_type: String,
    pub pm_from_index_type: String,
    pub pm_into_index_type: String,
    pub pdt_use_size_constraints: bool,
    pub pdt_use_age_constraints: bool,
}

impl Default for FingerprintIndexConfig {
    fn default() -> Self {
        Self {
            rw_bw_index_type: "FP7".to_owned(),
            pm_from_index_type: "FP7".to_owned(),
            pm_into_index_type: "FP7".to_owned(),
            pdt_use_size_constraints: true,
            pdt_use_age_constraints: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EncodingConfig {
    pub print_types: bool,
    pub app_encode: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchControlConfig {
    pub ordering: TermOrderingConfig,
    pub heuristic: HeuristicConfig,
    pub literal_selection: LiteralSelectionConfig,
    pub inference: InferenceConfig,
    pub completeness: CompletenessConfig,
    pub splitting: SplittingConfig,
    pub support: SearchSupportConfig,
    pub sat_check: SatCheckConfig,
    pub watchlist: WatchlistConfig,
    pub fv_index: FeatureVectorIndexConfig,
    pub fingerprint_index: FingerprintIndexConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EProverConfig {
    pub warnings: Vec<Diagnostic>,
    pub explicit_options: Vec<EProverOption>,
    pub files: Vec<String>,
    pub output_file: Option<String>,
    pub output_level: i64,
    pub verbose: i64,
    pub proof_object_level: i64,
    pub proof_output: i64,
    pub force_derivation_output: i64,
    pub training_examples: Option<i64>,
    pub parse_format: IoFormat,
    pub output_format: IoFormat,
    pub doc_output_format: DocOutputFormat,
    pub equation_print: EquationPrintConfig,
    pub pcl_output: PclOutputConfig,
    pub free_symbol_properties: FunctionProperties,
    pub encoding: EncodingConfig,
    pub saturated_output_descriptor: String,
    pub filter_saturated_descriptor: String,
    pub select_strategy: Option<String>,
    pub print_strategy: Option<String>,
    pub parse_strategy_file: Option<String>,
    pub sine: Option<String>,
    pub preprocessing: PreprocessingConfig,
    pub search: SearchControlConfig,
    pub strategy_scheduling: bool,
    pub schedule_cores: i64,
    pub serialize_schedule: bool,
    pub force_preprocessing_schedule: bool,
    pub step_limit: i64,
    pub answer_limit: i64,
    pub processed_set_limit: i64,
    pub unprocessed_limit: i64,
    pub total_clause_set_limit: i64,
    pub generated_limit: i64,
    pub term_bank_insert_limit: i64,
    pub cpu_limit: Option<i64>,
    pub soft_cpu_limit: Option<i64>,
    pub schedule_time_limit: Option<i64>,
    pub memory_limit: u64,
    pub delete_bad_limit: i64,
    pub flags: EProverFlags,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EProverFlags {
    bits: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EProverFlag {
    SyntaxOnly = 1 << 0,
    PrintPid = 1 << 1,
    PrintVersion = 1 << 2,
    Auto = 1 << 3,
    DeterministicRewriteSort = 1 << 4,
    DeterministicNewSort = 1 << 5,
    PrintFormulas = 1 << 6,
    PruneOnly = 1 << 7,
    CnfOnly = 1 << 8,
    RequireNonempty = 1 << 9,
    ProofStatistics = 1 << 10,
    FullDerivation = 1 << 11,
    RecordGivenClauses = 1 << 12,
    PrintStatistics = 1 << 13,
    PrintDetailedStatistics = 1 << 14,
    PrintSaturated = 1 << 15,
    PrintSaturatedInfo = 1 << 16,
    FilterSaturated = 1 << 17,
    ResourceInfo = 1 << 18,
    ConjecturesAreQuestions = 1 << 19,
    FormulaConjectureSeen = 1 << 20,
}

impl EProverFlags {
    pub fn set(&mut self, flag: EProverFlag) {
        self.bits |= flag as u32;
    }

    pub fn clear(&mut self, flag: EProverFlag) {
        self.bits &= !(flag as u32);
    }

    #[must_use]
    pub const fn contains(self, flag: EProverFlag) -> bool {
        (self.bits & flag as u32) != 0
    }
}

impl Default for EProverConfig {
    fn default() -> Self {
        Self {
            warnings: Vec::new(),
            explicit_options: Vec::new(),
            files: Vec::new(),
            output_file: None,
            output_level: 1,
            verbose: 0,
            proof_object_level: 0,
            proof_output: 0,
            force_derivation_output: 0,
            training_examples: None,
            parse_format: IoFormat::Auto,
            output_format: IoFormat::Lop,
            doc_output_format: DocOutputFormat::NoFormat,
            equation_print: EquationPrintConfig::default(),
            pcl_output: PclOutputConfig::default(),
            free_symbol_properties: FP_IGNORE_PROPS,
            encoding: EncodingConfig::default(),
            saturated_output_descriptor: DEFAULT_OUTPUT_DESCRIPTOR.to_owned(),
            filter_saturated_descriptor: DEFAULT_FILTER_DESCRIPTOR.to_owned(),
            select_strategy: None,
            print_strategy: None,
            parse_strategy_file: None,
            sine: None,
            preprocessing: PreprocessingConfig::default(),
            search: SearchControlConfig::default(),
            strategy_scheduling: false,
            schedule_cores: 1,
            serialize_schedule: false,
            force_preprocessing_schedule: true,
            step_limit: i64::MAX,
            answer_limit: 1,
            processed_set_limit: i64::MAX,
            unprocessed_limit: i64::MAX,
            total_clause_set_limit: i64::MAX,
            generated_limit: i64::MAX,
            term_bank_insert_limit: i64::MAX,
            cpu_limit: None,
            soft_cpu_limit: None,
            schedule_time_limit: None,
            memory_limit: 0,
            delete_bad_limit: DEFAULT_DELETE_BAD_LIMIT,
            flags: EProverFlags::default(),
        }
    }
}

/// Builds the C-shaped ordering parameter cell from parsed executable options.
///
/// # Errors
///
/// Returns a diagnostic if manually constructed config values do not match
/// known C ordering names.
pub fn order_parms_from_config(config: &TermOrderingConfig) -> Result<OrderParmsCell, Diagnostic> {
    let weight_generation = translate_weight_generation(&config.weight_generation)?;
    let precedence_generation = translate_precedence_generation(&config.precedence_generation)?;
    let modifiers = config.precedence_modifiers;

    Ok(OrderParmsCell {
        ordertype: to_params_term_ordering(config.ordering),
        to_weight_gen: weight_generation,
        to_prec_gen: precedence_generation,
        conj_only_mod: modifiers.conjecture_only,
        conj_axiom_mod: modifiers.conjecture_axiom,
        axiom_only_mod: modifiers.axiom_only,
        skolem_mod: modifiers.skolem,
        defpred_mod: modifiers.defpred,
        force_kbo_var_weight: false,
        rewrite_strong_rhs_inst: config.rewrite_strong_rhs_inst,
        to_pre_prec: config.precedence.clone(),
        to_pre_weights: config.weight_overrides.clone(),
        to_const_weight: config.constant_weight,
        to_defs_min: false,
        lit_cmp: i64::from(to_params_literal_cmp(config.literal_comparison).c_value()),
        ho_order_kind: config.ho_order_kind,
        lam_w: config.lambda_weight,
        db_w: config.db_weight,
    })
}

/// Builds the C-shaped heuristic parameter cell from parsed executable options.
///
/// # Errors
///
/// Returns a diagnostic if manually constructed config values cannot be
/// represented by the C-width fields expected by `HeuristicParmsCell`.
#[expect(
    clippy::too_many_lines,
    reason = "C executable option state maps one-for-one into HeuristicParmsCell fields"
)]
pub fn heuristic_parms_from_config(
    config: &EProverConfig,
) -> Result<HeuristicParmsCell, Diagnostic> {
    let preprocessing = &config.preprocessing;
    let search = &config.search;
    let heuristic = &search.heuristic;
    let literal_selection = &search.literal_selection;
    let inference = &search.inference;
    let ho_inference = &inference.higher_order;
    let ho_preprocessing = &inference.higher_order_preprocessing;
    let ho_search = &inference.higher_order_search;
    let ho_unification = &inference.higher_order_unification;
    let pred_elim = &preprocessing.predicate_elimination;
    let splitting = &search.splitting;
    let sat_check = &search.sat_check;
    let fingerprint_index = &search.fingerprint_index;

    Ok(HeuristicParmsCell {
        order_params: order_parms_from_config(&search.ordering)?,
        no_preproc: preprocessing.no_preprocessing,
        eqdef_maxclauses: preprocessing.eqdef_maxclauses,
        eqdef_incrlimit: preprocessing.eqdef_incrlimit,
        formula_def_limit: preprocessing.formula_def_limit,
        miniscope_limit: preprocessing.miniscope_limit,
        sine: config.sine.clone(),
        add_goal_defs_pos: preprocessing.goal_definitions.positive,
        add_goal_defs_neg: preprocessing.goal_definitions.negative,
        add_goal_defs_subterms: preprocessing.goal_definitions.subterms,
        bce: preprocessing.bce.enabled,
        bce_max_occs: i32_from_i64_config("bce_max_occs", preprocessing.bce.max_occs)?,
        pred_elim: pred_elim.enabled,
        pred_elim_gates: pred_elim
            .flags
            .contains(PredicateEliminationFlag::RecognizeGates),
        pred_elim_max_occs: i32_from_i64_config("pred_elim_max_occs", pred_elim.max_occs)?,
        pred_elim_tolerance: i32_from_i64_config("pred_elim_tolerance", pred_elim.tolerance)?,
        pred_elim_force_mu_decrease: pred_elim
            .flags
            .contains(PredicateEliminationFlag::ForceMuDecrease),
        pred_elim_ignore_conj_syms: pred_elim
            .flags
            .contains(PredicateEliminationFlag::IgnoreConjectureSymbols),
        heuristic_name: heuristic.name.clone(),
        heuristic_def: heuristic.heuristic_definitions.last().cloned(),
        prefer_initial_clauses: heuristic.prefer_initial_clauses,
        selection_strategy: literal_selection.strategy.clone(),
        pos_lit_sel_min: literal_selection.limits.positive_min,
        pos_lit_sel_max: literal_selection.limits.positive_max,
        neg_lit_sel_min: literal_selection.limits.negative_min,
        neg_lit_sel_max: literal_selection.limits.negative_max,
        all_lit_sel_min: literal_selection.limits.all_min,
        all_lit_sel_max: literal_selection.limits.all_max,
        weight_sel_min: literal_selection.limits.weight_min,
        select_on_proc_only: literal_selection.select_on_processing_only,
        inherit_paramod_lit: literal_selection.inherit_paramod_literals.any_clause,
        inherit_goal_pm_lit: literal_selection.inherit_paramod_literals.goal_clause,
        inherit_conj_pm_lit: literal_selection.inherit_paramod_literals.conjecture_clause,
        enable_eq_factoring: inference.enable_eq_factoring,
        enable_neg_unit_paramod: inference.enable_neg_unit_paramod,
        enable_given_forward_simpl: inference.enable_given_forward_simplification,
        pm_type: hcb_paramodulation_type(inference.paramodulation),
        ac_handling: hcb_ac_handling(preprocessing.ac_handling),
        ac_res_aggressive: preprocessing.ac_res_aggressive,
        forward_context_sr: inference.context_simplification.forward,
        forward_context_sr_aggressive: inference.context_simplification.forward_aggressive,
        backward_context_sr: inference.context_simplification.backward,
        forward_subsumption_aggressive: inference.subsumption.forward_aggressive,
        forward_demod: inference.demodulation.forward_demod,
        prefer_general: inference.demodulation.prefer_general,
        lambda_demod: inference.demodulation.lambda_demod,
        condensing: inference.condensing.enabled,
        condensing_aggressive: inference.condensing.aggressive,
        er_varlit_destructive: inference.equality_resolution.destructive,
        er_strong_destructive: inference.equality_resolution.strong_destructive,
        er_aggressive: inference.equality_resolution.aggressive,
        split_clauses: hcb_split_class_type(splitting.classes)?,
        split_method: hcb_split_type(splitting.method)?,
        split_aggressive: splitting.aggressive,
        split_fresh_defs: splitting.fresh_defs,
        diseq_decomposition: splitting.diseq_decomposition,
        diseq_decomp_maxarity: splitting.diseq_decomp_maxarity,
        rw_bw_index_type: fingerprint_index.rw_bw_index_type.clone(),
        pm_from_index_type: fingerprint_index.pm_from_index_type.clone(),
        pm_into_index_type: fingerprint_index.pm_into_index_type.clone(),
        sat_check_grounding: hcb_grounding_strategy(sat_check.grounding),
        sat_check_step_limit: sat_check.step_limit,
        sat_check_size_limit: sat_check.size_limit,
        sat_check_ttinsert_limit: sat_check.ttinsert_limit,
        sat_check_normconst: sat_check.normconst,
        sat_check_normalize: sat_check.normalize,
        sat_check_decision_limit: i32_from_i64_config(
            "sat_check_decision_limit",
            sat_check.decision_limit,
        )?,
        filter_orphans_limit: heuristic.filter_orphans_limit,
        forward_contract_limit: heuristic.forward_contract_limit,
        delete_bad_limit: config.delete_bad_limit,
        mem_limit: config.memory_limit,
        watchlist_simplify: search.watchlist.simplify,
        watchlist_is_static: search.watchlist.is_static,
        use_tptp_sos: search.support.use_tptp_sos,
        presat_interreduction: preprocessing.presat_interreduction,
        detsort_bw_rw: config.flags.contains(EProverFlag::DeterministicRewriteSort),
        detsort_tmpset: config.flags.contains(EProverFlag::DeterministicNewSort),
        arg_cong: hcb_ext_inference_type(ho_inference.arg_cong),
        neg_ext: hcb_ext_inference_type(ho_inference.neg_ext),
        pos_ext: hcb_ext_inference_type(ho_inference.pos_ext),
        ext_rules_max_depth: i32_from_i64_config(
            "ext_rules_max_depth",
            ho_inference.ext_rules_max_depth,
        )?,
        inverse_recognition: ho_inference.inverse_recognition,
        replace_inj_defs: ho_inference.replace_inj_defs,
        lift_lambdas: search.support.lift_lambdas,
        lambda_to_forall: ho_preprocessing.lambda_to_forall,
        unroll_only_formulas: ho_preprocessing.unroll_only_formulas,
        elim_leibniz_max_depth: i32_from_i64_config(
            "elim_leibniz_max_depth",
            ho_preprocessing.elim_leibniz_max_depth,
        )?,
        prim_enum_mode: hcb_prim_enum_mode(ho_search.primitive_enumeration.mode),
        prim_enum_max_depth: i32_from_i64_config(
            "prim_enum_max_depth",
            ho_search.primitive_enumeration.max_depth,
        )?,
        inst_choice_max_depth: i32_from_i64_config(
            "inst_choice_max_depth",
            ho_preprocessing.inst_choice_max_depth,
        )?,
        local_rw: ho_search.local_rw,
        prune_args: ho_search.prune_args,
        preinstantiate_induction: ho_preprocessing.preinstantiate_induction,
        fool_unroll: matches!(preprocessing.fool_unroll, FoolUnroll::Enabled),
        func_proj_limit: i32_from_i64_config("func_proj_limit", ho_unification.func_proj_limit)?,
        imit_limit: i32_from_i64_config("imit_limit", ho_unification.imit_limit)?,
        ident_limit: i32_from_i64_config("ident_limit", ho_unification.ident_limit)?,
        elim_limit: i32_from_i64_config("elim_limit", ho_unification.elim_limit)?,
        unif_mode: hcb_unif_mode(ho_unification.mode),
        pattern_oracle: ho_unification.pattern_oracle,
        fixpoint_oracle: ho_unification.fixpoint_oracle,
        max_unifiers: i32_from_i64_config("max_unifiers", ho_unification.max_unifiers)?,
        max_unif_steps: i32_from_i64_config("max_unif_steps", ho_unification.max_unif_steps)?,
    })
}

/// Builds the initial proof-control object from parsed executable options.
///
/// # Errors
///
/// Returns a diagnostic if manually constructed config values cannot be
/// represented by the C-shaped proof-control parameter fields.
pub fn proof_control_from_config(config: &EProverConfig) -> Result<ProofControl, Diagnostic> {
    proof_control_from_heuristic_parms(config, heuristic_parms_with_strategy_io(config)?)
}

fn proof_control_from_heuristic_parms(
    config: &EProverConfig,
    params: HeuristicParmsCell,
) -> Result<ProofControl, Diagnostic> {
    let mut control = ProofControl::new();
    control.set_heuristic_parms(params);
    control.set_fvi_parms(fv_index_params_from_config(&config.search.fv_index)?);
    control.set_record_gc_selection(config.flags.contains(EProverFlag::RecordGivenClauses));
    control
        .set_strong_unit_forward_subsumption(config.search.support.strong_unit_forward_subsumption);
    Ok(control)
}

fn heuristic_parms_with_strategy_io(
    config: &EProverConfig,
) -> Result<HeuristicParmsCell, Diagnostic> {
    let mut params = heuristic_parms_from_config(config)?;
    apply_strategy_io_to_params(config, &mut params)?;
    Ok(params)
}

fn apply_strategy_io_to_params(
    config: &EProverConfig,
    params: &mut HeuristicParmsCell,
) -> Result<(), Diagnostic> {
    if let Some(path) = &config.parse_strategy_file {
        let mut scanner = Scanner::from_file(Path::new(path), true)?;
        heuristic_parms_parse_into(&mut scanner, params, true)?;
        scanner.check_tok(TokenType::NO_TOKEN)?;
    }
    if let Some(name) = &config.select_strategy {
        get_heuristic_with_name(name, params)?;
    }
    Ok(())
}

fn overlay_explicit_heuristic_options(
    config: &EProverConfig,
    params: &mut HeuristicParmsCell,
) -> Result<(), Diagnostic> {
    let cli = heuristic_parms_from_config(config)?;
    let has = |option| config.explicit_options.contains(&option);

    if has(EProverOption::MemoryLimit) {
        params.mem_limit = cli.mem_limit;
        params.delete_bad_limit = cli.delete_bad_limit;
    }
    if has(EProverOption::DeleteBadLimit) {
        params.delete_bad_limit = cli.delete_bad_limit;
    }
    if has(EProverOption::NoPreprocessing) {
        params.no_preproc = cli.no_preproc;
    }
    if has(EProverOption::EqUnfoldLimit) || has(EProverOption::NoEqUnfolding) {
        params.eqdef_incrlimit = cli.eqdef_incrlimit;
    }
    if has(EProverOption::EqUnfoldMaxClauses) {
        params.eqdef_maxclauses = cli.eqdef_maxclauses;
    }
    if has(EProverOption::GoalDefs) {
        params.add_goal_defs_pos = cli.add_goal_defs_pos;
        params.add_goal_defs_neg = cli.add_goal_defs_neg;
    }
    if has(EProverOption::GoalSubtermDefs) {
        params.add_goal_defs_subterms = cli.add_goal_defs_subterms;
    }
    if has(EProverOption::Sine) {
        params.sine.clone_from(&cli.sine);
    }
    if has(EProverOption::PresatSimplify) {
        params.presat_interreduction = cli.presat_interreduction;
    }
    if has(EProverOption::AcHandling) {
        params.ac_handling = cli.ac_handling;
    }
    if has(EProverOption::AcNonAggressive) {
        params.ac_res_aggressive = cli.ac_res_aggressive;
    }
    if has(EProverOption::DefinitionalCnf) {
        params.formula_def_limit = cli.formula_def_limit;
    }
    if has(EProverOption::MiniscopeLimit) {
        params.miniscope_limit = cli.miniscope_limit;
    }
    if has(EProverOption::FoolUnroll) {
        params.fool_unroll = cli.fool_unroll;
    }
    if has(EProverOption::Bce) {
        params.bce = cli.bce;
    }
    if has(EProverOption::BceMaxOccs) {
        params.bce_max_occs = cli.bce_max_occs;
    }
    if has(EProverOption::PredElim) {
        params.pred_elim = cli.pred_elim;
    }
    if has(EProverOption::PredElimRecognizeGates) {
        params.pred_elim_gates = cli.pred_elim_gates;
    }
    if has(EProverOption::PredElimForceMuDecrease) {
        params.pred_elim_force_mu_decrease = cli.pred_elim_force_mu_decrease;
    }
    if has(EProverOption::PredElimIgnoreConjSyms) {
        params.pred_elim_ignore_conj_syms = cli.pred_elim_ignore_conj_syms;
    }
    if has(EProverOption::PredElimMaxOccs) {
        params.pred_elim_max_occs = cli.pred_elim_max_occs;
    }
    if has(EProverOption::PredElimTolerance) {
        params.pred_elim_tolerance = cli.pred_elim_tolerance;
    }

    overlay_explicit_order_options(config, params, &cli);
    overlay_explicit_selection_options(config, params, &cli);
    overlay_explicit_inference_options(config, params, &cli);
    overlay_explicit_index_and_split_options(config, params, &cli);
    overlay_explicit_higher_order_options(config, params, &cli);
    Ok(())
}

fn overlay_explicit_order_options(
    config: &EProverConfig,
    params: &mut HeuristicParmsCell,
    cli: &HeuristicParmsCell,
) {
    let has = |option| config.explicit_options.contains(&option);
    if has(EProverOption::TermOrdering) {
        params.order_params.ordertype = cli.order_params.ordertype;
    }
    if has(EProverOption::OrderWeightGeneration) {
        params.order_params.to_weight_gen = cli.order_params.to_weight_gen;
    }
    if has(EProverOption::OrderWeights) {
        params
            .order_params
            .to_pre_weights
            .clone_from(&cli.order_params.to_pre_weights);
    }
    if has(EProverOption::OrderPrecedenceGeneration) {
        params.order_params.to_prec_gen = cli.order_params.to_prec_gen;
    }
    if has(EProverOption::PrecPureConj) {
        params.order_params.conj_only_mod = cli.order_params.conj_only_mod;
    }
    if has(EProverOption::PrecConjAxiom) {
        params.order_params.conj_axiom_mod = cli.order_params.conj_axiom_mod;
    }
    if has(EProverOption::PrecPureAxiom) {
        params.order_params.axiom_only_mod = cli.order_params.axiom_only_mod;
    }
    if has(EProverOption::PrecSkolem) {
        params.order_params.skolem_mod = cli.order_params.skolem_mod;
    }
    if has(EProverOption::PrecDefPred) {
        params.order_params.defpred_mod = cli.order_params.defpred_mod;
    }
    if has(EProverOption::OrderConstantWeight) {
        params.order_params.to_const_weight = cli.order_params.to_const_weight;
    }
    if has(EProverOption::Precedence) {
        params
            .order_params
            .to_pre_prec
            .clone_from(&cli.order_params.to_pre_prec);
    }
    if has(EProverOption::RestrictLiteralComparisons)
        || has(EProverOption::LiteralComparison)
        || has(EProverOption::LpoRecursionLimit)
    {
        params.order_params.lit_cmp = cli.order_params.lit_cmp;
    }
    if has(EProverOption::KboLambdaWeight) {
        params.order_params.lam_w = cli.order_params.lam_w;
    }
    if has(EProverOption::KboDbWeight) {
        params.order_params.db_w = cli.order_params.db_w;
    }
    if has(EProverOption::StrongRwInst) {
        params.order_params.rewrite_strong_rhs_inst = cli.order_params.rewrite_strong_rhs_inst;
    }
    if has(EProverOption::HoOrderKind) {
        params.order_params.ho_order_kind = cli.order_params.ho_order_kind;
    }
}

fn overlay_explicit_selection_options(
    config: &EProverConfig,
    params: &mut HeuristicParmsCell,
    cli: &HeuristicParmsCell,
) {
    let has = |option| config.explicit_options.contains(&option);
    if has(EProverOption::LiteralSelectionStrategy) || has(EProverOption::NoGeneration) {
        params
            .selection_strategy
            .clone_from(&cli.selection_strategy);
    }
    if has(EProverOption::SelectOnProcessingOnly) {
        params.select_on_proc_only = cli.select_on_proc_only;
    }
    if has(EProverOption::InheritParamodLiterals) {
        params.inherit_paramod_lit = cli.inherit_paramod_lit;
    }
    if has(EProverOption::InheritGoalParamodLiterals) {
        params.inherit_goal_pm_lit = cli.inherit_goal_pm_lit;
    }
    if has(EProverOption::InheritConjectureParamodLiterals) {
        params.inherit_conj_pm_lit = cli.inherit_conj_pm_lit;
    }
    if has(EProverOption::SelectionPosMin) {
        params.pos_lit_sel_min = cli.pos_lit_sel_min;
    }
    if has(EProverOption::SelectionPosMax) {
        params.pos_lit_sel_max = cli.pos_lit_sel_max;
    }
    if has(EProverOption::SelectionNegMin) {
        params.neg_lit_sel_min = cli.neg_lit_sel_min;
    }
    if has(EProverOption::SelectionNegMax) {
        params.neg_lit_sel_max = cli.neg_lit_sel_max;
    }
    if has(EProverOption::SelectionAllMin) {
        params.all_lit_sel_min = cli.all_lit_sel_min;
    }
    if has(EProverOption::SelectionAllMax) {
        params.all_lit_sel_max = cli.all_lit_sel_max;
    }
    if has(EProverOption::SelectionWeightMin) {
        params.weight_sel_min = cli.weight_sel_min;
    }
    if has(EProverOption::PreferInitialClauses) {
        params.prefer_initial_clauses = cli.prefer_initial_clauses;
    }
    if has(EProverOption::ExpertHeuristic) {
        params.heuristic_name.clone_from(&cli.heuristic_name);
    }
    if has(EProverOption::FilterOrphansLimit) {
        params.filter_orphans_limit = cli.filter_orphans_limit;
    }
    if has(EProverOption::ForwardContractLimit) {
        params.forward_contract_limit = cli.forward_contract_limit;
    }
    if has(EProverOption::DefineHeuristic) {
        params.heuristic_def.clone_from(&cli.heuristic_def);
    }
}

fn overlay_explicit_inference_options(
    config: &EProverConfig,
    params: &mut HeuristicParmsCell,
    cli: &HeuristicParmsCell,
) {
    let has = |option| config.explicit_options.contains(&option);
    if has(EProverOption::DisableEqFactoring) {
        params.enable_eq_factoring = cli.enable_eq_factoring;
    }
    if has(EProverOption::DisableParamodIntoNegUnits) {
        params.enable_neg_unit_paramod = cli.enable_neg_unit_paramod;
    }
    if has(EProverOption::DisableGivenClauseForwardContraction) {
        params.enable_given_forward_simpl = cli.enable_given_forward_simpl;
    }
    if has(EProverOption::SimulParamod)
        || has(EProverOption::OrientedSimulParamod)
        || has(EProverOption::SupersimulParamod)
        || has(EProverOption::OrientedSupersimulParamod)
    {
        params.pm_type = cli.pm_type;
    }
    if has(EProverOption::SosUsesInputTypes) {
        params.use_tptp_sos = cli.use_tptp_sos;
    }
    if has(EProverOption::DestructiveEr) || has(EProverOption::StrongDestructiveEr) {
        params.er_varlit_destructive = cli.er_varlit_destructive;
        params.er_strong_destructive = cli.er_strong_destructive;
    }
    if has(EProverOption::DestructiveErAggressive) {
        params.er_aggressive = cli.er_aggressive;
    }
    if has(EProverOption::ForwardContextSr) || has(EProverOption::ForwardContextSrAggressive) {
        params.forward_context_sr = cli.forward_context_sr;
        params.forward_context_sr_aggressive = cli.forward_context_sr_aggressive;
    }
    if has(EProverOption::BackwardContextSr) {
        params.backward_context_sr = cli.backward_context_sr;
    }
    if has(EProverOption::ForwardSubsumptionAggressive) {
        params.forward_subsumption_aggressive = cli.forward_subsumption_aggressive;
    }
    if has(EProverOption::PreferGeneralDemodulators) {
        params.prefer_general = cli.prefer_general;
    }
    if has(EProverOption::ForwardDemodLevel) {
        params.forward_demod = cli.forward_demod;
    }
    if has(EProverOption::DemodUnderLambda) {
        params.lambda_demod = cli.lambda_demod;
    }
    if has(EProverOption::Condense) || has(EProverOption::CondenseAggressive) {
        params.condensing = cli.condensing;
        params.condensing_aggressive = cli.condensing_aggressive;
    }
    if has(EProverOption::SatCheckProcInterval) {
        params.sat_check_step_limit = cli.sat_check_step_limit;
    }
    if has(EProverOption::SatCheckGenInterval) {
        params.sat_check_size_limit = cli.sat_check_size_limit;
    }
    if has(EProverOption::SatCheckTTInsertInterval) {
        params.sat_check_ttinsert_limit = cli.sat_check_ttinsert_limit;
    }
    if has(EProverOption::SatCheck) {
        params.sat_check_grounding = cli.sat_check_grounding;
    }
    if has(EProverOption::SatCheckDecisionLimit) {
        params.sat_check_decision_limit = cli.sat_check_decision_limit;
    }
    if has(EProverOption::SatCheckNormalizeConst) {
        params.sat_check_normconst = cli.sat_check_normconst;
    }
    if has(EProverOption::SatCheckNormalizeUnproc) {
        params.sat_check_normalize = cli.sat_check_normalize;
    }
}

fn overlay_explicit_index_and_split_options(
    config: &EProverConfig,
    params: &mut HeuristicParmsCell,
    cli: &HeuristicParmsCell,
) {
    let has = |option| config.explicit_options.contains(&option);
    if has(EProverOption::RewriteBackwardIndex) || has(EProverOption::FingerprintIndex) {
        params.rw_bw_index_type.clone_from(&cli.rw_bw_index_type);
    }
    if has(EProverOption::ParamodFromIndex) || has(EProverOption::FingerprintIndex) {
        params
            .pm_from_index_type
            .clone_from(&cli.pm_from_index_type);
    }
    if has(EProverOption::ParamodIntoIndex) || has(EProverOption::FingerprintIndex) {
        params
            .pm_into_index_type
            .clone_from(&cli.pm_into_index_type);
    }
    if has(EProverOption::SplitClauses) {
        params.split_clauses = cli.split_clauses;
    }
    if has(EProverOption::SplitMethod) {
        params.split_method = cli.split_method;
    }
    if has(EProverOption::SplitAggressive) {
        params.split_aggressive = cli.split_aggressive;
    }
    if has(EProverOption::SplitReuseDefs) {
        params.split_fresh_defs = cli.split_fresh_defs;
    }
    if has(EProverOption::DisequalityDecomposition) {
        params.diseq_decomposition = cli.diseq_decomposition;
    }
    if has(EProverOption::DisequalityDecompMaxArity) {
        params.diseq_decomp_maxarity = cli.diseq_decomp_maxarity;
    }
    if has(EProverOption::StaticWatchlist) {
        params.watchlist_is_static = cli.watchlist_is_static;
    }
    if has(EProverOption::NoWatchlistSimplification) {
        params.watchlist_simplify = cli.watchlist_simplify;
    }
    if has(EProverOption::DeterministicRewriteSort) {
        params.detsort_bw_rw = cli.detsort_bw_rw;
    }
    if has(EProverOption::DeterministicNewSort) {
        params.detsort_tmpset = cli.detsort_tmpset;
    }
}

fn overlay_explicit_higher_order_options(
    config: &EProverConfig,
    params: &mut HeuristicParmsCell,
    cli: &HeuristicParmsCell,
) {
    let has = |option| config.explicit_options.contains(&option);
    if has(EProverOption::ArgCong) {
        params.arg_cong = cli.arg_cong;
    }
    if has(EProverOption::NegExt) {
        params.neg_ext = cli.neg_ext;
    }
    if has(EProverOption::PosExt) {
        params.pos_ext = cli.pos_ext;
    }
    if has(EProverOption::ExtSupMaxDepth) {
        params.ext_rules_max_depth = cli.ext_rules_max_depth;
    }
    if has(EProverOption::InverseRecognition) {
        params.inverse_recognition = cli.inverse_recognition;
    }
    if has(EProverOption::ReplaceInjDefs) {
        params.replace_inj_defs = cli.replace_inj_defs;
    }
    if has(EProverOption::LiftLambdas) {
        params.lift_lambdas = cli.lift_lambdas;
    }
    if has(EProverOption::CnfLambdaToForall) {
        params.lambda_to_forall = cli.lambda_to_forall;
    }
    if has(EProverOption::EliminateLeibnizEq) {
        params.elim_leibniz_max_depth = cli.elim_leibniz_max_depth;
    }
    if has(EProverOption::UnrollFormulasOnly) {
        params.unroll_only_formulas = cli.unroll_only_formulas;
    }
    if has(EProverOption::PrimEnumMode) {
        params.prim_enum_mode = cli.prim_enum_mode;
    }
    if has(EProverOption::PrimEnumMaxDepth) {
        params.prim_enum_max_depth = cli.prim_enum_max_depth;
    }
    if has(EProverOption::InstChoiceMaxDepth) {
        params.inst_choice_max_depth = cli.inst_choice_max_depth;
    }
    if has(EProverOption::LocalRw) {
        params.local_rw = cli.local_rw;
    }
    if has(EProverOption::PruneArgs) {
        params.prune_args = cli.prune_args;
    }
    if has(EProverOption::PreinstantiateInduction)
        || has(EProverOption::ClassificationTimeoutPortion)
    {
        params.preinstantiate_induction = cli.preinstantiate_induction;
    }
    if has(EProverOption::FuncProjLimit) {
        params.func_proj_limit = cli.func_proj_limit;
    }
    if has(EProverOption::ImitLimit) {
        params.imit_limit = cli.imit_limit;
    }
    if has(EProverOption::IdentLimit) {
        params.ident_limit = cli.ident_limit;
    }
    if has(EProverOption::ElimLimit) {
        params.elim_limit = cli.elim_limit;
    }
    if has(EProverOption::UnifMode) {
        params.unif_mode = cli.unif_mode;
    }
    if has(EProverOption::PatternOracle) {
        params.pattern_oracle = cli.pattern_oracle;
    }
    if has(EProverOption::FixpointOracle) {
        params.fixpoint_oracle = cli.fixpoint_oracle;
    }
    if has(EProverOption::MaxUnifiers) {
        params.max_unifiers = cli.max_unifiers;
    }
    if has(EProverOption::MaxUnifSteps) {
        params.max_unif_steps = cli.max_unif_steps;
    }
}

/// Builds C-shaped feature-vector index parameters from parsed executable
/// options.
///
/// # Errors
///
/// Returns a diagnostic if manually constructed config values are negative or
/// do not fit Rust's index size.
pub fn fv_index_params_from_config(
    config: &FeatureVectorIndexConfig,
) -> Result<FvIndexParams, Diagnostic> {
    Ok(FvIndexParams::new(
        fv_index_type(config.feature_type),
        config.use_perm_vectors,
        config.eliminate_uninformative,
        usize_from_i64_config("fv_index.max_symbols", config.max_symbols)?,
        usize_from_i64_config("fv_index.symbol_slack", config.symbol_slack)?,
    ))
}

fn translate_weight_generation(name: &str) -> Result<to_params::TOWeightGenMethod, Diagnostic> {
    if to_params::TO_WEIGHT_GEN_NAMES.contains(&name) {
        Ok(to_params::to_translate_weight_gen_method(name))
    } else {
        Err(config_conversion_error(format!(
            "Unknown order weight generation method '{name}'"
        )))
    }
}

fn translate_precedence_generation(name: &str) -> Result<to_params::TOPrecGenMethod, Diagnostic> {
    if to_params::TO_PREC_GEN_NAMES.contains(&name) {
        Ok(to_params::to_translate_prec_gen_method(name))
    } else {
        Err(config_conversion_error(format!(
            "Unknown order precedence generation method '{name}'"
        )))
    }
}

const fn to_params_term_ordering(value: TermOrdering) -> to_params::TermOrdering {
    match value {
        TermOrdering::NoOrdering => to_params::TermOrdering::NoOrdering,
        TermOrdering::Optimize => to_params::TermOrdering::Optimize,
        TermOrdering::Kbo => to_params::TermOrdering::Kbo,
        TermOrdering::Kbo6 => to_params::TermOrdering::Kbo6,
        TermOrdering::Lpo => to_params::TermOrdering::Lpo,
        TermOrdering::LpoCopy => to_params::TermOrdering::LpoCopy,
        TermOrdering::Lpo4 => to_params::TermOrdering::Lpo4,
        TermOrdering::Lpo4Copy => to_params::TermOrdering::Lpo4Copy,
        TermOrdering::Rpo => to_params::TermOrdering::Rpo,
        TermOrdering::Empty => to_params::TermOrdering::Empty,
    }
}

const fn to_params_literal_cmp(value: LiteralComparison) -> to_params::LiteralCmp {
    match value {
        LiteralComparison::None => to_params::LiteralCmp::NoCmp,
        LiteralComparison::Normal => to_params::LiteralCmp::Normal,
        LiteralComparison::TfoEqMax => to_params::LiteralCmp::TfoEqMax,
        LiteralComparison::TfoEqMin => to_params::LiteralCmp::TfoEqMin,
    }
}

const fn hcb_ac_handling(value: AcHandling) -> hcb::AcHandling {
    match value {
        AcHandling::None => hcb::AcHandling::None,
        AcHandling::DiscardAll => hcb::AcHandling::DiscardAll,
        AcHandling::KeepUnits => hcb::AcHandling::KeepUnits,
        AcHandling::KeepOrientable => hcb::AcHandling::KeepOrientable,
    }
}

const fn hcb_paramodulation_type(value: ParamodulationType) -> hcb::ParamodulationType {
    match value {
        ParamodulationType::Plain => hcb::ParamodulationType::Plain,
        ParamodulationType::Sim => hcb::ParamodulationType::Sim,
        ParamodulationType::OrientedSim => hcb::ParamodulationType::OrientedSim,
        ParamodulationType::SuperSim => hcb::ParamodulationType::SuperSim,
        ParamodulationType::OrientedSuperSim => hcb::ParamodulationType::OrientedSuperSim,
        ParamodulationType::DecreasingSim => hcb::ParamodulationType::DecreasingSim,
        ParamodulationType::SizeDecreasingSim => hcb::ParamodulationType::SizeDecreasingSim,
    }
}

fn hcb_split_class_type(value: i64) -> Result<hcb::SplitClassType, Diagnostic> {
    Ok(hcb::SplitClassType::from_c_value(i32_from_i64_config(
        "split_clauses",
        value,
    )?))
}

fn hcb_split_type(value: i64) -> Result<hcb::SplitType, Diagnostic> {
    hcb::SplitType::from_c_value(i32_from_i64_config("split_method", value)?)
        .ok_or_else(|| config_conversion_error(format!("Invalid split_method value '{value}'")))
}

const fn hcb_grounding_strategy(value: GroundingStrategy) -> hcb::GroundingStrategy {
    match value {
        GroundingStrategy::NoGrounding => hcb::GroundingStrategy::NoGrounding,
        GroundingStrategy::PseudoVar => hcb::GroundingStrategy::PseudoVar,
        GroundingStrategy::FirstConst => hcb::GroundingStrategy::FirstConst,
        GroundingStrategy::ConjMinMinFreq => hcb::GroundingStrategy::ConjMinMinFreq,
        GroundingStrategy::ConjMaxMinFreq => hcb::GroundingStrategy::ConjMaxMinFreq,
        GroundingStrategy::ConjMinMaxFreq => hcb::GroundingStrategy::ConjMinMaxFreq,
        GroundingStrategy::ConjMaxMaxFreq => hcb::GroundingStrategy::ConjMaxMaxFreq,
        GroundingStrategy::GlobalMax => hcb::GroundingStrategy::GlobalMax,
        GroundingStrategy::GlobalMin => hcb::GroundingStrategy::GlobalMin,
    }
}

const fn hcb_ext_inference_type(value: ExtInferenceType) -> hcb::ExtInferenceType {
    match value {
        ExtInferenceType::AllLits => hcb::ExtInferenceType::AllLits,
        ExtInferenceType::MaxLits => hcb::ExtInferenceType::MaxLits,
        ExtInferenceType::NoLits => hcb::ExtInferenceType::NoLits,
    }
}

const fn hcb_prim_enum_mode(value: PrimEnumMode) -> hcb::PrimEnumMode {
    match value {
        PrimEnumMode::Neg => hcb::PrimEnumMode::Neg,
        PrimEnumMode::And => hcb::PrimEnumMode::And,
        PrimEnumMode::Or => hcb::PrimEnumMode::Or,
        PrimEnumMode::Eq => hcb::PrimEnumMode::Eq,
        PrimEnumMode::Pragmatic => hcb::PrimEnumMode::Pragmatic,
        PrimEnumMode::Full => hcb::PrimEnumMode::Full,
        PrimEnumMode::LogSymbol => hcb::PrimEnumMode::LogSymbol,
    }
}

const fn hcb_unif_mode(value: UnificationMode) -> hcb::UnifMode {
    match value {
        UnificationMode::Single => hcb::UnifMode::Single,
        UnificationMode::Multi => hcb::UnifMode::Multi,
    }
}

const fn fv_index_type(value: FvIndexFeatureType) -> FvIndexType {
    match value {
        FvIndexFeatureType::NoFeatures => FvIndexType::NoFeatures,
        FvIndexFeatureType::AcFeatures => FvIndexType::AcFeatures,
        FvIndexFeatureType::SsFeatures => FvIndexType::SsFeatures,
        FvIndexFeatureType::AllFeatures => FvIndexType::AllFeatures,
        FvIndexFeatureType::BillFeatures => FvIndexType::BillFeatures,
        FvIndexFeatureType::BillPlusFeatures => FvIndexType::BillPlusFeatures,
        FvIndexFeatureType::AcFold => FvIndexType::AcFold,
        FvIndexFeatureType::AcStagger => FvIndexType::AcStagger,
        FvIndexFeatureType::CollectFeatures => FvIndexType::CollectFeatures,
    }
}

fn i32_from_i64_config(field_name: &str, value: i64) -> Result<i32, Diagnostic> {
    i32::try_from(value).map_err(|_| {
        config_conversion_error(format!(
            "Configuration field '{field_name}' value '{value}' does not fit C int"
        ))
    })
}

fn usize_from_i64_config(field_name: &str, value: i64) -> Result<usize, Diagnostic> {
    usize::try_from(value).map_err(|_| {
        config_conversion_error(format!(
            "Configuration field '{field_name}' value '{value}' does not fit usize"
        ))
    })
}

fn config_conversion_error(message: String) -> Diagnostic {
    Diagnostic::new(ErrorCode::USAGE_ERROR, message)
}

#[derive(Debug)]
pub enum EProverError {
    Diagnostic(Diagnostic),
    Io(io::Error),
}

impl EProverError {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Diagnostic(diagnostic) => diagnostic.code(),
            Self::Io(_) => ErrorCode::OTHER_ERROR,
        }
    }

    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Diagnostic(diagnostic) => diagnostic.message().to_owned(),
            Self::Io(error) => error.to_string(),
        }
    }
}

impl fmt::Display for EProverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for EProverError {}

impl From<Diagnostic> for EProverError {
    fn from(value: Diagnostic) -> Self {
        Self::Diagnostic(value)
    }
}

impl From<io::Error> for EProverError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

enum ConfiguredOutput<'a, W: Write + ?Sized> {
    Writer(&'a mut W),
    File { file: File, stdout: &'a mut W },
}

impl<W: Write + ?Sized> Write for ConfiguredOutput<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Writer(writer) => writer.write(buffer),
            Self::File { file, .. } => file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Writer(writer) => writer.flush(),
            Self::File { file, .. } => file.flush(),
        }
    }
}

impl<W: Write + ?Sized> ConfiguredOutput<'_, W> {
    fn write_stdout_side_channel(&mut self, buffer: &[u8]) -> io::Result<()> {
        match self {
            Self::Writer(writer) | Self::File { stdout: writer, .. } => writer.write_all(buffer),
        }
    }
}

fn open_configured_output<'a, W: Write + ?Sized>(
    stdout: &'a mut W,
    output_file: Option<&str>,
) -> Result<ConfiguredOutput<'a, W>, Diagnostic> {
    let Some(name) = output_file else {
        return Ok(ConfiguredOutput::Writer(stdout));
    };
    if name == "-" {
        return Ok(ConfiguredOutput::Writer(stdout));
    }

    let path = Path::new(name);
    File::create(path)
        .map(|file| ConfiguredOutput::File { file, stdout })
        .map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot open file {}: {error}", path.display()),
            )
        })
}

#[allow(clippy::cast_sign_loss)]
const fn c_rlimit_from_arg(value: i64) -> u64 {
    // The C path assigns CLStateGetIntArg()'s signed long to rlim_t.
    value as u64
}

fn check_hard_soft_limits(
    hard: i64,
    soft: i64,
    hard_option_changed: bool,
) -> Result<(), Diagnostic> {
    if c_rlimit_from_arg(hard) > c_rlimit_from_arg(soft) {
        return Ok(());
    }
    let message = if hard_option_changed {
        "Hard time limit has to be larger than softtime limit"
    } else {
        "Soft time limit has to be smaller than hardtime limit"
    };
    Err(Diagnostic::new(ErrorCode::USAGE_ERROR, message))
}

fn apply_time_limit_state(config: &EProverConfig) {
    let hard_limit = config
        .cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    let soft_limit = config
        .soft_cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    let schedule_limit = config.schedule_time_limit.map_or(0, c_rlimit_from_arg);
    configure_time_limits(hard_limit, soft_limit, schedule_limit);
}

fn setup_signal_handlers() -> Result<(), Diagnostic> {
    if let SignalOutcome::HandlerInstallFailed { diagnostic, .. } = e_signal_setup(SIGXCPU_COMPAT) {
        Err(diagnostic)
    } else {
        Ok(())
    }
}

#[cfg(any(test, target_os = "linux"))]
fn cpu_rlimit_to_apply(config: &EProverConfig) -> Option<u64> {
    let hard_limit = config
        .cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    let soft_limit = config
        .soft_cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);

    if hard_limit == RLIM_INFINITY_COMPAT && soft_limit == RLIM_INFINITY_COMPAT {
        return None;
    }
    if soft_limit == RLIM_INFINITY_COMPAT {
        Some(hard_limit)
    } else {
        Some(soft_limit)
    }
}

#[cfg(any(test, windows))]
fn native_hard_cpu_limit_to_apply(config: &EProverConfig) -> Option<u64> {
    let hard_limit = c_rlimit_from_arg(config.cpu_limit?);
    (hard_limit != RLIM_INFINITY_COMPAT).then_some(hard_limit)
}

fn apply_os_resource_limit_state(config: &EProverConfig) -> Vec<ResourceSetupMessage> {
    #[cfg(test)]
    {
        let _ = config;
        Vec::new()
    }

    #[cfg(not(test))]
    {
        let mut messages = Vec::new();
        #[cfg(target_os = "linux")]
        {
            if let Some(cpu_limit) = cpu_rlimit_to_apply(config) {
                if let Some(warning) = rlimit_warning_from_outcome(
                    set_soft_rlimit_with_error(RLIMIT_CPU_COMPAT, cpu_limit),
                    RLIMIT_CPU_COMPAT,
                    cpu_limit,
                    cpu_rlimit_desc(config),
                ) {
                    messages.push(ResourceSetupMessage::Warning(warning));
                }
                messages.extend(core_limit_failure_messages(set_soft_rlimit_with_error(
                    RLIMIT_CORE_COMPAT,
                    0,
                )));
            }
        }

        #[cfg(windows)]
        {
            if let Some(cpu_limit) = native_hard_cpu_limit_to_apply(config) {
                if let Some(warning) = resource_limit_warning_from_result(
                    set_hard_cpu_limit(cpu_limit),
                    cpu_limit,
                    "JOB_OBJECT_LIMIT_PROCESS_TIME",
                ) {
                    messages.push(ResourceSetupMessage::Warning(warning));
                }
            }
        }

        let memory_limit_result = set_memory_limit(config.memory_limit);
        #[cfg(target_os = "linux")]
        {
            if memory_limit_result == RLimResult::Reduced {
                messages.push(ResourceSetupMessage::Warning(
                    resource_limit_warning_from_result(
                        memory_limit_result,
                        config.memory_limit,
                        "memory limit",
                    )
                    .expect("reduced resource limits produce warnings"),
                ));
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            if let Some(warning) = resource_limit_warning_from_result(
                memory_limit_result,
                config.memory_limit,
                "memory limit",
            ) {
                messages.push(ResourceSetupMessage::Warning(warning));
            }
        }
        messages
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResourceSetupMessage {
    Warning(Diagnostic),
    #[cfg(any(test, target_os = "linux"))]
    Perror {
        os_error: i32,
    },
}

impl ResourceSetupMessage {
    #[must_use]
    fn render(&self, program_name: &str) -> String {
        match self {
            Self::Warning(warning) => warning.render_warning(program_name),
            #[cfg(any(test, target_os = "linux"))]
            Self::Perror { os_error } => {
                format!(
                    "{program_name}: {}\n",
                    resource_limit_error_message(*os_error)
                )
            }
        }
    }
}

#[cfg(any(test, target_os = "linux"))]
fn core_limit_failure_messages(outcome: RLimitOutcome) -> Vec<ResourceSetupMessage> {
    if outcome.result() == RLimResult::Success {
        return Vec::new();
    }

    let mut messages = Vec::new();
    if let Some(os_error) = outcome.os_error() {
        messages.push(ResourceSetupMessage::Perror { os_error });
    }
    messages.push(ResourceSetupMessage::Warning(Diagnostic::new(
        ErrorCode::SYSTEM_ERROR,
        "Cannot prevent core dumps!",
    )));
    messages
}

#[cfg(all(target_os = "linux", not(test)))]
fn cpu_rlimit_desc(config: &EProverConfig) -> &'static str {
    if config.soft_cpu_limit.is_some() {
        "RLIMIT_CPU (E-Soft)"
    } else {
        "RLIMIT_CPU (E-Hard)"
    }
}

#[cfg(any(test, target_os = "linux"))]
fn rlimit_warning_from_result(
    result: RLimResult,
    resource: i32,
    limit: u64,
    desc: &str,
) -> Option<Diagnostic> {
    rlimit_warning_from_outcome(RLimitOutcome::new(result, None), resource, limit, desc)
}

#[cfg(any(test, target_os = "linux"))]
fn rlimit_warning_from_outcome(
    outcome: RLimitOutcome,
    resource: i32,
    limit: u64,
    desc: &str,
) -> Option<Diagnostic> {
    #[cfg(target_os = "linux")]
    let data_resource = RLIMIT_DATA_COMPAT;
    #[cfg(not(target_os = "linux"))]
    let data_resource = 2;

    if outcome.result() == RLimResult::Failed && resource == data_resource {
        return None;
    }

    resource_limit_warning_from_outcome(outcome, limit, desc)
}

fn resource_limit_warning_from_result(
    result: RLimResult,
    limit: u64,
    desc: &str,
) -> Option<Diagnostic> {
    resource_limit_warning_from_outcome(RLimitOutcome::new(result, None), limit, desc)
}

fn resource_limit_warning_from_outcome(
    outcome: RLimitOutcome,
    limit: u64,
    desc: &str,
) -> Option<Diagnostic> {
    match outcome.result() {
        RLimResult::Failed => Some(Diagnostic::new(
            ErrorCode::SYSTEM_ERROR,
            resource_limit_failed_warning_message(desc, limit, outcome.os_error()),
        )),
        RLimResult::Reduced => Some(Diagnostic::new(
            ErrorCode::SYSTEM_ERROR,
            format!("Had to reduce limit {desc}"),
        )),
        RLimResult::Success => None,
    }
}

fn resource_limit_failed_warning_message(desc: &str, limit: u64, os_error: Option<i32>) -> String {
    let base = format!("Could not set limit {desc} to {limit}");
    match os_error {
        Some(error) => format!("{base} ({})", resource_limit_error_message(error)),
        None => base,
    }
}

fn apply_ordering_state(config: &EProverConfig) {
    if config.search.ordering.lpo_recursion_limit_changed {
        set_lpo_recursion_depth_limit(config.search.ordering.lpo_recursion_limit);
    }
}

const fn memory_limit_bytes_from_mb(memory_mb: i64) -> u64 {
    c_rlimit_from_arg(memory_mb).wrapping_mul(MEGA)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn auto_memory_limit_from_system_mb(system_memory_mb: i64) -> Result<(u64, i64), Diagnostic> {
    if system_memory_mb == -1 {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "Cannot find physical memory automatically. Give explicit value to --memory-limit",
        ));
    }

    let memory_mb = (system_memory_mb as f64 * 0.8) as i64;
    let delete_bad_limit = (f64::from((c_rlimit_from_arg(memory_mb).wrapping_sub(2)) as f32)
        * 0.7
        * MEGA as f64) as i64;
    Ok((memory_limit_bytes_from_mb(memory_mb), delete_bad_limit))
}

pub fn run<I, S>(
    argv: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, EProverError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    init_io(PROGRAM_NAME);
    let _io_guard = IoRunGuard;
    let _problem_type_guard = ProblemTypeRunGuard::new();
    setup_signal_handlers()?;
    match process_options(argv)? {
        EProverAction::Help => {
            stdout.write_all(print_help().as_bytes())?;
            Ok(ErrorCode::NO_ERROR.exit_status())
        }
        EProverAction::Version => {
            stdout.write_all(version::version_line().as_bytes())?;
            Ok(ErrorCode::NO_ERROR.exit_status())
        }
        EProverAction::Run(config) => {
            write_config_warnings(stderr, &config)?;
            write_resource_setup_messages(stderr, &apply_os_resource_limit_state(&config))?;
            run_config(stdout, &config)
        }
    }
}

fn write_config_warnings(
    stderr: &mut impl Write,
    config: &EProverConfig,
) -> Result<(), EProverError> {
    write_warnings(stderr, &config.warnings)
}

fn write_warnings(stderr: &mut impl Write, warnings: &[Diagnostic]) -> Result<(), EProverError> {
    for warning in warnings {
        stderr.write_all(warning.render_warning(PROGRAM_NAME).as_bytes())?;
    }
    Ok(())
}

fn write_resource_setup_messages(
    stderr: &mut impl Write,
    messages: &[ResourceSetupMessage],
) -> Result<(), EProverError> {
    for message in messages {
        stderr.write_all(message.render(PROGRAM_NAME).as_bytes())?;
    }
    Ok(())
}

pub fn process_options<I, S>(argv: I) -> Result<EProverAction, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = EProverConfig::default();
    while let Some(parsed) = state.next_opt(EPROVER_OPTIONS)? {
        if let Some(action) = apply_parsed_option(&mut config, &parsed)? {
            return Ok(action);
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(EProverAction::Run(Box::new(config)))
}

fn apply_parsed_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<Option<EProverAction>, Diagnostic> {
    let option_code = parsed.option().option_code;
    match option_code {
        EProverOption::Help => return Ok(Some(EProverAction::Help)),
        EProverOption::Version => return Ok(Some(EProverAction::Version)),
        _ => {}
    }
    config.explicit_options.push(option_code);

    if is_output_option(option_code) {
        apply_output_option(config, parsed)?;
    } else if is_proof_option(option_code) {
        apply_proof_option(config, parsed)?;
    } else if is_resource_option(option_code) {
        apply_resource_option(config, parsed)?;
    } else if is_strategy_option(option_code) {
        apply_strategy_option(config, parsed);
    } else if is_limit_option(option_code) {
        apply_limit_option(config, parsed)?;
    } else if is_format_option(option_code) {
        apply_format_option(config, parsed);
    } else if is_input_mode_option(option_code) {
        apply_input_mode_option(config, parsed);
    } else if is_simple_flag(option_code) {
        apply_simple_flag(config, option_code);
    } else if is_schedule_option(option_code) {
        apply_schedule_option(config, parsed)?;
    } else if is_preprocessing_option(option_code) {
        apply_preprocessing_option(config, parsed)?;
    } else if is_term_ordering_option(option_code) {
        apply_term_ordering_option(config, parsed)?;
    } else if is_search_control_option(option_code) {
        apply_search_control_option(config, parsed)?;
    } else {
        unreachable!("unhandled eprover option");
    }
    Ok(None)
}

const fn is_output_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::Verbose
            | EProverOption::Output
            | EProverOption::Silent
            | EProverOption::OutputLevel
            | EProverOption::PrintStatistics
            | EProverOption::PrintDetailedStatistics
            | EProverOption::PrintSaturated
            | EProverOption::PrintSatInfo
            | EProverOption::FilterSaturated
    )
}

const fn is_proof_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::ProofObject
            | EProverOption::ProofGraph
            | EProverOption::ProofStatistics
            | EProverOption::FullDerivation
            | EProverOption::ForceDerivation
            | EProverOption::RecordGivenClauses
            | EProverOption::TrainingExamples
            | EProverOption::PclTermsCompressed
            | EProverOption::PclCompact
            | EProverOption::PclShellLevel
    )
}

const fn is_resource_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::CpuLimit | EProverOption::SoftCpuLimit | EProverOption::MemoryLimit
    )
}

const fn is_strategy_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::SelectStrategy | EProverOption::PrintStrategy | EProverOption::ParseStrategy
    )
}

const fn is_limit_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::ProcessedClausesLimit
            | EProverOption::ProcessedSetLimit
            | EProverOption::UnprocessedLimit
            | EProverOption::TotalClauseSetLimit
            | EProverOption::GeneratedLimit
            | EProverOption::TermBankInsertLimit
            | EProverOption::Answers
    )
}

const fn is_format_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::EqnNoInfix
            | EProverOption::FullEquationalRep
            | EProverOption::PrintOrientedEqLitsAsRules
            | EProverOption::LopIn
            | EProverOption::PclOut
            | EProverOption::TptpIn
            | EProverOption::TptpOut
            | EProverOption::TptpFormat
            | EProverOption::TstpIn
            | EProverOption::TstpOut
            | EProverOption::TstpFormat
    )
}

const fn is_input_mode_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::SyntaxOnly
            | EProverOption::PrintFormulas
            | EProverOption::PruneOnly
            | EProverOption::CnfOnly
    )
}

const fn is_simple_flag(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::PrintPid
            | EProverOption::PrintVersion
            | EProverOption::RequireNonempty
            | EProverOption::ResourcesInfo
            | EProverOption::ConjecturesAreQuestions
            | EProverOption::DeterministicRewriteSort
            | EProverOption::DeterministicNewSort
    )
}

const fn is_schedule_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::Auto
            | EProverOption::AutoSchedule
            | EProverOption::SerializeSchedule
            | EProverOption::ForcePreprocessingSchedule
            | EProverOption::SatAutoSchedule
    )
}

const fn is_preprocessing_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::NoPreprocessing
            | EProverOption::EqUnfoldLimit
            | EProverOption::EqUnfoldMaxClauses
            | EProverOption::NoEqUnfolding
            | EProverOption::GoalDefs
            | EProverOption::GoalSubtermDefs
            | EProverOption::Sine
            | EProverOption::RelPruningLevel
            | EProverOption::PresatSimplify
            | EProverOption::AcHandling
            | EProverOption::AcNonAggressive
    )
}

const fn is_search_control_option(option: EProverOption) -> bool {
    is_literal_selection_option(option)
        || is_heuristic_control_option(option)
        || is_definition_option(option)
        || is_input_symbol_option(option)
        || is_cnf_control_option(option)
        || is_preprocessing_elimination_option(option)
        || is_inference_control_option(option)
        || is_inference_processing_option(option)
        || is_extension_inference_option(option)
        || is_higher_order_control_option(option)
        || is_unification_option(option)
        || is_watchlist_option(option)
        || is_subsumption_index_option(option)
        || is_fingerprint_index_option(option)
        || is_splitting_option(option)
}

const fn is_term_ordering_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::TermOrdering
            | EProverOption::OrderWeightGeneration
            | EProverOption::OrderWeights
            | EProverOption::OrderPrecedenceGeneration
            | EProverOption::PrecPureConj
            | EProverOption::PrecConjAxiom
            | EProverOption::PrecPureAxiom
            | EProverOption::PrecSkolem
            | EProverOption::PrecDefPred
            | EProverOption::OrderConstantWeight
            | EProverOption::Precedence
            | EProverOption::LpoRecursionLimit
            | EProverOption::RestrictLiteralComparisons
            | EProverOption::LiteralComparison
            | EProverOption::KboLambdaWeight
            | EProverOption::KboDbWeight
    )
}

const fn is_literal_selection_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::LiteralSelectionStrategy
            | EProverOption::NoGeneration
            | EProverOption::SelectOnProcessingOnly
            | EProverOption::InheritParamodLiterals
            | EProverOption::InheritGoalParamodLiterals
            | EProverOption::InheritConjectureParamodLiterals
            | EProverOption::SelectionPosMin
            | EProverOption::SelectionPosMax
            | EProverOption::SelectionNegMin
            | EProverOption::SelectionNegMax
            | EProverOption::SelectionAllMin
            | EProverOption::SelectionAllMax
            | EProverOption::SelectionWeightMin
    )
}

const fn is_heuristic_control_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::PreferInitialClauses
            | EProverOption::ExpertHeuristic
            | EProverOption::FilterOrphansLimit
            | EProverOption::ForwardContractLimit
            | EProverOption::DeleteBadLimit
    )
}

const fn is_definition_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::DefineWeightFunction | EProverOption::DefineHeuristic
    )
}

const fn is_input_symbol_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::FreeNumbers | EProverOption::FreeObjects
    )
}

const fn is_cnf_control_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::DefinitionalCnf
            | EProverOption::FoolUnroll
            | EProverOption::MiniscopeLimit
            | EProverOption::PrintTypes
            | EProverOption::AppEncode
            | EProverOption::ClassificationTimeoutPortion
    )
}

const fn is_preprocessing_elimination_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::Bce
            | EProverOption::BceMaxOccs
            | EProverOption::PredElim
            | EProverOption::PredElimRecognizeGates
            | EProverOption::PredElimForceMuDecrease
            | EProverOption::PredElimIgnoreConjSyms
            | EProverOption::PredElimMaxOccs
            | EProverOption::PredElimTolerance
    )
}

const fn is_inference_control_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::AssumeCompleteness
            | EProverOption::AssumeIncompleteness
            | EProverOption::DisableEqFactoring
            | EProverOption::DisableParamodIntoNegUnits
            | EProverOption::Condense
            | EProverOption::CondenseAggressive
            | EProverOption::DisableGivenClauseForwardContraction
            | EProverOption::SimulParamod
            | EProverOption::OrientedSimulParamod
            | EProverOption::SupersimulParamod
            | EProverOption::OrientedSupersimulParamod
    )
}

const fn is_inference_processing_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::SosUsesInputTypes
            | EProverOption::DestructiveEr
            | EProverOption::StrongDestructiveEr
            | EProverOption::DestructiveErAggressive
            | EProverOption::ForwardContextSr
            | EProverOption::ForwardContextSrAggressive
            | EProverOption::BackwardContextSr
            | EProverOption::PreferGeneralDemodulators
            | EProverOption::ForwardDemodLevel
            | EProverOption::DemodUnderLambda
            | EProverOption::StrongRwInst
            | EProverOption::StrongForwardSubsumption
            | EProverOption::SatCheckProcInterval
            | EProverOption::SatCheckGenInterval
            | EProverOption::SatCheckTTInsertInterval
            | EProverOption::SatCheck
            | EProverOption::SatCheckDecisionLimit
            | EProverOption::SatCheckNormalizeConst
            | EProverOption::SatCheckNormalizeUnproc
            | EProverOption::LiftLambdas
    )
}

const fn is_extension_inference_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::ArgCong | EProverOption::NegExt | EProverOption::PosExt
    )
}

const fn is_higher_order_control_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::ExtSupMaxDepth
            | EProverOption::InverseRecognition
            | EProverOption::ReplaceInjDefs
            | EProverOption::CnfLambdaToForall
            | EProverOption::EtaNormalize
            | EProverOption::HoOrderKind
            | EProverOption::EliminateLeibnizEq
            | EProverOption::UnrollFormulasOnly
            | EProverOption::PrimEnumMode
            | EProverOption::PrimEnumMaxDepth
            | EProverOption::InstChoiceMaxDepth
            | EProverOption::LocalRw
            | EProverOption::PruneArgs
            | EProverOption::PreinstantiateInduction
    )
}

const fn is_unification_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::FuncProjLimit
            | EProverOption::ImitLimit
            | EProverOption::IdentLimit
            | EProverOption::ElimLimit
            | EProverOption::UnifMode
            | EProverOption::PatternOracle
            | EProverOption::FixpointOracle
            | EProverOption::MaxUnifiers
            | EProverOption::MaxUnifSteps
    )
}

const fn is_watchlist_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::Watchlist
            | EProverOption::StaticWatchlist
            | EProverOption::NoWatchlistSimplification
    )
}

const fn is_subsumption_index_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::ForwardSubsumptionAggressive
            | EProverOption::ConventionalSubsumption
            | EProverOption::SubsumptionIndexing
            | EProverOption::FvIndexFeatureTypes
            | EProverOption::FvIndexMaxFeatures
            | EProverOption::FvIndexSlack
    )
}

const fn is_fingerprint_index_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::RewriteBackwardIndex
            | EProverOption::ParamodFromIndex
            | EProverOption::ParamodIntoIndex
            | EProverOption::FingerprintIndex
            | EProverOption::FingerprintNoSizeConstr
            | EProverOption::PdtNoSizeConstr
            | EProverOption::PdtNoAgeConstr
    )
}

const fn is_splitting_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::SplitClauses
            | EProverOption::SplitMethod
            | EProverOption::SplitAggressive
            | EProverOption::SplitReuseDefs
            | EProverOption::DisequalityDecomposition
            | EProverOption::DisequalityDecompMaxArity
    )
}

fn apply_output_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::Verbose => {
            config.verbose = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::Output => {
            config.output_file = Some(parsed.arg().unwrap_or("").to_owned());
        }
        EProverOption::Silent => {
            config.output_level = 0;
        }
        EProverOption::OutputLevel => {
            config.output_level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::PrintStatistics => config.flags.set(EProverFlag::PrintStatistics),
        EProverOption::PrintDetailedStatistics => {
            config.flags.set(EProverFlag::PrintDetailedStatistics);
            config.flags.set(EProverFlag::PrintStatistics);
        }
        EProverOption::PrintSaturated => {
            let descriptor = parsed.arg().unwrap_or("").to_owned();
            check_option_letter_string(&descriptor, "teigEIGaA", "-S (--print-saturated)")?;
            config.saturated_output_descriptor = descriptor;
            config.flags.set(EProverFlag::PrintSaturated);
        }
        EProverOption::PrintSatInfo => config.flags.set(EProverFlag::PrintSaturatedInfo),
        EProverOption::FilterSaturated => {
            let descriptor = parsed.arg().unwrap_or("").to_owned();
            check_option_letter_string(&descriptor, "eigEIGaA", "--filter-saturated")?;
            config.filter_saturated_descriptor = descriptor;
            config.flags.set(EProverFlag::FilterSaturated);
        }
        _ => unreachable!("non-output option routed to output handler"),
    }
    Ok(())
}

fn apply_proof_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::ProofObject => {
            let level = get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 3)?;
            config.proof_object_level = config.proof_object_level.max(level);
            if level > 0 {
                config.proof_output = config.proof_output.max(1);
            }
        }
        EProverOption::ProofGraph => {
            let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            config.proof_object_level = config.proof_object_level.max(1);
            config.proof_output = level + 1;
        }
        EProverOption::ProofStatistics => config.flags.set(EProverFlag::ProofStatistics),
        EProverOption::FullDerivation => config.flags.set(EProverFlag::FullDerivation),
        EProverOption::ForceDerivation => {
            config.force_derivation_output =
                get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 2)?;
            config.proof_object_level = config.proof_object_level.max(1);
        }
        EProverOption::RecordGivenClauses => {
            config.proof_object_level = config.proof_object_level.max(1);
            config.flags.set(EProverFlag::RecordGivenClauses);
        }
        EProverOption::TrainingExamples => {
            config.proof_object_level = config.proof_object_level.max(1);
            config.flags.set(EProverFlag::RecordGivenClauses);
            config.training_examples =
                Some(get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?);
        }
        EProverOption::PclTermsCompressed => config.pcl_output.full_terms = false,
        EProverOption::PclCompact => config.pcl_output.compact = true,
        EProverOption::PclShellLevel => {
            config.pcl_output.shell_level =
                get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 2)?;
        }
        _ => unreachable!("non-proof option routed to proof handler"),
    }
    Ok(())
}

fn apply_resource_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::CpuLimit => {
            let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            if let Some(soft_limit) = config.soft_cpu_limit {
                check_hard_soft_limits(limit, soft_limit, true)?;
            }
            config.cpu_limit = Some(limit);
            config.schedule_time_limit = Some(limit);
        }
        EProverOption::SoftCpuLimit => {
            let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            if let Some(hard_limit) = config.cpu_limit {
                check_hard_soft_limits(hard_limit, limit, false)?;
            }
            config.soft_cpu_limit = Some(limit);
            config.schedule_time_limit = Some(limit);
        }
        EProverOption::MemoryLimit => {
            let arg = parsed.arg().unwrap_or("");
            if arg == "Auto" {
                let (memory_limit, delete_bad_limit) =
                    auto_memory_limit_from_system_mb(get_system_phys_memory())?;
                config.memory_limit = memory_limit;
                config.delete_bad_limit = delete_bad_limit;
            } else {
                let memory_mb = get_int_arg(parsed.option(), arg)?;
                config.memory_limit = memory_limit_bytes_from_mb(memory_mb);
            }
        }
        _ => unreachable!("non-resource option routed to resource handler"),
    }
    Ok(())
}

fn apply_strategy_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    match parsed.option().option_code {
        EProverOption::SelectStrategy => {
            config.select_strategy = Some(parsed.arg().unwrap_or("").to_owned());
        }
        EProverOption::PrintStrategy => {
            config.print_strategy = Some(parsed.arg().unwrap_or("").to_owned());
        }
        EProverOption::ParseStrategy => {
            config.parse_strategy_file = Some(parsed.arg().unwrap_or("").to_owned());
        }
        _ => unreachable!("non-strategy option routed to strategy handler"),
    }
}

fn schedule_core_arg(parsed: &ParsedOpt<'_, EProverOption>) -> Result<i64, Diagnostic> {
    let arg = parsed.arg().unwrap_or("");
    if arg == "Auto" {
        Ok(-1)
    } else {
        get_int_arg(parsed.option(), arg)
    }
}

fn apply_schedule_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::Auto => {
            if !config.flags.contains(EProverFlag::Auto) {
                config.sine = Some("Auto".to_owned());
                config.flags.set(EProverFlag::Auto);
            }
        }
        EProverOption::AutoSchedule => {
            if !config.strategy_scheduling {
                config.schedule_cores = schedule_core_arg(parsed)?;
                config.sine = Some("Auto".to_owned());
                config.strategy_scheduling = true;
            }
        }
        EProverOption::SerializeSchedule => {
            config.serialize_schedule = get_bool_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::ForcePreprocessingSchedule => {
            config.force_preprocessing_schedule =
                get_bool_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::SatAutoSchedule => {
            if !config.strategy_scheduling {
                config.schedule_cores = schedule_core_arg(parsed)?;
                config.strategy_scheduling = true;
            }
        }
        _ => unreachable!("non-schedule option routed to schedule handler"),
    }
    Ok(())
}

fn apply_preprocessing_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::NoPreprocessing => config.preprocessing.no_preprocessing = true,
        EProverOption::EqUnfoldLimit => {
            config.preprocessing.eqdef_incrlimit =
                get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::EqUnfoldMaxClauses => {
            config.preprocessing.eqdef_maxclauses =
                get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::NoEqUnfolding => {
            config.preprocessing.eqdef_incrlimit = i64::MIN;
        }
        EProverOption::GoalDefs => apply_goal_defs(config, parsed.arg().unwrap_or(""))?,
        EProverOption::GoalSubtermDefs => {
            config.preprocessing.goal_definitions.subterms = true;
        }
        EProverOption::Sine => config.sine = Some(parsed.arg().unwrap_or("").to_owned()),
        EProverOption::RelPruningLevel => {
            config.preprocessing.relevance_prune_level =
                get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::PresatSimplify => {
            config.preprocessing.presat_interreduction =
                get_bool_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::AcHandling => apply_ac_handling(config, parsed.arg().unwrap_or(""))?,
        EProverOption::AcNonAggressive => config.preprocessing.ac_res_aggressive = false,
        _ => unreachable!("non-preprocessing option routed to preprocessing handler"),
    }
    Ok(())
}

fn apply_goal_defs(config: &mut EProverConfig, arg: &str) -> Result<(), Diagnostic> {
    match arg {
        "None" => {
            config.preprocessing.goal_definitions.positive = false;
            config.preprocessing.goal_definitions.negative = false;
            Ok(())
        }
        "All" => {
            config.preprocessing.goal_definitions.positive = true;
            config.preprocessing.goal_definitions.negative = true;
            Ok(())
        }
        "Neg" => {
            config.preprocessing.goal_definitions.positive = false;
            config.preprocessing.goal_definitions.negative = true;
            Ok(())
        }
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Option --goal-defs accepts only None, All, or Neg",
        )),
    }
}

fn apply_ac_handling(config: &mut EProverConfig, arg: &str) -> Result<(), Diagnostic> {
    config.preprocessing.ac_handling = match arg {
        "None" => AcHandling::None,
        "DiscardAll" => AcHandling::DiscardAll,
        "KeepUnits" => AcHandling::KeepUnits,
        "KeepOrientable" => AcHandling::KeepOrientable,
        _ => {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                "Option --ac_handling requires None, DiscardAll, KeepUnits, or KeepOrientable as an argument",
            ));
        }
    };
    Ok(())
}

fn apply_term_ordering_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::TermOrdering => set_term_ordering(config, value)?,
        EProverOption::OrderWeightGeneration => set_weight_generation(config, value)?,
        EProverOption::OrderWeights => {
            config.search.ordering.weight_overrides = Some(value.to_owned());
        }
        EProverOption::OrderPrecedenceGeneration => set_precedence_generation(config, value)?,
        EProverOption::PrecPureConj => {
            config.search.ordering.precedence_modifiers.conjecture_only =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::PrecConjAxiom => {
            config.search.ordering.precedence_modifiers.conjecture_axiom =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::PrecPureAxiom => {
            config.search.ordering.precedence_modifiers.axiom_only =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::PrecSkolem => {
            config.search.ordering.precedence_modifiers.skolem =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::PrecDefPred => {
            config.search.ordering.precedence_modifiers.defpred =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::OrderConstantWeight => {
            let constant_weight = get_int_arg(parsed.option(), value)?;
            if constant_weight <= 0 {
                return Err(Diagnostic::new(
                    ErrorCode::USAGE_ERROR,
                    "Argument to option -c (--order-constant-weight) has to be > 0",
                ));
            }
            config.search.ordering.constant_weight = constant_weight;
        }
        EProverOption::Precedence => {
            config.search.ordering.precedence = Some(value.to_owned());
        }
        EProverOption::LpoRecursionLimit => {
            let recursion_limit = get_int_arg(parsed.option(), value)?;
            if recursion_limit <= 0 {
                return Err(Diagnostic::new(
                    ErrorCode::USAGE_ERROR,
                    "Argument to option --lpo-recursion-limit has to be > 0",
                ));
            }
            config.search.ordering.lpo_recursion_limit = recursion_limit;
            config.search.ordering.lpo_recursion_limit_changed = true;
            if recursion_limit > LPO_RECURSION_WARNING_LIMIT {
                config.warnings.push(Diagnostic::new(
                    ErrorCode::NO_ERROR,
                    LPO_RECURSION_LIMIT_WARNING,
                ));
            }
            config.search.ordering.literal_comparison = LiteralComparison::None;
        }
        EProverOption::RestrictLiteralComparisons => {
            config.search.ordering.literal_comparison = LiteralComparison::None;
        }
        EProverOption::LiteralComparison => set_literal_comparison(config, value)?,
        EProverOption::KboLambdaWeight => {
            config.search.ordering.lambda_weight = get_int_arg(parsed.option(), value)?;
        }
        EProverOption::KboDbWeight => {
            config.search.ordering.db_weight = get_int_arg(parsed.option(), value)?;
        }
        _ => unreachable!("non-term-ordering option routed to term-ordering handler"),
    }
    Ok(())
}

fn set_term_ordering(config: &mut EProverConfig, value: &str) -> Result<(), Diagnostic> {
    config.search.ordering.ordering = match value {
        "LPO" => TermOrdering::Lpo,
        "LPOCopy" => TermOrdering::LpoCopy,
        "LPO4" => TermOrdering::Lpo4,
        "LPO4Copy" => TermOrdering::Lpo4Copy,
        "KBO" => TermOrdering::Kbo,
        "KBO6" => TermOrdering::Kbo6,
        "RPO" => TermOrdering::Rpo,
        _ => {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                "Option -t (--term-ordering) requires LPO, LPO4, KBO, KBO6 or RPO as an argument",
            ));
        }
    };
    Ok(())
}

fn set_weight_generation(config: &mut EProverConfig, value: &str) -> Result<(), Diagnostic> {
    if WEIGHT_GENERATION_METHODS
        .iter()
        .position(|method| *method == value)
        .is_some_and(|index| index > 0)
    {
        value.clone_into(&mut config.search.ordering.weight_generation);
        Ok(())
    } else {
        Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Wrong argument to option -w (--order-weight-generation). Possible values: {}",
                WEIGHT_GENERATION_METHODS.join(", ")
            ),
        ))
    }
}

fn set_precedence_generation(config: &mut EProverConfig, value: &str) -> Result<(), Diagnostic> {
    if PRECEDENCE_GENERATION_METHODS
        .iter()
        .position(|method| *method == value)
        .is_some_and(|index| index > 0)
    {
        value.clone_into(&mut config.search.ordering.precedence_generation);
        Ok(())
    } else {
        Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Wrong argument to option -G (--order-precedence-generation). Possible values: {}",
                PRECEDENCE_GENERATION_METHODS.join(", ")
            ),
        ))
    }
}

fn set_literal_comparison(config: &mut EProverConfig, value: &str) -> Result<(), Diagnostic> {
    config.search.ordering.literal_comparison = match value {
        "None" => LiteralComparison::None,
        "Normal" => LiteralComparison::Normal,
        "TFOEqMax" => LiteralComparison::TfoEqMax,
        "TFOEqMin" => LiteralComparison::TfoEqMin,
        _ => {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                "Wrong argument to --literal-comparison (valid: None, Normal, TFOEqMax, TFOEqMin).",
            ));
        }
    };
    Ok(())
}

fn apply_search_control_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let option_code = parsed.option().option_code;
    if is_literal_selection_option(option_code) {
        apply_literal_selection_option(config, parsed)?;
    } else if is_heuristic_control_option(option_code) {
        apply_heuristic_control_option(config, parsed)?;
    } else if is_definition_option(option_code) {
        apply_definition_option(config, parsed);
    } else if is_input_symbol_option(option_code) {
        apply_input_symbol_option(config, option_code);
    } else if is_cnf_control_option(option_code) {
        apply_cnf_control_option(config, parsed)?;
    } else if is_preprocessing_elimination_option(option_code) {
        apply_preprocessing_elimination_option(config, parsed)?;
    } else if is_inference_control_option(option_code) {
        apply_inference_control_option(config, option_code);
    } else if is_inference_processing_option(option_code) {
        apply_inference_processing_option(config, parsed)?;
    } else if is_extension_inference_option(option_code) {
        apply_extension_inference_option(config, parsed)?;
    } else if is_higher_order_control_option(option_code) {
        apply_higher_order_control_option(config, parsed)?;
    } else if is_unification_option(option_code) {
        apply_unification_option(config, parsed)?;
    } else if is_watchlist_option(option_code) {
        apply_watchlist_option(config, parsed);
    } else if is_subsumption_index_option(option_code) {
        apply_subsumption_index_option(config, parsed)?;
    } else if is_fingerprint_index_option(option_code) {
        apply_fingerprint_index_option(config, parsed)?;
    } else if is_splitting_option(option_code) {
        apply_splitting_option(config, parsed)?;
    } else {
        unreachable!("non-search-control option routed to search-control handler");
    }
    Ok(())
}

fn apply_literal_selection_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::LiteralSelectionStrategy => set_literal_selection_strategy(config, value)?,
        EProverOption::NoGeneration => {
            "NoGeneration".clone_into(&mut config.search.literal_selection.strategy);
        }
        EProverOption::SelectOnProcessingOnly => {
            config.search.literal_selection.select_on_processing_only = true;
        }
        EProverOption::InheritParamodLiterals => {
            config
                .search
                .literal_selection
                .inherit_paramod_literals
                .any_clause = true;
        }
        EProverOption::InheritGoalParamodLiterals => {
            config
                .search
                .literal_selection
                .inherit_paramod_literals
                .goal_clause = true;
        }
        EProverOption::InheritConjectureParamodLiterals => {
            config
                .search
                .literal_selection
                .inherit_paramod_literals
                .conjecture_clause = true;
        }
        EProverOption::SelectionPosMin => {
            config.search.literal_selection.limits.positive_min =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::SelectionPosMax => {
            config.search.literal_selection.limits.positive_max =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::SelectionNegMin => {
            config.search.literal_selection.limits.negative_min =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::SelectionNegMax => {
            config.search.literal_selection.limits.negative_max =
                get_int_arg(parsed.option(), value)?;
        }
        EProverOption::SelectionAllMin => {
            config.search.literal_selection.limits.all_min = get_int_arg(parsed.option(), value)?;
        }
        EProverOption::SelectionAllMax => {
            config.search.literal_selection.limits.all_max = get_int_arg(parsed.option(), value)?;
        }
        EProverOption::SelectionWeightMin => {
            config.search.literal_selection.limits.weight_min =
                get_int_arg(parsed.option(), value)?;
        }
        _ => unreachable!("non-literal-selection option routed to literal-selection handler"),
    }
    Ok(())
}

fn set_literal_selection_strategy(
    config: &mut EProverConfig,
    value: &str,
) -> Result<(), Diagnostic> {
    if LITERAL_SELECTION_STRATEGIES.contains(&value) {
        value.clone_into(&mut config.search.literal_selection.strategy);
        Ok(())
    } else {
        Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Wrong argument to option -W (--literal-selection-strategy). Possible values: {}",
                LITERAL_SELECTION_STRATEGIES.join(", ")
            ),
        ))
    }
}

fn apply_heuristic_control_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::PreferInitialClauses => {
            config.search.heuristic.prefer_initial_clauses = true;
        }
        EProverOption::ExpertHeuristic => {
            value.clone_into(&mut config.search.heuristic.name);
        }
        EProverOption::FilterOrphansLimit => {
            config.search.heuristic.filter_orphans_limit = get_int_arg(parsed.option(), value)?;
        }
        EProverOption::ForwardContractLimit => {
            config.search.heuristic.forward_contract_limit = get_int_arg(parsed.option(), value)?;
        }
        EProverOption::DeleteBadLimit => {
            config.delete_bad_limit = get_int_arg(parsed.option(), value)?;
        }
        _ => unreachable!("non-heuristic-control option routed to heuristic-control handler"),
    }
    Ok(())
}

fn apply_definition_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    let value = parsed.arg().unwrap_or("").to_owned();
    match parsed.option().option_code {
        EProverOption::DefineWeightFunction => {
            config
                .search
                .heuristic
                .weight_function_definitions
                .push(value);
        }
        EProverOption::DefineHeuristic => {
            config.search.heuristic.heuristic_definitions.push(value);
        }
        _ => unreachable!("non-definition option routed to definition handler"),
    }
}

fn apply_input_symbol_option(config: &mut EProverConfig, option: EProverOption) {
    match option {
        EProverOption::FreeNumbers => {
            config.free_symbol_properties |= FP_IS_INTEGER | FP_IS_RATIONAL | FP_IS_FLOAT;
        }
        EProverOption::FreeObjects => {
            config.free_symbol_properties |= FP_IS_OBJECT;
        }
        _ => unreachable!("non-input-symbol option routed to input-symbol handler"),
    }
}

fn apply_cnf_control_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::DefinitionalCnf => {
            config.preprocessing.formula_def_limit =
                get_int_arg_check_range(parsed.option(), value, 0, i64::MAX)?;
        }
        EProverOption::FoolUnroll => {
            config.preprocessing.fool_unroll =
                FoolUnroll::from(get_bool_arg(parsed.option(), value)?);
        }
        EProverOption::MiniscopeLimit => {
            config.preprocessing.miniscope_limit =
                get_int_arg_check_range(parsed.option(), value, 0, i64::MAX)?;
        }
        EProverOption::PrintTypes => config.encoding.print_types = true,
        EProverOption::AppEncode => config.encoding.app_encode = true,
        EProverOption::ClassificationTimeoutPortion => {
            config.preprocessing.classification_timeout_percentage =
                get_int_arg_check_range(parsed.option(), value, 1, 99)?;
            config
                .search
                .inference
                .higher_order_preprocessing
                .preinstantiate_induction = get_bool_arg(parsed.option(), value)?;
        }
        _ => unreachable!("non-CNF-control option routed to CNF-control handler"),
    }
    Ok(())
}

fn apply_preprocessing_elimination_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::Bce => {
            config.preprocessing.bce.enabled = get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::BceMaxOccs => {
            config.preprocessing.bce.max_occs =
                get_int_arg_check_range(parsed.option(), value, -1, C_INT_MAX)?;
        }
        EProverOption::PredElim => {
            config.preprocessing.predicate_elimination.enabled =
                get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::PredElimRecognizeGates => {
            let enabled = get_bool_arg(parsed.option(), value)?;
            config
                .preprocessing
                .predicate_elimination
                .flags
                .set(PredicateEliminationFlag::RecognizeGates, enabled);
        }
        EProverOption::PredElimForceMuDecrease => {
            let enabled = get_bool_arg(parsed.option(), value)?;
            config
                .preprocessing
                .predicate_elimination
                .flags
                .set(PredicateEliminationFlag::ForceMuDecrease, enabled);
        }
        EProverOption::PredElimIgnoreConjSyms => {
            let enabled = get_bool_arg(parsed.option(), value)?;
            config
                .preprocessing
                .predicate_elimination
                .flags
                .set(PredicateEliminationFlag::IgnoreConjectureSymbols, enabled);
        }
        EProverOption::PredElimMaxOccs => {
            config.preprocessing.predicate_elimination.max_occs =
                get_int_arg_check_range(parsed.option(), value, -1, C_INT_MAX)?;
        }
        EProverOption::PredElimTolerance => {
            config.preprocessing.predicate_elimination.tolerance =
                get_int_arg_check_range(parsed.option(), value, 0, C_INT_MAX)?;
        }
        _ => unreachable!("non-preprocessing-elimination option routed to handler"),
    }
    Ok(())
}

fn apply_inference_control_option(config: &mut EProverConfig, option: EProverOption) {
    match option {
        EProverOption::AssumeCompleteness => {
            config.search.completeness.assume_inference_system_complete = true;
        }
        EProverOption::AssumeIncompleteness => {
            config.search.completeness.incomplete = true;
        }
        EProverOption::DisableEqFactoring => {
            config.search.inference.enable_eq_factoring = false;
            config.search.completeness.inference_system_complete = false;
        }
        EProverOption::DisableParamodIntoNegUnits => {
            config.search.inference.enable_neg_unit_paramod = false;
            config.search.completeness.inference_system_complete = false;
        }
        EProverOption::Condense => config.search.inference.condensing.enabled = true,
        EProverOption::CondenseAggressive => {
            config.search.inference.condensing.enabled = true;
            config.search.inference.condensing.aggressive = true;
        }
        EProverOption::DisableGivenClauseForwardContraction => {
            config.search.inference.enable_given_forward_simplification = false;
        }
        EProverOption::SimulParamod => {
            config.search.inference.paramodulation = ParamodulationType::Sim;
        }
        EProverOption::OrientedSimulParamod => {
            config.search.inference.paramodulation = ParamodulationType::OrientedSim;
        }
        EProverOption::SupersimulParamod => {
            config.search.inference.paramodulation = ParamodulationType::SuperSim;
        }
        EProverOption::OrientedSupersimulParamod => {
            config.search.inference.paramodulation = ParamodulationType::OrientedSuperSim;
        }
        _ => unreachable!("non-inference-control option routed to inference-control handler"),
    }
}

fn apply_inference_processing_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::SosUsesInputTypes => config.search.support.use_tptp_sos = true,
        EProverOption::DestructiveEr => {
            config.search.inference.equality_resolution.destructive = true;
        }
        EProverOption::StrongDestructiveEr => {
            config.search.inference.equality_resolution.destructive = true;
            config
                .search
                .inference
                .equality_resolution
                .strong_destructive = true;
        }
        EProverOption::DestructiveErAggressive => {
            config.search.inference.equality_resolution.aggressive = true;
        }
        EProverOption::ForwardContextSr => {
            config.search.inference.context_simplification.forward = true;
        }
        EProverOption::ForwardContextSrAggressive => {
            config.search.inference.context_simplification.forward = true;
            config
                .search
                .inference
                .context_simplification
                .forward_aggressive = true;
        }
        EProverOption::BackwardContextSr => {
            config.search.inference.context_simplification.backward = true;
        }
        EProverOption::PreferGeneralDemodulators => {
            config.search.inference.demodulation.prefer_general = true;
        }
        EProverOption::ForwardDemodLevel => set_forward_demod_level(config, parsed, value)?,
        EProverOption::DemodUnderLambda => {
            config.search.inference.demodulation.lambda_demod =
                get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::StrongRwInst => {
            config.search.ordering.rewrite_strong_rhs_inst = true;
        }
        EProverOption::StrongForwardSubsumption => {
            config.search.support.strong_unit_forward_subsumption = true;
        }
        EProverOption::SatCheckProcInterval => {
            config.search.sat_check.step_limit =
                get_int_arg_check_range(parsed.option(), value, 1, i64::MAX)?;
        }
        EProverOption::SatCheckGenInterval => {
            config.search.sat_check.size_limit =
                get_int_arg_check_range(parsed.option(), value, 1, i64::MAX)?;
        }
        EProverOption::SatCheckTTInsertInterval => {
            config.search.sat_check.ttinsert_limit =
                get_int_arg_check_range(parsed.option(), value, 1, i64::MAX)?;
        }
        EProverOption::SatCheck => set_sat_check_grounding(config, value)?,
        EProverOption::SatCheckDecisionLimit => {
            config.search.sat_check.decision_limit =
                get_int_arg_check_range(parsed.option(), value, -1, C_INT_MAX)?;
        }
        EProverOption::SatCheckNormalizeConst => config.search.sat_check.normconst = true,
        EProverOption::SatCheckNormalizeUnproc => config.search.sat_check.normalize = true,
        EProverOption::LiftLambdas => {
            config.search.support.lift_lambdas = get_bool_arg(parsed.option(), value)?;
        }
        _ => unreachable!("non-inference-processing option routed to handler"),
    }
    Ok(())
}

fn apply_extension_inference_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let mode = parse_ext_inference_mode(parsed.option().option_code, parsed.arg().unwrap_or(""))?;
    match parsed.option().option_code {
        EProverOption::ArgCong => config.search.inference.higher_order.arg_cong = mode,
        EProverOption::NegExt => config.search.inference.higher_order.neg_ext = mode,
        EProverOption::PosExt => config.search.inference.higher_order.pos_ext = mode,
        _ => unreachable!("non-extension-inference option routed to extension handler"),
    }
    Ok(())
}

fn parse_ext_inference_mode(
    option: EProverOption,
    value: &str,
) -> Result<ExtInferenceType, Diagnostic> {
    match value {
        "all" => Ok(ExtInferenceType::AllLits),
        "max" => Ok(ExtInferenceType::MaxLits),
        "off" => Ok(ExtInferenceType::NoLits),
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            ext_inference_error_message(option),
        )),
    }
}

fn ext_inference_error_message(option: EProverOption) -> &'static str {
    match option {
        EProverOption::ArgCong => "neg-ext excepts either all, max or off",
        EProverOption::NegExt => "neg-ext excepts either all or max",
        EProverOption::PosExt => "pos-ext excepts either all or max",
        _ => unreachable!("non-extension-inference option routed to extension error helper"),
    }
}

fn apply_higher_order_control_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::ExtSupMaxDepth => {
            config.search.inference.higher_order.ext_rules_max_depth =
                get_int_arg_check_range(parsed.option(), value, -1, C_INT_MAX)?;
        }
        EProverOption::InverseRecognition => {
            config.search.inference.higher_order.inverse_recognition = true;
        }
        EProverOption::ReplaceInjDefs => {
            config.search.inference.higher_order.replace_inj_defs = true;
        }
        EProverOption::CnfLambdaToForall => {
            config
                .search
                .inference
                .higher_order_preprocessing
                .lambda_to_forall = get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::EtaNormalize => {
            config
                .search
                .inference
                .higher_order_preprocessing
                .eta_normalization = parse_eta_normalization(value)?;
        }
        EProverOption::HoOrderKind => {
            config.search.ordering.ho_order_kind = parse_ho_order_kind(value)?;
        }
        EProverOption::EliminateLeibnizEq => {
            config
                .search
                .inference
                .higher_order_preprocessing
                .elim_leibniz_max_depth =
                get_int_arg_check_range(parsed.option(), value, -1, C_INT_MAX)?;
        }
        EProverOption::UnrollFormulasOnly => {
            config
                .search
                .inference
                .higher_order_preprocessing
                .unroll_only_formulas = get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::PrimEnumMode => {
            config
                .search
                .inference
                .higher_order_search
                .primitive_enumeration
                .mode = parse_prim_enum_mode(value)?;
        }
        EProverOption::PrimEnumMaxDepth => {
            config
                .search
                .inference
                .higher_order_search
                .primitive_enumeration
                .max_depth = get_int_arg_check_range(parsed.option(), value, -1, C_INT_MAX)?;
        }
        EProverOption::InstChoiceMaxDepth => {
            config
                .search
                .inference
                .higher_order_preprocessing
                .inst_choice_max_depth =
                get_int_arg_check_range(parsed.option(), value, -1, C_INT_MAX)?;
        }
        EProverOption::LocalRw => {
            config.search.inference.higher_order_search.local_rw =
                get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::PruneArgs => {
            config.search.inference.higher_order_search.prune_args =
                get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::PreinstantiateInduction => {
            config
                .search
                .inference
                .higher_order_preprocessing
                .preinstantiate_induction = get_bool_arg(parsed.option(), value)?;
        }
        _ => unreachable!("non-higher-order-control option routed to handler"),
    }
    Ok(())
}

fn parse_eta_normalization(value: &str) -> Result<EtaNormalization, Diagnostic> {
    match value {
        "reduce" => Ok(EtaNormalization::Reduce),
        "expand" => Ok(EtaNormalization::Expand),
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Option --eta-normalize requires 'reduce' or 'expand' as an argument",
        )),
    }
}

fn parse_ho_order_kind(value: &str) -> Result<HoOrderKind, Diagnostic> {
    match value {
        "lfho" => Ok(HoOrderKind::LfhoOrder),
        "lambda" => Ok(HoOrderKind::LambdaOrder),
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Option --ho-order-kind requires 'lfho' or 'lambda' as an argument",
        )),
    }
}

fn parse_prim_enum_mode(value: &str) -> Result<PrimEnumMode, Diagnostic> {
    match value {
        "neg" => Ok(PrimEnumMode::Neg),
        "and" => Ok(PrimEnumMode::And),
        "or" => Ok(PrimEnumMode::Or),
        "eq" => Ok(PrimEnumMode::Eq),
        "pragmatic" => Ok(PrimEnumMode::Pragmatic),
        "full" => Ok(PrimEnumMode::Full),
        "logsym" => Ok(PrimEnumMode::LogSymbol),
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Option --prim-enum-mode excepts neg, and, or, eq, pragmatic, full, or logsym",
        )),
    }
}

fn apply_unification_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    let unification = &mut config.search.inference.higher_order_unification;
    match parsed.option().option_code {
        EProverOption::FuncProjLimit => {
            unification.func_proj_limit = get_int_arg_check_range(parsed.option(), value, 0, 63)?;
        }
        EProverOption::ImitLimit => {
            unification.imit_limit = get_int_arg_check_range(parsed.option(), value, 0, 63)?;
        }
        EProverOption::IdentLimit => {
            unification.ident_limit = get_int_arg_check_range(parsed.option(), value, 0, 63)?;
        }
        EProverOption::ElimLimit => {
            unification.elim_limit = get_int_arg_check_range(parsed.option(), value, 0, 63)?;
        }
        EProverOption::UnifMode => {
            unification.mode = parse_unification_mode(value)?;
        }
        EProverOption::PatternOracle => {
            unification.pattern_oracle = get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::FixpointOracle => {
            unification.fixpoint_oracle = get_bool_arg(parsed.option(), value)?;
        }
        EProverOption::MaxUnifiers => {
            unification.max_unifiers = get_int_arg_check_range(parsed.option(), value, 0, 1024)?;
        }
        EProverOption::MaxUnifSteps => {
            unification.max_unif_steps =
                get_int_arg_check_range(parsed.option(), value, 0, 100_000)?;
        }
        _ => unreachable!("non-unification option routed to unification handler"),
    }
    Ok(())
}

fn parse_unification_mode(value: &str) -> Result<UnificationMode, Diagnostic> {
    match value {
        "single" => Ok(UnificationMode::Single),
        "multi" => Ok(UnificationMode::Multi),
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "values of unif mode are eiter single or multi",
        )),
    }
}

fn set_forward_demod_level(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
    value: &str,
) -> Result<(), Diagnostic> {
    config.search.inference.demodulation.forward_demod =
        match get_int_arg_check_range(parsed.option(), value, 0, 2)? {
            0 => RewriteLevel::NoRewrite,
            1 => RewriteLevel::RuleRewrite,
            2 => RewriteLevel::FullRewrite,
            _ => unreachable!("forward demodulation range was already checked"),
        };
    Ok(())
}

fn set_sat_check_grounding(config: &mut EProverConfig, value: &str) -> Result<(), Diagnostic> {
    config.search.sat_check.grounding = match value {
        "NoGrounding" => GroundingStrategy::NoGrounding,
        "PseudoVar" => GroundingStrategy::PseudoVar,
        "FirstConst" => GroundingStrategy::FirstConst,
        "ConjMinMinFreq" => GroundingStrategy::ConjMinMinFreq,
        "ConjMaxMinFreq" => GroundingStrategy::ConjMaxMinFreq,
        "ConjMinMaxFreq" => GroundingStrategy::ConjMinMaxFreq,
        "ConjMaxMaxFreq" => GroundingStrategy::ConjMaxMaxFreq,
        "GlobalMax" => GroundingStrategy::GlobalMax,
        "GlobalMin" => GroundingStrategy::GlobalMin,
        _ => {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                format!(
                    "Wrong argument to option --sat-check. Possible values: {}",
                    GROUNDING_STRATEGY_NAMES.join(", ")
                ),
            ));
        }
    };
    Ok(())
}

fn apply_watchlist_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    match parsed.option().option_code {
        EProverOption::Watchlist => set_watchlist_source(config, parsed.arg().unwrap_or("")),
        EProverOption::StaticWatchlist => {
            config.search.watchlist.is_static = true;
            set_watchlist_source(config, parsed.arg().unwrap_or(""));
        }
        EProverOption::NoWatchlistSimplification => {
            config.search.watchlist.simplify = false;
        }
        _ => unreachable!("non-watchlist option routed to watchlist handler"),
    }
}

fn set_watchlist_source(config: &mut EProverConfig, value: &str) {
    config.search.watchlist.source = Some(
        if value == WATCHLIST_INLINE_STRING || value == WATCHLIST_INLINE_QSTRING {
            WatchlistSource::Inline
        } else {
            WatchlistSource::File(value.to_owned())
        },
    );
}

fn apply_subsumption_index_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::ForwardSubsumptionAggressive => {
            config.search.inference.subsumption.forward_aggressive = true;
        }
        EProverOption::ConventionalSubsumption => {
            config.search.fv_index.feature_type = FvIndexFeatureType::NoFeatures;
        }
        EProverOption::SubsumptionIndexing => set_subsumption_indexing(config, value)?,
        EProverOption::FvIndexFeatureTypes => set_fv_index_feature_type(config, value)?,
        EProverOption::FvIndexMaxFeatures => {
            let max_symbols = get_int_arg(parsed.option(), value)?;
            if max_symbols <= 0 {
                return Err(Diagnostic::new(
                    ErrorCode::USAGE_ERROR,
                    "Argument to option --fvindex-maxfeatures has to be > 0",
                ));
            }
            config.search.fv_index.max_symbols =
                get_int_arg_check_range(parsed.option(), value, 0, i64::MAX)?;
        }
        EProverOption::FvIndexSlack => {
            config.search.fv_index.symbol_slack =
                get_int_arg_check_range(parsed.option(), value, 0, i64::MAX)?;
        }
        _ => unreachable!("non-subsumption-index option routed to handler"),
    }
    Ok(())
}

fn set_subsumption_indexing(config: &mut EProverConfig, value: &str) -> Result<(), Diagnostic> {
    match value {
        "None" => {
            config.search.fv_index.feature_type = FvIndexFeatureType::NoFeatures;
            Ok(())
        }
        "Direct" => {
            config.search.fv_index.use_perm_vectors = false;
            Ok(())
        }
        "Perm" => {
            config.search.fv_index.use_perm_vectors = true;
            config.search.fv_index.eliminate_uninformative = false;
            Ok(())
        }
        "PermOpt" => {
            config.search.fv_index.use_perm_vectors = true;
            config.search.fv_index.eliminate_uninformative = true;
            Ok(())
        }
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Option --subsumption-indexing requires 'None', 'Direct', 'Perm', or 'PermOpt'.",
        )),
    }
}

fn set_fv_index_feature_type(config: &mut EProverConfig, value: &str) -> Result<(), Diagnostic> {
    config.search.fv_index.feature_type = match value {
        "None" => FvIndexFeatureType::NoFeatures,
        "AC" => FvIndexFeatureType::AcFeatures,
        "SS" => FvIndexFeatureType::SsFeatures,
        "All" => FvIndexFeatureType::AllFeatures,
        "Bill" => FvIndexFeatureType::BillFeatures,
        "BillPlus" => FvIndexFeatureType::BillPlusFeatures,
        "ACFold" => FvIndexFeatureType::AcFold,
        "ACStagger" => FvIndexFeatureType::AcStagger,
        _ => {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                "Option --fvindex-featuretypes requires 'None', 'AC', 'SS', 'All', 'Bill', 'BillPlus', 'ACFold', 'ACStagger'.",
            ));
        }
    };
    Ok(())
}

fn apply_fingerprint_index_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::RewriteBackwardIndex => {
            check_fp_index_arg(value, "--rw-bw-index")?;
            value.clone_into(&mut config.search.fingerprint_index.rw_bw_index_type);
        }
        EProverOption::ParamodFromIndex => {
            check_fp_index_arg(value, "--pm-from-index")?;
            value.clone_into(&mut config.search.fingerprint_index.pm_from_index_type);
        }
        EProverOption::ParamodIntoIndex => {
            check_fp_index_arg(value, "--pm-into-index")?;
            value.clone_into(&mut config.search.fingerprint_index.pm_into_index_type);
        }
        EProverOption::FingerprintIndex => {
            check_fp_index_arg(value, "--fp-index")?;
            value.clone_into(&mut config.search.fingerprint_index.rw_bw_index_type);
            value.clone_into(&mut config.search.fingerprint_index.pm_from_index_type);
            value.clone_into(&mut config.search.fingerprint_index.pm_into_index_type);
        }
        EProverOption::FingerprintNoSizeConstr => {}
        EProverOption::PdtNoSizeConstr => {
            config.search.fingerprint_index.pdt_use_size_constraints = false;
        }
        EProverOption::PdtNoAgeConstr => {
            config.search.fingerprint_index.pdt_use_age_constraints = false;
        }
        _ => unreachable!("non-fingerprint-index option routed to handler"),
    }
    Ok(())
}

fn check_fp_index_arg(value: &str, option_name: &str) -> Result<(), Diagnostic> {
    if FP_INDEX_NAMES.contains(&value) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Wrong argument to option {option_name}. Possible values: {}",
                FP_INDEX_NAMES.join(", ")
            ),
        ))
    }
}

fn apply_splitting_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = parsed.arg().unwrap_or("");
    match parsed.option().option_code {
        EProverOption::SplitClauses => {
            config.search.splitting.classes = get_int_arg(parsed.option(), value)?;
        }
        EProverOption::SplitMethod => {
            config.search.splitting.method = get_int_arg_check_range(parsed.option(), value, 0, 2)?;
        }
        EProverOption::SplitAggressive => config.search.splitting.aggressive = true,
        EProverOption::SplitReuseDefs => config.search.splitting.fresh_defs = false,
        EProverOption::DisequalityDecomposition => {
            config.search.splitting.diseq_decomposition = get_int_arg(parsed.option(), value)?;
        }
        EProverOption::DisequalityDecompMaxArity => {
            config.search.splitting.diseq_decomp_maxarity = get_int_arg(parsed.option(), value)?;
        }
        _ => unreachable!("non-splitting option routed to splitting handler"),
    }
    Ok(())
}

fn apply_limit_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
    match parsed.option().option_code {
        EProverOption::ProcessedClausesLimit => config.step_limit = value,
        EProverOption::ProcessedSetLimit => config.processed_set_limit = value,
        EProverOption::UnprocessedLimit => config.unprocessed_limit = value,
        EProverOption::TotalClauseSetLimit => config.total_clause_set_limit = value,
        EProverOption::GeneratedLimit => config.generated_limit = value,
        EProverOption::TermBankInsertLimit => config.term_bank_insert_limit = value,
        EProverOption::Answers => config.answer_limit = value,
        _ => unreachable!("non-limit option routed to limit handler"),
    }
    Ok(())
}

fn apply_format_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    match parsed.option().option_code {
        EProverOption::EqnNoInfix => config.equation_print.use_infix = false,
        EProverOption::FullEquationalRep => config.equation_print.full_equational_rep = true,
        EProverOption::PrintOrientedEqLitsAsRules => {
            config.equation_print.print_oriented = true;
        }
        EProverOption::LopIn => config.parse_format = IoFormat::Lop,
        EProverOption::PclOut => config.doc_output_format = DocOutputFormat::Pcl,
        EProverOption::TptpIn => config.parse_format = IoFormat::Tptp,
        EProverOption::TptpOut => {
            config.output_format = IoFormat::Tptp;
            config.equation_print.full_equational_rep = false;
            config.equation_print.use_infix = false;
        }
        EProverOption::TptpFormat => {
            config.parse_format = IoFormat::Tptp;
            config.output_format = IoFormat::Tptp;
            config.equation_print.full_equational_rep = false;
            config.equation_print.use_infix = false;
        }
        EProverOption::TstpIn => config.parse_format = IoFormat::Tstp,
        EProverOption::TstpOut => {
            config.doc_output_format = DocOutputFormat::Tstp;
            config.output_format = IoFormat::Tstp;
            config.equation_print.use_infix = true;
        }
        EProverOption::TstpFormat => {
            config.parse_format = IoFormat::Tstp;
            config.doc_output_format = DocOutputFormat::Tstp;
            config.output_format = IoFormat::Tstp;
            config.equation_print.use_infix = true;
        }
        _ => unreachable!("non-format option routed to format handler"),
    }
}

fn apply_input_mode_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    match parsed.option().option_code {
        EProverOption::SyntaxOnly => config.flags.set(EProverFlag::SyntaxOnly),
        EProverOption::PrintFormulas => {
            config.flags.set(EProverFlag::SyntaxOnly);
            config.flags.set(EProverFlag::PrintFormulas);
        }
        EProverOption::PruneOnly => {
            config.output_level = 4;
            config.flags.set(EProverFlag::PruneOnly);
        }
        EProverOption::CnfOnly => {
            "teigEIG".clone_into(&mut config.saturated_output_descriptor);
            config.processed_set_limit = 0;
            config.flags.set(EProverFlag::PrintSaturated);
            config.flags.set(EProverFlag::CnfOnly);
        }
        _ => unreachable!("non-input-mode option routed to input-mode handler"),
    }
}

fn apply_simple_flag(config: &mut EProverConfig, option: EProverOption) {
    match option {
        EProverOption::PrintPid => config.flags.set(EProverFlag::PrintPid),
        EProverOption::PrintVersion => config.flags.set(EProverFlag::PrintVersion),
        EProverOption::RequireNonempty => config.flags.set(EProverFlag::RequireNonempty),
        EProverOption::ResourcesInfo => config.flags.set(EProverFlag::ResourceInfo),
        EProverOption::ConjecturesAreQuestions => {
            config.flags.set(EProverFlag::ConjecturesAreQuestions);
        }
        EProverOption::DeterministicRewriteSort => {
            config.flags.set(EProverFlag::DeterministicRewriteSort);
        }
        EProverOption::DeterministicNewSort => {
            config.flags.set(EProverFlag::DeterministicNewSort);
        }
        _ => unreachable!("non-simple flag routed to simple-flag handler"),
    }
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\nE {VERSION} \"{E_NICKNAME}\"\n\n\
Usage: {PROGRAM_NAME} [options] [files]\n\n\
Read a set of first-order (or, in the -ho-version, higher-order)\n\
clauses and formulae and try to prove the conjecture (if given)\n\
or show the set unsatisfiable.\n\n"
    );
    result.push_str(&print_options(EPROVER_OPTIONS, Some("Options:\n\n")));
    result.push_str("\n\n");
    result.push_str(&version::footer());
    result
}

fn run_config(stdout: &mut impl Write, config: &EProverConfig) -> Result<u8, EProverError> {
    let mut runtime_config = config.clone();
    let verbose = i32::try_from(config.verbose).map_err(|_| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("--verbose argument {} is out of int range", config.verbose),
        )
    })?;
    set_verbose_level(verbose);
    let _ = set_output_level(config.output_level);
    apply_time_limit_state(config);
    apply_ordering_state(config);
    let mut output = open_configured_output(stdout, config.output_file.as_deref())?;

    if config.flags.contains(EProverFlag::PrintPid) {
        writeln!(output, "{DEFAULT_COMCHAR_RAW} Pid: {}", std::process::id())?;
    }
    if config.flags.contains(EProverFlag::PrintVersion) {
        writeln!(output, "{DEFAULT_COMCHAR_RAW} Version: {VERSION}")?;
    }
    output.flush()?;

    if config.print_strategy.is_some() {
        run_print_strategy(&mut output, config)?;
        return finish_run_config(&mut output, config, ErrorCode::NO_ERROR.exit_status());
    }

    if config.flags.contains(EProverFlag::SyntaxOnly) {
        run_syntax_only(&mut output, &mut runtime_config)?;
        if !runtime_config.flags.contains(EProverFlag::PrintFormulas) {
            write_syntax_only_success(&mut output)?;
        }
        return finish_run_config(&mut output, config, ErrorCode::NO_ERROR.exit_status());
    }

    if config.encoding.app_encode {
        run_app_encode(&mut output, &mut runtime_config)?;
        return finish_run_config(&mut output, config, ErrorCode::NO_ERROR.exit_status());
    }

    if config.flags.contains(EProverFlag::PruneOnly) {
        run_prune_only(&mut output, &mut runtime_config)?;
        return finish_run_config(&mut output, config, ErrorCode::NO_ERROR.exit_status());
    }

    let status = run_proof_search(&mut output, &mut runtime_config)?;
    finish_run_config(&mut output, config, status)
}

fn finish_run_config(
    output: &mut impl Write,
    config: &EProverConfig,
    status: u8,
) -> Result<u8, EProverError> {
    if config.flags.contains(EProverFlag::ResourceInfo) {
        output.write_all(format_resource_usage(current_resource_usage()).as_bytes())?;
    }
    output.flush()?;
    Ok(status)
}

fn run_print_strategy(output: &mut impl Write, config: &EProverConfig) -> Result<(), EProverError> {
    let Some(print_strategy) = config.print_strategy.as_deref() else {
        return Ok(());
    };
    let mut params = heuristic_parms_with_strategy_io(config)?;
    match print_strategy {
        ">all-strats<" => {
            output.write_all(strategies_print_predefined_string(false)?.as_bytes())?;
        }
        ">all-names<" => {
            output.write_all(strategies_print_predefined_string(true)?.as_bytes())?;
        }
        strategy_name => {
            if strategy_name != ">current-strategy<" {
                get_heuristic_with_name(strategy_name, &mut params)?;
            }
            output.write_all(heuristic_parms_strategy_print_string(&params).as_bytes())?;
        }
    }
    Ok(())
}

fn run_syntax_only(
    output: &mut impl Write,
    config: &mut EProverConfig,
) -> Result<(), EProverError> {
    let mut bank = temporary_executable_term_bank(config.free_symbol_properties)?;
    let mut clauses = ClauseSet::new();
    let mut watchlist = ClauseSet::new();

    config.flags.clear(EProverFlag::FormulaConjectureSeen);
    let files = config.files.clone();
    for file in &files {
        let before = clauses.len();
        let parsed_file = parse_clause_file(
            file,
            config.parse_format,
            FormulaPreprocessing::PARSE_ONLY,
            &mut bank,
            &mut clauses,
            &mut watchlist,
        )?;
        if parsed_file.formula_conjecture_seen {
            config.flags.set(EProverFlag::FormulaConjectureSeen);
        }
        apply_auto_parse_output_side_effects(config, parsed_file.detected_format);
        if config.flags.contains(EProverFlag::RequireNonempty) && clauses.len() == before {
            return Err(Diagnostic::new(
                ErrorCode::INPUT_SEMANTIC_ERROR,
                format!("Input file {file} did not contain any clauses"),
            )
            .into());
        }
    }

    if config.flags.contains(EProverFlag::RequireNonempty) && clauses.is_empty() {
        return Err(Diagnostic::new(
            ErrorCode::INPUT_SEMANTIC_ERROR,
            "Input did not contain any clauses",
        )
        .into());
    }

    if config.flags.contains(EProverFlag::PrintFormulas) {
        match config.output_format {
            IoFormat::Tptp => {
                let options = EqnPrintOptions::tptp().with_print_types(config.encoding.print_types);
                output.write_all(
                    clauses
                        .print_tptp_format_string_with_options(&bank, options)
                        .as_bytes(),
                )?;
            }
            IoFormat::Tstp => {
                let rendered = clauses.tstp_print_string_with_type_suffixes(
                    &bank,
                    true,
                    crate::basics::simple_stuff::ProblemType::FirstOrder,
                    config.encoding.print_types,
                )?;
                output.write_all(rendered.as_bytes())?;
            }
            IoFormat::Auto | IoFormat::Lop => {
                let rendered = clauses.print_lop_string_with_options(
                    &bank,
                    true,
                    config
                        .equation_print
                        .into_eqn_print_options(config.output_format)
                        .with_print_types(config.encoding.print_types),
                );
                output.write_all(rendered.as_bytes())?;
            }
        }
    }

    Ok(())
}

fn write_syntax_only_success(output: &mut impl Write) -> Result<(), EProverError> {
    write_comment_line_after_blank(output, "Parsing successful!")?;
    write_tstp_status(output, "Unknown")?;
    Ok(())
}

fn run_app_encode<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &mut EProverConfig,
) -> Result<(), EProverError> {
    let mut bank = temporary_executable_term_bank(config.free_symbol_properties)?;
    let mut formulas = Vec::new();
    let mut saw_any_input_owner = false;
    let mut saw_any_formula_owner = false;

    let files = config.files.clone();
    for file in &files {
        let parsed_file = parse_app_encode_file(file, config.parse_format, &mut bank)?;
        apply_auto_parse_output_side_effects(config, parsed_file.detected_format);
        if config.flags.contains(EProverFlag::RequireNonempty) && !parsed_file.saw_input_owner {
            return Err(Diagnostic::new(
                ErrorCode::INPUT_SEMANTIC_ERROR,
                format!("Input file {file} did not contain any clauses"),
            )
            .into());
        }
        saw_any_input_owner |= parsed_file.saw_input_owner;
        saw_any_formula_owner |= parsed_file.saw_formula_owner;
        formulas.extend(parsed_file.formulas);
    }

    if config.flags.contains(EProverFlag::RequireNonempty) && !saw_any_input_owner {
        return Err(Diagnostic::new(
            ErrorCode::INPUT_SEMANTIC_ERROR,
            "Input did not contain any clauses",
        )
        .into());
    }

    write_preprocessing_config_debug_line(output, config)?;
    write_app_encoded_formula_set(output, &mut bank, &formulas, saw_any_formula_owner)?;
    Ok(())
}

fn temporary_executable_term_bank(
    free_symbol_properties: FunctionProperties,
) -> Result<TermBank, Diagnostic> {
    let mut signature = Signature::new(TypeBank::new());
    signature.insert_internal_codes()?;
    signature.remove_distinct_props(free_symbol_properties);
    TermBank::new(signature)
}

fn write_app_encoded_formula_set<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    bank: &mut TermBank,
    formulas: &[SimpleAppEncodedFormula],
    saw_formula_owner: bool,
) -> Result<(), EProverError> {
    if !saw_formula_owner {
        return Ok(());
    }

    for formula in formulas {
        simple_fof_preload_app_encoded_formulas(&formula.formulas, bank)?;
    }

    let mut rendered = Vec::new();
    bank.signature()
        .type_bank()
        .app_encode_types(&mut rendered, ProblemType::FirstOrder, true)?;
    bank.signature().print_app_encoded_decls(&mut rendered)?;

    for formula in formulas {
        if simple_fof_formulas_are_app_encoded_prop_true(&formula.formulas) {
            continue;
        }

        let mut formula_text = String::new();
        write!(
            formula_text,
            "tff({}, {}, ",
            formula.name,
            app_encoded_formula_role(formula.type_)
        )
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
        simple_fof_write_app_encoded_formulas(&mut formula_text, bank, &formula.formulas)?;
        formula_text.push_str(").\n");
        rendered.extend_from_slice(formula_text.as_bytes());
    }

    output.write_stdout_side_channel(&rendered)?;
    Ok(())
}

fn simple_fof_preload_app_encoded_formulas(
    formulas: &[SimpleFofFormula],
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    for formula in formulas {
        simple_fof_preload_app_encoded_formula(formula, bank)?;
    }
    Ok(())
}

fn simple_fof_preload_app_encoded_formula(
    formula: &SimpleFofFormula,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    match formula {
        SimpleFofFormula::Truth(_) => {}
        SimpleFofFormula::Literal(literal) => {
            simple_fof_preload_app_encoded_literal(literal, bank)?;
        }
        SimpleFofFormula::Implication {
            antecedents,
            consequents,
        }
        | SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents,
        } => {
            simple_fof_preload_app_encoded_formulas(antecedents, bank)?;
            simple_fof_preload_app_encoded_formulas(consequents, bank)?;
        }
        SimpleFofFormula::Equivalence { left, right }
        | SimpleFofFormula::Xor { left, right }
        | SimpleFofFormula::Nand { left, right }
        | SimpleFofFormula::Nor { left, right } => {
            simple_fof_preload_app_encoded_formulas(left, bank)?;
            simple_fof_preload_app_encoded_formulas(right, bank)?;
        }
        SimpleFofFormula::Conjunction(formulas)
        | SimpleFofFormula::Disjunction(formulas)
        | SimpleFofFormula::Negation(formulas)
        | SimpleFofFormula::Universal { formulas, .. }
        | SimpleFofFormula::Existential { formulas, .. } => {
            simple_fof_preload_app_encoded_formulas(formulas, bank)?;
        }
    }
    Ok(())
}

fn simple_fof_preload_app_encoded_literal(
    literal: &Eqn,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    if let Some(term) = simple_fof_literal_formula_term(literal, bank) {
        simple_fof_preload_app_encoded_formula_term(term, bank)
    } else if literal.is_equ_lit(bank)
        && simple_fof_term_contains_fool(&[literal.left(), literal.right()])
    {
        simple_fof_preload_app_encoded_formula_or_term(literal.left(), bank)?;
        simple_fof_preload_app_encoded_formula_or_term(literal.right(), bank)
    } else {
        let mut sink = String::new();
        eqn_write_app_encode(&mut sink, bank, literal, false)
    }
}

fn simple_fof_preload_app_encoded_formula_term(
    term: &Term,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    if term.f_code() == SIG_ITE_CODE {
        for argument in term.argument_clones().into_iter().flatten() {
            simple_fof_preload_app_encoded_formula_term(&argument, bank)?;
        }
        return Ok(());
    }

    if term.f_code() == SIG_LET_CODE {
        return simple_fof_preload_app_encoded_let(term, bank);
    }

    if term.f_code() == bank.signature().not_code() {
        let argument = simple_fof_formula_term_argument(term, 0, "$not")?;
        return simple_fof_preload_app_encoded_formula_term(&argument, bank);
    }

    if simple_fof_app_encoded_formula_binary_operator(bank, term.f_code()).is_some() {
        for index in 0..2 {
            let argument = simple_fof_formula_term_argument(term, index, "binary formula")?;
            simple_fof_preload_app_encoded_formula_term(&argument, bank)?;
        }
        return Ok(());
    }

    if term.f_code() == bank.signature().qall_code() || term.f_code() == bank.signature().qex_code()
    {
        let body = simple_fof_formula_term_argument(term, 1, "quantified formula")?;
        return simple_fof_preload_app_encoded_formula_term(&body, bank);
    }

    if term.f_code() == bank.signature().eqn_code() || term.f_code() == bank.signature().neqn_code()
    {
        let literal = Eqn::tb_term_decode(bank, term)?;
        simple_fof_preload_app_encoded_literal(&literal, bank)
    } else {
        let literal = Eqn::alloc(term.clone(), bank.true_term().clone(), bank, true)?;
        let mut sink = String::new();
        eqn_write_app_encode(&mut sink, bank, &literal, false)
    }
}

fn simple_fof_preload_app_encoded_let(term: &Term, bank: &mut TermBank) -> Result<(), Diagnostic> {
    let body_index = term
        .arity()
        .checked_sub(1)
        .ok_or_else(|| Diagnostic::new(ErrorCode::TYPE_ERROR, "$let body is uninitialized"))?;
    for index in 0..body_index {
        let definition = simple_fof_formula_term_argument(term, index, "$let definition")?;
        simple_fof_preload_app_encoded_let_definition(&definition, bank)?;
    }
    let body = simple_fof_formula_term_argument(term, body_index, "$let body")?;
    simple_fof_preload_app_encoded_formula_or_term(&body, bank)
}

fn simple_fof_preload_app_encoded_let_definition(
    definition: &Term,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    if definition.f_code() != bank.signature().eqn_code() {
        return Err(Diagnostic::new(
            ErrorCode::TYPE_ERROR,
            "$let definition must be an equality",
        ));
    }
    let left = simple_fof_formula_term_argument(definition, 0, "$let definition")?;
    let right = simple_fof_formula_term_argument(definition, 1, "$let definition")?;
    simple_fof_preload_app_encoded_formula_or_term(&left, bank)?;
    simple_fof_preload_app_encoded_formula_or_term(&right, bank)
}

fn simple_fof_preload_app_encoded_formula_or_term(
    term: &Term,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    if term.type_().as_ref().is_some_and(Type::is_bool) {
        simple_fof_preload_app_encoded_formula_term(term, bank)
    } else if term.f_code() == SIG_ITE_CODE {
        simple_fof_preload_app_encoded_formula_term(
            &simple_fof_formula_term_argument(term, 0, "$ite")?,
            bank,
        )?;
        for index in 1..3 {
            simple_fof_preload_app_encoded_formula_or_term(
                &simple_fof_formula_term_argument(term, index, "$ite")?,
                bank,
            )?;
        }
        Ok(())
    } else if term.f_code() == SIG_LET_CODE {
        simple_fof_preload_app_encoded_let(term, bank)
    } else {
        let _encoded = term_app_encode(term, bank.signature_mut())?;
        Ok(())
    }
}

fn simple_fof_term_contains_fool(terms: &[&Term]) -> bool {
    let mut stack: Vec<Term> = terms.iter().map(|term| (*term).clone()).collect();
    while let Some(term) = stack.pop() {
        if matches!(term.f_code(), SIG_ITE_CODE | SIG_LET_CODE) {
            return true;
        }
        stack.extend(term.argument_clones().into_iter().flatten());
    }
    false
}

fn simple_fof_formulas_are_app_encoded_prop_true(formulas: &[SimpleFofFormula]) -> bool {
    matches!(formulas, [SimpleFofFormula::Truth(true)])
}

fn app_encoded_formula_role(properties: FormulaProperties) -> &'static str {
    match properties.query_tptp_type() {
        CP_TYPE_AXIOM if properties.query(CP_INPUT_FORMULA) => "axiom",
        CP_TYPE_HYPOTHESIS => "hypothesis",
        CP_TYPE_CONJECTURE => "conjecture",
        CP_TYPE_QUESTION => "question",
        CP_TYPE_LEMMA => "lemma",
        CP_TYPE_NEG_CONJECTURE => "negated_conjecture",
        _ => "plain",
    }
}

fn simple_fof_write_app_encoded_formulas(
    output: &mut String,
    bank: &mut TermBank,
    formulas: &[SimpleFofFormula],
) -> Result<(), Diagnostic> {
    if let Some((first, rest)) = formulas.split_first() {
        if rest.is_empty() {
            return simple_fof_write_app_encoded_formula(output, bank, first);
        }

        output.push('(');
        simple_fof_write_app_encoded_formula(output, bank, first)?;
        for formula in rest {
            output.push('&');
            simple_fof_write_app_encoded_formula(output, bank, formula)?;
        }
        output.push(')');
        Ok(())
    } else {
        simple_fof_write_app_encoded_truth(output, bank, true)
    }
}

fn simple_fof_write_app_encoded_formula(
    output: &mut String,
    bank: &mut TermBank,
    formula: &SimpleFofFormula,
) -> Result<(), Diagnostic> {
    match formula {
        SimpleFofFormula::Truth(value) => simple_fof_write_app_encoded_truth(output, bank, *value),
        SimpleFofFormula::Literal(literal) => {
            simple_fof_write_app_encoded_literal(output, bank, literal)
        }
        SimpleFofFormula::Implication {
            antecedents,
            consequents,
        } => simple_fof_write_app_encoded_binary(output, bank, antecedents, "=>", consequents),
        SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents,
        } => simple_fof_write_app_encoded_binary(output, bank, consequents, "<=", antecedents),
        SimpleFofFormula::Equivalence { left, right } => {
            simple_fof_write_app_encoded_binary(output, bank, left, "<=>", right)
        }
        SimpleFofFormula::Xor { left, right } => {
            simple_fof_write_app_encoded_binary(output, bank, left, "<~>", right)
        }
        SimpleFofFormula::Nand { left, right } => {
            simple_fof_write_app_encoded_binary(output, bank, left, "~&", right)
        }
        SimpleFofFormula::Nor { left, right } => {
            simple_fof_write_app_encoded_binary(output, bank, left, "~|", right)
        }
        SimpleFofFormula::Conjunction(formulas) => {
            simple_fof_write_app_encoded_chain(output, bank, formulas, "&")
        }
        SimpleFofFormula::Disjunction(formulas) => {
            simple_fof_write_app_encoded_chain(output, bank, formulas, "|")
        }
        SimpleFofFormula::Negation(formulas) => {
            output.push_str("~(");
            simple_fof_write_app_encoded_formulas(output, bank, formulas)?;
            output.push(')');
            Ok(())
        }
        SimpleFofFormula::Universal { bound, formulas } => {
            simple_fof_write_app_encoded_quantified(output, bank, "!", bound, formulas)
        }
        SimpleFofFormula::Existential { bound, formulas } => {
            simple_fof_write_app_encoded_quantified(output, bank, "?", bound, formulas)
        }
    }
}

fn simple_fof_write_app_encoded_literal(
    output: &mut String,
    bank: &mut TermBank,
    literal: &Eqn,
) -> Result<(), Diagnostic> {
    if let Some(term) = simple_fof_literal_formula_term(literal, bank) {
        if literal.is_positive() {
            simple_fof_write_app_encoded_formula_term(output, bank, term)
        } else {
            output.push_str("~(");
            simple_fof_write_app_encoded_formula_term(output, bank, term)?;
            output.push(')');
            Ok(())
        }
    } else if literal.is_equ_lit(bank)
        && simple_fof_term_contains_fool(&[literal.left(), literal.right()])
    {
        simple_fof_write_app_encoded_formula_or_term(output, bank, literal.left())?;
        if literal.is_negative() {
            output.push('!');
        }
        output.push('=');
        simple_fof_write_app_encoded_formula_or_term(output, bank, literal.right())
    } else {
        eqn_write_app_encode(output, bank, literal, false)
    }
}

fn simple_fof_literal_formula_term<'a>(literal: &'a Eqn, bank: &TermBank) -> Option<&'a Term> {
    (literal.right() == bank.true_term()
        && matches!(literal.left().f_code(), SIG_ITE_CODE | SIG_LET_CODE))
    .then_some(literal.left())
}

fn simple_fof_write_app_encoded_formula_term(
    output: &mut String,
    bank: &mut TermBank,
    term: &Term,
) -> Result<(), Diagnostic> {
    if term.f_code() == SIG_ITE_CODE {
        output.push_str("$ite(");
        for index in 0..3 {
            if index != 0 {
                output.push(',');
            }
            let argument = term.argument(index).ok_or_else(|| {
                Diagnostic::new(ErrorCode::TYPE_ERROR, "$ite argument is uninitialized")
            })?;
            simple_fof_write_app_encoded_formula_term(output, bank, &argument)?;
        }
        output.push(')');
        return Ok(());
    }

    if term.f_code() == SIG_LET_CODE {
        return simple_fof_write_app_encoded_let(output, bank, term);
    }

    if term.f_code() == bank.signature().not_code() {
        output.push_str("~(");
        let argument = simple_fof_formula_term_argument(term, 0, "$not")?;
        simple_fof_write_app_encoded_formula_term(output, bank, &argument)?;
        output.push(')');
        return Ok(());
    }

    if let Some(operator) = simple_fof_app_encoded_formula_binary_operator(bank, term.f_code()) {
        output.push('(');
        let left = simple_fof_formula_term_argument(term, 0, "binary formula")?;
        simple_fof_write_app_encoded_formula_term(output, bank, &left)?;
        output.push_str(operator);
        let right = simple_fof_formula_term_argument(term, 1, "binary formula")?;
        simple_fof_write_app_encoded_formula_term(output, bank, &right)?;
        output.push(')');
        return Ok(());
    }

    if term.f_code() == bank.signature().qall_code() || term.f_code() == bank.signature().qex_code()
    {
        let quantifier = if term.f_code() == bank.signature().qall_code() {
            "!"
        } else {
            "?"
        };
        let variable = simple_fof_formula_term_argument(term, 0, "quantified formula")?;
        let body = simple_fof_formula_term_argument(term, 1, "quantified formula")?;
        let type_ = variable.type_().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                "app-encoded quantified variable has no type",
            )
        })?;
        let type_name = type_app_encoded_name(&type_)?;
        write!(
            output,
            "{quantifier}[{}:{type_name}]:",
            bank.term_string(&variable, true)
        )
        .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
        simple_fof_write_app_encoded_formula_term(output, bank, &body)?;
        return Ok(());
    }

    if term.f_code() == bank.signature().eqn_code() || term.f_code() == bank.signature().neqn_code()
    {
        let literal = Eqn::tb_term_decode(bank, term)?;
        simple_fof_write_app_encoded_literal(output, bank, &literal)
    } else {
        let literal = Eqn::alloc(term.clone(), bank.true_term().clone(), bank, true)?;
        eqn_write_app_encode(output, bank, &literal, false)
    }
}

fn simple_fof_write_app_encoded_let(
    output: &mut String,
    bank: &mut TermBank,
    term: &Term,
) -> Result<(), Diagnostic> {
    let body_index = term
        .arity()
        .checked_sub(1)
        .ok_or_else(|| Diagnostic::new(ErrorCode::TYPE_ERROR, "$let body is uninitialized"))?;
    let mut definitions = Vec::with_capacity(body_index);
    for index in 0..body_index {
        let definition = simple_fof_formula_term_argument(term, index, "$let definition")?;
        definitions.push(simple_fof_app_encoded_let_definition_parts(
            bank,
            &definition,
        )?);
    }

    output.push_str("$let(");
    simple_fof_write_app_encoded_let_declarations(output, bank, &definitions)?;
    output.push(',');
    simple_fof_write_app_encoded_let_definitions(output, bank, &definitions)?;
    output.push(',');
    let body = simple_fof_formula_term_argument(term, body_index, "$let body")?;
    simple_fof_write_app_encoded_formula_or_term(output, bank, &body)?;
    output.push(')');
    Ok(())
}

fn simple_fof_app_encoded_let_definition_parts(
    bank: &TermBank,
    definition: &Term,
) -> Result<(Term, Term), Diagnostic> {
    if definition.f_code() != bank.signature().eqn_code() {
        return Err(Diagnostic::new(
            ErrorCode::TYPE_ERROR,
            "$let definition must be an equality",
        ));
    }
    let left = simple_fof_formula_term_argument(definition, 0, "$let definition")?;
    let right = simple_fof_formula_term_argument(definition, 1, "$let definition")?;
    Ok((left, right))
}

fn simple_fof_write_app_encoded_let_declarations(
    output: &mut String,
    bank: &TermBank,
    definitions: &[(Term, Term)],
) -> Result<(), Diagnostic> {
    if definitions.len() > 1 {
        output.push('[');
    }
    for (index, (left, _right)) in definitions.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        let name = bank
            .signature()
            .find_name(left.f_code())
            .ok_or_else(|| Diagnostic::new(ErrorCode::TYPE_ERROR, "$let symbol has no name"))?;
        let type_ = bank
            .signature()
            .get_type(left.f_code())
            .cloned()
            .or_else(|| left.type_())
            .ok_or_else(|| Diagnostic::new(ErrorCode::TYPE_ERROR, "$let symbol has no type"))?;
        let type_name = type_app_encoded_name(&type_)?;
        write!(output, "{name}:{type_name}")
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
    }
    if definitions.len() > 1 {
        output.push(']');
    }
    Ok(())
}

fn simple_fof_write_app_encoded_let_definitions(
    output: &mut String,
    bank: &mut TermBank,
    definitions: &[(Term, Term)],
) -> Result<(), Diagnostic> {
    if definitions.len() > 1 {
        output.push('[');
    }
    for (index, (left, right)) in definitions.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        simple_fof_write_app_encoded_formula_or_term(output, bank, left)?;
        output.push_str(":=");
        simple_fof_write_app_encoded_formula_or_term(output, bank, right)?;
    }
    if definitions.len() > 1 {
        output.push(']');
    }
    Ok(())
}

fn simple_fof_write_app_encoded_formula_or_term(
    output: &mut String,
    bank: &mut TermBank,
    term: &Term,
) -> Result<(), Diagnostic> {
    if term.type_().as_ref().is_some_and(Type::is_bool) {
        simple_fof_write_app_encoded_formula_term(output, bank, term)
    } else if term.f_code() == SIG_ITE_CODE {
        output.push_str("$ite(");
        simple_fof_write_app_encoded_formula_term(
            output,
            bank,
            &simple_fof_formula_term_argument(term, 0, "$ite")?,
        )?;
        for index in 1..3 {
            output.push(',');
            simple_fof_write_app_encoded_formula_or_term(
                output,
                bank,
                &simple_fof_formula_term_argument(term, index, "$ite")?,
            )?;
        }
        output.push(')');
        Ok(())
    } else if term.f_code() == SIG_LET_CODE {
        simple_fof_write_app_encoded_let(output, bank, term)
    } else {
        let encoded = term_app_encode(term, bank.signature_mut())?;
        bank.write_term(output, &encoded, true)
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))
    }
}

fn simple_fof_formula_term_argument(
    term: &Term,
    index: usize,
    context: &str,
) -> Result<Term, Diagnostic> {
    term.argument(index).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::TYPE_ERROR,
            format!("{context} argument is uninitialized"),
        )
    })
}

fn simple_fof_app_encoded_formula_binary_operator(
    bank: &TermBank,
    f_code: i64,
) -> Option<&'static str> {
    if f_code == bank.signature().and_code() {
        Some("&")
    } else if f_code == bank.signature().or_code() {
        Some("|")
    } else if f_code == bank.signature().impl_code() {
        Some("=>")
    } else if f_code == bank.signature().equiv_code() {
        Some("<=>")
    } else if f_code == bank.signature().nand_code() {
        Some("~&")
    } else if f_code == bank.signature().nor_code() {
        Some("~|")
    } else if f_code == bank.signature().bimpl_code() {
        Some("<=")
    } else if f_code == bank.signature().xor_code() {
        Some("<~>")
    } else {
        None
    }
}

fn simple_fof_write_app_encoded_truth(
    output: &mut String,
    bank: &mut TermBank,
    value: bool,
) -> Result<(), Diagnostic> {
    let literal = Eqn::alloc(
        bank.true_term().clone(),
        bank.true_term().clone(),
        bank,
        value,
    )?;
    eqn_write_app_encode(output, bank, &literal, false)
}

fn simple_fof_write_app_encoded_binary(
    output: &mut String,
    bank: &mut TermBank,
    left: &[SimpleFofFormula],
    operator: &str,
    right: &[SimpleFofFormula],
) -> Result<(), Diagnostic> {
    output.push('(');
    simple_fof_write_app_encoded_formulas(output, bank, left)?;
    output.push_str(operator);
    simple_fof_write_app_encoded_formulas(output, bank, right)?;
    output.push(')');
    Ok(())
}

fn simple_fof_write_app_encoded_chain(
    output: &mut String,
    bank: &mut TermBank,
    formulas: &[SimpleFofFormula],
    operator: &str,
) -> Result<(), Diagnostic> {
    output.push('(');
    if let Some((first, rest)) = formulas.split_first() {
        simple_fof_write_app_encoded_formula(output, bank, first)?;
        for formula in rest {
            output.push_str(operator);
            simple_fof_write_app_encoded_formula(output, bank, formula)?;
        }
    } else {
        simple_fof_write_app_encoded_truth(output, bank, true)?;
    }
    output.push(')');
    Ok(())
}

fn simple_fof_write_app_encoded_quantified(
    output: &mut String,
    bank: &mut TermBank,
    quantifier: &str,
    bound: &[Term],
    formulas: &[SimpleFofFormula],
) -> Result<(), Diagnostic> {
    output.push_str(quantifier);
    output.push('[');
    for (index, variable) in bound.iter().enumerate() {
        if index != 0 {
            output.push_str(", ");
        }
        let type_ = variable.type_().ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::TYPE_ERROR,
                "app-encoded quantified variable has no type",
            )
        })?;
        let type_name = type_app_encoded_name(&type_)?;
        write!(output, "{}:{type_name}", bank.term_string(variable, true))
            .map_err(|_| Diagnostic::new(ErrorCode::OTHER_ERROR, "failed to write formula"))?;
    }
    output.push_str("]:");
    simple_fof_write_app_encoded_formulas(output, bank, formulas)
}

fn run_prune_only<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &mut EProverConfig,
) -> Result<(), EProverError> {
    let mut state = proof_state_alloc(config.free_symbol_properties)?;
    let _parsed_ax_no = parse_input_files_into_axioms(config, &mut state)?;
    write_preprocessing_config_debug_line(output, config)?;
    load_configured_watchlist_source(config, &mut state)?;
    let _sine_pruned = apply_proof_state_sine(output, config.sine.as_deref(), &mut state)?;
    let _relevancy_pruned = apply_clause_relevance_pruning(config, &mut state);
    let _preproc_removed = apply_clause_set_preprocessing(
        &mut state,
        config.preprocessing.no_preprocessing,
        config.search.inference.higher_order.replace_inj_defs,
        config.preprocessing.eqdef_incrlimit,
        config.preprocessing.eqdef_maxclauses,
    )?;
    let bce_max_occs = i32_from_i64_config("bce_max_occs", config.preprocessing.bce.max_occs)?;
    apply_blocked_clause_elimination(
        output,
        config.preprocessing.bce.enabled,
        bce_max_occs,
        &mut state,
    )?;
    apply_goal_definition_transformation(
        &mut state,
        config.preprocessing.goal_definitions.positive,
        config.preprocessing.goal_definitions.negative,
        config.preprocessing.goal_definitions.subterms,
    )?;
    let _next_doc_ident = write_initial_clause_docs(output, config, &mut state)?;
    write_comment_line_after_blank(output, "Pruning successful!")?;
    write_tstp_status(output, "Unknown")?;
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "executable proof-search pipeline keeps C ordering visible"
)]
fn run_proof_search<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &mut EProverConfig,
) -> Result<u8, EProverError> {
    let mut state = proof_state_alloc(config.free_symbol_properties)?;
    let parsed_ax_no = parse_input_files_into_axioms(config, &mut state)?;
    let mut heuristic_params = heuristic_parms_from_config(config)?;
    let auto_context =
        apply_auto_mode_preprocessing_selection(output, config, &state, &mut heuristic_params)?;
    write_preprocessing_params_debug_line(output, &heuristic_params)?;
    load_configured_watchlist_source(config, &mut state)?;
    let sine_pruned = apply_proof_state_sine(output, heuristic_params.sine.as_deref(), &mut state)?;
    let relevancy_pruned = sine_pruned + apply_clause_relevance_pruning(config, &mut state);
    let raw_clause_no = state.axioms().members();
    let preproc_removed = apply_clause_set_preprocessing(
        &mut state,
        heuristic_params.no_preproc,
        heuristic_params.replace_inj_defs,
        heuristic_params.eqdef_incrlimit,
        heuristic_params.eqdef_maxclauses,
    )?;
    apply_blocked_clause_elimination(
        output,
        heuristic_params.bce,
        heuristic_params.bce_max_occs,
        &mut state,
    )?;
    apply_goal_definition_transformation(
        &mut state,
        heuristic_params.add_goal_defs_pos,
        heuristic_params.add_goal_defs_neg,
        heuristic_params.add_goal_defs_subterms,
    )?;
    if relevancy_pruned != 0 || config.search.completeness.incomplete {
        state.set_state_is_complete(false);
    }
    let next_doc_ident = write_initial_clause_docs(output, config, &mut state)?;
    let next_doc_ident =
        write_watchlist_initial_clause_docs(output, config, &mut state, next_doc_ident)?;

    apply_auto_mode_search_selection(
        output,
        config,
        &state,
        auto_context.as_ref(),
        &mut heuristic_params,
    )?;
    apply_strategy_io_to_params(config, &mut heuristic_params)?;
    let mut control = proof_control_from_heuristic_parms(config, heuristic_params)?;
    let mut params = control.heuristic_parms().clone();
    let fvi_params = control.fvi_parms().clone();
    let wfcb_defs = &config.search.heuristic.weight_function_definitions;
    let mut hcb_defs = config.search.heuristic.heuristic_definitions.clone();
    {
        let (bank, axioms) = state.terms_and_axioms_mut();
        proof_control_init(
            &mut control,
            bank,
            axioms,
            &mut params,
            &fvi_params,
            wfcb_defs,
            &mut hcb_defs,
            false,
        )?;
    }
    proof_state_init_with_ac_output(output, config.output_level, &mut state, &mut control)?;
    write_preprocessing_time(output, config)?;
    if config.flags.contains(EProverFlag::CnfOnly) {
        write_cnf_only_success(output)?;
        write_saturated_output(output, config, &state, None)?;
        write_proof_statistics(
            output,
            config,
            &state,
            parsed_ax_no,
            relevancy_pruned,
            raw_clause_no,
            preproc_removed,
        )?;
        return Ok(ErrorCode::NO_ERROR.exit_status());
    }
    let index_signature = state.terms().signature().clone();
    let mut global_indices = proof_search_global_indices(&index_signature, &control);
    let presat_outcome = if control.heuristic_parms().presat_interreduction {
        run_presaturation_interreduction(output, &mut state, &mut control, &mut global_indices)?
    } else {
        None
    };
    let mut outcome = if let Some(outcome) = presat_outcome {
        outcome
    } else {
        run_main_saturation(config, &mut state, &mut control, &mut global_indices)?
    };
    if let Some(filtered_empty) = filter_saturated_unprocessed(config, &mut state, &mut control)? {
        outcome = SaturateOutcome::Returned {
            clause: Box::new(filtered_empty),
            reason: SaturateReturnReason::Filter,
            processed_steps: outcome.processed_steps(),
        };
    }
    let inference_system_complete = proof_search_inference_system_complete(&state, &control);
    write_proof_search_side_outputs(output, config, &mut state, &outcome, next_doc_ident)?;
    write_proof_search_result_outputs(
        output,
        config,
        &state,
        &outcome,
        inference_system_complete,
        next_doc_ident,
    )?;
    write_supported_proof_object_statistics(
        output,
        config,
        &state,
        &outcome,
        inference_system_complete,
    )?;
    process_success_proof_object_gc(output, config, &mut state, &outcome)?;
    let saturated_success = match &outcome {
        SaturateOutcome::Returned { clause, .. } => Some(clause.as_ref()),
        SaturateOutcome::Stopped { .. } => None,
    };
    if !should_suppress_saturated_output_after_force_deriv(config, &outcome) {
        write_saturated_output(output, config, &state, saturated_success)?;
    }
    write_proof_statistics(
        output,
        config,
        &state,
        parsed_ax_no,
        relevancy_pruned,
        raw_clause_no,
        preproc_removed,
    )?;
    Ok(saturate_outcome_exit_status(
        &outcome,
        &state,
        inference_system_complete,
        config.search.completeness.assume_inference_system_complete,
    ))
}

fn run_main_saturation(
    config: &EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
    control: &mut ProofControl,
    indices: &mut GlobalIndices<'_>,
) -> Result<SaturateOutcome, EProverError> {
    Ok(proof_state_saturate_with_global_indices(
        state,
        control,
        config.step_limit,
        config.processed_set_limit,
        config.unprocessed_limit,
        config.total_clause_set_limit,
        config.generated_limit,
        config.term_bank_insert_limit,
        config.answer_limit,
        indices,
    )?)
}

fn should_suppress_saturated_output_after_force_deriv(
    config: &EProverConfig,
    outcome: &SaturateOutcome,
) -> bool {
    config.force_derivation_output >= 2 && matches!(outcome, SaturateOutcome::Stopped { .. })
}

struct AutoModeContext {
    limits: SpecLimits,
    raw_features: RawSpecFeatureCell,
    preprocessing_schedule: Vec<ScheduleCell>,
    selected_preprocessing_index: usize,
}

fn apply_auto_mode_preprocessing_selection<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    params: &mut HeuristicParmsCell,
) -> Result<Option<AutoModeContext>, EProverError> {
    if !(config.flags.contains(EProverFlag::Auto) || config.strategy_scheduling) {
        return Ok(None);
    }

    let limits = create_default_spec_limits();
    let mut raw_features = RawSpecFeatureCell::default();
    raw_spec_features_compute(&mut raw_features, state);
    raw_spec_features_classify(&mut raw_features, &limits, Some(RAW_DEFAULT_MASK));
    let mut preprocessing_schedule = get_preprocessing_schedule(&raw_features.class)?.schedule;
    output.write_stdout_side_channel(
        format!(
            "{DEFAULT_COMCHAR_RAW} Preprocessing class: {}.\n",
            raw_features.class
        )
        .as_bytes(),
    )?;
    let selected_preprocessing_index = if config.strategy_scheduling {
        select_scheduled_preprocessing_cell(output, config, &mut preprocessing_schedule)?
    } else {
        0
    };
    let preproc_name =
        schedule_heuristic_name(&preprocessing_schedule, selected_preprocessing_index)?;
    get_heuristic_with_name(preproc_name, params)?;
    overlay_explicit_heuristic_options(config, params)?;

    if !config.strategy_scheduling {
        output.write_stdout_side_channel(
            format!("{DEFAULT_COMCHAR_RAW} Configuration: {preproc_name}\n").as_bytes(),
        )?;
    }

    Ok(Some(AutoModeContext {
        limits,
        raw_features,
        preprocessing_schedule,
        selected_preprocessing_index,
    }))
}

fn apply_auto_mode_search_selection<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    auto_context: Option<&AutoModeContext>,
    params: &mut HeuristicParmsCell,
) -> Result<(), EProverError> {
    let Some(auto_context) = auto_context else {
        return Ok(());
    };
    if config.flags.contains(EProverFlag::CnfOnly) {
        return Ok(());
    }

    let choice_max_depth = params.inst_choice_max_depth;
    let mut features = SpecFeatureCell::default();
    spec_features_compute_clause_set_without_choice(&mut features, state.axioms(), state.terms());
    features.order = auto_context.raw_features.order;
    features.goal_order = auto_context.raw_features.conj_order;
    features.num_of_definitions = auto_context.raw_features.num_of_definitions;
    features.perc_of_form_defs = auto_context.raw_features.perc_of_form_defs;
    spec_features_add_eval(&mut features, &auto_context.limits);
    let class = spec_type_string(&features, DEFAULT_MASK);
    let mut search_schedule = get_search_schedule(&class)?.schedule;
    output.write_stdout_side_channel(
        format!("{DEFAULT_COMCHAR_RAW} Search class: {class}\n").as_bytes(),
    )?;
    let search_name = if config.strategy_scheduling {
        select_scheduled_search_cell(output, config, auto_context, &mut search_schedule)?
    } else {
        first_schedule_heuristic_name(&search_schedule)?.to_owned()
    };

    get_heuristic_with_name(&search_name, params)?;
    params.inst_choice_max_depth = choice_max_depth;
    overlay_explicit_heuristic_options(config, params)?;

    if !config.strategy_scheduling {
        output.write_stdout_side_channel(
            format!("{DEFAULT_COMCHAR_RAW} Configuration: {search_name}\n").as_bytes(),
        )?;
    }
    Ok(())
}

fn first_schedule_heuristic_name(
    schedule: &[crate::heuristics::new_autoschedule::ScheduleCell],
) -> Result<&str, Diagnostic> {
    schedule
        .first()
        .map(|cell| cell.heuristic_name.as_str())
        .ok_or_else(|| Diagnostic::new(ErrorCode::OTHER_ERROR, "auto schedule is empty"))
}

fn schedule_heuristic_name(schedule: &[ScheduleCell], index: usize) -> Result<&str, Diagnostic> {
    schedule
        .get(index)
        .map(|cell| cell.heuristic_name.as_str())
        .ok_or_else(|| Diagnostic::new(ErrorCode::OTHER_ERROR, "auto schedule is empty"))
}

fn select_scheduled_preprocessing_cell<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    schedule: &mut Vec<ScheduleCell>,
) -> Result<usize, EProverError> {
    let mut cores = configured_schedule_cores(config);
    let serialize = config.serialize_schedule || cores == 1;
    let report = schedule_times_init_multi_core(
        schedule,
        schedule_time_used_seconds(),
        configured_schedule_time_limit(config),
        true,
        &mut cores,
        serialize,
    );
    write_schedule_report(
        output,
        report.scheduled,
        report.cores,
        report.limit,
        report.total_time,
    )?;
    if schedule.is_empty() {
        return Err(Diagnostic::new(ErrorCode::OTHER_ERROR, "auto schedule is empty").into());
    }
    Ok(0)
}

fn select_scheduled_search_cell<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    auto_context: &AutoModeContext,
    search_schedule: &mut Vec<ScheduleCell>,
) -> Result<String, EProverError> {
    let preprocessing_tail =
        &auto_context.preprocessing_schedule[auto_context.selected_preprocessing_index..];
    initialize_placeholder_search_schedule(
        search_schedule,
        preprocessing_tail,
        config.force_preprocessing_schedule,
    )?;

    let selected_preprocessing =
        &auto_context.preprocessing_schedule[auto_context.selected_preprocessing_index];
    let mut cores = selected_preprocessing.cores.max(1);
    let report = schedule_times_init_multi_core(
        search_schedule,
        schedule_time_used_seconds(),
        f64_from_u64_for_schedule(selected_preprocessing.time_absolute),
        false,
        &mut cores,
        false,
    );
    write_schedule_report(
        output,
        report.scheduled,
        report.cores,
        report.limit,
        report.total_time,
    )?;
    first_schedule_heuristic_name(search_schedule)
        .map(str::to_owned)
        .map_err(EProverError::from)
}

fn write_schedule_report(
    output: &mut impl Write,
    scheduled: usize,
    cores: i32,
    limit: u64,
    total_time: u64,
) -> Result<(), EProverError> {
    write_comment_line(
        output,
        &format!("Scheduled {scheduled} strats onto {cores} cores with {limit} seconds ({total_time} total)"),
    )
}

fn configured_schedule_cores(config: &EProverConfig) -> i32 {
    if config.schedule_cores == -1 {
        return i32_from_usize_saturating(get_core_number()).max(1);
    }
    i32_from_i64_saturating(config.schedule_cores).max(1)
}

fn configured_schedule_time_limit(config: &EProverConfig) -> f64 {
    let limit = config
        .schedule_time_limit
        .and_then(|limit| u64::try_from(limit).ok())
        .unwrap_or(DEFAULT_SCHED_TIME_LIMIT);
    f64_from_u64_for_schedule(limit)
}

fn schedule_time_used_seconds() -> f64 {
    let usage = current_resource_usage();
    usage.user_time_seconds + usage.system_time_seconds
}

fn i32_from_usize_saturating(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn i32_from_i64_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or_else(|_| {
        if value.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

#[allow(clippy::cast_precision_loss)]
fn f64_from_u64_for_schedule(value: u64) -> f64 {
    value as f64
}

fn proof_search_global_indices<'sig>(
    signature: &'sig Signature,
    control: &ProofControl,
) -> GlobalIndices<'sig> {
    let params = control.heuristic_parms();
    GlobalIndices::new_for_problem(
        signature,
        params.rw_bw_index_type.as_str(),
        params.pm_from_index_type.as_str(),
        params.pm_into_index_type.as_str(),
        params.ext_rules_max_depth,
        ProblemType::FirstOrder,
    )
}

fn write_proof_search_result_outputs<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    outcome: &SaturateOutcome,
    inference_system_complete: bool,
    next_doc_ident: i64,
) -> Result<(), EProverError> {
    write_stopped_result_doc_quotes(
        output,
        config,
        state,
        outcome,
        inference_system_complete,
        next_doc_ident,
    )?;
    write_proof_search_result(
        output,
        config,
        outcome,
        state,
        inference_system_complete,
        config.search.completeness.assume_inference_system_complete,
    )?;
    match outcome {
        SaturateOutcome::Returned { clause, .. } => {
            write_proof_success_object_output(output, config, state, clause, next_doc_ident)?;
        }
        SaturateOutcome::Stopped { .. } => {
            if let Some(status) =
                stopped_proof_object_status(config, state, outcome, inference_system_complete)
            {
                write_stopped_proof_output(output, config, state, status)?;
            }
        }
    }
    Ok(())
}

fn write_stopped_result_doc_quotes<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    outcome: &SaturateOutcome,
    inference_system_complete: bool,
    next_doc_ident: i64,
) -> Result<(), EProverError> {
    if config.output_level < 2 {
        return Ok(());
    }

    let Some((prop, comment)) =
        stopped_result_doc_quote_spec(config, state, outcome, inference_system_complete)
    else {
        return Ok(());
    };

    write_proof_state_prop_doc_quote(output, config, state, prop, comment, next_doc_ident)?;
    Ok(())
}

fn stopped_result_doc_quote_spec(
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    outcome: &SaturateOutcome,
    inference_system_complete: bool,
) -> Option<(FormulaProperties, &'static str)> {
    match outcome {
        SaturateOutcome::Returned { .. } => None,
        SaturateOutcome::Stopped {
            reason: SaturateStopReason::WatchlistEmpty,
            ..
        } => Some((CP_SUBSUMES_WATCH, "final_subsumes_wl")),
        SaturateOutcome::Stopped {
            reason: SaturateStopReason::Saturated,
            ..
        } if state.unprocessed().is_empty()
            && state.state_is_complete()
            && (inference_system_complete
                || config.search.completeness.assume_inference_system_complete) =>
        {
            Some((CP_IGNORE_PROPS, "final"))
        }
        SaturateOutcome::Stopped { .. } => Some((CP_IGNORE_PROPS, "exists")),
    }
}

fn write_proof_state_prop_doc_quote<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    prop: FormulaProperties,
    comment: &str,
    mut doc_ident: i64,
) -> Result<(), EProverError> {
    for set in [
        state.processed_pos_rules(),
        state.processed_pos_eqns(),
        state.processed_neg_units(),
        state.processed_non_units(),
        state.unprocessed(),
    ] {
        doc_ident = write_clause_set_prop_doc_quote(
            output,
            config,
            state.terms(),
            set,
            prop,
            comment,
            doc_ident,
        )?;
    }
    Ok(())
}

fn write_clause_set_prop_doc_quote<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    bank: &TermBank,
    set: &ClauseSet,
    prop: FormulaProperties,
    comment: &str,
    mut doc_ident: i64,
) -> Result<i64, EProverError> {
    for clause in set.iter() {
        if clause.query_prop(prop) {
            write_clause_quote_doc(
                output,
                config,
                bank,
                clause,
                doc_ident,
                clause.ident(),
                comment,
            )?;
            doc_ident = doc_ident.saturating_add(1);
        }
    }
    Ok(doc_ident)
}

fn process_success_proof_object_gc<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
    outcome: &SaturateOutcome,
) -> Result<(), EProverError> {
    if config.proof_object_level <= 0 {
        return Ok(());
    }
    let SaturateOutcome::Returned { clause, .. } = outcome else {
        return Ok(());
    };

    let _ = state.mark_proof_clause_ancestors(clause);
    let _ = state.analyse_gc();
    if let Some(training_flags) = config.training_examples {
        write_training_examples(output, config, state, training_flags)?;
    }
    Ok(())
}

fn write_proof_search_side_outputs<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
    outcome: &SaturateOutcome,
    next_doc_ident: i64,
) -> Result<(), EProverError> {
    write_answer_outputs(output, state)?;
    write_sat_check_success_output(output, outcome)?;
    if let SaturateOutcome::Returned { clause, .. } = outcome {
        write_proof_success_doc(output, config, state.terms(), clause, next_doc_ident)?;
    }
    Ok(())
}

fn write_sat_check_success_output(
    output: &mut impl Write,
    outcome: &SaturateOutcome,
) -> Result<(), EProverError> {
    if matches!(
        outcome,
        SaturateOutcome::Returned {
            reason: SaturateReturnReason::SatCheck,
            ..
        }
    ) {
        write_comment_line(output, "SatCheck found unsatisfiable ground set")?;
    }
    Ok(())
}

fn run_presaturation_interreduction(
    output: &mut impl Write,
    state: &mut crate::clauses::proofstate::ProofState,
    control: &mut ProofControl,
    indices: &mut GlobalIndices<'_>,
) -> Result<Option<SaturateOutcome>, EProverError> {
    let selection_strategy = control.heuristic_parms().selection_strategy.clone();
    NO_GENERATION.clone_into(&mut control.heuristic_parms_mut().selection_strategy);
    let outcome = proof_state_saturate_with_global_indices(
        state,
        control,
        i64::MAX,
        i64::MAX,
        i64::MAX,
        i64::MAX,
        i64::MAX,
        i64::MAX,
        i64::MAX,
        indices,
    );
    control.heuristic_parms_mut().selection_strategy = selection_strategy;
    let outcome = outcome?;
    write_comment_line(output, "Presaturation interreduction done")?;
    if matches!(outcome, SaturateOutcome::Returned { .. }) {
        Ok(Some(outcome))
    } else {
        proof_state_reset_processed_with_global_indices(state, control, indices)?;
        Ok(None)
    }
}

fn filter_saturated_unprocessed(
    config: &EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
    control: &mut ProofControl,
) -> Result<Option<crate::clauses::clause::Clause>, EProverError> {
    if !config.flags.contains(EProverFlag::FilterSaturated) {
        return Ok(None);
    }
    proof_state_filter_unprocessed(state, control, &config.filter_saturated_descriptor)
        .map_err(Into::into)
}

fn parse_input_files_into_axioms(
    config: &mut EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
) -> Result<i64, EProverError> {
    let mut parsed_total = 0_i64;
    config.flags.clear(EProverFlag::FormulaConjectureSeen);
    let files = config.files.clone();
    for file in &files {
        let before = state.axioms().len();
        let mut parsed = ClauseSet::new();
        let mut parsed_watchlist = ClauseSet::new();
        let parsed_file = parse_clause_file(
            file,
            config.parse_format,
            FormulaPreprocessing::from_config(config),
            state.terms_mut(),
            &mut parsed,
            &mut parsed_watchlist,
        )?;
        if parsed_file.formula_conjecture_seen {
            config.flags.set(EProverFlag::FormulaConjectureSeen);
        }
        apply_auto_parse_output_side_effects(config, parsed_file.detected_format);
        let parsed_count = parsed.len();
        parsed_total = parsed_total.saturating_add(i64::try_from(parsed_count).unwrap_or(i64::MAX));
        state.add_raw_formula_features(parsed_file.raw_formula_features);
        state.axioms_mut().insert_set(&mut parsed);
        if !parsed_watchlist.is_empty() {
            let watchlist = state.watchlist_mut().ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    "Cannot store inline watchlist clauses after the watchlist has been disabled",
                )
            })?;
            watchlist.insert_set(&mut parsed_watchlist);
        }
        if config.flags.contains(EProverFlag::RequireNonempty) && parsed_count == 0 {
            return Err(Diagnostic::new(
                ErrorCode::INPUT_SEMANTIC_ERROR,
                format!("Input file {file} did not contain any clauses"),
            )
            .into());
        }
        debug_assert_eq!(state.axioms().len(), before + parsed_count);
    }

    if config.flags.contains(EProverFlag::RequireNonempty) && state.axioms().is_empty() {
        return Err(Diagnostic::new(
            ErrorCode::INPUT_SEMANTIC_ERROR,
            "Input did not contain any clauses",
        )
        .into());
    }

    Ok(parsed_total)
}

fn write_preprocessing_config_debug_line<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
) -> Result<(), EProverError> {
    output.write_stdout_side_channel(preprocessing_config_debug_line(config).as_bytes())?;
    Ok(())
}

fn write_preprocessing_params_debug_line<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    params: &HeuristicParmsCell,
) -> Result<(), EProverError> {
    output.write_stdout_side_channel(preprocessing_params_debug_line(params).as_bytes())?;
    Ok(())
}

fn preprocessing_config_debug_line(config: &EProverConfig) -> String {
    let ho_preprocessing = &config.search.inference.higher_order_preprocessing;
    format!(
        "{DEFAULT_COMCHAR_RAW} (lift_lambdas = {}, lambda_to_forall = {},unroll_only_formulas = {}, sine = {})\n",
        i32::from(config.search.support.lift_lambdas),
        i32::from(ho_preprocessing.lambda_to_forall),
        i32::from(ho_preprocessing.unroll_only_formulas),
        config.sine.as_deref().unwrap_or("(null)")
    )
}

fn preprocessing_params_debug_line(params: &HeuristicParmsCell) -> String {
    format!(
        "{DEFAULT_COMCHAR_RAW} (lift_lambdas = {}, lambda_to_forall = {},unroll_only_formulas = {}, sine = {})\n",
        i32::from(params.lift_lambdas),
        i32::from(params.lambda_to_forall),
        i32::from(params.unroll_only_formulas),
        params.sine.as_deref().unwrap_or("(null)")
    )
}

fn apply_proof_state_sine<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    configured_filter: Option<&str>,
    state: &mut crate::clauses::proofstate::ProofState,
) -> Result<i64, EProverError> {
    let Some(mut filter_name) = configured_filter else {
        return Ok(0);
    };
    if filter_name == "NoSInE" {
        return Ok(0);
    }
    if filter_name == "Auto" {
        let Some(auto_filter) = auto_sine_filter_name(state) else {
            write_comment_line(output, "No SInE strategy applied")?;
            return Ok(0);
        };
        filter_name = auto_filter;
    }

    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} SinE strategy is {filter_name}"
    )?;
    let resolution = sine_get_filter(filter_name)?;
    match resolution.filter().type_ {
        AxFilterType::Threshold => Ok(apply_threshold_sine_filter(
            state,
            resolution.filter().threshold,
        )),
        AxFilterType::GSinE => Ok(apply_gsine_clause_filter(state, resolution.filter())),
        AxFilterType::LambdaDefines => Ok(apply_lambda_defines_filter(state)),
        AxFilterType::NoFilter => Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "resolved SInE filter has no concrete type",
        )
        .into()),
    }
}

fn apply_threshold_sine_filter(
    state: &mut crate::clauses::proofstate::ProofState,
    threshold: i64,
) -> i64 {
    let original_axioms = state.axioms().members();
    let selected_axioms = {
        let mut clause_sets = PStack::new();
        let mut selected_clauses = PStack::new();
        clause_sets.push(state.axioms());
        select_threshold_clause_sets(&clause_sets, 0, threshold, &mut selected_clauses)
    };

    if selected_axioms == original_axioms {
        return 0;
    }

    state.axioms_mut().clear();
    original_axioms - selected_axioms
}

fn apply_gsine_clause_filter(
    state: &mut crate::clauses::proofstate::ProofState,
    filter: &AxFilter,
) -> i64 {
    let original_axioms = state.axioms().members();
    let selected_idents = {
        let mut clause_sets = PStack::new();
        clause_sets.push(state.axioms());
        let mut generality = GenDistrib::new(state.terms().signature());
        generality.add_clause_sets(&clause_sets);
        let mut selected_clauses = PStack::new();
        let params = ClauseSineParams {
            gen_measure: filter.gen_measure,
            use_hypotheses: filter.use_hypotheses,
            benevolence: filter.benevolence,
            generosity: filter.generosity,
            max_recursion_depth: filter.max_recursion_depth,
            max_set_size: filter.max_set_size,
            max_set_fraction: filter.max_set_fraction,
            add_no_symbol_axioms: filter.add_no_symbol_axioms,
        };
        select_axioms_clause_sets(
            &mut generality,
            &clause_sets,
            0,
            params,
            &mut selected_clauses,
        );
        selected_clauses
            .as_slice()
            .iter()
            .map(|clause| clause.ident())
            .collect::<Vec<_>>()
    };

    replace_axioms_with_selected_idents(state, original_axioms, selected_idents)
}

fn apply_lambda_defines_filter(state: &mut crate::clauses::proofstate::ProofState) -> i64 {
    let original_axioms = state.axioms().members();
    if state.raw_formula_features().sentence_no == 0 {
        state.axioms_mut().clear();
        return original_axioms;
    }

    let selected_idents = state
        .axioms()
        .iter()
        .filter(|clause| {
            clause.query_prop(CP_IS_LAMBDA_DEF) || clause.is_conjecture() || clause.is_hypothesis()
        })
        .map(Clause::ident)
        .collect::<Vec<_>>();
    replace_axioms_with_selected_idents(state, original_axioms, selected_idents)
}

fn replace_axioms_with_selected_idents(
    state: &mut crate::clauses::proofstate::ProofState,
    original_axioms: i64,
    selected_idents: Vec<i64>,
) -> i64 {
    let mut selected_axioms = ClauseSet::new();
    let mut moved = BTreeSet::new();
    for ident in selected_idents {
        if !moved.insert(ident) {
            continue;
        }
        if let Some(clause) = state.axioms_mut().extract_by_id(ident) {
            selected_axioms.insert(clause);
        }
    }
    let selected_count = selected_axioms.members();
    *state.axioms_mut() = selected_axioms;
    original_axioms - selected_count
}

fn auto_sine_filter_name(state: &crate::clauses::proofstate::ProofState) -> Option<&'static str> {
    let mut limits = SpecLimits::alloc();
    limits.ax_some_limit = 1_199;
    limits.ax_many_limit = 10_396;
    limits.term_medium_limit = 664_277;
    limits.term_large_limit = 5_573_560;
    limits.symbols_medium_limit = 2_471;
    limits.symbols_large_limit = 4_140;
    limits.predc_medium_limit = 0;
    limits.predc_large_limit = 2;
    limits.pred_medium_limit = 1_225;
    limits.pred_large_limit = 4_000;
    limits.func_medium_limit = 8;
    limits.func_large_limit = 110;
    limits.fun_medium_limit = 360;
    limits.fun_large_limit = 400;

    let mut features = RawSpecFeatureCell::default();
    raw_spec_features_compute(&mut features, state);
    raw_spec_features_classify(&mut features, &limits, Some(SINE_AUTO_MASK));

    if features.conjecture_count + features.hypothesis_count == 0 {
        return None;
    }
    SINE_AUTO_CLASSES
        .iter()
        .find(|(class, _)| {
            class.as_bytes().get(..SINE_AUTO_CLASS_LEN)
                == features.class.as_bytes().get(..SINE_AUTO_CLASS_LEN)
        })
        .and_then(|(_, filter)| *filter)
}

fn apply_clause_relevance_pruning(
    config: &EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
) -> i64 {
    let level = config.preprocessing.relevance_prune_level;
    if level == 0 {
        return 0;
    }

    let (pruned, removed) =
        clause_set_relevance_prune(state.terms().signature(), state.axioms(), level);
    *state.axioms_mut() = pruned;
    removed
}

fn apply_clause_set_preprocessing(
    state: &mut crate::clauses::proofstate::ProofState,
    no_preprocessing: bool,
    replace_injectivity_defs: bool,
    eqdef_incrlimit: i64,
    eqdef_maxclauses: i64,
) -> Result<i64, EProverError> {
    let mut tmp_bank = TermBank::new(state.terms().signature().clone())?;
    let (bank, axioms, watchlist, archive) = state.terms_axioms_watchlist_archive_mut();
    let mut removed = 0;
    if !no_preprocessing {
        removed += clause_set_preprocess(
            axioms,
            archive,
            &mut tmp_bank,
            bank,
            replace_injectivity_defs,
            eqdef_incrlimit,
            eqdef_maxclauses,
        )?;
    }
    removed += clause_set_unfold_eq_def_normalize(
        axioms,
        watchlist,
        archive,
        &mut tmp_bank,
        bank,
        eqdef_incrlimit,
        eqdef_maxclauses,
    )?;
    Ok(removed)
}

fn apply_blocked_clause_elimination<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    enabled: bool,
    max_occs: i32,
    state: &mut crate::clauses::proofstate::ProofState,
) -> Result<i64, EProverError> {
    if !enabled || problem_type() != ProblemType::FirstOrder {
        return Ok(0);
    }

    let mut tmp_bank = TermBank::new(state.terms().signature().clone())?;
    let mut bce_output = String::new();
    let result = {
        let (bank, axioms, archive) = state.terms_axioms_archive_mut();
        eliminate_blocked_clauses_with_output(
            axioms,
            archive,
            max_occs,
            bank,
            &mut tmp_bank,
            &mut bce_output,
        )?
    };
    output.write_stdout_side_channel(bce_output.as_bytes())?;
    Ok(result.eliminated_count)
}

fn apply_goal_definition_transformation(
    state: &mut crate::clauses::proofstate::ProofState,
    positive: bool,
    negative: bool,
    subterms: bool,
) -> Result<i64, EProverError> {
    if !positive && !negative {
        return Ok(0);
    }

    let (bank, axioms) = state.terms_and_axioms_mut();
    Ok(clause_set_gd_transform(
        bank, axioms, positive, negative, subterms,
    )?)
}

fn load_configured_watchlist_source(
    config: &EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
) -> Result<(), EProverError> {
    let Some(source) = &config.search.watchlist.source else {
        state.load_watchlist(ProofStateWatchlistSource::Disabled, config.parse_format)?;
        return Ok(());
    };
    let proof_state_source = match source {
        WatchlistSource::Inline => ProofStateWatchlistSource::Inline,
        WatchlistSource::File(path) => ProofStateWatchlistSource::File(Path::new(path)),
    };
    state.load_watchlist(proof_state_source, config.parse_format)?;
    Ok(())
}

fn write_initial_clause_docs<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
) -> Result<i64, EProverError> {
    if config.output_level < 2 {
        return Ok(1);
    }

    let (bank, axioms) = state.terms_and_axioms_mut();
    write_clause_set_initial_docs(output, config, bank, axioms, 1)
}

fn write_watchlist_initial_clause_docs<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    state: &mut crate::clauses::proofstate::ProofState,
    next_doc_ident: i64,
) -> Result<i64, EProverError> {
    if config.output_level < 2 {
        return Ok(next_doc_ident);
    }

    let (bank, watchlist) = state.terms_and_watchlist_mut();
    let Some(watchlist) = watchlist else {
        return Ok(next_doc_ident);
    };
    write_clause_set_initial_docs(output, config, bank, watchlist, next_doc_ident)
}

fn write_clause_set_initial_docs<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    bank: &TermBank,
    set: &mut ClauseSet,
    start_ident: i64,
) -> Result<i64, EProverError> {
    let mut session = ProofDocSession::new(
        proof_doc_output_format(config),
        config.output_level,
        crate::basics::simple_stuff::ProblemType::FirstOrder,
    );
    session.pcl_shell_level = i32::try_from(config.pcl_output.shell_level).map_err(|_| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "configured PCL shell level is outside C int range",
        )
    })?;
    session.step_options = pcl_step_print_options(config);
    session.id_source = ProofDocIdSource::from_current(start_ident.saturating_sub(1));

    for clause in set.iter_mut() {
        let mut rendered = String::new();
        let result = session.doc_clause_creation(
            &mut rendered,
            bank,
            clause,
            ClauseCreationInference::Initial,
            ClauseCreationParents::none(),
            None,
        )?;
        output.write_stdout_side_channel(result.stdout_before.as_bytes())?;
        if let Some(offset) = result.stdout_after_offset {
            let (before_stdout, after_stdout) = rendered.split_at(offset);
            output.write_all(before_stdout.as_bytes())?;
            output.write_stdout_side_channel(result.stdout_after.as_bytes())?;
            output.write_all(after_stdout.as_bytes())?;
        } else {
            output.write_all(rendered.as_bytes())?;
            output.write_stdout_side_channel(result.stdout_after.as_bytes())?;
        }
    }
    Ok(session.id_source.current_ident().saturating_add(1))
}

fn write_pcl_doc_step_start(
    output: &mut String,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    print_clause: bool,
) -> fmt::Result {
    pcl_print_start(
        output,
        bank,
        clause,
        print_clause,
        pcl_step_print_options(config),
    )
}

const fn pcl_step_print_options(config: &EProverConfig) -> PclStepPrintOptions {
    PclStepPrintOptions {
        full_terms: config.pcl_output.full_terms,
        compact: config.pcl_output.compact,
        print_types: config.encoding.print_types,
        eqn_print_options: EqnPrintOptions::tptp().with_print_types(config.encoding.print_types),
    }
}

const fn effective_doc_output_format(config: &EProverConfig) -> DocOutputFormat {
    match config.doc_output_format {
        DocOutputFormat::NoFormat => DocOutputFormat::Pcl,
        format => format,
    }
}

const fn proof_doc_output_format(config: &EProverConfig) -> ProofDocOutputFormat {
    match effective_doc_output_format(config) {
        DocOutputFormat::NoFormat => ProofDocOutputFormat::NoFormat,
        DocOutputFormat::Lop => ProofDocOutputFormat::Lop,
        DocOutputFormat::Pcl => ProofDocOutputFormat::Pcl,
        DocOutputFormat::Tstp => ProofDocOutputFormat::Tstp,
        DocOutputFormat::Tptp => ProofDocOutputFormat::Tptp,
        DocOutputFormat::Xml => ProofDocOutputFormat::Xml,
    }
}

fn proof_doc_write_error(_error: fmt::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "failed to write proof clause documentation",
    )
}

fn write_proof_success_doc(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
) -> Result<(), EProverError> {
    if config.output_level < 2 {
        return Ok(());
    }
    match effective_doc_output_format(config) {
        DocOutputFormat::Pcl => {
            write_pcl_proof_success_doc(output, config, bank, clause, doc_ident)
        }
        DocOutputFormat::Tstp => {
            write_tstp_proof_success_doc(output, config, bank, clause, doc_ident)
        }
        _ => {
            write_comment_line(output, "Output format not implemented.")?;
            Ok(())
        }
    }
}

fn clause_quote_doc_clause_with_parent(
    clause: &Clause,
    doc_ident: i64,
    parent_ident: i64,
) -> (Clause, i64) {
    let mut quoted = clause.clone();
    quoted.del_prop(CP_INPUT_FORMULA);
    quoted.set_ident(doc_ident);
    (quoted, parent_ident)
}

fn write_clause_quote_doc<W: Write + ?Sized>(
    output: &mut ConfiguredOutput<'_, W>,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
    parent_ident: i64,
    comment: &str,
) -> Result<(), EProverError> {
    match effective_doc_output_format(config) {
        DocOutputFormat::Pcl => write_pcl_clause_quote_doc_with_parent(
            output,
            config,
            bank,
            clause,
            doc_ident,
            parent_ident,
            comment,
        ),
        DocOutputFormat::Tstp => write_tstp_clause_quote_doc_with_parent(
            output,
            config,
            bank,
            clause,
            doc_ident,
            parent_ident,
            comment,
        ),
        _ => {
            write_comment_line(output, "Output format not implemented.")?;
            Ok(())
        }
    }
}

fn write_pcl_proof_success_doc(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
) -> Result<(), EProverError> {
    write_pcl_proof_success_doc_with_parent(output, config, bank, clause, doc_ident, clause.ident())
}

fn write_pcl_proof_success_doc_with_parent(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
    parent_ident: i64,
) -> Result<(), EProverError> {
    write_pcl_clause_quote_doc_with_parent(
        output,
        config,
        bank,
        clause,
        doc_ident,
        parent_ident,
        "proof",
    )
}

fn write_pcl_clause_quote_doc_with_parent(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
    parent_ident: i64,
    comment: &str,
) -> Result<(), EProverError> {
    let (quoted, old_id) = clause_quote_doc_clause_with_parent(clause, doc_ident, parent_ident);
    let mut rendered = String::new();
    write_pcl_doc_step_start(
        &mut rendered,
        config,
        bank,
        &quoted,
        config.pcl_output.shell_level < 1,
    )
    .map_err(proof_doc_write_error)?;
    write!(&mut rendered, "{old_id}").map_err(proof_doc_write_error)?;
    pcl_print_end(
        &mut rendered,
        &quoted,
        Some(comment),
        pcl_step_print_options(config),
    )
    .map_err(proof_doc_write_error)?;
    output.write_all(rendered.as_bytes())?;
    Ok(())
}

fn write_tstp_proof_success_doc(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
) -> Result<(), EProverError> {
    write_tstp_proof_success_doc_with_parent(
        output,
        config,
        bank,
        clause,
        doc_ident,
        clause.ident(),
    )
}

fn write_tstp_proof_success_doc_with_parent(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
    parent_ident: i64,
) -> Result<(), EProverError> {
    write_tstp_clause_quote_doc_with_parent(
        output,
        config,
        bank,
        clause,
        doc_ident,
        parent_ident,
        "proof",
    )
}

fn write_tstp_clause_quote_doc_with_parent(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
    parent_ident: i64,
    comment: &str,
) -> Result<(), EProverError> {
    let (quoted, old_id) = clause_quote_doc_clause_with_parent(clause, doc_ident, parent_ident);
    let mut rendered = String::new();
    clause_write_tstp_with_type_suffixes(
        &mut rendered,
        bank,
        &quoted,
        config.pcl_output.full_terms,
        false,
        crate::basics::simple_stuff::ProblemType::FirstOrder,
        config.encoding.print_types,
    )?;
    writeln!(&mut rendered, ", c_0_{old_id},['{comment}']).").map_err(proof_doc_write_error)?;
    output.write_all(rendered.as_bytes())?;
    Ok(())
}

fn write_proof_success_object_output(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &ProofState,
    clause: &Clause,
    next_doc_ident: i64,
) -> Result<(), EProverError> {
    match config.proof_output {
        1 => write_proof_success_list_output(output, config, state, clause, next_doc_ident),
        level if level >= 2 => {
            let roots = proof_success_object_roots(config, state, clause);
            let graph = state.proof_object_graph_for_roots(roots);
            write_proof_object_dot(output, config, state.terms(), &graph)
        }
        _ => Ok(()),
    }
}

fn write_proof_success_list_output(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &ProofState,
    clause: &Clause,
    next_doc_ident: i64,
) -> Result<(), EProverError> {
    write_comment_line(output, "SZS output start CNFRefutation")?;
    let graph =
        state.proof_object_graph_for_roots(proof_success_object_roots(config, state, clause));
    if graph.clauses.is_empty() {
        let doc_ident = proof_object_success_doc_ident(config, clause, next_doc_ident);
        let parent_ident = proof_object_success_parent_ident(config, clause);
        write_proof_success_list_fallback(
            output,
            config,
            state.terms(),
            clause,
            doc_ident,
            parent_ident,
        )?;
    } else {
        write_proof_object_list_graph(output, config, state, &graph)?;
    }
    write_comment_line(output, "SZS output end CNFRefutation")?;
    Ok(())
}

fn write_proof_object_list_graph(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &ProofState,
    graph: &ProofObjectGraph<'_>,
) -> Result<(), EProverError> {
    for (proof_clause, is_root) in proof_object_list_display_clauses(graph) {
        write_saturation_proof_object_clause(
            output,
            config,
            state.terms(),
            &proof_clause,
            is_root,
        )?;
    }
    Ok(())
}

fn proof_object_list_display_clauses(graph: &ProofObjectGraph<'_>) -> Vec<(Clause, bool)> {
    let root_indices: BTreeSet<usize> = graph.root_indices.iter().copied().collect();
    let print_clauses: Vec<(usize, &Clause)> = graph
        .clauses
        .iter()
        .enumerate()
        .rev()
        .map(|(index, clause)| (index, *clause))
        .collect();
    let mut display_ids_by_index = vec![0; graph.clauses.len()];
    let mut fallback_display_ids = BTreeMap::new();
    for (print_index, (graph_index, clause)) in print_clauses.iter().enumerate() {
        let display_id = usize_to_i64(print_index.saturating_add(1));
        display_ids_by_index[*graph_index] = display_id;
        fallback_display_ids
            .entry(ClauseDerivationRef::from(*clause))
            .or_insert(display_id);
    }

    print_clauses
        .into_iter()
        .map(|(graph_index, clause)| {
            let mut display = clause.clone();
            display.set_ident(display_ids_by_index[graph_index]);
            if let Some(derivation) = clause.derivation() {
                display.set_derivation(Some(remap_derivation_for_display(
                    derivation,
                    graph,
                    graph_index,
                    &display_ids_by_index,
                    &fallback_display_ids,
                )));
            }
            let is_root = root_indices.contains(&graph_index);
            (display, is_root)
        })
        .collect()
}

fn remap_derivation_for_display(
    derivation: &PStack<DerivationEntry>,
    graph: &ProofObjectGraph<'_>,
    child_index: usize,
    display_ids_by_index: &[i64],
    fallback_display_ids: &BTreeMap<ClauseDerivationRef, i64>,
) -> PStack<DerivationEntry> {
    let mut remapped = PStack::new();
    for entry in derivation.as_slice() {
        remapped.push(match *entry {
            DerivationEntry::ClauseParent(parent) => {
                let ident = proof_object_display_parent_ident(
                    graph,
                    child_index,
                    parent,
                    display_ids_by_index,
                    fallback_display_ids,
                );
                DerivationEntry::ClauseParent(ClauseDerivationRef::new(ident, parent.source()))
            }
            entry => entry,
        });
    }
    remapped
}

fn proof_object_display_parent_ident(
    graph: &ProofObjectGraph<'_>,
    child_index: usize,
    parent: ClauseDerivationRef,
    display_ids_by_index: &[i64],
    fallback_display_ids: &BTreeMap<ClauseDerivationRef, i64>,
) -> i64 {
    graph
        .edges
        .iter()
        .find(|edge| {
            edge.child_index == child_index
                && ClauseDerivationRef::from(graph.clauses[edge.parent_index]) == parent
        })
        .map_or_else(
            || {
                fallback_display_ids
                    .get(&parent)
                    .copied()
                    .unwrap_or(parent.ident())
            },
            |edge| display_ids_by_index[edge.parent_index],
        )
}

fn write_proof_success_list_fallback(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    doc_ident: i64,
    parent_ident: i64,
) -> Result<(), EProverError> {
    match effective_doc_output_format(config) {
        DocOutputFormat::Pcl => {
            write_pcl_proof_success_doc_with_parent(
                output,
                config,
                bank,
                clause,
                doc_ident,
                parent_ident,
            )?;
        }
        DocOutputFormat::Tstp => {
            write_tstp_proof_success_doc_with_parent(
                output,
                config,
                bank,
                clause,
                doc_ident,
                parent_ident,
            )?;
        }
        _ => write_comment_line(output, "Output format not implemented.")?,
    }
    Ok(())
}

fn stopped_proof_object_status(
    config: &EProverConfig,
    state: &ProofState,
    outcome: &SaturateOutcome,
    inference_system_complete: bool,
) -> Option<&'static str> {
    if state.statistics().answer_count != 0 || !matches!(outcome, SaturateOutcome::Stopped { .. }) {
        return None;
    }
    if is_complete_saturation_proof_object(state, outcome, inference_system_complete) {
        return Some("Saturation");
    }
    (config.force_derivation_output > 0).then_some("Derivation")
}

fn is_complete_saturation_proof_object(
    state: &ProofState,
    outcome: &SaturateOutcome,
    inference_system_complete: bool,
) -> bool {
    state.statistics().answer_count == 0
        && matches!(
            outcome,
            SaturateOutcome::Stopped {
                reason: SaturateStopReason::Saturated,
                ..
            }
        )
        && inference_system_complete
        && state.state_is_complete()
        && !state.has_interpreted_symbols()
}

fn write_stopped_proof_output(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &ProofState,
    status: &str,
) -> Result<(), EProverError> {
    if config.proof_output < 1 {
        return Ok(());
    }
    if config.proof_output >= 2 {
        let graph = state.proof_object_graph_for_roots(stopped_proof_object_roots(config, state));
        return write_proof_object_dot(output, config, state.terms(), &graph);
    }

    writeln!(output, "{DEFAULT_COMCHAR_RAW} SZS output start {status}")?;
    let graph = state.proof_object_graph_for_roots(stopped_proof_object_roots(config, state));
    write_proof_object_list_graph(output, config, state, &graph)?;
    writeln!(output, "{DEFAULT_COMCHAR_RAW} SZS output end {status}")?;
    Ok(())
}

fn write_saturation_proof_object_clause(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    is_root: bool,
) -> Result<(), EProverError> {
    let mut rendered = String::new();
    match effective_doc_output_format(config) {
        DocOutputFormat::Pcl => {
            write_pcl_doc_step_start(&mut rendered, config, bank, clause, true)
                .map_err(proof_doc_write_error)?;
            if let Some(derivation) = deriv_stack_pcl_string_with_ac_axioms(
                clause.derivation(),
                bank.signature().ac_axioms(),
            ) {
                rendered.push_str(&derivation);
            } else {
                rendered.push_str(&source_info_pcl_string(clause.info()));
            }
            if is_root {
                if config.pcl_output.compact {
                    rendered.push(':');
                } else {
                    rendered.push_str(" : ");
                }
                rendered.push_str(proof_object_root_marker(clause));
            }
            rendered.push('\n');
        }
        DocOutputFormat::Tstp => {
            clause_write_tstp_with_type_suffixes(
                &mut rendered,
                bank,
                clause,
                config.pcl_output.full_terms,
                false,
                ProblemType::FirstOrder,
                config.encoding.print_types,
            )?;
            if let Some(derivation) = deriv_stack_tstp_string_with_ac_axioms(
                clause.derivation(),
                bank.signature().ac_axioms(),
            ) {
                rendered.push_str(", ");
                rendered.push_str(&derivation);
            } else {
                let source_info = source_info_tstp_string(clause.info());
                if !source_info.is_empty() {
                    rendered.push_str(", ");
                    rendered.push_str(&source_info);
                }
            }
            if is_root {
                rendered.push_str(", [");
                rendered.push_str(proof_object_root_marker(clause));
                rendered.push(']');
            }
            rendered.push_str(").");
            rendered.push('\n');
        }
        _ => write_comment_line(output, "Output format not implemented.")?,
    }
    output.write_all(rendered.as_bytes())?;
    Ok(())
}

fn proof_object_root_marker(clause: &Clause) -> &'static str {
    if clause.is_empty() {
        "'proof'"
    } else {
        "'final'"
    }
}

fn write_proof_object_dot(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    graph: &ProofObjectGraph<'_>,
) -> Result<(), EProverError> {
    output.write_all(
        b"digraph proof{\n  rankdir=TB\n  graph [splines=true overlap=false];\n  subgraph ax{\n  rank=\"same\";\n",
    )?;
    let mut axiom_open = true;
    for index in (0..graph.clauses.len()).rev() {
        let clause = graph.clauses[index];
        if axiom_open && clause.derivation().is_some() {
            output.write_all(b"   }\n")?;
            axiom_open = false;
        }
        write_proof_object_dot_clause(
            output,
            config,
            bank,
            clause,
            proof_object_dot_node_id(graph, index),
        )?;
    }
    if axiom_open {
        output.write_all(b"   }\n")?;
    }
    for edge in &graph.edges {
        writeln!(
            output,
            "    {} -> {} [style=\"bold\"{}]",
            proof_object_dot_node_id(graph, edge.parent_index),
            proof_object_dot_node_id(graph, edge.child_index),
            proof_object_dot_node_colour(graph.clauses[edge.child_index])
        )?;
    }
    output.write_all(b"}\n")?;
    Ok(())
}

fn write_proof_object_dot_clause(
    output: &mut impl Write,
    config: &EProverConfig,
    bank: &TermBank,
    clause: &Clause,
    node_id: usize,
) -> Result<(), EProverError> {
    let label = if config.proof_output > 2 {
        let mut rendered = String::new();
        clause_write_tstp_with_type_suffixes(
            &mut rendered,
            bank,
            clause,
            true,
            false,
            ProblemType::FirstOrder,
            config.encoding.print_types,
        )?;
        if let Some(derivation) = deriv_stack_tstp_string_with_ac_axioms(
            clause.derivation(),
            bank.signature().ac_axioms(),
        ) {
            rendered.push_str(",\n");
            rendered.push_str(&derivation);
        } else {
            let source_info = source_info_tstp_string(clause.info());
            if !source_info.is_empty() {
                rendered.push_str(",\n");
                rendered.push_str(&source_info);
            }
        }
        rendered.push_str(").");
        rendered
    } else {
        format!("c{node_id}")
    };
    writeln!(
        output,
        "  {} [shape=box{},style=filled,label=\"{}\"]",
        node_id,
        proof_object_dot_node_colour(clause),
        dot_label_escape(&label)
    )?;
    Ok(())
}

fn proof_object_dot_node_id(graph: &ProofObjectGraph<'_>, index: usize) -> usize {
    graph.clauses.len().saturating_sub(index)
}

fn proof_object_dot_node_colour(clause: &Clause) -> &'static str {
    if clause.is_empty() {
        ",color=blue,fillcolor=darkorchid1"
    } else if clause.is_conjecture() {
        ",color=blue,fillcolor=lightskyblue1"
    } else if clause.derivation().is_some() {
        ",color=green,fillcolor=palegreen"
    } else {
        ",color=green,fillcolor=forestgreen"
    }
}

fn dot_label_escape(label: &str) -> String {
    let mut escaped = String::with_capacity(label.len());
    for character in label.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => {}
            _ => escaped.push(character),
        }
    }
    escaped
}

fn write_supported_proof_object_statistics(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &ProofState,
    outcome: &SaturateOutcome,
    inference_system_complete: bool,
) -> Result<(), EProverError> {
    if config.proof_object_level <= 0 || !config.flags.contains(EProverFlag::ProofStatistics) {
        return Ok(());
    }

    let analysis = match outcome {
        SaturateOutcome::Returned { clause, .. } => {
            let roots = proof_success_object_roots(config, state, clause.as_ref());
            state.proof_object_analysis_for_roots(roots)
        }
        SaturateOutcome::Stopped { .. }
            if stopped_proof_object_status(config, state, outcome, inference_system_complete)
                .is_some() =>
        {
            state.proof_object_analysis_for_roots(stopped_proof_object_roots(config, state))
        }
        SaturateOutcome::Stopped { .. } => return Ok(()),
    };

    write_proof_object_analysis(output, analysis)
}

fn proof_success_object_roots<'a>(
    config: &EProverConfig,
    state: &'a ProofState,
    clause: &'a Clause,
) -> Vec<&'a Clause> {
    let mut roots = vec![clause];
    if config.flags.contains(EProverFlag::FullDerivation) {
        roots.extend(proof_object_saturation_roots(state));
        roots.extend(state.unprocessed().iter());
    }
    roots
}

fn proof_object_saturation_roots(state: &ProofState) -> Vec<&Clause> {
    state
        .processed_pos_rules()
        .iter()
        .chain(state.processed_pos_eqns().iter())
        .chain(state.processed_neg_units().iter())
        .chain(state.processed_non_units().iter())
        .collect()
}

fn stopped_proof_object_roots<'a>(
    config: &EProverConfig,
    state: &'a ProofState,
) -> Vec<&'a Clause> {
    let mut roots = proof_object_saturation_roots(state);
    if config.force_derivation_output >= 2 {
        roots.extend(state.unprocessed().iter());
    }
    roots
}

fn write_proof_object_analysis(
    output: &mut impl Write,
    analysis: ProofObjectAnalysis,
) -> Result<(), EProverError> {
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object total steps             : {}",
        analysis.total_step_count()
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object clause steps            : {}",
        analysis.clause_step_count
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object formula steps           : {}",
        analysis.formula_step_count
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object conjectures             : {}",
        analysis.conjecture_count()
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object clause conjectures      : {}",
        analysis.clause_conjecture_count
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object formula conjectures     : {}",
        analysis.formula_conjecture_count
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object initial clauses used    : {}",
        analysis.initial_clause_count
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object initial formulas used   : {}",
        analysis.initial_formula_count
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object generating inferences   : {}",
        analysis.generating_inference_count
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Proof object simplifying inferences  : {}",
        analysis.simplifying_inference_count
    )?;
    Ok(())
}

fn proof_object_success_doc_ident(
    config: &EProverConfig,
    clause: &Clause,
    next_doc_ident: i64,
) -> i64 {
    if config.output_level >= 2 {
        next_doc_ident
    } else {
        proof_object_success_parent_ident(config, clause).saturating_add(1)
    }
}

fn proof_object_success_parent_ident(config: &EProverConfig, clause: &Clause) -> i64 {
    let ident = clause.ident();
    if config.output_level >= 2 || ident >= 0 {
        ident
    } else {
        1
    }
}

fn write_cnf_only_success(output: &mut impl Write) -> Result<(), EProverError> {
    write_comment_line_after_blank(output, "CNFization successful!")?;
    write_tstp_status(output, "Unknown")?;
    Ok(())
}

fn write_proof_search_result(
    output: &mut impl Write,
    config: &EProverConfig,
    outcome: &SaturateOutcome,
    state: &crate::clauses::proofstate::ProofState,
    inference_system_complete: bool,
    assume_inference_system_complete: bool,
) -> Result<(), EProverError> {
    let proof_success_status = proof_success_status(config, outcome, state);
    if state.statistics().answer_count > 0 {
        write_comment_line_after_blank(output, "Proof found!")?;
        if !state.statistics().status_reported {
            write_tstp_status(output, proof_success_status)?;
        }
        return Ok(());
    }

    match outcome {
        SaturateOutcome::Returned { .. } => {
            write_comment_line_after_blank(output, "Proof found!")?;
            if !state.statistics().status_reported {
                write_tstp_status(output, proof_success_status)?;
            }
        }
        SaturateOutcome::Stopped {
            reason: SaturateStopReason::Saturated,
            ..
        } => {
            write_saturated_final_result(
                output,
                config,
                state,
                inference_system_complete,
                assume_inference_system_complete,
            )?;
        }
        SaturateOutcome::Stopped {
            reason: SaturateStopReason::WatchlistEmpty,
            ..
        } => {
            write_comment_line_after_blank(output, "Watchlist is empty!")?;
            write_tstp_status(output, "ResourceOut")?;
        }
        SaturateOutcome::Stopped { .. } => {
            write_comment_line_after_blank(output, "Failure: User resource limit exceeded!")?;
            write_tstp_status(output, "ResourceOut")?;
        }
    }
    Ok(())
}

fn proof_success_status(
    config: &EProverConfig,
    outcome: &SaturateOutcome,
    state: &ProofState,
) -> &'static str {
    if !config.flags.contains(EProverFlag::FormulaConjectureSeen) {
        return "Unsatisfiable";
    }

    match outcome {
        SaturateOutcome::Returned { clause, .. } => {
            if proof_tree_has_conjecture(state, clause) {
                "Theorem"
            } else {
                "ContradictoryAxioms"
            }
        }
        SaturateOutcome::Stopped { .. } => "Theorem",
    }
}

fn proof_tree_has_conjecture(state: &ProofState, root: &Clause) -> bool {
    if root.is_conjecture() {
        return true;
    }

    let mut pending = direct_clause_parent_refs(root);
    let mut seen = vec![(root.ident(), root.query_csscpa_source())];
    while let Some(parent_ref) = pending.pop() {
        let key = clause_derivation_ref_key(parent_ref);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);

        let Some(parent) = proof_state_find_clause_by_ref(state, parent_ref) else {
            continue;
        };
        if parent.is_conjecture() {
            return true;
        }
        pending.extend(direct_clause_parent_refs(parent));
    }
    false
}

fn direct_clause_parent_refs(clause: &Clause) -> Vec<ClauseDerivationRef> {
    let Some(derivation) = clause.derivation() else {
        return Vec::new();
    };

    let entries = derivation.as_slice();
    let mut parents = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        let DerivationEntry::Operation(op) = entries[index] else {
            index += 1;
            continue;
        };
        index += 1;

        collect_direct_clause_parent_arg(
            entries,
            &mut index,
            op_has_cnf_arg1(op),
            op_has_arg1(op),
            &mut parents,
        );
        collect_direct_clause_parent_arg(
            entries,
            &mut index,
            op_has_cnf_arg2(op),
            op_has_arg2(op),
            &mut parents,
        );
    }
    parents
}

fn collect_direct_clause_parent_arg(
    entries: &[DerivationEntry],
    index: &mut usize,
    is_clause_parent: bool,
    has_arg: bool,
    parents: &mut Vec<ClauseDerivationRef>,
) {
    if is_clause_parent {
        if let Some(entry) = entries.get(*index) {
            match entry {
                DerivationEntry::ClauseParent(parent) => parents.push(*parent),
                DerivationEntry::Demodulator(demodulator) => {
                    parents.extend(demodulator_clause_refs(*demodulator));
                }
                DerivationEntry::FormulaParent(_)
                | DerivationEntry::Operation(_)
                | DerivationEntry::NumericArg(_) => {}
            }
        }
        *index += 1;
    } else if has_arg {
        *index += 1;
    }
}

fn proof_state_find_clause_by_ref(
    state: &ProofState,
    parent_ref: ClauseDerivationRef,
) -> Option<&Clause> {
    [
        state.axioms(),
        state.ax_archive(),
        state.processed_pos_rules(),
        state.processed_pos_eqns(),
        state.processed_neg_units(),
        state.processed_non_units(),
        state.unprocessed(),
        state.tmp_store(),
        state.eval_store(),
        state.archive(),
    ]
    .into_iter()
    .find_map(|set| clause_set_find_clause_by_ref(set, parent_ref))
}

fn clause_set_find_clause_by_ref(
    set: &ClauseSet,
    parent_ref: ClauseDerivationRef,
) -> Option<&Clause> {
    let parent_key = clause_derivation_ref_key(parent_ref);
    set.iter().find(|clause| {
        clause.ident() == parent_key.0
            && (parent_key.1 == 0 || clause.query_csscpa_source() == parent_key.1)
    })
}

const fn clause_derivation_ref_key(parent_ref: ClauseDerivationRef) -> (i64, u64) {
    (parent_ref.ident(), parent_ref.source())
}

fn write_saturated_final_result(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    inference_system_complete: bool,
    assume_inference_system_complete: bool,
) -> Result<(), EProverError> {
    if !(inference_system_complete || assume_inference_system_complete) {
        write_comment_line_after_blank(output, "Clause set closed under restricted calculus!")?;
        write_tstp_status(output, "GaveUp")?;
    } else if state.state_is_complete()
        && inference_system_complete
        && state.has_interpreted_symbols()
    {
        write_comment_line_after_blank(output, "Clause set saturated up to interpreted theories!")?;
        write_tstp_status(output, "GaveUp")?;
    } else if state.state_is_complete() && inference_system_complete {
        write_comment_line_after_blank(output, "No proof found!")?;
        let saturated_status = if config.flags.contains(EProverFlag::FormulaConjectureSeen) {
            "CounterSatisfiable"
        } else {
            "Satisfiable"
        };
        write_tstp_status(output, saturated_status)?;
    } else {
        write_comment_line_after_blank(output, "Failure: Out of unprocessed clauses!")?;
        write_tstp_status(output, "GaveUp")?;
    }
    Ok(())
}

fn write_answer_outputs(
    output: &mut impl Write,
    state: &mut crate::clauses::proofstate::ProofState,
) -> Result<(), EProverError> {
    for answer_output in state.take_answer_outputs() {
        output.write_all(answer_output.as_bytes())?;
    }
    Ok(())
}

fn write_training_examples(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    flags: i64,
) -> Result<(), EProverError> {
    let examples = state.pick_training_examples();
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Training examples: {} positive, {} negative",
        examples.positive.len(),
        examples.negative.len()
    )?;
    if flags & TRAINING_PRINT_POS != 0 {
        write_comment_line(output, "Training: Positive examples begin")?;
        write_training_clause_examples(output, config, state, &examples.positive, "% trainpos")?;
        write_comment_line(output, "Training: Positive examples end")?;
    }
    if flags & TRAINING_PRINT_NEG != 0 {
        write_comment_line(output, "Training: Negative examples begin")?;
        write_training_clause_examples(output, config, state, &examples.negative, "%trainneg")?;
        write_comment_line(output, "Training: Negative examples end")?;
    }
    Ok(())
}

fn write_training_clause_examples(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    clauses: &[&Clause],
    extra: &str,
) -> Result<(), EProverError> {
    let eqn_print_options = config
        .equation_print
        .into_eqn_print_options(config.output_format)
        .with_print_types(config.encoding.print_types);
    for clause in clauses {
        let rendered = clause_print_for_output_format(
            state.terms(),
            clause,
            config.output_format,
            eqn_print_options,
            config.encoding.print_types,
        )?;
        output.write_all(rendered.as_bytes())?;
        output.write_all(extra.as_bytes())?;
        output.write_all(b"\n")?;
    }
    Ok(())
}

fn write_tstp_status(output: &mut impl Write, status: &str) -> Result<(), EProverError> {
    writeln!(output, "{DEFAULT_COMCHAR_RAW} SZS status {status}")?;
    Ok(())
}

fn write_comment_line(output: &mut impl Write, message: &str) -> Result<(), EProverError> {
    writeln!(output, "{DEFAULT_COMCHAR_RAW} {message}")?;
    Ok(())
}

fn write_comment_line_after_blank(
    output: &mut impl Write,
    message: &str,
) -> Result<(), EProverError> {
    output.write_all(b"\n")?;
    write_comment_line(output, message)
}

fn write_preprocessing_time(
    output: &mut impl Write,
    config: &EProverConfig,
) -> Result<(), EProverError> {
    if !config.flags.contains(EProverFlag::ResourceInfo) {
        return Ok(());
    }
    let usage = current_resource_usage();
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Preprocessing time       : {:.3} s",
        usage.user_time_seconds + usage.system_time_seconds
    )?;
    Ok(())
}

fn write_saturated_output(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    success: Option<&Clause>,
) -> Result<(), EProverError> {
    if !config.flags.contains(EProverFlag::PrintSaturated) {
        return Ok(());
    }
    let eqn_print_options = config
        .equation_print
        .into_eqn_print_options(config.output_format)
        .with_print_types(config.encoding.print_types);
    if let Some(success) = success {
        write_comment_line(output, "Saturated system contains the empty clause:")?;
        let rendered = clause_print_for_output_format(
            state.terms(),
            success,
            config.output_format,
            eqn_print_options,
            config.encoding.print_types,
        )?;
        output.write_all(rendered.as_bytes())?;
        output.write_all(b"\n\n")?;
    }
    let rendered = proof_state_print_selective_string(
        state,
        &config.saturated_output_descriptor,
        config.flags.contains(EProverFlag::PrintSaturatedInfo),
        config.output_format,
        eqn_print_options,
    )?;
    output.write_all(rendered.as_bytes())?;
    output.write_all(b"\n")?;
    Ok(())
}

fn clause_print_for_output_format(
    bank: &TermBank,
    clause: &Clause,
    output_format: IoFormat,
    eqn_print_options: EqnPrintOptions,
    print_types: bool,
) -> Result<String, Diagnostic> {
    match output_format {
        IoFormat::Tptp => Ok(clause_print_tptp_format_string_with_options(
            bank,
            clause,
            eqn_print_options,
        )),
        IoFormat::Tstp => {
            let mut rendered = String::new();
            clause_write_tstp_with_type_suffixes(
                &mut rendered,
                bank,
                clause,
                true,
                true,
                ProblemType::FirstOrder,
                print_types,
            )?;
            Ok(rendered)
        }
        IoFormat::Lop | IoFormat::Auto => Ok(clause_print_lop_format_string_with_options(
            bank,
            clause,
            true,
            eqn_print_options,
        )),
    }
}

fn write_proof_statistics(
    output: &mut impl Write,
    config: &EProverConfig,
    state: &crate::clauses::proofstate::ProofState,
    parsed_ax_no: i64,
    relevancy_pruned: i64,
    raw_clause_no: i64,
    preproc_removed: i64,
) -> Result<(), EProverError> {
    if config.output_level <= 1 && !config.flags.contains(EProverFlag::PrintStatistics) {
        return Ok(());
    }
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Parsed axioms                        : {parsed_ax_no}"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Removed by relevancy pruning/SinE    : {relevancy_pruned}"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Initial clauses                      : {raw_clause_no}"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Removed in clause preprocessing      : {preproc_removed}"
    )?;
    output.write_all(
        state
            .statistics_string(config.flags.contains(EProverFlag::RecordGivenClauses))
            .as_bytes(),
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Clause-clause subsumption calls (NU) : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Rec. Clause-clause subsumption calls : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Non-unit clause-clause subsumptions  : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Unit Clause-clause subsumption calls : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Rewrite failures with RHS unbound    : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} BW rewrite match attempts            : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} BW rewrite match successes           : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Condensation attempts                : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Condensation successes               : 0"
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Termbank termtop insertions          : {}",
        state.terms().insertions()
    )?;
    writeln!(
        output,
        "{DEFAULT_COMCHAR_RAW} Search garbage collected termcells   : {}",
        state.terms().recovered()
    )?;
    Ok(())
}

fn saturate_outcome_exit_status(
    outcome: &SaturateOutcome,
    state: &crate::clauses::proofstate::ProofState,
    inference_system_complete: bool,
    assume_inference_system_complete: bool,
) -> u8 {
    if state.statistics().answer_count > 0 {
        return ErrorCode::PROOF_FOUND.exit_status();
    }
    match outcome {
        SaturateOutcome::Returned { .. } => ErrorCode::PROOF_FOUND.exit_status(),
        SaturateOutcome::Stopped {
            reason: SaturateStopReason::Saturated,
            ..
        } if inference_system_complete
            && state.state_is_complete()
            && !state.has_interpreted_symbols() =>
        {
            ErrorCode::SATISFIABLE.exit_status()
        }
        SaturateOutcome::Stopped {
            reason: SaturateStopReason::Saturated,
            ..
        } if !inference_system_complete && !assume_inference_system_complete => {
            ErrorCode::INCOMPLETE_PROOFSTATE.exit_status()
        }
        SaturateOutcome::Stopped {
            reason: SaturateStopReason::Saturated,
            ..
        } => ErrorCode::INCOMPLETE_PROOFSTATE.exit_status(),
        SaturateOutcome::Stopped { .. } => ErrorCode::RESOURCE_OUT.exit_status(),
    }
}

fn proof_search_inference_system_complete(
    state: &crate::clauses::proofstate::ProofState,
    control: &ProofControl,
) -> bool {
    let heuristic = control.heuristic_parms();
    !state
        .terms()
        .signature()
        .has_unimplemented_interpreted_symbols()
        && heuristic.selection_strategy != NO_GENERATION
        && heuristic.order_params.lit_cmp != i64::from(to_params::LiteralCmp::TfoEqMax.c_value())
        && heuristic.enable_eq_factoring
        && heuristic.enable_neg_unit_paramod
}

fn parse_clause_file(
    file: &str,
    parse_format: IoFormat,
    formula_preprocessing: FormulaPreprocessing,
    bank: &mut TermBank,
    clauses: &mut ClauseSet,
    watchlist: &mut ClauseSet,
) -> Result<ParsedClauseFile, Diagnostic> {
    set_problem_type(ProblemType::FirstOrder)?;
    let mut scanner = if file == "-" {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input).map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot read standard input: {error}"),
            )
        })?;
        Scanner::from_file_content("-", input, false)?
    } else {
        Scanner::from_file(Path::new(file), false)?
    };
    scanner.set_format(parse_format);
    let detected_format = scanner.format();
    let mut formula_conjecture_seen = false;
    let mut raw_formula_features = RawFormulaFeatures::default();
    match detected_format {
        IoFormat::Tstp => {
            let parsed = parse_tstp_entry_list(
                &mut scanner,
                bank,
                clauses,
                watchlist,
                None,
                formula_preprocessing,
            )?;
            formula_conjecture_seen = parsed.formula_conjecture_seen;
            raw_formula_features.add(parsed.raw_formula_features);
        }
        IoFormat::Tptp => {
            let parsed = parse_tptp_entry_list(
                &mut scanner,
                bank,
                clauses,
                watchlist,
                None,
                formula_preprocessing,
            )?;
            formula_conjecture_seen = parsed.formula_conjecture_seen;
            raw_formula_features.add(parsed.raw_formula_features);
        }
        _ => {
            clauses.parse_list(&mut scanner, bank, ProblemType::FirstOrder)?;
        }
    }
    if !scanner.test_tok(TokenType::NO_TOKEN) {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{}(just read '{}'): Unexpected token after clause list",
                token_pos_rep(scanner.current_token()),
                scanner.current_token().literal()
            ),
        ));
    }
    Ok(ParsedClauseFile {
        detected_format,
        formula_conjecture_seen,
        raw_formula_features,
    })
}

fn parse_app_encode_file(
    file: &str,
    parse_format: IoFormat,
    bank: &mut TermBank,
) -> Result<ParsedAppEncodeFile, Diagnostic> {
    set_problem_type(ProblemType::FirstOrder)?;
    let mut scanner = if file == "-" {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input).map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot read standard input: {error}"),
            )
        })?;
        Scanner::from_file_content("-", input, false)?
    } else {
        Scanner::from_file(Path::new(file), false)?
    };
    scanner.set_format(parse_format);
    let detected_format = scanner.format();
    let (saw_input_owner, saw_formula_owner, formulas) = match detected_format {
        IoFormat::Tstp => parse_tstp_app_encode_entry_list(&mut scanner, bank)?,
        IoFormat::Tptp => parse_tptp_app_encode_entry_list(&mut scanner, bank)?,
        _ => {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{}(just read '{}'): --app-encode currently supports TPTP/TSTP formula input",
                    token_pos_rep(scanner.current_token()),
                    scanner.current_token().literal()
                ),
            ));
        }
    };
    if !scanner.test_tok(TokenType::NO_TOKEN) {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{}(just read '{}'): Unexpected token after app-encode input",
                token_pos_rep(scanner.current_token()),
                scanner.current_token().literal()
            ),
        ));
    }
    Ok(ParsedAppEncodeFile {
        detected_format,
        saw_input_owner,
        saw_formula_owner,
        formulas,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ParsedClauseFile {
    detected_format: IoFormat,
    formula_conjecture_seen: bool,
    raw_formula_features: RawFormulaFeatures,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ParsedEntryList {
    formula_conjecture_seen: bool,
    raw_formula_features: RawFormulaFeatures,
}

impl ParsedEntryList {
    fn add(&mut self, other: Self) {
        self.formula_conjecture_seen |= other.formula_conjecture_seen;
        self.raw_formula_features.add(other.raw_formula_features);
    }
}

#[derive(Clone, Debug)]
struct ParsedAppEncodeFile {
    detected_format: IoFormat,
    saw_input_owner: bool,
    saw_formula_owner: bool,
    formulas: Vec<SimpleAppEncodedFormula>,
}

#[derive(Clone, Debug)]
struct SimpleAppEncodedFormula {
    name: String,
    type_: FormulaProperties,
    formulas: Vec<SimpleFofFormula>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SimpleFofFormula {
    Truth(bool),
    Literal(Eqn),
    Implication {
        antecedents: Vec<SimpleFofFormula>,
        consequents: Vec<SimpleFofFormula>,
    },
    ReverseImplication {
        antecedents: Vec<SimpleFofFormula>,
        consequents: Vec<SimpleFofFormula>,
    },
    Equivalence {
        left: Vec<SimpleFofFormula>,
        right: Vec<SimpleFofFormula>,
    },
    Xor {
        left: Vec<SimpleFofFormula>,
        right: Vec<SimpleFofFormula>,
    },
    Nand {
        left: Vec<SimpleFofFormula>,
        right: Vec<SimpleFofFormula>,
    },
    Nor {
        left: Vec<SimpleFofFormula>,
        right: Vec<SimpleFofFormula>,
    },
    Conjunction(Vec<SimpleFofFormula>),
    Disjunction(Vec<SimpleFofFormula>),
    Negation(Vec<SimpleFofFormula>),
    Universal {
        bound: Vec<Term>,
        formulas: Vec<SimpleFofFormula>,
    },
    Existential {
        bound: Vec<Term>,
        formulas: Vec<SimpleFofFormula>,
    },
}

fn parse_tptp_app_encode_entry_list(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<(bool, bool, Vec<SimpleAppEncodedFormula>), Diagnostic> {
    let mut formulas = Vec::new();
    let mut saw_input_owner = false;
    let mut saw_formula_owner = false;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        if scanner.test_id("input_clause") {
            saw_input_owner = true;
            let _clause = clause_parse(scanner, bank, ProblemType::FirstOrder)?;
        } else if scanner.test_id("input_formula") {
            saw_input_owner = true;
            saw_formula_owner = true;
            formulas.push(parse_simple_tptp_app_encode_formula(scanner, bank)?);
        } else if scanner.test_id("include") {
            parse_app_encode_ignored_include(scanner)?;
        } else {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{}(just read '{}'): --app-encode currently supports input_clause clauses and input_formula formula entries for old TPTP input",
                    token_pos_rep(scanner.current_token()),
                    scanner.current_token().literal()
                ),
            ));
        }
    }
    Ok((saw_input_owner, saw_formula_owner, formulas))
}

fn parse_tstp_app_encode_entry_list(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<(bool, bool, Vec<SimpleAppEncodedFormula>), Diagnostic> {
    let mut formulas = Vec::new();
    let mut saw_input_owner = false;
    let mut saw_formula_owner = false;
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        if scanner.test_id("cnf") {
            saw_input_owner = true;
            let _clause = clause_parse(scanner, bank, ProblemType::FirstOrder)?;
        } else if scanner.test_id("fof|tff|tcf") {
            if let Some(formula) = parse_simple_tstp_app_encode_formula(scanner, bank)? {
                saw_input_owner = true;
                saw_formula_owner = true;
                formulas.push(formula);
            }
        } else if scanner.test_id("include") {
            parse_app_encode_ignored_include(scanner)?;
        } else if scanner.test_id("thf") {
            return Err(thf_requires_hol_error(scanner));
        } else {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{}(just read '{}'): --app-encode currently supports cnf clauses and fof/tff/tcf formula entries for TSTP input",
                    token_pos_rep(scanner.current_token()),
                    scanner.current_token().literal()
                ),
            ));
        }
    }
    Ok((saw_input_owner, saw_formula_owner, formulas))
}

fn parse_app_encode_ignored_include(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    scanner.accept_id("include")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    scanner.accept_tok(TokenType::SQ_STRING)?;
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        parse_skip_parenthesized_expr(scanner)?;
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    Ok(())
}

fn parse_tptp_entry_list(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    clauses: &mut ClauseSet,
    watchlist: &mut ClauseSet,
    mut selectors: Option<&mut StrTree<i64, i64>>,
    formula_preprocessing: FormulaPreprocessing,
) -> Result<ParsedEntryList, Diagnostic> {
    let mut result = ParsedEntryList::default();
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        if scanner.test_id("input_clause") {
            let clause = clause_parse(scanner, bank, ProblemType::FirstOrder)?;
            if tstp_entry_selected(
                clause.info().and_then(ClauseInfo::name),
                selectors.as_deref_mut(),
            ) {
                insert_input_or_watchlist_clause(clauses, watchlist, clause);
            }
        } else if scanner.test_id("input_formula") {
            let parsed = parse_simple_tptp_formula_clause(scanner, bank, formula_preprocessing)?;
            if tstp_entry_selected(Some(parsed.name.as_str()), selectors.as_deref_mut()) {
                if parsed.raw_formula_type != CP_TYPE_WATCH_CLAUSE {
                    result.formula_conjecture_seen |= parsed.formula_conjecture_seen;
                    result.raw_formula_features.add(parsed.raw_formula_features);
                }
                for clause in parsed.clauses {
                    insert_input_or_watchlist_clause(clauses, watchlist, clause);
                }
            }
        } else if scanner.test_id("include") {
            let mut include_selectors = StrTree::new();
            let skip_includes = StrTree::new();
            if let Some(mut included) =
                scanner.parse_include(&mut include_selectors, &skip_includes)?
            {
                result.add(parse_tptp_entry_list(
                    &mut included,
                    bank,
                    clauses,
                    watchlist,
                    Some(&mut include_selectors),
                    formula_preprocessing,
                )?);
            }
        } else {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{}(just read '{}'): TPTP input currently supports input_clause clauses and the temporary atomic/connective-fragment input_formula bridge",
                    token_pos_rep(scanner.current_token()),
                    scanner.current_token().literal()
                ),
            ));
        }
    }
    if let Some(selector_tree) = selectors.as_ref() {
        check_tstp_include_selectors_found(scanner, selector_tree)?;
    }
    Ok(result)
}

fn parse_tstp_entry_list(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    clauses: &mut ClauseSet,
    watchlist: &mut ClauseSet,
    mut selectors: Option<&mut StrTree<i64, i64>>,
    formula_preprocessing: FormulaPreprocessing,
) -> Result<ParsedEntryList, Diagnostic> {
    let mut result = ParsedEntryList::default();
    while !scanner.test_tok(TokenType::NO_TOKEN) {
        if scanner.test_id("cnf") {
            let clause = clause_parse(scanner, bank, ProblemType::FirstOrder)?;
            if tstp_entry_selected(
                clause.info().and_then(ClauseInfo::name),
                selectors.as_deref_mut(),
            ) {
                insert_input_or_watchlist_clause(clauses, watchlist, clause);
            }
        } else if scanner.test_id("fof|tff|tcf") {
            let parsed = parse_simple_tstp_formula_clause(scanner, bank, formula_preprocessing)?;
            if tstp_entry_selected(Some(parsed.name.as_str()), selectors.as_deref_mut()) {
                if parsed.raw_formula_type != CP_TYPE_WATCH_CLAUSE {
                    result.formula_conjecture_seen |= parsed.formula_conjecture_seen;
                    result.raw_formula_features.add(parsed.raw_formula_features);
                }
                for clause in parsed.clauses {
                    insert_input_or_watchlist_clause(clauses, watchlist, clause);
                }
            }
        } else if scanner.test_id("include") {
            let mut include_selectors = StrTree::new();
            let skip_includes = StrTree::new();
            if let Some(mut included) =
                scanner.parse_include(&mut include_selectors, &skip_includes)?
            {
                result.add(parse_tstp_entry_list(
                    &mut included,
                    bank,
                    clauses,
                    watchlist,
                    Some(&mut include_selectors),
                    formula_preprocessing,
                )?);
            }
        } else if scanner.test_id("thf") {
            return Err(thf_requires_hol_error(scanner));
        } else {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                format!(
                    "{}(just read '{}'): TSTP input currently supports cnf clauses, first-order fof/tff/tcf type declarations, and the temporary atomic/connective-fragment fof/tff/tcf bridge",
                    token_pos_rep(scanner.current_token()),
                    scanner.current_token().literal()
                ),
            ));
        }
    }
    if let Some(selector_tree) = selectors.as_ref() {
        check_tstp_include_selectors_found(scanner, selector_tree)?;
    }
    Ok(result)
}

fn insert_input_or_watchlist_clause(
    clauses: &mut ClauseSet,
    watchlist: &mut ClauseSet,
    clause: Clause,
) {
    if clause.query_tptp_type() == CP_TYPE_WATCH_CLAUSE {
        watchlist.insert(clause);
    } else {
        clauses.insert(clause);
    }
}

fn thf_requires_hol_error(scanner: &Scanner) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): {THF_REQUIRES_HOL_MESSAGE}",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal()
        ),
    )
}

fn tstp_formula_free_variables_error(position: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!("{position} {TSTP_FORMULA_FREE_VARIABLES_MESSAGE}"),
    )
}

fn tstp_entry_selected(name: Option<&str>, selectors: Option<&mut StrTree<i64, i64>>) -> bool {
    let Some(selectors) = selectors else {
        return true;
    };
    if selectors.is_empty() {
        return true;
    }
    if selectors.find(EMPTY_INCLUDE_SELECTOR_SENTINEL).is_some() {
        return false;
    }
    let Some(name) = name else {
        return false;
    };
    let Some(entry) = selectors.find_mut(name) else {
        return false;
    };
    entry.val1 = 1;
    true
}

fn check_tstp_include_selectors_found(
    scanner: &Scanner,
    selectors: &StrTree<i64, i64>,
) -> Result<(), Diagnostic> {
    let missing = selectors
        .iter()
        .filter_map(|(name, entry)| (entry.val1 == 0).then_some(name))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }

    let mut message = String::new();
    if let Some(include_pos) = scanner.include_pos() {
        message.push_str(include_pos);
        message.push(' ');
    }
    message
        .push_str("\"include\" statement cannot find the following requested clauses/formulae in ");
    message.push_str(&String::from_utf8_lossy(
        scanner.current_token().source_bytes(),
    ));
    message.push_str(": ");
    message.push_str(&missing.join(", "));
    Err(Diagnostic::new(ErrorCode::INPUT_SEMANTIC_ERROR, message))
}

struct ParsedSimpleFofClause {
    name: String,
    raw_formula_type: FormulaProperties,
    clauses: Vec<Clause>,
    formula_conjecture_seen: bool,
    raw_formula_features: RawFormulaFeatures,
}

struct SimpleFofBoundVariable {
    name: String,
    variable: Option<Term>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FormulaPreprocessing {
    annotate_questions: bool,
    add_answer_literals: bool,
    conjectures_are_questions: bool,
}

impl FormulaPreprocessing {
    const PARSE_ONLY: Self = Self {
        annotate_questions: false,
        add_answer_literals: false,
        conjectures_are_questions: false,
    };

    fn from_config(config: &EProverConfig) -> Self {
        Self {
            annotate_questions: true,
            add_answer_literals: config.answer_limit > 0,
            conjectures_are_questions: config.flags.contains(EProverFlag::ConjecturesAreQuestions),
        }
    }
}

fn parse_simple_tstp_app_encode_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Option<SimpleAppEncodedFormula>, Diagnostic> {
    bank.vars().clear_ext_names();

    let formula_kind = scanner.current_token().literal();
    scanner.accept_id("fof|tff|tcf")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME | TokenType::POS_INT | TokenType::SQ_STRING)?;
    scanner.accept_tok(TokenType::COMMA)?;
    if scanner.test_id("type") {
        scanner.accept_id("type")?;
        scanner.accept_tok(TokenType::COMMA)?;
        bank.signature_mut()
            .parse_tff_type_declaration(scanner, ProblemType::FirstOrder)?;
        parse_simple_tstp_optional_source(scanner)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        scanner.accept_tok(TokenType::FULLSTOP)?;
        return Ok(None);
    }

    let roles = if formula_kind == "tcf" {
        "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question|watchlist"
    } else {
        "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question"
    };
    scanner.check_id(roles)?;
    let role = scanner.current_token().literal();
    scanner.accept_tok(TokenType::IDENT)?;
    scanner.accept_tok(TokenType::COMMA)?;

    let formula_type = clause_type_from_identifier(&role, ProblemType::FirstOrder);
    let formula_position = token_pos_rep(scanner.current_token());
    let formulas = parse_simple_fof_formulas(scanner, bank)?;
    if !simple_fof_global_free_variables(&formulas).is_empty() {
        return Err(tstp_formula_free_variables_error(&formula_position));
    }
    if scanner.test_tok(TokenType::FOF_BIN_OP | TokenType::EXIST_QUANTOR) {
        return Err(simple_fof_unsupported_error(scanner));
    }
    parse_simple_tstp_optional_source(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;

    Ok(Some(SimpleAppEncodedFormula {
        name,
        type_: formula_type | CP_INPUT_FORMULA,
        formulas,
    }))
}

fn parse_simple_tptp_app_encode_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<SimpleAppEncodedFormula, Diagnostic> {
    bank.vars().clear_ext_names();

    scanner.accept_id("input_formula")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME | TokenType::POS_INT)?;
    scanner.accept_tok(TokenType::COMMA)?;
    scanner.check_id("axiom|hypothesis|negated_conjecture|conjecture|question|lemma|unknown")?;
    let role = scanner.current_token().literal();
    scanner.accept_tok(TokenType::IDENT)?;
    scanner.accept_tok(TokenType::COMMA)?;

    let formula_type = old_tptp_input_formula_clause_type(&role);
    let formulas = parse_simple_old_tptp_fof_formulas(scanner, bank)?;
    if scanner.test_tok(TokenType::FOF_BIN_OP | TokenType::EXIST_QUANTOR) {
        return Err(simple_fof_unsupported_error(scanner));
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;

    Ok(SimpleAppEncodedFormula {
        name,
        type_: formula_type | CP_INPUT_FORMULA,
        formulas,
    })
}

fn parse_simple_tstp_formula_clause(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    formula_preprocessing: FormulaPreprocessing,
) -> Result<ParsedSimpleFofClause, Diagnostic> {
    bank.vars().clear_ext_names();
    let start_source = String::from_utf8_lossy(scanner.current_token().source_bytes()).into_owned();
    let start_line = usize_to_i64(scanner.current_token().line());
    let start_column = usize_to_i64(scanner.current_token().column());

    let formula_kind = scanner.current_token().literal();
    scanner.accept_id("fof|tff|tcf")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME | TokenType::POS_INT | TokenType::SQ_STRING)?;
    scanner.accept_tok(TokenType::COMMA)?;
    if scanner.test_id("type") {
        scanner.accept_id("type")?;
        scanner.accept_tok(TokenType::COMMA)?;
        bank.signature_mut()
            .parse_tff_type_declaration(scanner, ProblemType::FirstOrder)?;
        parse_simple_tstp_optional_source(scanner)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        scanner.accept_tok(TokenType::FULLSTOP)?;
        return Ok(ParsedSimpleFofClause {
            name,
            raw_formula_type: CP_TYPE_AXIOM,
            clauses: Vec::new(),
            formula_conjecture_seen: false,
            raw_formula_features: RawFormulaFeatures::default(),
        });
    }

    let roles = if formula_kind == "tcf" {
        "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question|watchlist"
    } else {
        "axiom|definition|theorem|assumption|hypothesis|conjecture|negated_conjecture|lemma|unknown|plain|question"
    };
    scanner.check_id(roles)?;
    let role = scanner.current_token().literal();
    scanner.accept_tok(TokenType::IDENT)?;
    scanner.accept_tok(TokenType::COMMA)?;

    let mut clause_type = clause_type_from_identifier(&role, ProblemType::FirstOrder);
    let raw_formula_type = clause_type;
    let annotate_question = should_annotate_question(clause_type, formula_preprocessing);
    if annotate_question {
        clause_type = CP_TYPE_CONJECTURE;
    }
    let formula_conjecture_seen = clause_type == CP_TYPE_CONJECTURE;
    if formula_conjecture_seen {
        clause_type = CP_TYPE_NEG_CONJECTURE;
    }
    let formula_position = token_pos_rep(scanner.current_token());
    let mut formulas = parse_simple_fof_formulas(scanner, bank)?;
    if !simple_fof_global_free_variables(&formulas).is_empty() {
        return Err(tstp_formula_free_variables_error(&formula_position));
    }
    let raw_formula_features = simple_fof_raw_formula_features(&formulas, raw_formula_type, bank);
    if annotate_question {
        formulas = simple_fof_annotate_question_formulas(
            formulas,
            formula_preprocessing.add_answer_literals,
            bank,
        )?;
    }
    let literal_lists =
        simple_fof_formulas_to_clause_literal_lists(formulas, formula_conjecture_seen, bank)?;
    if scanner.test_tok(TokenType::FOF_BIN_OP | TokenType::EXIST_QUANTOR) {
        return Err(simple_fof_unsupported_error(scanner));
    }
    parse_simple_tstp_optional_source(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;

    let mut clauses = Vec::with_capacity(literal_lists.len());
    for literals in literal_lists {
        let mut clause = Clause::alloc(literals);
        clause.set_tptp_type(clause_type);
        clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
        clause.set_info(Some(ClauseInfo::new(
            Some(name.as_str()),
            Some(start_source.as_str()),
            start_line,
            start_column,
        )));
        clauses.push(clause);
    }
    let raw_formula_features =
        simple_fof_raw_formula_features_with_lowered_clauses(raw_formula_features, &clauses);
    Ok(ParsedSimpleFofClause {
        name,
        raw_formula_type,
        clauses,
        formula_conjecture_seen,
        raw_formula_features,
    })
}

fn parse_simple_tptp_formula_clause(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    formula_preprocessing: FormulaPreprocessing,
) -> Result<ParsedSimpleFofClause, Diagnostic> {
    bank.vars().clear_ext_names();
    let start_source = String::from_utf8_lossy(scanner.current_token().source_bytes()).into_owned();
    let start_line = usize_to_i64(scanner.current_token().line());
    let start_column = usize_to_i64(scanner.current_token().column());

    scanner.accept_id("input_formula")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let name = scanner.current_token().literal();
    scanner.accept_tok(TokenType::NAME | TokenType::POS_INT)?;
    scanner.accept_tok(TokenType::COMMA)?;
    scanner.check_id("axiom|hypothesis|negated_conjecture|conjecture|question|lemma|unknown")?;
    let role = scanner.current_token().literal();
    scanner.accept_tok(TokenType::IDENT)?;
    scanner.accept_tok(TokenType::COMMA)?;

    let mut clause_type = old_tptp_input_formula_clause_type(&role);
    let raw_formula_type = clause_type;
    let annotate_question = should_annotate_question(clause_type, formula_preprocessing);
    if annotate_question {
        clause_type = CP_TYPE_CONJECTURE;
    }
    let formula_conjecture_seen = clause_type == CP_TYPE_CONJECTURE;
    if formula_conjecture_seen {
        clause_type = CP_TYPE_NEG_CONJECTURE;
    }
    let mut formulas = parse_simple_old_tptp_fof_formulas(scanner, bank)?;
    let raw_formula_features = simple_fof_raw_formula_features(&formulas, raw_formula_type, bank);
    if annotate_question {
        formulas = simple_fof_annotate_question_formulas(
            formulas,
            formula_preprocessing.add_answer_literals,
            bank,
        )?;
    }
    let literal_lists =
        simple_fof_formulas_to_clause_literal_lists(formulas, formula_conjecture_seen, bank)?;
    if scanner.test_tok(TokenType::FOF_BIN_OP | TokenType::EXIST_QUANTOR) {
        return Err(simple_fof_unsupported_error(scanner));
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;

    let mut clauses = Vec::with_capacity(literal_lists.len());
    for literals in literal_lists {
        let mut clause = Clause::alloc(literals);
        clause.set_tptp_type(clause_type);
        clause.set_prop(CP_INITIAL | CP_INPUT_FORMULA);
        clause.set_info(Some(ClauseInfo::new(
            Some(name.as_str()),
            Some(start_source.as_str()),
            start_line,
            start_column,
        )));
        clauses.push(clause);
    }
    let raw_formula_features =
        simple_fof_raw_formula_features_with_lowered_clauses(raw_formula_features, &clauses);
    Ok(ParsedSimpleFofClause {
        name,
        raw_formula_type,
        clauses,
        formula_conjecture_seen,
        raw_formula_features,
    })
}

fn simple_fof_raw_formula_features(
    formulas: &[SimpleFofFormula],
    formula_type: FormulaProperties,
    bank: &TermBank,
) -> RawFormulaFeatures {
    let order = simple_fof_formulas_order(formulas);
    let (conjecture_count, hypothesis_count, conj_order) = if formula_type.is_conjecture() {
        (1, 0, order)
    } else if formula_type.is_hypothesis() {
        (0, 1, order)
    } else {
        (0, 0, 0)
    };

    RawFormulaFeatures {
        sentence_no: 1,
        term_size: simple_fof_formulas_standard_weight(formulas, bank),
        conjecture_count,
        hypothesis_count,
        order,
        conj_order,
        ..RawFormulaFeatures::default()
    }
}

fn simple_fof_raw_formula_features_with_lowered_clauses(
    mut features: RawFormulaFeatures,
    clauses: &[Clause],
) -> RawFormulaFeatures {
    features.lowered_clause_no = usize_to_i64(clauses.len());
    features.lowered_clause_term_size = clauses.iter().map(Clause::standard_weight).sum();
    for clause in clauses {
        if clause.is_conjecture() {
            features.lowered_conjecture_count += 1;
        }
        if clause.is_hypothesis() {
            features.lowered_hypothesis_count += 1;
        }
    }
    features
}

fn simple_fof_formulas_standard_weight(formulas: &[SimpleFofFormula], bank: &TermBank) -> i64 {
    match formulas {
        [] => FOF_LOGICAL_SYMBOL_WEIGHT,
        [formula] => simple_fof_formula_standard_weight(formula, bank),
        many => {
            (usize_to_i64(many.len()).saturating_sub(1) * FOF_LOGICAL_SYMBOL_WEIGHT)
                + many
                    .iter()
                    .map(|formula| simple_fof_formula_standard_weight(formula, bank))
                    .sum::<i64>()
        }
    }
}

fn simple_fof_formula_standard_weight(formula: &SimpleFofFormula, bank: &TermBank) -> i64 {
    match formula {
        SimpleFofFormula::Truth(_) => FOF_LOGICAL_SYMBOL_WEIGHT,
        SimpleFofFormula::Literal(literal) => {
            let atom_weight = if literal.is_equ_lit(bank) {
                FOF_LOGICAL_SYMBOL_WEIGHT
                    + term_standard_weight(literal.left())
                    + term_standard_weight(literal.right())
            } else {
                term_standard_weight(literal.left())
            };
            if literal.is_positive() {
                atom_weight
            } else {
                atom_weight + FOF_LOGICAL_SYMBOL_WEIGHT
            }
        }
        SimpleFofFormula::Implication {
            antecedents,
            consequents,
        }
        | SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents,
        } => {
            FOF_LOGICAL_SYMBOL_WEIGHT
                + simple_fof_formulas_standard_weight(antecedents, bank)
                + simple_fof_formulas_standard_weight(consequents, bank)
        }
        SimpleFofFormula::Equivalence { left, right }
        | SimpleFofFormula::Xor { left, right }
        | SimpleFofFormula::Nand { left, right }
        | SimpleFofFormula::Nor { left, right } => {
            FOF_LOGICAL_SYMBOL_WEIGHT
                + simple_fof_formulas_standard_weight(left, bank)
                + simple_fof_formulas_standard_weight(right, bank)
        }
        SimpleFofFormula::Conjunction(formulas) | SimpleFofFormula::Disjunction(formulas) => {
            simple_fof_formulas_standard_weight(formulas, bank)
        }
        SimpleFofFormula::Negation(formulas) => {
            FOF_LOGICAL_SYMBOL_WEIGHT + simple_fof_formulas_standard_weight(formulas, bank)
        }
        SimpleFofFormula::Universal { bound, formulas }
        | SimpleFofFormula::Existential { bound, formulas } => {
            simple_fof_formulas_standard_weight(formulas, bank)
                + FOF_QUANTIFIER_BINDER_WEIGHT * usize_to_i64(bound.len())
        }
    }
}

fn simple_fof_formulas_order(formulas: &[SimpleFofFormula]) -> i32 {
    formulas
        .iter()
        .map(simple_fof_formula_order)
        .max()
        .unwrap_or(1)
}

fn simple_fof_formula_order(formula: &SimpleFofFormula) -> i32 {
    match formula {
        SimpleFofFormula::Truth(_) | SimpleFofFormula::Literal(_) => 1,
        SimpleFofFormula::Implication {
            antecedents,
            consequents,
        }
        | SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents,
        } => simple_fof_formulas_order(antecedents).max(simple_fof_formulas_order(consequents)),
        SimpleFofFormula::Equivalence { left, right }
        | SimpleFofFormula::Xor { left, right }
        | SimpleFofFormula::Nand { left, right }
        | SimpleFofFormula::Nor { left, right } => {
            simple_fof_formulas_order(left).max(simple_fof_formulas_order(right))
        }
        SimpleFofFormula::Conjunction(formulas)
        | SimpleFofFormula::Disjunction(formulas)
        | SimpleFofFormula::Negation(formulas)
        | SimpleFofFormula::Universal { formulas, .. }
        | SimpleFofFormula::Existential { formulas, .. } => simple_fof_formulas_order(formulas),
    }
}

fn old_tptp_input_formula_clause_type(role: &str) -> FormulaProperties {
    match role {
        "conjecture" => CP_TYPE_CONJECTURE,
        "question" => CP_TYPE_QUESTION,
        "negated_conjecture" => CP_TYPE_NEG_CONJECTURE,
        "hypothesis" => CP_TYPE_HYPOTHESIS,
        _ => CP_TYPE_AXIOM,
    }
}

fn should_annotate_question(
    clause_type: FormulaProperties,
    preprocessing: FormulaPreprocessing,
) -> bool {
    preprocessing.annotate_questions
        && (clause_type == CP_TYPE_QUESTION
            || (clause_type == CP_TYPE_CONJECTURE && preprocessing.conjectures_are_questions))
}

fn simple_fof_annotate_question_formulas(
    formulas: Vec<SimpleFofFormula>,
    add_answer_literals: bool,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    if !add_answer_literals || formulas.len() != 1 {
        return Ok(formulas);
    }

    let mut formulas = formulas;
    let formula = formulas.pop().expect("formula length checked");
    let SimpleFofFormula::Existential { bound, formulas } = formula else {
        return Ok(vec![formula]);
    };
    if bound.is_empty() {
        return Ok(vec![SimpleFofFormula::Existential { bound, formulas }]);
    }

    let (bound, mut formulas) = simple_fof_collect_leading_existentials(bound, formulas);
    let answer = simple_fof_answer_literal_formula(&bound, bank)?;
    formulas.push(answer);
    Ok(vec![SimpleFofFormula::Existential { bound, formulas }])
}

fn simple_fof_collect_leading_existentials(
    mut bound: Vec<Term>,
    formulas: Vec<SimpleFofFormula>,
) -> (Vec<Term>, Vec<SimpleFofFormula>) {
    let mut formulas = formulas;
    while formulas.len() == 1 {
        match formulas.pop().expect("formula length checked") {
            SimpleFofFormula::Existential {
                bound: nested_bound,
                formulas: nested_formulas,
            } => {
                bound.extend(nested_bound);
                formulas = nested_formulas;
            }
            formula => return (bound, vec![formula]),
        }
    }
    (bound, formulas)
}

fn simple_fof_answer_literal_formula(
    variables: &[Term],
    bank: &mut TermBank,
) -> Result<SimpleFofFormula, Diagnostic> {
    let answer_payload = bank.alloc_new_skolem(variables, None)?;
    let answer = Term::top_alloc(bank.signature().answer_code(), 1);
    answer.set_type(Some(bank.signature().type_bank().bool_type()));
    answer.set_argument(0, answer_payload);
    let answer = bank.term_top_insert(answer)?;
    let true_term = bank.true_term().clone();
    let literal = Eqn::alloc(answer, true_term, bank, false)?;
    Ok(SimpleFofFormula::Literal(literal))
}

fn simple_fof_formulas_to_clause_literal_lists(
    formulas: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    let universal_dependencies = simple_fof_global_free_variables(&formulas);
    simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        formulas,
        negate_as_conjecture,
        &universal_dependencies,
        bank,
    )
}

fn simple_fof_formulas_to_clause_literal_lists_with_dependencies(
    formulas: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if negate_as_conjecture && formulas.len() > 1 {
        return simple_fof_negated_conjunction_to_clause_literal_lists(
            formulas,
            universal_dependencies,
            bank,
        );
    }

    let mut literal_lists = Vec::new();
    for formula in formulas {
        literal_lists.extend(
            simple_fof_formula_to_clause_literal_lists_with_dependencies(
                formula,
                negate_as_conjecture,
                universal_dependencies,
                bank,
            )?,
        );
    }
    Ok(literal_lists)
}

fn simple_fof_negated_conjunction_to_clause_literal_lists(
    formulas: Vec<SimpleFofFormula>,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    let mut literal_lists = vec![EqnList::new()];
    for formula in formulas {
        let negated_formula_literal_lists =
            simple_fof_formula_to_clause_literal_lists_with_dependencies(
                formula,
                true,
                universal_dependencies,
                bank,
            )?;
        literal_lists =
            simple_fof_clause_literal_list_products(&literal_lists, &negated_formula_literal_lists);
    }
    Ok(literal_lists)
}

fn simple_fof_clause_literal_list_products(
    prefixes: &[EqnList],
    suffixes: &[EqnList],
) -> Vec<EqnList> {
    let mut distributed = Vec::with_capacity(prefixes.len().saturating_mul(suffixes.len()));
    for prefix in prefixes {
        for suffix in suffixes {
            let mut literals = prefix.clone();
            literals.append(suffix.clone());
            distributed.push(literals);
        }
    }
    distributed
}

fn simple_fof_skolemized_existential_scope_to_clause_literal_lists(
    formulas: Vec<SimpleFofFormula>,
    bound: &[Term],
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if bound.is_empty() {
        return simple_fof_formulas_to_clause_literal_lists_with_dependencies(
            formulas,
            negate_as_conjecture,
            universal_dependencies,
            bank,
        );
    }

    let variables = simple_fof_formula_variables(&formulas);
    let dependencies =
        simple_fof_active_dependencies_for_variables(universal_dependencies, &variables);

    let mut subst = Substitution::new();
    let copy_result = (|| {
        for variable in bound {
            if variable.binding().is_none() && variables.contains_key(&term_identity_id(variable)) {
                let type_ = variable.type_();
                let skolem = bank.alloc_new_skolem(&dependencies, type_.as_ref())?;
                subst.add_binding(variable, &skolem);
            }
        }
        let literal_lists = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
            formulas,
            negate_as_conjecture,
            universal_dependencies,
            bank,
        )?;
        literal_lists
            .iter()
            .map(|literal_list| literal_list.copy_to_bank(bank))
            .collect()
    })();
    subst.backtrack();
    copy_result
}

fn simple_fof_formula_variables(formulas: &[SimpleFofFormula]) -> BTreeMap<usize, Term> {
    let mut variables = BTreeMap::new();
    for formula in formulas {
        simple_fof_formula_collect_variables(formula, &mut variables);
    }
    variables
}

fn simple_fof_global_free_variables(formulas: &[SimpleFofFormula]) -> Vec<Term> {
    let bound = simple_fof_bound_variable_ids(formulas);
    let mut variables = simple_fof_formula_variables(formulas)
        .into_iter()
        .filter(|(id, _)| !bound.contains(id))
        .map(|(_, variable)| variable)
        .collect::<Vec<_>>();
    variables.sort_by_key(|variable| std::cmp::Reverse(variable.f_code()));
    variables
}

fn simple_fof_formula_collect_variables(
    formula: &SimpleFofFormula,
    variables: &mut BTreeMap<usize, Term>,
) {
    match formula {
        SimpleFofFormula::Truth(_) => {}
        SimpleFofFormula::Literal(literal) => {
            literal.collect_variables(variables);
        }
        SimpleFofFormula::Implication {
            antecedents,
            consequents,
        }
        | SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents,
        } => {
            simple_fof_formulas_collect_variables(antecedents, variables);
            simple_fof_formulas_collect_variables(consequents, variables);
        }
        SimpleFofFormula::Equivalence { left, right }
        | SimpleFofFormula::Xor { left, right }
        | SimpleFofFormula::Nand { left, right }
        | SimpleFofFormula::Nor { left, right } => {
            simple_fof_formulas_collect_variables(left, variables);
            simple_fof_formulas_collect_variables(right, variables);
        }
        SimpleFofFormula::Conjunction(formulas)
        | SimpleFofFormula::Disjunction(formulas)
        | SimpleFofFormula::Negation(formulas)
        | SimpleFofFormula::Universal { formulas, .. }
        | SimpleFofFormula::Existential { formulas, .. } => {
            simple_fof_formulas_collect_variables(formulas, variables);
        }
    }
}

fn simple_fof_bound_variable_ids(formulas: &[SimpleFofFormula]) -> BTreeSet<usize> {
    let mut variables = BTreeSet::new();
    for formula in formulas {
        simple_fof_formula_collect_bound_variable_ids(formula, &mut variables);
    }
    variables
}

fn simple_fof_formula_collect_bound_variable_ids(
    formula: &SimpleFofFormula,
    variables: &mut BTreeSet<usize>,
) {
    match formula {
        SimpleFofFormula::Truth(_) | SimpleFofFormula::Literal(_) => {}
        SimpleFofFormula::Implication {
            antecedents,
            consequents,
        }
        | SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents,
        } => {
            simple_fof_formulas_collect_bound_variable_ids(antecedents, variables);
            simple_fof_formulas_collect_bound_variable_ids(consequents, variables);
        }
        SimpleFofFormula::Equivalence { left, right }
        | SimpleFofFormula::Xor { left, right }
        | SimpleFofFormula::Nand { left, right }
        | SimpleFofFormula::Nor { left, right } => {
            simple_fof_formulas_collect_bound_variable_ids(left, variables);
            simple_fof_formulas_collect_bound_variable_ids(right, variables);
        }
        SimpleFofFormula::Conjunction(formulas)
        | SimpleFofFormula::Disjunction(formulas)
        | SimpleFofFormula::Negation(formulas) => {
            simple_fof_formulas_collect_bound_variable_ids(formulas, variables);
        }
        SimpleFofFormula::Universal { bound, formulas }
        | SimpleFofFormula::Existential { bound, formulas } => {
            for variable in bound {
                variables.insert(term_identity_id(variable));
            }
            simple_fof_formulas_collect_bound_variable_ids(formulas, variables);
        }
    }
}

fn simple_fof_formulas_collect_bound_variable_ids(
    formulas: &[SimpleFofFormula],
    variables: &mut BTreeSet<usize>,
) {
    for formula in formulas {
        simple_fof_formula_collect_bound_variable_ids(formula, variables);
    }
}

fn simple_fof_formulas_collect_variables(
    formulas: &[SimpleFofFormula],
    variables: &mut BTreeMap<usize, Term>,
) {
    for formula in formulas {
        simple_fof_formula_collect_variables(formula, variables);
    }
}

fn simple_fof_active_dependencies_for_variables(
    universal_dependencies: &[Term],
    variables: &BTreeMap<usize, Term>,
) -> Vec<Term> {
    let mut dependencies = Vec::new();
    for variable in universal_dependencies {
        if variables.contains_key(&term_identity_id(variable))
            && !dependencies.iter().any(|existing| existing == variable)
        {
            dependencies.push(variable.clone());
        }
    }
    dependencies
}

fn simple_fof_extend_universal_dependencies(
    universal_dependencies: &[Term],
    bound: &[Term],
) -> Vec<Term> {
    let mut dependencies = universal_dependencies.to_vec();
    for variable in bound {
        if !dependencies.iter().any(|existing| existing == variable) {
            dependencies.push(variable.clone());
        }
    }
    dependencies
}

fn simple_fof_truth_to_clause_literal_lists(
    value: bool,
    negate_as_conjecture: bool,
) -> Vec<EqnList> {
    let is_true = if negate_as_conjecture { !value } else { value };
    if is_true {
        Vec::new()
    } else {
        vec![EqnList::new()]
    }
}

fn simple_fof_literal_to_clause_literal_lists(
    literal: Eqn,
    negate_as_conjecture: bool,
) -> Vec<EqnList> {
    let mut literals = EqnList::from_vec(vec![literal]);
    if negate_as_conjecture {
        literals.negate_eqns();
    }
    vec![literals]
}

fn simple_fof_formula_to_clause_literal_lists_with_dependencies(
    formula: SimpleFofFormula,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    match formula {
        SimpleFofFormula::Truth(value) => Ok(simple_fof_truth_to_clause_literal_lists(
            value,
            negate_as_conjecture,
        )),
        SimpleFofFormula::Literal(literal) => Ok(simple_fof_literal_to_clause_literal_lists(
            literal,
            negate_as_conjecture,
        )),
        SimpleFofFormula::Implication {
            antecedents,
            consequents,
        }
        | SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents,
        } => simple_fof_implication_formula_to_clause_literal_lists(
            antecedents,
            consequents,
            negate_as_conjecture,
            universal_dependencies,
            bank,
        ),
        SimpleFofFormula::Equivalence { left, right } => {
            simple_fof_equivalence_formula_to_clause_literal_lists(
                left,
                right,
                negate_as_conjecture,
                universal_dependencies,
                bank,
            )
        }
        SimpleFofFormula::Xor { left, right } => simple_fof_xor_formula_to_clause_literal_lists(
            left,
            right,
            negate_as_conjecture,
            universal_dependencies,
            bank,
        ),
        SimpleFofFormula::Nand { left, right } => simple_fof_nand_formula_to_clause_literal_lists(
            left,
            right,
            negate_as_conjecture,
            universal_dependencies,
            bank,
        ),
        SimpleFofFormula::Nor { left, right } => simple_fof_nor_formula_to_clause_literal_lists(
            left,
            right,
            negate_as_conjecture,
            universal_dependencies,
            bank,
        ),
        SimpleFofFormula::Conjunction(formulas) => {
            simple_fof_conjunction_formula_to_clause_literal_lists(
                formulas,
                negate_as_conjecture,
                universal_dependencies,
                bank,
            )
        }
        SimpleFofFormula::Disjunction(disjuncts) => {
            simple_fof_disjunction_formula_to_clause_literal_lists(
                disjuncts,
                negate_as_conjecture,
                universal_dependencies,
                bank,
            )
        }
        SimpleFofFormula::Negation(formulas) => {
            simple_fof_formulas_to_clause_literal_lists_with_dependencies(
                formulas,
                !negate_as_conjecture,
                universal_dependencies,
                bank,
            )
        }
        SimpleFofFormula::Universal { bound, formulas } => {
            simple_fof_universal_scope_to_clause_literal_lists(
                &bound,
                formulas,
                negate_as_conjecture,
                universal_dependencies,
                bank,
            )
        }
        SimpleFofFormula::Existential { bound, formulas } => {
            simple_fof_existential_scope_to_clause_literal_lists(
                &bound,
                formulas,
                negate_as_conjecture,
                universal_dependencies,
                bank,
            )
        }
    }
}

fn simple_fof_implication_formula_to_clause_literal_lists(
    antecedents: Vec<SimpleFofFormula>,
    consequents: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if negate_as_conjecture {
        simple_fof_negated_implication_to_clause_literal_lists(
            antecedents,
            consequents,
            universal_dependencies,
            bank,
        )
    } else {
        simple_fof_implication_to_clause_literal_lists(
            antecedents,
            consequents,
            universal_dependencies,
            bank,
        )
    }
}

fn simple_fof_equivalence_formula_to_clause_literal_lists(
    left: Vec<SimpleFofFormula>,
    right: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if negate_as_conjecture {
        simple_fof_negated_equivalence_to_clause_literal_lists(
            left,
            right,
            universal_dependencies,
            bank,
        )
    } else {
        simple_fof_equivalence_to_clause_literal_lists(left, right, universal_dependencies, bank)
    }
}

fn simple_fof_xor_formula_to_clause_literal_lists(
    left: Vec<SimpleFofFormula>,
    right: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if negate_as_conjecture {
        simple_fof_equivalence_to_clause_literal_lists(left, right, universal_dependencies, bank)
    } else {
        simple_fof_negated_equivalence_to_clause_literal_lists(
            left,
            right,
            universal_dependencies,
            bank,
        )
    }
}

fn simple_fof_nand_formula_to_clause_literal_lists(
    left: Vec<SimpleFofFormula>,
    right: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    let mut conjuncts = left;
    conjuncts.extend(right);
    simple_fof_conjunction_formula_to_clause_literal_lists(
        conjuncts,
        !negate_as_conjecture,
        universal_dependencies,
        bank,
    )
}

fn simple_fof_nor_formula_to_clause_literal_lists(
    left: Vec<SimpleFofFormula>,
    right: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    let mut disjuncts = simple_fof_formulas_to_disjuncts(left);
    disjuncts.extend(simple_fof_formulas_to_disjuncts(right));
    simple_fof_disjunction_formula_to_clause_literal_lists(
        disjuncts,
        !negate_as_conjecture,
        universal_dependencies,
        bank,
    )
}

fn simple_fof_conjunction_formula_to_clause_literal_lists(
    formulas: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if negate_as_conjecture {
        return simple_fof_negated_conjunction_to_clause_literal_lists(
            formulas,
            universal_dependencies,
            bank,
        );
    }
    simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        formulas,
        negate_as_conjecture,
        universal_dependencies,
        bank,
    )
}

fn simple_fof_disjunction_formula_to_clause_literal_lists(
    disjuncts: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    simple_fof_disjunction_to_clause_literal_lists(
        disjuncts,
        negate_as_conjecture,
        universal_dependencies,
        bank,
    )
}

fn simple_fof_universal_scope_to_clause_literal_lists(
    bound: &[Term],
    formulas: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if negate_as_conjecture {
        simple_fof_skolemized_existential_scope_to_clause_literal_lists(
            formulas,
            bound,
            true,
            universal_dependencies,
            bank,
        )
    } else {
        let universal_dependencies =
            simple_fof_extend_universal_dependencies(universal_dependencies, bound);
        simple_fof_formulas_to_clause_literal_lists_with_dependencies(
            formulas,
            false,
            &universal_dependencies,
            bank,
        )
    }
}

fn simple_fof_existential_scope_to_clause_literal_lists(
    bound: &[Term],
    formulas: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if negate_as_conjecture {
        let universal_dependencies =
            simple_fof_extend_universal_dependencies(universal_dependencies, bound);
        simple_fof_formulas_to_clause_literal_lists_with_dependencies(
            formulas,
            true,
            &universal_dependencies,
            bank,
        )
    } else {
        simple_fof_skolemized_existential_scope_to_clause_literal_lists(
            formulas,
            bound,
            false,
            universal_dependencies,
            bank,
        )
    }
}

fn simple_fof_implication_to_clause_literal_lists(
    antecedents: Vec<SimpleFofFormula>,
    consequents: Vec<SimpleFofFormula>,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    let negated_antecedents = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        antecedents,
        true,
        universal_dependencies,
        bank,
    )?;
    let positive_consequents = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        consequents,
        false,
        universal_dependencies,
        bank,
    )?;
    Ok(simple_fof_clause_literal_list_products(
        &negated_antecedents,
        &positive_consequents,
    ))
}

fn simple_fof_negated_implication_to_clause_literal_lists(
    antecedents: Vec<SimpleFofFormula>,
    consequents: Vec<SimpleFofFormula>,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    let mut literal_lists = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        antecedents,
        false,
        universal_dependencies,
        bank,
    )?;
    literal_lists.extend(
        simple_fof_formulas_to_clause_literal_lists_with_dependencies(
            consequents,
            true,
            universal_dependencies,
            bank,
        )?,
    );
    Ok(literal_lists)
}

fn simple_fof_literal_formulas(literals: Vec<Eqn>) -> Vec<SimpleFofFormula> {
    let mut formulas = Vec::with_capacity(literals.len());
    for literal in literals {
        formulas.push(SimpleFofFormula::Literal(literal));
    }
    formulas
}

fn simple_fof_truth_formula(value: bool) -> Vec<SimpleFofFormula> {
    vec![SimpleFofFormula::Truth(value)]
}

fn simple_fof_equivalence_to_clause_literal_lists(
    left: Vec<SimpleFofFormula>,
    right: Vec<SimpleFofFormula>,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    let negative_left = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        left.clone(),
        true,
        universal_dependencies,
        bank,
    )?;
    let positive_left = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        left,
        false,
        universal_dependencies,
        bank,
    )?;
    let negative_right = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        right.clone(),
        true,
        universal_dependencies,
        bank,
    )?;
    let positive_right = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        right,
        false,
        universal_dependencies,
        bank,
    )?;

    let mut literal_lists =
        simple_fof_clause_literal_list_products(&negative_left, &positive_right);
    literal_lists.extend(simple_fof_clause_literal_list_products(
        &positive_left,
        &negative_right,
    ));
    Ok(literal_lists)
}

fn simple_fof_negated_equivalence_to_clause_literal_lists(
    left: Vec<SimpleFofFormula>,
    right: Vec<SimpleFofFormula>,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    let positive_left = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        left.clone(),
        false,
        universal_dependencies,
        bank,
    )?;
    let negative_left = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        left,
        true,
        universal_dependencies,
        bank,
    )?;
    let positive_right = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        right.clone(),
        false,
        universal_dependencies,
        bank,
    )?;
    let negative_right = simple_fof_formulas_to_clause_literal_lists_with_dependencies(
        right,
        true,
        universal_dependencies,
        bank,
    )?;

    let mut literal_lists =
        simple_fof_clause_literal_list_products(&positive_left, &positive_right);
    literal_lists.extend(simple_fof_clause_literal_list_products(
        &negative_left,
        &negative_right,
    ));
    Ok(literal_lists)
}

fn simple_fof_disjunction_to_clause_literal_lists(
    disjuncts: Vec<SimpleFofFormula>,
    negate_as_conjecture: bool,
    universal_dependencies: &[Term],
    bank: &mut TermBank,
) -> Result<Vec<EqnList>, Diagnostic> {
    if negate_as_conjecture {
        let mut literal_lists = Vec::new();
        for disjunct in disjuncts {
            literal_lists.extend(
                simple_fof_formula_to_clause_literal_lists_with_dependencies(
                    disjunct,
                    true,
                    universal_dependencies,
                    bank,
                )?,
            );
        }
        return Ok(literal_lists);
    }

    let mut literal_lists = vec![EqnList::new()];
    for disjunct in disjuncts {
        let disjunct_literals = simple_fof_formula_to_clause_literal_lists_with_dependencies(
            disjunct,
            false,
            universal_dependencies,
            bank,
        )?;
        literal_lists = simple_fof_clause_literal_list_products(&literal_lists, &disjunct_literals);
    }
    Ok(literal_lists)
}

fn parse_simple_old_tptp_fof_formulas(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    parse_simple_old_tptp_fof_formula(scanner, bank)
}

fn parse_simple_old_tptp_fof_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    let formulas = parse_simple_old_tptp_fof_elementary_formula(scanner, bank)?;
    if scanner.test_tok(TokenType::FOF_LR_IMPL) {
        scanner.accept_tok(TokenType::FOF_LR_IMPL)?;
        let consequents = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Implication {
            antecedents: formulas,
            consequents,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_RL_IMPL) {
        scanner.accept_tok(TokenType::FOF_RL_IMPL)?;
        let antecedents = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents: formulas,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_EQUIV) {
        scanner.accept_tok(TokenType::FOF_EQUIV)?;
        let right = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Equivalence {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::EQUAL_SIGN) {
        scanner.accept_tok(TokenType::EQUAL_SIGN)?;
        let right = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Equivalence {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_XOR) {
        scanner.accept_tok(TokenType::FOF_XOR)?;
        let right = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Xor {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::NEG_EQUAL_SIGN) {
        scanner.accept_tok(TokenType::NEG_EQUAL_SIGN)?;
        let right = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Xor {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_NAND) {
        scanner.accept_tok(TokenType::FOF_NAND)?;
        let right = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Nand {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_NOR) {
        scanner.accept_tok(TokenType::FOF_NOR)?;
        let right = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Nor {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_AND) {
        scanner.accept_tok(TokenType::FOF_AND)?;
        let mut conjuncts = formulas;
        conjuncts.extend(parse_simple_old_tptp_fof_formula(scanner, bank)?);
        return Ok(conjuncts);
    }
    if scanner.test_tok(TokenType::FOF_OR) {
        scanner.accept_tok(TokenType::FOF_OR)?;
        let mut disjuncts = simple_fof_formulas_to_disjuncts(formulas);
        disjuncts.extend(simple_fof_formulas_to_disjuncts(
            parse_simple_old_tptp_fof_formula(scanner, bank)?,
        ));
        return Ok(vec![SimpleFofFormula::Disjunction(disjuncts)]);
    }
    if scanner.test_tok(TokenType::FOF_BIN_OP) {
        return Err(simple_fof_unsupported_error(scanner));
    }
    Ok(formulas)
}

fn parse_simple_old_tptp_fof_elementary_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    if let Some(formulas) = parse_simple_fof_truth_constant(scanner)? {
        return Ok(formulas);
    }
    if scanner.test_id("$distinct") {
        return parse_simple_fof_distinct_formula(scanner, bank);
    }
    if scanner.test_tok(TokenType::UNIV_QUANTOR | TokenType::EXIST_QUANTOR) {
        return parse_simple_old_tptp_fof_quantified_formula(scanner, bank);
    }
    if scanner.test_tok(TokenType::OPEN_BRACKET) {
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
        let formulas = parse_simple_old_tptp_fof_formula(scanner, bank)?;
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
        return Ok(formulas);
    }
    if scanner.test_tok(TokenType::TILDE_SIGN) {
        scanner.accept_tok(TokenType::TILDE_SIGN)?;
        let formulas = parse_simple_old_tptp_fof_elementary_formula(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Negation(formulas)]);
    }
    if let Some(formulas) = parse_simple_fof_fool_term_atom(scanner, bank)? {
        return Ok(formulas);
    }

    let literal = eqn_fof_parse(scanner, bank, ProblemType::FirstOrder)?;
    Ok(simple_fof_literal_formulas(vec![literal]))
}

fn parse_simple_old_tptp_fof_quantified_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    let universal = scanner.test_tok(TokenType::UNIV_QUANTOR);
    if universal {
        scanner.accept_tok(TokenType::UNIV_QUANTOR)?;
    } else {
        scanner.accept_tok(TokenType::EXIST_QUANTOR)?;
    }
    scanner.accept_tok(TokenType::OPEN_SQUARE)?;
    bank.vars().push_env();
    let parsed = (|| {
        let mut bound = Vec::new();
        let name = scanner.current_token().literal();
        scanner.accept_tok(TokenType::NAME)?;
        bound.push(parse_simple_fof_quantified_variable_declaration(
            scanner, bank, name,
        )?);
        while scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            let name = scanner.current_token().literal();
            scanner.accept_tok(TokenType::NAME)?;
            bound.push(parse_simple_fof_quantified_variable_declaration(
                scanner, bank, name,
            )?);
        }
        scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
        scanner.accept_tok(TokenType::COLON)?;
        let formulas = parse_simple_old_tptp_fof_elementary_formula(scanner, bank)?;
        Ok((bound, formulas))
    })();
    let (bound, formulas) = match parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            bank.vars().pop_env();
            return Err(error);
        }
    };

    let formulas = if universal {
        simple_fof_wrap_universal_formulas(bank, bound, formulas)
    } else {
        let bound = simple_fof_resolve_bound_variables(bank, bound);
        vec![SimpleFofFormula::Existential { bound, formulas }]
    };
    bank.vars().pop_env();
    Ok(formulas)
}

fn parse_simple_fof_formulas(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    parse_simple_fof_connective_formulas(scanner, bank)
}

fn parse_simple_fof_connective_formulas(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    let formulas = parse_simple_fof_disjunction_chain(scanner, bank)?;
    if scanner.test_tok(TokenType::FOF_LR_IMPL) {
        scanner.accept_tok(TokenType::FOF_LR_IMPL)?;
        let consequents = parse_simple_fof_implication_operand(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Implication {
            antecedents: formulas,
            consequents,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_RL_IMPL) {
        scanner.accept_tok(TokenType::FOF_RL_IMPL)?;
        let antecedents = parse_simple_fof_implication_operand(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::ReverseImplication {
            antecedents,
            consequents: formulas,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_EQUIV) {
        scanner.accept_tok(TokenType::FOF_EQUIV)?;
        let right = parse_simple_fof_equivalence_operand(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Equivalence {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::EQUAL_SIGN) {
        scanner.accept_tok(TokenType::EQUAL_SIGN)?;
        let right = parse_simple_fof_equivalence_operand(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Equivalence {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_XOR) {
        scanner.accept_tok(TokenType::FOF_XOR)?;
        let right = parse_simple_fof_equivalence_operand(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Xor {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::NEG_EQUAL_SIGN) {
        scanner.accept_tok(TokenType::NEG_EQUAL_SIGN)?;
        let right = parse_simple_fof_equivalence_operand(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Xor {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_NAND) {
        scanner.accept_tok(TokenType::FOF_NAND)?;
        let right = parse_simple_fof_equivalence_operand(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Nand {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_NOR) {
        scanner.accept_tok(TokenType::FOF_NOR)?;
        let right = parse_simple_fof_equivalence_operand(scanner, bank)?;
        return Ok(vec![SimpleFofFormula::Nor {
            left: formulas,
            right,
        }]);
    }
    if scanner.test_tok(TokenType::FOF_BIN_OP) {
        return Err(simple_fof_unsupported_error(scanner));
    }
    Ok(formulas)
}

fn parse_simple_fof_implication_operand(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    parse_simple_fof_connective_formulas(scanner, bank)
}

fn parse_simple_fof_equivalence_operand(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    parse_simple_fof_connective_formulas(scanner, bank)
}

fn parse_simple_fof_disjunction_chain(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    let formulas = parse_simple_fof_conjunction_chain(scanner, bank)?;
    if scanner.test_tok(TokenType::FOF_OR) {
        let mut disjuncts = simple_fof_formulas_to_disjuncts(formulas);
        while scanner.test_tok(TokenType::FOF_OR) {
            scanner.accept_tok(TokenType::FOF_OR)?;
            disjuncts.extend(simple_fof_formulas_to_disjuncts(
                parse_simple_fof_conjunction_chain(scanner, bank)?,
            ));
        }
        return Ok(vec![SimpleFofFormula::Disjunction(disjuncts)]);
    }
    Ok(formulas)
}

fn parse_simple_fof_conjunction_chain(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    let mut formulas = parse_simple_fof_primary_formula(scanner, bank)?;
    while scanner.test_tok(TokenType::FOF_AND) {
        scanner.accept_tok(TokenType::FOF_AND)?;
        formulas.extend(parse_simple_fof_primary_formula(scanner, bank)?);
    }
    Ok(formulas)
}

fn parse_simple_fof_primary_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    if let Some(formulas) = parse_simple_fof_truth_constant(scanner)? {
        return Ok(formulas);
    }
    if scanner.test_id("$distinct") {
        return parse_simple_fof_distinct_formula(scanner, bank);
    }
    if scanner.test_tok(TokenType::EXIST_QUANTOR) {
        return parse_simple_fof_existential_formula(scanner, bank);
    }
    let (universal_bound, universal_scope_count) =
        parse_simple_fof_universal_prefix(scanner, bank)?;
    let formulas = match (|| {
        if scanner.test_tok(TokenType::EXIST_QUANTOR) {
            parse_simple_fof_existential_formula(scanner, bank)
        } else if scanner.test_tok(TokenType::OPEN_BRACKET) {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            let formulas = parse_simple_fof_connective_formulas(scanner, bank)?;
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            Ok(formulas)
        } else if scanner.test_tok(TokenType::TILDE_SIGN) {
            scanner.accept_tok(TokenType::TILDE_SIGN)?;
            let formulas = parse_simple_fof_primary_formula(scanner, bank)?;
            Ok(vec![SimpleFofFormula::Negation(formulas)])
        } else if let Some(formulas) = parse_simple_fof_truth_constant(scanner)? {
            Ok(formulas)
        } else if scanner.test_id("$distinct") {
            parse_simple_fof_distinct_formula(scanner, bank)
        } else if let Some(formulas) = parse_simple_fof_fool_term_atom(scanner, bank)? {
            Ok(formulas)
        } else {
            let literal = eqn_fof_parse(scanner, bank, ProblemType::FirstOrder)?;
            Ok(simple_fof_literal_formulas(vec![literal]))
        }
    })() {
        Ok(formulas) => formulas,
        Err(error) => {
            pop_simple_fof_var_envs(bank, universal_scope_count);
            return Err(error);
        }
    };

    let formulas = simple_fof_wrap_universal_formulas(bank, universal_bound, formulas);
    pop_simple_fof_var_envs(bank, universal_scope_count);
    Ok(formulas)
}

fn simple_fof_formulas_to_disjuncts(formulas: Vec<SimpleFofFormula>) -> Vec<SimpleFofFormula> {
    if formulas.len() == 1 {
        return formulas;
    }
    vec![SimpleFofFormula::Conjunction(formulas)]
}

fn simple_fof_wrap_universal_formulas(
    bank: &TermBank,
    bound: Vec<SimpleFofBoundVariable>,
    formulas: Vec<SimpleFofFormula>,
) -> Vec<SimpleFofFormula> {
    if bound.is_empty() {
        return formulas;
    }

    let unique_bound = simple_fof_resolve_bound_variables(bank, bound);
    if unique_bound.is_empty() {
        formulas
    } else {
        vec![SimpleFofFormula::Universal {
            bound: unique_bound,
            formulas,
        }]
    }
}

fn parse_simple_fof_universal_prefix(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<(Vec<SimpleFofBoundVariable>, usize), Diagnostic> {
    let mut bound = Vec::new();
    let mut scope_count = 0;
    let parsed = (|| {
        while scanner.test_tok(TokenType::UNIV_QUANTOR) {
            scanner.accept_tok(TokenType::UNIV_QUANTOR)?;
            scanner.accept_tok(TokenType::OPEN_SQUARE)?;
            bank.vars().push_env();
            scope_count += 1;
            let name = scanner.current_token().literal();
            scanner.accept_tok(TokenType::NAME)?;
            bound.push(parse_simple_fof_quantified_variable_declaration(
                scanner, bank, name,
            )?);
            while scanner.test_tok(TokenType::COMMA) {
                scanner.accept_tok(TokenType::COMMA)?;
                let name = scanner.current_token().literal();
                scanner.accept_tok(TokenType::NAME)?;
                bound.push(parse_simple_fof_quantified_variable_declaration(
                    scanner, bank, name,
                )?);
            }
            scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
            scanner.accept_tok(TokenType::COLON)?;
        }
        Ok(())
    })();
    if let Err(error) = parsed {
        pop_simple_fof_var_envs(bank, scope_count);
        return Err(error);
    }
    Ok((bound, scope_count))
}

fn parse_simple_fof_existential_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    scanner.accept_tok(TokenType::EXIST_QUANTOR)?;
    scanner.accept_tok(TokenType::OPEN_SQUARE)?;
    bank.vars().push_env();
    let parsed = (|| {
        let mut bound = Vec::new();
        let name = scanner.current_token().literal();
        scanner.accept_tok(TokenType::NAME)?;
        bound.push(parse_simple_fof_quantified_variable_declaration(
            scanner, bank, name,
        )?);
        while scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            let name = scanner.current_token().literal();
            scanner.accept_tok(TokenType::NAME)?;
            bound.push(parse_simple_fof_quantified_variable_declaration(
                scanner, bank, name,
            )?);
        }
        scanner.accept_tok(TokenType::CLOSE_SQUARE)?;
        scanner.accept_tok(TokenType::COLON)?;

        let formulas = if scanner.test_tok(TokenType::OPEN_BRACKET) {
            scanner.accept_tok(TokenType::OPEN_BRACKET)?;
            let formulas = parse_simple_fof_connective_formulas(scanner, bank)?;
            scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
            formulas
        } else if scanner
            .test_tok(TokenType::UNIV_QUANTOR | TokenType::EXIST_QUANTOR | TokenType::TILDE_SIGN)
        {
            parse_simple_fof_primary_formula(scanner, bank)?
        } else if let Some(formulas) = parse_simple_fof_truth_constant(scanner)? {
            formulas
        } else if scanner.test_id("$distinct") {
            parse_simple_fof_distinct_formula(scanner, bank)?
        } else if let Some(formulas) = parse_simple_fof_fool_term_atom(scanner, bank)? {
            formulas
        } else {
            if scanner.test_tok(TokenType::FOF_BIN_OP | TokenType::OPEN_BRACKET) {
                return Err(simple_fof_unsupported_error(scanner));
            }

            let literal = eqn_fof_parse(scanner, bank, ProblemType::FirstOrder)?;
            simple_fof_literal_formulas(vec![literal])
        };
        Ok((bound, formulas))
    })();
    let (bound, formulas) = match parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            bank.vars().pop_env();
            return Err(error);
        }
    };

    let bound = simple_fof_resolve_bound_variables(bank, bound);
    bank.vars().pop_env();
    Ok(vec![SimpleFofFormula::Existential { bound, formulas }])
}

fn parse_simple_fof_quantified_variable_declaration(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    name: String,
) -> Result<SimpleFofBoundVariable, Diagnostic> {
    if scanner.test_tok(TokenType::COLON) {
        scanner.accept_tok(TokenType::COLON)?;
        let type_ = bank
            .signature_mut()
            .type_bank_mut()
            .parse_type(scanner, ProblemType::FirstOrder)?;
        let variable = bank.vars().ext_name_declare_alloc_sort(&name, &type_);
        return Ok(SimpleFofBoundVariable {
            name,
            variable: Some(variable),
        });
    }
    let variable = bank.vars().ext_name_declare_alloc(&name);
    Ok(SimpleFofBoundVariable {
        name,
        variable: Some(variable),
    })
}

fn pop_simple_fof_var_envs(bank: &TermBank, count: usize) {
    for _ in 0..count {
        bank.vars().pop_env();
    }
}

fn simple_fof_resolve_bound_variables(
    bank: &TermBank,
    bound: Vec<SimpleFofBoundVariable>,
) -> Vec<Term> {
    let mut variables = Vec::new();
    for declaration in bound {
        let variable = declaration
            .variable
            .or_else(|| bank.vars().ext_name_find(&declaration.name));
        if let Some(variable) = variable {
            if !variables.iter().any(|existing| existing == &variable) {
                variables.push(variable);
            }
        }
    }
    variables
}

fn parse_simple_fof_truth_constant(
    scanner: &mut Scanner,
) -> Result<Option<Vec<SimpleFofFormula>>, Diagnostic> {
    if scanner.test_id("$true") {
        scanner.accept_id("$true")?;
        return Ok(Some(simple_fof_truth_formula(true)));
    }
    if scanner.test_id("$false") {
        scanner.accept_id("$false")?;
        return Ok(Some(simple_fof_truth_formula(false)));
    }
    Ok(None)
}

fn parse_simple_fof_fool_term_atom(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Option<Vec<SimpleFofFormula>>, Diagnostic> {
    if !scanner.test_tok(TokenType::ITE_TOKEN | TokenType::LET_TOKEN) {
        return Ok(None);
    }

    let term = bank.parse_term_with_distinct_checks(scanner)?;
    if !term.type_().as_ref().is_some_and(Type::is_bool) {
        return parse_simple_fof_non_boolean_fool_literal(scanner, bank, term).map(Some);
    }
    let literal = Eqn::alloc(term, bank.true_term().clone(), bank, true)?;
    Ok(Some(simple_fof_literal_formulas(vec![literal])))
}

fn parse_simple_fof_non_boolean_fool_literal(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    left: Term,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    if !scanner.test_tok(TokenType::EQUAL_SIGN | TokenType::NEG_EQUAL_SIGN) {
        return Err(Diagnostic::new(
            ErrorCode::TYPE_ERROR,
            "FOOL formula atom must have Boolean type",
        ));
    }
    let positive = scanner.test_tok(TokenType::EQUAL_SIGN);
    scanner.accept_tok(TokenType::EQUAL_SIGN | TokenType::NEG_EQUAL_SIGN)?;
    let right = bank.parse_term_with_distinct_checks(scanner)?;
    let literal = Eqn::alloc(left, right, bank, positive)?;
    Ok(simple_fof_literal_formulas(vec![literal]))
}

fn parse_simple_fof_distinct_formula(
    scanner: &mut Scanner,
    bank: &mut TermBank,
) -> Result<Vec<SimpleFofFormula>, Diagnostic> {
    scanner.accept_id("$distinct")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    let mut args = Vec::new();
    args.push(parse_simple_fof_distinct_constant(scanner, bank, None)?);
    let expected_type = args[0].type_();
    while scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        args.push(parse_simple_fof_distinct_constant(
            scanner,
            bank,
            expected_type.as_ref(),
        )?);
    }
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;

    if args.len() < 2 {
        return Ok(simple_fof_truth_formula(true));
    }

    let mut disequalities = Vec::new();
    for left_index in 0..args.len() {
        for right_index in (left_index + 1)..args.len() {
            disequalities.push(Eqn::alloc(
                args[left_index].clone(),
                args[right_index].clone(),
                bank,
                false,
            )?);
        }
    }
    Ok(simple_fof_literal_formulas(disequalities))
}

fn parse_simple_fof_distinct_constant(
    scanner: &mut Scanner,
    bank: &mut TermBank,
    expected_type: Option<&Type>,
) -> Result<Term, Diagnostic> {
    let source = String::from_utf8_lossy(scanner.current_token().source_bytes()).into_owned();
    let line = scanner.current_token().line();
    let column = scanner.current_token().column();
    let term = bank.parse_term_with_distinct_checks(scanner)?;
    if term.is_free_var() || term.arity() != 0 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!("{source}:{line}:{column}: constant expected in $distinct argument list"),
        ));
    }
    if expected_type.is_some_and(|type_| term.type_().as_ref() != Some(type_)) {
        return Err(Diagnostic::new(
            ErrorCode::TYPE_ERROR,
            format!(
                "{source}:{line}:{column}: All $distinct arguments have to be constants of the same type"
            ),
        ));
    }
    Ok(term)
}

fn parse_simple_tstp_optional_source(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    if scanner.test_tok(TokenType::COMMA) {
        scanner.accept_tok(TokenType::COMMA)?;
        tstp_skip_source(scanner)?;
        if scanner.test_tok(TokenType::COMMA) {
            scanner.accept_tok(TokenType::COMMA)?;
            scanner.check_tok(TokenType::OPEN_SQUARE)?;
            parse_skip_parenthesized_expr(scanner)?;
        }
    }
    Ok(())
}

fn tstp_skip_source(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    if scanner.test_tok(TokenType::OPEN_SQUARE) {
        parse_skip_parenthesized_expr(scanner)
    } else {
        scanner.accept_tok(TokenType::IDENTIFIER | TokenType::POS_INT)?;
        if scanner.test_tok(TokenType::OPEN_BRACKET) {
            parse_skip_parenthesized_expr(scanner)?;
        }
        Ok(())
    }
}

fn simple_fof_unsupported_error(scanner: &Scanner) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): FOF formula requires full clausification; this port currently supports only atomic formulas, top-level and existential-body Boolean $ite/$let atoms, $true/$false truth constants, atomic existential formulas, supported parenthesized existential bodies including quantified operands in positive or negative polarity, universally quantified fragments with supported existential scopes, universally quantified implications, equivalences, formula-level equality, XORs, formula-level disequality, NANDs, and NORs with supported existential operands, grouped or unparenthesized non-conjecture conjunctions/disjunctions including supported existential conjuncts or disjuncts, conjunctive disjuncts with supported existential conjuncts, and grouped or unparenthesized conjecture conjunctions/disjunctions of supported fragments",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal()
        ),
    )
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn apply_auto_parse_output_side_effects(config: &mut EProverConfig, detected_format: IoFormat) {
    if config.parse_format == IoFormat::Auto && detected_format == IoFormat::Tstp {
        config.output_format = IoFormat::Tstp;
        if config.doc_output_format == DocOutputFormat::NoFormat {
            config.doc_output_format = DocOutputFormat::Tstp;
        }
    }
    if config.doc_output_format == DocOutputFormat::NoFormat {
        config.doc_output_format = DocOutputFormat::Pcl;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        auto_memory_limit_from_system_mb, core_limit_failure_messages, cpu_rlimit_to_apply,
        fv_index_params_from_config, heuristic_parms_from_config, native_hard_cpu_limit_to_apply,
        order_parms_from_config, preprocessing_config_debug_line, process_options,
        proof_control_from_config, resource_limit_warning_from_outcome,
        resource_limit_warning_from_result, rlimit_warning_from_result, run, run_config,
        temporary_executable_term_bank, write_resource_setup_messages,
        write_saturation_proof_object_clause, write_stopped_proof_output, AcHandling,
        DocOutputFormat, EProverAction, EProverConfig, EProverFlag, EtaNormalization,
        ExtInferenceType, FoolUnroll, FvIndexFeatureType, GroundingStrategy, LiteralComparison,
        ParamodulationType, PredicateEliminationFlag, PrimEnumMode, TermOrdering, UnificationMode,
        WatchlistSource, LPO_RECURSION_LIMIT_WARNING, MEGA, THF_REQUIRES_HOL_MESSAGE,
        TSTP_FORMULA_FREE_VARIABLES_MESSAGE,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::os_wrapper::{resource_limit_error_message, RLimResult, RLimitOutcome};
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::basics::simple_stuff::ProblemType;
    use crate::basics::verbose::{set_verbose_level, verbose_level};
    use crate::clauses::clause::{clause_parse, Clause};
    use crate::clauses::clause_props::CP_TYPE_AXIOM;
    use crate::clauses::clauseinfo::ClauseInfo;
    use crate::clauses::derivation::{clause_push_derivation, DC_EQ_RES};
    use crate::clauses::freqvectors::FvIndexType;
    use crate::clauses::proofstate::proof_state_alloc;
    use crate::heuristics::{hcb as hcb_params, to_params};
    use crate::inout::output::{output_level, set_output_level};
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::inout::signals::{
        configure_time_limits, hard_time_limit, schedule_time_limit, set_time_is_up,
        soft_time_limit, time_is_up, RLIM_INFINITY_COMPAT,
    };
    use crate::orderings::cto_lpo::{lpo_recursion_depth_limit, set_lpo_recursion_depth_limit};
    use crate::prover::version::VERSION;
    use crate::terms::signature::{
        FP_IGNORE_PROPS, FP_IS_FLOAT, FP_IS_INTEGER, FP_IS_OBJECT, FP_IS_RATIONAL,
    };
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::RewriteLevel;
    use crate::test_support::global_state_lock;
    use std::ffi::OsString;
    use std::fmt::Write as _;

    struct EnvGuard {
        name: &'static str,
        previous: Option<OsString>,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("eprover-{name}-{}.out", std::process::id()))
    }

    fn set_env_var(name: &'static str, value: &str) -> EnvGuard {
        let previous = std::env::var_os(name);
        std::env::set_var(name, value);
        EnvGuard { name, previous }
    }

    fn parse_lop_test_clause(bank: &mut TermBank, input: &str, ident: i64) -> Clause {
        let mut scanner = Scanner::from_user_string(input, false).unwrap();
        scanner.set_format(IoFormat::Lop);
        let mut clause = clause_parse(&mut scanner, bank, ProblemType::FirstOrder).unwrap();
        clause.set_ident(ident);
        clause
    }

    fn default_preprocessing_debug_line() -> String {
        preprocessing_config_debug_line(&EProverConfig::default())
    }

    fn default_proof_search_prefix() -> String {
        format!(
            "{}% Scanning for AC axioms\n",
            default_preprocessing_debug_line()
        )
    }

    fn run_config_from<I, S>(argv: I) -> Box<EProverConfig>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let EProverAction::Run(config) = process_options(argv).unwrap() else {
            panic!("expected run config");
        };
        config
    }

    #[test]
    fn preprocessing_config_debug_line_preserves_c_shape() {
        assert_eq!(
            default_preprocessing_debug_line(),
            "% (lift_lambdas = 1, lambda_to_forall = 1,unroll_only_formulas = 1, sine = (null))\n"
        );

        let config = run_config_from([
            "eprover",
            "--lift-lambdas=false",
            "--cnf-lambda-to-forall=false",
            "--unroll-formulas-only=false",
            "--sine=Auto",
        ]);

        assert_eq!(
            preprocessing_config_debug_line(&config),
            "% (lift_lambdas = 0, lambda_to_forall = 0,unroll_only_formulas = 0, sine = Auto)\n"
        );
    }

    #[test]
    fn process_options_recognizes_version_action() {
        let action = process_options(["eprover", "--version"]).unwrap();
        assert_eq!(action, EProverAction::Version);
    }

    #[test]
    fn process_options_keeps_non_option_files_and_inserts_stdin_default() {
        let action = process_options(["eprover", "a.p", "--silent", "b.p"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_level, 0);
        assert_eq!(config.files, ["a.p", "b.p"]);

        let action = process_options(["eprover", "--silent"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.files, ["-"]);
    }

    #[test]
    fn process_options_tracks_cpu_limit_schedule_state_like_c() {
        let action = process_options(["eprover", "--cpu-limit"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.cpu_limit, Some(300));
        assert_eq!(config.soft_cpu_limit, None);
        assert_eq!(config.schedule_time_limit, Some(300));
        assert_eq!(cpu_rlimit_to_apply(&config), Some(300));
        assert_eq!(native_hard_cpu_limit_to_apply(&config), Some(300));

        let action =
            process_options(["eprover", "--soft-cpu-limit=25", "--cpu-limit=100"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.cpu_limit, Some(100));
        assert_eq!(config.soft_cpu_limit, Some(25));
        assert_eq!(config.schedule_time_limit, Some(100));
        assert_eq!(cpu_rlimit_to_apply(&config), Some(25));
        assert_eq!(native_hard_cpu_limit_to_apply(&config), Some(100));

        let action =
            process_options(["eprover", "--cpu-limit=100", "--soft-cpu-limit=25"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.schedule_time_limit, Some(25));
        assert_eq!(cpu_rlimit_to_apply(&config), Some(25));
        assert_eq!(native_hard_cpu_limit_to_apply(&config), Some(100));

        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(cpu_rlimit_to_apply(&config), None);
        assert_eq!(native_hard_cpu_limit_to_apply(&config), None);

        let action = process_options(["eprover", "--cpu-limit=-1"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(native_hard_cpu_limit_to_apply(&config), None);
    }

    #[test]
    fn resource_limit_warning_text_matches_c_shapes() {
        let failed =
            resource_limit_warning_from_result(RLimResult::Failed, 100, "RLIMIT_CPU (E-Hard)")
                .unwrap();
        assert_eq!(failed.code(), ErrorCode::SYSTEM_ERROR);
        assert_eq!(
            failed.message(),
            "Could not set limit RLIMIT_CPU (E-Hard) to 100"
        );

        let errno_failed = resource_limit_warning_from_outcome(
            RLimitOutcome::new(RLimResult::Failed, Some(22)),
            100,
            "RLIMIT_CPU (E-Hard)",
        )
        .unwrap();
        assert_eq!(
            errno_failed.message(),
            format!(
                "Could not set limit RLIMIT_CPU (E-Hard) to 100 ({})",
                resource_limit_error_message(22)
            )
        );

        let reduced =
            resource_limit_warning_from_result(RLimResult::Reduced, 100, "RLIMIT_CPU (E-Soft)")
                .unwrap();
        assert_eq!(reduced.code(), ErrorCode::SYSTEM_ERROR);
        assert_eq!(reduced.message(), "Had to reduce limit RLIMIT_CPU (E-Soft)");

        assert_eq!(
            resource_limit_warning_from_result(RLimResult::Success, 100, "RLIMIT_CPU"),
            None
        );
    }

    #[test]
    fn core_limit_failure_renders_c_perror_before_warning() {
        let os_error = 22;
        let messages =
            core_limit_failure_messages(RLimitOutcome::new(RLimResult::Failed, Some(os_error)));
        let mut stderr = Vec::new();

        write_resource_setup_messages(&mut stderr, &messages).unwrap();

        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            format!(
                "eprover: {}\neprover: Warning: Cannot prevent core dumps!\n",
                resource_limit_error_message(os_error)
            )
        );
    }

    #[test]
    fn core_limit_success_has_no_setup_messages() {
        assert!(
            core_limit_failure_messages(RLimitOutcome::new(RLimResult::Success, None)).is_empty()
        );
    }

    #[test]
    fn linux_rlimit_data_failure_warning_is_masked_like_c() {
        #[cfg(target_os = "linux")]
        let data_resource = super::RLIMIT_DATA_COMPAT;
        #[cfg(not(target_os = "linux"))]
        let data_resource = 2;

        assert_eq!(
            rlimit_warning_from_result(RLimResult::Failed, data_resource, 100, "RLIMIT_DATA",),
            None
        );
        assert!(rlimit_warning_from_result(
            RLimResult::Failed,
            data_resource + 1,
            100,
            "RLIMIT_CPU",
        )
        .is_some());
    }

    #[test]
    fn process_options_records_memory_limit_state_like_c() {
        let action = process_options(["eprover", "-m", "128"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.memory_limit, 128 * MEGA);
        assert_eq!(config.delete_bad_limit, i64::MAX);

        let action = process_options(["eprover", "--memory-limit=-1"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.memory_limit, u64::MAX.wrapping_mul(MEGA));
    }

    #[test]
    fn auto_memory_limit_derives_search_limits_like_c() {
        let (memory_limit, delete_bad_limit) = auto_memory_limit_from_system_mb(1_000).unwrap();

        assert_eq!(memory_limit, 800 * MEGA);
        assert_eq!(delete_bad_limit, 585_734_553);

        let error = auto_memory_limit_from_system_mb(-1).unwrap_err();
        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            error.message(),
            "Cannot find physical memory automatically. Give explicit value to --memory-limit"
        );
    }

    #[test]
    fn process_options_records_input_mode_flags_like_c() {
        let action = process_options(["eprover", "--print-formulas"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::SyntaxOnly));
        assert!(config.flags.contains(EProverFlag::PrintFormulas));

        let action = process_options(["eprover", "--prune"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_level, 4);
        assert!(config.flags.contains(EProverFlag::PruneOnly));

        let action = process_options(["eprover", "--cnf", "--error-on-empty"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::CnfOnly));
        assert!(config.flags.contains(EProverFlag::PrintSaturated));
        assert!(config.flags.contains(EProverFlag::RequireNonempty));
        assert_eq!(config.saturated_output_descriptor, "teigEIG");
        assert_eq!(config.processed_set_limit, 0);
        assert_eq!(config.step_limit, i64::MAX);
    }

    #[test]
    fn process_options_records_reporting_descriptors_like_c() {
        let action = process_options([
            "eprover",
            "--print-statistics",
            "--print-detailed-statistics",
            "-S",
            "--filter-saturated=eig",
            "--print-sat-info",
            "-R",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::PrintStatistics));
        assert!(config.flags.contains(EProverFlag::PrintDetailedStatistics));
        assert!(config.flags.contains(EProverFlag::PrintSaturated));
        assert!(config.flags.contains(EProverFlag::FilterSaturated));
        assert!(config.flags.contains(EProverFlag::PrintSaturatedInfo));
        assert!(config.flags.contains(EProverFlag::ResourceInfo));
        assert_eq!(config.saturated_output_descriptor, "eigEIG");
        assert_eq!(config.filter_saturated_descriptor, "eig");

        let action =
            process_options(["eprover", "--print-saturated=teA", "--filter-saturated=eig"])
                .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.saturated_output_descriptor, "teA");
        assert_eq!(config.filter_saturated_descriptor, "eig");
    }

    #[test]
    fn process_options_rejects_invalid_reporting_descriptors() {
        let error = process_options(["eprover", "--print-saturated=tx"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Illegal argument to option -S (--print-saturated)"
        );

        let error = process_options(["eprover", "--filter-saturated"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Illegal argument to option --filter-saturated"
        );

        let error = process_options(["eprover", "--filter-saturated=Fx"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Illegal argument to option --filter-saturated"
        );
    }

    #[test]
    fn process_options_records_strategy_and_limit_state_like_c() {
        let action = process_options([
            "eprover",
            "--select-strategy=AutoSched",
            "--print-strategy",
            "--parse-strategy=strategy.txt",
            "-C",
            "10",
            "-P",
            "20",
            "-U",
            "30",
            "-T",
            "40",
            "--generated-limit=50",
            "--tb-insert-limit=60",
            "--answers",
            "--conjectures-are-questions",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.select_strategy.as_deref(), Some("AutoSched"));
        assert_eq!(config.print_strategy.as_deref(), Some(">current-strategy<"));
        assert_eq!(config.parse_strategy_file.as_deref(), Some("strategy.txt"));
        assert_eq!(config.step_limit, 10);
        assert_eq!(config.processed_set_limit, 20);
        assert_eq!(config.unprocessed_limit, 30);
        assert_eq!(config.total_clause_set_limit, 40);
        assert_eq!(config.generated_limit, 50);
        assert_eq!(config.term_bank_insert_limit, 60);
        assert_eq!(config.answer_limit, 2_147_483_647);
        assert!(config.flags.contains(EProverFlag::ConjecturesAreQuestions));

        let action = process_options(["eprover", "--answers=7", "--print-strategy=Named"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.answer_limit, 7);
        assert_eq!(config.print_strategy.as_deref(), Some("Named"));
    }

    #[test]
    fn process_options_records_format_state_like_c() {
        let action = process_options([
            "eprover",
            "--lop-in",
            "--pcl-out",
            "--pcl-terms-compressed",
            "--pcl-compact",
            "--pcl-shell-level",
            "--print-oriented-eqlits-as-rules",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.parse_format, IoFormat::Lop);
        assert_eq!(config.output_format, IoFormat::Lop);
        assert_eq!(config.doc_output_format, DocOutputFormat::Pcl);
        assert!(!config.pcl_output.full_terms);
        assert!(config.pcl_output.compact);
        assert_eq!(config.pcl_output.shell_level, 1);
        assert!(config.equation_print.print_oriented);

        let action = process_options(["eprover", "--pcl-shell-level=2"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.pcl_output.shell_level, 2);
    }

    #[test]
    fn process_options_records_tptp_and_tstp_format_side_effects_like_c() {
        let action = process_options(["eprover", "--full-equational-rep", "--tptp-out"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_format, IoFormat::Tptp);
        assert!(!config.equation_print.full_equational_rep);
        assert!(!config.equation_print.use_infix);

        let action = process_options(["eprover", "--tptp2-format"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.parse_format, IoFormat::Tptp);
        assert_eq!(config.output_format, IoFormat::Tptp);
        assert!(!config.equation_print.use_infix);

        let action = process_options([
            "eprover",
            "--eqn-no-infix",
            "--full-equational-rep",
            "--tstp-out",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_format, IoFormat::Tstp);
        assert_eq!(config.doc_output_format, DocOutputFormat::Tstp);
        assert!(config.equation_print.use_infix);
        assert!(config.equation_print.full_equational_rep);

        let action = process_options(["eprover", "--tptp3-format"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.parse_format, IoFormat::Tstp);
        assert_eq!(config.output_format, IoFormat::Tstp);
        assert_eq!(config.doc_output_format, DocOutputFormat::Tstp);
    }

    #[test]
    fn process_options_records_auto_schedule_state_like_c() {
        let action = process_options(["eprover", "--auto"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::Auto));
        assert_eq!(config.sine.as_deref(), Some("Auto"));
        assert!(!config.strategy_scheduling);

        let action = process_options([
            "eprover",
            "--auto-schedule=Auto",
            "--serialize-schedule=true",
            "--force-preproc-sched=false",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.strategy_scheduling);
        assert_eq!(config.schedule_cores, -1);
        assert_eq!(config.sine.as_deref(), Some("Auto"));
        assert!(config.serialize_schedule);
        assert!(!config.force_preprocessing_schedule);

        let action =
            process_options(["eprover", "--satauto-schedule", "--auto-schedule=4"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.strategy_scheduling);
        assert_eq!(config.schedule_cores, 1);
        assert_eq!(config.sine, None);

        let action =
            process_options(["eprover", "--auto-schedule=4", "--satauto-schedule=8"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.schedule_cores, 4);
        assert_eq!(config.sine.as_deref(), Some("Auto"));
    }

    #[test]
    fn process_options_rejects_invalid_schedule_bool_args() {
        let error = process_options(["eprover", "--serialize-schedule=yes"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--force-preproc-sched=maybe"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_records_preprocessing_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.preprocessing.no_preprocessing);
        assert_eq!(config.preprocessing.eqdef_incrlimit, 20);
        assert_eq!(config.preprocessing.eqdef_maxclauses, 20_000);
        assert_eq!(config.preprocessing.ac_handling, AcHandling::DiscardAll);
        assert!(config.preprocessing.ac_res_aggressive);
        assert!(!config.preprocessing.presat_interreduction);
        assert_eq!(config.preprocessing.relevance_prune_level, 0);

        let action = process_options([
            "eprover",
            "--no-preprocessing",
            "--eq-unfold-limit=7",
            "--eq-unfold-maxclauses=11",
            "--goal-defs",
            "--goal-subterm-defs",
            "--sine",
            "--rel-pruning-level",
            "--presat-simplify=false",
            "--ac-handling=KeepOrientable",
            "--ac-non-aggressive",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.preprocessing.no_preprocessing);
        assert_eq!(config.preprocessing.eqdef_incrlimit, 7);
        assert_eq!(config.preprocessing.eqdef_maxclauses, 11);
        assert!(config.preprocessing.goal_definitions.positive);
        assert!(config.preprocessing.goal_definitions.negative);
        assert!(config.preprocessing.goal_definitions.subterms);
        assert_eq!(config.sine.as_deref(), Some("Auto"));
        assert_eq!(config.preprocessing.relevance_prune_level, 3);
        assert!(!config.preprocessing.presat_interreduction);
        assert_eq!(config.preprocessing.ac_handling, AcHandling::KeepOrientable);
        assert!(!config.preprocessing.ac_res_aggressive);

        let action = process_options(["eprover", "--no-eq-unfolding", "--ac-handling"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.preprocessing.eqdef_incrlimit, i64::MIN);
        assert_eq!(config.preprocessing.ac_handling, AcHandling::KeepUnits);
    }

    #[test]
    fn process_options_records_definition_and_cnf_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config
            .search
            .heuristic
            .weight_function_definitions
            .is_empty());
        assert!(config.search.heuristic.heuristic_definitions.is_empty());
        assert_eq!(config.free_symbol_properties, FP_IGNORE_PROPS);
        assert_eq!(config.preprocessing.formula_def_limit, 24);
        assert_eq!(config.preprocessing.miniscope_limit, 1_048_576);
        assert_eq!(config.preprocessing.fool_unroll, FoolUnroll::Enabled);
        assert!(!config.encoding.print_types);
        assert!(!config.encoding.app_encode);
        assert_eq!(
            config.search.inference.higher_order.arg_cong,
            ExtInferenceType::AllLits
        );
        assert_eq!(
            config.search.inference.higher_order.neg_ext,
            ExtInferenceType::NoLits
        );
        assert_eq!(
            config.search.inference.higher_order.pos_ext,
            ExtInferenceType::NoLits
        );

        let action = process_options([
            "eprover",
            "-D",
            "wf1",
            "--define-weight-function=wf2",
            "-H",
            "h1",
            "--define-heuristic=h2",
            "--free-numbers",
            "--free-objects",
            "--definitional-cnf",
            "--miniscope-limit",
            "--fool-unroll=false",
            "--print-types",
            "--app-encode",
            "--arg-cong=max",
            "--neg-ext=all",
            "--pos-ext=off",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.heuristic.weight_function_definitions,
            ["wf1", "wf2"]
        );
        assert_eq!(config.search.heuristic.heuristic_definitions, ["h1", "h2"]);
        assert!(config
            .free_symbol_properties
            .contains_all(FP_IS_INTEGER | FP_IS_RATIONAL | FP_IS_FLOAT | FP_IS_OBJECT));
        assert_eq!(config.preprocessing.formula_def_limit, 24);
        assert_eq!(config.preprocessing.miniscope_limit, 2_147_483_648);
        assert_eq!(config.preprocessing.fool_unroll, FoolUnroll::Disabled);
        assert!(config.encoding.print_types);
        assert!(config.encoding.app_encode);
        assert_eq!(
            config.search.inference.higher_order.arg_cong,
            ExtInferenceType::MaxLits
        );
        assert_eq!(
            config.search.inference.higher_order.neg_ext,
            ExtInferenceType::AllLits
        );
        assert_eq!(
            config.search.inference.higher_order.pos_ext,
            ExtInferenceType::NoLits
        );

        let action =
            process_options(["eprover", "--definitional-cnf=0", "--miniscope-limit=7"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.preprocessing.formula_def_limit, 0);
        assert_eq!(config.preprocessing.miniscope_limit, 7);
    }

    #[test]
    fn process_options_records_ho_unification_defaults_like_c() {
        let config = run_config_from(["eprover"]);
        let preprocessing = &config.preprocessing;
        let pred_elim = &preprocessing.predicate_elimination;
        let inference = &config.search.inference;
        let ho_preprocessing = &inference.higher_order_preprocessing;
        let ho_search = &inference.higher_order_search;
        let unification = inference.higher_order_unification;

        assert_eq!(preprocessing.classification_timeout_percentage, 2);
        assert_eq!(
            (preprocessing.bce.enabled, preprocessing.bce.max_occs),
            (false, 512)
        );
        assert_eq!(
            (pred_elim.enabled, pred_elim.max_occs, pred_elim.tolerance),
            (false, 512, 0)
        );
        assert!(!pred_elim
            .flags
            .contains(PredicateEliminationFlag::RecognizeGates));
        assert!(!pred_elim
            .flags
            .contains(PredicateEliminationFlag::ForceMuDecrease));
        assert!(!pred_elim
            .flags
            .contains(PredicateEliminationFlag::IgnoreConjectureSymbols));
        assert_eq!(inference.higher_order.ext_rules_max_depth, -1);
        assert!(!inference.higher_order.inverse_recognition);
        assert!(!inference.higher_order.replace_inj_defs);
        assert_eq!(config.search.ordering.ho_order_kind, HoOrderKind::LfhoOrder);
        assert_eq!(ho_preprocessing.eta_normalization, EtaNormalization::Reduce);
        assert!(ho_preprocessing.lambda_to_forall);
        assert!(ho_preprocessing.unroll_only_formulas);
        assert_eq!(ho_preprocessing.elim_leibniz_max_depth, -1);
        assert_eq!(
            ho_search.primitive_enumeration.mode,
            PrimEnumMode::Pragmatic
        );
        assert_eq!(ho_search.primitive_enumeration.max_depth, -1);
        assert_eq!(ho_preprocessing.inst_choice_max_depth, -1);
        assert!(!ho_search.local_rw);
        assert!(!ho_search.prune_args);
        assert!(!ho_preprocessing.preinstantiate_induction);
        assert_eq!(unification.func_proj_limit, 0);
        assert_eq!(unification.imit_limit, 0);
        assert_eq!(unification.ident_limit, 0);
        assert_eq!(unification.elim_limit, 0);
        assert_eq!(unification.mode, UnificationMode::Single);
        assert!(unification.pattern_oracle);
        assert!(unification.fixpoint_oracle);
        assert_eq!(unification.max_unifiers, 4);
        assert_eq!(unification.max_unif_steps, 256);
    }

    #[test]
    fn process_options_records_ho_unification_overrides_like_c() {
        let config = run_config_from([
            "eprover",
            "--ext-sup-max-depth=5",
            "--inverse-recognition",
            "--replace-inj-defs",
            "--bce=true",
            "--bce-max-occs=-1",
            "--pred-elim=true",
            "--pred-elim-recognize-gates=true",
            "--pred-elim-force-mu-decrease=true",
            "--pred-elim-ignore-conj-syms=true",
            "--pred-elim-max-occs=9",
            "--pred-elim-tolerance=2",
            "--cnf-lambda-to-forall=false",
            "--eta-normalize=expand",
            "--ho-order-kind=lambda",
            "--eliminate-leibniz-eq=3",
            "--unroll-formulas-only=false",
            "--prim-enum-mode=logsym",
            "--prim-enum-max-depth=4",
            "--inst-choice-max-depth=6",
            "--local-rw=true",
            "--prune-args=true",
            "--preinstantiate-induction=true",
            "--func-proj-limit=1",
            "--imit-limit=2",
            "--ident-limit=3",
            "--elim-limit=4",
            "--unif-mode=multi",
            "--pattern-oracle=false",
            "--fixpoint-oracle=false",
            "--max-unifiers=8",
            "--max-unif-steps=9",
        ]);
        let preprocessing = &config.preprocessing;
        let pred_elim = &preprocessing.predicate_elimination;
        let inference = &config.search.inference;
        let ho_preprocessing = &inference.higher_order_preprocessing;
        let ho_search = &inference.higher_order_search;
        let unification = inference.higher_order_unification;

        assert_eq!(
            (preprocessing.bce.enabled, preprocessing.bce.max_occs),
            (true, -1)
        );
        assert_eq!(
            (pred_elim.enabled, pred_elim.max_occs, pred_elim.tolerance),
            (true, 9, 2)
        );
        assert!(pred_elim
            .flags
            .contains(PredicateEliminationFlag::RecognizeGates));
        assert!(pred_elim
            .flags
            .contains(PredicateEliminationFlag::ForceMuDecrease));
        assert!(pred_elim
            .flags
            .contains(PredicateEliminationFlag::IgnoreConjectureSymbols));
        assert_eq!(inference.higher_order.ext_rules_max_depth, 5);
        assert!(inference.higher_order.inverse_recognition);
        assert!(inference.higher_order.replace_inj_defs);
        assert_eq!(ho_preprocessing.eta_normalization, EtaNormalization::Expand);
        assert_eq!(
            config.search.ordering.ho_order_kind,
            HoOrderKind::LambdaOrder
        );
        assert!(!ho_preprocessing.lambda_to_forall);
        assert!(!ho_preprocessing.unroll_only_formulas);
        assert_eq!(ho_preprocessing.elim_leibniz_max_depth, 3);
        assert_eq!(
            ho_search.primitive_enumeration.mode,
            PrimEnumMode::LogSymbol
        );
        assert_eq!(ho_search.primitive_enumeration.max_depth, 4);
        assert_eq!(ho_preprocessing.inst_choice_max_depth, 6);
        assert!(ho_search.local_rw);
        assert!(ho_search.prune_args);
        assert!(ho_preprocessing.preinstantiate_induction);
        assert_eq!(unification.func_proj_limit, 1);
        assert_eq!(unification.imit_limit, 2);
        assert_eq!(unification.ident_limit, 3);
        assert_eq!(unification.elim_limit, 4);
        assert_eq!(unification.mode, UnificationMode::Multi);
        assert!(!unification.pattern_oracle);
        assert!(!unification.fixpoint_oracle);
        assert_eq!(unification.max_unifiers, 8);
        assert_eq!(unification.max_unif_steps, 9);
    }

    #[test]
    fn process_options_rejects_invalid_ho_unification_args() {
        let error =
            process_options(["eprover", "--classification-timeout-portion=37"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "--classification-timeout-portion expects 'true' or 'false' instead of '37'"
        );

        let error = process_options(["eprover", "--ext-sup-max-depth=-2"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--bce=maybe"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--bce-max-occs=-2"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--pred-elim-tolerance=-1"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--eta-normalize=both"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --eta-normalize requires 'reduce' or 'expand' as an argument"
        );

        let error = process_options(["eprover", "--ho-order-kind=both"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --ho-order-kind requires 'lfho' or 'lambda' as an argument"
        );

        let error = process_options(["eprover", "--prim-enum-mode=logsymbol"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --prim-enum-mode excepts neg, and, or, eq, pragmatic, full, or logsym"
        );

        let error = process_options(["eprover", "--func-proj-limit=64"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--unif-mode=bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "values of unif mode are eiter single or multi"
        );

        let error = process_options(["eprover", "--max-unifiers=1025"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--max-unif-steps=100001"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_records_goal_defs_modes_like_c() {
        let action = process_options(["eprover", "--goal-defs=None"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.preprocessing.goal_definitions.positive);
        assert!(!config.preprocessing.goal_definitions.negative);

        let action = process_options(["eprover", "--goal-defs=Neg"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.preprocessing.goal_definitions.positive);
        assert!(config.preprocessing.goal_definitions.negative);

        let action = process_options(["eprover", "--goal-defs=All"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.preprocessing.goal_definitions.positive);
        assert!(config.preprocessing.goal_definitions.negative);
    }

    #[test]
    fn process_options_rejects_invalid_preprocessing_modes() {
        let error = process_options(["eprover", "--goal-defs=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --goal-defs accepts only None, All, or Neg"
        );

        let error = process_options(["eprover", "--ac-handling=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --ac_handling requires None, DiscardAll, KeepUnits, or KeepOrientable as an argument"
        );

        let error = process_options(["eprover", "--presat-simplify=maybe"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_records_literal_selection_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.literal_selection.strategy, "NoSelection");
        assert_eq!(
            config.search.literal_selection.limits.positive_max,
            i64::MAX
        );
        assert_eq!(
            config.search.literal_selection.limits.negative_max,
            i64::MAX
        );
        assert_eq!(config.search.literal_selection.limits.all_max, i64::MAX);
        assert!(!config.search.literal_selection.select_on_processing_only);

        let action = process_options([
            "eprover",
            "-W",
            "SelectMaxLComplex",
            "--select-on-processing-only",
            "-i",
            "-j",
            "--inherit-conjecture-pm-literals",
            "--selection-pos-min=1",
            "--selection-pos-max=2",
            "--selection-neg-min=3",
            "--selection-neg-max=4",
            "--selection-all-min=5",
            "--selection-all-max=6",
            "--selection-weight-min=7",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        let selection = &config.search.literal_selection;
        assert_eq!(selection.strategy, "SelectMaxLComplex");
        assert!(selection.select_on_processing_only);
        assert!(selection.inherit_paramod_literals.any_clause);
        assert!(selection.inherit_paramod_literals.goal_clause);
        assert!(selection.inherit_paramod_literals.conjecture_clause);
        assert_eq!(selection.limits.positive_min, 1);
        assert_eq!(selection.limits.positive_max, 2);
        assert_eq!(selection.limits.negative_min, 3);
        assert_eq!(selection.limits.negative_max, 4);
        assert_eq!(selection.limits.all_min, 5);
        assert_eq!(selection.limits.all_max, 6);
        assert_eq!(selection.limits.weight_min, 7);

        let action = process_options(["eprover", "--no-generation"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.literal_selection.strategy, "NoGeneration");
    }

    #[test]
    fn process_options_records_heuristic_limits_and_completeness_like_c() {
        let action = process_options([
            "eprover",
            "--prefer-initial-clauses",
            "-x",
            "Auto",
            "--filter-orphans-limit",
            "--forward-contract-limit=88",
            "--delete-bad-limit",
            "--assume-completeness",
            "--assume-incompleteness",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.search.heuristic.prefer_initial_clauses);
        assert_eq!(config.search.heuristic.name, "Auto");
        assert_eq!(config.search.heuristic.filter_orphans_limit, 100);
        assert_eq!(config.search.heuristic.forward_contract_limit, 88);
        assert_eq!(config.delete_bad_limit, 1_500_000);
        assert!(config.search.completeness.inference_system_complete);
        assert!(config.search.completeness.assume_inference_system_complete);
        assert!(config.search.completeness.incomplete);
    }

    #[test]
    fn process_options_records_inference_and_splitting_state_like_c() {
        let action = process_options([
            "eprover",
            "--disable-eq-factoring",
            "--disable-paramod-into-neg-units",
            "--condense-aggressive",
            "--disable-given-clause-fw-contraction",
            "--oriented-supersimul-paramod",
            "--split-clauses",
            "--split-method=2",
            "--split-aggressive",
            "--split-reuse-defs",
            "--disequality-decomposition",
            "--disequality-decomp-maxarity=3",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        let inference = &config.search.inference;
        assert!(!inference.enable_eq_factoring);
        assert!(!inference.enable_neg_unit_paramod);
        assert!(!inference.enable_given_forward_simplification);
        assert_eq!(
            inference.paramodulation,
            ParamodulationType::OrientedSuperSim
        );
        assert!(inference.condensing.enabled);
        assert!(inference.condensing.aggressive);
        assert!(!config.search.completeness.inference_system_complete);
        assert_eq!(config.search.splitting.classes, 7);
        assert_eq!(config.search.splitting.method, 2);
        assert!(config.search.splitting.aggressive);
        assert!(!config.search.splitting.fresh_defs);
        assert_eq!(config.search.splitting.diseq_decomposition, 1024);
        assert_eq!(config.search.splitting.diseq_decomp_maxarity, 3);

        let action = process_options(["eprover", "--simul-paramod"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.inference.paramodulation,
            ParamodulationType::Sim
        );

        let action = process_options(["eprover", "--oriented-simul-paramod"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.inference.paramodulation,
            ParamodulationType::OrientedSim
        );

        let action = process_options(["eprover", "--supersimul-paramod"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.inference.paramodulation,
            ParamodulationType::SuperSim
        );
    }

    #[test]
    fn process_options_records_inference_processing_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        let inference = &config.search.inference;
        assert_eq!(
            inference.demodulation.forward_demod,
            RewriteLevel::FullRewrite
        );
        assert!(!inference.demodulation.lambda_demod);
        assert!(!inference.demodulation.prefer_general);
        assert!(!inference.context_simplification.forward);
        assert!(!inference.context_simplification.forward_aggressive);
        assert!(!inference.context_simplification.backward);
        assert!(!inference.equality_resolution.destructive);
        assert!(!inference.equality_resolution.strong_destructive);
        assert!(!inference.equality_resolution.aggressive);
        assert!(!config.search.support.use_tptp_sos);
        assert!(config.search.support.lift_lambdas);
        assert!(!config.search.support.strong_unit_forward_subsumption);
        assert!(!config.search.ordering.rewrite_strong_rhs_inst);

        let action = process_options([
            "eprover",
            "--sos-uses-input-types",
            "--destructive-er",
            "--strong-destructive-er",
            "--destructive-er-aggressive",
            "--forward-context-sr-aggressive",
            "--backward-context-sr",
            "--prefer-general-demodulators",
            "--forward-demod-level=1",
            "--demod-under-lambda=true",
            "--strong-rw-inst",
            "--strong-forward-subsumption",
            "--lift-lambdas=false",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        let inference = &config.search.inference;
        assert!(config.search.support.use_tptp_sos);
        assert!(inference.equality_resolution.destructive);
        assert!(inference.equality_resolution.strong_destructive);
        assert!(inference.equality_resolution.aggressive);
        assert!(inference.context_simplification.forward);
        assert!(inference.context_simplification.forward_aggressive);
        assert!(inference.context_simplification.backward);
        assert!(inference.demodulation.prefer_general);
        assert_eq!(
            inference.demodulation.forward_demod,
            RewriteLevel::RuleRewrite
        );
        assert!(inference.demodulation.lambda_demod);
        assert!(config.search.ordering.rewrite_strong_rhs_inst);
        assert!(config.search.support.strong_unit_forward_subsumption);
        assert!(!config.search.support.lift_lambdas);

        let action = process_options(["eprover", "--destructive-er-aggressive"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.search.inference.equality_resolution.destructive);
        assert!(config.search.inference.equality_resolution.aggressive);
    }

    #[test]
    fn process_options_records_sat_check_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        let sat_check = &config.search.sat_check;
        assert_eq!(sat_check.grounding, GroundingStrategy::NoGrounding);
        assert_eq!(sat_check.step_limit, i64::MAX);
        assert_eq!(sat_check.size_limit, i64::MAX);
        assert_eq!(sat_check.ttinsert_limit, i64::MAX);
        assert!(!sat_check.normconst);
        assert!(!sat_check.normalize);
        assert_eq!(sat_check.decision_limit, 10_000);

        let action = process_options([
            "eprover",
            "--satcheck-proc-interval",
            "--satcheck-gen-interval=6",
            "--satcheck-ttinsert-interval",
            "--satcheck=ConjMinMinFreq",
            "--satcheck-decision-limit",
            "--satcheck-normalize-const",
            "--satcheck-normalize-unproc",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        let sat_check = &config.search.sat_check;
        assert_eq!(sat_check.step_limit, 5_000);
        assert_eq!(sat_check.size_limit, 6);
        assert_eq!(sat_check.ttinsert_limit, 5_000_000);
        assert_eq!(sat_check.grounding, GroundingStrategy::ConjMinMinFreq);
        assert_eq!(sat_check.decision_limit, 100);
        assert!(sat_check.normconst);
        assert!(sat_check.normalize);

        let action =
            process_options(["eprover", "--satcheck", "--satcheck-decision-limit=-1"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.sat_check.grounding,
            GroundingStrategy::FirstConst
        );
        assert_eq!(config.search.sat_check.decision_limit, -1);
    }

    #[test]
    fn process_options_records_watchlist_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.watchlist.source, None);
        assert!(config.search.watchlist.simplify);
        assert!(!config.search.watchlist.is_static);

        let action = process_options(["eprover", "--watchlist"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.watchlist.source,
            Some(WatchlistSource::Inline)
        );
        assert!(!config.search.watchlist.is_static);

        let action = process_options(["eprover", "--watchlist=Use inline watchlist type"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.watchlist.source,
            Some(WatchlistSource::Inline)
        );

        let action = process_options([
            "eprover",
            "--static-watchlist=watch.p",
            "--no-watchlist-simplification",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.watchlist.source,
            Some(WatchlistSource::File("watch.p".to_owned()))
        );
        assert!(config.search.watchlist.is_static);
        assert!(!config.search.watchlist.simplify);
    }

    #[test]
    fn process_options_records_subsumption_index_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.search.inference.subsumption.forward_aggressive);
        assert_eq!(
            config.search.fv_index.feature_type,
            FvIndexFeatureType::AcFold
        );
        assert!(!config.search.fv_index.use_perm_vectors);
        assert!(!config.search.fv_index.eliminate_uninformative);
        assert_eq!(config.search.fv_index.max_symbols, 17);
        assert_eq!(config.search.fv_index.symbol_slack, 0);

        let action = process_options([
            "eprover",
            "--fw-subsumption-aggressive",
            "--subsumption-indexing=Perm",
            "--fvindex-featuretypes=BillPlus",
            "--fvindex-maxfeatures",
            "--fvindex-slack=3",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.search.inference.subsumption.forward_aggressive);
        assert_eq!(
            config.search.fv_index.feature_type,
            FvIndexFeatureType::BillPlusFeatures
        );
        assert!(config.search.fv_index.use_perm_vectors);
        assert!(!config.search.fv_index.eliminate_uninformative);
        assert_eq!(config.search.fv_index.max_symbols, 200);
        assert_eq!(config.search.fv_index.symbol_slack, 3);

        let action = process_options(["eprover", "--conventional-subsumption"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.fv_index.feature_type,
            FvIndexFeatureType::NoFeatures
        );

        let action = process_options([
            "eprover",
            "--subsumption-indexing=PermOpt",
            "--subsumption-indexing=Direct",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.search.fv_index.use_perm_vectors);
        assert!(config.search.fv_index.eliminate_uninformative);
    }

    #[test]
    fn fv_index_params_from_config_maps_cli_state() {
        let config = run_config_from([
            "eprover",
            "--subsumption-indexing=PermOpt",
            "--fvindex-featuretypes=BillPlus",
            "--fvindex-maxfeatures=200",
            "--fvindex-slack=3",
        ]);

        let params = fv_index_params_from_config(&config.search.fv_index).unwrap_or_else(|err| {
            panic!("{err}");
        });

        assert_eq!(params.cspec().features(), FvIndexType::BillPlusFeatures);
        assert!(params.use_perm_vectors());
        assert!(params.eliminate_uninformative());
        assert_eq!(params.max_symbols(), 200);
        assert_eq!(params.symbol_slack(), 3);
    }

    #[test]
    fn proof_control_from_config_installs_configured_parameters() {
        let config = run_config_from([
            "eprover",
            "--expert-heuristic=Auto",
            "--split-clauses=3",
            "--delete-bad-limit=77",
            "--subsumption-indexing=Perm",
            "--fvindex-featuretypes=ACStagger",
            "--fvindex-maxfeatures=19",
            "--fvindex-slack=2",
            "--record-gcs",
            "--strong-forward-subsumption",
        ]);

        let control = proof_control_from_config(&config).unwrap_or_else(|err| panic!("{err}"));

        assert!(control.ocb().is_none());
        assert!(control.active_hcb().is_none());
        assert!(control.wfcbs().is_empty());
        assert!(control.hcbs().is_empty());
        assert_eq!(control.solver().generation(), 1);
        assert!(control.record_gc_selection());
        assert!(control.strong_unit_forward_subsumption());
        assert_eq!(control.heuristic_parms().heuristic_name, "Auto");
        assert_eq!(control.heuristic_parms().delete_bad_limit, 77);
        assert_eq!(control.heuristic_parms().split_clauses.c_value(), 3);
        assert_eq!(
            control.fvi_parms().cspec().features(),
            FvIndexType::AcStagger
        );
        assert!(control.fvi_parms().use_perm_vectors());
        assert!(!control.fvi_parms().eliminate_uninformative());
        assert_eq!(control.fvi_parms().max_symbols(), 19);
        assert_eq!(control.fvi_parms().symbol_slack(), 2);
    }

    #[test]
    fn proof_control_from_config_applies_selected_predefined_strategy() {
        let config = run_config_from([
            "eprover",
            "--select-strategy=G-E--_208_C12_11_nc_F1_SE_CS_SP_PS_S5PRR_S04BN",
        ]);

        let control = proof_control_from_config(&config).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(control.heuristic_parms().heuristic_name, "Default");
        assert_eq!(
            control.heuristic_parms().selection_strategy,
            "PSelectComplexExceptUniqMaxHorn"
        );
        assert_eq!(
            control.heuristic_parms().forward_demod,
            RewriteLevel::FullRewrite
        );
    }

    #[test]
    fn process_options_records_fingerprint_index_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.fingerprint_index.rw_bw_index_type, "FP7");
        assert_eq!(config.search.fingerprint_index.pm_from_index_type, "FP7");
        assert_eq!(config.search.fingerprint_index.pm_into_index_type, "FP7");
        assert!(config.search.fingerprint_index.pdt_use_size_constraints);
        assert!(config.search.fingerprint_index.pdt_use_age_constraints);

        let action = process_options([
            "eprover",
            "--rw-bw-index=FP0",
            "--pm-from-index=NoIndex",
            "--pm-into-index=NPDT",
            "--pdt-no-size-constr",
            "--pdt-no-age-constr",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.fingerprint_index.rw_bw_index_type, "FP0");
        assert_eq!(
            config.search.fingerprint_index.pm_from_index_type,
            "NoIndex"
        );
        assert_eq!(config.search.fingerprint_index.pm_into_index_type, "NPDT");
        assert!(!config.search.fingerprint_index.pdt_use_size_constraints);
        assert!(!config.search.fingerprint_index.pdt_use_age_constraints);

        let action = process_options(["eprover", "--fp-index=FP7M"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.fingerprint_index.rw_bw_index_type, "FP7M");
        assert_eq!(config.search.fingerprint_index.pm_from_index_type, "FP7M");
        assert_eq!(config.search.fingerprint_index.pm_into_index_type, "FP7M");

        let action = process_options(["eprover", "--fp-no-size-constr"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.search.fingerprint_index.pdt_use_size_constraints);
        assert!(config.search.fingerprint_index.pdt_use_age_constraints);
    }

    #[test]
    fn process_options_rejects_invalid_search_control_args() {
        let error = process_options(["eprover", "-W", "none"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().starts_with(
            "Wrong argument to option -W (--literal-selection-strategy). Possible values: "
        ));
        assert!(error.message().contains("NoSelection"));

        let error = process_options(["eprover", "--split-method=3"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--forward-demod-level=3"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--demod-under-lambda=maybe"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--lift-lambdas=maybe"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--definitional-cnf=-1"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--miniscope-limit=-1"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--fool-unroll=maybe"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--arg-cong=bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "neg-ext excepts either all, max or off");

        let error = process_options(["eprover", "--neg-ext=bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "neg-ext excepts either all or max");

        let error = process_options(["eprover", "--pos-ext=bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "pos-ext excepts either all or max");

        let error = process_options(["eprover", "--satcheck-proc-interval=0"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--satcheck-gen-interval=0"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--satcheck-ttinsert-interval=0"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--satcheck=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error
            .message()
            .starts_with("Wrong argument to option --sat-check. Possible values: "));
        assert!(error.message().contains("ConjMinMinFreq"));

        let error = process_options(["eprover", "--satcheck-decision-limit=-2"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error =
            process_options(["eprover", "--satcheck-decision-limit=2147483648"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--subsumption-indexing=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --subsumption-indexing requires 'None', 'Direct', 'Perm', or 'PermOpt'."
        );

        let error = process_options(["eprover", "--fvindex-featuretypes=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error
            .message()
            .starts_with("Option --fvindex-featuretypes requires "));

        let error = process_options(["eprover", "--fvindex-maxfeatures=0"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Argument to option --fvindex-maxfeatures has to be > 0"
        );

        let error = process_options(["eprover", "--fvindex-slack=-1"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--rw-bw-index=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error
            .message()
            .starts_with("Wrong argument to option --rw-bw-index. Possible values: "));
        assert!(error.message().contains("FP7"));
    }

    #[test]
    fn process_options_records_term_ordering_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        let ordering = &config.search.ordering;
        assert_eq!(ordering.ordering, TermOrdering::Kbo6);
        assert_eq!(ordering.weight_generation, "none");
        assert_eq!(ordering.precedence_generation, "none");
        assert_eq!(ordering.constant_weight, 0);
        assert_eq!(ordering.precedence, None);
        assert_eq!(ordering.weight_overrides, None);
        assert_eq!(ordering.lpo_recursion_limit, 1_000);
        assert_eq!(ordering.literal_comparison, LiteralComparison::Normal);
        assert_eq!(ordering.lambda_weight, 20);
        assert_eq!(ordering.db_weight, 10);
        assert!(!ordering.rewrite_strong_rhs_inst);

        let action = process_options(["eprover", "--term-ordering=RPO"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.ordering.ordering, TermOrdering::Rpo);

        let action = process_options([
            "eprover",
            "-t",
            "LPO4Copy",
            "-w",
            "invfreqrank",
            "--order-weights=f:2,g:3",
            "-G",
            "invfreq",
            "--prec-pure-conj",
            "--prec-conj-axiom=6",
            "--prec-pure-axiom",
            "--prec-skolem=4",
            "--prec-defpred",
            "-c",
            "3",
            "--precedence=f>g",
            "--kbo-lam-weight=30",
            "--kbo-db-weight=12",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        let ordering = &config.search.ordering;
        assert_eq!(ordering.ordering, TermOrdering::Lpo4Copy);
        assert_eq!(ordering.weight_generation, "invfreqrank");
        assert_eq!(ordering.weight_overrides.as_deref(), Some("f:2,g:3"));
        assert_eq!(ordering.precedence_generation, "invfreq");
        assert_eq!(ordering.precedence_modifiers.conjecture_only, 10);
        assert_eq!(ordering.precedence_modifiers.conjecture_axiom, 6);
        assert_eq!(ordering.precedence_modifiers.axiom_only, 2);
        assert_eq!(ordering.precedence_modifiers.skolem, 4);
        assert_eq!(ordering.precedence_modifiers.defpred, 2);
        assert_eq!(ordering.constant_weight, 3);
        assert_eq!(ordering.precedence.as_deref(), Some("f>g"));
        assert_eq!(ordering.lambda_weight, 30);
        assert_eq!(ordering.db_weight, 12);

        let action = process_options(["eprover", "--precedence"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.ordering.precedence.as_deref(), Some(""));
    }

    #[test]
    fn order_parms_from_config_maps_cli_ordering_state() {
        let config = run_config_from([
            "eprover",
            "-t",
            "LPO4Copy",
            "-w",
            "invfreqrank",
            "--order-weights=f:2,g:3",
            "-G",
            "invfreq",
            "--prec-pure-conj",
            "--prec-conj-axiom=6",
            "--prec-pure-axiom",
            "--prec-skolem=4",
            "--prec-defpred",
            "-c",
            "3",
            "--precedence=f>g",
            "--literal-comparison=TFOEqMin",
            "--kbo-lam-weight=30",
            "--kbo-db-weight=12",
            "--strong-rw-inst",
            "--ho-order-kind=lambda",
        ]);

        let params =
            order_parms_from_config(&config.search.ordering).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(params.ordertype, to_params::TermOrdering::Lpo4Copy);
        assert_eq!(
            params.to_weight_gen,
            to_params::TOWeightGenMethod::InvFrequencyRank
        );
        assert_eq!(
            params.to_prec_gen,
            to_params::TOPrecGenMethod::ByInvFrequency
        );
        assert_eq!(params.conj_only_mod, 10);
        assert_eq!(params.conj_axiom_mod, 6);
        assert_eq!(params.axiom_only_mod, 2);
        assert_eq!(params.skolem_mod, 4);
        assert_eq!(params.defpred_mod, 2);
        assert_eq!(params.to_const_weight, 3);
        assert_eq!(params.to_pre_prec.as_deref(), Some("f>g"));
        assert_eq!(params.to_pre_weights.as_deref(), Some("f:2,g:3"));
        assert_eq!(
            params.lit_cmp,
            i64::from(to_params::LiteralCmp::TfoEqMin.c_value())
        );
        assert_eq!(params.lam_w, 30);
        assert_eq!(params.db_w, 12);
        assert!(params.rewrite_strong_rhs_inst);
        assert_eq!(params.ho_order_kind, HoOrderKind::LambdaOrder);

        let rpo_config = run_config_from(["eprover", "--term-ordering=RPO"]);
        let rpo_params = order_parms_from_config(&rpo_config.search.ordering)
            .unwrap_or_else(|err| panic!("{err}"));
        assert_eq!(rpo_params.ordertype, to_params::TermOrdering::Rpo);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one regression test covers the one-to-one executable config to HeuristicParmsCell field map"
    )]
    fn heuristic_parms_from_config_maps_cli_search_state() {
        let config = run_config_from([
            "eprover",
            "--memory-limit=2",
            "--delete-bad-limit=77",
            "--no-preprocessing",
            "--eq-unfold-limit=7",
            "--eq-unfold-maxclauses=11",
            "--goal-defs=Neg",
            "--goal-subterm-defs",
            "--sine=Auto",
            "--ac-handling=KeepOrientable",
            "--ac-non-aggressive",
            "--fool-unroll=false",
            "--bce=true",
            "--bce-max-occs=9",
            "--pred-elim=true",
            "--pred-elim-recognize-gates=true",
            "--pred-elim-force-mu-decrease=true",
            "--pred-elim-ignore-conj-syms=true",
            "--pred-elim-max-occs=10",
            "--pred-elim-tolerance=3",
            "--prefer-initial-clauses",
            "-x",
            "Auto",
            "-H",
            "h1",
            "--define-heuristic=h2",
            "-W",
            "SelectMaxLComplex",
            "--select-on-processing-only",
            "-i",
            "-j",
            "--inherit-conjecture-pm-literals",
            "--selection-pos-min=1",
            "--selection-neg-max=9",
            "--disable-eq-factoring",
            "--disable-paramod-into-neg-units",
            "--condense-aggressive",
            "--disable-given-clause-fw-contraction",
            "--oriented-supersimul-paramod",
            "--split-clauses=3",
            "--split-method=2",
            "--split-aggressive",
            "--split-reuse-defs",
            "--disequality-decomposition=5",
            "--disequality-decomp-maxarity=4",
            "--rw-bw-index=FP0",
            "--pm-from-index=NoIndex",
            "--pm-into-index=NPDT",
            "--sos-uses-input-types",
            "--destructive-er",
            "--strong-destructive-er",
            "--destructive-er-aggressive",
            "--forward-context-sr-aggressive",
            "--backward-context-sr",
            "--fw-subsumption-aggressive",
            "--prefer-general-demodulators",
            "--forward-demod-level=1",
            "--demod-under-lambda=true",
            "--satcheck=ConjMinMinFreq",
            "--satcheck-proc-interval=6",
            "--satcheck-gen-interval=7",
            "--satcheck-ttinsert-interval=8",
            "--satcheck-decision-limit=-1",
            "--satcheck-normalize-const",
            "--satcheck-normalize-unproc",
            "--static-watchlist=watch.p",
            "--no-watchlist-simplification",
            "--presat-simplify=true",
            "--detsort-rw",
            "--detsort-new",
            "--arg-cong=max",
            "--neg-ext=all",
            "--pos-ext=off",
            "--ext-sup-max-depth=4",
            "--inverse-recognition",
            "--replace-inj-defs",
            "--lift-lambdas=false",
            "--cnf-lambda-to-forall=false",
            "--unroll-formulas-only=false",
            "--eliminate-leibniz-eq=5",
            "--prim-enum-mode=full",
            "--prim-enum-max-depth=6",
            "--inst-choice-max-depth=7",
            "--local-rw=true",
            "--prune-args=true",
            "--preinstantiate-induction=true",
            "--func-proj-limit=1",
            "--imit-limit=2",
            "--ident-limit=3",
            "--elim-limit=4",
            "--unif-mode=multi",
            "--pattern-oracle=false",
            "--fixpoint-oracle=false",
            "--max-unifiers=8",
            "--max-unif-steps=9",
        ]);

        let params = heuristic_parms_from_config(&config).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(params.mem_limit, 2 * MEGA);
        assert_eq!(params.delete_bad_limit, 77);
        assert!(params.no_preproc);
        assert_eq!(params.eqdef_incrlimit, 7);
        assert_eq!(params.eqdef_maxclauses, 11);
        assert!(!params.add_goal_defs_pos);
        assert!(params.add_goal_defs_neg);
        assert!(params.add_goal_defs_subterms);
        assert_eq!(params.sine.as_deref(), Some("Auto"));
        assert_eq!(params.ac_handling, hcb_params::AcHandling::KeepOrientable);
        assert!(!params.ac_res_aggressive);
        assert!(!params.fool_unroll);
        assert!(params.bce);
        assert_eq!(params.bce_max_occs, 9);
        assert!(params.pred_elim);
        assert!(params.pred_elim_gates);
        assert_eq!(params.pred_elim_max_occs, 10);
        assert_eq!(params.pred_elim_tolerance, 3);
        assert!(params.pred_elim_force_mu_decrease);
        assert!(params.pred_elim_ignore_conj_syms);
        assert_eq!(params.heuristic_name, "Auto");
        assert_eq!(params.heuristic_def.as_deref(), Some("h2"));
        assert!(params.prefer_initial_clauses);
        assert_eq!(params.selection_strategy, "SelectMaxLComplex");
        assert_eq!(params.pos_lit_sel_min, 1);
        assert_eq!(params.neg_lit_sel_max, 9);
        assert!(params.select_on_proc_only);
        assert!(params.inherit_paramod_lit);
        assert!(params.inherit_goal_pm_lit);
        assert!(params.inherit_conj_pm_lit);
        assert!(!params.enable_eq_factoring);
        assert!(!params.enable_neg_unit_paramod);
        assert!(!params.enable_given_forward_simpl);
        assert_eq!(
            params.pm_type,
            hcb_params::ParamodulationType::OrientedSuperSim
        );
        assert!(params.condensing);
        assert!(params.condensing_aggressive);
        assert_eq!(params.split_clauses.c_value(), 3);
        assert!(params
            .split_clauses
            .contains(hcb_params::SplitClassType::HORN));
        assert!(params
            .split_clauses
            .contains(hcb_params::SplitClassType::NON_HORN));
        assert_eq!(params.split_method, hcb_params::SplitType::GroundFull);
        assert!(params.split_aggressive);
        assert!(!params.split_fresh_defs);
        assert_eq!(params.diseq_decomposition, 5);
        assert_eq!(params.diseq_decomp_maxarity, 4);
        assert_eq!(params.rw_bw_index_type, "FP0");
        assert_eq!(params.pm_from_index_type, "NoIndex");
        assert_eq!(params.pm_into_index_type, "NPDT");
        assert!(params.use_tptp_sos);
        assert!(params.er_varlit_destructive);
        assert!(params.er_strong_destructive);
        assert!(params.er_aggressive);
        assert!(params.forward_context_sr);
        assert!(params.forward_context_sr_aggressive);
        assert!(params.backward_context_sr);
        assert!(params.forward_subsumption_aggressive);
        assert!(params.prefer_general);
        assert_eq!(params.forward_demod, RewriteLevel::RuleRewrite);
        assert!(params.lambda_demod);
        assert_eq!(
            params.sat_check_grounding,
            hcb_params::GroundingStrategy::ConjMinMinFreq
        );
        assert_eq!(params.sat_check_step_limit, 6);
        assert_eq!(params.sat_check_size_limit, 7);
        assert_eq!(params.sat_check_ttinsert_limit, 8);
        assert_eq!(params.sat_check_decision_limit, -1);
        assert!(params.sat_check_normconst);
        assert!(params.sat_check_normalize);
        assert!(params.watchlist_is_static);
        assert!(!params.watchlist_simplify);
        assert!(params.presat_interreduction);
        assert!(params.detsort_bw_rw);
        assert!(params.detsort_tmpset);
        assert_eq!(params.arg_cong, hcb_params::ExtInferenceType::MaxLits);
        assert_eq!(params.neg_ext, hcb_params::ExtInferenceType::AllLits);
        assert_eq!(params.pos_ext, hcb_params::ExtInferenceType::NoLits);
        assert_eq!(params.ext_rules_max_depth, 4);
        assert!(params.inverse_recognition);
        assert!(params.replace_inj_defs);
        assert!(!params.lift_lambdas);
        assert!(!params.lambda_to_forall);
        assert!(!params.unroll_only_formulas);
        assert_eq!(params.elim_leibniz_max_depth, 5);
        assert_eq!(params.prim_enum_mode, hcb_params::PrimEnumMode::Full);
        assert_eq!(params.prim_enum_max_depth, 6);
        assert_eq!(params.inst_choice_max_depth, 7);
        assert!(params.local_rw);
        assert!(params.prune_args);
        assert!(params.preinstantiate_induction);
        assert_eq!(params.func_proj_limit, 1);
        assert_eq!(params.imit_limit, 2);
        assert_eq!(params.ident_limit, 3);
        assert_eq!(params.elim_limit, 4);
        assert_eq!(params.unif_mode, hcb_params::UnifMode::Multi);
        assert!(!params.pattern_oracle);
        assert!(!params.fixpoint_oracle);
        assert_eq!(params.max_unifiers, 8);
        assert_eq!(params.max_unif_steps, 9);
    }

    #[test]
    fn process_options_records_lpo_literal_comparison_fallthrough_like_c() {
        let action = process_options(["eprover", "--lpo-recursion-limit"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.ordering.lpo_recursion_limit, 100);
        assert!(config.search.ordering.lpo_recursion_limit_changed);
        assert!(config.warnings.is_empty());
        assert_eq!(
            config.search.ordering.literal_comparison,
            LiteralComparison::None
        );

        let action = process_options([
            "eprover",
            "--lpo-recursion-limit=25",
            "--literal-comparison=TFOEqMin",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.ordering.lpo_recursion_limit, 25);
        assert!(config.search.ordering.lpo_recursion_limit_changed);
        assert_eq!(
            config.search.ordering.literal_comparison,
            LiteralComparison::TfoEqMin
        );

        let action = process_options(["eprover", "--lpo-recursion-limit=20001"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.ordering.lpo_recursion_limit, 20001);
        assert!(config.search.ordering.lpo_recursion_limit_changed);
        assert_eq!(
            config.search.ordering.literal_comparison,
            LiteralComparison::None
        );
        assert_eq!(config.warnings.len(), 1);
        assert_eq!(config.warnings[0].message(), LPO_RECURSION_LIMIT_WARNING);

        let action = process_options(["eprover", "--restrict-literal-comparisons"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(
            config.search.ordering.literal_comparison,
            LiteralComparison::None
        );
    }

    #[test]
    fn process_options_rejects_invalid_term_ordering_args() {
        let error = process_options(["eprover", "-t", "Auto"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option -t (--term-ordering) requires LPO, LPO4, KBO, KBO6 or RPO as an argument"
        );

        let error = process_options(["eprover", "-w", "none"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().starts_with(
            "Wrong argument to option -w (--order-weight-generation). Possible values: "
        ));
        assert!(error.message().contains("invfreqrank"));

        let error = process_options(["eprover", "-G", "none"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().starts_with(
            "Wrong argument to option -G (--order-precedence-generation). Possible values: "
        ));
        assert!(error.message().contains("invfreq"));

        let error = process_options(["eprover", "-c", "0"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Argument to option -c (--order-constant-weight) has to be > 0"
        );

        let error = process_options(["eprover", "--lpo-recursion-limit=0"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Argument to option --lpo-recursion-limit has to be > 0"
        );

        let error = process_options(["eprover", "--literal-comparison=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Wrong argument to --literal-comparison (valid: None, Normal, TFOEqMax, TFOEqMin)."
        );
    }

    #[test]
    fn process_options_rejects_invalid_pcl_shell_level() {
        let error = process_options(["eprover", "--pcl-shell-level=3"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_records_proof_output_state_like_c() {
        let action = process_options(["eprover", "--proof-object=0"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.proof_object_level, 0);
        assert_eq!(config.proof_output, 0);

        let action = process_options(["eprover", "--proof-object=3", "--proof-object=1"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.proof_object_level, 3);
        assert_eq!(config.proof_output, 1);

        let action = process_options(["eprover", "--proof-graph=2"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.proof_object_level, 1);
        assert_eq!(config.proof_output, 3);

        let action =
            process_options(["eprover", "-d", "--proof-statistics", "--record-gcs"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::FullDerivation));
        assert!(config.flags.contains(EProverFlag::ProofStatistics));
        assert!(config.flags.contains(EProverFlag::RecordGivenClauses));
        assert_eq!(config.proof_object_level, 1);

        let action =
            process_options(["eprover", "--force-deriv=2", "--training-examples=3"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.force_derivation_output, 2);
        assert_eq!(config.training_examples, Some(3));
        assert!(config.flags.contains(EProverFlag::RecordGivenClauses));
        assert_eq!(config.proof_object_level, 1);
    }

    #[test]
    fn process_options_rejects_force_derivation_outside_c_range() {
        let error = process_options(["eprover", "--force-deriv=3"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_rejects_non_increasing_cpu_limits() {
        let error =
            process_options(["eprover", "--cpu-limit=10", "--soft-cpu-limit=10"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Soft time limit has to be smaller than hardtime limit"
        );

        let error =
            process_options(["eprover", "--soft-cpu-limit=20", "--cpu-limit=10"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Hard time limit has to be larger than softtime limit"
        );
    }

    #[test]
    fn run_version_prints_c_compatible_version_line() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(["eprover", "-V"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "E 3.3.5 Countess Grey (facc36eaf92d70896d830140efc4382df9e8dcdb)\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn run_help_prints_usage() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(["eprover", "-h"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Usage: eprover [options] [files]"));
        assert!(output.contains("--version"));
    }

    #[test]
    fn run_print_strategy_prints_current_parameters_without_input() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", "--print-strategy"], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.starts_with("{\n"));
        assert!(output.contains("heuristic_name:                Default"));
        assert!(output.contains("selection_strategy:             NoSelection"));
    }

    #[test]
    fn run_print_strategy_prints_predefined_names_without_input() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-strategy=>all-names<"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.starts_with("G-E--_208_C12_11_nc_F1_SE_CS_SP_PS_S5PRR_S04BN\n"));
        assert!(output.lines().count() > 400);
        assert!(!output.contains(" = "));
    }

    #[test]
    fn run_print_strategy_validates_selected_strategy_before_all_names() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                "eprover",
                "--select-strategy=Missing",
                "--print-strategy=>all-names<",
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            error.message(),
            "Error: Configuration name Missing not found."
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn run_print_strategy_prints_named_predefined_strategy_without_input() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-strategy=G-E--_208_C12_11_nc_F1_SE_CS_SP_PS_S5PRR_S04BN",
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.starts_with("{\n"));
        assert!(output.contains("selection_strategy:             PSelectComplexExceptUniqMaxHorn"));
        assert!(output.contains("pm_type:                        ParamodSim"));
    }

    #[test]
    fn run_app_encode_prints_supported_tff_formula_and_exits() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-tff");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(f_type, type, f: person > person).\n\
             tff(p_type, type, p: person > $o).\n\
             tff(ax, axiom, p(f(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("%-- person."));
        assert!(printed.contains("tff(typedecl"));
        assert!(printed.contains("tff(symboltypedecl"));
        assert!(printed.contains("app_"));
        assert!(printed.contains("tff(ax, axiom, "));
        assert!(!printed.contains("SZS status"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_ignores_includes_and_skips_top_level_true() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-ignore-include");
        std::fs::write(
            &path,
            "include('definitely_missing_for_app_encode.ax',[missing]).\n\
             fof(skip_true, axiom, $true).\n\
             fof(show_false, axiom, $false).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(!printed.contains("skip_true"));
        assert!(printed.contains("tff(show_false, axiom, "));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_accepts_tstp_cnf_entries_without_formula_output() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-cnf-only");
        std::fs::write(&path, "cnf(c1, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--app-encode",
                "--error-on-empty",
                "--tstp-in",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            default_preprocessing_debug_line()
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_accepts_old_tptp_input_clause_entries() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-old-tptp-input-clause");
        std::fs::write(
            &path,
            "input_clause(c1, axiom, [++p(a)]).\n\
             input_formula(f1, axiom, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", "--tptp-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(f1, axiom, "));
        assert!(!printed.contains("input_clause"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_accepts_old_tptp_existential_fool_body() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-old-tptp-existential-fool-body");
        std::fs::write(
            &path,
            "input_formula(ex_ite, axiom, ?[X]:$ite(p(X), q(X), r(X))).\n\
             input_formula(ex_let_eq, axiom, ?[Y]:$let(f:$i, f := a, f) = Y).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", "--tptp-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(ex_ite, axiom, ?[X"));
        assert!(printed.contains("]:$ite(app_"));
        assert!(printed.contains("tff(ex_let_eq, axiom, ?[X"));
        assert!(printed.contains("]:$let(f:$i,f:=a,f)=X"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_accepts_mixed_tstp_cnf_and_formula_entries() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-mixed-cnf-fof");
        std::fs::write(
            &path,
            "cnf(c1, axiom, (p(c))).\n\
             fof(f1, axiom, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", "--tstp-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(f1, axiom, "));
        assert!(!printed.contains("cnf(c1"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_preserves_supported_binary_operator_spelling() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-binary-operators");
        std::fs::write(
            &path,
            "fof(reverse, axiom, (p(a)<=q(a))).\n\
             fof(xor, axiom, (p(a)<~>q(a))).\n\
             fof(nand, axiom, (p(a)~&q(a))).\n\
             fof(nor, axiom, (p(a)~|q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("<="));
        assert!(printed.contains("<~>"));
        assert!(printed.contains("~&"));
        assert!(printed.contains("~|"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_accepts_boolean_ite_formula_atom() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-boolean-ite");
        std::fs::write(&path, "fof(ite_bool, axiom, $ite(p(a), q(a), ~r(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(ite_bool, axiom, $ite("));
        assert!(printed.contains("app_"));
        assert!(printed.contains("~(app_"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_accepts_boolean_let_formula_atom() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-boolean-let");
        std::fs::write(&path, "fof(let_bool, axiom, $let(f:$o, f := p(a), f)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(let_bool, axiom, $let(f:$o,f:=app_"));
        assert!(printed.contains(",f))."));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_rewrites_boolean_ite_equality_as_equivalence() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-boolean-ite-equality");
        std::fs::write(
            &path,
            "fof(eq_bool, axiom, ($ite(p(a), q(a), r(a)) = s(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(eq_bool, axiom, ($ite("));
        assert!(printed.contains("<=>app_"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_rewrites_boolean_ite_disequality_as_xor() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-boolean-ite-disequality");
        std::fs::write(
            &path,
            "fof(ne_bool, axiom, ($ite(p(a), q(a), r(a)) != s(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(ne_bool, axiom, ($ite("));
        assert!(printed.contains("<~>app_"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_accepts_non_boolean_fool_term_equality() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-non-boolean-fool-term-equality");
        std::fs::write(
            &path,
            "tff(a_type, type, a: $i).\n\
             tff(b_type, type, b: $i).\n\
             tff(c_type, type, c: $i).\n\
             tff(p_type, type, p: $i > $o).\n\
             fof(ite_i_eq, axiom, ($ite(p(a), a, b) = c)).\n\
             fof(let_i_eq, axiom, ($let(f:$i, f := a, f) = b)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(ite_i_eq, axiom, $ite(app_"));
        assert!(printed.contains(",a,b)=c)."));
        assert!(printed.contains("tff(let_i_eq, axiom, $let(f:$i,f:=a,f)=b)."));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_accepts_existential_fool_body() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-existential-fool-body");
        std::fs::write(
            &path,
            "tff(a_type, type, a: $i).\n\
             tff(b_type, type, b: $i).\n\
             tff(p_type, type, p: $i > $o).\n\
             tff(q_type, type, q: $i > $o).\n\
             tff(r_type, type, r: $i > $o).\n\
             fof(ex_ite, axiom, ?[X]:$ite(p(X), q(X), r(X))).\n\
             fof(ex_let_eq, axiom, ?[Y]:$let(f:$i, f := a, f) = Y).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("tff(ex_ite, axiom, ?[X"));
        assert!(printed.contains("]:$ite(app_"));
        assert!(printed.contains("tff(ex_let_eq, axiom, ?[X"));
        assert!(printed.contains("]:$let(f:$i,f:=a,f)=X"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_app_encode_reserves_internal_symbol_codes_for_temporary_bank() {
        let _guard = global_state_lock();
        let path = temp_path("app-encode-internal-symbol-codes");
        let mut input = String::new();
        for index in 1..=9 {
            writeln!(&mut input, "fof(ax{index}, axiom, p{index}(a{index})).").unwrap();
        }
        std::fs::write(&path, input).unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--app-encode", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("tff(ax9, axiom, "));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_applies_verbose_option_to_global_gate() {
        let _guard = global_state_lock();
        set_verbose_level(0);
        let path = temp_path("verbose");
        std::fs::write(&path, "").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--verbose=2", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(verbose_level(), 2);
        set_verbose_level(0);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_rejects_verbose_values_outside_c_int_range() {
        let _guard = global_state_lock();
        set_verbose_level(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--verbose=2147483648"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("out of int range"));
        assert_eq!(verbose_level(), 0);
    }

    #[test]
    fn run_applies_cpu_limit_options_to_signal_state() {
        let _guard = global_state_lock();
        configure_time_limits(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let path = temp_path("cpu-limits");
        std::fs::write(&path, "").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--soft-cpu-limit=25",
                "--cpu-limit=100",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(hard_time_limit(), 100);
        assert_eq!(soft_time_limit(), 25);
        assert_eq!(schedule_time_limit(), 100);

        configure_time_limits(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_without_cpu_limit_resets_prior_time_up_latch() {
        let _guard = global_state_lock();
        configure_time_limits(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let _ = set_time_is_up(true);
        let path = temp_path("cpu-limit-reset");
        std::fs::write(&path, "").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(!time_is_up());
        assert_eq!(hard_time_limit(), RLIM_INFINITY_COMPAT);
        assert_eq!(soft_time_limit(), RLIM_INFINITY_COMPAT);
        assert_eq!(schedule_time_limit(), 0);

        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_cooperative_time_limit_uses_resource_out_exit_status() {
        let _guard = global_state_lock();
        configure_time_limits(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        let path = temp_path("cpu-limit-resource-out");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--cpu-limit=0", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        assert!(String::from_utf8(stdout)
            .unwrap()
            .contains("\n% Failure: User resource limit exceeded!\n% SZS status ResourceOut\n"));
        assert!(stderr.is_empty());

        configure_time_limits(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_applies_output_level_options_to_global_gate() {
        let _guard = global_state_lock();
        let _ = set_output_level(1);
        let silent_path = temp_path("silent");
        let output_path = temp_path("output-level");
        std::fs::write(&silent_path, "").unwrap();
        std::fs::write(&output_path, "").unwrap();
        let silent_arg = silent_path.to_string_lossy().into_owned();
        let output_arg = output_path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--silent", silent_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(output_level(), 0);

        let status = run(
            ["eprover", "--output-level=3", output_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(output_level(), 3);
        let _ = set_output_level(1);
        std::fs::remove_file(&silent_path).unwrap();
        std::fs::remove_file(&output_path).unwrap();
    }

    #[test]
    fn run_print_info_uses_configured_output_target() {
        let _guard = global_state_lock();
        let path = temp_path("print-info");
        let input_path = temp_path("print-info-input");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&input_path, "").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let input_arg = input_path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-version",
                "-o",
                path_arg.as_str(),
                input_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        let debug_line = default_preprocessing_debug_line();
        assert_eq!(std::str::from_utf8(&stdout).unwrap(), debug_line);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!(
                "% Version: {VERSION}\n% Scanning for AC axioms\n\n% No proof found!\n% SZS status Satisfiable\n"
            )
        );
        std::fs::remove_file(&path).unwrap();
        stdout.clear();

        let status = run(
            ["eprover", "--print-version", "-o", "-", input_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "% Version: {VERSION}\n{debug_line}% Scanning for AC axioms\n\n% No proof found!\n% SZS status Satisfiable\n"
            )
        );
        std::fs::remove_file(&input_path).unwrap();
    }

    #[test]
    fn run_initial_pcl_docs_keep_c_stdout_markers_with_output_file() {
        let _guard = global_state_lock();
        let output_path = temp_path("initial-doc-output-file");
        let input_path = temp_path("initial-doc-output-input");
        let _ = std::fs::remove_file(&output_path);
        std::fs::write(&input_path, "p(a).\n").unwrap();
        let output_arg = output_path.to_string_lossy().into_owned();
        let input_arg = input_path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--lop-in",
                "-o",
                output_arg.as_str(),
                input_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!("{}XX\nXX\n", default_preprocessing_debug_line())
        );
        assert_eq!(
            std::fs::read_to_string(&output_path).unwrap(),
            format!(
                "     1 : :[++p(a)] : initial(\"{input_arg}\", at_line_1_column_1)\n\n% Pruning successful!\n% SZS status Unknown\n"
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&output_path).unwrap();
        std::fs::remove_file(&input_path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_lop_clause_files() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only");
        std::fs::write(&path, "p(a).\nq(a) <- p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", "--lop-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_reserves_internal_symbol_codes_for_temporary_bank() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-internal-symbol-codes");
        let mut input = String::new();
        for index in 1..=9 {
            writeln!(&mut input, "fof(ax{index}, axiom, p{index}(a{index})).").unwrap();
        }
        std::fs::write(&path, input).unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_honors_free_symbol_distinct_masks() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-free-symbols");
        std::fs::write(&path, "12(a).\n\"obj\"(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();

        let mut rejected_stdout = Vec::new();
        let mut rejected_stderr = Vec::new();
        let error = run(
            ["eprover", "--syntax-only", "--lop-in", path_arg.as_str()],
            &mut rejected_stdout,
            &mut rejected_stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Number cannot have argument list"));
        assert!(rejected_stdout.is_empty());
        assert!(rejected_stderr.is_empty());

        let mut accepted_stdout = Vec::new();
        let mut accepted_stderr = Vec::new();
        let status = run(
            [
                "eprover",
                "--syntax-only",
                "--lop-in",
                "--free-numbers",
                "--free-objects",
                path_arg.as_str(),
            ],
            &mut accepted_stdout,
            &mut accepted_stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(accepted_stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(accepted_stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:(human(X) => mortal(X))).\n\
             fof(reverse_rule, axiom, ![X]:(mortal(X) <= human(X))).\n\
             fof(equiv_rule, axiom, ![X]:(human(X) <=> person(X))).\n\
             fof(fact, axiom, human(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_distributed_implication_fragments() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof-distributed-implication");
        std::fs::write(
            &path,
            "fof(rule1, axiom, ![X]:((human(X) | robot(X)) => mortal(X))).\n\
             fof(rule2, axiom, ![X]:(human(X) => (mortal(X) & breathing(X)))).\n\
             fof(goal, conjecture, (p(a) => (q(a) & r(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_distributed_equivalence_fragments() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof-distributed-equivalence");
        std::fs::write(
            &path,
            "fof(eq1, axiom, ((p(a) & q(a)) <=> r(a))).\n\
             fof(eq2, axiom, ((p(a) | q(a)) <=> (r(a) & s(a)))).\n\
             fof(goal, conjecture, ((p(a) & q(a)) <=> r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_formula_equality_fragments() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof-formula-equality");
        std::fs::write(
            &path,
            "fof(eq, axiom, (((p(a) | q(a)) = r(a)))).\n\
             fof(ne, axiom, (((s(a) & t(a)) != u(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_axiom_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof-conjunction");
        std::fs::write(
            &path,
            "fof(axs, axiom, (human(socrates) & ![X]:(human(X) => mortal(X)))).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_conjecture_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof-conjecture-conjunction");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, (p(a) & q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_disjunction_fragments() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof-disjunction");
        std::fs::write(
            &path,
            "fof(either, axiom, (p(a) | q(a))).\n\
             fof(goal, conjecture, (p(a) | q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_unparenthesized_fof_connective_fragments() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof-unparenthesized-connectives");
        std::fs::write(
            &path,
            "fof(both, axiom, p(a) & q(a)).\n\
             fof(either, axiom, p(a) | q(a)).\n\
             fof(rule, axiom, p(a) & q(a) => r(a)).\n\
             fof(split, axiom, p(a) => q(a) & r(a)).\n\
             fof(eq, axiom, p(a) | q(a) <=> r(a)).\n\
             fof(goal, conjecture, p(a) & q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_parenthesized_negations() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-only-fof-negation");
        std::fs::write(
            &path,
            "fof(not_both, axiom, ~(p(a) & q(a))).\n\
             fof(not_either, conjecture, ~(p(a) | q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_resources_info_prints_c_shaped_footer() {
        let _guard = global_state_lock();
        let path = temp_path("resources-info");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--syntax-only",
                "--lop-in",
                "--resources-info",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("\n% Parsing successful!\n% SZS status Unknown\n"));
        assert!(printed.contains("\n% -------------------------------------------------\n"));
        assert!(printed.contains("% User time                : "));
        assert!(printed.contains("% System time              : "));
        assert!(printed.contains("% Total time               : "));
        assert!(printed.contains("% Maximum resident set size: "));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_resources_info_prints_preprocessing_time() {
        let _guard = global_state_lock();
        let path = temp_path("resources-info-proof");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--resources-info", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        let preproc = printed
            .find("% Preprocessing time       : ")
            .expect("preprocessing time line should be present");
        let result = printed
            .find("\n% No proof found!\n")
            .expect("result banner should be present");
        let footer = printed
            .find("\n% -------------------------------------------------\n")
            .expect("resource footer should be present");
        assert!(preproc < result);
        assert!(result < footer);
        assert!(printed.contains("% Total time               : "));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_prints_success_for_supported_clause_files() {
        let _guard = global_state_lock();
        let path = temp_path("prune-only");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--prune", "--lop-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let expected = format!(
            "{}XX\n     1 : :[++p(a)] : XX\ninitial(\"{path_arg}\", at_line_1_column_1)\n\n% Pruning successful!\n% SZS status Unknown\n",
            default_preprocessing_debug_line()
        );
        assert_eq!(String::from_utf8(stdout).unwrap(), expected);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_applies_clause_preprocessing_before_initial_docs() {
        let _guard = global_state_lock();
        let path = temp_path("prune-clause-preprocess");
        std::fs::write(
            &path,
            "cnf(drop, axiom, (a=a)).\n\
             cnf(keep, axiom, (p(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(!printed.contains(&format!("file('{path_arg}', drop)")));
        assert!(printed.contains(&format!("file('{path_arg}', keep)")));
        assert!(printed.contains("\n% Pruning successful!\n% SZS status Unknown\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_no_preprocessing_preserves_tautologies() {
        let _guard = global_state_lock();
        let path = temp_path("prune-no-clause-preprocess");
        std::fs::write(
            &path,
            "cnf(drop, axiom, (a=a)).\n\
             cnf(keep, axiom, (p(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--no-preprocessing",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains(&format!("file('{path_arg}', drop)")));
        assert!(printed.contains(&format!("file('{path_arg}', keep)")));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_applies_eq_definition_unfolding_before_initial_docs() {
        let _guard = global_state_lock();
        let path = temp_path("prune-eqdef-unfold");
        std::fs::write(
            &path,
            "cnf(def, axiom, (f(X)=X)).\n\
             cnf(use, axiom, (p(f(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("p(a)"));
        assert!(!printed.contains("p(f(a))"));
        assert!(!printed.contains(&format!("file('{path_arg}', def)")));
        assert!(printed.contains("\n% Pruning successful!\n% SZS status Unknown\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_no_eq_unfolding_preserves_eq_definitions() {
        let _guard = global_state_lock();
        let path = temp_path("prune-no-eqdef-unfold");
        std::fs::write(
            &path,
            "cnf(def, axiom, (f(X)=X)).\n\
             cnf(use, axiom, (p(f(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--no-eq-unfolding",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("p(f(a))"));
        assert!(printed.contains(&format!("file('{path_arg}', def)")));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_no_preprocessing_still_applies_eq_definition_unfolding() {
        let _guard = global_state_lock();
        let path = temp_path("prune-no-preproc-eqdef-unfold");
        std::fs::write(
            &path,
            "cnf(def, axiom, (f(X)=X)).\n\
             cnf(use, axiom, (p(f(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--no-preprocessing",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("p(a)"));
        assert!(!printed.contains("p(f(a))"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_accepts_supported_formula_bridge_input() {
        let _guard = global_state_lock();
        let path = temp_path("prune-formula-bridge");
        std::fs::write(
            &path,
            "fof(rule, axiom, (p(a)=>q(a))).\n\
             fof(fact, axiom, p(a)).\n\
             fof(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--prune", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains(&format!("axiom, (p(a)), file('{path_arg}', fact)).\n")));
        assert!(printed.contains(&format!(
            "axiom, (q(a)|~p(a)), file('{path_arg}', rule)).\n"
        )));
        assert!(printed.contains(&format!(
            "negated_conjecture, (~q(a)), file('{path_arg}', goal)).\n"
        )));
        assert!(printed.ends_with("\n% Pruning successful!\n% SZS status Unknown\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_auto_tstp_input_uses_tstp_initial_docs() {
        let _guard = global_state_lock();
        let path = temp_path("prune-auto-tstp");
        std::fs::write(&path, "cnf(c1, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--prune", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let expected = format!(
            "{}cnf(c_0_1, axiom, (p(a)), file('{path_arg}', c1)).\n\n% Pruning successful!\n% SZS status Unknown\n",
            default_preprocessing_debug_line()
        );
        assert_eq!(String::from_utf8(stdout).unwrap(), expected);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_applies_clause_relevance_before_initial_docs() {
        let _guard = global_state_lock();
        let path = temp_path("prune-relevance");
        std::fs::write(
            &path,
            "cnf(goal, negated_conjecture, (f(a)=a)).\n\
             cnf(rel, axiom, (g(a)=a)).\n\
             cnf(irr, axiom, (h(b)=b)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--rel-pruning-level=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains(&format!("file('{path_arg}', goal)")));
        assert!(!printed.contains(&format!("file('{path_arg}', rel)")));
        assert!(!printed.contains(&format!("file('{path_arg}', irr)")));
        assert!(!printed.contains("g(a)"));
        assert!(!printed.contains("h(b)"));
        assert!(printed.contains("\n% Pruning successful!\n% SZS status Unknown\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_applies_threshold_sine_before_initial_docs() {
        let _guard = global_state_lock();
        let path = temp_path("prune-threshold-sine");
        std::fs::write(&path, "p(a).\nq(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--lop-in",
                "--sine=Threshold(1)",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "% (lift_lambdas = 1, lambda_to_forall = 1,unroll_only_formulas = 1, sine = Threshold(1))\n\
% SinE strategy is Threshold(1)\n\n\
% Pruning successful!\n\
% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_applies_clause_gsine_before_initial_docs() {
        let _guard = global_state_lock();
        let path = temp_path("prune-gsine");
        std::fs::write(
            &path,
            "cnf(goal, negated_conjecture, (p(g))).\n\
             cnf(link, axiom, (g=h)).\n\
             cnf(far, axiom, (h=k)).\n\
             cnf(irr, axiom, (r(u))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--sine=GSinE(CountTerms,,false,10.0,,2,10,1.0)",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains("% SinE strategy is GSinE(CountTerms,,false,10.0,,2,10,1.0)\n"));
        assert!(printed.contains(&format!("file('{path_arg}', goal)")));
        assert!(printed.contains(&format!("file('{path_arg}', link)")));
        assert!(printed.contains(&format!("file('{path_arg}', far)")));
        assert!(!printed.contains(&format!("file('{path_arg}', irr)")));
        assert!(!printed.contains("r(u)"));
        assert!(printed.contains("\n% Pruning successful!\n% SZS status Unknown\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_applies_bce_before_initial_docs() {
        let _guard = global_state_lock();
        let path = temp_path("prune-bce");
        std::fs::write(
            &path,
            "cnf(left, axiom, (p(a)|q(a))).\n\
             cnf(right, axiom, (~p(a)|q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--prune", "--bce=true", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}% BCE start: 2\n% BCE eliminated: 2.\n\n% Pruning successful!\n% SZS status Unknown\n",
                default_preprocessing_debug_line()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_applies_goal_defs_before_initial_docs() {
        let _guard = global_state_lock();
        let path = temp_path("prune-goal-defs");
        std::fs::write(&path, "cnf(goal, negated_conjecture, (f(a)=a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--goal-defs=All",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains(&format!(
            "cnf(c_0_1, negated_conjecture, (f(a)=a), file('{path_arg}', goal)).\n"
        )));
        assert!(printed.contains("cnf(c_0_2, plain, (f(a)=edef"));
        assert!(printed.contains("\n% Pruning successful!\n% SZS status Unknown\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_lambda_def_clears_represented_clause_axioms() {
        let _guard = global_state_lock();
        let path = temp_path("prune-lambda-def-clauses");
        std::fs::write(
            &path,
            "cnf(goal, negated_conjecture, (g=g)).\n\
             cnf(ax, axiom, (a=a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--sine=LambdaDef",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "% (lift_lambdas = 1, lambda_to_forall = 1,unroll_only_formulas = 1, sine = LambdaDef)\n\
% SinE strategy is LambdaDef\n\n\
% Pruning successful!\n\
% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_prune_only_lambda_def_keeps_lowered_formula_goals_and_hypotheses() {
        let _guard = global_state_lock();
        let path = temp_path("prune-lambda-def-formulas");
        std::fs::write(
            &path,
            "fof(fact, axiom, p(a)).\n\
             fof(hyp, hypothesis, h(a)).\n\
             fof(goal, conjecture, g(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--prune",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--sine=LambdaDef",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains("% SinE strategy is LambdaDef\n"));
        assert!(!printed.contains(&format!("file('{path_arg}', fact)")));
        assert!(printed.contains(&format!("file('{path_arg}', hyp)")));
        assert!(printed.contains(&format!("file('{path_arg}', goal)")));
        assert!(printed.contains("hypothesis"));
        assert!(printed.contains("negated_conjecture"));
        assert!(printed.contains("\n% Pruning successful!\n% SZS status Unknown\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_finds_empty_clause_from_false_lop_clause() {
        let _guard = global_state_lock();
        let path = temp_path("proof-found");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Proof found!\n% SZS status Unsatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_prints_initial_ac_scan_status() {
        let _guard = global_state_lock();
        let path = temp_path("proof-initial-ac-status");
        std::fs::write(&path, "f(X,Y)=f(Y,X).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--cnf", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        let ac_status = "% Scanning for AC axioms\n% f is commutative\n% AC handling enabled\n";
        assert!(printed.contains(ac_status));
        assert!(
            printed.find(ac_status).unwrap() < printed.find("\n% CNFization successful!").unwrap()
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_prints_sat_check_unsat_comment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-sat-check-comment");
        std::fs::write(&path, "a=b.\na!=b.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--satcheck=GlobalMin",
                "--satcheck-proc-interval=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}% SatCheck found unsatisfiable ground set\n\n% Proof found!\n% SZS status Unsatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_theorem_for_fof_conjecture_refutation() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture");
        std::fs::write(
            &path,
            "fof(fact, axiom, a=b).\n\
             fof(goal, conjecture, a=b).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_auto_mode_selects_generated_preprocessing_and_search_strategies() {
        let _guard = global_state_lock();
        let path = temp_path("proof-auto-generated-strategy");
        std::fs::write(&path, "cnf(a, axiom, ($false)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--auto", "--tstp-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let output = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(output.contains("% Preprocessing class: FSSSSMSSSSSNFFN.\n"));
        assert!(output.contains("% Configuration: G-E--_302_C18_F1_URBAN_RG_S04BN\n"));
        assert!(output.contains("% No SInE strategy applied\n"));
        assert!(output.contains("% Search class: FUHPF-FFSF00-SFFFFFNN\n"));
        assert!(output.contains("% Configuration: SAT001_MinMin_p005000_rr_RG\n"));
        assert!(output.contains("% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_auto_cnf_skips_generated_search_strategy_like_c() {
        let _guard = global_state_lock();
        let path = temp_path("proof-auto-cnf");
        std::fs::write(&path, "cnf(a, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--auto", "--cnf", "--tstp-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let output = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(output.contains("% Preprocessing class: FSSSSMSSSSSNFFN.\n"));
        assert!(output.contains("% Configuration: G-E--_302_C18_F1_URBAN_RG_S04BN\n"));
        assert!(!output.contains("% Search class:"));
        assert!(output.contains("% CNFization successful!\n% SZS status Unknown\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_auto_mode_replays_explicit_sine_after_generated_strategy() {
        let _guard = global_state_lock();
        let path = temp_path("proof-auto-sine");
        std::fs::write(&path, "cnf(a, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--auto",
                "--sine=NoSInE",
                "--cnf",
                "--tstp-in",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let output = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(output.contains(
            "% (lift_lambdas = 1, lambda_to_forall = 1,unroll_only_formulas = 1, sine = NoSInE)\n"
        ));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_auto_schedule_uses_generated_schedules_in_process() {
        let _guard = global_state_lock();
        let path = temp_path("proof-auto-schedule-in-process");
        std::fs::write(&path, "cnf(a, axiom, ($false)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--auto-schedule=1",
                "--tstp-in",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let output = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(output.contains("% Preprocessing class: FSSSSMSSSSSNFFN.\n"));
        assert!(output.contains("% Scheduled 1 strats onto 1 cores with "));
        assert!(output.contains("% No SInE strategy applied\n"));
        assert!(output.contains("% Search class: FUHPF-FFSF00-SFFFFFNN\n"));
        assert!(output.contains("% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(!output.contains("strategy scheduling process execution is not ported yet"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_theorem_for_old_tptp_input_formula_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("proof-tptp-input-formula-conjecture");
        std::fs::write(
            &path,
            "input_formula(fact, axiom, p(a)).\n\
             input_formula(goal, conjecture, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_preserves_old_tptp_right_recursive_binary_parse() {
        let _guard = global_state_lock();
        let path = temp_path("proof-tptp-input-formula-right-recursive-binary");
        std::fs::write(
            &path,
            "input_formula(rule, axiom, p(a) | q(a) => r(a)).\n\
             input_formula(fact, axiom, p(a)).\n\
             input_formula(goal, conjecture, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_old_tptp_formula_equality_with_compound_left() {
        let _guard = global_state_lock();
        let path = temp_path("proof-tptp-input-formula-equality-compound-left");
        std::fs::write(
            &path,
            "input_formula(rule, axiom, ((p(a) | q(a)) = r(a))).\n\
             input_formula(fact, axiom, p(a)).\n\
             input_formula(goal, conjecture, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_preserves_old_tptp_quantifier_element_scope() {
        let _guard = global_state_lock();
        let path = temp_path("proof-tptp-input-formula-quantifier-element-scope");
        std::fs::write(
            &path,
            "input_formula(rule, axiom, ![X]:p(X) => q(a)).\n\
             input_formula(fact, axiom, p(a)).\n\
             input_formula(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_theorem_for_fof_true_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-true-conjecture");
        std::fs::write(&path, "fof(goal, conjecture, $true).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_theorem_for_tff_formula_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("proof-tff-formula-conjecture");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             tff(fact, axiom, p(a)).\n\
             tff(goal, conjecture, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_theorem_for_tcf_formula_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("proof-tcf-formula-conjecture");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             tcf(fact, axiom, ![X: person]:p(X)).\n\
             tcf(goal, conjecture, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_theorem_for_tff_typed_universal_quantifier() {
        let _guard = global_state_lock();
        let path = temp_path("proof-tff-typed-universal");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             tff(rule, axiom, ![X: person]:p(X)).\n\
             tff(goal, conjecture, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_existential_conjecture_atom() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-existential-conjecture");
        std::fs::write(
            &path,
            "fof(fact, axiom, p(a)).\n\
             fof(goal, conjecture, ?[X]:p(X)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_positive_existential_atom() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-positive-existential");
        std::fs::write(
            &path,
            "fof(fact, axiom, ?[X]:p(X)).\n\
             fof(goal, conjecture, ?[Y]:p(Y)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_saturates_same_name_shadowed_fof_existential() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-shadowed-existential");
        std::fs::write(
            &path,
            "fof(test1, axiom, ![X,Y]:((p(X,Y)&?[X]:(q(Y,X)=>q(X,Y))))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status Satisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_parenthesized_existential_conjunction() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-parenthesized-existential-conjunction");
        std::fs::write(
            &path,
            "fof(fact, axiom, ?[X]:(p(X)&q(X))).\n\
             fof(goal, conjecture, ?[Y]:q(Y)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_nested_fof_existentials() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-nested-existentials");
        std::fs::write(
            &path,
            "fof(fact, axiom, ?[X]:(?[Y]:p(X,Y))).\n\
             fof(goal, conjecture, ?[Z]:(?[W]:p(Z,W))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_distinct_formula() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-distinct");
        std::fs::write(
            &path,
            "fof(distinct, axiom, $distinct(a,b)).\n\
             fof(equal, axiom, a=b).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjunction_with_existential_conjunct() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjunction-existential-conjunct");
        std::fs::write(
            &path,
            "fof(fact, axiom, ?[X]:p(X) & q(a)).\n\
             fof(no_p, axiom, ![Y]:~p(Y)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_conjunction_with_existential_conjunct() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-conjunction-existential-conjunct");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, ?[X]:p(X) & q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_universal_positive_existential_atom() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-universal-positive-existential");
        std::fs::write(
            &path,
            "fof(fact, axiom, ![Y]: ?[X]:p(X,Y)).\n\
             fof(goal, conjecture, ?[X]:p(X,a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_universal_existential_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-universal-existential-conjecture");
        std::fs::write(
            &path,
            "fof(fact, axiom, ![Y]:p(a,Y)).\n\
             fof(goal, conjecture, ![Y]: ?[X]:p(X,Y)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_existential_universal_scope() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-existential-universal-scope");
        std::fs::write(
            &path,
            "fof(fact, axiom, ?[X]:(![Y]:p(X,Y))).\n\
             fof(goal, conjecture, ?[X]:p(X,a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_existential_negated_universal_body() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-existential-negated-universal-body");
        std::fs::write(
            &path,
            "fof(negated_all, axiom, ?[X]:(~![Y]:p(X,Y))).\n\
             fof(all_p, axiom, ![A,B]:p(A,B)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_unparenthesized_fof_existential_universal_scope() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-existential-universal-scope");
        std::fs::write(
            &path,
            "fof(fact, axiom, ?[X]:![Y]:p(X,Y)).\n\
             fof(goal, conjecture, ?[X]:p(X,a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_implication_with_existential_consequent() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-implication-existential-consequent");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![Y]:(p(Y)=>?[X]:q(X,Y))).\n\
             fof(fact, axiom, p(a)).\n\
             fof(goal, conjecture, ?[X]:q(X,a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_implication_with_existential_antecedent() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-implication-existential-antecedent");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![Y]:((?[X]:p(X,Y))=>q(Y))).\n\
             fof(fact, axiom, p(b,a)).\n\
             fof(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_equivalence_with_existential_from_left() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-equivalence-existential-left");
        std::fs::write(
            &path,
            "fof(eq, axiom, ![Y]:(p(Y)<=>?[X]:q(X,Y))).\n\
             fof(fact, axiom, p(a)).\n\
             fof(goal, conjecture, ?[X]:q(X,a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_equivalence_with_existential_from_right() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-equivalence-existential-right");
        std::fs::write(
            &path,
            "fof(eq, axiom, ![Y]:(p(Y)<=>?[X]:q(X,Y))).\n\
             fof(fact, axiom, q(b,a)).\n\
             fof(goal, conjecture, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_contradictory_axioms_when_fof_conjecture_is_unused() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unused-conjecture");
        std::fs::write(
            &path,
            "fof(pos, axiom, p(a)).\n\
             fof(neg, axiom, ~p(a)).\n\
             fof(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status ContradictoryAxioms\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_keeps_explicit_fof_negated_conjecture_unsatisfiable() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-explicit-negated-conjecture");
        std::fs::write(
            &path,
            "fof(fact, axiom, p(a)).\n\
             fof(goal, negated_conjecture, ~p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_predicate_horn_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-predicate-horn");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:(human(X) => mortal(X))).\n\
             fof(fact, axiom, human(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_grouped_horn_antecedent_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-grouped-horn-antecedent");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:((human(X) & wise(X)) => mortal(X))).\n\
             fof(human, axiom, human(socrates)).\n\
             fof(wise, axiom, wise(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_grouped_implication_consequent_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-grouped-implication-consequent");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:((human(X) & wise(X)) => (mortal(X) | famous(X)))).\n\
             fof(human, axiom, human(socrates)).\n\
             fof(wise, axiom, wise(socrates)).\n\
             fof(not_famous, axiom, ~famous(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_disjunctive_implication_antecedent_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-disjunctive-implication-antecedent");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:((human(X) | robot(X)) => mortal(X))).\n\
             fof(fact, axiom, human(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_unparenthesized_fof_implication_antecedent_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-implication-antecedent");
        std::fs::write(
            &path,
            "fof(rule, axiom, p(a) & q(a) => r(a)).\n\
             fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_respects_unparenthesized_fof_implication_antecedent_precedence() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-implication-antecedent-precedence");
        std::fs::write(
            &path,
            "fof(rule, axiom, p(a) & q(a) => r(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjunctive_implication_consequent_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjunctive-implication-consequent");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:(human(X) => (mortal(X) & breathing(X)))).\n\
             fof(fact, axiom, human(socrates)).\n\
             fof(goal, conjecture, breathing(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_unparenthesized_fof_implication_consequent_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-implication-consequent");
        std::fs::write(
            &path,
            "fof(rule, axiom, p(a) => q(a) & r(a)).\n\
             fof(p, axiom, p(a)).\n\
             fof(goal, conjecture, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_reverse_implication_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-reverse-implication");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:(mortal(X) <= human(X))).\n\
             fof(fact, axiom, human(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_grouped_reverse_implication_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-grouped-reverse-implication");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:(mortal(X) <= (human(X) & wise(X)))).\n\
             fof(human, axiom, human(socrates)).\n\
             fof(wise, axiom, wise(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_grouped_reverse_consequent_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-grouped-reverse-consequent");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:((mortal(X) | famous(X)) <= (human(X) & wise(X)))).\n\
             fof(human, axiom, human(socrates)).\n\
             fof(wise, axiom, wise(socrates)).\n\
             fof(not_famous, axiom, ~famous(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_equivalence_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-equivalence");
        std::fs::write(
            &path,
            "fof(rule, axiom, ![X]:(human(X) <=> mortal(X))).\n\
             fof(fact, axiom, human(socrates)).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_formula_equality_with_compound_left() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-formula-equality-compound-left");
        std::fs::write(
            &path,
            "fof(rule, axiom, (((p(a) | q(a)) = r(a)))).\n\
             fof(fact, axiom, p(a)).\n\
             fof(goal, conjecture, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_formula_disequality_with_compound_left() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-formula-disequality-compound-left");
        std::fs::write(
            &path,
            "fof(diff, axiom, (((p(a) | q(a)) != r(a)))).\n\
             fof(p, axiom, p(a)).\n\
             fof(r, axiom, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_negated_equivalence_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-negated-equivalence");
        std::fs::write(
            &path,
            "fof(not_same, axiom, ~(p(a) <=> q(a))).\n\
             fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_xor_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-xor");
        std::fs::write(
            &path,
            "fof(xor, axiom, (p(a) <~> q(a))).\n\
             fof(p, axiom, p(a)).\n\
             fof(goal, conjecture, ~q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_xor_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-xor");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(goal, conjecture, (p(a) <~> q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_nand_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-nand");
        std::fs::write(
            &path,
            "fof(nand, axiom, (p(a) ~& q(a))).\n\
             fof(p, axiom, p(a)).\n\
             fof(goal, conjecture, ~q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_nand_with_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-nand-existential");
        std::fs::write(
            &path,
            "fof(nand, axiom, (p(a) ~& ?[X]:q(X))).\n\
             fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(b)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_nor_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-nor");
        std::fs::write(
            &path,
            "fof(not_p, axiom, ~p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(goal, conjecture, (p(a) ~| q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_nor_with_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-nor-existential");
        std::fs::write(
            &path,
            "fof(nor, axiom, (p(a) ~| ?[X]:q(X))).\n\
             fof(p, axiom, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_equivalence_with_conjunctive_left_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-equivalence-conjunctive-left");
        std::fs::write(
            &path,
            "fof(eq, axiom, ((p(a) & q(a)) <=> r(a))).\n\
             fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_equivalence_with_conjunctive_right_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-equivalence-conjunctive-right");
        std::fs::write(
            &path,
            "fof(eq, axiom, (p(a) <=> (q(a) & r(a)))).\n\
             fof(p, axiom, p(a)).\n\
             fof(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_negated_compound_equivalence_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-negated-compound-equivalence");
        std::fs::write(
            &path,
            "fof(not_eq, axiom, ~((p(a) & q(a)) <=> r(a))).\n\
             fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(r, axiom, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_axiom_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjunction");
        std::fs::write(
            &path,
            "fof(axs, axiom, (![X]:(human(X) => mortal(X)) & human(socrates))).\n\
             fof(goal, conjecture, mortal(socrates)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_unparenthesized_fof_axiom_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-conjunction");
        std::fs::write(
            &path,
            "fof(axs, axiom, p(a) & q(a)).\n\
             fof(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-conjunction");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, (p(a) & q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_conjunction_with_implication_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-conjunction-implication");
        std::fs::write(
            &path,
            "fof(q, axiom, q(a)).\n\
             fof(r, axiom, r(a)).\n\
             fof(goal, conjecture, ((p(a) => q(a)) & r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_false_fof_conjecture_conjunction_with_implication_as_counter_satisfiable(
    ) {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-conjunction-implication-counter-satisfiable");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(r, axiom, r(a)).\n\
             fof(goal, conjecture, ((p(a) => q(a)) & r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_axiom_disjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-axiom-disjunction");
        std::fs::write(
            &path,
            "fof(either, axiom, (p(a) | q(a))).\n\
             fof(not_p, axiom, ~p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_unparenthesized_fof_axiom_disjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-disjunction");
        std::fs::write(
            &path,
            "fof(either, axiom, p(a) | q(a)).\n\
             fof(not_p, axiom, ~p(a)).\n\
             fof(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_unparenthesized_fof_disjunction_right_conjunction() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-disjunction-right-conjunction");
        std::fs::write(
            &path,
            "fof(either, axiom, p(a) | q(a) & r(a)).\n\
             fof(not_p, axiom, ~p(a)).\n\
             fof(goal, conjecture, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_disjunction_with_left_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-disjunction-left-conjunction");
        std::fs::write(
            &path,
            "fof(split, axiom, ((p(a) & q(a)) | r(a))).\n\
             fof(not_p, axiom, ~p(a)).\n\
             fof(not_r, axiom, ~r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_disjunction_with_right_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-disjunction-right-conjunction");
        std::fs::write(
            &path,
            "fof(split, axiom, (p(a) | (q(a) & r(a)))).\n\
             fof(not_p, axiom, ~p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_disjunction_with_existential_disjunct() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-disjunction-existential-disjunct");
        std::fs::write(
            &path,
            "fof(either, axiom, ?[X]:p(X) | q(a)).\n\
             fof(no_p, axiom, ![Y]:~p(Y)).\n\
             fof(no_q, axiom, ~q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_disjunction_with_existential_conjunctive_operand() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-disjunction-existential-conjunction");
        std::fs::write(
            &path,
            "fof(split, axiom, ((p(a) & ?[X]:q(X)) | r(a))).\n\
             fof(no_q, axiom, ![Y]:~q(Y)).\n\
             fof(no_r, axiom, ~r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_disjunction_with_equivalence_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-disjunction-equivalence");
        std::fs::write(
            &path,
            "fof(eq_or_r, axiom, ((p(a) <=> q(a)) | r(a))).\n\
             fof(p, axiom, p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(not_r, axiom, ~r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_conjunction_with_disjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-conjunction-disjunction");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(r, axiom, r(a)).\n\
             fof(goal, conjecture, ((p(a) | q(a)) & r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_disjunction_with_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-disjunction-conjunction");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(goal, conjecture, (p(a) | (q(a) & r(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_false_fof_conjecture_disjunction_with_conjunction_as_counter_satisfiable(
    ) {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-disjunction-conjunction-counter-satisfiable");
        std::fs::write(
            &path,
            "fof(not_p, axiom, ~p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(goal, conjecture, (p(a) | (q(a) & r(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_disjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-disjunction");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(goal, conjecture, (p(a) | q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_unparenthesized_fof_conjecture_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-conjecture-conjunction");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, p(a) & q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_false_unparenthesized_fof_disjunction_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-unparenthesized-conjecture-disjunction-counter");
        std::fs::write(
            &path,
            "fof(not_p, axiom, ~p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(goal, conjecture, p(a) | q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_implication_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-implication");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, (p(a) => q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_implication_with_conjunctive_consequent() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-implication-conjunctive-consequent");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(r, axiom, r(a)).\n\
             fof(goal, conjecture, (p(a) => (q(a) & r(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_false_fof_conjecture_implication_with_conjunctive_consequent_as_counter_satisfiable(
    ) {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-implication-conjunctive-consequent-counter");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(goal, conjecture, (p(a) => (q(a) & r(a)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_implication_with_disjunctive_antecedent() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-implication-disjunctive-antecedent");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(r, axiom, r(a)).\n\
             fof(goal, conjecture, ((p(a) | q(a)) => r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_grouped_conjecture_reverse_implication_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-grouped-conjecture-reverse-implication");
        std::fs::write(
            &path,
            "fof(human, axiom, human(socrates)).\n\
             fof(wise, axiom, wise(socrates)).\n\
             fof(mortal, axiom, mortal(socrates)).\n\
             fof(goal, conjecture, ((mortal(socrates) | famous(socrates)) <= (human(socrates) & wise(socrates)))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_false_fof_conjecture_implication_as_counter_satisfiable() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-implication-counter-satisfiable");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(goal, conjecture, (p(a) => q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_equivalence_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-equivalence");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(goal, conjecture, (p(a) <=> q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_conjunction_with_equivalence_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-conjunction-equivalence");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(r, axiom, r(a)).\n\
             fof(goal, conjecture, ((p(a) <=> q(a)) & r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_false_fof_conjecture_equivalence_as_counter_satisfiable() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-equivalence-counter-satisfiable");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(goal, conjecture, (p(a) <=> q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_conjecture_compound_equivalence_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-compound-equivalence");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(r, axiom, r(a)).\n\
             fof(goal, conjecture, ((p(a) & q(a)) <=> r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_false_fof_conjecture_compound_equivalence_as_counter_satisfiable() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-conjecture-compound-equivalence-counter-satisfiable");
        std::fs::write(
            &path,
            "fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n\
             fof(not_r, axiom, ~r(a)).\n\
             fof(goal, conjecture, ((p(a) & q(a)) <=> r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_negated_conjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-negated-conjunction");
        std::fs::write(
            &path,
            "fof(not_both, axiom, ~(p(a) & q(a))).\n\
             fof(p, axiom, p(a)).\n\
             fof(q, axiom, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_supported_fof_negated_conjecture_disjunction_fragment() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-negated-conjecture-disjunction");
        std::fs::write(
            &path,
            "fof(not_p, axiom, ~p(a)).\n\
             fof(not_q, axiom, ~q(a)).\n\
             fof(goal, conjecture, ~(p(a) | q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_follows_tstp_include_without_selector() {
        let _guard = global_state_lock();
        let include_path = temp_path("proof-fof-include-all-inc");
        let path = temp_path("proof-fof-include-all-main");
        std::fs::write(&include_path, "fof(fact, axiom, p(a)).\n").unwrap();
        let include_arg = include_path.to_string_lossy().into_owned();
        std::fs::write(
            &path,
            format!("include('{include_arg}').\nfof(goal, conjecture, p(a)).\n"),
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&include_path).unwrap();
    }

    #[test]
    fn run_proof_search_resolves_tstp_include_through_tptp_env() {
        let _guard = global_state_lock();
        let tptp_root = temp_path("proof-fof-include-tptp-root");
        _ = std::fs::remove_dir_all(&tptp_root);
        let axioms_dir = tptp_root.join("Axioms");
        std::fs::create_dir_all(&axioms_dir).unwrap();
        std::fs::write(axioms_dir.join("ENV001.ax"), "fof(fact, axiom, p(a)).\n").unwrap();
        let _env = set_env_var("TPTP", &tptp_root.to_string_lossy());
        let path = temp_path("proof-fof-include-tptp-main");
        std::fs::write(
            &path,
            "include('Axioms/ENV001.ax').\nfof(goal, conjecture, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir_all(&tptp_root).unwrap();
    }

    #[test]
    fn run_proof_search_bounded_boo_include_uses_first_order_index_state() {
        let _guard = global_state_lock();
        let tptp_root = std::env::current_dir()
            .unwrap()
            .join("eprover")
            .join("EXAMPLE_PROBLEMS")
            .join("TPTP");
        let _env = set_env_var("TPTP", &tptp_root.to_string_lossy());
        let path = tptp_root.join("BOO006-1.p");
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                path_arg.as_str(),
                "--auto",
                "--silent",
                "--processed-clauses-limit=5",
                "--memory-limit=2048",
                "--detsort-rw",
                "--detsort-new",
                "--proof-object=1",
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed
            .contains("\n% Failure: User resource limit exceeded!\n% SZS status ResourceOut\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn run_proof_search_honors_tstp_include_name_selector() {
        let _guard = global_state_lock();
        let include_path = temp_path("proof-fof-include-selected-inc");
        let path = temp_path("proof-fof-include-selected-main");
        std::fs::write(
            &include_path,
            "fof(selected, axiom, p(a)).\n\
             fof(unselected, axiom, q(a)).\n",
        )
        .unwrap();
        let include_arg = include_path.to_string_lossy().into_owned();
        std::fs::write(
            &path,
            format!("include('{include_arg}',[selected]).\nfof(goal, conjecture, p(a)).\n"),
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&include_path).unwrap();
    }

    #[test]
    fn run_proof_search_filters_unselected_tstp_include_entries() {
        let _guard = global_state_lock();
        let include_path = temp_path("proof-fof-include-filter-inc");
        let path = temp_path("proof-fof-include-filter-main");
        std::fs::write(
            &include_path,
            "fof(selected, axiom, p(a)).\n\
             fof(unselected, axiom, q(a)).\n",
        )
        .unwrap();
        let include_arg = include_path.to_string_lossy().into_owned();
        std::fs::write(
            &path,
            format!("include('{include_arg}',[selected]).\nfof(goal, conjecture, q(a)).\n"),
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&include_path).unwrap();
    }

    #[test]
    fn run_proof_search_honors_old_tptp_input_formula_include_selector() {
        let _guard = global_state_lock();
        let include_path = temp_path("proof-tptp-input-formula-include-selected-inc");
        let path = temp_path("proof-tptp-input-formula-include-selected-main");
        std::fs::write(
            &include_path,
            "input_formula(selected, axiom, p(a)).\n\
             input_formula(unselected, axiom, q(a)).\n",
        )
        .unwrap();
        let include_arg = include_path.to_string_lossy().into_owned();
        std::fs::write(
            &path,
            format!(
                "include('{include_arg}',[selected]).\ninput_formula(goal, conjecture, p(a)).\n"
            ),
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--tptp-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed.contains("\n% Proof found!\n% SZS status Theorem\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&include_path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_missing_tstp_include_selector() {
        let _guard = global_state_lock();
        let include_path = temp_path("proof-fof-include-missing-selector-inc");
        let path = temp_path("proof-fof-include-missing-selector-main");
        std::fs::write(&include_path, "fof(present, axiom, p(a)).\n").unwrap();
        let include_arg = include_path.to_string_lossy().into_owned();
        std::fs::write(
            &path,
            format!("include('{include_arg}',[missing]).\nfof(goal, conjecture, p(a)).\n"),
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::INPUT_SEMANTIC_ERROR);
        assert!(error.message().contains(
            "\"include\" statement cannot find the following requested clauses/formulae"
        ));
        assert!(error.message().contains("missing"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&include_path).unwrap();
    }

    #[test]
    fn run_proof_search_prints_answer_tuple_before_final_banner() {
        let _guard = global_state_lock();
        let path = temp_path("proof-answer");
        std::fs::write(&path, "$answer(ans(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}% SZS status Theorem\n% SZS answers Tuple [[a]|_]\n\n% Proof found!\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_annotates_existential_question_with_answer_literal() {
        let _guard = global_state_lock();
        let path = temp_path("proof-question-answer");
        std::fs::write(
            &path,
            "fof(hume, axiom, philosopher(hume)).\n\
             fof(phil_wise, axiom, ![X]:(philosopher(X) => wise(X))).\n\
             fof(is_there_wisdom, question, ?[X]:wise(X)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with(&default_preprocessing_debug_line()));
        assert!(printed
            .contains("% SZS status Theorem\n% SZS answers Tuple [[hume]|_]\n\n% Proof found!\n"));
        assert!(!printed.contains("% No proof found!"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_saturated_clause_set_as_satisfiable() {
        let _guard = global_state_lock();
        let path = temp_path("proof-saturated");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status Satisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_saturated_fof_conjecture_as_counter_satisfiable() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-counter-satisfiable");
        std::fs::write(
            &path,
            "fof(fact, axiom, p(a)).\n\
             fof(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status CounterSatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_saturated_explicit_negated_conjecture_as_satisfiable() {
        let _guard = global_state_lock();
        let path = temp_path("proof-fof-negated-conjecture-satisfiable");
        std::fs::write(&path, "fof(goal, negated_conjecture, ~q(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(["eprover", path_arg.as_str()], &mut stdout, &mut stderr).unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% No proof found!\n% SZS status Satisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_no_generation_as_restricted_calculus() {
        let _guard = global_state_lock();
        let path = temp_path("proof-restricted-calculus");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--no-generation", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Clause set closed under restricted calculus!\n% SZS status GaveUp\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_unimplemented_interpreted_symbol_as_restricted_calculus() {
        let _guard = global_state_lock();
        let path = temp_path("proof-interpreted-calculus");
        std::fs::write(&path, "$foo.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Clause set closed under restricted calculus!\n% SZS status GaveUp\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_assumed_incompleteness_as_gave_up() {
        let _guard = global_state_lock();
        let path = temp_path("proof-assume-incomplete");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--assume-incompleteness",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Failure: Out of unprocessed clauses!\n% SZS status GaveUp\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_prints_requested_saturated_sections() {
        let _guard = global_state_lock();
        let path = temp_path("proof-print-saturated");
        std::fs::write(&path, "a=b.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--no-generation",
                "--print-saturated=e",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.starts_with(&format!(
            "{}\n% Clause set closed under restricted calculus!\n% SZS status GaveUp\n",
            default_proof_search_prefix()
        )));
        assert!(printed.contains("% Processed positive unit clauses:\n"));
        assert!(printed.lines().any(|line| line.ends_with("<- .")));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_print_sat_info_comments_saturated_clauses() {
        let _guard = global_state_lock();
        let path = temp_path("proof-print-sat-info");
        std::fs::write(&path, "a=b.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--no-generation",
                "--print-saturated=e",
                "--print-sat-info",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.contains("% Processed positive unit clauses:\n"));
        assert!(
            printed.lines().any(|line| line.contains("<- . % info(")),
            "{printed}"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_closes_with_configured_global_pm_indexes() {
        let _guard = global_state_lock();
        let path = temp_path("proof-global-pm-indexes");
        std::fs::write(&path, "a=b.\nf(a)!=f(b).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--pm-from-index=FP1",
                "--pm-into-index=FP1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Proof found!\n% SZS status Unsatisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_prints_saturated_success_clause_like_c() {
        let _guard = global_state_lock();
        let path = temp_path("proof-print-saturated-success");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--print-saturated=e",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.starts_with(&format!(
            "{}\n% Proof found!\n% SZS status Unsatisfiable\n",
            default_proof_search_prefix()
        )));
        assert!(printed.contains(
            "% Saturated system contains the empty clause:\n <- .\n\n% Processed positive unit clauses:\n"
        ));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_prints_saturated_success_clause_in_selected_output_format() {
        let _guard = global_state_lock();
        let path = temp_path("proof-print-saturated-success-tstp");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--tstp-out",
                "--print-saturated=e",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.starts_with(&format!(
            "{}\n% Proof found!\n% SZS status Unsatisfiable\n",
            default_proof_search_prefix()
        )));
        assert!(printed.contains("% Saturated system contains the empty clause:\ncnf("));
        assert!(printed.contains(", ($false)).\n\n% Processed positive unit clauses:\n"));
        assert!(!printed.contains("\n <- .\n\n% Processed positive unit clauses:\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_saturated_output_honors_equation_print_options() {
        let _guard = global_state_lock();
        let path = temp_path("proof-print-saturated-eqn-options");
        std::fs::write(&path, "a=b.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--no-generation",
                "--eqn-no-infix",
                "--print-saturated=e",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.contains("% Processed positive unit clauses:\n"));
        assert!(printed.contains("equal(a, b) <- .\n"));
        assert!(!printed.contains("b=a <- .\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_prints_statistics_when_requested() {
        let _guard = global_state_lock();
        let path = temp_path("proof-print-statistics");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--print-statistics",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.starts_with(&format!(
            "{}\n% No proof found!\n% SZS status Satisfiable\n",
            default_proof_search_prefix()
        )));
        assert!(printed.contains("% Parsed axioms                        : 1\n"));
        assert!(printed.contains("% Initial clauses in saturation        : 1\n"));
        assert!(printed.contains("% Processed clauses                    : 1\n"));
        assert!(printed.contains("% Termbank termtop insertions          : "));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_statistics_count_clause_relevance_pruning() {
        let _guard = global_state_lock();
        let path = temp_path("proof-relevance-statistics");
        std::fs::write(
            &path,
            "cnf(goal, negated_conjecture, (f(a)=a)).\n\
             cnf(rel, axiom, (g(a)=a)).\n\
             cnf(irr, axiom, (h(b)=b)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--tstp-in",
                "--tstp-out",
                "--no-generation",
                "--print-statistics",
                "--rel-pruning-level=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.contains("% Parsed axioms                        : 3\n"));
        assert!(printed.contains("% Removed by relevancy pruning/SinE    : 2\n"));
        assert!(printed.contains("% Initial clauses                      : 1\n"));
        assert!(printed.contains("\n% Clause set closed under restricted calculus!\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_statistics_count_threshold_sine_pruning() {
        let _guard = global_state_lock();
        let path = temp_path("proof-threshold-sine-statistics");
        std::fs::write(&path, "p(a).\nq(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--print-statistics",
                "--sine=Threshold(1)",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.contains("% SinE strategy is Threshold(1)\n"));
        assert!(printed.contains("% Parsed axioms                        : 2\n"));
        assert!(printed.contains("% Removed by relevancy pruning/SinE    : 2\n"));
        assert!(printed.contains("% Initial clauses                      : 0\n"));
        assert!(printed.contains("\n% Failure: Out of unprocessed clauses!\n% SZS status GaveUp\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_statistics_count_clause_gsine_pruning() {
        let _guard = global_state_lock();
        let path = temp_path("proof-gsine-statistics");
        std::fs::write(
            &path,
            "cnf(goal, negated_conjecture, (g=g)).\n\
             cnf(link, axiom, (g=h)).\n\
             cnf(far, axiom, (h=k)).\n\
             cnf(irr, axiom, (u=u)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--tstp-in",
                "--tstp-out",
                "--no-generation",
                "--print-statistics",
                "--sine=GSinE(CountTerms,,false,10.0,,2,10,1.0)",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.contains("% SinE strategy is GSinE(CountTerms,,false,10.0,,2,10,1.0)\n"));
        assert!(printed.contains("% Parsed axioms                        : 4\n"));
        assert!(printed.contains("% Removed by relevancy pruning/SinE    : 1\n"));
        assert!(printed.contains("% Initial clauses                      : 3\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_applies_selected_bce_preprocessing() {
        let _guard = global_state_lock();
        let path = temp_path("proof-bce");
        std::fs::write(
            &path,
            "cnf(left, axiom, (p(a)|q(a))).\n\
             cnf(right, axiom, (~p(a)|q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--bce=true",
                "--print-statistics",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains("% BCE start: 2\n% BCE eliminated: 2.\n"));
        assert!(printed.contains("% Parsed axioms                        : 2\n"));
        assert!(printed.contains("% Initial clauses                      : 2\n"));
        assert!(printed.contains("\n% No proof found!\n% SZS status Satisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_applies_selected_goal_defs_preprocessing() {
        let _guard = global_state_lock();
        let path = temp_path("proof-goal-defs");
        std::fs::write(&path, "cnf(goal, negated_conjecture, (f(a)=a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--goal-defs=All",
                "--print-statistics",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains("% Parsed axioms                        : 1\n"));
        assert!(printed.contains("% Initial clauses                      : 1\n"));
        assert!(printed.contains("% Initial clauses in saturation        : 2\n"));
        assert!(printed.contains("\n% No proof found!\n% SZS status Satisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_statistics_count_clause_preprocessing() {
        let _guard = global_state_lock();
        let path = temp_path("proof-clause-preprocess-statistics");
        std::fs::write(
            &path,
            "cnf(drop, axiom, (a=a)).\n\
             cnf(keep, axiom, (p(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-statistics", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains("% Parsed axioms                        : 2\n"));
        assert!(printed.contains("% Initial clauses                      : 2\n"));
        assert!(printed.contains("% Removed in clause preprocessing      : 1\n"));
        assert!(printed.contains("% Initial clauses in saturation        : 1\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_statistics_count_lambda_def_pruning() {
        let _guard = global_state_lock();
        let path = temp_path("proof-lambda-def-statistics");
        std::fs::write(&path, "p(a).\nq(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--print-statistics",
                "--sine=LambdaDef",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.contains("% SinE strategy is LambdaDef\n"));
        assert!(printed.contains("% Parsed axioms                        : 2\n"));
        assert!(printed.contains("% Removed by relevancy pruning/SinE    : 2\n"));
        assert!(printed.contains("% Initial clauses                      : 0\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_runs_presaturation_interreduction() {
        let _guard = global_state_lock();
        let path = temp_path("proof-presaturation");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--presat-simplify=true",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}% Presaturation interreduction done\n\n% No proof found!\n% SZS status Satisfiable\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_cnf_only_prints_initialized_clause_state_without_saturation() {
        let _guard = global_state_lock();
        let path = temp_path("proof-cnf-only");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--cnf", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("% CNFization successful!\n"));
        assert!(printed.contains("% SZS status Unknown\n"));
        assert!(printed.contains("% Unprocessed non-unit clauses:\n"));
        assert!(!printed.contains("% Processed clauses                    :"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_cnf_only_accepts_supported_formula_bridge_input_without_saturation() {
        let _guard = global_state_lock();
        let path = temp_path("proof-cnf-formula-bridge");
        std::fs::write(
            &path,
            "fof(rule, axiom, (p(a)=>q(a))).\n\
             fof(fact, axiom, p(a)).\n\
             fof(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--cnf", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("% CNFization successful!\n"));
        assert!(printed.contains("% SZS status Unknown\n"));
        assert!(printed.contains("p(a) <- .\n"));
        assert!(printed.contains("?- q(a).\n"));
        assert!(printed.contains("q(a) <- p(a).\n"));
        assert!(!printed.contains("% Proof found!"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_cnf_only_accepts_boolean_let_formula_atom() {
        let _guard = global_state_lock();
        let path = temp_path("proof-cnf-boolean-let");
        std::fs::write(&path, "fof(let_bool, axiom, $let(f:$o, f := p(a), f)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--cnf", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("% CNFization successful!\n"));
        assert!(printed.contains("$let($eq(f,$eq(p(a),$true)),$eq(f,$true)) <- .\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_output_level_two_prints_initial_clause_docs_in_default_pcl() {
        let _guard = global_state_lock();
        let path = temp_path("proof-initial-docs-pcl");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--output-level=2",
                "--no-generation",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        let expected_prefix = format!(
            "{}XX\n     1 : :[++p(a)] : XX\ninitial(\"{path_arg}\", at_line_1_column_1)\n",
            default_preprocessing_debug_line()
        );
        assert!(printed.starts_with(&expected_prefix));
        assert!(printed.contains(
            "\n     2 : :[++p(a)] : 1 : 'exists'\n\n% Clause set closed under restricted calculus!\n"
        ));
        assert!(printed.contains("\n% Clause set closed under restricted calculus!\n"));
        assert!(printed.contains("% SZS status GaveUp\n"));
        assert!(printed.contains("% Parsed axioms                        : 1\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_output_level_two_quotes_final_saturated_clauses_in_default_pcl() {
        let _guard = global_state_lock();
        let path = temp_path("proof-final-docs-pcl");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--output-level=2", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains("\n     2 : :[++p(a)] : 1 : 'final'\n\n% No proof found!\n"));
        assert!(printed.contains("% SZS status Satisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_output_level_two_prints_proof_clause_doc_in_default_pcl() {
        let _guard = global_state_lock();
        let path = temp_path("proof-success-docs-pcl");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--output-level=2", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("\n     2 : :[] : 1 : 'proof'\n\n% Proof found!\n"));
        assert!(printed.contains("% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_output_level_two_prints_initial_clause_docs_in_tstp() {
        let _guard = global_state_lock();
        let path = temp_path("proof-initial-docs-tstp");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--tstp-out",
                "--output-level=2",
                "--no-generation",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        let expected_prefix = format!(
            "{}cnf(c_0_1, axiom, (p(a)), file('{path_arg}', at_line_1_column_1)).\n",
            default_preprocessing_debug_line()
        );
        assert!(printed.starts_with(&expected_prefix));
        assert!(printed.contains("\n% Clause set closed under restricted calculus!\n"));
        assert!(printed.contains("% SZS status GaveUp\n"));
        assert!(printed.contains("% Parsed axioms                        : 1\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_output_level_two_prints_proof_clause_doc_in_tstp() {
        let _guard = global_state_lock();
        let path = temp_path("proof-success-docs-tstp");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--tstp-out",
                "--output-level=2",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(
            printed.contains("\ncnf(c_0_2, plain, ($false), c_0_1,['proof']).\n\n% Proof found!\n")
        );
        assert!(printed.contains("% SZS status Unsatisfiable\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_object_list_prints_supported_success_block_in_default_pcl() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-success-pcl");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--proof-object=1", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains(
            "\n% Proof found!\n% SZS status Unsatisfiable\n% SZS output start CNFRefutation\n"
        ));
        assert!(printed.contains("     1 : :[] : cn() : 'proof'\n"));
        assert!(printed.contains("% SZS output end CNFRefutation\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_full_deriv_proof_object_list_prints_success_nonproof_roots() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-success-full-deriv-list");
        std::fs::write(&path, "a!=a.\np(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--full-deriv",
                "--proof-object=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("% SZS output start CNFRefutation\n"));
        assert!(printed.contains("p(a)"));
        assert!(printed.contains(": 'final'\n"));
        assert!(printed.contains(": 'proof'\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_object_zero_does_not_enable_proof_object_output() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-zero");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--proof-object=0", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\n"));
        assert!(!printed.contains("SZS output start CNFRefutation"));
        assert!(!printed.contains("SZS output end CNFRefutation"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_object_list_prints_supported_success_block_in_tstp() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-success-tstp");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--tstp-out",
                "--proof-object=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains(
            "\n% Proof found!\n% SZS status Unsatisfiable\n% SZS output start CNFRefutation\n"
        ));
        assert!(printed
            .contains("cnf(c_0_1, axiom, ($false), inference(cn,[status(thm)],[]), ['proof']).\n"));
        assert!(printed.contains("% SZS output end CNFRefutation\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_graph_prints_supported_success_dot() {
        let _guard = global_state_lock();
        let path = temp_path("proof-graph-success-dot");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--proof-graph=1", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("\n% Proof found!\n% SZS status Unsatisfiable\ndigraph proof{\n"));
        assert!(printed.contains("  rankdir=TB\n"));
        assert!(printed.contains(
            "  1 [shape=box,color=blue,fillcolor=darkorchid1,style=filled,label=\"c1\"]\n"
        ));
        assert!(!printed.contains(" -> "));
        assert!(!printed.contains("SZS output start CNFRefutation"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_graph_level2_prints_clause_and_derivation_labels() {
        let _guard = global_state_lock();
        let path = temp_path("proof-graph-detailed-dot");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--lop-in", "--proof-graph=2", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("label=\"cnf("));
        assert!(printed.contains(",\\ninference(cn,[status(thm)],[])).\"]\n"));
        assert!(!printed.contains("label=\"c2\""));
        assert!(!printed.contains(" -> "));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_graph_level2_prints_source_info_for_initial_node() {
        let _guard = global_state_lock();
        let path = temp_path("proof-graph-source-info-dot");
        std::fs::write(&path, "cnf(source_node, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--proof-graph=2", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains("label=\"cnf("));
        assert!(printed.contains(",\\nfile('"));
        assert!(printed.contains("proof-graph-source-info-dot"));
        assert!(printed.contains(", source_node)).\"]\n"));
        assert!(!printed.contains("inference("));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_record_gcs_updates_success_statistics() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-record-gcs");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--print-statistics",
                "--record-gcs",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("% Proof object given clauses           : 1\n"));
        assert!(printed.contains("% Proof search given clauses           : 1\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_statistics_with_record_gcs_prints_success_proof_object_analysis() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-statistics-record-gcs");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--proof-statistics",
                "--record-gcs",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("% Proof object total steps             : 1\n"));
        assert!(printed.contains("% Proof object clause steps            : 1\n"));
        assert!(printed.contains("% Proof object formula steps           : 0\n"));
        assert!(printed.contains("% Proof object initial clauses used    : 1\n"));
        assert!(printed.contains("% Proof object generating inferences   : 0\n"));
        assert!(printed.contains("% Proof object simplifying inferences  : 1\n"));
        assert!(!printed.contains("SZS output start CNFRefutation"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_training_examples_prints_counts_and_requested_sections() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-training-examples");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--training-examples=3",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("% Training examples: 1 positive, 0 negative\n"));
        assert!(printed.contains("% Training: Positive examples begin\n"));
        assert!(printed.contains("% trainpos\n"));
        assert!(printed.contains("% Training: Positive examples end\n"));
        assert!(printed.contains("% Training: Negative examples begin\n"));
        assert!(!printed.contains("%trainneg\n"));
        assert!(printed.contains("% Training: Negative examples end\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_training_examples_prints_clauses_in_selected_output_format() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-training-examples-tstp");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--tstp-out",
                "--training-examples=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert!(printed.contains("% Training examples: 1 positive, 0 negative\n"));
        assert!(printed.contains("% Training: Positive examples begin\n"));
        assert!(printed.contains("cnf("));
        assert!(printed.contains(", ($false)).% trainpos\n"));
        assert!(!printed.contains("\n <- a=a.% trainpos\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_object_list_prints_saturation_block_for_no_proof_tstp() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-saturation-tstp");
        std::fs::write(&path, "cnf(keep, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--proof-object=1", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains(
            "\n% No proof found!\n% SZS status Satisfiable\n% SZS output start Saturation\n"
        ));
        assert!(printed.contains("cnf("));
        assert!(printed.contains("(p(a))"));
        assert!(printed.contains(", ['final']).\n"));
        assert!(printed.contains("% SZS output end Saturation\n"));
        assert!(!printed.contains("CNFRefutation"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_statistics_prints_saturation_proof_object_analysis() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-statistics-saturation");
        std::fs::write(&path, "cnf(keep, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--proof-object=1",
                "--proof-statistics",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains("% SZS output start Saturation\n"));
        assert!(printed.contains("% Proof object total steps             : 1\n"));
        assert!(printed.contains("% Proof object clause steps            : 1\n"));
        assert!(printed.contains("% Proof object formula steps           : 0\n"));
        assert!(printed.contains("% Proof object initial clauses used    : 1\n"));
        assert!(printed.contains("% Proof object generating inferences   : 0\n"));
        assert!(printed.contains("% Proof object simplifying inferences  : 0\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_graph_prints_saturation_dot_for_complete_no_proof() {
        let _guard = global_state_lock();
        let path = temp_path("proof-graph-saturation-dot");
        std::fs::write(&path, "cnf(keep, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--proof-graph=1", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains("\n% No proof found!\n% SZS status Satisfiable\ndigraph proof{\n"));
        assert!(printed.contains(
            "  1 [shape=box,color=green,fillcolor=forestgreen,style=filled,label=\"c1\"]\n"
        ));
        assert!(!printed.contains("SZS output start Saturation"));
        assert!(!printed.contains("CNFRefutation"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_object_list_prints_saturation_block_for_no_proof_pcl() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-saturation-pcl");
        std::fs::write(&path, "cnf(keep, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--pcl-out",
                "--proof-object=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::SATISFIABLE.exit_status());
        assert!(printed.contains("% SZS output start Saturation\n"));
        assert!(printed.contains("p(a)"));
        assert!(printed.contains(" : 'final'\n"));
        assert!(printed.contains("% SZS output end Saturation\n"));
        assert!(!printed.contains("CNFRefutation"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn write_stopped_proof_object_list_expands_represented_ancestors() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap();
        let mut parent = parse_lop_test_clause(state.terms_mut(), "p(a).", 31);
        parent.set_csscpa_source(1);
        let mut root = parse_lop_test_clause(state.terms_mut(), "q(a).", 32);
        root.set_csscpa_source(2);
        clause_push_derivation(&mut root, DC_EQ_RES, Some(&parent), None);

        state.axioms_mut().insert(parent);
        state.processed_non_units_mut().insert(root);

        let config = EProverConfig {
            proof_output: 1,
            doc_output_format: DocOutputFormat::Pcl,
            ..EProverConfig::default()
        };
        let mut output = Vec::new();
        write_stopped_proof_output(&mut output, &config, &state, "Saturation").unwrap();

        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains("% SZS output start Saturation\n"));
        let parent_position = printed
            .find("p(a)")
            .unwrap_or_else(|| panic!("missing ancestor clause in:\n{printed}"));
        let final_position = printed
            .find("'final'")
            .unwrap_or_else(|| panic!("missing final root marker in:\n{printed}"));
        assert!(parent_position < final_position, "{printed}");
        let root_line = printed
            .lines()
            .find(|line| line.contains("'final'"))
            .unwrap_or_else(|| panic!("missing final root line in:\n{printed}"));
        assert!(root_line.contains("(1)"), "{printed}");
        assert_eq!(printed.matches("'final'").count(), 1);
        assert!(printed.contains("% SZS output end Saturation\n"));
    }

    #[test]
    fn write_saturation_proof_object_clause_uses_source_info_and_root_marker() {
        let bank = temporary_executable_term_bank(FP_IGNORE_PROPS).unwrap();
        let mut clause = Clause::empty();
        clause.set_ident(7);
        clause.set_tptp_type(CP_TYPE_AXIOM);
        clause.set_info(Some(ClauseInfo::new(Some("named"), Some("file.p"), 3, 4)));

        let mut config = EProverConfig {
            doc_output_format: DocOutputFormat::Pcl,
            ..EProverConfig::default()
        };
        let mut output = Vec::new();
        write_saturation_proof_object_clause(&mut output, &config, &bank, &clause, true).unwrap();
        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains("initial(\"file.p\", named) : 'proof'\n"));

        let mut output = Vec::new();
        write_saturation_proof_object_clause(&mut output, &config, &bank, &clause, false).unwrap();
        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains("initial(\"file.p\", named)\n"));
        assert!(!printed.contains("'proof'"));

        config.doc_output_format = DocOutputFormat::Tstp;
        let mut output = Vec::new();
        write_saturation_proof_object_clause(&mut output, &config, &bank, &clause, true).unwrap();
        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains(", file('file.p', named), ['proof']).\n"));

        let mut output = Vec::new();
        write_saturation_proof_object_clause(&mut output, &config, &bank, &clause, false).unwrap();
        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains(", file('file.p', named)).\n"));
        assert!(!printed.contains("'proof'"));
    }

    #[test]
    fn run_proof_object_list_skips_saturation_block_for_incomplete_no_proof() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-incomplete-no-saturation");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--no-generation",
                "--proof-object=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.contains("\n% Clause set closed under restricted calculus!\n"));
        assert!(!printed.contains("SZS output start Saturation"));
        assert!(!printed.contains("SZS output end Saturation"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_force_deriv_prints_derivation_block_for_incomplete_no_proof() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-force-deriv-incomplete");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--no-generation",
                "--proof-object=1",
                "--force-deriv=1",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert!(printed.contains("\n% Clause set closed under restricted calculus!\n"));
        assert!(printed.contains("% SZS output start Derivation\n"));
        assert!(printed.contains("p(a)"));
        assert!(printed.contains("'final'\n"));
        assert!(printed.contains("% SZS output end Derivation\n"));
        assert!(!printed.contains("SZS output start Saturation"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_force_deriv_level2_prints_unprocessed_resource_roots() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-force-deriv-resource");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--processed-clauses-limit=0",
                "--proof-object=1",
                "--force-deriv=2",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        assert!(printed.contains("\n% Failure: User resource limit exceeded!\n"));
        assert!(printed.contains("% SZS output start Derivation\n"));
        assert!(printed.contains("p(a)"));
        assert!(printed.contains("% SZS output end Derivation\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_force_deriv_statistics_can_print_without_derivation_format() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-force-deriv-statistics");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--processed-clauses-limit=0",
                "--force-deriv=2",
                "--proof-statistics",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        assert!(!printed.contains("SZS output start Derivation"));
        assert!(printed.contains("% Proof object total steps             : 1\n"));
        assert!(printed.contains("% Proof object initial clauses used    : 1\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_force_deriv_level2_suppresses_later_saturated_output() {
        let _guard = global_state_lock();
        let path = temp_path("proof-object-force-deriv-no-print-saturated");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--processed-clauses-limit=0",
                "--print-saturated=e",
                "--force-deriv=2",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        assert!(printed.contains("\n% Failure: User resource limit exceeded!\n"));
        assert!(!printed.contains("SZS output start Derivation"));
        assert!(!printed.contains("p(a)"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_resource_status_when_step_limit_fires() {
        let _guard = global_state_lock();
        let path = temp_path("proof-step-limit");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--no-generation",
                "--processed-clauses-limit=0",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Failure: User resource limit exceeded!\n% SZS status ResourceOut\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_config_filter_saturated_can_promote_unprocessed_empty_clause_to_proof() {
        let _guard = global_state_lock();
        let path = temp_path("proof-filter-saturated");
        std::fs::write(&path, "a!=a.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();

        let mut config = EProverConfig {
            files: vec![path_arg],
            parse_format: IoFormat::Lop,
            step_limit: 0,
            filter_saturated_descriptor: "n".to_owned(),
            ..EProverConfig::default()
        };
        config.flags.set(EProverFlag::FilterSaturated);

        let status = run_config(&mut stdout, &config).unwrap();

        assert_eq!(status, ErrorCode::PROOF_FOUND.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Proof found!\n% SZS status Unsatisfiable\n",
                default_proof_search_prefix()
            )
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_proof_search_loads_dynamic_watchlist_file() {
        let _guard = global_state_lock();
        let input_path = temp_path("proof-dynamic-watch-input");
        let watch_path = temp_path("proof-dynamic-watch-list");
        std::fs::write(&input_path, "p(a).\n").unwrap();
        std::fs::write(&watch_path, "p(a).\n").unwrap();
        let input_arg = input_path.to_string_lossy().into_owned();
        let watch_arg = format!("--watchlist={}", watch_path.to_string_lossy());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--no-generation",
                watch_arg.as_str(),
                input_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Watchlist is empty!\n% SZS status ResourceOut\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&input_path).unwrap();
        std::fs::remove_file(&watch_path).unwrap();
    }

    #[test]
    fn run_output_level_two_documents_inline_watchlist_after_input_clauses() {
        let _guard = global_state_lock();
        let path = temp_path("proof-inline-watch-docs");
        std::fs::write(
            &path,
            "tcf(watch, watchlist, p(a)).\ncnf(input, axiom, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--output-level=2",
                "--no-generation",
                "--watchlist=Use inline watchlist type",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        let input_doc = format!("cnf(c_0_1, axiom, (p(a)), file('{path_arg}', input)).\n");
        let watch_doc =
            format!("cnf(c_0_2, watchlist, (p(a)), file('{path_arg}', watch),['wl']).\n");
        let final_doc =
            "cnf(c_0_3, plain, (p(a)), c_0_1,['final_subsumes_wl']).\n\n% Watchlist is empty!\n";
        let input_doc_pos = printed.find(&input_doc).unwrap();
        let watch_doc_pos = printed.find(&watch_doc).unwrap();
        let final_doc_pos = printed.find(final_doc).unwrap();
        assert!(input_doc_pos < watch_doc_pos);
        assert!(watch_doc_pos < final_doc_pos);
        assert!(printed.contains("% SZS status ResourceOut\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_output_level_two_unfolds_inline_watchlist_before_docs() {
        let _guard = global_state_lock();
        let path = temp_path("proof-inline-watch-eqdef-docs");
        std::fs::write(
            &path,
            "cnf(def, axiom, (f(X)=X)).\n\
             tcf(watch, watchlist, p(f(a))).\n\
             cnf(input, axiom, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--output-level=2",
                "--no-generation",
                "--watchlist=Use inline watchlist type",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        let input_doc = format!("cnf(c_0_1, axiom, (p(a)), file('{path_arg}', input)).\n");
        let watch_doc =
            format!("cnf(c_0_2, watchlist, (p(a)), file('{path_arg}', watch),['wl']).\n");
        let final_doc =
            "cnf(c_0_3, plain, (p(a)), c_0_1,['final_subsumes_wl']).\n\n% Watchlist is empty!\n";
        let input_doc_pos = printed.find(&input_doc).unwrap();
        let watch_doc_pos = printed.find(&watch_doc).unwrap();
        let final_doc_pos = printed.find(final_doc).unwrap();
        assert!(input_doc_pos < watch_doc_pos);
        assert!(watch_doc_pos < final_doc_pos);
        assert!(!printed.contains("p(f(a))"));
        assert!(!printed.contains(&format!("file('{path_arg}', def)")));
        assert!(printed.contains("% SZS status ResourceOut\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_output_level_two_unfolds_file_watchlist_before_docs() {
        let _guard = global_state_lock();
        let input_path = temp_path("proof-file-watch-eqdef-input");
        let watch_path = temp_path("proof-file-watch-eqdef-list");
        std::fs::write(
            &input_path,
            "cnf(def, axiom, (f(X)=X)).\n\
             cnf(input, axiom, (p(a))).\n",
        )
        .unwrap();
        std::fs::write(&watch_path, "cnf(watch, axiom, (p(f(a)))).\n").unwrap();
        let input_arg = input_path.to_string_lossy().into_owned();
        let watch_path_arg = watch_path.to_string_lossy().into_owned();
        let watch_arg = format!("--watchlist={watch_path_arg}");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--tstp-in",
                "--tstp-out",
                "--output-level=2",
                "--no-generation",
                watch_arg.as_str(),
                input_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        let input_doc = format!("cnf(c_0_1, axiom, (p(a)), file('{input_arg}', input)).\n");
        let watch_doc =
            format!("cnf(c_0_2, watchlist, (p(a)), file('{watch_path_arg}', watch),['wl']).\n");
        let final_doc =
            "cnf(c_0_3, plain, (p(a)), c_0_1,['final_subsumes_wl']).\n\n% Watchlist is empty!\n";
        let input_doc_pos = printed.find(&input_doc).unwrap();
        let watch_doc_pos = printed.find(&watch_doc).unwrap();
        let final_doc_pos = printed.find(final_doc).unwrap();
        assert!(input_doc_pos < watch_doc_pos);
        assert!(watch_doc_pos < final_doc_pos);
        assert!(!printed.contains("p(f(a))"));
        assert!(!printed.contains(&format!("file('{input_arg}', def)")));
        assert!(printed.contains("% SZS status ResourceOut\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&input_path).unwrap();
        std::fs::remove_file(&watch_path).unwrap();
    }

    #[test]
    fn run_proof_search_uses_selected_inline_watchlist_from_include() {
        let _guard = global_state_lock();
        let include_path = temp_path("proof-inline-watch-include-inc");
        let path = temp_path("proof-inline-watch-include-main");
        std::fs::write(
            &include_path,
            "tcf(selected, watchlist, p(a)).\n\
             tcf(unselected, watchlist, q(a)).\n",
        )
        .unwrap();
        let include_arg = include_path.to_string_lossy().into_owned();
        std::fs::write(
            &path,
            format!("include('{include_arg}',[selected]).\ncnf(input, axiom, p(a)).\n"),
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--no-generation",
                "--watchlist=Use inline watchlist type",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Watchlist is empty!\n% SZS status ResourceOut\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_file(&include_path).unwrap();
    }

    #[test]
    fn run_output_level_two_quotes_watchlist_subsumers_in_default_pcl() {
        let _guard = global_state_lock();
        let input_path = temp_path("proof-watch-final-doc-input");
        let watch_path = temp_path("proof-watch-final-doc-list");
        std::fs::write(&input_path, "p(a).\n").unwrap();
        std::fs::write(&watch_path, "p(a).\n").unwrap();
        let input_arg = input_path.to_string_lossy().into_owned();
        let watch_arg = format!("--watchlist={}", watch_path.to_string_lossy());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--output-level=2",
                "--no-generation",
                watch_arg.as_str(),
                input_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::RESOURCE_OUT.exit_status());
        let input_doc =
            format!("     1 : :[++p(a)] : XX\ninitial(\"{input_arg}\", at_line_1_column_1)\n");
        let watch_doc = format!(
            "     2 : :[++p(a)] : XX\ninitial(\"{}\", at_line_1_column_1) : 'wl'\n",
            watch_path.to_string_lossy()
        );
        let final_doc = "     3 : :[++p(a)] : 1 : 'final_subsumes_wl'\n\n% Watchlist is empty!\n";
        let input_doc_pos = printed.find(&input_doc).unwrap();
        let watch_doc_pos = printed.find(&watch_doc).unwrap();
        let final_doc_pos = printed.find(final_doc).unwrap();
        assert!(input_doc_pos < watch_doc_pos);
        assert!(watch_doc_pos < final_doc_pos);
        assert!(printed.contains("% SZS status ResourceOut\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&input_path).unwrap();
        std::fs::remove_file(&watch_path).unwrap();
    }

    #[test]
    fn run_proof_search_static_watchlist_keeps_matched_file_clause() {
        let _guard = global_state_lock();
        let input_path = temp_path("proof-static-watch-input");
        let watch_path = temp_path("proof-static-watch-list");
        std::fs::write(&input_path, "p(a).\n").unwrap();
        std::fs::write(&watch_path, "p(a).\n").unwrap();
        let input_arg = input_path.to_string_lossy().into_owned();
        let watch_arg = format!("--static-watchlist={}", watch_path.to_string_lossy());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--lop-in",
                "--no-generation",
                watch_arg.as_str(),
                input_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::INCOMPLETE_PROOFSTATE.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!(
                "{}\n% Clause set closed under restricted calculus!\n% SZS status GaveUp\n",
                default_proof_search_prefix()
            )
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&input_path).unwrap();
        std::fs::remove_file(&watch_path).unwrap();
    }

    #[test]
    fn run_proof_search_reports_missing_watchlist_file() {
        let _guard = global_state_lock();
        let input_path = temp_path("proof-missing-watch-input");
        let watch_path = temp_path("proof-missing-watch-list");
        let _ = std::fs::remove_file(&watch_path);
        std::fs::write(&input_path, "p(a).\n").unwrap();
        let input_arg = input_path.to_string_lossy().into_owned();
        let watch_arg = format!("--watchlist={}", watch_path.to_string_lossy());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                "eprover",
                "--lop-in",
                watch_arg.as_str(),
                input_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            default_preprocessing_debug_line()
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&input_path).unwrap();
    }

    #[test]
    fn run_emits_lpo_recursion_limit_warning_like_c() {
        let _guard = global_state_lock();
        let old_limit = lpo_recursion_depth_limit();
        let path = temp_path("syntax-lpo-warning");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--syntax-only",
                "--lop-in",
                "--lpo-recursion-limit=20001",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            "eprover: Warning: Using very large values for --lpo-recursion-limit may lead to stack overflows and segmentation faults.\n"
        );

        set_lpo_recursion_depth_limit(old_limit);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_renders_parsed_lop_clauses() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas");
        std::fs::write(&path, "p(a).\nq(a) <- p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", "--lop-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "p(a) <- .\nq(a) <- p(a).\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_honors_lop_equation_print_options() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-eqn-options");
        std::fs::write(&path, "a=b.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--lop-in",
                "--eqn-no-infix",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(String::from_utf8(stdout).unwrap(), "equal(a, b) <- .\n");
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_honors_print_types() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-print-types");
        std::fs::write(&path, "p(a).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--lop-in",
                "--print-types",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(String::from_utf8(stdout).unwrap(), "p(a:$i):$o <- .\n");
        assert!(stderr.is_empty());

        let mut tptp_stdout = Vec::new();
        let mut tptp_stderr = Vec::new();
        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--lop-in",
                "--tptp-out",
                "--print-types",
                path_arg.as_str(),
            ],
            &mut tptp_stdout,
            &mut tptp_stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(tptp_stdout).unwrap();
        assert!(printed.starts_with("input_clause(i_0_"));
        assert!(printed.ends_with(",axiom,[++p(a:$i):$o]).\n"));
        assert!(tptp_stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_uses_tstp_type_declarations_for_term_types() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-tstp-type-declarations");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             fof(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             fof(test1, axiom, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--tstp-in",
                "--print-types",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(String::from_utf8(stdout).unwrap(), "p(a:person):$o <- .\n");
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_uses_tff_formula_entries() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-tff-formula-entries");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             tff(test1, axiom, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--tstp-in",
                "--print-types",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(String::from_utf8(stdout).unwrap(), "p(a:person):$o <- .\n");
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_uses_tcf_formula_entries() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-tcf-formula-entries");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             tcf(test1, axiom, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--tstp-in",
                "--print-types",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(String::from_utf8(stdout).unwrap(), "p(a:person):$o <- .\n");
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_tff_typed_existential_quantifier() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-tff-typed-existential");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(p_type, type, p: person > $o).\n\
             tff(exists, axiom, ?[X: person]:p(X)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--tstp-in",
                "--print-types",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "p(esk1_0:person):$o <- .\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_restores_typed_quantifier_shadow_scope() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-tff-quantifier-shadow-scope");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(p_type, type, p: $i > $o).\n\
             tff(q_type, type, q: person > $o).\n\
             tff(r_type, type, r: $i > $o).\n\
             tff(test, axiom, ![X:$i]:(p(X)|![X:person]:q(X)|r(X))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--tstp-in",
                "--print-types",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "p(X1:$i):$o; q(X2:person):$o; r(X1:$i):$o <- .\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_expands_fof_distinct_formula() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-fof-distinct");
        std::fs::write(&path, "fof(test1, axiom, $distinct(a,b,c)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains(", axiom, (a!=b)).\n"));
        assert!(printed.contains(", axiom, (a!=c)).\n"));
        assert!(printed.contains(", axiom, (b!=c)).\n"));
        assert_eq!(printed.matches("cnf(i_0_").count(), 3);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_negates_fof_distinct_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-fof-distinct-conjecture");
        std::fs::write(&path, "fof(goal, conjecture, $distinct(a,b,c)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", negated_conjecture, (a=b|a=c|b=c)).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_lowers_fof_truth_constants() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-fof-truth-constants");
        std::fs::write(
            &path,
            "fof(true_axiom, axiom, $true).\n\
             fof(false_axiom, axiom, $false).\n\
             fof(false_disjunction, axiom, ($false|p(a))).\n\
             fof(true_conjunction, axiom, ($true&p(a))).\n\
             fof(goal, conjecture, $true).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(!printed.contains("$true"));
        assert!(printed.contains(", axiom, ($false)).\n"));
        assert_eq!(printed.matches(", axiom, (p(a))).\n").count(), 2);
        assert!(printed.contains(", negated_conjecture, ($false)).\n"));
        assert_eq!(printed.matches("cnf(i_0_").count(), 4);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_uses_old_tptp_output_when_requested() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-tptp-out");
        std::fs::write(&path, "a=b.\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--lop-in",
                "--tptp-out",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("input_clause(i_0_"));
        assert!(printed.ends_with(",axiom,[++equal(a, b)]).\n"));
        assert!(!printed.starts_with("cnf("));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_preserves_old_tptp_formula_role_mapping() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-tptp-input-formula-default-roles");
        std::fs::write(
            &path,
            "input_formula(lem, lemma, p(a)).\n\
             input_formula(unk, unknown, q(a)).\n\
             input_formula(que, question, r(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--tptp-in",
                "--tstp-out",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(printed.matches(", axiom, ").count(), 2);
        assert_eq!(printed.matches(", question, ").count(), 1);
        assert!(!printed.contains(", lemma, "));
        assert!(!printed.contains(", plain, "));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_lowers_old_tptp_existential_fool_body() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-old-tptp-existential-fool-body");
        std::fs::write(
            &path,
            "input_formula(ex_ite, axiom, ?[X]:$ite(p(X), q(X), r(X))).\n\
             input_formula(ex_let_eq, axiom, ?[Y]:$let(f:$i, f := a, f) = Y).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                "eprover",
                "--print-formulas",
                "--tptp-in",
                "--tstp-out",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed
            .contains("$ite($eq(p(esk1_0),$true),$eq(q(esk1_0),$true),$eq(r(esk1_0),$true))"));
        assert!(printed.contains("$let($eq(f,$eq(a,$true)),f)=esk2_0"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_auto_tstp_input_uses_tstp_output() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-auto-tstp");
        std::fs::write(&path, "cnf(c1, axiom, (p(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (p(a))).\n"));
        assert!(!printed.contains("<-"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_preserves_tstp_question_formula_role() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-tstp-question-role");
        std::fs::write(&path, "fof(query, question, p(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", question, (p(a))).\n"));
        assert!(!printed.contains(", plain, "));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_supported_fof_positive_existential_atom() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-positive-existential");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:p(X)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (p(esk1_0))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_existential_fool_body() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-existential-fool-body");
        std::fs::write(
            &path,
            "tff(a_type, type, a: $i).\n\
             tff(b_type, type, b: $i).\n\
             tff(p_type, type, p: $i > $o).\n\
             tff(q_type, type, q: $i > $o).\n\
             tff(r_type, type, r: $i > $o).\n\
             fof(ex_ite, axiom, ?[X]:$ite(p(X), q(X), r(X))).\n\
             fof(ex_let_eq, axiom, ?[Y]:$let(f:$i, f := a, f) = Y).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed
            .contains("$ite($eq(p(esk1_0),$true),$eq(q(esk1_0),$true),$eq(r(esk1_0),$true))"));
        assert!(printed.contains("$let($eq(f,a),f)=esk2_0"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_rejects_tstp_formula_free_variables() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tstp-formula-free-variables");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:p(X,Y)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains(TSTP_FORMULA_FREE_VARIABLES_MESSAGE));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_supported_fof_parenthesized_existential_conjunction() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-parenthesized-existential-conjunction");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:(p(X)&q(X))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.contains(", axiom, (p(esk1_0))).\n"));
        assert!(printed.contains(", axiom, (q(esk1_0))).\n"));
        assert_eq!(printed.matches("esk1_0").count(), 2);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_nested_fof_existentials() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-nested-existentials");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:(?[Y]:p(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (p(esk1_0,esk2_0))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_unparenthesized_nested_fof_existentials() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-unparenthesized-nested-existentials");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:?[Y]:p(X,Y)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (p(esk1_0,esk2_0))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_negates_nested_fof_existential_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-negated-nested-existentials");
        std::fs::write(&path, "fof(goal, conjecture, ?[X]:(?[Y]:p(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", negated_conjecture, (~p(X1,X2))).\n"));
        assert!(!printed.contains("esk"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_fof_conjunction_with_existential_conjunct() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-conjunction-existential-conjunct");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:p(X)&q(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains(", axiom, (p(esk1_0))).\n"));
        assert!(printed.contains(", axiom, (q(a))).\n"));
        assert_eq!(printed.matches("cnf(i_0_").count(), 2);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_negates_fof_conjecture_conjunction_with_existential_conjunct() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-conjecture-conjunction-existential-conjunct");
        std::fs::write(&path, "fof(goal, conjecture, ?[X]:p(X)&q(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", negated_conjecture, (~p(X1)|~q(a))).\n"));
        assert!(!printed.contains("esk"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_fof_disjunction_with_existential_disjunct() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-disjunction-existential-disjunct");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:p(X)|q(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (p(esk1_0)|q(a))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_distributes_existential_conjunction_disjunct() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-disjunction-existential-conjunction");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:(p(X)&q(X))|r(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains(", axiom, (p(esk1_0)|r(a))).\n"));
        assert!(printed.contains(", axiom, (q(esk1_0)|r(a))).\n"));
        assert_eq!(printed.matches("cnf(i_0_").count(), 2);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_distributes_conjunctive_disjunct_with_existential_conjunct() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-disjunction-existential-conjunct");
        std::fs::write(&path, "fof(test1, axiom, ((p(a)&?[X]:q(X))|r(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains(", axiom, (p(a)|r(a))).\n"));
        assert!(printed.contains(", axiom, (q(esk1_0)|r(a))).\n"));
        assert_eq!(printed.matches("cnf(i_0_").count(), 2);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_negates_supported_fof_parenthesized_existential_conjunction() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-negated-parenthesized-existential-conjunction");
        std::fs::write(&path, "fof(goal, conjecture, ?[X]:(p(X)&q(X))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.contains(", negated_conjecture, ("));
        assert!(printed.contains("~p("));
        assert!(printed.contains("~q("));
        assert!(!printed.contains("esk"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_universal_positive_existential_atom() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-universal-positive-existential");
        std::fs::write(&path, "fof(test1, axiom, ![Y]: ?[X]:p(X,Y)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (p(esk1_1(X1),X1))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_negated_universal_existential_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-negated-universal-existential-conjecture");
        std::fs::write(&path, "fof(goal, conjecture, ![Y]: ?[X]:p(X,Y)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", negated_conjecture, (~p(X2,esk1_0))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_existential_before_universal_scope() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-existential-universal-scope");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:(![Y]:p(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (p(esk1_0,X2))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_unparenthesized_existential_before_universal_scope() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-unparenthesized-existential-universal-scope");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:![Y]:p(X,Y)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (p(esk1_0,X2))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_unparenthesized_negated_existential_body() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-unparenthesized-negated-existential-body");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:~p(X)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (~p(esk1_0))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_negated_universal_in_existential_body() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-existential-negated-universal-body");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:(~![Y]:p(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (~p(esk1_0,esk2_0))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_lowers_quantified_implication_operand_in_existential_body() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-existential-quantified-implication");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:(p(X)=>![Y]:q(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (q(esk1_0,X2)|~p(esk1_0))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_existential_consequent_in_implication() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-implication-existential-consequent");
        std::fs::write(&path, "fof(test1, axiom, ![Y]:(p(Y)=>?[X]:q(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (q(esk1_1(X1),X1)|~p(X1))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_keeps_existential_antecedent_universal_in_implication() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-implication-existential-antecedent");
        std::fs::write(&path, "fof(test1, axiom, ![Y]:((?[X]:p(X,Y))=>q(Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (q(X1)|~p(X2,X1))).\n"));
        assert!(!printed.contains("esk"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_existential_operand_in_equivalence() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-equivalence-existential-operand");
        std::fs::write(&path, "fof(test1, axiom, ![Y]:(p(Y)<=>?[X]:q(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains(", axiom, (q(esk1_1(X1),X1)|~p(X1))).\n"));
        assert!(printed.contains(", axiom, (p(X1)|~q(X2,X1))).\n"));
        assert_eq!(printed.matches("cnf(i_0_").count(), 2);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_lowers_xor_with_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-xor-existential-operand");
        std::fs::write(&path, "fof(test1, axiom, (p(a)<~>?[X]:q(X))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains(", axiom, (p(a)|q(esk1_0))).\n"));
        assert!(printed.contains(", axiom, (~p(a)|~q(X1))).\n"));
        assert_eq!(printed.matches("cnf(i_0_").count(), 2);
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_lowers_nand_with_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-nand-existential-operand");
        std::fs::write(&path, "fof(test1, axiom, (p(a)~&?[X]:q(X))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (~p(a)|~q(X1))).\n"));
        assert!(!printed.contains("esk"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_lowers_nor_with_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-nor-existential-operand");
        std::fs::write(&path, "fof(test1, axiom, (p(a)~|?[X]:q(X))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.contains(", axiom, (~p(a))).\n"));
        assert!(printed.contains(", axiom, (~q(X1))).\n"));
        assert_eq!(printed.matches("cnf(i_0_").count(), 2);
        assert!(!printed.contains("esk"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_lowers_universal_nand_with_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-universal-nand-existential-operand");
        std::fs::write(&path, "fof(test1, axiom, ![Y]:(p(Y)~&?[X]:q(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", axiom, (~p(X1)|~q(X2,X1))).\n"));
        assert!(!printed.contains("esk"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_print_formulas_skolemizes_negated_universal_conjecture() {
        let _guard = global_state_lock();
        let path = temp_path("print-formulas-universal-conjecture");
        std::fs::write(&path, "fof(goal, conjecture, ![Y]:p(Y)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--print-formulas", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let printed = String::from_utf8(stdout).unwrap();
        assert!(printed.starts_with("cnf(i_0_"));
        assert!(printed.ends_with(", negated_conjecture, (~p(esk1_0))).\n"));
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_rejects_trailing_tokens_after_clause_list() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-junk");
        std::fs::write(&path, "p(a). )").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--syntax-only", "--lop-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("Unexpected token after clause list"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_old_tptp_input_formula() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tptp-input-formula");
        std::fs::write(&path, "input_formula(goal, conjecture, p(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_old_tptp_truth_constants() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tptp-truth-constants");
        std::fs::write(
            &path,
            "input_formula(old_true, axiom, $true).\n\
             input_formula(old_false, conjecture, $false).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_old_tptp_existential_fool_body() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-old-tptp-existential-fool-body");
        std::fs::write(
            &path,
            "input_formula(ex_ite, axiom, ?[X]:$ite(p(X), q(X), r(X))).\n\
             input_formula(ex_let_eq, axiom, ?[Y]:$let(f:$i, f := a, f) = Y).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", "--tptp-in", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_tstp_type_declarations_before_fof_formula() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tstp-type-declarations");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             fof(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             fof(test1, axiom, p(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_tff_formula_entries() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tff-formula-entries");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             tff(fact, axiom, p(a)).\n\
             tff(rule, axiom, (p(a)=>q(a))).\n\
             tff(goal, conjecture, q(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_tcf_formula_entries() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tcf-formula-entries");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(p_type, type, p: person > $o).\n\
             tff(q_type, type, q: person > $o).\n\
             tcf(fact, axiom, p(a)).\n\
             tcf(rule, axiom, ![X: person]:(p(X)|q(X))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_tcf_watchlist_formula_role() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tcf-watchlist-role");
        std::fs::write(&path, "tcf(watch, watchlist, p(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_error_on_empty_ignores_tcf_watchlist_formula_role() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tcf-watchlist-role-empty");
        std::fs::write(&path, "tcf(watch, watchlist, p(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                "eprover",
                "--syntax-only",
                "--error-on-empty",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::INPUT_SEMANTIC_ERROR);
        assert!(error.message().contains("did not contain any clauses"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_rejects_non_tcf_watchlist_formula_role() {
        let _guard = global_state_lock();
        for formula_kind in ["fof", "tff"] {
            let path = temp_path(&format!("syntax-{formula_kind}-watchlist-role"));
            std::fs::write(&path, format!("{formula_kind}(watch, watchlist, p(a)).\n")).unwrap();
            let path_arg = path.to_string_lossy().into_owned();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();

            let error = run(
                ["eprover", "--syntax-only", path_arg.as_str()],
                &mut stdout,
                &mut stderr,
            )
            .unwrap_err();

            assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
            assert!(error.message().contains("watchlist"));
            assert!(stdout.is_empty());
            assert!(stderr.is_empty());
            std::fs::remove_file(&path).unwrap();
        }
    }

    #[test]
    fn run_syntax_only_rejects_thf_with_hol_diagnostic() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-thf-requires-hol");
        std::fs::write(&path, "thf(goal, conjecture, p(a)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains(THF_REQUIRES_HOL_MESSAGE));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_tff_typed_quantifier_variables() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tff-typed-quantifiers");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(p_type, type, p: person > $o).\n\
             tff(q_type, type, q: person * person > $o).\n\
             tff(rule, axiom, ![X: person]:(p(X)=>?[Y: person]:q(X,Y))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_distinct_formula() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-distinct");
        std::fs::write(&path, "fof(test1, axiom, $distinct(a,b,c)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_boolean_ite_formula() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-boolean-ite");
        std::fs::write(&path, "fof(ite_bool, axiom, $ite(p(a), q(a), ~r(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_boolean_let_formula() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-boolean-let");
        std::fs::write(&path, "fof(let_bool, axiom, $let(f:$o, f := p(a), f)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_boolean_ite_formula_equality() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-boolean-ite-equality");
        std::fs::write(
            &path,
            "fof(eq_bool, axiom, ($ite(p(a), q(a), r(a)) = s(a))).\n\
             fof(ne_bool, axiom, ($ite(p(a), q(a), r(a)) != s(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_non_boolean_fool_term_equality() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-non-boolean-fool-term-equality");
        std::fs::write(
            &path,
            "tff(a_type, type, a: $i).\n\
             tff(b_type, type, b: $i).\n\
             tff(c_type, type, c: $i).\n\
             tff(p_type, type, p: $i > $o).\n\
             fof(ite_i_eq, axiom, ($ite(p(a), a, b) = c)).\n\
             fof(let_i_eq, axiom, ($let(f:$i, f := a, f) = b)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_existential_fool_body() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-existential-fool-body");
        std::fs::write(
            &path,
            "tff(a_type, type, a: $i).\n\
             tff(b_type, type, b: $i).\n\
             tff(p_type, type, p: $i > $o).\n\
             tff(q_type, type, q: $i > $o).\n\
             tff(r_type, type, r: $i > $o).\n\
             fof(ex_ite, axiom, ?[X]:$ite(p(X), q(X), r(X))).\n\
             fof(ex_let_eq, axiom, ?[Y]:$let(f:$i, f := a, f) = Y).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_truth_constants() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-truth-constants");
        std::fs::write(
            &path,
            "fof(true_axiom, axiom, $true).\n\
             fof(false_axiom, axiom, $false).\n\
             fof(negated, axiom, ~$false).\n\
             fof(conj, axiom, ($true&p(a))).\n\
             fof(disj, axiom, ($false|p(a))).\n\
             fof(exists, axiom, ?[X]:$true).\n\
             fof(univ, conjecture, ![X]:$false).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_rejects_fof_distinct_mixed_argument_types() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-distinct-mixed-types");
        std::fs::write(
            &path,
            "tff(person_type, type, person: $tType).\n\
             tff(a_type, type, a: person).\n\
             tff(b_type, type, b: $i).\n\
             fof(test1, axiom, $distinct(a,b)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::TYPE_ERROR);
        assert!(error
            .message()
            .contains("All $distinct arguments have to be constants of the same type"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_parenthesized_existential_conjunction() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-parenthesized-existential-conjunction");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:(p(X)&q(X))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_conjunction_with_existential_conjunct() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-conjunction-existential-conjunct");
        std::fs::write(
            &path,
            "fof(ax, axiom, ?[X]:p(X)&q(a)).\n\
             fof(goal, conjecture, ?[Y]:r(Y)&s(a)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_existential_universal_scope() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-existential-nested-quantifier");
        std::fs::write(&path, "fof(test1, axiom, ?[X]:(![Y]:p(X,Y))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_unparenthesized_existential_unitary_bodies() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-unparenthesized-existential-unitary-bodies");
        std::fs::write(
            &path,
            "fof(univ, axiom, ?[X]:![Y]:p(X,Y)).\n\
             fof(ex, axiom, ?[X]:?[Y]:q(X,Y)).\n\
             fof(neg, axiom, ?[X]:~r(X)).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_same_name_shadowed_fof_quantifier() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-shadowed-quantifier");
        std::fs::write(
            &path,
            "fof(test1, axiom, ![X,Y]:((p(X,Y)&?[X]:(q(Y,X)=>q(X,Y))))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_same_sort_shadowed_tff_quantifier() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-tff-shadowed-quantifier");
        std::fs::write(
            &path,
            "tff(p_type, type, p: $i > $o).\n\
             tff(q_type, type, q: $i > $o).\n\
             tff(rule, axiom, ![X: $i]:(p(X)|![X: $i]:q(X))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_implication_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-implication-existential-operand");
        std::fs::write(
            &path,
            "fof(rule1, axiom, ![Y]:(p(Y)=>?[X]:q(X,Y))).\n\
             fof(rule2, axiom, ![Y]:((?[X]:r(X,Y))=>s(Y))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_equivalence_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-equivalence-existential-operand");
        std::fs::write(
            &path,
            "fof(eq1, axiom, ![Y]:(p(Y)<=>?[X]:q(X,Y))).\n\
             fof(eq2, axiom, (r(a)<=>?[X]:s(X))).\n\
             fof(xor, axiom, (m(a)<~>?[X]:n(X))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_nested_existential_in_parenthesized_existential_body() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-existential-nested-existential");
        std::fs::write(
            &path,
            "fof(test1, axiom, ?[X]:(?[Y]:p(X,Y))).\n\
             fof(test2, axiom, ?[X]:(~![Y]:q(X,Y))).\n\
             fof(test3, axiom, ?[X]:(r(X)=>![Y]:s(X,Y))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_negated_universal_with_existential_scope() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-negated-universal-existential");
        std::fs::write(&path, "fof(goal, conjecture, ![Y]: ?[X]:p(X,Y)).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_fof_existential_conjecture_atom() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-existential-conjecture");
        std::fs::write(&path, "fof(goal, conjecture, ?[X]:(p(X))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_xor() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-xor");
        std::fs::write(&path, "fof(goal, conjecture, (p(a)<~>q(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_nand_and_nor() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-nand-nor");
        std::fs::write(
            &path,
            "fof(nand, axiom, (p(a)~&q(a))).\n\
             fof(nor, conjecture, (p(a)~|q(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_supported_nand_nor_existential_operand() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-nand-nor-existential");
        std::fs::write(
            &path,
            "fof(nand, axiom, (p(a)~&?[X]:q(X))).\n\
             fof(nor, axiom, (r(a)~|?[X]:s(X))).\n\
             fof(universal_nand, axiom, ![Y]:(t(Y)~&?[X]:u(X,Y))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_conjecture_equivalence() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-conjecture-equivalence");
        std::fs::write(&path, "fof(goal, conjecture, (p(a)<=>q(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_disjunction_with_conjunctive_operand() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-disjunction-conjunction-mix");
        std::fs::write(&path, "fof(test1, axiom, ((p(a)&q(a)) | r(a))).\n").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_disjunction_with_existential_disjunct() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-disjunction-existential-disjunct");
        std::fs::write(
            &path,
            "fof(either, axiom, ?[X]:p(X)|q(a)).\n\
             fof(split, axiom, ?[Y]:(r(Y)&s(Y))|t(a)).\n\
             fof(conjunctive, axiom, ((u(a)&?[Z]:v(Z))|w(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_parses_fof_conjecture_conjunction_with_implication() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-fof-conjecture-conjunction-imp");
        std::fs::write(
            &path,
            "fof(goal, conjecture, (![X]:(p(X) => q(X)) & r(a))).\n",
        )
        .unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            ["eprover", "--syntax-only", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "\n% Parsing successful!\n% SZS status Unknown\n"
        );
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn run_syntax_only_observes_error_on_empty() {
        let _guard = global_state_lock();
        let path = temp_path("syntax-empty");
        std::fs::write(&path, "").unwrap();
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                "eprover",
                "--syntax-only",
                "--error-on-empty",
                path_arg.as_str(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::INPUT_SEMANTIC_ERROR);
        assert!(error.message().contains("did not contain any clauses"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        std::fs::remove_file(&path).unwrap();
    }
}
