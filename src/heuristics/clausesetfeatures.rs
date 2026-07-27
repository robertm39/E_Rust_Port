use crate::basics::defines::{bool_to_str, DEFAULT_COMCHAR_RAW};
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::clausefunc::clause_recognizes_choice;
use crate::clauses::clausesets::{eq_axioms_print_string, ClauseSet};
use crate::clauses::eqn::EqnPrintOptions;
use crate::clauses::eqn_props::EP_IS_EQU_LITERAL;
use crate::clauses::formulasets::FormulaSet;
use crate::clauses::proofstate::ProofState;
use crate::heuristics::clausefeatures::{
    clause_count_maximal_literals, clause_count_maximal_terms, clause_count_singleton_set,
    clause_count_unorientable_literals, clause_count_variable_set,
    clause_line_print_format_string_with_options, clause_line_print_string, clause_line_string,
    clause_tptp_depth_info_add,
};
use crate::inout::basicparser::{parse_float, parse_int, parse_plain_filename};
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::simpletypes::{type_get_order, type_has_bool, var_order};
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::Term;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClauseSetArityInformation {
    pub max_fun_arity: i32,
    pub avg_fun_arity: i32,
    pub sum_fun_arity: i32,
    pub max_pred_arity: i32,
    pub avg_pred_arity: i32,
    pub sum_pred_arity: i32,
    pub non_const_funs: i32,
    pub non_const_preds: i32,
    pub fun_const_count: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClauseSetHoFeatures {
    pub has_ho_features: bool,
    pub order: i32,
    pub quantifies_booleans: bool,
    pub has_defined_choice: bool,
    pub perc_app_var_lits: f64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum SpecFeatureClass {
    #[default]
    Unit = 0,
    Horn = 1,
    General = 2,
    NoEq = 3,
    SomeEq = 4,
    PureEq = 5,
    FewPosNonGroundUnits = 6,
    SomePosNonGroundUnits = 7,
    ManyPosNonGroundUnits = 8,
    FewPosGround = 9,
    SomePosGround = 10,
    ManyPosGround = 11,
    FewAxioms = 12,
    SomeAxioms = 13,
    ManyAxioms = 14,
    FewLiterals = 15,
    SomeLiterals = 16,
    ManyLiterals = 17,
    SmallTerms = 18,
    MediumTerms = 19,
    LargeTerms = 20,
    Arity0 = 21,
    Arity1 = 22,
    Arity2 = 23,
    Arity3Plus = 24,
    AritySumSmall = 25,
    AritySumMedium = 26,
    AritySumLarge = 27,
    DepthShallow = 28,
    DepthMedium = 29,
    DepthDeep = 30,
    Fo = 31,
    So = 32,
    Ho = 33,
    FewDefs = 34,
    MediumDefs = 35,
    ManyDefs = 36,
    FewFormDefs = 37,
    MediumFormDefs = 38,
    ManyFormDefs = 39,
    FewApplits = 40,
    MediumApplits = 41,
    ManyApplits = 42,
}

pub const NGU_ABSOLUTE: bool = true;
pub const NGU_FEW_DEFAULT: f64 = 0.25;
pub const NGU_MANY_DEFAULT: f64 = 0.75;
pub const NGU_FEW_ABSDEFAULT: f64 = 1.0;
pub const NGU_MANY_ABSDEFAULT: f64 = 3.0;
pub const GPC_ABSOLUTE: bool = true;
pub const GPC_FEW_DEFAULT: f64 = 0.25;
pub const GPC_MANY_DEFAULT: f64 = 0.75;
pub const GPC_FEW_ABSDEFAULT: f64 = 2.0;
pub const GPC_MANY_ABSDEFAULT: f64 = 5.0;
pub const AX_SOME_DEFAULT: i64 = 1_000;
pub const AX_MANY_DEFAULT: i64 = 10_000;
pub const LIT_SOME_DEFAULT: i64 = 400;
pub const LIT_MANY_DEFAULT: i64 = 4_000;
pub const TERM_MED_DEFAULT: i64 = 200;
pub const TERM_LARGE_DEFAULT: i64 = 1_500;
pub const FAR_SUM_MED_DEFAULT: i32 = 4;
pub const FAR_SUM_LARGE_DEFAULT: i32 = 29;
pub const DEPTH_MEDIUM_DEFAULT: i64 = 0;
pub const DEPTH_DEEP_DEFAULT: i64 = 6;
pub const SYMBOLS_MEDIUM_DEFAULT: i32 = 100;
pub const SYMBOLS_LARGE_DEFAULT: i32 = 1_000;
pub const PREDC_MEDIUM_DEFAULT: i32 = 0;
pub const PREDC_LARGE_DEFAULT: i32 = 2;
pub const PRED_MEDIUM_DEFAULT: i32 = 1_225;
pub const PRED_LARGE_DEFAULT: i32 = 4_000;
pub const FUNC_MEDIUM_DEFAULT: i32 = 8;
pub const FUNC_LARGE_DEFAULT: i32 = 110;
pub const FUN_MEDIUM_DEFAULT: i32 = 360;
pub const FUN_LARGE_DEFAULT: i32 = 400;
pub const NUM_LAMS_MEDIUM_DEFAULT: i32 = 2;
pub const NUM_LAMS_LARGE_DEFAULT: i32 = 8;
pub const ORDER_MEDIUM_DEFAULT: i32 = 2;
pub const ORDER_LARGE_DEFAULT: i32 = 3;
pub const DEFS_MEDIUM_DEFAULT: i32 = 8;
pub const DEFS_LARGE_DEFAULT: i32 = 64;
pub const DEFS_PERC_MEDIUM_DEFAULT: f64 = 0.15;
pub const DEFS_PERC_LARGE_DEFAULT: f64 = 0.5;
pub const PERC_APPLIT_MEDIUM_DEFAULT: f64 = 0.1;
pub const PERC_APPLIT_LARGE_DEFAULT: f64 = 0.5;
pub const SPEC_STRING_MEM: usize = 22;
pub const DEFAULT_OUTPUT_DESCRIPTOR: &str = "eigEIG";
pub const DEFAULT_CLASS_MASK: &str = "aaaaaaaaaaaaa";

const SPEC_TYPE_LEN: usize = SPEC_STRING_MEM - 1;
const SPEC_FEATURE_ENCODING: &[u8] = b"UHGNSPFSMFSMFSMFSMSML0123SMLSMDFSHFSMFSMFSM";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpecLimits {
    pub ngu_absolute: bool,
    pub ngu_few_limit: f64,
    pub ngu_many_limit: f64,
    pub gpc_absolute: bool,
    pub gpc_few_limit: f64,
    pub gpc_many_limit: f64,
    pub ax_some_limit: i64,
    pub ax_many_limit: i64,
    pub lit_some_limit: i64,
    pub lit_many_limit: i64,
    pub term_medium_limit: i64,
    pub term_large_limit: i64,
    pub far_sum_medium_limit: i32,
    pub far_sum_large_limit: i32,
    pub depth_medium_limit: i64,
    pub depth_deep_limit: i64,
    pub symbols_medium_limit: i32,
    pub symbols_large_limit: i32,
    pub predc_medium_limit: i32,
    pub predc_large_limit: i32,
    pub pred_medium_limit: i32,
    pub pred_large_limit: i32,
    pub func_medium_limit: i32,
    pub func_large_limit: i32,
    pub fun_medium_limit: i32,
    pub fun_large_limit: i32,
    pub order_medium_limit: i32,
    pub order_large_limit: i32,
    pub num_of_lams_medium_limit: i32,
    pub num_of_lams_large_limit: i32,
    pub num_of_defs_medium_limit: i32,
    pub num_of_defs_large_limit: i32,
    pub perc_form_defs_medium_limit: f64,
    pub perc_form_defs_large_limit: f64,
    pub perc_app_lits_medium_limit: f64,
    pub perc_app_lits_large_limit: f64,
}

impl Default for SpecLimits {
    fn default() -> Self {
        Self::alloc()
    }
}

impl SpecLimits {
    #[must_use]
    pub const fn alloc() -> Self {
        Self {
            ngu_absolute: NGU_ABSOLUTE,
            ngu_few_limit: if NGU_ABSOLUTE {
                NGU_FEW_ABSDEFAULT
            } else {
                NGU_FEW_DEFAULT
            },
            ngu_many_limit: if NGU_ABSOLUTE {
                NGU_MANY_ABSDEFAULT
            } else {
                NGU_MANY_DEFAULT
            },
            gpc_absolute: GPC_ABSOLUTE,
            gpc_few_limit: if GPC_ABSOLUTE {
                GPC_FEW_ABSDEFAULT
            } else {
                GPC_FEW_DEFAULT
            },
            gpc_many_limit: if GPC_ABSOLUTE {
                GPC_MANY_ABSDEFAULT
            } else {
                GPC_MANY_DEFAULT
            },
            ax_some_limit: AX_SOME_DEFAULT,
            ax_many_limit: AX_MANY_DEFAULT,
            lit_some_limit: LIT_SOME_DEFAULT,
            lit_many_limit: LIT_MANY_DEFAULT,
            term_medium_limit: TERM_MED_DEFAULT,
            term_large_limit: TERM_LARGE_DEFAULT,
            far_sum_medium_limit: FAR_SUM_MED_DEFAULT,
            far_sum_large_limit: FAR_SUM_LARGE_DEFAULT,
            depth_medium_limit: DEPTH_MEDIUM_DEFAULT,
            depth_deep_limit: DEPTH_DEEP_DEFAULT,
            symbols_medium_limit: SYMBOLS_MEDIUM_DEFAULT,
            symbols_large_limit: SYMBOLS_LARGE_DEFAULT,
            predc_medium_limit: PREDC_MEDIUM_DEFAULT,
            predc_large_limit: PREDC_LARGE_DEFAULT,
            pred_medium_limit: PRED_MEDIUM_DEFAULT,
            pred_large_limit: PRED_LARGE_DEFAULT,
            func_medium_limit: FUNC_MEDIUM_DEFAULT,
            func_large_limit: FUNC_LARGE_DEFAULT,
            fun_medium_limit: FUN_MEDIUM_DEFAULT,
            fun_large_limit: FUN_LARGE_DEFAULT,
            order_medium_limit: ORDER_MEDIUM_DEFAULT,
            order_large_limit: ORDER_LARGE_DEFAULT,
            num_of_lams_medium_limit: NUM_LAMS_MEDIUM_DEFAULT,
            num_of_lams_large_limit: NUM_LAMS_LARGE_DEFAULT,
            num_of_defs_medium_limit: DEFS_MEDIUM_DEFAULT,
            num_of_defs_large_limit: DEFS_LARGE_DEFAULT,
            perc_form_defs_medium_limit: DEFS_PERC_MEDIUM_DEFAULT,
            perc_form_defs_large_limit: DEFS_PERC_MEDIUM_DEFAULT,
            perc_app_lits_medium_limit: PERC_APPLIT_MEDIUM_DEFAULT,
            perc_app_lits_large_limit: PERC_APPLIT_LARGE_DEFAULT,
        }
    }

    #[must_use]
    pub const fn default_auto() -> Self {
        Self {
            ax_many_limit: 100_000,
            depth_medium_limit: 4,
            depth_deep_limit: 7,
            perc_form_defs_large_limit: DEFS_PERC_LARGE_DEFAULT,
            ..Self::alloc()
        }
    }
}

#[must_use]
pub const fn create_default_spec_limits() -> SpecLimits {
    SpecLimits::default_auto()
}

#[derive(Clone, Debug, Default, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "C-compatible feature cell mirrors che_clausesetfeatures fields"
)]
pub struct SpecFeatureCell {
    pub axiomtypes: SpecFeatureClass,
    pub goaltypes: SpecFeatureClass,
    pub eq_content: SpecFeatureClass,
    pub ng_unit_content: SpecFeatureClass,
    pub ground_positive_content: SpecFeatureClass,
    pub goals_are_ground: bool,
    pub set_clause_size: SpecFeatureClass,
    pub set_literal_size: SpecFeatureClass,
    pub set_termcell_size: SpecFeatureClass,
    pub max_fun_ar_class: SpecFeatureClass,
    pub avg_fun_ar_class: SpecFeatureClass,
    pub sum_fun_ar_class: SpecFeatureClass,
    pub max_depth_class: SpecFeatureClass,
    pub has_ho_features: bool,
    pub quantifies_booleans: bool,
    pub has_defined_choice: bool,
    pub order_class: SpecFeatureClass,
    pub goal_order_class: SpecFeatureClass,
    pub defs_class: SpecFeatureClass,
    pub form_defs_class: SpecFeatureClass,
    pub appvar_lits_class: SpecFeatureClass,
    pub clauses: i64,
    pub goals: i64,
    pub axioms: i64,
    pub literals: i64,
    pub term_cells: i64,
    pub clause_max_depth: i64,
    pub clause_avg_depth: i64,
    pub unit: i64,
    pub unitgoals: i64,
    pub unitaxioms: i64,
    pub horn: i64,
    pub horngoals: i64,
    pub hornaxioms: i64,
    pub eq_clauses: i64,
    pub peq_clauses: i64,
    pub groundunitaxioms: i64,
    pub positiveaxioms: i64,
    pub groundpositiveaxioms: i64,
    pub groundgoals: i64,
    pub ng_unit_axioms_part: f64,
    pub ground_positive_axioms_part: f64,
    pub max_fun_arity: i32,
    pub avg_fun_arity: i32,
    pub sum_fun_arity: i32,
    pub max_pred_arity: i32,
    pub avg_pred_arity: i32,
    pub sum_pred_arity: i32,
    pub fun_const_count: i32,
    pub fun_nonconst_count: i32,
    pub pred_nonconst_count: i32,
    pub order: i32,
    pub goal_order: i32,
    pub num_of_definitions: i32,
    pub perc_of_form_defs: f64,
    pub perc_of_appvar_lits: f64,
}

#[must_use]
pub const fn spec_no_eq(features: &SpecFeatureCell) -> bool {
    features.eq_clauses == 0
}

#[must_use]
pub fn clause_set_count_goals(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_goal)
}

#[must_use]
pub fn clause_set_count_axioms(set: &ClauseSet) -> i64 {
    set.members() - clause_set_count_goals(set)
}

#[must_use]
pub fn clause_set_count_unit(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_unit)
}

#[must_use]
pub fn clause_set_count_unit_goals(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_unit() && clause.is_goal())
}

#[must_use]
pub fn clause_set_count_unit_axioms(set: &ClauseSet) -> i64 {
    clause_set_count_unit(set) - clause_set_count_unit_goals(set)
}

#[must_use]
pub fn clause_set_is_unit_set(set: &ClauseSet) -> bool {
    set.members() == clause_set_count_unit(set)
}

#[must_use]
pub fn clause_set_axioms_are_unit(set: &ClauseSet) -> bool {
    clause_set_count_unit_axioms(set) == clause_set_count_axioms(set)
}

#[must_use]
pub fn clause_set_goals_are_unit(set: &ClauseSet) -> bool {
    clause_set_count_unit_goals(set) == clause_set_count_goals(set)
}

#[must_use]
pub fn clause_set_count_horn(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_horn)
}

#[must_use]
pub fn clause_set_count_horn_goals(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_horn() && clause.is_goal())
}

#[must_use]
pub fn clause_set_count_horn_axioms(set: &ClauseSet) -> i64 {
    clause_set_count_horn(set) - clause_set_count_horn_goals(set)
}

#[must_use]
pub fn clause_set_is_horn_set(set: &ClauseSet) -> bool {
    set.members() == clause_set_count_horn(set)
}

#[must_use]
pub fn clause_set_axioms_are_horn(set: &ClauseSet) -> bool {
    clause_set_count_horn_axioms(set) == clause_set_count_axioms(set)
}

#[must_use]
pub fn clause_set_goals_are_horn(set: &ClauseSet) -> bool {
    clause_set_count_horn_goals(set) == clause_set_count_goals(set)
}

#[must_use]
pub fn clause_set_count_equational(bank: &TermBank, set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_equational(bank))
}

#[must_use]
pub fn clause_set_is_equational_set(bank: &TermBank, set: &ClauseSet) -> bool {
    set.members() == clause_set_count_equational(bank, set)
}

#[must_use]
pub fn clause_set_is_equational(bank: &TermBank, set: &ClauseSet) -> bool {
    clause_set_count_equational(bank, set) >= 1
}

#[must_use]
pub fn clause_set_count_pure_equational(bank: &TermBank, set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_pure_equational(bank))
}

#[must_use]
pub fn clause_set_is_pure_equational_set(bank: &TermBank, set: &ClauseSet) -> bool {
    set.members() == clause_set_count_pure_equational(bank, set)
}

#[must_use]
pub fn clause_set_count_pos_units(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_demodulator)
}

#[must_use]
pub fn clause_set_count_ground_goals(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_goal() && clause.is_ground())
}

#[must_use]
pub fn clause_set_goals_are_ground(set: &ClauseSet) -> bool {
    clause_set_count_goals(set) == clause_set_count_ground_goals(set)
}

#[must_use]
pub fn clause_set_count_ground(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_ground)
}

#[must_use]
pub fn clause_set_is_ground(set: &ClauseSet) -> bool {
    clause_set_count_ground(set) == set.members()
}

#[must_use]
pub fn clause_set_count_ground_positive_axioms(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_positive() && clause.is_ground())
}

#[must_use]
pub fn clause_set_count_positive_axioms(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_positive)
}

#[must_use]
pub fn clause_set_count_ground_unit_axioms(set: &ClauseSet) -> i64 {
    count_clauses(set, |clause| clause.is_demodulator() && clause.is_ground())
}

#[must_use]
pub fn clause_set_count_non_ground_unit_axioms(set: &ClauseSet) -> i64 {
    clause_set_count_unit_axioms(set) - clause_set_count_ground_unit_axioms(set)
}

#[must_use]
pub fn clause_set_count_range_restricted(set: &ClauseSet) -> i64 {
    count_clauses(set, Clause::is_range_restricted)
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn clause_set_non_ground_axiom_part(set: &ClauseSet) -> f64 {
    let unit_axioms = clause_set_count_unit_axioms(set);
    if unit_axioms == 0 {
        0.0
    } else {
        (unit_axioms - clause_set_count_ground_unit_axioms(set)) as f64 / unit_axioms as f64
    }
}

#[allow(clippy::cast_precision_loss)]
pub fn spec_features_add_basic_eval(features: &mut SpecFeatureCell) {
    features.goals_are_ground = features.groundgoals == features.goals;
    features.axiomtypes = if features.unitaxioms == features.axioms {
        SpecFeatureClass::Unit
    } else if features.hornaxioms == features.axioms {
        SpecFeatureClass::Horn
    } else {
        SpecFeatureClass::General
    };
    features.goaltypes = if features.unitgoals == features.goals {
        SpecFeatureClass::Unit
    } else if features.horngoals == features.goals {
        SpecFeatureClass::Horn
    } else {
        SpecFeatureClass::General
    };
    features.eq_content = if features.peq_clauses == features.clauses {
        SpecFeatureClass::PureEq
    } else if features.eq_clauses != 0 {
        SpecFeatureClass::SomeEq
    } else {
        SpecFeatureClass::NoEq
    };
    features.max_fun_ar_class = arity_feature_class(features.max_fun_arity);
    features.avg_fun_ar_class = arity_feature_class(features.avg_fun_arity);
    features.ng_unit_axioms_part = if features.unitaxioms == 0 {
        0.0
    } else {
        (features.unitaxioms - features.groundunitaxioms) as f64 / features.unitaxioms as f64
    };
    features.ground_positive_axioms_part = if features.positiveaxioms == 0 {
        0.0
    } else {
        features.groundpositiveaxioms as f64 / features.positiveaxioms as f64
    };
}

#[allow(clippy::cast_precision_loss)]
pub fn spec_features_add_eval(features: &mut SpecFeatureCell, limits: &SpecLimits) {
    features.goals_are_ground = features.groundgoals == features.goals;

    if limits.ngu_absolute {
        features.ng_unit_content = SpecFeatureClass::FewPosNonGroundUnits;
        if (features.unitaxioms - features.groundunitaxioms) as f64 > limits.ngu_few_limit {
            features.ng_unit_content = SpecFeatureClass::SomePosNonGroundUnits;
        }
        if (features.unitaxioms - features.groundunitaxioms) as f64 > limits.ngu_many_limit {
            features.ng_unit_content = SpecFeatureClass::ManyPosNonGroundUnits;
        }
    } else if features.ng_unit_axioms_part <= limits.ngu_few_limit {
        features.ng_unit_content = SpecFeatureClass::FewPosNonGroundUnits;
    } else if features.ng_unit_axioms_part >= limits.ngu_many_limit {
        features.ng_unit_content = SpecFeatureClass::ManyPosNonGroundUnits;
    } else {
        features.ng_unit_content = SpecFeatureClass::SomePosNonGroundUnits;
    }

    if limits.gpc_absolute {
        features.ground_positive_content = SpecFeatureClass::FewPosGround;
        if features.groundpositiveaxioms as f64 > limits.gpc_few_limit {
            features.ground_positive_content = SpecFeatureClass::SomePosGround;
        }
        if features.groundpositiveaxioms as f64 > limits.gpc_many_limit {
            features.ground_positive_content = SpecFeatureClass::ManyPosGround;
        }
    } else if features.ground_positive_axioms_part <= limits.gpc_few_limit {
        features.ground_positive_content = SpecFeatureClass::FewPosGround;
    } else if features.ground_positive_axioms_part >= limits.gpc_many_limit {
        features.ground_positive_content = SpecFeatureClass::ManyPosGround;
    } else {
        features.ground_positive_content = SpecFeatureClass::SomePosGround;
    }

    features.set_clause_size =
        size_feature_class_i64(features.clauses, limits.ax_some_limit, limits.ax_many_limit);
    features.set_literal_size = literal_feature_class(
        features.literals,
        limits.lit_some_limit,
        limits.lit_many_limit,
    );
    features.set_termcell_size = term_feature_class(
        features.term_cells,
        limits.term_medium_limit,
        limits.term_large_limit,
    );
    features.max_fun_ar_class = arity_feature_class(features.max_fun_arity);
    features.avg_fun_ar_class = arity_feature_class(features.avg_fun_arity);

    features.ng_unit_axioms_part = if features.unitaxioms == 0 {
        0.0
    } else {
        (features.unitaxioms - features.groundunitaxioms) as f64 / features.unitaxioms as f64
    };
    features.ground_positive_axioms_part = if features.positiveaxioms == 0 {
        0.0
    } else {
        features.groundpositiveaxioms as f64 / features.positiveaxioms as f64
    };

    features.sum_fun_ar_class = if features.sum_fun_arity < limits.far_sum_medium_limit {
        SpecFeatureClass::AritySumSmall
    } else if features.sum_fun_arity < limits.far_sum_large_limit {
        SpecFeatureClass::AritySumMedium
    } else {
        SpecFeatureClass::AritySumLarge
    };
    features.max_depth_class = if features.clause_max_depth < limits.depth_medium_limit {
        SpecFeatureClass::DepthShallow
    } else if features.clause_max_depth < limits.depth_deep_limit {
        SpecFeatureClass::DepthMedium
    } else {
        SpecFeatureClass::DepthDeep
    };
    features.order_class = order_feature_class(features.order);
    features.goal_order_class = order_feature_class(features.goal_order);
    features.defs_class = if features.num_of_definitions < limits.num_of_defs_medium_limit {
        SpecFeatureClass::FewDefs
    } else if features.num_of_definitions < limits.num_of_defs_large_limit {
        SpecFeatureClass::MediumDefs
    } else {
        SpecFeatureClass::ManyDefs
    };
    features.form_defs_class = if features.perc_of_form_defs < limits.perc_form_defs_medium_limit {
        SpecFeatureClass::FewFormDefs
    } else if features.perc_of_form_defs < limits.perc_form_defs_large_limit {
        SpecFeatureClass::MediumFormDefs
    } else {
        SpecFeatureClass::ManyFormDefs
    };
    features.appvar_lits_class = if features.perc_of_appvar_lits < limits.perc_app_lits_medium_limit
    {
        SpecFeatureClass::FewApplits
    } else if features.perc_of_appvar_lits < limits.perc_app_lits_large_limit {
        SpecFeatureClass::MediumApplits
    } else {
        SpecFeatureClass::ManyApplits
    };
}

#[must_use]
pub fn spec_features_print_string(features: &SpecFeatureCell) -> String {
    format!(
        concat!(
            "( {:3}, {:3}, {:3}, {:3}, {:3}, {:3}, {:3}, {:3}, {:3},",
            " {:3}, {:3}, {:3}, {:3}, {:3}, {:3}, {:8.6}, {:8.6},",
            " {:3}, {:3}, {:3}, {:3}, {:3}, {:3}, {:3}, {:8.6}, {:8.6}, {}, {} )"
        ),
        features.goals,
        features.axioms,
        features.clauses,
        features.literals,
        features.term_cells,
        features.unitgoals,
        features.unitaxioms,
        features.horngoals,
        features.hornaxioms,
        features.eq_clauses,
        features.peq_clauses,
        features.groundunitaxioms,
        features.groundgoals,
        features.groundpositiveaxioms,
        features.positiveaxioms,
        features.ng_unit_axioms_part,
        features.ground_positive_axioms_part,
        features.max_fun_arity,
        features.avg_fun_arity,
        features.sum_fun_arity,
        features.clause_max_depth,
        features.clause_avg_depth,
        features.order,
        features.num_of_definitions,
        features.perc_of_form_defs,
        features.perc_of_appvar_lits,
        bool_string(features.quantifies_booleans),
        bool_string(features.has_defined_choice),
    )
}

/// Encodes the C `SpecTypeString` classification using the process problem type.
///
/// # Panics
///
/// Panics if `mask` is shorter than 13 bytes or longer than 22 bytes, matching
/// the C assertion on `strlen(mask)`.
#[must_use]
pub fn spec_type_string(features: &SpecFeatureCell, mask: &str) -> String {
    spec_type_string_for_problem(features, mask, problem_type())
}

/// Encodes the C `SpecTypeString` classification for an explicit problem type.
///
/// # Panics
///
/// Panics if `mask` is shorter than 13 bytes or longer than 22 bytes, matching
/// the C assertion on `strlen(mask)`.
#[must_use]
pub fn spec_type_string_for_problem(
    features: &SpecFeatureCell,
    mask: &str,
    problem_type: ProblemType,
) -> String {
    assert!((13..=SPEC_STRING_MEM).contains(&mask.len()));
    let mut result = [b'-'; SPEC_TYPE_LEN];
    result[0] = if problem_type == ProblemType::HigherOrder {
        b'H'
    } else {
        b'F'
    };
    result[1] = spec_feature_encoding(features.axiomtypes);
    result[2] = spec_feature_encoding(features.goaltypes);
    result[3] = spec_feature_encoding(features.eq_content);
    result[4] = spec_feature_encoding(features.ng_unit_content);
    result[5] = if features.goals_are_ground {
        b'G'
    } else {
        b'N'
    };
    result[6] = spec_feature_encoding(features.set_clause_size);
    result[7] = spec_feature_encoding(features.set_literal_size);
    result[8] = spec_feature_encoding(features.set_termcell_size);
    result[9] = spec_feature_encoding(features.ground_positive_content);
    result[10] = spec_feature_encoding(features.max_fun_ar_class);
    result[11] = spec_feature_encoding(features.avg_fun_ar_class);
    result[12] = spec_feature_encoding(features.sum_fun_ar_class);
    result[13] = spec_feature_encoding(features.max_depth_class);
    result[14] = spec_feature_encoding(features.order_class);
    result[15] = spec_feature_encoding(features.goal_order_class);
    result[16] = spec_feature_encoding(features.defs_class);
    result[17] = spec_feature_encoding(features.form_defs_class);
    result[18] = spec_feature_encoding(features.appvar_lits_class);
    result[19] = if features.quantifies_booleans {
        b'B'
    } else {
        b'N'
    };
    result[20] = if features.has_defined_choice {
        b'C'
    } else {
        b'N'
    };

    for (index, mask_byte) in mask.bytes().take(SPEC_TYPE_LEN).enumerate() {
        if mask_byte == b'-' {
            result[index] = b'-';
        }
    }

    String::from_utf8_lossy(&result).into_owned()
}

#[must_use]
pub fn spec_type_print_string(features: &SpecFeatureCell, mask: &str) -> String {
    spec_type_string(features, mask)
}

#[must_use]
pub fn clause_set_print_pos_units_string<F>(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    render_clause: F,
) -> String
where
    F: FnMut(&Clause) -> String,
{
    clause_set_print_filtered_string(bank, set, print_info, Clause::is_demodulator, render_clause)
}

#[must_use]
pub fn clause_set_print_neg_units_string<F>(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    render_clause: F,
) -> String
where
    F: FnMut(&Clause) -> String,
{
    clause_set_print_filtered_string(
        bank,
        set,
        print_info,
        |clause| clause.is_unit() && clause.is_goal(),
        render_clause,
    )
}

#[must_use]
pub fn clause_set_print_non_units_string<F>(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    render_clause: F,
) -> String
where
    F: FnMut(&Clause) -> String,
{
    clause_set_print_filtered_string(
        bank,
        set,
        print_info,
        |clause| !clause.is_unit(),
        render_clause,
    )
}

#[must_use]
pub fn clause_set_print_pos_units_default_string(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
) -> String {
    clause_set_print_filtered_default_string(bank, set, print_info, Clause::is_demodulator)
}

/// Returns `ClauseSetPrintPosUnits` with explicit `ClausePrint` dispatch.
///
/// # Errors
///
/// Returns a diagnostic if TSTP rendering rejects a selected clause.
pub fn clause_set_print_pos_units_format_string(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    eqn_print_options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    clause_set_print_filtered_default_format_string(
        bank,
        set,
        print_info,
        Clause::is_demodulator,
        output_format,
        problem_type,
        eqn_print_options,
    )
}

#[must_use]
pub fn clause_set_print_neg_units_default_string(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
) -> String {
    clause_set_print_filtered_default_string(bank, set, print_info, |clause| {
        clause.is_unit() && clause.is_goal()
    })
}

/// Returns `ClauseSetPrintNegUnits` with explicit `ClausePrint` dispatch.
///
/// # Errors
///
/// Returns a diagnostic if TSTP rendering rejects a selected clause.
pub fn clause_set_print_neg_units_format_string(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    eqn_print_options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    clause_set_print_filtered_default_format_string(
        bank,
        set,
        print_info,
        |clause| clause.is_unit() && clause.is_goal(),
        output_format,
        problem_type,
        eqn_print_options,
    )
}

#[must_use]
pub fn clause_set_print_non_units_default_string(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
) -> String {
    clause_set_print_filtered_default_string(bank, set, print_info, |clause| !clause.is_unit())
}

/// Returns `ClauseSetPrintNonUnits` with explicit `ClausePrint` dispatch.
///
/// # Errors
///
/// Returns a diagnostic if TSTP rendering rejects a selected clause.
pub fn clause_set_print_non_units_format_string(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    eqn_print_options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    clause_set_print_filtered_default_format_string(
        bank,
        set,
        print_info,
        |clause| !clause.is_unit(),
        output_format,
        problem_type,
        eqn_print_options,
    )
}

/// Returns the C `ProofStatePrintSelective` shape for the currently ported
/// clause and equality-axiom output helpers.
///
/// # Errors
///
/// Returns diagnostics for invalid descriptor characters, type-declaration
/// rendering failures, or equality-axiom output formats that are not yet
/// supported by the lower-level equality printer.
pub fn proof_state_print_selective_string(
    state: &ProofState,
    descriptor: &str,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    eqn_print_options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    let mut output = String::new();
    let print_context = SelectiveClausePrintContext {
        bank: state.terms(),
        print_info,
        output_format,
        problem_type,
        eqn_print_options,
    };
    for current in descriptor.bytes() {
        match current {
            b't' => {
                if problem_type == ProblemType::HigherOrder || !state.axioms().is_untyped() {
                    push_comment_line(&mut output, "Type declarations:");
                    output.push_str(&type_decls_tstp_string(state, problem_type)?);
                }
            }
            b'e' => {
                push_selective_clause_section(
                    &mut output,
                    "Processed positive unit clauses:",
                    &[state.processed_pos_rules(), state.processed_pos_eqns()],
                    print_context,
                    clause_set_print_pos_units_with_options,
                )?;
            }
            b'i' => {
                push_selective_clause_section(
                    &mut output,
                    "Processed negative unit clauses:",
                    &[state.processed_neg_units()],
                    print_context,
                    clause_set_print_neg_units_with_options,
                )?;
            }
            b'g' => {
                push_selective_clause_section(
                    &mut output,
                    "Processed non-unit clauses:",
                    &[state.processed_non_units()],
                    print_context,
                    clause_set_print_non_units_with_options,
                )?;
            }
            b'E' => {
                push_selective_clause_section(
                    &mut output,
                    "Unprocessed positive unit clauses:",
                    &[state.unprocessed()],
                    print_context,
                    clause_set_print_pos_units_with_options,
                )?;
            }
            b'I' => {
                push_selective_clause_section(
                    &mut output,
                    "Unprocessed negative unit clauses:",
                    &[state.unprocessed()],
                    print_context,
                    clause_set_print_neg_units_with_options,
                )?;
            }
            b'G' => {
                push_selective_clause_section(
                    &mut output,
                    "Unprocessed non-unit clauses:",
                    &[state.unprocessed()],
                    print_context,
                    clause_set_print_non_units_with_options,
                )?;
            }
            b'a' | b'A' => {
                if clause_set_is_equational(state.terms(), state.axioms()) {
                    push_comment_line(&mut output, "Equality axioms:");
                    output.push_str(&eq_axioms_print_string(
                        state.terms().signature(),
                        output_format,
                        current == b'a',
                    )?);
                } else {
                    push_comment_line(&mut output, "No equality axioms required.");
                }
            }
            _ => {
                return Err(Diagnostic::new(
                    ErrorCode::OTHER_ERROR,
                    format!(
                        "Illegal character '{}' in proof-state print descriptor",
                        char::from(current)
                    ),
                ));
            }
        }
    }
    Ok(output)
}

#[derive(Clone, Copy)]
struct SelectiveClausePrintContext<'bank> {
    bank: &'bank TermBank,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    eqn_print_options: EqnPrintOptions,
}

type SelectiveClausePrintFn = fn(
    &TermBank,
    &ClauseSet,
    bool,
    IoFormat,
    ProblemType,
    EqnPrintOptions,
) -> Result<String, Diagnostic>;

fn push_selective_clause_section(
    output: &mut String,
    header: &str,
    sets: &[&ClauseSet],
    context: SelectiveClausePrintContext<'_>,
    render_set: SelectiveClausePrintFn,
) -> Result<(), Diagnostic> {
    push_comment_line(output, header);
    for set in sets {
        output.push_str(&render_set(
            context.bank,
            set,
            context.print_info,
            context.output_format,
            context.problem_type,
            context.eqn_print_options,
        )?);
    }
    output.push('\n');
    Ok(())
}

fn push_comment_line(output: &mut String, text: &str) {
    output.push_str(DEFAULT_COMCHAR_RAW);
    output.push(' ');
    output.push_str(text);
    output.push('\n');
}

fn clause_set_print_pos_units_with_options(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    clause_set_print_pos_units_format_string(
        bank,
        set,
        print_info,
        output_format,
        problem_type,
        options,
    )
}

fn clause_set_print_neg_units_with_options(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    clause_set_print_neg_units_format_string(
        bank,
        set,
        print_info,
        output_format,
        problem_type,
        options,
    )
}

fn clause_set_print_non_units_with_options(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    output_format: IoFormat,
    problem_type: ProblemType,
    options: EqnPrintOptions,
) -> Result<String, Diagnostic> {
    clause_set_print_non_units_format_string(
        bank,
        set,
        print_info,
        output_format,
        problem_type,
        options,
    )
}

fn type_decls_tstp_string(
    state: &ProofState,
    problem_type: ProblemType,
) -> Result<String, Diagnostic> {
    let mut output = Vec::new();
    state
        .terms()
        .signature()
        .print_type_decls_tstp(&mut output, problem_type)
        .map_err(|error| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                format!("Failed to print type declarations: {error}"),
            )
        })?;
    String::from_utf8(output).map_err(|error| {
        Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            format!("Type declaration output was not valid UTF-8: {error}"),
        )
    })
}

pub fn spec_features_parse(
    scanner: &mut Scanner,
    features: &mut SpecFeatureCell,
) -> Result<(), Diagnostic> {
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    features.goals = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.axioms = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.clauses = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.literals = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.term_cells = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.unitgoals = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.unitaxioms = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.horngoals = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.hornaxioms = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.eq_clauses = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.peq_clauses = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.groundunitaxioms = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.groundgoals = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.groundpositiveaxioms = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.positiveaxioms = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.ng_unit_axioms_part = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.ground_positive_axioms_part = parse_float(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.max_fun_arity = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.avg_fun_arity = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.sum_fun_arity = parse_i32(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.clause_max_depth = parse_int(scanner)?;
    scanner.accept_tok(TokenType::COMMA)?;
    features.clause_avg_depth = parse_int(scanner)?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::COLON)?;

    let class = parse_plain_filename(scanner)?;
    parse_spec_class(features, &class)
}

#[must_use]
pub fn spec_limits_print_string(limits: &SpecLimits) -> String {
    format!(
        concat!(
            "[ {} | {} | {} | {} | {} | {} |",
            " {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | ",
            " {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | ",
            " {} | {} | {} | {} ]\n"
        ),
        bool_int(limits.ngu_absolute),
        c_g_float(limits.ngu_few_limit),
        c_g_float(limits.ngu_many_limit),
        bool_int(limits.gpc_absolute),
        c_g_float(limits.gpc_few_limit),
        c_g_float(limits.gpc_many_limit),
        limits.ax_some_limit,
        limits.ax_many_limit,
        limits.lit_some_limit,
        limits.lit_many_limit,
        limits.term_medium_limit,
        limits.term_large_limit,
        limits.far_sum_medium_limit,
        limits.far_sum_large_limit,
        limits.depth_medium_limit,
        limits.depth_deep_limit,
        limits.symbols_medium_limit,
        limits.symbols_large_limit,
        limits.predc_medium_limit,
        limits.predc_large_limit,
        limits.pred_medium_limit,
        limits.pred_large_limit,
        limits.func_medium_limit,
        limits.func_large_limit,
        limits.fun_medium_limit,
        limits.fun_large_limit,
        limits.order_medium_limit,
        limits.order_large_limit,
        limits.num_of_lams_medium_limit,
        limits.num_of_lams_large_limit,
        limits.num_of_defs_medium_limit,
        limits.num_of_defs_large_limit,
        c_g_float(limits.perc_form_defs_medium_limit),
        c_g_float(limits.perc_form_defs_large_limit),
        c_g_float(limits.perc_app_lits_medium_limit),
        c_g_float(limits.perc_app_lits_large_limit),
    )
}

/// Collects the arity statistics used by the C strategy feature extractor.
///
/// # Panics
///
/// Panics if a positive f-code in `signature` has no arity entry, or if the
/// signature f-code count cannot be represented as a Rust vector size.
#[must_use]
pub fn clause_set_collect_arity_information(
    set: &ClauseSet,
    signature: &Signature,
) -> ClauseSetArityInformation {
    let mut max_fun_arity = 0;
    let mut sum_fun_arity = 0;
    let mut fun_count = 0;
    let mut fun_const_count = 0;
    let mut non_const_preds = 0;
    let mut max_pred_arity = 0;
    let mut sum_pred_arity = 0;
    let mut pred_count = 0;
    let mut dist_array = vec![0; fcode_index(signature.f_count() + 1)];

    set.add_symbol_distribution(&mut dist_array);

    for symbol in 1..=signature.f_count() {
        let index = fcode_index(symbol);
        if signature.is_special(symbol) || dist_array[index] == 0 {
            continue;
        }
        let arity = signature
            .find_arity(symbol)
            .unwrap_or_else(|| panic!("signature arity must exist for positive f-code"));
        if signature.is_predicate(symbol) {
            max_pred_arity = max_pred_arity.max(arity);
            sum_pred_arity += arity;
            pred_count += 1;
            if arity != 0 {
                non_const_preds += 1;
            }
        } else if arity != 0 {
            max_fun_arity = max_fun_arity.max(arity);
            sum_fun_arity += arity;
            fun_count += 1;
        } else {
            fun_const_count += 1;
        }
    }

    ClauseSetArityInformation {
        max_fun_arity,
        avg_fun_arity: if fun_count == 0 {
            0
        } else {
            sum_fun_arity / fun_count
        },
        sum_fun_arity,
        max_pred_arity,
        avg_pred_arity: if pred_count == 0 {
            0
        } else {
            sum_pred_arity / pred_count
        },
        sum_pred_arity,
        non_const_funs: fun_count,
        non_const_preds,
        fun_const_count,
    }
}

/// Computes the clause-set higher-order statistics from
/// `ClauseSetComputeHOFeatures`.
///
/// C also recognizes defined-choice clauses through `ClauseRecognizeChoice`,
/// which beta/eta-normalizes lambda/DB terms and optionally records the
/// recognized choice symbol in a side map. The proof-state initialization path
/// owns that side map; this feature layer stays side-effect-free, so callers
/// supply the boolean recognizer. Passing `|_| false` preserves the non-choice
/// parts exactly.
///
/// # Panics
///
/// Panics if an external signature symbol or collected variable has no type, or
/// if the computed type order cannot fit the C `int` result type.
#[must_use]
#[expect(
    clippy::cast_precision_loss,
    reason = "C computes this higher-order feature ratio as double"
)]
pub fn clause_set_compute_ho_features<F>(
    set: &ClauseSet,
    signature: &Signature,
    mut recognize_choice: F,
) -> ClauseSetHoFeatures
where
    F: FnMut(&Clause) -> bool,
{
    let mut is_fo = true;
    let mut quantifies_booleans = false;
    let mut has_defined_choice = false;
    let mut app_var_lit_count = 0;
    let mut order = 0;

    for symbol in (signature.internal_symbols() + 1)..=signature.f_count() {
        let type_ = signature
            .get_type(symbol)
            .unwrap_or_else(|| panic!("external signature symbol {symbol} must have a type"));
        order = order.max(type_get_order(type_));
    }

    for clause in set.iter() {
        let mut variables = BTreeMap::new();
        clause.collect_variables(&mut variables);
        for variable in variables.values() {
            let type_ = variable
                .type_()
                .unwrap_or_else(|| panic!("collected variable must have a type"));
            order = order.max(var_order(&type_));
            quantifies_booleans = quantifies_booleans || type_has_bool(&type_);
        }

        let mut has_app_var = false;
        for literal in clause.literals().as_slice() {
            is_fo = is_fo
                && term_is_first_order_for_ho_features(literal.left())
                && term_is_first_order_for_ho_features(literal.right());
            has_app_var = has_app_var
                || literal.left().is_applied_free_var()
                || literal.right().is_applied_free_var();
        }

        has_defined_choice = has_defined_choice || recognize_choice(clause);
        app_var_lit_count += i64::from(has_app_var);
    }

    ClauseSetHoFeatures {
        has_ho_features: !is_fo,
        order: i32::try_from(order)
            .unwrap_or_else(|_| panic!("higher-order feature order must fit C int")),
        quantifies_booleans,
        has_defined_choice,
        perc_app_var_lits: if set.members() == 0 {
            0.0
        } else {
            app_var_lit_count as f64 / set.members() as f64
        },
    }
}

#[must_use]
pub fn clause_set_compute_ho_features_without_choice(
    set: &ClauseSet,
    signature: &Signature,
) -> ClauseSetHoFeatures {
    clause_set_compute_ho_features(set, signature, |_| false)
}

/// Computes `ClauseSetComputeHOFeatures` with the C no-map
/// `ClauseRecognizeChoice(NULL, clause)` behavior.
///
/// # Errors
///
/// Returns diagnostics from choice-axiom beta normalization.
///
/// # Panics
///
/// Panics under the same type/order invariants as
/// [`clause_set_compute_ho_features`].
#[expect(
    clippy::cast_precision_loss,
    reason = "C computes this higher-order feature ratio as double"
)]
pub fn clause_set_compute_ho_features_with_choice_recognition(
    set: &ClauseSet,
    bank: &mut TermBank,
) -> Result<ClauseSetHoFeatures, Diagnostic> {
    let mut is_fo = true;
    let mut quantifies_booleans = false;
    let mut has_defined_choice = false;
    let mut app_var_lit_count = 0;
    let mut order = 0;

    for symbol in (bank.signature().internal_symbols() + 1)..=bank.signature().f_count() {
        let type_ = bank
            .signature()
            .get_type(symbol)
            .unwrap_or_else(|| panic!("external signature symbol {symbol} must have a type"));
        order = order.max(type_get_order(type_));
    }

    for clause in set.iter() {
        let mut variables = BTreeMap::new();
        clause.collect_variables(&mut variables);
        for variable in variables.values() {
            let type_ = variable
                .type_()
                .unwrap_or_else(|| panic!("collected variable must have a type"));
            order = order.max(var_order(&type_));
            quantifies_booleans = quantifies_booleans || type_has_bool(&type_);
        }

        let mut has_app_var = false;
        for literal in clause.literals().as_slice() {
            is_fo = is_fo
                && term_is_first_order_for_ho_features(literal.left())
                && term_is_first_order_for_ho_features(literal.right());
            has_app_var = has_app_var
                || literal.left().is_applied_free_var()
                || literal.right().is_applied_free_var();
        }

        has_defined_choice = has_defined_choice || clause_recognizes_choice(bank, clause)?;
        app_var_lit_count += i64::from(has_app_var);
    }

    Ok(ClauseSetHoFeatures {
        has_ho_features: !is_fo,
        order: i32::try_from(order)
            .unwrap_or_else(|_| panic!("higher-order feature order must fit C int")),
        quantifies_booleans,
        has_defined_choice,
        perc_app_var_lits: if set.members() == 0 {
            0.0
        } else {
            app_var_lit_count as f64 / set.members() as f64
        },
    })
}

/// Computes the clause-set portion of C `SpecFeaturesCompute`.
///
/// Formula-set order scanning and formula-definition statistics are not owned
/// by `ClauseSet`, so this helper deliberately stops at the clause/bank
/// boundary. Like C, it computes the higher-order clause aggregate, then resets
/// `order` and `goal_order` to `1` so later formula scans can raise them.
///
/// # Panics
///
/// Panics if symbol/type data required by the underlying feature helpers is
/// missing, or if the constant-function count cannot fit C's `int` field.
pub fn spec_features_compute_clause_set<F>(
    features: &mut SpecFeatureCell,
    set: &ClauseSet,
    bank: &TermBank,
    recognize_choice: F,
) where
    F: FnMut(&Clause) -> bool,
{
    let ho_features = clause_set_compute_ho_features(set, bank.signature(), recognize_choice);
    spec_features_compute_clause_set_with_ho_features(features, set, bank, ho_features);
}

fn spec_features_compute_clause_set_with_ho_features(
    features: &mut SpecFeatureCell,
    set: &ClauseSet,
    bank: &TermBank,
    ho_features: ClauseSetHoFeatures,
) {
    features.clauses = set.members();
    features.goals = clause_set_count_goals(set);
    features.axioms = features.clauses - features.goals;
    features.literals = set.literals();
    features.term_cells = clause_set_term_cells(bank, set);

    let mut depth_sum = 0;
    let mut count = 0;
    features.clause_max_depth = 0;
    clause_set_tptp_depth_info_add(
        bank,
        set,
        &mut features.clause_max_depth,
        &mut depth_sum,
        &mut count,
    );
    features.clause_avg_depth = if count == 0 { 0 } else { depth_sum / count };

    features.unit = clause_set_count_unit(set);
    features.unitgoals = clause_set_count_unit_goals(set);
    features.unitaxioms = features.unit - features.unitgoals;

    features.horn = clause_set_count_horn(set);
    features.horngoals = clause_set_count_horn_goals(set);
    features.hornaxioms = features.horn - features.horngoals;

    features.eq_clauses = clause_set_count_equational(bank, set);
    features.peq_clauses = clause_set_count_pure_equational(bank, set);
    features.groundunitaxioms = clause_set_count_ground_unit_axioms(set);
    features.groundgoals = clause_set_count_ground_goals(set);
    features.positiveaxioms = clause_set_count_positive_axioms(set);
    features.groundpositiveaxioms = clause_set_count_ground_positive_axioms(set);

    let arity = clause_set_collect_arity_information(set, bank.signature());
    features.max_fun_arity = arity.max_fun_arity;
    features.avg_fun_arity = arity.avg_fun_arity;
    features.sum_fun_arity = arity.sum_fun_arity;
    features.max_pred_arity = arity.max_pred_arity;
    features.avg_pred_arity = arity.avg_pred_arity;
    features.sum_pred_arity = arity.sum_pred_arity;
    features.fun_nonconst_count = arity.non_const_funs;
    features.pred_nonconst_count = arity.non_const_preds;
    features.fun_const_count = i32::try_from(arity.fun_const_count)
        .unwrap_or_else(|_| panic!("function constant count must fit C int"));

    spec_features_add_basic_eval(features);

    features.num_of_definitions = -1;
    features.has_ho_features = ho_features.has_ho_features;
    features.quantifies_booleans = ho_features.quantifies_booleans;
    features.has_defined_choice = ho_features.has_defined_choice;
    features.perc_of_appvar_lits = ho_features.perc_app_var_lits;
    features.order = 1;
    features.goal_order = 1;
}

/// Computes the clause-set portion of C `SpecFeaturesCompute` without the
/// defined-choice recognizer.
///
/// This is useful for callers that need side-effect-free feature extraction
/// without running the choice-axiom beta-normalization check.
///
/// # Panics
///
/// Panics under the same conditions as
/// [`spec_features_compute_clause_set`].
pub fn spec_features_compute_clause_set_without_choice(
    features: &mut SpecFeatureCell,
    set: &ClauseSet,
    bank: &TermBank,
) {
    spec_features_compute_clause_set(features, set, bank, |_| false);
}

/// Computes the clause-set portion of C `SpecFeaturesCompute` with built-in
/// no-map choice recognition.
///
/// # Errors
///
/// Returns diagnostics from choice-axiom beta normalization.
///
/// # Panics
///
/// Panics under the same conditions as
/// [`spec_features_compute_clause_set`].
pub fn spec_features_compute_clause_set_with_choice_recognition(
    features: &mut SpecFeatureCell,
    set: &ClauseSet,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    let ho_features = clause_set_compute_ho_features_with_choice_recognition(set, bank)?;
    spec_features_compute_clause_set_with_ho_features(features, set, bank, ho_features);
    Ok(())
}

/// Computes C `SpecFeaturesCompute`, including optional active/archive formula
/// order scans.
///
/// C leaves formula-definition statistics at their sentinel values here: it
/// sets `num_of_definitions = -1`, does not assign `perc_of_form_defs`, and
/// only lets formula sets raise `order`/`goal_order` after clause HO order was
/// computed and reset. `None` mirrors C's null formula-set arguments.
///
/// # Panics
///
/// Panics under the same conditions as [`spec_features_compute_clause_set`], or
/// if a formula term order cannot fit C's `int` result type.
pub fn spec_features_compute<F>(
    features: &mut SpecFeatureCell,
    set: &ClauseSet,
    fset: Option<&FormulaSet>,
    farch: Option<&FormulaSet>,
    bank: &TermBank,
    recognize_choice: F,
) where
    F: FnMut(&Clause) -> bool,
{
    spec_features_compute_clause_set(features, set, bank, recognize_choice);
    for formulas in [farch, fset].into_iter().flatten() {
        spec_features_scan_formula_order(features, bank.signature(), formulas);
    }
}

/// Computes C `SpecFeaturesCompute` with no defined-choice recognizer.
///
/// # Panics
///
/// Panics under the same conditions as [`spec_features_compute`].
pub fn spec_features_compute_without_choice(
    features: &mut SpecFeatureCell,
    set: &ClauseSet,
    fset: Option<&FormulaSet>,
    farch: Option<&FormulaSet>,
    bank: &TermBank,
) {
    spec_features_compute(features, set, fset, farch, bank, |_| false);
}

/// Computes C `SpecFeaturesCompute` with built-in no-map defined-choice
/// recognition.
///
/// # Errors
///
/// Returns diagnostics from choice-axiom beta normalization.
///
/// # Panics
///
/// Panics under the same conditions as [`spec_features_compute`].
pub fn spec_features_compute_with_choice_recognition(
    features: &mut SpecFeatureCell,
    set: &ClauseSet,
    fset: Option<&FormulaSet>,
    farch: Option<&FormulaSet>,
    bank: &mut TermBank,
) -> Result<(), Diagnostic> {
    spec_features_compute_clause_set_with_choice_recognition(features, set, bank)?;
    for formulas in [farch, fset].into_iter().flatten() {
        spec_features_scan_formula_order(features, bank.signature(), formulas);
    }
    Ok(())
}

fn spec_features_scan_formula_order(
    features: &mut SpecFeatureCell,
    signature: &Signature,
    formulas: &FormulaSet,
) {
    for formula in formulas.iter() {
        let order = usize_to_i32(formula.conjecture_order(signature));
        features.order = features.order.max(order);
        if formula.is_conjecture() || formula.is_hypothesis() {
            features.goal_order = features.goal_order.max(order);
        }
    }
}

#[must_use]
pub fn clause_set_count_maximal_terms(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_maximal_terms).sum()
}

#[must_use]
pub fn clause_set_count_maximal_literals(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_maximal_literals).sum()
}

/// Counts distinct variable f-codes per clause and sums the clause counts.
///
/// # Panics
///
/// Panics under the same conditions as
/// [`crate::heuristics::clausefeatures::clause_count_variable_set`].
#[must_use]
pub fn clause_set_count_variables(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_variable_set).sum()
}

/// Counts singleton variable f-codes per clause and sums the clause counts.
///
/// # Panics
///
/// Panics under the same conditions as
/// [`crate::heuristics::clausefeatures::clause_count_singleton_set`].
#[must_use]
pub fn clause_set_count_singletons(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_singleton_set).sum()
}

/// Adds TPTP-style depth statistics for all clauses in `set`.
///
/// # Panics
///
/// Panics under the same conditions as
/// [`crate::heuristics::clausefeatures::clause_tptp_depth_info_add`].
pub fn clause_set_tptp_depth_info_add(
    bank: &TermBank,
    set: &ClauseSet,
    depthmax: &mut i64,
    depthsum: &mut i64,
    count: &mut i64,
) -> i64 {
    for clause in set.iter() {
        clause_tptp_depth_info_add(bank, clause, depthmax, depthsum, count);
    }
    *depthmax
}

#[must_use]
pub fn clause_set_count_unorientable_literals(set: &ClauseSet) -> i64 {
    set.iter().map(clause_count_unorientable_literals).sum()
}

#[must_use]
pub fn clause_set_count_eqn_literals(set: &ClauseSet) -> i64 {
    set.iter()
        .map(|clause| usize_to_i64(clause.prop_lit_number(EP_IS_EQU_LITERAL)))
        .sum()
}

#[must_use]
pub fn clause_set_max_standard_weight(set: &ClauseSet) -> i64 {
    set.find_max_standard_weight()
        .map_or(-1, Clause::standard_weight)
}

#[must_use]
pub fn clause_set_term_cells(bank: &TermBank, set: &ClauseSet) -> i64 {
    set.iter()
        .map(|clause| {
            clause_weight_to_i64(clause.literal_weight(bank, 1.0, 1.0, 1.0, 1, 1, 1.0, false))
        })
        .sum()
}

#[must_use]
pub fn clause_set_max_literal_number(set: &ClauseSet) -> i64 {
    set.iter()
        .map(|clause| usize_to_i64(clause.literal_number()))
        .max()
        .unwrap_or(0)
}

fn count_clauses<F>(set: &ClauseSet, predicate: F) -> i64
where
    F: Fn(&Clause) -> bool,
{
    usize_to_i64(set.iter().filter(|clause| predicate(clause)).count())
}

fn term_is_first_order_for_ho_features(term: &Term) -> bool {
    !(term.is_non_fo_pattern() || term.has_lambda_subterm() || term.has_db_subterm())
}

fn clause_set_print_filtered_string<P, R>(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    mut predicate: P,
    mut render_clause: R,
) -> String
where
    P: FnMut(&Clause) -> bool,
    R: FnMut(&Clause) -> String,
{
    let mut result = String::new();
    for clause in set.iter() {
        if predicate(clause) {
            let rendered = render_clause(clause);
            result.push_str(&clause_line_string(bank, &rendered, clause, print_info));
        }
    }
    result
}

fn clause_set_print_filtered_default_string<P>(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    mut predicate: P,
) -> String
where
    P: FnMut(&Clause) -> bool,
{
    let mut result = String::new();
    for clause in set.iter() {
        if predicate(clause) {
            result.push_str(&clause_line_print_string(bank, clause, print_info));
        }
    }
    result
}

fn clause_set_print_filtered_default_format_string<P>(
    bank: &TermBank,
    set: &ClauseSet,
    print_info: bool,
    mut predicate: P,
    output_format: IoFormat,
    problem_type: ProblemType,
    eqn_print_options: EqnPrintOptions,
) -> Result<String, Diagnostic>
where
    P: FnMut(&Clause) -> bool,
{
    let mut result = String::new();
    for clause in set.iter() {
        if predicate(clause) {
            result.push_str(&clause_line_print_format_string_with_options(
                bank,
                clause,
                print_info,
                output_format,
                problem_type,
                eqn_print_options,
            )?);
        }
    }
    Ok(result)
}

fn arity_feature_class(arity: i32) -> SpecFeatureClass {
    match arity {
        0 => SpecFeatureClass::Arity0,
        1 => SpecFeatureClass::Arity1,
        2 => SpecFeatureClass::Arity2,
        _ => SpecFeatureClass::Arity3Plus,
    }
}

fn size_feature_class_i64(value: i64, some_limit: i64, many_limit: i64) -> SpecFeatureClass {
    if value < some_limit {
        SpecFeatureClass::FewAxioms
    } else if value < many_limit {
        SpecFeatureClass::SomeAxioms
    } else {
        SpecFeatureClass::ManyAxioms
    }
}

fn literal_feature_class(value: i64, some_limit: i64, many_limit: i64) -> SpecFeatureClass {
    if value < some_limit {
        SpecFeatureClass::FewLiterals
    } else if value < many_limit {
        SpecFeatureClass::SomeLiterals
    } else {
        SpecFeatureClass::ManyLiterals
    }
}

fn term_feature_class(value: i64, medium_limit: i64, large_limit: i64) -> SpecFeatureClass {
    if value < medium_limit {
        SpecFeatureClass::SmallTerms
    } else if value < large_limit {
        SpecFeatureClass::MediumTerms
    } else {
        SpecFeatureClass::LargeTerms
    }
}

fn order_feature_class(order: i32) -> SpecFeatureClass {
    match order.cmp(&2) {
        std::cmp::Ordering::Less => SpecFeatureClass::Fo,
        std::cmp::Ordering::Equal => SpecFeatureClass::So,
        std::cmp::Ordering::Greater => {
            assert!(order >= 3, "higher-order feature class requires order >= 3");
            SpecFeatureClass::Ho
        }
    }
}

fn spec_feature_encoding(class: SpecFeatureClass) -> u8 {
    let index = usize::try_from(class as i32)
        .unwrap_or_else(|_| panic!("SpecFeatureClass discriminant must be non-negative"));
    *SPEC_FEATURE_ENCODING
        .get(index)
        .unwrap_or_else(|| panic!("SpecFeatureClass discriminant must have a C encoding"))
}

fn parse_spec_class(features: &mut SpecFeatureCell, class: &str) -> Result<(), Diagnostic> {
    let class = class.as_bytes();
    if class.len() < 5 {
        return Err(spec_class_error(
            "Insufficient class information in class name(s) (to short)",
        ));
    }

    features.axiomtypes = match class[0] {
        b'G' => SpecFeatureClass::General,
        b'H' => SpecFeatureClass::Horn,
        b'U' => SpecFeatureClass::Unit,
        _ => {
            return Err(spec_class_error(
                "Insufficient class information in class name(s)",
            ));
        }
    };
    features.goaltypes = match class[1] {
        b'H' => SpecFeatureClass::Horn,
        b'U' => SpecFeatureClass::Unit,
        _ => {
            return Err(spec_class_error(
                "Insufficient class information in class name(s)",
            ));
        }
    };
    features.eq_content = match class[2] {
        b'N' => SpecFeatureClass::NoEq,
        b'S' => SpecFeatureClass::SomeEq,
        b'P' => SpecFeatureClass::PureEq,
        _ => {
            return Err(spec_class_error(
                "Insufficient class information in class name(s)",
            ));
        }
    };
    features.goals_are_ground = match class[4] {
        b'G' => true,
        b'N' => false,
        _ => {
            return Err(spec_class_error(
                "Insufficient class information in class name(s)",
            ));
        }
    };
    Ok(())
}

fn spec_class_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

fn bool_string(value: bool) -> &'static str {
    bool_to_str(value)
}

const fn bool_int(value: bool) -> i32 {
    if value {
        1
    } else {
        0
    }
}

fn c_g_float(value: f64) -> String {
    format!("{value}")
}

fn fcode_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).unwrap_or_else(|_| panic!("f-code must fit feature-array index"))
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or_else(|_| panic!("value must fit C int"))
}

fn parse_i32(scanner: &mut Scanner) -> Result<i32, Diagnostic> {
    parse_int(scanner).map(i64_to_i32)
}

#[allow(clippy::cast_possible_truncation)]
fn i64_to_i32(value: i64) -> i32 {
    value as i32
}

#[allow(clippy::cast_possible_truncation)]
fn clause_weight_to_i64(weight: f64) -> i64 {
    weight as i64
}

#[cfg(test)]
mod tests {
    use super::{
        clause_set_axioms_are_horn, clause_set_axioms_are_unit,
        clause_set_collect_arity_information, clause_set_compute_ho_features,
        clause_set_compute_ho_features_with_choice_recognition,
        clause_set_compute_ho_features_without_choice, clause_set_count_axioms,
        clause_set_count_eqn_literals, clause_set_count_equational, clause_set_count_goals,
        clause_set_count_ground, clause_set_count_ground_goals,
        clause_set_count_ground_positive_axioms, clause_set_count_ground_unit_axioms,
        clause_set_count_horn, clause_set_count_horn_axioms, clause_set_count_horn_goals,
        clause_set_count_maximal_literals, clause_set_count_maximal_terms,
        clause_set_count_non_ground_unit_axioms, clause_set_count_pos_units,
        clause_set_count_positive_axioms, clause_set_count_pure_equational,
        clause_set_count_range_restricted, clause_set_count_singletons, clause_set_count_unit,
        clause_set_count_unit_axioms, clause_set_count_unit_goals,
        clause_set_count_unorientable_literals, clause_set_count_variables,
        clause_set_goals_are_ground, clause_set_goals_are_horn, clause_set_goals_are_unit,
        clause_set_is_equational, clause_set_is_equational_set, clause_set_is_ground,
        clause_set_is_horn_set, clause_set_is_pure_equational_set, clause_set_is_unit_set,
        clause_set_max_literal_number, clause_set_max_standard_weight,
        clause_set_non_ground_axiom_part, clause_set_print_neg_units_default_string,
        clause_set_print_neg_units_format_string, clause_set_print_neg_units_string,
        clause_set_print_non_units_default_string, clause_set_print_non_units_format_string,
        clause_set_print_non_units_string, clause_set_print_pos_units_default_string,
        clause_set_print_pos_units_format_string, clause_set_print_pos_units_string,
        clause_set_term_cells, clause_set_tptp_depth_info_add, create_default_spec_limits,
        proof_state_print_selective_string, spec_features_add_basic_eval, spec_features_add_eval,
        spec_features_compute, spec_features_compute_clause_set,
        spec_features_compute_with_choice_recognition, spec_features_parse,
        spec_features_print_string, spec_limits_print_string, spec_type_print_string,
        spec_type_string_for_problem, ClauseSetHoFeatures, SpecFeatureCell, SpecFeatureClass,
        SpecLimits, DEFAULT_CLASS_MASK, DEFAULT_OUTPUT_DESCRIPTOR, SPEC_STRING_MEM,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clause_props::CP_TYPE_HYPOTHESIS;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::{Eqn, EqnPrintOptions};
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::formulasets::{FormulaSet, WrappedFormula};
    use crate::clauses::proofstate::proof_state_alloc;
    use crate::heuristics::clausefeatures::{
        clause_count_maximal_literals, clause_count_maximal_terms, clause_count_singleton_set,
        clause_count_unorientable_literals, clause_count_variable_set, clause_tptp_depth_info_add,
    };
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::terms::functypes::FunCode;
    use crate::terms::lambda::apply_terms as lambda_apply_terms;
    use crate::terms::signature::{Signature, FP_IGNORE_PROPS};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term, TP_HAS_LAMBDA_SUBTERM};
    use crate::terms::typebanks::TypeBank;

    fn term_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        TermBank::new(signature).unwrap_or_else(|err| panic!("{err}"))
    }

    fn individual(bank: &TermBank) -> Type {
        bank.signature().type_bank().default_type()
    }

    fn typed_const(bank: &mut TermBank, name: &str) -> Term {
        let type_ = individual(bank);
        typed_const_with_type(bank, name, &type_)
    }

    fn typed_const_with_type(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let f_code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(f_code, type_.clone())
            .unwrap_or_else(|err| panic!("{err}"));
        bank.create_const_term(f_code)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let type_ = individual(bank);
        let f_code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(f_code, alloc_arrow_type(vec![type_.clone(), type_.clone()]))
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 1);
        term.set_type(Some(type_));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = individual(bank);
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_.clone(), type_.clone()]),
            )
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(type_));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_predicate_binary(bank: &mut TermBank, name: &str, left: &Term, right: &Term) -> Term {
        let type_ = individual(bank);
        let bool_type = bank.signature().type_bank().bool_type();
        let f_code = bank.signature_mut().insert_id(name, 2, false);
        bank.signature_mut()
            .declare_final_type(
                f_code,
                alloc_arrow_type(vec![type_.clone(), type_, bool_type.clone()]),
            )
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(f_code, 2);
        term.set_type(Some(bool_type));
        term.set_argument(0, left.clone());
        term.set_argument(1, right.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_var(bank: &TermBank, f_code: FunCode) -> Term {
        bank.vars().var_assert_alloc(f_code, &individual(bank))
    }

    fn predicate_var(bank: &mut TermBank, f_code: FunCode) -> Term {
        let individual = individual(bank);
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![individual, bool_type]));
        bank.vars().var_assert_alloc(f_code, &predicate_type)
    }

    fn apply_many(bank: &mut TermBank, head: &Term, args: &[Term]) -> Term {
        lambda_apply_terms(bank, head, args).unwrap_or_else(|err| panic!("{err}"))
    }

    fn choice_const(bank: &mut TermBank, name: &str) -> Term {
        let individual = individual(bank);
        let bool_type = bank.signature().type_bank().bool_type();
        let predicate_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![individual.clone(), bool_type]));
        let choice_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![predicate_type, individual]));
        typed_const_with_type(bank, name, &choice_type)
    }

    fn choice_axiom(bank: &mut TermBank, name: &str, p_code: FunCode, x_code: FunCode) -> Clause {
        let predicate = predicate_var(bank, p_code);
        let witness = typed_var(bank, x_code);
        let choice = choice_const(bank, name);
        let choice_applied = apply_many(bank, &choice, std::slice::from_ref(&predicate));
        let negative_atom = apply_many(bank, &predicate, std::slice::from_ref(&witness));
        let positive_atom = apply_many(bank, &predicate, std::slice::from_ref(&choice_applied));
        let true_term = bank.true_term().clone();
        clause_from(vec![
            equation(bank, &negative_atom, &true_term, false),
            equation(bank, &positive_atom, &true_term, true),
        ])
    }

    fn equation(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn predicate_literal(bank: &mut TermBank, atom: &Term) -> Eqn {
        let mut literal = Eqn::create_true_lit(bank).unwrap_or_else(|err| panic!("{err}"));
        literal.set_left_raw(atom.clone());
        literal
    }

    fn clause_from(literals: Vec<Eqn>) -> Clause {
        let mut clause = Clause::alloc(EqnList::from_vec(literals));
        clause.set_weight(clause.standard_weight());
        clause
    }

    fn assert_f64_eq(left: f64, right: f64) {
        assert!((left - right).abs() < f64::EPSILON);
    }

    #[test]
    fn spec_feature_class_discriminants_match_c_enum() {
        assert_eq!(SpecFeatureClass::Unit as i32, 0);
        assert_eq!(SpecFeatureClass::PureEq as i32, 5);
        assert_eq!(SpecFeatureClass::ManyPosGround as i32, 11);
        assert_eq!(SpecFeatureClass::LargeTerms as i32, 20);
        assert_eq!(SpecFeatureClass::AritySumLarge as i32, 27);
        assert_eq!(SpecFeatureClass::Ho as i32, 33);
        assert_eq!(SpecFeatureClass::ManyApplits as i32, 42);
    }

    #[test]
    fn spec_limits_allocation_and_auto_defaults_match_c_values() {
        let allocated = SpecLimits::alloc();
        assert!(allocated.ngu_absolute);
        assert_f64_eq(allocated.ngu_few_limit, 1.0);
        assert_f64_eq(allocated.ngu_many_limit, 3.0);
        assert!(allocated.gpc_absolute);
        assert_f64_eq(allocated.gpc_few_limit, 2.0);
        assert_f64_eq(allocated.gpc_many_limit, 5.0);
        assert_eq!(allocated.ax_some_limit, 1000);
        assert_eq!(allocated.ax_many_limit, 10000);
        assert_eq!(allocated.depth_medium_limit, 0);
        assert_eq!(allocated.depth_deep_limit, 6);
        assert_f64_eq(allocated.perc_form_defs_medium_limit, 0.15);
        assert_f64_eq(allocated.perc_form_defs_large_limit, 0.15);

        let auto = SpecLimits::default_auto();
        assert_eq!(auto.ax_some_limit, 1000);
        assert_eq!(auto.ax_many_limit, 100_000);
        assert_eq!(auto.lit_some_limit, 400);
        assert_eq!(auto.lit_many_limit, 4000);
        assert_eq!(auto.term_medium_limit, 200);
        assert_eq!(auto.term_large_limit, 1500);
        assert_eq!(auto.depth_medium_limit, 4);
        assert_eq!(auto.depth_deep_limit, 7);
        assert_f64_eq(auto.perc_form_defs_large_limit, 0.5);
        assert_f64_eq(auto.perc_app_lits_medium_limit, 0.1);
        assert_f64_eq(auto.perc_app_lits_large_limit, 0.5);
        assert_eq!(create_default_spec_limits(), auto);
        assert_eq!(SPEC_STRING_MEM, 22);
        assert_eq!(DEFAULT_OUTPUT_DESCRIPTOR, "eigEIG");
        assert_eq!(DEFAULT_CLASS_MASK, "aaaaaaaaaaaaa");
    }

    #[test]
    fn spec_type_string_matches_c_encoding_and_masking() {
        let features = SpecFeatureCell {
            axiomtypes: SpecFeatureClass::General,
            goaltypes: SpecFeatureClass::Horn,
            eq_content: SpecFeatureClass::SomeEq,
            ng_unit_content: SpecFeatureClass::ManyPosNonGroundUnits,
            goals_are_ground: false,
            set_clause_size: SpecFeatureClass::SomeAxioms,
            set_literal_size: SpecFeatureClass::ManyLiterals,
            set_termcell_size: SpecFeatureClass::LargeTerms,
            ground_positive_content: SpecFeatureClass::FewPosGround,
            max_fun_ar_class: SpecFeatureClass::Arity2,
            avg_fun_ar_class: SpecFeatureClass::Arity1,
            sum_fun_ar_class: SpecFeatureClass::AritySumLarge,
            max_depth_class: SpecFeatureClass::DepthDeep,
            order_class: SpecFeatureClass::Ho,
            goal_order_class: SpecFeatureClass::So,
            defs_class: SpecFeatureClass::MediumDefs,
            form_defs_class: SpecFeatureClass::ManyFormDefs,
            appvar_lits_class: SpecFeatureClass::FewApplits,
            quantifies_booleans: true,
            has_defined_choice: false,
            ..SpecFeatureCell::default()
        };

        assert_eq!(
            spec_type_string_for_problem(&features, DEFAULT_CLASS_MASK, ProblemType::HigherOrder),
            "HGHSMNSMLF21LDHSSMFBN"
        );
        assert_eq!(
            spec_type_string_for_problem(&features, DEFAULT_CLASS_MASK, ProblemType::FirstOrder),
            "FGHSMNSMLF21LDHSSMFBN"
        );
        assert_eq!(
            spec_type_print_string(&features, &"-".repeat(22)),
            "-".repeat(21)
        );
    }

    #[test]
    fn spec_features_print_string_matches_c_field_order() {
        let features = SpecFeatureCell {
            goals: 1,
            axioms: 2,
            clauses: 3,
            literals: 4,
            term_cells: 5,
            unitgoals: 6,
            unitaxioms: 7,
            horngoals: 8,
            hornaxioms: 9,
            eq_clauses: 10,
            peq_clauses: 11,
            groundunitaxioms: 12,
            groundgoals: 13,
            groundpositiveaxioms: 14,
            positiveaxioms: 15,
            ng_unit_axioms_part: 0.25,
            ground_positive_axioms_part: 0.75,
            max_fun_arity: 16,
            avg_fun_arity: 17,
            sum_fun_arity: 18,
            clause_max_depth: 19,
            clause_avg_depth: 20,
            order: 21,
            num_of_definitions: 22,
            perc_of_form_defs: 0.125,
            perc_of_appvar_lits: 0.5,
            quantifies_booleans: true,
            has_defined_choice: false,
            ..SpecFeatureCell::default()
        };

        assert_eq!(
            spec_features_print_string(&features),
            "(   1,   2,   3,   4,   5,   6,   7,   8,   9,  10,  11,  12,  13,  14,  15, 0.250000, 0.750000,  16,  17,  18,  19,  20,  21,  22, 0.125000, 0.500000, true, false )"
        );
    }

    #[test]
    fn spec_features_parse_matches_c_legacy_shape_and_class_recovery() {
        let mut features = SpecFeatureCell {
            ng_unit_content: SpecFeatureClass::ManyPosNonGroundUnits,
            ..SpecFeatureCell::default()
        };
        let mut scanner = Scanner::from_user_string(
            "(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0.25,0.75,16,17,18,19,20): UHSMG tail",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        spec_features_parse(&mut scanner, &mut features).unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(features.goals, 1);
        assert_eq!(features.axioms, 2);
        assert_eq!(features.clause_avg_depth, 20);
        assert_eq!(features.axiomtypes, SpecFeatureClass::Unit);
        assert_eq!(features.goaltypes, SpecFeatureClass::Horn);
        assert_eq!(features.eq_content, SpecFeatureClass::SomeEq);
        assert_eq!(
            features.ng_unit_content,
            SpecFeatureClass::ManyPosNonGroundUnits
        );
        assert!(features.goals_are_ground);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn spec_features_parse_rejects_general_goal_class_like_c() {
        let mut features = SpecFeatureCell::default();
        let mut scanner = Scanner::from_user_string(
            "(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,0.25,0.75,16,17,18,19,20): UGSFG",
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        let error = spec_features_parse(&mut scanner, &mut features).unwrap_err();

        assert!(error.to_string().contains("Insufficient class information"));
    }

    #[test]
    fn spec_limits_print_string_matches_c_shape() {
        assert_eq!(
            spec_limits_print_string(&SpecLimits::alloc()),
            "[ 1 | 1 | 3 | 1 | 2 | 5 | 1000 | 10000 | 400 | 4000 | 200 | 1500 | 4 | 29 | 0 | 6 |  100 | 1000 | 0 | 2 | 1225 | 4000 | 8 | 110 | 360 | 400 | 2 | 3 | 2 | 8 | 8 | 64 |  0.15 | 0.15 | 0.1 | 0.5 ]\n"
        );
    }

    #[test]
    fn selective_clause_set_line_helpers_preserve_c_filters_and_order() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "selective_a");
        let b = typed_const(&mut bank, "selective_b");
        let c = typed_const(&mut bank, "selective_c");
        let mut positive_unit = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        positive_unit.set_ident(11);
        let mut negative_unit = clause_from(vec![equation(&mut bank, &b, &a, false)]);
        negative_unit.set_ident(12);
        let mut non_unit = clause_from(vec![
            equation(&mut bank, &a, &b, true),
            equation(&mut bank, &b, &c, false),
        ]);
        non_unit.set_ident(13);
        let set = ClauseSet::from_clauses([positive_unit, negative_unit, non_unit]);
        let render = |clause: &Clause| format!("c_{}", clause.ident());

        assert_eq!(
            clause_set_print_pos_units_string(&bank, &set, false, render),
            "c_11\n"
        );
        assert_eq!(
            clause_set_print_neg_units_string(&bank, &set, false, render),
            "c_12\n"
        );
        assert_eq!(
            clause_set_print_non_units_string(&bank, &set, false, render),
            "c_13\n"
        );
    }

    #[test]
    fn selective_clause_set_default_line_helpers_render_lop_clauses() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "default_selective_a");
        let b = typed_const(&mut bank, "default_selective_b");
        let c = typed_const(&mut bank, "default_selective_c");
        let positive_unit = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        let negative_unit = clause_from(vec![equation(&mut bank, &b, &a, false)]);
        let non_unit = clause_from(vec![
            equation(&mut bank, &a, &b, true),
            equation(&mut bank, &b, &c, false),
        ]);
        let set = ClauseSet::from_clauses([positive_unit, negative_unit, non_unit]);

        assert_eq!(
            clause_set_print_pos_units_default_string(&bank, &set, false),
            "default_selective_a=default_selective_b <- .\n"
        );
        assert_eq!(
            clause_set_print_neg_units_default_string(&bank, &set, false),
            " <- default_selective_b=default_selective_a.\n"
        );
        assert_eq!(
            clause_set_print_non_units_default_string(&bank, &set, false),
            "default_selective_a=default_selective_b <- default_selective_b=default_selective_c.\n"
        );
    }

    #[test]
    fn selective_clause_set_format_line_helpers_dispatch_clause_output() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "format_selective_a");
        let b = typed_const(&mut bank, "format_selective_b");
        let c = typed_const(&mut bank, "format_selective_c");
        let positive_unit = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        let negative_unit = clause_from(vec![equation(&mut bank, &b, &a, false)]);
        let non_unit = clause_from(vec![
            equation(&mut bank, &a, &b, true),
            equation(&mut bank, &b, &c, false),
        ]);
        let set = ClauseSet::from_clauses([positive_unit, negative_unit, non_unit]);

        let positive_tptp = clause_set_print_pos_units_format_string(
            &bank,
            &set,
            false,
            IoFormat::Tptp,
            ProblemType::FirstOrder,
            EqnPrintOptions::tptp(),
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert!(positive_tptp.starts_with("input_clause("));
        assert!(positive_tptp.contains("++equal(format_selective_a, format_selective_b)"));
        assert!(!positive_tptp.contains("<-"));

        let negative_tptp = clause_set_print_neg_units_format_string(
            &bank,
            &set,
            false,
            IoFormat::Tptp,
            ProblemType::FirstOrder,
            EqnPrintOptions::tptp(),
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert!(negative_tptp.starts_with("input_clause("));
        assert!(negative_tptp.contains("--equal(format_selective_b, format_selective_a)"));
        assert!(!negative_tptp.contains("<-"));

        let non_unit_tstp = clause_set_print_non_units_format_string(
            &bank,
            &set,
            true,
            IoFormat::Tstp,
            ProblemType::FirstOrder,
            EqnPrintOptions::lop(),
        )
        .unwrap_or_else(|err| panic!("{err}"));
        assert!(non_unit_tstp.starts_with("cnf(") || non_unit_tstp.starts_with("tcf("));
        assert!(non_unit_tstp.contains("format_selective_a"));
        assert!(non_unit_tstp.contains(" % info("));
        assert!(!non_unit_tstp.contains("<-"));
    }

    #[test]
    fn proof_state_print_selective_uses_explicit_higher_order_problem_type() {
        let mut state = proof_state_alloc(FP_IGNORE_PROPS).unwrap_or_else(|err| panic!("{err}"));
        let mut clause = Clause::empty();
        clause.set_ident(19);
        state.processed_non_units_mut().insert(clause);

        let printed = proof_state_print_selective_string(
            &state,
            "g",
            false,
            IoFormat::Tstp,
            ProblemType::HigherOrder,
            EqnPrintOptions::lop(),
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(
            printed.contains("% Processed non-unit clauses:\nthf("),
            "{printed}"
        );
        assert!(!printed.contains("\ncnf("), "{printed}");
    }

    #[test]
    fn spec_features_basic_eval_sets_shape_classes_and_ratios() {
        let mut features = SpecFeatureCell {
            clauses: 5,
            goals: 2,
            axioms: 3,
            unitgoals: 2,
            unitaxioms: 2,
            horngoals: 2,
            hornaxioms: 3,
            eq_clauses: 3,
            peq_clauses: 5,
            groundgoals: 2,
            groundunitaxioms: 1,
            positiveaxioms: 4,
            groundpositiveaxioms: 3,
            max_fun_arity: 3,
            avg_fun_arity: 1,
            ..SpecFeatureCell::default()
        };

        spec_features_add_basic_eval(&mut features);

        assert!(features.goals_are_ground);
        assert_eq!(features.axiomtypes, SpecFeatureClass::Horn);
        assert_eq!(features.goaltypes, SpecFeatureClass::Unit);
        assert_eq!(features.eq_content, SpecFeatureClass::PureEq);
        assert_eq!(features.max_fun_ar_class, SpecFeatureClass::Arity3Plus);
        assert_eq!(features.avg_fun_ar_class, SpecFeatureClass::Arity1);
        assert!((features.ng_unit_axioms_part - 0.5).abs() < f64::EPSILON);
        assert!((features.ground_positive_axioms_part - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn spec_features_add_eval_matches_c_thresholds() {
        let limits = SpecLimits::default_auto();
        let mut features = SpecFeatureCell {
            clauses: 999,
            goals: 2,
            groundgoals: 2,
            literals: 400,
            term_cells: 1500,
            unitaxioms: 5,
            groundunitaxioms: 1,
            positiveaxioms: 10,
            groundpositiveaxioms: 6,
            max_fun_arity: 3,
            avg_fun_arity: 2,
            sum_fun_arity: 4,
            clause_max_depth: 7,
            order: 3,
            goal_order: 2,
            num_of_definitions: 64,
            perc_of_form_defs: 0.5,
            perc_of_appvar_lits: 0.5,
            ..SpecFeatureCell::default()
        };

        spec_features_add_eval(&mut features, &limits);

        assert!(features.goals_are_ground);
        assert_eq!(
            features.ng_unit_content,
            SpecFeatureClass::ManyPosNonGroundUnits
        );
        assert_eq!(
            features.ground_positive_content,
            SpecFeatureClass::ManyPosGround
        );
        assert_eq!(features.set_clause_size, SpecFeatureClass::FewAxioms);
        assert_eq!(features.set_literal_size, SpecFeatureClass::SomeLiterals);
        assert_eq!(features.set_termcell_size, SpecFeatureClass::LargeTerms);
        assert_eq!(features.max_fun_ar_class, SpecFeatureClass::Arity3Plus);
        assert_eq!(features.avg_fun_ar_class, SpecFeatureClass::Arity2);
        assert!((features.ng_unit_axioms_part - 0.8).abs() < f64::EPSILON);
        assert!((features.ground_positive_axioms_part - 0.6).abs() < f64::EPSILON);
        assert_eq!(features.sum_fun_ar_class, SpecFeatureClass::AritySumMedium);
        assert_eq!(features.max_depth_class, SpecFeatureClass::DepthDeep);
        assert_eq!(features.order_class, SpecFeatureClass::Ho);
        assert_eq!(features.goal_order_class, SpecFeatureClass::So);
        assert_eq!(features.defs_class, SpecFeatureClass::ManyDefs);
        assert_eq!(features.form_defs_class, SpecFeatureClass::ManyFormDefs);
        assert_eq!(features.appvar_lits_class, SpecFeatureClass::ManyApplits);
    }

    #[test]
    fn spec_features_relative_limits_use_existing_ratios_before_recomputing() {
        let limits = SpecLimits {
            ngu_absolute: false,
            ngu_few_limit: 0.25,
            ngu_many_limit: 0.75,
            gpc_absolute: false,
            gpc_few_limit: 0.25,
            gpc_many_limit: 0.75,
            ..SpecLimits::alloc()
        };
        let mut features = SpecFeatureCell {
            ng_unit_axioms_part: 0.5,
            ground_positive_axioms_part: 0.9,
            unitaxioms: 4,
            groundunitaxioms: 2,
            positiveaxioms: 10,
            groundpositiveaxioms: 4,
            ..SpecFeatureCell::default()
        };

        spec_features_add_eval(&mut features, &limits);

        assert_eq!(
            features.ng_unit_content,
            SpecFeatureClass::SomePosNonGroundUnits
        );
        assert_eq!(
            features.ground_positive_content,
            SpecFeatureClass::ManyPosGround
        );
        assert!((features.ng_unit_axioms_part - 0.5).abs() < f64::EPSILON);
        assert!((features.ground_positive_axioms_part - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn polarity_unit_horn_ground_and_range_counts_match_clause_macros() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let positive_ground_unit = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        let negative_var_unit = clause_from(vec![equation(&mut bank, &fx, &a, false)]);
        let positive_two_literal = clause_from(vec![
            equation(&mut bank, &a, &b, true),
            equation(&mut bank, &b, &c, true),
        ]);
        let mixed_range_restricted = clause_from(vec![
            equation(&mut bank, &fx, &a, true),
            equation(&mut bank, &x, &b, false),
        ]);
        let set = ClauseSet::from_clauses([
            positive_ground_unit,
            negative_var_unit,
            positive_two_literal,
            mixed_range_restricted,
        ]);

        assert_eq!(clause_set_count_goals(&set), 1);
        assert_eq!(clause_set_count_axioms(&set), 3);
        assert_eq!(clause_set_count_unit(&set), 2);
        assert_eq!(clause_set_count_unit_goals(&set), 1);
        assert_eq!(clause_set_count_unit_axioms(&set), 1);
        assert!(!clause_set_is_unit_set(&set));
        assert!(!clause_set_axioms_are_unit(&set));
        assert!(clause_set_goals_are_unit(&set));
        assert_eq!(clause_set_count_horn(&set), 3);
        assert_eq!(clause_set_count_horn_goals(&set), 1);
        assert_eq!(clause_set_count_horn_axioms(&set), 2);
        assert!(!clause_set_is_horn_set(&set));
        assert!(!clause_set_axioms_are_horn(&set));
        assert!(clause_set_goals_are_horn(&set));
        assert_eq!(clause_set_count_ground(&set), 2);
        assert_eq!(clause_set_count_ground_goals(&set), 0);
        assert!(!clause_set_goals_are_ground(&set));
        assert!(!clause_set_is_ground(&set));
        assert_eq!(clause_set_count_positive_axioms(&set), 2);
        assert_eq!(clause_set_count_ground_positive_axioms(&set), 2);
        assert_eq!(clause_set_count_pos_units(&set), 1);
        assert_eq!(clause_set_count_ground_unit_axioms(&set), 1);
        assert_eq!(clause_set_count_non_ground_unit_axioms(&set), 0);
        assert!(clause_set_non_ground_axiom_part(&set).abs() < f64::EPSILON);
        assert_eq!(clause_set_count_range_restricted(&set), 3);
    }

    #[test]
    fn equational_counts_distinguish_clause_predicates_from_literal_property_bits() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let p = typed_predicate_binary(&mut bank, "p", &a, &b);
        let equation_clause = clause_from(vec![equation(&mut bank, &a, &b, true)]);
        let predicate_clause = clause_from(vec![predicate_literal(&mut bank, &p)]);
        let mixed_clause = clause_from(vec![
            equation(&mut bank, &b, &a, false),
            predicate_literal(&mut bank, &p),
        ]);
        let set = ClauseSet::from_clauses([equation_clause, predicate_clause, mixed_clause]);

        assert_eq!(clause_set_count_equational(&bank, &set), 2);
        assert!(clause_set_is_equational(&bank, &set));
        assert!(!clause_set_is_equational_set(&bank, &set));
        assert_eq!(clause_set_count_pure_equational(&bank, &set), 1);
        assert!(!clause_set_is_pure_equational_set(&bank, &set));
        assert_eq!(clause_set_count_eqn_literals(&set), 2);
    }

    #[test]
    fn arity_information_uses_seen_non_special_symbols() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let fa = typed_unary(&mut bank, "f", &a);
        let hab = typed_binary(&mut bank, "h", &a, &b);
        let p = typed_predicate_binary(&mut bank, "p", &hab, &fa);
        let set = ClauseSet::from_clauses([clause_from(vec![
            equation(&mut bank, &hab, &fa, true),
            predicate_literal(&mut bank, &p),
        ])]);

        let info = clause_set_collect_arity_information(&set, bank.signature());

        assert_eq!(info.fun_const_count, 2);
        assert_eq!(info.non_const_funs, 2);
        assert_eq!(info.max_fun_arity, 2);
        assert_eq!(info.sum_fun_arity, 3);
        assert_eq!(info.avg_fun_arity, 1);
        assert_eq!(info.non_const_preds, 1);
        assert_eq!(info.max_pred_arity, 2);
        assert_eq!(info.sum_pred_arity, 2);
        assert_eq!(info.avg_pred_arity, 2);
    }

    #[test]
    fn ho_feature_extraction_matches_c_symbol_variable_and_literal_scans() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let bool_type = bank.signature().type_bank().bool_type();
        let pred_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![individual.clone(), bool_type]));
        let higher_order_type =
            bank.signature_mut()
                .type_bank_mut()
                .insert_type_shared(alloc_arrow_type(vec![
                    pred_type.clone(),
                    individual.clone(),
                ]));
        let ho_symbol = bank
            .signature_mut()
            .insert_id("ho_feature_symbol", 1, false);
        bank.signature_mut()
            .declare_final_type(ho_symbol, higher_order_type)
            .unwrap_or_else(|err| panic!("{err}"));

        let x = bank.vars().var_assert_alloc(-2, &individual);
        let p = bank.vars().var_assert_alloc(-4, &pred_type);
        let app = bank.term_apply_arg(&p, &x);
        let mut app_clause = clause_from(vec![predicate_literal(&mut bank, &app)]);
        app_clause.set_ident(1001);

        let marked = typed_const(&mut bank, "lambda_marked_feature_term");
        marked.set_prop(TP_HAS_LAMBDA_SUBTERM);
        let a = typed_const(&mut bank, "ho_feature_a");
        let mut non_fo_clause = clause_from(vec![equation(&mut bank, &marked, &a, true)]);
        non_fo_clause.set_ident(1002);

        let set = ClauseSet::from_clauses([app_clause, non_fo_clause]);

        let features =
            clause_set_compute_ho_features(&set, bank.signature(), |clause| clause.ident() == 1002);

        assert_eq!(
            features,
            ClauseSetHoFeatures {
                has_ho_features: true,
                order: 2,
                quantifies_booleans: true,
                has_defined_choice: true,
                perc_app_var_lits: 0.5,
            }
        );
        assert!(
            !clause_set_compute_ho_features_without_choice(&set, bank.signature())
                .has_defined_choice
        );
    }

    #[test]
    fn ho_feature_extraction_can_use_c_null_map_choice_recognition() {
        let mut bank = term_bank();
        let choice_clause = choice_axiom(&mut bank, "feature_choice", -90, -92);
        let set = ClauseSet::from_clauses([choice_clause.clone()]);
        let original_left = choice_clause.literals().as_slice()[0].left().clone();

        let features =
            clause_set_compute_ho_features_with_choice_recognition(&set, &mut bank).unwrap();

        assert!(features.has_defined_choice);
        assert!(
            !clause_set_compute_ho_features_without_choice(&set, bank.signature())
                .has_defined_choice
        );
        assert_eq!(
            set.iter().next().unwrap().literals().as_slice()[0].left(),
            &original_left
        );

        let mut spec_features = SpecFeatureCell::default();
        spec_features_compute_with_choice_recognition(
            &mut spec_features,
            &set,
            None,
            None,
            &mut bank,
        )
        .unwrap();
        assert!(spec_features.has_defined_choice);
    }

    #[test]
    fn ho_feature_extraction_handles_empty_sets_like_c() {
        let bank = term_bank();

        assert_eq!(
            clause_set_compute_ho_features_without_choice(&ClauseSet::new(), bank.signature()),
            ClauseSetHoFeatures::default()
        );
    }

    #[test]
    fn spec_features_compute_clause_set_fills_c_clause_fields_and_resets_order() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let bool_type = bank.signature().type_bank().bool_type();
        let pred_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![individual.clone(), bool_type]));
        let a = typed_const(&mut bank, "spec_compute_a");
        let x = bank.vars().var_assert_alloc(-2, &individual);
        let p = bank.vars().var_assert_alloc(-4, &pred_type);
        let app = bank.term_apply_arg(&p, &x);
        let mut app_clause = clause_from(vec![predicate_literal(&mut bank, &app)]);
        app_clause.set_ident(7001);

        let fx = typed_unary(&mut bank, "spec_compute_f", &x);
        fx.set_prop(TP_HAS_LAMBDA_SUBTERM);
        let goal_clause = clause_from(vec![equation(&mut bank, &fx, &a, false)]);
        let set = ClauseSet::from_clauses([app_clause, goal_clause]);

        let mut expected_depthmax = 0;
        let mut expected_depthsum = 0;
        let mut expected_depthcount = 0;
        clause_set_tptp_depth_info_add(
            &bank,
            &set,
            &mut expected_depthmax,
            &mut expected_depthsum,
            &mut expected_depthcount,
        );

        let mut features = SpecFeatureCell {
            perc_of_form_defs: 0.875,
            ..SpecFeatureCell::default()
        };
        spec_features_compute_clause_set(&mut features, &set, &bank, |clause| {
            clause.ident() == 7001
        });

        assert_eq!(features.clauses, 2);
        assert_eq!(features.goals, 1);
        assert_eq!(features.axioms, 1);
        assert_eq!(features.literals, 2);
        assert_eq!(features.term_cells, clause_set_term_cells(&bank, &set));
        assert_eq!(features.clause_max_depth, expected_depthmax);
        assert_eq!(
            features.clause_avg_depth,
            expected_depthsum / expected_depthcount
        );
        assert_eq!(features.unit, 2);
        assert_eq!(features.unitgoals, 1);
        assert_eq!(features.unitaxioms, 1);
        assert_eq!(features.horn, 2);
        assert_eq!(features.horngoals, 1);
        assert_eq!(features.hornaxioms, 1);
        assert_eq!(features.eq_clauses, 1);
        assert_eq!(features.peq_clauses, 1);
        assert_eq!(features.groundunitaxioms, 0);
        assert_eq!(features.groundgoals, 0);
        assert_eq!(features.positiveaxioms, 1);
        assert_eq!(features.groundpositiveaxioms, 0);
        assert_eq!(features.max_fun_arity, 1);
        assert_eq!(features.avg_fun_arity, 1);
        assert_eq!(features.sum_fun_arity, 1);
        assert_eq!(features.max_pred_arity, 0);
        assert_eq!(features.avg_pred_arity, 0);
        assert_eq!(features.sum_pred_arity, 0);
        assert_eq!(features.fun_const_count, 1);
        assert_eq!(features.fun_nonconst_count, 1);
        assert_eq!(features.pred_nonconst_count, 0);
        assert!(!features.goals_are_ground);
        assert_eq!(features.axiomtypes, SpecFeatureClass::Unit);
        assert_eq!(features.goaltypes, SpecFeatureClass::Unit);
        assert_eq!(features.eq_content, SpecFeatureClass::SomeEq);
        assert_eq!(features.max_fun_ar_class, SpecFeatureClass::Arity1);
        assert_eq!(features.avg_fun_ar_class, SpecFeatureClass::Arity1);
        assert!((features.ng_unit_axioms_part - 1.0).abs() < f64::EPSILON);
        assert!(features.ground_positive_axioms_part.abs() < f64::EPSILON);
        assert!(features.has_ho_features);
        assert!(features.quantifies_booleans);
        assert!(features.has_defined_choice);
        assert_eq!(features.order, 1);
        assert_eq!(features.goal_order, 1);
        assert_eq!(features.num_of_definitions, -1);
        assert!((features.perc_of_appvar_lits - 0.5).abs() < f64::EPSILON);
        assert!((features.perc_of_form_defs - 0.875).abs() < f64::EPSILON);
    }

    #[test]
    fn spec_features_compute_scans_formula_archive_and_active_orders_after_clause_reset() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let unary_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![
                individual.clone(),
                individual.clone(),
            ]));
        let higher_type = bank
            .signature_mut()
            .type_bank_mut()
            .insert_type_shared(alloc_arrow_type(vec![unary_type.clone(), individual]));
        let archived_axiom = WrappedFormula::wt_formula_alloc(typed_const_with_type(
            &mut bank,
            "arch_ho",
            &unary_type,
        ));
        let mut active_goal = WrappedFormula::wt_formula_alloc(typed_const_with_type(
            &mut bank,
            "active_ho",
            &higher_type,
        ));
        active_goal.set_tptp_type(CP_TYPE_HYPOTHESIS);
        let mut farch = FormulaSet::new();
        farch.insert(archived_axiom);
        let mut fset = FormulaSet::new();
        fset.insert(active_goal);
        let empty_clauses = ClauseSet::new();
        let mut features = SpecFeatureCell {
            order: 99,
            goal_order: 99,
            num_of_definitions: 42,
            perc_of_form_defs: 0.875,
            ..SpecFeatureCell::default()
        };

        spec_features_compute(
            &mut features,
            &empty_clauses,
            Some(&fset),
            Some(&farch),
            &bank,
            |_| false,
        );

        assert_eq!(features.clauses, 0);
        assert_eq!(features.order, 3);
        assert_eq!(features.goal_order, 3);
        assert_eq!(features.num_of_definitions, -1);
        assert!((features.perc_of_form_defs - 0.875).abs() < f64::EPSILON);

        let mut no_formula_features = SpecFeatureCell::default();
        spec_features_compute(
            &mut no_formula_features,
            &empty_clauses,
            None,
            None,
            &bank,
            |_| false,
        );

        assert_eq!(no_formula_features.order, 1);
        assert_eq!(no_formula_features.goal_order, 1);
    }

    #[test]
    fn aggregate_literal_variable_depth_and_size_features_sum_clause_helpers() {
        let mut bank = term_bank();
        let a = typed_const(&mut bank, "a");
        let b = typed_const(&mut bank, "b");
        let c = typed_const(&mut bank, "c");
        let x = typed_var(&bank, -2);
        let fx = typed_unary(&mut bank, "f", &x);
        let mut first_lit = equation(&mut bank, &fx, &a, true);
        first_lit.set_prop(EP_IS_MAXIMAL | EP_IS_ORIENTED);
        let mut second_lit = equation(&mut bank, &b, &c, false);
        second_lit.set_prop(EP_IS_MAXIMAL);
        let first = clause_from(vec![first_lit, second_lit]);
        let second = clause_from(vec![equation(&mut bank, &x, &b, false)]);
        let set = ClauseSet::from_clauses([first, second]);
        let expected_max_terms: i64 = set.iter().map(clause_count_maximal_terms).sum();
        let expected_max_literals: i64 = set.iter().map(clause_count_maximal_literals).sum();
        let expected_unorientable: i64 = set.iter().map(clause_count_unorientable_literals).sum();
        let expected_variables: i64 = set.iter().map(clause_count_variable_set).sum();
        let expected_singletons: i64 = set.iter().map(clause_count_singleton_set).sum();
        let expected_term_cells: i64 = set
            .iter()
            .map(|clause| {
                super::clause_weight_to_i64(
                    clause.literal_weight(&bank, 1.0, 1.0, 1.0, 1, 1, 1.0, false),
                )
            })
            .sum();
        let mut expected_depthmax = 0;
        let mut expected_depthsum = 0;
        let mut expected_count = 0;
        for clause in set.iter() {
            clause_tptp_depth_info_add(
                &bank,
                clause,
                &mut expected_depthmax,
                &mut expected_depthsum,
                &mut expected_count,
            );
        }

        assert_eq!(clause_set_count_maximal_terms(&set), expected_max_terms);
        assert_eq!(
            clause_set_count_maximal_literals(&set),
            expected_max_literals
        );
        assert_eq!(
            clause_set_count_unorientable_literals(&set),
            expected_unorientable
        );
        assert_eq!(clause_set_count_variables(&set), expected_variables);
        assert_eq!(clause_set_count_singletons(&set), expected_singletons);
        assert_eq!(clause_set_term_cells(&bank, &set), expected_term_cells);
        assert_eq!(
            clause_set_max_standard_weight(&set),
            set.find_max_standard_weight()
                .map_or(-1, Clause::standard_weight)
        );
        assert_eq!(clause_set_max_literal_number(&set), 2);

        let mut depthmax = 0;
        let mut depthsum = 0;
        let mut count = 0;
        assert_eq!(
            clause_set_tptp_depth_info_add(&bank, &set, &mut depthmax, &mut depthsum, &mut count,),
            expected_depthmax
        );
        assert_eq!(depthsum, expected_depthsum);
        assert_eq!(count, expected_count);
    }
}
