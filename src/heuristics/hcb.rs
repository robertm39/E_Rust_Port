use crate::basics::defines::bool_to_str;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::{FormulaProperties, CP_DELETE_CLAUSE};
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::neweval::{evals_alloc, EvalCell};
use crate::heuristics::to_params::{
    order_parms_parse_into_report, order_parms_print_string, OrderParmsCell,
};
use crate::heuristics::wfcbadmin::WfcbAdmin;
use crate::inout::basicparser::{parse_bool, parse_int, parse_int_limited, parse_int_max};
use crate::inout::scanner::{describe_token, token_pos_rep, Scanner, TokenType};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::RewriteLevel;

pub const NO_EXT_SUP: i32 = -1;
pub const NO_ELIM_LEIBNIZ: i32 = -1;

pub const HCB_DEFAULT_HEURISTIC: &str = "Default";
pub const DEFAULT_EQDEF_MAXCLAUSES: i64 = 20_000;
pub const DEFAULT_EQDEF_INCRLIMIT: i64 = 20;
pub const DEFAULT_FORMULA_DEF_LIMIT: i64 = 24;
pub const DEFAULT_SYM_OCCS: i32 = 512;
pub const DEFAULT_MINISCOPE_LIMIT: i64 = 1_048_576;
pub const DEFAULT_FILTER_ORPHANS_LIMIT: i64 = i64::MAX;
pub const DEFAULT_FORWARD_CONTRACT_LIMIT: i64 = i64::MAX;
pub const DEFAULT_DELETE_BAD_LIMIT: i64 = i64::MAX;
pub const DEFAULT_RW_BW_INDEX_NAME: &str = "FP7";
pub const DEFAULT_PM_FROM_INDEX_NAME: &str = "FP7";
pub const DEFAULT_PM_INTO_INDEX_NAME: &str = "FP7";
pub const DEFAULT_LITERAL_SELECTION: &str = "NoSelection";
pub const DEFAULT_SAT_CHECK_DECISION_LIMIT: i32 = 10_000;
pub const DEFAULT_MAX_UNIFIERS: i32 = 4;
pub const DEFAULT_MAX_UNIF_STEPS: i32 = 256;
pub const HCB_INITIAL_CAPACITY: usize = 4;
pub const MAX_PM_INDEX_NAME_LEN: usize = 20;

pub const LITERAL_SELECTION_NAMES: &[&str] = &[
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

pub type WfcbHandle = usize;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum AcHandling {
    None = 0,
    #[default]
    DiscardAll = 1,
    KeepUnits = 2,
    KeepOrientable = 3,
}

impl AcHandling {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::DiscardAll),
            2 => Some(Self::KeepUnits),
            3 => Some(Self::KeepOrientable),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
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

impl ParamodulationType {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Plain),
            1 => Some(Self::Sim),
            2 => Some(Self::OrientedSim),
            3 => Some(Self::SuperSim),
            4 => Some(Self::OrientedSuperSim),
            5 => Some(Self::DecreasingSim),
            6 => Some(Self::SizeDecreasingSim),
            _ => None,
        }
    }

    #[must_use]
    pub fn from_name(value: &str) -> Option<Self> {
        [
            Self::Plain,
            Self::Sim,
            Self::OrientedSim,
            Self::SuperSim,
            Self::OrientedSuperSim,
            Self::DecreasingSim,
            Self::SizeDecreasingSim,
        ]
        .into_iter()
        .find(|variant| variant.name() == value)
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Plain => "ParamodPlain",
            Self::Sim => "ParamodSim",
            Self::OrientedSim => "ParamodOrientedSim",
            Self::SuperSim => "ParamodSuperSim",
            Self::OrientedSuperSim => "ParamodOrientedSuperSim",
            Self::DecreasingSim => "ParamodDecreasingSim",
            Self::SizeDecreasingSim => "ParamodSizeDecreasingSim",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum SplitType {
    #[default]
    GroundNone = 0,
    GroundOne = 1,
    GroundFull = 2,
}

impl SplitType {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::GroundNone),
            1 => Some(Self::GroundOne),
            2 => Some(Self::GroundFull),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SplitClassType(i32);

impl SplitClassType {
    pub const NONE: Self = Self(0);
    pub const HORN: Self = Self(1);
    pub const NON_HORN: Self = Self(2);
    pub const NEGATIVE: Self = Self(4);
    pub const POSITIVE: Self = Self(8);
    pub const MIXED: Self = Self(16);
    pub const ALL: Self = Self(7);

    #[must_use]
    pub const fn from_c_value(value: i32) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self.0
    }

    #[must_use]
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
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

impl GroundingStrategy {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::NoGrounding),
            1 => Some(Self::PseudoVar),
            2 => Some(Self::FirstConst),
            3 => Some(Self::ConjMinMinFreq),
            4 => Some(Self::ConjMaxMinFreq),
            5 => Some(Self::ConjMinMaxFreq),
            6 => Some(Self::ConjMaxMaxFreq),
            7 => Some(Self::GlobalMax),
            8 => Some(Self::GlobalMin),
            _ => None,
        }
    }

    #[must_use]
    pub fn from_name(value: &str) -> Option<Self> {
        [
            Self::NoGrounding,
            Self::PseudoVar,
            Self::FirstConst,
            Self::ConjMinMinFreq,
            Self::ConjMaxMinFreq,
            Self::ConjMinMaxFreq,
            Self::ConjMaxMaxFreq,
            Self::GlobalMax,
            Self::GlobalMin,
        ]
        .into_iter()
        .find(|variant| variant.name() == value)
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoGrounding => "NoGrounding",
            Self::PseudoVar => "PseudoVar",
            Self::FirstConst => "FirstConst",
            Self::ConjMinMinFreq => "ConjMinMinFreq",
            Self::ConjMaxMinFreq => "ConjMaxMinFreq",
            Self::ConjMinMaxFreq => "ConjMinMaxFreq",
            Self::ConjMaxMaxFreq => "ConjMaxMaxFreq",
            Self::GlobalMax => "GlobalMax",
            Self::GlobalMin => "GlobalMin",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum ExtInferenceType {
    AllLits = 0,
    MaxLits = 1,
    #[default]
    NoLits = 2,
}

impl ExtInferenceType {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::AllLits),
            1 => Some(Self::MaxLits),
            2 => Some(Self::NoLits),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        ext_inference_type_name_raw(self.c_value())
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

impl PrimEnumMode {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Neg),
            1 => Some(Self::And),
            2 => Some(Self::Or),
            3 => Some(Self::Eq),
            4 => Some(Self::Pragmatic),
            5 => Some(Self::Full),
            6 => Some(Self::LogSymbol),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        prim_enum_mode_name_raw(self.c_value())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum UnifMode {
    #[default]
    Single = 0,
    Multi = 1,
}

impl UnifMode {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Single),
            1 => Some(Self::Multi),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        unif_mode_name_raw(self.c_value())
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeuristicParmsCell {
    pub order_params: OrderParmsCell,
    pub no_preproc: bool,
    pub eqdef_maxclauses: i64,
    pub eqdef_incrlimit: i64,
    pub formula_def_limit: i64,
    pub miniscope_limit: i64,
    pub sine: Option<String>,
    pub add_goal_defs_pos: bool,
    pub add_goal_defs_neg: bool,
    pub add_goal_defs_subterms: bool,
    pub bce: bool,
    pub bce_max_occs: i32,
    pub pred_elim: bool,
    pub pred_elim_gates: bool,
    pub pred_elim_max_occs: i32,
    pub pred_elim_tolerance: i32,
    pub pred_elim_force_mu_decrease: bool,
    pub pred_elim_ignore_conj_syms: bool,
    pub heuristic_name: String,
    pub heuristic_def: Option<String>,
    pub prefer_initial_clauses: bool,
    pub selection_strategy: String,
    pub pos_lit_sel_min: i64,
    pub pos_lit_sel_max: i64,
    pub neg_lit_sel_min: i64,
    pub neg_lit_sel_max: i64,
    pub all_lit_sel_min: i64,
    pub all_lit_sel_max: i64,
    pub weight_sel_min: i64,
    pub select_on_proc_only: bool,
    pub inherit_paramod_lit: bool,
    pub inherit_goal_pm_lit: bool,
    pub inherit_conj_pm_lit: bool,
    pub enable_eq_factoring: bool,
    pub enable_neg_unit_paramod: bool,
    pub enable_given_forward_simpl: bool,
    pub pm_type: ParamodulationType,
    pub ac_handling: AcHandling,
    pub ac_res_aggressive: bool,
    pub forward_context_sr: bool,
    pub forward_context_sr_aggressive: bool,
    pub backward_context_sr: bool,
    pub forward_subsumption_aggressive: bool,
    pub forward_demod: RewriteLevel,
    pub prefer_general: bool,
    pub lambda_demod: bool,
    pub condensing: bool,
    pub condensing_aggressive: bool,
    pub er_varlit_destructive: bool,
    pub er_strong_destructive: bool,
    pub er_aggressive: bool,
    pub split_clauses: SplitClassType,
    pub split_method: SplitType,
    pub split_aggressive: bool,
    pub split_fresh_defs: bool,
    pub diseq_decomposition: i64,
    pub diseq_decomp_maxarity: i64,
    pub rw_bw_index_type: String,
    pub pm_from_index_type: String,
    pub pm_into_index_type: String,
    pub sat_check_grounding: GroundingStrategy,
    pub sat_check_step_limit: i64,
    pub sat_check_size_limit: i64,
    pub sat_check_ttinsert_limit: i64,
    pub sat_check_normconst: bool,
    pub sat_check_normalize: bool,
    pub sat_check_decision_limit: i32,
    pub filter_orphans_limit: i64,
    pub forward_contract_limit: i64,
    pub delete_bad_limit: i64,
    pub mem_limit: u64,
    pub watchlist_simplify: bool,
    pub watchlist_is_static: bool,
    pub use_tptp_sos: bool,
    pub presat_interreduction: bool,
    pub detsort_bw_rw: bool,
    pub detsort_tmpset: bool,
    pub arg_cong: ExtInferenceType,
    pub neg_ext: ExtInferenceType,
    pub pos_ext: ExtInferenceType,
    pub ext_rules_max_depth: i32,
    pub inverse_recognition: bool,
    pub replace_inj_defs: bool,
    pub lift_lambdas: bool,
    pub lambda_to_forall: bool,
    pub unroll_only_formulas: bool,
    pub elim_leibniz_max_depth: i32,
    pub prim_enum_mode: PrimEnumMode,
    pub prim_enum_max_depth: i32,
    pub inst_choice_max_depth: i32,
    pub local_rw: bool,
    pub prune_args: bool,
    pub preinstantiate_induction: bool,
    pub fool_unroll: bool,
    pub func_proj_limit: i32,
    pub imit_limit: i32,
    pub ident_limit: i32,
    pub elim_limit: i32,
    pub unif_mode: UnifMode,
    pub pattern_oracle: bool,
    pub fixpoint_oracle: bool,
    pub max_unifiers: i32,
    pub max_unif_steps: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HeuristicParmsParseReport {
    pub complete: bool,
    pub missing_fields: Vec<&'static str>,
    pub warnings: Vec<Diagnostic>,
}

impl Default for HeuristicParmsCell {
    #[allow(clippy::too_many_lines)]
    fn default() -> Self {
        Self {
            order_params: OrderParmsCell::default(),
            no_preproc: false,
            eqdef_maxclauses: DEFAULT_EQDEF_MAXCLAUSES,
            eqdef_incrlimit: DEFAULT_EQDEF_INCRLIMIT,
            formula_def_limit: DEFAULT_FORMULA_DEF_LIMIT,
            miniscope_limit: DEFAULT_MINISCOPE_LIMIT,
            sine: None,
            add_goal_defs_pos: false,
            add_goal_defs_neg: false,
            add_goal_defs_subterms: false,
            bce: false,
            bce_max_occs: DEFAULT_SYM_OCCS,
            pred_elim: false,
            pred_elim_gates: false,
            pred_elim_max_occs: DEFAULT_SYM_OCCS,
            pred_elim_tolerance: 0,
            pred_elim_force_mu_decrease: false,
            pred_elim_ignore_conj_syms: false,
            heuristic_name: HCB_DEFAULT_HEURISTIC.to_owned(),
            heuristic_def: None,
            prefer_initial_clauses: false,
            selection_strategy: DEFAULT_LITERAL_SELECTION.to_owned(),
            pos_lit_sel_min: 0,
            pos_lit_sel_max: i64::MAX,
            neg_lit_sel_min: 0,
            neg_lit_sel_max: i64::MAX,
            all_lit_sel_min: 0,
            all_lit_sel_max: i64::MAX,
            weight_sel_min: 0,
            select_on_proc_only: false,
            inherit_paramod_lit: false,
            inherit_goal_pm_lit: false,
            inherit_conj_pm_lit: false,
            enable_eq_factoring: true,
            enable_neg_unit_paramod: true,
            enable_given_forward_simpl: true,
            pm_type: ParamodulationType::Plain,
            ac_handling: AcHandling::DiscardAll,
            ac_res_aggressive: true,
            forward_context_sr: false,
            forward_context_sr_aggressive: false,
            backward_context_sr: false,
            forward_subsumption_aggressive: false,
            forward_demod: RewriteLevel::FullRewrite,
            prefer_general: false,
            lambda_demod: false,
            condensing: false,
            condensing_aggressive: false,
            er_varlit_destructive: false,
            er_strong_destructive: false,
            er_aggressive: false,
            split_clauses: SplitClassType::NONE,
            split_method: SplitType::GroundNone,
            split_aggressive: false,
            split_fresh_defs: true,
            diseq_decomposition: 0,
            diseq_decomp_maxarity: i64::MAX,
            rw_bw_index_type: DEFAULT_RW_BW_INDEX_NAME.to_owned(),
            pm_from_index_type: DEFAULT_PM_FROM_INDEX_NAME.to_owned(),
            pm_into_index_type: DEFAULT_PM_INTO_INDEX_NAME.to_owned(),
            sat_check_grounding: GroundingStrategy::NoGrounding,
            sat_check_step_limit: i64::MAX,
            sat_check_size_limit: i64::MAX,
            sat_check_ttinsert_limit: i64::MAX,
            sat_check_normconst: false,
            sat_check_normalize: false,
            sat_check_decision_limit: DEFAULT_SAT_CHECK_DECISION_LIMIT,
            filter_orphans_limit: DEFAULT_FILTER_ORPHANS_LIMIT,
            forward_contract_limit: DEFAULT_FORWARD_CONTRACT_LIMIT,
            delete_bad_limit: DEFAULT_DELETE_BAD_LIMIT,
            mem_limit: 0,
            watchlist_simplify: true,
            watchlist_is_static: false,
            use_tptp_sos: false,
            presat_interreduction: false,
            detsort_bw_rw: false,
            detsort_tmpset: false,
            arg_cong: ExtInferenceType::AllLits,
            neg_ext: ExtInferenceType::NoLits,
            pos_ext: ExtInferenceType::NoLits,
            ext_rules_max_depth: NO_EXT_SUP,
            inverse_recognition: false,
            replace_inj_defs: false,
            lift_lambdas: true,
            lambda_to_forall: true,
            unroll_only_formulas: true,
            elim_leibniz_max_depth: NO_ELIM_LEIBNIZ,
            prim_enum_mode: PrimEnumMode::Pragmatic,
            prim_enum_max_depth: -1,
            inst_choice_max_depth: -1,
            local_rw: false,
            prune_args: false,
            preinstantiate_induction: false,
            fool_unroll: true,
            func_proj_limit: 0,
            imit_limit: 0,
            ident_limit: 0,
            elim_limit: 0,
            unif_mode: UnifMode::Single,
            pattern_oracle: true,
            fixpoint_oracle: true,
            max_unifiers: DEFAULT_MAX_UNIFIERS,
            max_unif_steps: DEFAULT_MAX_UNIF_STEPS,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HcbSelectFunction {
    #[default]
    StandardClauseSelect,
    SingleWeightClauseSelect,
}

pub struct HcbCell<Data = ()> {
    wfcb_list: Vec<WfcbHandle>,
    current_eval: usize,
    select_switch: Vec<i64>,
    select_count: i64,
    hcb_select: HcbSelectFunction,
    hcb_exit: fn(Data),
    data: Option<Data>,
}

impl HcbCell<()> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_data(None, default_exit_fun)
    }
}

impl Default for HcbCell<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Data> HcbCell<Data> {
    #[must_use]
    pub fn with_data(data: Option<Data>, hcb_exit: fn(Data)) -> Self {
        Self {
            wfcb_list: Vec::with_capacity(HCB_INITIAL_CAPACITY),
            current_eval: 0,
            select_switch: Vec::with_capacity(HCB_INITIAL_CAPACITY),
            select_count: 0,
            hcb_select: HcbSelectFunction::StandardClauseSelect,
            hcb_exit,
            data,
        }
    }

    #[must_use]
    pub fn wfcb_no(&self) -> usize {
        self.wfcb_list.len()
    }

    #[must_use]
    pub fn wfcb_capacity(&self) -> usize {
        self.wfcb_list.capacity()
    }

    #[must_use]
    pub fn wfcb_handle(&self, pos: usize) -> Option<WfcbHandle> {
        self.wfcb_list.get(pos).copied()
    }

    #[must_use]
    pub const fn current_eval(&self) -> usize {
        self.current_eval
    }

    #[must_use]
    pub fn select_switch_capacity(&self) -> usize {
        self.select_switch.capacity()
    }

    #[must_use]
    pub fn select_switch(&self, pos: usize) -> Option<i64> {
        self.select_switch.get(pos).copied()
    }

    #[must_use]
    pub const fn select_count(&self) -> i64 {
        self.select_count
    }

    #[must_use]
    pub const fn hcb_select(&self) -> HcbSelectFunction {
        self.hcb_select
    }

    #[must_use]
    pub const fn data(&self) -> Option<&Data> {
        self.data.as_ref()
    }
}

impl<Data> Drop for HcbCell<Data> {
    fn drop(&mut self) {
        if let Some(data) = self.data.take() {
            (self.hcb_exit)(data);
        }
    }
}

#[must_use]
pub fn hcb_alloc() -> HcbCell<()> {
    HcbCell::new()
}

/// Adds an admin-owned WFCB handle to an HCB schedule.
///
/// # Panics
///
/// Panics if `steps` is not positive, matching the C assertion in
/// `HCBAddWFCB`.
pub fn hcb_add_wfcb<Data>(hcb: &mut HcbCell<Data>, wfcb: WfcbHandle, steps: i64) -> usize {
    assert!(steps > 0, "steps must be positive");

    let cumulative_steps = hcb
        .select_switch
        .last()
        .map_or(steps, |previous| previous + steps);
    hcb.wfcb_list.push(wfcb);
    hcb.select_switch.push(cumulative_steps);
    hcb.hcb_select = if hcb.wfcb_no() == 1 {
        HcbSelectFunction::SingleWeightClauseSelect
    } else {
        HcbSelectFunction::StandardClauseSelect
    };
    hcb.wfcb_no()
}

/// Evaluates a clause through every WFCB in `hcb` into an existing
/// evaluation list.
///
/// This keeps an explicit storage adapter for callers that have not moved to
/// clause-owned evaluation cells yet.
///
/// # Panics
///
/// Panics if `evaluations` does not have one slot per HCB WFCB, if an HCB
/// WFCB handle does not exist in `admin`, or if a WFCB writes outside the
/// evaluation cell.
pub fn hcb_clause_evaluate_into<Data>(
    hcb: &HcbCell<Data>,
    admin: &mut WfcbAdmin,
    evaluations: &mut EvalCell,
    bank: &TermBank,
    clause: &Clause,
) {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::ClauseEvalTimer,
    );
    assert_eq!(
        evaluations.eval_no(),
        hcb.wfcb_no(),
        "evaluation width must match HCB WFCB count"
    );

    let empty = clause.is_sem_false();
    for (pos, wfcb_handle) in hcb.wfcb_list.iter().copied().enumerate() {
        let wfcb = admin
            .wfcb_mut(wfcb_handle)
            .unwrap_or_else(|| panic!("unknown WFCB handle {wfcb_handle}"));
        wfcb.add_evaluation(evaluations, bank, clause, pos, empty);
    }
}

/// Evaluates a mutable clause through every WFCB in `hcb` with explicit
/// owner-bank ordering context.
///
/// # Errors
///
/// Returns a diagnostic from bank-backed ordering preparation.
///
/// # Panics
///
/// Panics under the same handle and evaluation-width conditions as
/// [`hcb_clause_evaluate_into`].
pub fn hcb_clause_evaluate_into_with_bank<Data>(
    hcb: &HcbCell<Data>,
    admin: &mut WfcbAdmin,
    evaluations: &mut EvalCell,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<(), Diagnostic> {
    let _timer = crate::basics::perf_counters::start(
        crate::basics::perf_counters::PerfCounter::ClauseEvalTimer,
    );
    assert_eq!(
        evaluations.eval_no(),
        hcb.wfcb_no(),
        "evaluation width must match HCB WFCB count"
    );

    let empty = clause.is_sem_false();
    for (pos, wfcb_handle) in hcb.wfcb_list.iter().copied().enumerate() {
        let wfcb = admin
            .wfcb_mut(wfcb_handle)
            .unwrap_or_else(|| panic!("unknown WFCB handle {wfcb_handle}"));
        wfcb.add_evaluation_with_bank(evaluations, ocb, bank, clause, pos, empty)?;
    }
    Ok(())
}

/// Evaluates a clause through every WFCB in `hcb` and stores the resulting
/// evaluation list on the clause.
///
/// # Panics
///
/// Panics if `clause` already has evaluations or if an HCB WFCB handle does
/// not exist in `admin`.
pub fn hcb_clause_evaluate<Data>(
    hcb: &HcbCell<Data>,
    admin: &mut WfcbAdmin,
    bank: &TermBank,
    clause: &mut Clause,
) {
    assert!(
        clause.evaluations().is_none(),
        "clause must not already have evaluations"
    );
    let mut evaluations = evals_alloc(hcb.wfcb_no());
    hcb_clause_evaluate_into(hcb, admin, &mut evaluations, bank, clause);
    clause.add_eval_cell(evaluations);
}

/// Evaluates a clause through every WFCB in `hcb` with mutable owner-bank
/// context and stores the resulting evaluation list on the clause.
///
/// # Errors
///
/// Returns a diagnostic from bank-backed ordering preparation.
///
/// # Panics
///
/// Panics if `clause` already has evaluations or if an HCB WFCB handle does
/// not exist in `admin`.
pub fn hcb_clause_evaluate_with_bank<Data>(
    hcb: &HcbCell<Data>,
    admin: &mut WfcbAdmin,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    clause: &mut Clause,
) -> Result<(), Diagnostic> {
    assert!(
        clause.evaluations().is_none(),
        "clause must not already have evaluations"
    );
    let mut evaluations = evals_alloc(hcb.wfcb_no());
    hcb_clause_evaluate_into_with_bank(hcb, admin, &mut evaluations, ocb, bank, clause)?;
    clause.add_eval_cell(evaluations);
    Ok(())
}

pub fn hcb_clause_set_reweight<Data>(
    hcb: &HcbCell<Data>,
    admin: &mut WfcbAdmin,
    bank: &TermBank,
    set: &mut ClauseSet,
) {
    set.remove_evaluations();
    for clause in set.iter_mut() {
        hcb_clause_evaluate(hcb, admin, bank, clause);
    }
    set.rebuild_eval_indices();
}

/// Re-evaluates every clause in a set with mutable owner-bank context.
///
/// # Errors
///
/// Returns a diagnostic from bank-backed ordering preparation.
pub fn hcb_clause_set_reweight_with_bank<Data>(
    hcb: &HcbCell<Data>,
    admin: &mut WfcbAdmin,
    ocb: &mut OrderControlBlock,
    bank: &mut TermBank,
    set: &mut ClauseSet,
) -> Result<(), Diagnostic> {
    set.remove_evaluations();
    for clause in set.iter_mut() {
        hcb_clause_evaluate_with_bank(hcb, admin, ocb, bank, clause)?;
    }
    set.rebuild_eval_indices();
    Ok(())
}

/// Returns the current evaluation index and advances the standard-selection
/// schedule.
///
/// This is the scheduling mutation performed by `HCBStandardClauseSelect`
/// after `ClauseSetFindBest` and orphan deletion. [`ClauseSet`] owns the
/// evaluation roots and exact object-to-clause lookup; this helper isolates the
/// independent HCB schedule transition.
///
/// # Panics
///
/// Panics if a non-empty HCB has an invalid current evaluation index.
#[must_use]
pub fn hcb_standard_selection_eval_and_advance<Data>(hcb: &mut HcbCell<Data>) -> Option<usize> {
    if hcb.wfcb_no() == 0 {
        return None;
    }
    assert!(
        hcb.current_eval < hcb.wfcb_no(),
        "current evaluation index must reference an HCB WFCB"
    );

    let selected_eval = hcb.current_eval;
    hcb.select_count += 1;
    while hcb.select_count == hcb.select_switch[hcb.current_eval] {
        hcb.current_eval += 1;
        if hcb.current_eval == hcb.wfcb_no() {
            hcb.select_count = 0;
            hcb.current_eval = 0;
            break;
        }
    }
    Some(selected_eval)
}

pub fn hcb_standard_clause_select<Data>(
    hcb: &mut HcbCell<Data>,
    set: &mut ClauseSet,
) -> Option<Clause> {
    hcb_standard_clause_select_with(hcb, set, |_| false)
}

/// Selects and extracts the next clause using the C `HCBStandardClauseSelect`
/// schedule, deleting clauses reported as orphaned by `is_orphaned`.
///
/// The default `hcb_standard_clause_select` keeps the low-level no-op
/// predicate; proof-control callers use the `_with` form with proof-state
/// parent-liveness data.
///
/// # Panics
///
/// Panics if a non-empty HCB has an invalid current evaluation index.
pub fn hcb_standard_clause_select_with<Data>(
    hcb: &mut HcbCell<Data>,
    set: &mut ClauseSet,
    mut is_orphaned: impl FnMut(&Clause) -> bool,
) -> Option<Clause> {
    if hcb.wfcb_no() == 0 {
        return None;
    }
    assert!(
        hcb.current_eval < hcb.wfcb_no(),
        "current evaluation index must reference an HCB WFCB"
    );
    let selected_eval = hcb.current_eval;
    let selected = select_best_non_orphan(set, selected_eval, &mut is_orphaned);
    let _ = hcb_standard_selection_eval_and_advance(hcb);
    selected
}

pub fn hcb_single_weight_clause_select<Data>(
    hcb: &HcbCell<Data>,
    set: &mut ClauseSet,
) -> Option<Clause> {
    hcb_single_weight_clause_select_with(hcb, set, |_| false)
}

/// Selects and extracts the best clause for the HCB's current evaluation index,
/// deleting clauses reported as orphaned by `is_orphaned`.
///
/// # Panics
///
/// Panics if a non-empty HCB has an invalid current evaluation index.
pub fn hcb_single_weight_clause_select_with<Data>(
    hcb: &HcbCell<Data>,
    set: &mut ClauseSet,
    mut is_orphaned: impl FnMut(&Clause) -> bool,
) -> Option<Clause> {
    if hcb.wfcb_no() == 0 {
        return None;
    }
    assert!(
        hcb.current_eval < hcb.wfcb_no(),
        "current evaluation index must reference an HCB WFCB"
    );
    select_best_non_orphan(set, hcb.current_eval, &mut is_orphaned)
}

fn select_best_non_orphan(
    set: &mut ClauseSet,
    eval_index: usize,
    is_orphaned: &mut impl FnMut(&Clause) -> bool,
) -> Option<Clause> {
    loop {
        let orphaned = {
            let clause = set.find_best(eval_index)?;
            is_orphaned(clause)
        };
        let clause = set.extract_best(eval_index)?;
        if !orphaned {
            return Some(clause);
        }
    }
}

pub fn hcb_clause_set_del_prop<Data>(
    hcb: &HcbCell<Data>,
    set: &mut ClauseSet,
    mut number: i64,
    prop: FormulaProperties,
) -> i64 {
    if number <= 0 || hcb.wfcb_no() == 0 {
        return 0;
    }

    let eval_orders = (0..hcb.wfcb_no())
        .map(|idx| set.eval_order_objects(idx))
        .collect::<Vec<_>>();
    let mut positions = vec![0; hcb.wfcb_no()];
    let c_loop_iterations = hcb_delprop_c_loop_iterations(hcb);
    if c_loop_iterations == 0 {
        return 0;
    }

    let mut prop_cleared = 0;
    while number > 0 {
        for idx in 0..hcb.wfcb_no() {
            for _ in 0..c_loop_iterations {
                while let Some(&object) = eval_orders[idx].get(positions[idx]) {
                    positions[idx] += 1;
                    if set.del_prop_by_eval_object(object, prop) {
                        prop_cleared += 1;
                        break;
                    }
                }

                number -= 1;
                if number == 0 {
                    break;
                }
            }
            if number == 0 {
                break;
            }
        }
    }
    prop_cleared
}

pub fn hcb_clause_set_delete_bad_clauses<Data>(
    hcb: &HcbCell<Data>,
    set: &mut ClauseSet,
    number: i64,
) -> i64 {
    set.set_prop(CP_DELETE_CLAUSE);
    let _ = hcb_clause_set_del_prop(hcb, set, number, CP_DELETE_CLAUSE);
    set.delete_marked_entries()
}

fn hcb_delprop_c_loop_iterations<Data>(hcb: &HcbCell<Data>) -> usize {
    let mut j = 0_usize;
    while i64::try_from(j).is_ok_and(|index| index < hcb.select_switch.get(j).copied().unwrap_or(0))
    {
        j += 1;
    }
    j
}

pub fn default_exit_fun<Data>(_data: Data) {}

#[must_use]
pub fn heuristic_parms_alloc() -> HeuristicParmsCell {
    HeuristicParmsCell::default()
}

pub fn heuristic_parms_initialize(handle: &mut HeuristicParmsCell) {
    *handle = HeuristicParmsCell::default();
}

pub fn heuristic_parms_parse(
    scanner: &mut Scanner,
    warn_missing: bool,
) -> Result<HeuristicParmsCell, Diagnostic> {
    let mut result = heuristic_parms_alloc();
    heuristic_parms_parse_into(scanner, &mut result, warn_missing)?;
    Ok(result)
}

pub fn heuristic_parms_parse_into(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    warn_missing: bool,
) -> Result<bool, Diagnostic> {
    Ok(heuristic_parms_parse_into_report(scanner, handle, warn_missing)?.complete)
}

#[expect(
    clippy::too_many_lines,
    reason = "C-compatible parser preserves the explicit HeuristicParmsParseInto field order"
)]
pub fn heuristic_parms_parse_into_report(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    warn_missing: bool,
) -> Result<HeuristicParmsParseReport, Diagnostic> {
    let mut report = HeuristicParmsParseReport {
        complete: true,
        ..HeuristicParmsParseReport::default()
    };

    scanner.accept_tok(TokenType::OPEN_CURLY)?;
    if scanner.test_tok(TokenType::OPEN_CURLY) {
        let order_report =
            order_parms_parse_into_report(scanner, &mut handle.order_params, warn_missing)?;
        report.complete &= order_report.complete;
        report.missing_fields.extend(order_report.missing_fields);
        report.warnings.extend(order_report.warnings);
    } else {
        note_missing(&mut report, "ordering information", warn_missing);
    }

    parse_bool_field(
        scanner,
        "no_preproc",
        &mut handle.no_preproc,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "eqdef_maxclauses",
        &mut handle.eqdef_maxclauses,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "eqdef_incrlimit",
        &mut handle.eqdef_incrlimit,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "formula_def_limit",
        &mut handle.formula_def_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "miniscope_limit",
        &mut handle.miniscope_limit,
        &mut report,
        warn_missing,
    )?;
    parse_string_field(scanner, "sine", &mut handle.sine, &mut report, warn_missing)?;
    parse_bool_field(
        scanner,
        "add_goal_defs_pos",
        &mut handle.add_goal_defs_pos,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "add_goal_defs_neg",
        &mut handle.add_goal_defs_neg,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "add_goal_defs_subterms",
        &mut handle.add_goal_defs_subterms,
        &mut report,
        warn_missing,
    )?;
    parse_identifier_field(
        scanner,
        "heuristic_name",
        &mut handle.heuristic_name,
        &mut report,
        warn_missing,
    )?;
    parse_string_field(
        scanner,
        "heuristic_def",
        &mut handle.heuristic_def,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "prefer_initial_clauses",
        &mut handle.prefer_initial_clauses,
        &mut report,
        warn_missing,
    )?;
    parse_selection_strategy_field(scanner, handle, &mut report, warn_missing)?;
    parse_i64_field(
        scanner,
        "pos_lit_sel_min",
        &mut handle.pos_lit_sel_min,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "pos_lit_sel_max",
        &mut handle.pos_lit_sel_max,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "neg_lit_sel_min",
        &mut handle.neg_lit_sel_min,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "neg_lit_sel_max",
        &mut handle.neg_lit_sel_max,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "all_lit_sel_min",
        &mut handle.all_lit_sel_min,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "all_lit_sel_max",
        &mut handle.all_lit_sel_max,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "weight_sel_min",
        &mut handle.weight_sel_min,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "select_on_proc_only",
        &mut handle.select_on_proc_only,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "inherit_paramod_lit",
        &mut handle.inherit_paramod_lit,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "inherit_goal_pm_lit",
        &mut handle.inherit_goal_pm_lit,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "inherit_conj_pm_lit",
        &mut handle.inherit_conj_pm_lit,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "enable_eq_factoring",
        &mut handle.enable_eq_factoring,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "enable_neg_unit_paramod",
        &mut handle.enable_neg_unit_paramod,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "enable_given_forward_simpl",
        &mut handle.enable_given_forward_simpl,
        &mut report,
        warn_missing,
    )?;
    parse_paramod_type_field(scanner, handle, &mut report, warn_missing)?;
    parse_ac_handling_field(scanner, handle, &mut report, warn_missing)?;
    parse_bool_field(
        scanner,
        "ac_res_aggressive",
        &mut handle.ac_res_aggressive,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "forward_context_sr",
        &mut handle.forward_context_sr,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "forward_context_sr_aggressive",
        &mut handle.forward_context_sr_aggressive,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "backward_context_sr",
        &mut handle.backward_context_sr,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "forward_subsumption_aggressive",
        &mut handle.forward_subsumption_aggressive,
        &mut report,
        warn_missing,
    )?;
    parse_forward_demod_field(scanner, handle, &mut report, warn_missing)?;
    parse_bool_field(
        scanner,
        "prefer_general",
        &mut handle.prefer_general,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "condensing",
        &mut handle.condensing,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "condensing_aggressive",
        &mut handle.condensing_aggressive,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "er_varlit_destructive",
        &mut handle.er_varlit_destructive,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "er_strong_destructive",
        &mut handle.er_strong_destructive,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "er_aggressive",
        &mut handle.er_aggressive,
        &mut report,
        warn_missing,
    )?;
    parse_split_class_field(scanner, handle, &mut report, warn_missing)?;
    parse_split_method_field(scanner, handle, &mut report, warn_missing)?;
    parse_bool_field(
        scanner,
        "split_aggressive",
        &mut handle.split_aggressive,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "split_fresh_defs",
        &mut handle.split_fresh_defs,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "diseq_decomposition",
        &mut handle.diseq_decomposition,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "diseq_decomp_maxarity",
        &mut handle.diseq_decomp_maxarity,
        &mut report,
        warn_missing,
    )?;
    parse_index_name_field(
        scanner,
        "rw_bw_index_type",
        &mut handle.rw_bw_index_type,
        &mut report,
        warn_missing,
    )?;
    parse_index_name_field(
        scanner,
        "pm_from_index_type",
        &mut handle.pm_from_index_type,
        &mut report,
        warn_missing,
    )?;
    parse_index_name_field(
        scanner,
        "pm_into_index_type",
        &mut handle.pm_into_index_type,
        &mut report,
        warn_missing,
    )?;
    parse_grounding_strategy_field(scanner, handle, &mut report, warn_missing)?;
    parse_i64_field(
        scanner,
        "sat_check_step_limit",
        &mut handle.sat_check_step_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "sat_check_size_limit",
        &mut handle.sat_check_size_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "sat_check_ttinsert_limit",
        &mut handle.sat_check_ttinsert_limit,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "sat_check_normconst",
        &mut handle.sat_check_normconst,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "sat_check_normalize",
        &mut handle.sat_check_normalize,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "sat_check_decision_limit",
        &mut handle.sat_check_decision_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "filter_orphans_limit",
        &mut handle.filter_orphans_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "forward_contract_limit",
        &mut handle.forward_contract_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i64_field(
        scanner,
        "delete_bad_limit",
        &mut handle.delete_bad_limit,
        &mut report,
        warn_missing,
    )?;
    parse_mem_limit_field(scanner, handle, &mut report, warn_missing)?;
    parse_bool_field(
        scanner,
        "watchlist_simplify",
        &mut handle.watchlist_simplify,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "watchlist_is_static",
        &mut handle.watchlist_is_static,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "use_tptp_sos",
        &mut handle.use_tptp_sos,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "presat_interreduction",
        &mut handle.presat_interreduction,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "detsort_bw_rw",
        &mut handle.detsort_bw_rw,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "detsort_tmpset",
        &mut handle.detsort_tmpset,
        &mut report,
        warn_missing,
    )?;
    parse_ext_inference_type_field(
        scanner,
        "arg_cong",
        &mut handle.arg_cong,
        &mut report,
        warn_missing,
    )?;
    parse_ext_inference_type_field(
        scanner,
        "neg_ext",
        &mut handle.neg_ext,
        &mut report,
        warn_missing,
    )?;
    parse_ext_inference_type_field(
        scanner,
        "pos_ext",
        &mut handle.pos_ext,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "ext_rules_max_depth",
        &mut handle.ext_rules_max_depth,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "inverse_recognition",
        &mut handle.inverse_recognition,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "replace_inj_defs",
        &mut handle.replace_inj_defs,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "lift_lambdas",
        &mut handle.lift_lambdas,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "lambda_to_forall",
        &mut handle.lambda_to_forall,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "unroll_only_formulas",
        &mut handle.unroll_only_formulas,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "elim_leibniz_max_depth",
        &mut handle.elim_leibniz_max_depth,
        &mut report,
        warn_missing,
    )?;
    parse_prim_enum_mode_field(scanner, handle, &mut report, warn_missing)?;
    parse_i32_field(
        scanner,
        "prim_enum_max_depth",
        &mut handle.prim_enum_max_depth,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "inst_choice_max_depth",
        &mut handle.inst_choice_max_depth,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "local_rw",
        &mut handle.local_rw,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "prune_args",
        &mut handle.prune_args,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "preinstantiate_induction",
        &mut handle.preinstantiate_induction,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "fool_unroll",
        &mut handle.fool_unroll,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "func_proj_limit",
        &mut handle.func_proj_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "imit_limit",
        &mut handle.imit_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "ident_limit",
        &mut handle.ident_limit,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "elim_limit",
        &mut handle.elim_limit,
        &mut report,
        warn_missing,
    )?;
    parse_unif_mode_field(scanner, handle, &mut report, warn_missing)?;
    parse_bool_field(
        scanner,
        "pattern_oracle",
        &mut handle.pattern_oracle,
        &mut report,
        warn_missing,
    )?;
    parse_bool_field(
        scanner,
        "fixpoint_oracle",
        &mut handle.fixpoint_oracle,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "max_unifiers",
        &mut handle.max_unifiers,
        &mut report,
        warn_missing,
    )?;
    parse_i32_field(
        scanner,
        "max_unif_steps",
        &mut handle.max_unif_steps,
        &mut report,
        warn_missing,
    )?;
    scanner.accept_tok(TokenType::CLOSE_CURLY)?;
    Ok(report)
}

fn parse_bool_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut bool,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        *target = parse_bool(scanner)?;
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_i64_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut i64,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        *target = parse_int(scanner)?;
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_i32_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut i32,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        let value = parse_int(scanner)?;
        *target = i64_to_i32(scanner, value)?;
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_identifier_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut String,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        *target = parse_identifier(scanner)?;
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_index_name_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut String,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        *target = truncate_c_identifier(&parse_identifier(scanner)?);
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_string_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut Option<String>,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, name)? {
        *target = Some(parse_c_string(scanner)?);
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_selection_strategy_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "selection_strategy")? {
        handle.selection_strategy = parse_named_identifier(scanner, LITERAL_SELECTION_NAMES)?;
    } else {
        note_missing(report, "selection_strategy", warn_missing);
    }
    Ok(())
}

fn parse_paramod_type_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    const PARAMODULATION_NAMES: &[&str] = &[
        "ParamodPlain",
        "ParamodSim",
        "ParamodOrientedSim",
        "ParamodSuperSim",
        "ParamodOrientedSuperSim",
        "ParamodDecreasingSim",
        "ParamodSizeDecreasingSim",
    ];

    if parse_field_prefix(scanner, "pm_type")? {
        let value = parse_named_identifier(scanner, PARAMODULATION_NAMES)?;
        handle.pm_type = ParamodulationType::from_name(&value).ok_or_else(|| {
            Diagnostic::new(ErrorCode::OTHER_ERROR, "paramodulation name table mismatch")
        })?;
    } else {
        note_missing(report, "pm_type", warn_missing);
    }
    Ok(())
}

fn parse_ac_handling_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "ac_handling")? {
        let parsed = parse_int(scanner)?;
        let value = i64_to_i32(scanner, parsed)?;
        handle.ac_handling = AcHandling::from_c_value(value)
            .ok_or_else(|| enum_value_error(scanner, "AcHandling"))?;
    } else {
        note_missing(report, "ac_handling", warn_missing);
    }
    Ok(())
}

fn parse_forward_demod_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "forward_demod")? {
        handle.forward_demod = match parse_int_limited(scanner, 0, 2)? {
            0 => RewriteLevel::NoRewrite,
            1 => RewriteLevel::RuleRewrite,
            2 => RewriteLevel::FullRewrite,
            _ => unreachable!("ParseIntLimited accepted only 0..=2"),
        };
    } else {
        note_missing(report, "forward_demod", warn_missing);
    }
    Ok(())
}

fn parse_split_class_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "split_clauses")? {
        let parsed = parse_int(scanner)?;
        let value = i64_to_i32(scanner, parsed)?;
        handle.split_clauses = SplitClassType::from_c_value(value);
    } else {
        note_missing(report, "split_clauses", warn_missing);
    }
    Ok(())
}

fn parse_split_method_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "split_method")? {
        let parsed = parse_int_limited(scanner, 0, 2)?;
        let value = i64_to_i32(scanner, parsed)?;
        handle.split_method =
            SplitType::from_c_value(value).ok_or_else(|| enum_value_error(scanner, "SplitType"))?;
    } else {
        note_missing(report, "split_method", warn_missing);
    }
    Ok(())
}

fn parse_grounding_strategy_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
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

    if parse_field_prefix(scanner, "sat_check_grounding")? {
        let value = parse_named_identifier(scanner, GROUNDING_STRATEGY_NAMES)?;
        handle.sat_check_grounding = GroundingStrategy::from_name(&value).ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "grounding strategy name table mismatch",
            )
        })?;
    } else {
        note_missing(report, "sat_check_grounding", warn_missing);
    }
    Ok(())
}

fn parse_mem_limit_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    if parse_field_prefix(scanner, "mem_limit")? {
        handle.mem_limit = intmax_to_u64(parse_int_max(scanner)?);
    } else {
        note_missing(report, "mem_limit", warn_missing);
    }
    Ok(())
}

fn parse_ext_inference_type_field(
    scanner: &mut Scanner,
    name: &'static str,
    target: &mut ExtInferenceType,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    const EXT_INFERENCE_TYPE_NAMES: &[&str] = &["all", "max", "off"];

    if parse_field_prefix(scanner, name)? {
        scanner.check_tok(TokenType::STRING | TokenType::IDENTIFIER)?;
        let value = scanner.current_token().literal();
        *target = str_to_ext_inference_type(&value)
            .ok_or_else(|| named_value_error(scanner, EXT_INFERENCE_TYPE_NAMES))?;
        scanner.next_token()?;
    } else {
        note_missing(report, name, warn_missing);
    }
    Ok(())
}

fn parse_prim_enum_mode_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    const PRIM_ENUM_MODE_NAMES: &[&str] =
        &["neg", "and", "or", "eq", "pragmatic", "full", "logsymbol"];

    if parse_field_prefix(scanner, "prim_enum_mode")? {
        scanner.check_tok(TokenType::STRING | TokenType::IDENTIFIER)?;
        let value = scanner.current_token().literal();
        handle.prim_enum_mode = str_to_prim_enum_mode(&value)
            .ok_or_else(|| named_value_error(scanner, PRIM_ENUM_MODE_NAMES))?;
        scanner.next_token()?;
    } else {
        note_missing(report, "prim_enum_mode", warn_missing);
    }
    Ok(())
}

fn parse_unif_mode_field(
    scanner: &mut Scanner,
    handle: &mut HeuristicParmsCell,
    report: &mut HeuristicParmsParseReport,
    warn_missing: bool,
) -> Result<(), Diagnostic> {
    const UNIF_MODE_NAMES: &[&str] = &["single", "multi"];

    if parse_field_prefix(scanner, "unif_mode")? {
        scanner.check_tok(TokenType::STRING | TokenType::IDENTIFIER)?;
        let value = scanner.current_token().literal();
        handle.unif_mode =
            str_to_unif_mode(&value).ok_or_else(|| named_value_error(scanner, UNIF_MODE_NAMES))?;
        scanner.next_token()?;
    } else {
        note_missing(report, "unif_mode", warn_missing);
    }
    Ok(())
}

fn parse_field_prefix(scanner: &mut Scanner, name: &str) -> Result<bool, Diagnostic> {
    if scanner.test_id(name) {
        scanner.next_token()?;
        scanner.accept_tok(TokenType::COLON)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn parse_identifier(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    scanner.check_tok(TokenType::IDENTIFIER)?;
    let result = scanner.current_token().literal();
    scanner.next_token()?;
    Ok(result)
}

fn parse_named_identifier(scanner: &mut Scanner, names: &[&str]) -> Result<String, Diagnostic> {
    scanner.check_tok(TokenType::IDENTIFIER)?;
    let result = scanner.current_token().literal();
    if !names.contains(&result.as_str()) {
        return Err(named_value_error(scanner, names));
    }
    scanner.next_token()?;
    Ok(result)
}

fn parse_c_string(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    scanner.check_tok(TokenType::STRING)?;
    let bytes = scanner.current_token().literal_bytes();
    if bytes.len() < 2 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Quoted string literal is too short",
        ));
    }
    let result = String::from_utf8_lossy(&bytes[1..bytes.len() - 1]).into_owned();
    scanner.next_token()?;
    Ok(result)
}

fn note_missing(report: &mut HeuristicParmsParseReport, name: &'static str, warn_missing: bool) {
    report.complete = false;
    report.missing_fields.push(name);
    if warn_missing {
        report.warnings.push(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            format!("Config misses {name}"),
        ));
    }
}

fn named_value_error(scanner: &Scanner, names: &[&str]) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): Identifier ({}) expected, but {}('{}') read ",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal(),
            names.join("|"),
            describe_token(scanner.current_token().kind()),
            scanner.current_token().literal()
        ),
    )
}

fn enum_value_error(scanner: &Scanner, type_name: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): invalid {type_name} value",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal()
        ),
    )
}

fn i64_to_i32(scanner: &Scanner, value: i64) -> Result<i32, Diagnostic> {
    i32::try_from(value).map_err(|_| {
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{}(just read '{}'): integer does not fit int",
                token_pos_rep(scanner.current_token()),
                scanner.current_token().literal()
            ),
        )
    })
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
fn intmax_to_u64(value: i128) -> u64 {
    value as u64
}

fn truncate_c_identifier(value: &str) -> String {
    let max_bytes = MAX_PM_INDEX_NAME_LEN - 1;
    let bytes = value.as_bytes();
    let len = bytes.len().min(max_bytes);
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn heuristic_parms_print_string(handle: &HeuristicParmsCell) -> String {
    format!(
        concat!(
            "{{\n",
            "{}",
            "   no_preproc:                     {}\n",
            "   eqdef_maxclauses:               {}\n",
            "   eqdef_incrlimit:                {}\n",
            "   formula_def_limit:              {}\n",
            "   miniscope_limit:                {}\n",
            "   sine:                           \"{}\"\n",
            "   add_goal_defs_pos:             {}\n",
            "   add_goal_defs_neg:             {}\n",
            "   add_goal_defs_subterms:        {}\n",
            "   heuristic_name:                {}\n",
            "   heuristic_def:                 \"{}\"\n",
            "   prefer_initial_clauses:         {}\n",
            "   selection_strategy:             {}\n",
            "   pos_lit_sel_min:                {}\n",
            "   pos_lit_sel_max:                {}\n",
            "   neg_lit_sel_min:                {}\n",
            "   neg_lit_sel_max:                {}\n",
            "   all_lit_sel_min:                {}\n",
            "   all_lit_sel_max:                {}\n",
            "   weight_sel_min:                 {}\n",
            "   select_on_proc_only:            {}\n",
            "   inherit_paramod_lit:            {}\n",
            "   inherit_goal_pm_lit:            {}\n",
            "   inherit_conj_pm_lit:            {}\n",
            "   enable_eq_factoring:            {}\n",
            "   enable_neg_unit_paramod:        {}\n",
            "   enable_given_forward_simpl:     {}\n",
            "   pm_type:                        {}\n",
            "   ac_handling:                    {}\n",
            "   ac_res_aggressive:              {}\n",
            "   forward_context_sr:             {}\n",
            "   forward_context_sr_aggressive:  {}\n",
            "   backward_context_sr:            {}\n",
            "   forward_subsumption_aggressive: {}\n",
            "   forward_demod:                  {}\n",
            "   prefer_general:                 {}\n",
            "   condensing:                     {}\n",
            "   condensing_aggressive:          {}\n",
            "   er_varlit_destructive:          {}\n",
            "   er_strong_destructive:          {}\n",
            "   er_aggressive:                  {}\n",
            "   split_clauses:                  {}\n",
            "   split_method:                   {}\n",
            "   split_aggressive:               {}\n",
            "   split_fresh_defs:               {}\n",
            "   diseq_decomposition:            {}\n",
            "   diseq_decomp_maxarity:          {}\n",
            "   rw_bw_index_type:               {}\n",
            "   pm_from_index_type:             {}\n",
            "   pm_into_index_type:             {}\n",
            "   sat_check_grounding:            {}\n",
            "   sat_check_step_limit:           {}\n",
            "   sat_check_size_limit:           {}\n",
            "   sat_check_ttinsert_limit:       {}\n",
            "   sat_check_normconst:            {}\n",
            "   sat_check_normalize:            {}\n",
            "   sat_check_decision_limit:       {}\n",
            "   filter_orphans_limit:           {}\n",
            "   forward_contract_limit:         {}\n",
            "   delete_bad_limit:               {}\n",
            "   mem_limit:                      {}\n",
            "   watchlist_simplify:             {}\n",
            "   watchlist_is_static:            {}\n",
            "   use_tptp_sos:                   {}\n",
            "   presat_interreduction:          {}\n",
            "   detsort_bw_rw:                  {}\n",
            "   detsort_tmpset:                 {}\n",
            "   arg_cong:                       {}\n",
            "   neg_ext:                        {}\n",
            "   pos_ext:                        {}\n",
            "   ext_rules_max_depth:            {}\n",
            "   inverse_recognition:            {}\n",
            "   replace_inj_defs:               {}\n",
            "   lift_lambdas:                  {}\n",
            "   lambda_to_forall:              {}\n",
            "   unroll_only_formulas:          {}\n",
            "   elim_leibniz_max_depth:        {}\n",
            "   prim_enum_mode:                {}\n",
            "   prim_enum_max_depth:           {}\n",
            "   inst_choice_max_depth:         {}\n",
            "   local_rw:                      {}\n",
            "   prune_args:                    {}\n",
            "   preinstantiate_induction:      {}\n",
            "   fool_unroll:                   {}\n",
            "   func_proj_limit:               {}\n",
            "   imit_limit:                    {}\n",
            "   ident_limit:                   {}\n",
            "   elim_limit:                    {}\n",
            "   unif_mode:                     {}\n",
            "   pattern_oracle:                {}\n",
            "   fixpoint_oracle:               {}\n",
            "   max_unifiers:                  {}\n",
            "   max_unif_steps:                {}\n",
            "}}\n"
        ),
        order_parms_print_string(&handle.order_params),
        bool_name(handle.no_preproc),
        handle.eqdef_maxclauses,
        handle.eqdef_incrlimit,
        handle.formula_def_limit,
        handle.miniscope_limit,
        handle.sine.as_deref().unwrap_or("None"),
        bool_name(handle.add_goal_defs_pos),
        bool_name(handle.add_goal_defs_neg),
        bool_name(handle.add_goal_defs_subterms),
        handle.heuristic_name.as_str(),
        handle.heuristic_def.as_deref().unwrap_or(""),
        bool_name(handle.prefer_initial_clauses),
        handle.selection_strategy.as_str(),
        handle.pos_lit_sel_min,
        handle.pos_lit_sel_max,
        handle.neg_lit_sel_min,
        handle.neg_lit_sel_max,
        handle.all_lit_sel_min,
        handle.all_lit_sel_max,
        handle.weight_sel_min,
        bool_name(handle.select_on_proc_only),
        bool_name(handle.inherit_paramod_lit),
        bool_name(handle.inherit_goal_pm_lit),
        bool_name(handle.inherit_conj_pm_lit),
        bool_name(handle.enable_eq_factoring),
        bool_name(handle.enable_neg_unit_paramod),
        bool_name(handle.enable_given_forward_simpl),
        handle.pm_type.name(),
        handle.ac_handling.c_value(),
        bool_name(handle.ac_res_aggressive),
        bool_name(handle.forward_context_sr),
        bool_name(handle.forward_context_sr_aggressive),
        bool_name(handle.backward_context_sr),
        bool_name(handle.forward_subsumption_aggressive),
        handle.forward_demod as u8,
        bool_name(handle.prefer_general),
        bool_name(handle.condensing),
        bool_name(handle.condensing_aggressive),
        bool_name(handle.er_varlit_destructive),
        bool_name(handle.er_strong_destructive),
        bool_name(handle.er_aggressive),
        handle.split_clauses.c_value(),
        handle.split_method.c_value(),
        bool_name(handle.split_aggressive),
        bool_name(handle.split_fresh_defs),
        handle.diseq_decomposition,
        handle.diseq_decomp_maxarity,
        handle.rw_bw_index_type.as_str(),
        handle.pm_from_index_type.as_str(),
        handle.pm_into_index_type.as_str(),
        handle.sat_check_grounding.name(),
        handle.sat_check_step_limit,
        handle.sat_check_size_limit,
        handle.sat_check_ttinsert_limit,
        bool_name(handle.sat_check_normconst),
        bool_name(handle.sat_check_normalize),
        handle.sat_check_decision_limit,
        handle.filter_orphans_limit,
        handle.forward_contract_limit,
        handle.delete_bad_limit,
        handle.mem_limit,
        bool_name(handle.watchlist_simplify),
        bool_name(handle.watchlist_is_static),
        bool_name(handle.use_tptp_sos),
        bool_name(handle.presat_interreduction),
        bool_name(handle.detsort_bw_rw),
        bool_name(handle.detsort_tmpset),
        handle.arg_cong.name(),
        handle.neg_ext.name(),
        handle.pos_ext.name(),
        handle.ext_rules_max_depth,
        bool_name(handle.inverse_recognition),
        bool_name(handle.replace_inj_defs),
        bool_name(handle.lift_lambdas),
        bool_name(handle.lambda_to_forall),
        bool_name(handle.unroll_only_formulas),
        handle.elim_leibniz_max_depth,
        handle.prim_enum_mode.name(),
        handle.prim_enum_max_depth,
        handle.inst_choice_max_depth,
        bool_name(handle.local_rw),
        bool_name(handle.prune_args),
        bool_name(handle.preinstantiate_induction),
        bool_name(handle.fool_unroll),
        handle.func_proj_limit,
        handle.imit_limit,
        handle.ident_limit,
        handle.elim_limit,
        handle.unif_mode.name(),
        bool_name(handle.pattern_oracle),
        bool_name(handle.fixpoint_oracle),
        handle.max_unifiers,
        handle.max_unif_steps
    )
}

#[must_use]
pub const fn bool_name(value: bool) -> &'static str {
    bool_to_str(value)
}

#[must_use]
pub const fn ext_inference_type_name_raw(value: i32) -> &'static str {
    match value {
        0 => "all",
        1 => "max",
        _ => "off",
    }
}

#[must_use]
pub const fn prim_enum_mode_name_raw(value: i32) -> &'static str {
    match value {
        0 => "neg",
        1 => "and",
        2 => "or",
        3 => "eq",
        4 => "pragmatic",
        5 => "full",
        6 => "logsymbol",
        _ => "unknown",
    }
}

#[must_use]
pub const fn unif_mode_name_raw(value: i32) -> &'static str {
    match value {
        0 => "single",
        _ => "multi",
    }
}

#[must_use]
pub fn str_to_ext_inference_type(value: &str) -> Option<ExtInferenceType> {
    match value {
        "all" => Some(ExtInferenceType::AllLits),
        "max" => Some(ExtInferenceType::MaxLits),
        "off" => Some(ExtInferenceType::NoLits),
        _ => None,
    }
}

#[must_use]
pub fn str_to_prim_enum_mode_raw(value: &str) -> i32 {
    match value {
        "neg" => PrimEnumMode::Neg.c_value(),
        "and" => PrimEnumMode::And.c_value(),
        "or" => PrimEnumMode::Or.c_value(),
        "eq" => PrimEnumMode::Eq.c_value(),
        "pragmatic" => PrimEnumMode::Pragmatic.c_value(),
        "full" => PrimEnumMode::Full.c_value(),
        "logsymbol" => PrimEnumMode::LogSymbol.c_value(),
        _ => -1,
    }
}

#[must_use]
pub fn str_to_prim_enum_mode(value: &str) -> Option<PrimEnumMode> {
    PrimEnumMode::from_c_value(str_to_prim_enum_mode_raw(value))
}

#[must_use]
pub fn str_to_unif_mode_raw(value: &str) -> i32 {
    match value {
        "single" => UnifMode::Single.c_value(),
        "multi" => UnifMode::Multi.c_value(),
        _ => -1,
    }
}

#[must_use]
pub fn str_to_unif_mode(value: &str) -> Option<UnifMode> {
    UnifMode::from_c_value(str_to_unif_mode_raw(value))
}

#[cfg(test)]
mod tests {
    use super::{
        bool_name, ext_inference_type_name_raw, hcb_add_wfcb, hcb_alloc, hcb_clause_evaluate,
        hcb_clause_evaluate_into, hcb_clause_evaluate_with_bank, hcb_clause_set_del_prop,
        hcb_clause_set_delete_bad_clauses, hcb_single_weight_clause_select_with,
        hcb_standard_clause_select, hcb_standard_clause_select_with,
        hcb_standard_selection_eval_and_advance, heuristic_parms_alloc, heuristic_parms_initialize,
        heuristic_parms_parse, heuristic_parms_parse_into, heuristic_parms_parse_into_report,
        heuristic_parms_print_string, prim_enum_mode_name_raw, str_to_ext_inference_type,
        str_to_prim_enum_mode, str_to_prim_enum_mode_raw, str_to_unif_mode, str_to_unif_mode_raw,
        unif_mode_name_raw, AcHandling, ExtInferenceType, GroundingStrategy, HcbCell,
        HcbSelectFunction, HeuristicParmsCell, ParamodulationType, PrimEnumMode, SplitClassType,
        SplitType, UnifMode, DEFAULT_DELETE_BAD_LIMIT, DEFAULT_EQDEF_INCRLIMIT,
        DEFAULT_EQDEF_MAXCLAUSES, DEFAULT_FILTER_ORPHANS_LIMIT, DEFAULT_FORMULA_DEF_LIMIT,
        DEFAULT_FORWARD_CONTRACT_LIMIT, DEFAULT_LITERAL_SELECTION, DEFAULT_MAX_UNIFIERS,
        DEFAULT_MAX_UNIF_STEPS, DEFAULT_MINISCOPE_LIMIT, DEFAULT_PM_FROM_INDEX_NAME,
        DEFAULT_PM_INTO_INDEX_NAME, DEFAULT_RW_BW_INDEX_NAME, DEFAULT_SAT_CHECK_DECISION_LIMIT,
        DEFAULT_SYM_OCCS, HCB_DEFAULT_HEURISTIC, HCB_INITIAL_CAPACITY, LITERAL_SELECTION_NAMES,
        MAX_PM_INDEX_NAME_LEN, NO_ELIM_LEIBNIZ, NO_EXT_SUP,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::{CP_DELETE_CLAUSE, CP_INITIAL, CP_IS_ORIENTED};
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::neweval::{evals_alloc, EvalPriority, PRIO_BEST, PRIO_NORMAL};
    use crate::heuristics::to_params::{OrderParmsCell, TermOrdering};
    use crate::heuristics::wfcb::{wfcb_alloc, wfcb_alloc_with_bank, BoxedWfcb};
    use crate::heuristics::wfcbadmin::WfcbAdmin;
    use crate::inout::scanner::Scanner;
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::RewriteLevel;
    use crate::terms::typebanks::TypeBank;
    use std::cell::Cell;
    use std::rc::Rc;

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap_or_else(|err| panic!("{err}"))
    }

    fn clause_with_evaluations(ident: i64, values: &[(EvalPriority, f32)]) -> Clause {
        let mut clause = Clause::empty();
        clause.set_ident(ident);
        let mut evaluations = evals_alloc(values.len());
        for (pos, &(priority, heuristic)) in values.iter().enumerate() {
            evaluations.eval_mut(pos).set_priority(priority);
            evaluations.eval_mut(pos).set_heuristic(heuristic);
        }
        clause.add_eval_cell(evaluations);
        clause
    }

    #[test]
    fn hcb_default_constants_match_c_defines() {
        assert_eq!(NO_EXT_SUP, -1);
        assert_eq!(NO_ELIM_LEIBNIZ, -1);
        assert_eq!(HCB_DEFAULT_HEURISTIC, "Default");
        assert_eq!(DEFAULT_EQDEF_MAXCLAUSES, 20_000);
        assert_eq!(DEFAULT_EQDEF_INCRLIMIT, 20);
        assert_eq!(DEFAULT_FORMULA_DEF_LIMIT, 24);
        assert_eq!(DEFAULT_SYM_OCCS, 512);
        assert_eq!(DEFAULT_MINISCOPE_LIMIT, 1_048_576);
        assert_eq!(DEFAULT_FILTER_ORPHANS_LIMIT, i64::MAX);
        assert_eq!(DEFAULT_FORWARD_CONTRACT_LIMIT, i64::MAX);
        assert_eq!(DEFAULT_DELETE_BAD_LIMIT, i64::MAX);
        assert_eq!(DEFAULT_RW_BW_INDEX_NAME, "FP7");
        assert_eq!(DEFAULT_PM_FROM_INDEX_NAME, "FP7");
        assert_eq!(DEFAULT_PM_INTO_INDEX_NAME, "FP7");
        assert_eq!(DEFAULT_LITERAL_SELECTION, "NoSelection");
        assert_eq!(DEFAULT_SAT_CHECK_DECISION_LIMIT, 10_000);
        assert_eq!(DEFAULT_MAX_UNIFIERS, 4);
        assert_eq!(DEFAULT_MAX_UNIF_STEPS, 256);
    }

    #[test]
    fn enum_discriminants_match_c_declaration_order() {
        assert_eq!(AcHandling::None.c_value(), 0);
        assert_eq!(AcHandling::DiscardAll.c_value(), 1);
        assert_eq!(AcHandling::KeepUnits.c_value(), 2);
        assert_eq!(AcHandling::KeepOrientable.c_value(), 3);

        assert_eq!(ParamodulationType::Plain.c_value(), 0);
        assert_eq!(ParamodulationType::Sim.c_value(), 1);
        assert_eq!(ParamodulationType::OrientedSim.c_value(), 2);
        assert_eq!(ParamodulationType::SuperSim.c_value(), 3);
        assert_eq!(ParamodulationType::OrientedSuperSim.c_value(), 4);
        assert_eq!(ParamodulationType::DecreasingSim.c_value(), 5);
        assert_eq!(ParamodulationType::SizeDecreasingSim.c_value(), 6);

        assert_eq!(SplitType::GroundNone.c_value(), 0);
        assert_eq!(SplitType::GroundOne.c_value(), 1);
        assert_eq!(SplitType::GroundFull.c_value(), 2);

        assert_eq!(SplitClassType::NONE.c_value(), 0);
        assert_eq!(SplitClassType::HORN.c_value(), 1);
        assert_eq!(SplitClassType::NON_HORN.c_value(), 2);
        assert_eq!(SplitClassType::NEGATIVE.c_value(), 4);
        assert_eq!(SplitClassType::POSITIVE.c_value(), 8);
        assert_eq!(SplitClassType::MIXED.c_value(), 16);
        assert_eq!(SplitClassType::ALL.c_value(), 7);
        assert!(SplitClassType::from_c_value(3).contains(SplitClassType::HORN));
        assert!(SplitClassType::from_c_value(3).contains(SplitClassType::NON_HORN));
        assert!(!SplitClassType::ALL.contains(SplitClassType::POSITIVE));

        assert_eq!(GroundingStrategy::NoGrounding.c_value(), 0);
        assert_eq!(GroundingStrategy::PseudoVar.c_value(), 1);
        assert_eq!(GroundingStrategy::FirstConst.c_value(), 2);
        assert_eq!(GroundingStrategy::ConjMinMinFreq.c_value(), 3);
        assert_eq!(GroundingStrategy::ConjMaxMinFreq.c_value(), 4);
        assert_eq!(GroundingStrategy::ConjMinMaxFreq.c_value(), 5);
        assert_eq!(GroundingStrategy::ConjMaxMaxFreq.c_value(), 6);
        assert_eq!(GroundingStrategy::GlobalMax.c_value(), 7);
        assert_eq!(GroundingStrategy::GlobalMin.c_value(), 8);

        assert_eq!(ExtInferenceType::AllLits.c_value(), 0);
        assert_eq!(ExtInferenceType::MaxLits.c_value(), 1);
        assert_eq!(ExtInferenceType::NoLits.c_value(), 2);

        assert_eq!(PrimEnumMode::Neg.c_value(), 0);
        assert_eq!(PrimEnumMode::And.c_value(), 1);
        assert_eq!(PrimEnumMode::Or.c_value(), 2);
        assert_eq!(PrimEnumMode::Eq.c_value(), 3);
        assert_eq!(PrimEnumMode::Pragmatic.c_value(), 4);
        assert_eq!(PrimEnumMode::Full.c_value(), 5);
        assert_eq!(PrimEnumMode::LogSymbol.c_value(), 6);

        assert_eq!(UnifMode::Single.c_value(), 0);
        assert_eq!(UnifMode::Multi.c_value(), 1);
    }

    #[test]
    fn from_c_value_rejects_unknown_discriminants() {
        assert_eq!(AcHandling::from_c_value(4), None);
        assert_eq!(ParamodulationType::from_c_value(7), None);
        assert_eq!(SplitType::from_c_value(3), None);
        assert_eq!(GroundingStrategy::from_c_value(9), None);
        assert_eq!(ExtInferenceType::from_c_value(-1), None);
        assert_eq!(PrimEnumMode::from_c_value(7), None);
        assert_eq!(UnifMode::from_c_value(2), None);
    }

    #[test]
    fn raw_name_helpers_preserve_macro_fallbacks() {
        assert_eq!(bool_name(true), "true");
        assert_eq!(bool_name(false), "false");

        assert_eq!(ParamodulationType::Plain.name(), "ParamodPlain");
        assert_eq!(
            ParamodulationType::SizeDecreasingSim.name(),
            "ParamodSizeDecreasingSim"
        );
        assert_eq!(GroundingStrategy::NoGrounding.name(), "NoGrounding");
        assert_eq!(GroundingStrategy::GlobalMin.name(), "GlobalMin");

        assert_eq!(ExtInferenceType::AllLits.name(), "all");
        assert_eq!(ExtInferenceType::MaxLits.name(), "max");
        assert_eq!(ExtInferenceType::NoLits.name(), "off");
        assert_eq!(ext_inference_type_name_raw(-1), "off");
        assert_eq!(ext_inference_type_name_raw(99), "off");

        assert_eq!(PrimEnumMode::LogSymbol.name(), "logsymbol");
        assert_eq!(prim_enum_mode_name_raw(-1), "unknown");
        assert_eq!(prim_enum_mode_name_raw(99), "unknown");

        assert_eq!(UnifMode::Single.name(), "single");
        assert_eq!(UnifMode::Multi.name(), "multi");
        assert_eq!(unif_mode_name_raw(-1), "multi");
        assert_eq!(unif_mode_name_raw(99), "multi");
    }

    #[test]
    fn hcb_strategy_parser_names_match_c_spellings() {
        assert_eq!(
            str_to_ext_inference_type("all"),
            Some(ExtInferenceType::AllLits)
        );
        assert_eq!(
            str_to_ext_inference_type("max"),
            Some(ExtInferenceType::MaxLits)
        );
        assert_eq!(
            str_to_ext_inference_type("off"),
            Some(ExtInferenceType::NoLits)
        );
        assert_eq!(str_to_ext_inference_type("none"), None);

        assert_eq!(str_to_prim_enum_mode_raw("logsymbol"), 6);
        assert_eq!(str_to_prim_enum_mode_raw("logsym"), -1);
        assert_eq!(
            str_to_prim_enum_mode("logsymbol"),
            Some(PrimEnumMode::LogSymbol)
        );
        assert_eq!(str_to_prim_enum_mode("logsym"), None);

        assert_eq!(str_to_unif_mode_raw("single"), 0);
        assert_eq!(str_to_unif_mode_raw("multi"), 1);
        assert_eq!(str_to_unif_mode_raw("many"), -1);
        assert_eq!(str_to_unif_mode("single"), Some(UnifMode::Single));
        assert_eq!(str_to_unif_mode("multi"), Some(UnifMode::Multi));
        assert_eq!(str_to_unif_mode("many"), None);
    }

    #[test]
    fn heuristic_parms_allocation_and_initialize_use_c_defaults() {
        let mut handle = HeuristicParmsCell {
            no_preproc: true,
            heuristic_name: "Custom".to_owned(),
            sat_check_decision_limit: 99,
            ..heuristic_parms_alloc()
        };

        heuristic_parms_initialize(&mut handle);

        assert_eq!(handle, HeuristicParmsCell::default());
    }

    #[test]
    fn heuristic_parms_default_preprocessing_and_strategy_fields_match_c() {
        let handle = HeuristicParmsCell::default();

        assert_eq!(handle.order_params, OrderParmsCell::default());
        assert!(!handle.no_preproc);
        assert_eq!(handle.eqdef_maxclauses, DEFAULT_EQDEF_MAXCLAUSES);
        assert_eq!(handle.eqdef_incrlimit, DEFAULT_EQDEF_INCRLIMIT);
        assert_eq!(handle.formula_def_limit, DEFAULT_FORMULA_DEF_LIMIT);
        assert_eq!(handle.miniscope_limit, DEFAULT_MINISCOPE_LIMIT);
        assert_eq!(handle.sine, None);
        assert!(!handle.add_goal_defs_pos);
        assert!(!handle.add_goal_defs_neg);
        assert!(!handle.add_goal_defs_subterms);
        assert!(!handle.bce);
        assert_eq!(handle.bce_max_occs, DEFAULT_SYM_OCCS);
        assert!(!handle.pred_elim);
        assert!(!handle.pred_elim_gates);
        assert_eq!(handle.pred_elim_max_occs, DEFAULT_SYM_OCCS);
        assert_eq!(handle.pred_elim_tolerance, 0);
        assert!(!handle.pred_elim_force_mu_decrease);
        assert!(!handle.pred_elim_ignore_conj_syms);
        assert_eq!(handle.heuristic_name, HCB_DEFAULT_HEURISTIC);
        assert_eq!(handle.heuristic_def, None);
        assert!(!handle.prefer_initial_clauses);
    }

    #[test]
    fn heuristic_parms_default_literal_selection_fields_match_c() {
        let handle = HeuristicParmsCell::default();

        assert_eq!(handle.selection_strategy, DEFAULT_LITERAL_SELECTION);
        assert_eq!(handle.pos_lit_sel_min, 0);
        assert_eq!(handle.pos_lit_sel_max, i64::MAX);
        assert_eq!(handle.neg_lit_sel_min, 0);
        assert_eq!(handle.neg_lit_sel_max, i64::MAX);
        assert_eq!(handle.all_lit_sel_min, 0);
        assert_eq!(handle.all_lit_sel_max, i64::MAX);
        assert_eq!(handle.weight_sel_min, 0);
        assert!(!handle.select_on_proc_only);
        assert!(!handle.inherit_paramod_lit);
        assert!(!handle.inherit_goal_pm_lit);
        assert!(!handle.inherit_conj_pm_lit);
    }

    #[test]
    fn heuristic_parms_default_inference_and_indexing_fields_match_c() {
        let handle = HeuristicParmsCell::default();

        assert!(handle.enable_eq_factoring);
        assert!(handle.enable_neg_unit_paramod);
        assert!(handle.enable_given_forward_simpl);
        assert_eq!(handle.pm_type, ParamodulationType::Plain);
        assert_eq!(handle.ac_handling, AcHandling::DiscardAll);
        assert!(handle.ac_res_aggressive);
        assert!(!handle.forward_context_sr);
        assert!(!handle.forward_context_sr_aggressive);
        assert!(!handle.backward_context_sr);
        assert!(!handle.forward_subsumption_aggressive);
        assert_eq!(handle.forward_demod, RewriteLevel::FullRewrite);
        assert!(!handle.prefer_general);
        assert!(!handle.lambda_demod);
        assert!(!handle.condensing);
        assert!(!handle.condensing_aggressive);
        assert!(!handle.er_varlit_destructive);
        assert!(!handle.er_strong_destructive);
        assert!(!handle.er_aggressive);
        assert_eq!(handle.split_clauses, SplitClassType::NONE);
        assert_eq!(handle.split_method, SplitType::GroundNone);
        assert!(!handle.split_aggressive);
        assert!(handle.split_fresh_defs);
        assert_eq!(handle.diseq_decomposition, 0);
        assert_eq!(handle.diseq_decomp_maxarity, i64::MAX);
        assert_eq!(handle.rw_bw_index_type, DEFAULT_RW_BW_INDEX_NAME);
        assert_eq!(handle.pm_from_index_type, DEFAULT_PM_FROM_INDEX_NAME);
        assert_eq!(handle.pm_into_index_type, DEFAULT_PM_INTO_INDEX_NAME);
    }

    #[test]
    fn heuristic_parms_default_sat_and_misc_fields_match_c() {
        let handle = HeuristicParmsCell::default();

        assert_eq!(handle.sat_check_grounding, GroundingStrategy::NoGrounding);
        assert_eq!(handle.sat_check_step_limit, i64::MAX);
        assert_eq!(handle.sat_check_size_limit, i64::MAX);
        assert_eq!(handle.sat_check_ttinsert_limit, i64::MAX);
        assert!(!handle.sat_check_normconst);
        assert!(!handle.sat_check_normalize);
        assert_eq!(
            handle.sat_check_decision_limit,
            DEFAULT_SAT_CHECK_DECISION_LIMIT
        );
        assert_eq!(handle.filter_orphans_limit, DEFAULT_FILTER_ORPHANS_LIMIT);
        assert_eq!(
            handle.forward_contract_limit,
            DEFAULT_FORWARD_CONTRACT_LIMIT
        );
        assert_eq!(handle.delete_bad_limit, DEFAULT_DELETE_BAD_LIMIT);
        assert_eq!(handle.mem_limit, 0);
        assert!(handle.watchlist_simplify);
        assert!(!handle.watchlist_is_static);
        assert!(!handle.use_tptp_sos);
        assert!(!handle.presat_interreduction);
        assert!(!handle.detsort_bw_rw);
        assert!(!handle.detsort_tmpset);
    }

    #[test]
    fn heuristic_parms_default_higher_order_fields_match_c() {
        let handle = HeuristicParmsCell::default();

        assert_eq!(handle.arg_cong, ExtInferenceType::AllLits);
        assert_eq!(handle.neg_ext, ExtInferenceType::NoLits);
        assert_eq!(handle.pos_ext, ExtInferenceType::NoLits);
        assert_eq!(handle.ext_rules_max_depth, NO_EXT_SUP);
        assert!(!handle.inverse_recognition);
        assert!(!handle.replace_inj_defs);
        assert!(handle.lift_lambdas);
        assert!(handle.lambda_to_forall);
        assert!(handle.unroll_only_formulas);
        assert_eq!(handle.elim_leibniz_max_depth, NO_ELIM_LEIBNIZ);
        assert_eq!(handle.prim_enum_mode, PrimEnumMode::Pragmatic);
        assert_eq!(handle.prim_enum_max_depth, -1);
        assert_eq!(handle.inst_choice_max_depth, -1);
        assert!(!handle.local_rw);
        assert!(!handle.prune_args);
        assert!(!handle.preinstantiate_induction);
        assert!(handle.fool_unroll);
        assert_eq!(handle.func_proj_limit, 0);
        assert_eq!(handle.imit_limit, 0);
        assert_eq!(handle.ident_limit, 0);
        assert_eq!(handle.elim_limit, 0);
        assert_eq!(handle.unif_mode, UnifMode::Single);
        assert!(handle.pattern_oracle);
        assert!(handle.fixpoint_oracle);
        assert_eq!(handle.max_unifiers, DEFAULT_MAX_UNIFIERS);
        assert_eq!(handle.max_unif_steps, DEFAULT_MAX_UNIF_STEPS);
    }

    #[test]
    fn heuristic_parms_print_string_matches_c_default_surface() {
        let printed = heuristic_parms_print_string(&HeuristicParmsCell::default());

        assert!(printed.starts_with("{\n   {\n"));
        assert!(printed.ends_with("}\n"));
        assert!(printed.contains("      ordertype:               KBO6\n"));
        assert!(printed.contains("   no_preproc:                     false\n"));
        assert!(printed.contains("   sine:                           \"None\"\n"));
        assert!(printed.contains("   heuristic_def:                 \"\"\n"));
        assert!(printed.contains("   selection_strategy:             NoSelection\n"));
        assert!(printed.contains("   pm_type:                        ParamodPlain\n"));
        assert!(printed.contains("   ac_handling:                    1\n"));
        assert!(printed.contains("   sat_check_grounding:            NoGrounding\n"));
        assert!(printed.contains("   arg_cong:                       all\n"));
        assert!(printed.contains("   prim_enum_mode:                pragmatic\n"));
        assert!(printed.contains("   unif_mode:                     single\n"));
        assert!(!printed.contains("   bce:"));
        assert!(!printed.contains("   pred_elim:"));
        assert!(!printed.contains("   lambda_demod:"));
        assert_substrings_in_order(
            &printed,
            &[
                "   no_preproc:",
                "   heuristic_name:",
                "   selection_strategy:",
                "   enable_eq_factoring:",
                "   pm_type:",
                "   split_clauses:",
                "   sat_check_grounding:",
                "   watchlist_simplify:",
                "   arg_cong:",
                "   unif_mode:",
                "   max_unif_steps:",
            ],
        );
    }

    #[test]
    fn heuristic_parms_print_string_formats_nondefault_values() {
        let handle = HeuristicParmsCell {
            no_preproc: true,
            sine: Some("Auto".to_owned()),
            heuristic_name: "CustomHeuristic".to_owned(),
            heuristic_def: Some("PreferGoals".to_owned()),
            prefer_initial_clauses: true,
            selection_strategy: "SelectComplex".to_owned(),
            pm_type: ParamodulationType::OrientedSuperSim,
            ac_handling: AcHandling::KeepUnits,
            forward_demod: RewriteLevel::RuleRewrite,
            split_clauses: SplitClassType::ALL,
            split_method: SplitType::GroundFull,
            split_aggressive: true,
            sat_check_grounding: GroundingStrategy::GlobalMin,
            sat_check_normconst: true,
            sat_check_normalize: true,
            mem_limit: 123,
            arg_cong: ExtInferenceType::MaxLits,
            neg_ext: ExtInferenceType::AllLits,
            pos_ext: ExtInferenceType::MaxLits,
            prim_enum_mode: PrimEnumMode::Full,
            unif_mode: UnifMode::Multi,
            pattern_oracle: false,
            ..HeuristicParmsCell::default()
        };
        let printed = heuristic_parms_print_string(&handle);

        assert!(printed.contains("   no_preproc:                     true\n"));
        assert!(printed.contains("   sine:                           \"Auto\"\n"));
        assert!(printed.contains("   heuristic_name:                CustomHeuristic\n"));
        assert!(printed.contains("   heuristic_def:                 \"PreferGoals\"\n"));
        assert!(printed.contains("   prefer_initial_clauses:         true\n"));
        assert!(printed.contains("   selection_strategy:             SelectComplex\n"));
        assert!(printed.contains("   pm_type:                        ParamodOrientedSuperSim\n"));
        assert!(printed.contains("   ac_handling:                    2\n"));
        assert!(printed.contains("   forward_demod:                  1\n"));
        assert!(printed.contains("   split_clauses:                  7\n"));
        assert!(printed.contains("   split_method:                   2\n"));
        assert!(printed.contains("   split_aggressive:               true\n"));
        assert!(printed.contains("   sat_check_grounding:            GlobalMin\n"));
        assert!(printed.contains("   sat_check_normconst:            true\n"));
        assert!(printed.contains("   sat_check_normalize:            true\n"));
        assert!(printed.contains("   mem_limit:                      123\n"));
        assert!(printed.contains("   arg_cong:                       max\n"));
        assert!(printed.contains("   neg_ext:                        all\n"));
        assert!(printed.contains("   pos_ext:                        max\n"));
        assert!(printed.contains("   prim_enum_mode:                full\n"));
        assert!(printed.contains("   unif_mode:                     multi\n"));
        assert!(printed.contains("   pattern_oracle:                false\n"));
    }

    #[test]
    fn heuristic_parms_parse_round_trips_printed_nondefault_cell() {
        let original = HeuristicParmsCell {
            order_params: OrderParmsCell {
                ordertype: TermOrdering::Lpo4Copy,
                ..OrderParmsCell::default()
            },
            no_preproc: true,
            sine: Some("Auto".to_owned()),
            heuristic_name: "CustomHeuristic".to_owned(),
            heuristic_def: Some("HeuristicDef".to_owned()),
            prefer_initial_clauses: true,
            selection_strategy: "SelectCQAr".to_owned(),
            pos_lit_sel_min: 1,
            neg_lit_sel_max: 9,
            select_on_proc_only: true,
            enable_eq_factoring: false,
            pm_type: ParamodulationType::SuperSim,
            ac_handling: AcHandling::KeepUnits,
            forward_demod: RewriteLevel::RuleRewrite,
            split_clauses: SplitClassType::ALL,
            split_method: SplitType::GroundOne,
            rw_bw_index_type: "FP6".to_owned(),
            pm_from_index_type: "FP5".to_owned(),
            pm_into_index_type: "FP4".to_owned(),
            sat_check_grounding: GroundingStrategy::GlobalMax,
            sat_check_normconst: true,
            sat_check_normalize: true,
            arg_cong: ExtInferenceType::MaxLits,
            neg_ext: ExtInferenceType::AllLits,
            pos_ext: ExtInferenceType::NoLits,
            prim_enum_mode: PrimEnumMode::Full,
            unif_mode: UnifMode::Multi,
            pattern_oracle: false,
            ..HeuristicParmsCell::default()
        };
        let mut scanner = scanner(&format!("{} tail", heuristic_parms_print_string(&original)));

        let parsed =
            heuristic_parms_parse(&mut scanner, true).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(parsed, original);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn heuristic_parms_parse_accepts_split_class_bitmasks_like_c() {
        let mut scanner = scanner("{ split_clauses: 3 } tail");
        let mut params = HeuristicParmsCell::default();

        let complete =
            heuristic_parms_parse_into(&mut scanner, &mut params, false).unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert!(!complete);
        assert_eq!(params.split_clauses.c_value(), 3);
        assert!(params.split_clauses.contains(SplitClassType::HORN));
        assert!(params.split_clauses.contains(SplitClassType::NON_HORN));
        assert!(!params.split_clauses.contains(SplitClassType::NEGATIVE));
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn heuristic_parms_parse_accepts_long_min_eqdef_sentinel() {
        let mut scanner = scanner("{ eqdef_incrlimit: -9223372036854775808 } tail");
        let mut params = HeuristicParmsCell::default();

        let complete =
            heuristic_parms_parse_into(&mut scanner, &mut params, false).unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert!(!complete);
        assert_eq!(params.eqdef_incrlimit, i64::MIN);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn heuristic_parms_parse_preserves_c_string_and_intmax_quirks() {
        let mut default_scanner = scanner(&format!(
            "{} tail",
            heuristic_parms_print_string(&HeuristicParmsCell::default())
        ));

        let parsed = heuristic_parms_parse(&mut default_scanner, false)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(parsed.sine.as_deref(), Some("None"));
        assert_eq!(parsed.heuristic_def.as_deref(), Some(""));
        assert_ne!(parsed, HeuristicParmsCell::default());

        let mut mem_scanner = scanner("{ mem_limit: 5 } tail");
        let mut params = HeuristicParmsCell::default();
        let complete = heuristic_parms_parse_into(&mut mem_scanner, &mut params, false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert!(!complete);
        assert_eq!(params.mem_limit, u64::MAX - 4);
        assert_eq!(mem_scanner.current_token().literal(), "tail");
    }

    #[test]
    fn heuristic_parms_parse_reports_missing_fields_and_preserves_existing_values() {
        let mut scanner = scanner("{ no_preproc: false } tail");
        let mut params = HeuristicParmsCell {
            no_preproc: true,
            selection_strategy: "SelectCQAr".to_owned(),
            ..HeuristicParmsCell::default()
        };

        let report = heuristic_parms_parse_into_report(&mut scanner, &mut params, true)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert!(!report.complete);
        assert!(report.missing_fields.contains(&"ordering information"));
        assert!(report.missing_fields.contains(&"eqdef_maxclauses"));
        assert!(report.missing_fields.contains(&"selection_strategy"));
        assert_eq!(report.warnings.len(), report.missing_fields.len());
        assert!(!params.no_preproc);
        assert_eq!(params.selection_strategy, "SelectCQAr");
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn heuristic_parms_parse_validates_names_and_truncates_index_identifiers() {
        assert!(LITERAL_SELECTION_NAMES.contains(&"SelectMaxLComplexAvoidAppVar"));

        let mut bad_selection = scanner("{ selection_strategy: NoSuchSelection }");
        let mut params = HeuristicParmsCell::default();
        let error =
            heuristic_parms_parse_into_report(&mut bad_selection, &mut params, false).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("NoSelection"));

        let mut quoted = scanner(r#"{ arg_cong: "all" }"#);
        let error = heuristic_parms_parse_into_report(&mut quoted, &mut params, false).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("all|max|off"));

        let long_name = "ABCDEFGHIJKLMNOPQRSTUV";
        let mut index = scanner(&format!("{{ rw_bw_index_type: {long_name} }} tail"));
        let report = heuristic_parms_parse_into_report(&mut index, &mut params, false)
            .unwrap_or_else(|err| {
                panic!("{err}");
            });

        assert!(!report.complete);
        assert_eq!(params.rw_bw_index_type.len(), MAX_PM_INDEX_NAME_LEN - 1);
        assert_eq!(params.rw_bw_index_type, "ABCDEFGHIJKLMNOPQRS");
        assert_eq!(index.current_token().literal(), "tail");
    }

    #[test]
    fn hcb_alloc_initializes_empty_control_block_like_c() {
        let hcb = hcb_alloc();

        assert_eq!(hcb.wfcb_no(), 0);
        assert!(hcb.wfcb_capacity() >= HCB_INITIAL_CAPACITY);
        assert_eq!(hcb.current_eval(), 0);
        assert!(hcb.select_switch_capacity() >= HCB_INITIAL_CAPACITY);
        assert_eq!(hcb.select_count(), 0);
        assert_eq!(hcb.hcb_select(), HcbSelectFunction::StandardClauseSelect);
        assert_eq!(hcb.wfcb_handle(0), None);
        assert_eq!(hcb.select_switch(0), None);
        assert_eq!(hcb.data(), None);
    }

    #[test]
    fn hcb_add_wfcb_stores_cumulative_switch_counts() {
        let mut hcb = hcb_alloc();

        assert_eq!(hcb_add_wfcb(&mut hcb, 10, 3), 1);
        assert_eq!(hcb.wfcb_handle(0), Some(10));
        assert_eq!(hcb.select_switch(0), Some(3));
        assert_eq!(
            hcb.hcb_select(),
            HcbSelectFunction::SingleWeightClauseSelect
        );

        assert_eq!(hcb_add_wfcb(&mut hcb, 11, 5), 2);
        assert_eq!(hcb.wfcb_handle(1), Some(11));
        assert_eq!(hcb.select_switch(1), Some(8));
        assert_eq!(hcb.hcb_select(), HcbSelectFunction::StandardClauseSelect);
    }

    #[test]
    #[should_panic(expected = "steps must be positive")]
    fn hcb_add_wfcb_rejects_nonpositive_steps() {
        let mut hcb = hcb_alloc();

        hcb_add_wfcb(&mut hcb, 10, 0);
    }

    #[test]
    fn standard_selection_advances_on_cumulative_switch_boundaries() {
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, 10, 2);
        hcb_add_wfcb(&mut hcb, 11, 3);
        hcb_add_wfcb(&mut hcb, 12, 1);

        let selected = (0..8)
            .map(|_| hcb_standard_selection_eval_and_advance(&mut hcb))
            .collect::<Vec<_>>();

        assert_eq!(
            selected,
            vec![
                Some(0),
                Some(0),
                Some(1),
                Some(1),
                Some(1),
                Some(2),
                Some(0),
                Some(0)
            ]
        );
        assert_eq!(hcb.select_count(), 2);
        assert_eq!(hcb.current_eval(), 1);
    }

    #[test]
    fn standard_selection_empty_hcb_has_no_schedule_state_to_advance() {
        let mut hcb = hcb_alloc();

        assert_eq!(hcb_standard_selection_eval_and_advance(&mut hcb), None);
        assert_eq!(hcb.select_count(), 0);
        assert_eq!(hcb.current_eval(), 0);
    }

    #[test]
    fn standard_clause_select_extracts_best_and_discards_orphans() {
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, 10, 1);
        hcb_add_wfcb(&mut hcb, 11, 1);
        let orphan_id = 101;
        let selected_id = 102;
        let eval_one_best_id = 103;
        let orphan = clause_with_evaluations(orphan_id, &[(PRIO_NORMAL, 1.0), (PRIO_NORMAL, 20.0)]);
        let selected =
            clause_with_evaluations(selected_id, &[(PRIO_NORMAL, 2.0), (PRIO_NORMAL, 30.0)]);
        let eval_one_best =
            clause_with_evaluations(eval_one_best_id, &[(PRIO_NORMAL, 3.0), (PRIO_NORMAL, 1.0)]);
        let mut set = ClauseSet::from_clauses([orphan, selected, eval_one_best]);

        let first = hcb_standard_clause_select_with(&mut hcb, &mut set, |clause| {
            clause.ident() == orphan_id
        })
        .unwrap();

        assert_eq!(first.ident(), selected_id);
        assert!(set.find_by_id(orphan_id).is_none());
        assert_eq!(hcb.select_count(), 1);
        assert_eq!(hcb.current_eval(), 1);

        let second = hcb_standard_clause_select(&mut hcb, &mut set).unwrap();

        assert_eq!(second.ident(), eval_one_best_id);
        assert_eq!(hcb.select_count(), 0);
        assert_eq!(hcb.current_eval(), 0);
        assert!(set.is_empty());
    }

    #[test]
    fn single_weight_clause_select_does_not_advance_schedule() {
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, 10, 4);
        let first_id = 201;
        let second_id = 202;
        let first = clause_with_evaluations(first_id, &[(PRIO_NORMAL, 5.0)]);
        let second = clause_with_evaluations(second_id, &[(PRIO_NORMAL, 1.0)]);
        let mut set = ClauseSet::from_clauses([first, second]);

        let selected = hcb_single_weight_clause_select_with(&hcb, &mut set, |_| false).unwrap();

        assert_eq!(selected.ident(), second_id);
        assert_eq!(hcb.select_count(), 0);
        assert_eq!(hcb.current_eval(), 0);
        assert_eq!(set.find_best(0).map(Clause::ident), Some(first_id));
    }

    #[test]
    fn hcb_clause_set_delete_bad_clauses_keeps_best_number() {
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, 10, 4);
        let first_id = 301;
        let second_id = 302;
        let third_id = 303;
        let first = clause_with_evaluations(first_id, &[(PRIO_NORMAL, 1.0)]);
        let second = clause_with_evaluations(second_id, &[(PRIO_NORMAL, 2.0)]);
        let third = clause_with_evaluations(third_id, &[(PRIO_NORMAL, 3.0)]);
        let mut set = ClauseSet::from_clauses([first, second, third]);

        let deleted = hcb_clause_set_delete_bad_clauses(&hcb, &mut set, 2);

        assert_eq!(deleted, 1);
        assert_eq!(
            set.iter().map(Clause::ident).collect::<Vec<_>>(),
            vec![first_id, second_id]
        );
        assert!(set
            .iter()
            .all(|clause| !clause.query_prop(CP_DELETE_CLAUSE)));
    }

    #[test]
    fn hcb_clause_set_del_prop_preserves_c_select_switch_j_bound() {
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, 10, 4);
        hcb_add_wfcb(&mut hcb, 11, 4);
        let first_id = 401;
        let second_id = 402;
        let third_id = 403;
        let fourth_id = 404;
        let first = clause_with_evaluations(first_id, &[(PRIO_NORMAL, 1.0), (PRIO_NORMAL, 4.0)]);
        let second = clause_with_evaluations(second_id, &[(PRIO_NORMAL, 2.0), (PRIO_NORMAL, 3.0)]);
        let third = clause_with_evaluations(third_id, &[(PRIO_NORMAL, 3.0), (PRIO_NORMAL, 2.0)]);
        let fourth = clause_with_evaluations(fourth_id, &[(PRIO_NORMAL, 4.0), (PRIO_NORMAL, 1.0)]);
        let mut set = ClauseSet::from_clauses([first, second, third, fourth]);
        set.set_prop(CP_INITIAL);

        let cleared = hcb_clause_set_del_prop(&hcb, &mut set, 3, CP_INITIAL);

        assert_eq!(cleared, 3);
        assert!(!set.find_by_id(first_id).unwrap().query_prop(CP_INITIAL));
        assert!(!set.find_by_id(second_id).unwrap().query_prop(CP_INITIAL));
        assert!(set.find_by_id(third_id).unwrap().query_prop(CP_INITIAL));
        assert!(!set.find_by_id(fourth_id).unwrap().query_prop(CP_INITIAL));
    }

    #[test]
    fn hcb_drop_calls_exit_only_when_data_is_present() {
        let exit_count = Rc::new(Cell::new(0));
        let mut hcb = HcbCell::with_data(
            Some(HcbDropData {
                exit_count: Rc::clone(&exit_count),
            }),
            record_hcb_exit,
        );

        hcb_add_wfcb(&mut hcb, 1, 1);
        assert_eq!(exit_count.get(), 0);
        drop(hcb);
        assert_eq!(exit_count.get(), 1);

        let hcb = HcbCell::with_data(None, record_hcb_exit);
        drop(hcb);
        assert_eq!(exit_count.get(), 1);
    }

    struct HcbDropData {
        exit_count: Rc<Cell<i32>>,
    }

    fn record_hcb_exit(data: HcbDropData) {
        let HcbDropData { exit_count } = data;
        exit_count.set(exit_count.get() + 1);
    }

    #[test]
    fn hcb_clause_evaluate_into_uses_admin_handles() {
        let mut admin = WfcbAdmin::new();
        let first = admin.add_wfcb("first", boxed_hcb_test_wfcb(2.5));
        let second = admin.add_wfcb("second", boxed_hcb_test_wfcb(7.25));
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, first, 1);
        hcb_add_wfcb(&mut hcb, second, 1);
        let bank = hcb_test_bank();
        let clause = Clause::empty();
        let mut evaluations = evals_alloc(hcb.wfcb_no());

        hcb_clause_evaluate_into(&hcb, &mut admin, &mut evaluations, &bank, &clause);

        assert_eq!(evaluations.eval(0).heuristic().to_bits(), 2.5_f32.to_bits());
        assert_eq!(
            evaluations.eval(1).heuristic().to_bits(),
            7.25_f32.to_bits()
        );
        assert_eq!(evaluations.eval(0).priority(), PRIO_BEST);
        assert_eq!(evaluations.eval(1).priority(), PRIO_BEST);
    }

    #[test]
    fn hcb_clause_evaluate_stores_clause_owned_evaluations() {
        let mut admin = WfcbAdmin::new();
        let first = admin.add_wfcb("first", boxed_hcb_test_wfcb(3.5));
        let second = admin.add_wfcb("second", boxed_hcb_test_wfcb(9.0));
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, first, 1);
        hcb_add_wfcb(&mut hcb, second, 1);
        let bank = hcb_test_bank();
        let mut clause = Clause::empty();

        hcb_clause_evaluate(&hcb, &mut admin, &bank, &mut clause);

        let evaluations = clause.evaluations().expect("HCB attaches evaluations");
        assert_eq!(evaluations.eval_no(), hcb.wfcb_no());
        assert_eq!(evaluations.eval(0).heuristic().to_bits(), 3.5_f32.to_bits());
        assert_eq!(evaluations.eval(1).heuristic().to_bits(), 9.0_f32.to_bits());
        assert_eq!(evaluations.object(), None);
    }

    #[test]
    fn hcb_clause_evaluate_with_bank_uses_banked_wfcb_dispatch() {
        let mut admin = WfcbAdmin::new();
        let first = admin.add_wfcb("first", boxed_hcb_test_wfcb_with_bank(4.0));
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, first, 1);
        let mut bank = hcb_test_bank();
        let mut ocb = hcb_empty_ocb(&bank);
        let mut clause = Clause::empty();

        hcb_clause_evaluate_with_bank(&hcb, &mut admin, &mut ocb, &mut bank, &mut clause)
            .unwrap_or_else(|err| panic!("{err}"));

        let evaluations = clause.evaluations().expect("HCB attaches evaluations");
        assert_eq!(evaluations.eval_no(), 1);
        assert_eq!(evaluations.eval(0).heuristic().to_bits(), 6.0_f32.to_bits());
        assert_eq!(evaluations.eval(0).priority(), PRIO_BEST);
        assert!(clause.query_prop(CP_IS_ORIENTED));
    }

    #[test]
    #[should_panic(expected = "clause must not already have evaluations")]
    fn hcb_clause_evaluate_rejects_existing_clause_evaluations() {
        let mut admin = WfcbAdmin::new();
        let first = admin.add_wfcb("first", boxed_hcb_test_wfcb(3.5));
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, first, 1);
        let bank = hcb_test_bank();
        let mut clause = Clause::empty();

        hcb_clause_evaluate(&hcb, &mut admin, &bank, &mut clause);
        hcb_clause_evaluate(&hcb, &mut admin, &bank, &mut clause);
    }

    #[test]
    #[should_panic(expected = "evaluation width must match HCB WFCB count")]
    fn hcb_clause_evaluate_into_requires_matching_eval_width() {
        let mut admin = WfcbAdmin::new();
        let first = admin.add_wfcb("first", boxed_hcb_test_wfcb(2.5));
        let mut hcb = hcb_alloc();
        hcb_add_wfcb(&mut hcb, first, 1);
        let bank = hcb_test_bank();
        let clause = Clause::empty();
        let mut evaluations = evals_alloc(0);

        hcb_clause_evaluate_into(&hcb, &mut admin, &mut evaluations, &bank, &clause);
    }

    #[derive(Clone, Copy)]
    struct HcbEvalData {
        weight: f64,
    }

    fn boxed_hcb_test_wfcb(weight: f64) -> BoxedWfcb {
        Box::new(wfcb_alloc(
            hcb_test_eval,
            hcb_test_priority,
            hcb_test_exit,
            Some(HcbEvalData { weight }),
        ))
    }

    fn hcb_test_eval(data: Option<&mut HcbEvalData>, _bank: &TermBank, _clause: &Clause) -> f64 {
        data.map_or(0.0, |data| data.weight)
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "test callback must match the banked WFCB signature"
    )]
    fn hcb_test_eval_with_bank(
        data: Option<&mut HcbEvalData>,
        _ocb: &mut OrderControlBlock,
        _bank: &mut TermBank,
        clause: &mut Clause,
    ) -> Result<f64, crate::basics::error::Diagnostic> {
        clause.set_prop(CP_IS_ORIENTED);
        Ok(data.map_or(0.0, |data| data.weight + 2.0))
    }

    fn hcb_test_priority(_bank: &TermBank, _clause: &Clause) -> EvalPriority {
        PRIO_NORMAL + 3
    }

    fn hcb_test_exit(data: HcbEvalData) {
        let HcbEvalData { weight } = data;
        assert!(weight.is_finite());
    }

    fn hcb_test_bank() -> TermBank {
        TermBank::new(Signature::new(TypeBank::new())).unwrap_or_else(|err| panic!("{err}"))
    }

    fn hcb_empty_ocb(bank: &TermBank) -> OrderControlBlock {
        OrderControlBlock::alloc(
            TermOrdering::Empty,
            false,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        )
    }

    fn boxed_hcb_test_wfcb_with_bank(weight: f64) -> BoxedWfcb {
        Box::new(wfcb_alloc_with_bank(
            hcb_test_eval,
            hcb_test_eval_with_bank,
            hcb_test_priority,
            hcb_test_exit,
            Some(HcbEvalData { weight }),
        ))
    }

    fn assert_substrings_in_order(text: &str, needles: &[&str]) {
        let mut next_start = 0;
        for needle in needles {
            let relative = text[next_start..]
                .find(needle)
                .unwrap_or_else(|| panic!("missing substring {needle}"));
            next_start += relative + needle.len();
        }
    }
}
