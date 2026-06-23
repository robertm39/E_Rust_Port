use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::basics::error::{check_option_letter_string, Diagnostic, ErrorCode};
use crate::basics::os_wrapper::{get_system_phys_memory, set_memory_limit};
use crate::basics::verbose::set_verbose_level;
use crate::inout::commandline::{
    get_bool_arg, get_int_arg, get_int_arg_check_range, print_options, CommandLineState, ParsedOpt,
};
use crate::inout::output::set_output_level;
use crate::inout::scanner::IoFormat;
use crate::inout::signals::{set_hard_time_limit, set_schedule_time_limit, set_soft_time_limit};
use crate::prover::options::{EProverOption, EPROVER_OPTIONS};
use crate::prover::version::{self, E_NICKNAME, PROGRAM_NAME, VERSION};
use crate::terms::termtypes::RewriteLevel;

const MEGA: u64 = 1_048_576;
const C_INT_MAX: i64 = i32::MAX as i64;
const DEFAULT_DELETE_BAD_LIMIT: i64 = i64::MAX;
const DEFAULT_EQDEF_INCRLIMIT: i64 = 20;
const DEFAULT_EQDEF_MAXCLAUSES: i64 = 20_000;
const DEFAULT_HEURISTIC_NAME: &str = "Default";
const DEFAULT_LAMBDA_WEIGHT: i64 = 20;
const DEFAULT_DB_WEIGHT: i64 = 10;
const DEFAULT_LPO_RECURSION_LIMIT: i64 = 1_000;
const DEFAULT_OUTPUT_DESCRIPTOR: &str = "eigEIG";
const DEFAULT_FILTER_DESCRIPTOR: &str = "Fc";
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreprocessingConfig {
    pub no_preprocessing: bool,
    pub eqdef_maxclauses: i64,
    pub eqdef_incrlimit: i64,
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
    pub literal_comparison: LiteralComparison,
    pub lambda_weight: i64,
    pub db_weight: i64,
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
            literal_comparison: LiteralComparison::Normal,
            lambda_weight: DEFAULT_LAMBDA_WEIGHT,
            db_weight: DEFAULT_DB_WEIGHT,
            rewrite_strong_rhs_inst: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeuristicConfig {
    pub name: String,
    pub prefer_initial_clauses: bool,
    pub filter_orphans_limit: i64,
    pub forward_contract_limit: i64,
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        Self {
            name: DEFAULT_HEURISTIC_NAME.to_owned(),
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
}

impl EProverFlags {
    pub fn set(&mut self, flag: EProverFlag) {
        self.bits |= flag as u32;
    }

    #[must_use]
    pub const fn contains(self, flag: EProverFlag) -> bool {
        (self.bits & flag as u32) != 0
    }
}

impl Default for EProverConfig {
    fn default() -> Self {
        Self {
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
    File(File),
}

impl<W: Write + ?Sized> Write for ConfiguredOutput<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Writer(writer) => writer.write(buffer),
            Self::File(file) => file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Writer(writer) => writer.flush(),
            Self::File(file) => file.flush(),
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
        .map(ConfiguredOutput::File)
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
    if let Some(limit) = config.cpu_limit {
        let _ = set_hard_time_limit(c_rlimit_from_arg(limit));
    }
    if let Some(limit) = config.soft_cpu_limit {
        let _ = set_soft_time_limit(c_rlimit_from_arg(limit));
    }
    if let Some(limit) = config.schedule_time_limit {
        let _ = set_schedule_time_limit(c_rlimit_from_arg(limit));
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
    _stderr: &mut impl Write,
) -> Result<u8, EProverError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv)? {
        EProverAction::Help => {
            stdout.write_all(print_help().as_bytes())?;
            Ok(ErrorCode::NO_ERROR.exit_status())
        }
        EProverAction::Version => {
            stdout.write_all(version::version_line().as_bytes())?;
            Ok(ErrorCode::NO_ERROR.exit_status())
        }
        EProverAction::Run(config) => run_config(stdout, &config),
    }
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
        || is_inference_control_option(option)
        || is_inference_processing_option(option)
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
            config.proof_output = config.proof_output.max(1);
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
        _ => {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                "Option -t (--term-ordering) requires LPO, LPO4, KBO or KBO6 as an argument",
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
    } else if is_inference_control_option(option_code) {
        apply_inference_control_option(config, option_code);
    } else if is_inference_processing_option(option_code) {
        apply_inference_processing_option(config, parsed)?;
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
            config.step_limit = 0;
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
    let verbose = i32::try_from(config.verbose).map_err(|_| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("--verbose argument {} is out of int range", config.verbose),
        )
    })?;
    set_verbose_level(verbose);
    let _ = set_output_level(config.output_level);
    apply_time_limit_state(config);
    let _ = set_memory_limit(config.memory_limit);
    let mut output = open_configured_output(stdout, config.output_file.as_deref())?;

    if config.flags.contains(EProverFlag::PrintPid) {
        writeln!(output, "# Pid: {}", std::process::id())?;
    }
    if config.flags.contains(EProverFlag::PrintVersion) {
        writeln!(output, "# Version: {VERSION}")?;
    }
    output.flush()?;

    Err(Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "Rust eprover proof search is not implemented yet",
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::{
        auto_memory_limit_from_system_mb, process_options, run, AcHandling, DocOutputFormat,
        EProverAction, EProverFlag, FvIndexFeatureType, GroundingStrategy, LiteralComparison,
        ParamodulationType, TermOrdering, WatchlistSource, MEGA,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::{set_verbose_level, verbose_level};
    use crate::inout::output::{output_level, set_output_level};
    use crate::inout::scanner::IoFormat;
    use crate::inout::signals::{
        hard_time_limit, schedule_time_limit, set_hard_time_limit, set_schedule_time_limit,
        set_soft_time_limit, soft_time_limit, RLIM_INFINITY_COMPAT,
    };
    use crate::prover::version::VERSION;
    use crate::terms::termtypes::RewriteLevel;
    use std::sync::{Mutex, OnceLock};

    fn global_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("eprover-{name}-{}.out", std::process::id()))
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

        let action =
            process_options(["eprover", "--soft-cpu-limit=25", "--cpu-limit=100"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.cpu_limit, Some(100));
        assert_eq!(config.soft_cpu_limit, Some(25));
        assert_eq!(config.schedule_time_limit, Some(100));

        let action =
            process_options(["eprover", "--cpu-limit=100", "--soft-cpu-limit=25"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.schedule_time_limit, Some(25));
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
        assert_eq!(config.step_limit, 0);
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
    fn process_options_records_lpo_literal_comparison_fallthrough_like_c() {
        let action = process_options(["eprover", "--lpo-recursion-limit"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.search.ordering.lpo_recursion_limit, 100);
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
        assert_eq!(
            config.search.ordering.literal_comparison,
            LiteralComparison::TfoEqMin
        );

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
            "Option -t (--term-ordering) requires LPO, LPO4, KBO or KBO6 as an argument"
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
    fn run_applies_verbose_option_to_global_gate() {
        let _guard = global_test_lock();
        set_verbose_level(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(["eprover", "--verbose=2"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(verbose_level(), 2);
        set_verbose_level(0);
    }

    #[test]
    fn run_rejects_verbose_values_outside_c_int_range() {
        let _guard = global_test_lock();
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
        let _guard = global_test_lock();
        let _ = set_hard_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_soft_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_schedule_time_limit(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--soft-cpu-limit=25", "--cpu-limit=100"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(hard_time_limit(), 100);
        assert_eq!(soft_time_limit(), 25);
        assert_eq!(schedule_time_limit(), 100);

        let _ = set_hard_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_soft_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_schedule_time_limit(0);
    }

    #[test]
    fn run_applies_output_level_options_to_global_gate() {
        let _guard = global_test_lock();
        let _ = set_output_level(1);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(["eprover", "--silent"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(output_level(), 0);

        let error = run(["eprover", "--output-level=3"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(output_level(), 3);
        let _ = set_output_level(1);
    }

    #[test]
    fn run_print_info_uses_configured_output_target() {
        let _guard = global_test_lock();
        let path = temp_path("print-info");
        let _ = std::fs::remove_file(&path);
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--print-version", "-o", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(stdout.is_empty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!("# Version: {VERSION}\n")
        );
        std::fs::remove_file(&path).unwrap();

        let error = run(
            ["eprover", "--print-version", "-o", "-"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!("# Version: {VERSION}\n")
        );
    }
}
