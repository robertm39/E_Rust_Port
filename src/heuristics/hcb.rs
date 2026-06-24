use crate::heuristics::to_params::OrderParmsCell;
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
    pub const fn c_value(self) -> i32 {
        self as i32
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
#[repr(i32)]
pub enum SplitClassType {
    #[default]
    None = 0,
    Horn = 1,
    NonHorn = 2,
    Negative = 4,
    Positive = 8,
    Mixed = 16,
    All = 7,
}

impl SplitClassType {
    #[must_use]
    pub const fn from_c_value(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Horn),
            2 => Some(Self::NonHorn),
            4 => Some(Self::Negative),
            8 => Some(Self::Positive),
            16 => Some(Self::Mixed),
            7 => Some(Self::All),
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
    pub const fn c_value(self) -> i32 {
        self as i32
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
            split_clauses: SplitClassType::None,
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

#[must_use]
pub fn heuristic_parms_alloc() -> HeuristicParmsCell {
    HeuristicParmsCell::default()
}

pub fn heuristic_parms_initialize(handle: &mut HeuristicParmsCell) {
    *handle = HeuristicParmsCell::default();
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
        ext_inference_type_name_raw, heuristic_parms_alloc, heuristic_parms_initialize,
        prim_enum_mode_name_raw, str_to_ext_inference_type, str_to_prim_enum_mode,
        str_to_prim_enum_mode_raw, str_to_unif_mode, str_to_unif_mode_raw, unif_mode_name_raw,
        AcHandling, ExtInferenceType, GroundingStrategy, HeuristicParmsCell, ParamodulationType,
        PrimEnumMode, SplitClassType, SplitType, UnifMode, DEFAULT_DELETE_BAD_LIMIT,
        DEFAULT_EQDEF_INCRLIMIT, DEFAULT_EQDEF_MAXCLAUSES, DEFAULT_FILTER_ORPHANS_LIMIT,
        DEFAULT_FORMULA_DEF_LIMIT, DEFAULT_FORWARD_CONTRACT_LIMIT, DEFAULT_LITERAL_SELECTION,
        DEFAULT_MAX_UNIFIERS, DEFAULT_MAX_UNIF_STEPS, DEFAULT_MINISCOPE_LIMIT,
        DEFAULT_PM_FROM_INDEX_NAME, DEFAULT_PM_INTO_INDEX_NAME, DEFAULT_RW_BW_INDEX_NAME,
        DEFAULT_SAT_CHECK_DECISION_LIMIT, DEFAULT_SYM_OCCS, HCB_DEFAULT_HEURISTIC, NO_ELIM_LEIBNIZ,
        NO_EXT_SUP,
    };
    use crate::heuristics::to_params::OrderParmsCell;
    use crate::terms::termtypes::RewriteLevel;

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

        assert_eq!(SplitClassType::None.c_value(), 0);
        assert_eq!(SplitClassType::Horn.c_value(), 1);
        assert_eq!(SplitClassType::NonHorn.c_value(), 2);
        assert_eq!(SplitClassType::Negative.c_value(), 4);
        assert_eq!(SplitClassType::Positive.c_value(), 8);
        assert_eq!(SplitClassType::Mixed.c_value(), 16);
        assert_eq!(SplitClassType::All.c_value(), 7);

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
        assert_eq!(SplitClassType::from_c_value(3), None);
        assert_eq!(GroundingStrategy::from_c_value(9), None);
        assert_eq!(ExtInferenceType::from_c_value(-1), None);
        assert_eq!(PrimEnumMode::from_c_value(7), None);
        assert_eq!(UnifMode::from_c_value(2), None);
    }

    #[test]
    fn raw_name_helpers_preserve_macro_fallbacks() {
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
        assert_eq!(handle.split_clauses, SplitClassType::None);
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
}
