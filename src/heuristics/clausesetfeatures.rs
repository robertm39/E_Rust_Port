use crate::clauses::clause::Clause;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn_props::EP_IS_EQU_LITERAL;
use crate::heuristics::clausefeatures::{
    clause_count_maximal_literals, clause_count_maximal_terms, clause_count_singleton_set,
    clause_count_unorientable_literals, clause_count_variable_set, clause_tptp_depth_info_add,
};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;

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

fn fcode_index(f_code: FunCode) -> usize {
    usize::try_from(f_code).unwrap_or_else(|_| panic!("f-code must fit feature-array index"))
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[allow(clippy::cast_possible_truncation)]
fn clause_weight_to_i64(weight: f64) -> i64 {
    weight as i64
}

#[cfg(test)]
mod tests {
    use super::{
        clause_set_axioms_are_horn, clause_set_axioms_are_unit,
        clause_set_collect_arity_information, clause_set_count_axioms,
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
        clause_set_non_ground_axiom_part, clause_set_term_cells, clause_set_tptp_depth_info_add,
        spec_features_add_basic_eval, spec_features_add_eval, SpecFeatureCell, SpecFeatureClass,
        SpecLimits,
    };
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqn_props::{EP_IS_MAXIMAL, EP_IS_ORIENTED};
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::clausefeatures::{
        clause_count_maximal_literals, clause_count_maximal_terms, clause_count_singleton_set,
        clause_count_unorientable_literals, clause_count_variable_set, clause_tptp_depth_info_add,
    };
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::Signature;
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
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
