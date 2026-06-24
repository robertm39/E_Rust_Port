use crate::basics::partial_orderings::HoOrderKind;
use crate::heuristics::to_params::{
    ho_order_kind_name, LiteralCmp, OrderParmsCell, TOPrecGenMethod, TOWeightGenMethod,
    TermOrdering, DEFAULT_DB_WEIGHT, DEFAULT_LAMBDA_WEIGHT, W_CONST_NO_SPECIAL_WEIGHT,
    W_CONST_NO_WEIGHT,
};

pub const KBO_BONUS: i64 = 1;
pub const MAX_TERM_PENALTY: i64 = 2;
pub const MAX_LITERAL_PENALTY: i64 = 1;
pub const UNORIENT_LITERAL_PENALTY: i64 = 1;
pub const MAX_CONST_WEIGHT: i64 = 2;
pub const DEFAULT_COMCHAR_RAW: &str = "%";

pub fn init_oparms(oparms: &mut OrderParmsCell) {
    oparms.ordertype = TermOrdering::Kbo6;
    oparms.to_const_weight = W_CONST_NO_SPECIAL_WEIGHT;
    oparms.to_weight_gen = TOWeightGenMethod::SelectMaximal;
    oparms.to_prec_gen = TOPrecGenMethod::UnaryFirst;
    oparms.conj_only_mod = 0;
    oparms.conj_axiom_mod = 0;
    oparms.axiom_only_mod = 0;
    oparms.lit_cmp = i64::from(LiteralCmp::Normal.c_value());
    oparms.ho_order_kind = HoOrderKind::LfhoOrder;
    oparms.db_w = DEFAULT_DB_WEIGHT;
    oparms.lam_w = DEFAULT_LAMBDA_WEIGHT;
    oparms.force_kbo_var_weight = false;
}

#[must_use]
pub fn print_oparms_string(oparms: &OrderParmsCell, output_level: i64) -> String {
    if output_level == 0 {
        return String::new();
    }

    let mut result = format!(
        concat!(
            "{comment} Auto-mode selected ordering type {ordertype}\n",
            "{comment} Auto-mode selected ordering precedence scheme <{prec}>\n"
        ),
        comment = DEFAULT_COMCHAR_RAW,
        ordertype = oparms.ordertype.name(),
        prec = oparms.to_prec_gen.name().unwrap_or("")
    );

    if matches!(oparms.ordertype, TermOrdering::Kbo | TermOrdering::Kbo6) {
        result.push_str(DEFAULT_COMCHAR_RAW);
        result.push_str(" Auto-mode selected weight ordering scheme <");
        result.push_str(oparms.to_weight_gen.name().unwrap_or(""));
        result.push_str(">\n");
    }
    result.push_str(DEFAULT_COMCHAR_RAW);
    result.push('\n');
    result
}

/// Advances the implicit C order-type cycle.
///
/// # Panics
///
/// Panics if `ordering.ordertype` is not one of the C states handled by
/// `OrderNextType`: `NoOrdering`, `KBO`, or `LPO`.
pub fn order_next_type(ordering: &mut OrderParmsCell) -> bool {
    match ordering.ordertype {
        TermOrdering::NoOrdering => {
            ordering.ordertype = TermOrdering::Kbo;
            true
        }
        TermOrdering::Kbo => {
            ordering.ordertype = TermOrdering::Lpo;
            true
        }
        TermOrdering::Lpo => {
            ordering.ordertype = TermOrdering::NoOrdering;
            false
        }
        _ => panic!("Unexpected ordertype!"),
    }
}

/// Advances the C weight-generation enum by raw discriminant value.
///
/// # Panics
///
/// Panics if `ordering.to_weight_gen` is above `WMaxMethod`; this mirrors the
/// C assertion before incrementing.
pub fn order_next_weight_gen(ordering: &mut OrderParmsCell) -> bool {
    assert!(ordering.to_weight_gen.c_value() <= TOWeightGenMethod::ConstantWeight.c_value());

    if ordering.to_weight_gen == TOWeightGenMethod::ConstantWeight {
        ordering.to_weight_gen = TOWeightGenMethod::NoMethod;
        return false;
    }
    ordering.to_weight_gen = TOWeightGenMethod::from_c_value(ordering.to_weight_gen.c_value() + 1)
        .unwrap_or_else(|| panic!("weight-generation enum successor must exist"));
    true
}

/// Advances the C precedence-generation enum by raw discriminant value.
///
/// # Panics
///
/// Panics if `ordering.to_prec_gen` is above `PMaxMethod`; this mirrors the C
/// assertion before incrementing.
pub fn order_next_prec_gen(ordering: &mut OrderParmsCell) -> bool {
    assert!(ordering.to_prec_gen.c_value() <= TOPrecGenMethod::OrientAxioms.c_value());

    if ordering.to_prec_gen == TOPrecGenMethod::OrientAxioms {
        ordering.to_prec_gen = TOPrecGenMethod::NoMethod;
        return false;
    }
    ordering.to_prec_gen = TOPrecGenMethod::from_c_value(ordering.to_prec_gen.c_value() + 1)
        .unwrap_or_else(|| panic!("precedence-generation enum successor must exist"));
    true
}

/// Advances the special constant-weight cycle used during ordering search.
///
/// # Panics
///
/// Panics if `ordering.to_const_weight` is not `WConstNoSpecialWeight`,
/// `WConstNoWeight`, or a positive integer, matching the C assertion.
pub fn order_next_const_weight(ordering: &mut OrderParmsCell) -> bool {
    assert!(
        ordering.to_const_weight == W_CONST_NO_SPECIAL_WEIGHT
            || ordering.to_const_weight == W_CONST_NO_WEIGHT
            || ordering.to_const_weight > 0
    );

    if ordering.to_const_weight == W_CONST_NO_SPECIAL_WEIGHT {
        ordering.to_const_weight = W_CONST_NO_WEIGHT;
        return false;
    }
    if ordering.to_const_weight == MAX_CONST_WEIGHT {
        ordering.to_const_weight = W_CONST_NO_SPECIAL_WEIGHT;
        return true;
    }
    ordering.to_const_weight += 1;
    true
}

/// Advances the combined C ordering-parameter search state.
///
/// # Panics
///
/// Panics if one of the delegated `OrderNext*` helpers receives a state that
/// would fail the corresponding C assertion.
pub fn order_next_ordering(ordering: &mut OrderParmsCell, mask: &OrderParmsCell) -> bool {
    if ordering.ordertype == TermOrdering::Kbo {
        if mask.to_const_weight == W_CONST_NO_WEIGHT {
            if order_next_const_weight(ordering) {
                return true;
            }
            order_next_const_weight(ordering);
        }
        if mask.to_prec_gen == TOPrecGenMethod::NoMethod {
            if order_next_prec_gen(ordering) {
                return true;
            }
            order_next_prec_gen(ordering);
        }
    }
    if mask.to_weight_gen == TOWeightGenMethod::NoMethod {
        if order_next_weight_gen(ordering) {
            return true;
        }
        order_next_weight_gen(ordering);
    }
    if mask.ordertype == TermOrdering::NoOrdering {
        if order_next_type(ordering) {
            return true;
        }
        order_next_type(ordering);
    }
    false
}

#[must_use]
pub fn auto_ordering_analysis_string(label: &str) -> String {
    format!("\n{DEFAULT_COMCHAR_RAW} {label}-Ordering is analysing problem.\n")
}

#[must_use]
pub fn describe_auto_ordering(oparms: &OrderParmsCell) -> String {
    format!(
        "({}, {}, {}, {}, {})",
        oparms.ordertype.name(),
        oparms.to_prec_gen.name().unwrap_or(""),
        oparms.to_weight_gen.name().unwrap_or(""),
        oparms.to_const_weight,
        ho_order_kind_name(oparms.ho_order_kind)
    )
}

#[cfg(test)]
mod tests {
    use super::{
        auto_ordering_analysis_string, describe_auto_ordering, init_oparms,
        order_next_const_weight, order_next_ordering, order_next_prec_gen, order_next_type,
        order_next_weight_gen, print_oparms_string, KBO_BONUS, MAX_CONST_WEIGHT,
        MAX_LITERAL_PENALTY, MAX_TERM_PENALTY, UNORIENT_LITERAL_PENALTY,
    };
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::heuristics::to_params::{
        LiteralCmp, OrderParmsCell, TOPrecGenMethod, TOWeightGenMethod, TermOrdering,
        DEFAULT_DB_WEIGHT, DEFAULT_LAMBDA_WEIGHT, W_CONST_NO_SPECIAL_WEIGHT, W_CONST_NO_WEIGHT,
    };

    #[test]
    fn evaluation_constants_match_c_defines() {
        assert_eq!(KBO_BONUS, 1);
        assert_eq!(MAX_TERM_PENALTY, 2);
        assert_eq!(MAX_LITERAL_PENALTY, 1);
        assert_eq!(UNORIENT_LITERAL_PENALTY, 1);
        assert_eq!(MAX_CONST_WEIGHT, 2);
    }

    #[test]
    fn init_oparms_sets_c_fields_and_leaves_unmentioned_fields_untouched() {
        let mut params = OrderParmsCell {
            rewrite_strong_rhs_inst: true,
            to_pre_prec: Some("prec".to_owned()),
            to_pre_weights: Some("weights".to_owned()),
            skolem_mod: 77,
            defpred_mod: 88,
            to_defs_min: true,
            ..OrderParmsCell::default()
        };

        init_oparms(&mut params);

        assert_eq!(params.ordertype, TermOrdering::Kbo6);
        assert_eq!(params.to_const_weight, W_CONST_NO_SPECIAL_WEIGHT);
        assert_eq!(params.to_weight_gen, TOWeightGenMethod::SelectMaximal);
        assert_eq!(params.to_prec_gen, TOPrecGenMethod::UnaryFirst);
        assert_eq!(params.conj_only_mod, 0);
        assert_eq!(params.conj_axiom_mod, 0);
        assert_eq!(params.axiom_only_mod, 0);
        assert_eq!(params.lit_cmp, i64::from(LiteralCmp::Normal.c_value()));
        assert_eq!(params.ho_order_kind, HoOrderKind::LfhoOrder);
        assert_eq!(params.db_w, DEFAULT_DB_WEIGHT);
        assert_eq!(params.lam_w, DEFAULT_LAMBDA_WEIGHT);
        assert!(!params.force_kbo_var_weight);

        assert!(params.rewrite_strong_rhs_inst);
        assert_eq!(params.to_pre_prec.as_deref(), Some("prec"));
        assert_eq!(params.to_pre_weights.as_deref(), Some("weights"));
        assert_eq!(params.skolem_mod, 77);
        assert_eq!(params.defpred_mod, 88);
        assert!(params.to_defs_min);
    }

    #[test]
    fn print_oparms_matches_comment_shape_and_kbo_weight_line_condition() {
        let mut params = OrderParmsCell::default();
        init_oparms(&mut params);

        assert_eq!(print_oparms_string(&params, 0), "");
        assert_eq!(
            print_oparms_string(&params, 1),
            concat!(
                "% Auto-mode selected ordering type KBO6\n",
                "% Auto-mode selected ordering precedence scheme <unary_first>\n",
                "% Auto-mode selected weight ordering scheme <firstmaximal0>\n",
                "%\n"
            )
        );

        params.ordertype = TermOrdering::Lpo;
        assert_eq!(
            print_oparms_string(&params, 1),
            concat!(
                "% Auto-mode selected ordering type LPO\n",
                "% Auto-mode selected ordering precedence scheme <unary_first>\n",
                "%\n"
            )
        );
    }

    #[test]
    fn order_next_type_follows_c_three_state_cycle() {
        let mut params = OrderParmsCell {
            ordertype: TermOrdering::NoOrdering,
            ..OrderParmsCell::default()
        };

        assert!(order_next_type(&mut params));
        assert_eq!(params.ordertype, TermOrdering::Kbo);
        assert!(order_next_type(&mut params));
        assert_eq!(params.ordertype, TermOrdering::Lpo);
        assert!(!order_next_type(&mut params));
        assert_eq!(params.ordertype, TermOrdering::NoOrdering);
    }

    #[test]
    #[should_panic(expected = "Unexpected ordertype!")]
    fn order_next_type_panics_on_unexpected_c_state() {
        let mut params = OrderParmsCell::default();

        order_next_type(&mut params);
    }

    #[test]
    fn order_next_weight_and_prec_generators_increment_raw_enum_values() {
        let mut params = OrderParmsCell {
            to_weight_gen: TOWeightGenMethod::InvalidEntry,
            to_prec_gen: TOPrecGenMethod::InvalidEntry,
            ..OrderParmsCell::default()
        };

        assert!(order_next_weight_gen(&mut params));
        assert_eq!(params.to_weight_gen, TOWeightGenMethod::NoMethod);
        assert!(order_next_weight_gen(&mut params));
        assert_eq!(params.to_weight_gen, TOWeightGenMethod::SelectMaximal);
        params.to_weight_gen = TOWeightGenMethod::ConstantWeight;
        assert!(!order_next_weight_gen(&mut params));
        assert_eq!(params.to_weight_gen, TOWeightGenMethod::NoMethod);

        assert!(order_next_prec_gen(&mut params));
        assert_eq!(params.to_prec_gen, TOPrecGenMethod::NoMethod);
        assert!(order_next_prec_gen(&mut params));
        assert_eq!(params.to_prec_gen, TOPrecGenMethod::UnaryFirst);
        params.to_prec_gen = TOPrecGenMethod::OrientAxioms;
        assert!(!order_next_prec_gen(&mut params));
        assert_eq!(params.to_prec_gen, TOPrecGenMethod::NoMethod);
    }

    #[test]
    fn order_next_const_weight_preserves_c_cycle_and_over_max_increment() {
        let mut params = OrderParmsCell {
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };

        assert!(!order_next_const_weight(&mut params));
        assert_eq!(params.to_const_weight, W_CONST_NO_WEIGHT);
        assert!(order_next_const_weight(&mut params));
        assert_eq!(params.to_const_weight, 1);
        assert!(order_next_const_weight(&mut params));
        assert_eq!(params.to_const_weight, 2);
        assert!(order_next_const_weight(&mut params));
        assert_eq!(params.to_const_weight, W_CONST_NO_SPECIAL_WEIGHT);

        params.to_const_weight = 3;
        assert!(order_next_const_weight(&mut params));
        assert_eq!(params.to_const_weight, 4);
    }

    #[test]
    fn order_next_ordering_uses_c_nested_parameter_order_and_reset_artifacts() {
        let mask = OrderParmsCell {
            ordertype: TermOrdering::NoOrdering,
            to_weight_gen: TOWeightGenMethod::NoMethod,
            to_prec_gen: TOPrecGenMethod::NoMethod,
            to_const_weight: W_CONST_NO_WEIGHT,
            ..OrderParmsCell::default()
        };
        let mut ordering = OrderParmsCell {
            ordertype: TermOrdering::Kbo,
            to_weight_gen: TOWeightGenMethod::SelectMaximal,
            to_prec_gen: TOPrecGenMethod::UnaryFirst,
            to_const_weight: 1,
            ..OrderParmsCell::default()
        };

        assert!(order_next_ordering(&mut ordering, &mask));
        assert_eq!(ordering.to_const_weight, 2);
        assert_eq!(ordering.to_prec_gen, TOPrecGenMethod::UnaryFirst);

        assert!(order_next_ordering(&mut ordering, &mask));
        assert_eq!(ordering.to_const_weight, W_CONST_NO_SPECIAL_WEIGHT);
        assert_eq!(ordering.to_prec_gen, TOPrecGenMethod::UnaryFirst);

        assert!(order_next_ordering(&mut ordering, &mask));
        assert_eq!(ordering.to_const_weight, 1);
        assert_eq!(ordering.to_prec_gen, TOPrecGenMethod::UnaryFirstFreq);
    }

    #[test]
    fn order_next_ordering_wraps_type_to_first_candidate_when_exhausted() {
        let mask = OrderParmsCell {
            ordertype: TermOrdering::NoOrdering,
            to_weight_gen: TOWeightGenMethod::SelectMaximal,
            to_prec_gen: TOPrecGenMethod::UnaryFirst,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };
        let mut ordering = OrderParmsCell {
            ordertype: TermOrdering::Lpo,
            ..OrderParmsCell::default()
        };

        assert!(!order_next_ordering(&mut ordering, &mask));
        assert_eq!(ordering.ordertype, TermOrdering::Kbo);
    }

    #[test]
    fn helper_strings_cover_auto_ordering_debug_output() {
        let mut params = OrderParmsCell::default();
        init_oparms(&mut params);
        params.ho_order_kind = HoOrderKind::LambdaOrder;

        assert_eq!(
            auto_ordering_analysis_string("AutoSched4"),
            "\n% AutoSched4-Ordering is analysing problem.\n"
        );
        assert_eq!(
            describe_auto_ordering(&params),
            "(KBO6, unary_first, firstmaximal0, -1, lambda)"
        );
    }
}
