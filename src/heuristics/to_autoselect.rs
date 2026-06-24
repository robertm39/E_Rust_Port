use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::HoOrderKind;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::hcb::HeuristicParmsCell;
use crate::heuristics::to_params::{
    ho_order_kind_name, LiteralCmp, OrderParmsCell, TOPrecGenMethod, TOWeightGenMethod,
    TermOrdering, DEFAULT_DB_WEIGHT, DEFAULT_LAMBDA_WEIGHT, W_CONST_NO_SPECIAL_WEIGHT,
    W_CONST_NO_WEIGHT,
};
use crate::heuristics::to_precgen::generate_precedence_into_ocb_with_order;
use crate::heuristics::to_weightgen::{generate_weights_into_ocb, WeightGenerationContext};
use crate::orderings::ocb::OrderControlBlock;
use crate::terms::signature::Signature;

pub const KBO_BONUS: i64 = 1;
pub const MAX_TERM_PENALTY: i64 = 2;
pub const MAX_LITERAL_PENALTY: i64 = 1;
pub const UNORIENT_LITERAL_PENALTY: i64 = 1;
pub const MAX_CONST_WEIGHT: i64 = 2;
pub const DEFAULT_COMCHAR_RAW: &str = "%";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoOrderingMode {
    Auto,
    AutoSched0,
    AutoSched1,
    AutoSched2,
    AutoSched3,
    AutoSched4,
    AutoSched5,
    AutoSched6,
    AutoSched7,
    AutoSched8,
    AutoSched9,
}

impl AutoOrderingMode {
    #[must_use]
    pub const fn analysis_label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::AutoSched0 => "AutoSched0",
            Self::AutoSched1 => "AutoSched1",
            Self::AutoSched2 => "AutoSched2",
            Self::AutoSched3 => "AutoSched3",
            Self::AutoSched4 => "AutoSched4",
            Self::AutoSched5 => "AutoSched5",
            Self::AutoSched6 => "AutoSched6",
            Self::AutoSched7 => "AutoSched7",
            Self::AutoSched8 => "AutoSched8",
            Self::AutoSched9 => "AutoSched9",
        }
    }
}

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

#[must_use]
pub fn auto_ordering_params(mode: AutoOrderingMode, ho_order_kind: HoOrderKind) -> OrderParmsCell {
    let mut params = OrderParmsCell::default();
    match mode {
        AutoOrderingMode::Auto
        | AutoOrderingMode::AutoSched0
        | AutoOrderingMode::AutoSched1
        | AutoOrderingMode::AutoSched2
        | AutoOrderingMode::AutoSched3
        | AutoOrderingMode::AutoSched4
        | AutoOrderingMode::AutoSched5
        | AutoOrderingMode::AutoSched6
        | AutoOrderingMode::AutoSched7
        | AutoOrderingMode::AutoSched8
        | AutoOrderingMode::AutoSched9 => init_oparms(&mut params),
    }
    params.ho_order_kind = ho_order_kind;
    params
}

/// Generate an initialized auto/AutoSched ordering.
///
/// This covers C `generate_auto_ordering` and `generate_autosched0_ordering`
/// through `generate_autosched9_ordering`, which all call `init_oparms` and
/// then `TOCreateOrdering`.
///
/// # Errors
///
/// Returns diagnostics from [`to_create_ordering`].
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as
/// [`to_create_ordering`].
pub fn generate_auto_ordering(
    signature: &mut Signature,
    axioms: &ClauseSet,
    mode: AutoOrderingMode,
    ho_order_kind: HoOrderKind,
    higher_order_problem: bool,
) -> Result<OrderControlBlock, Diagnostic> {
    let params = auto_ordering_params(mode, ho_order_kind);
    to_create_ordering(signature, axioms, &params, None, None, higher_order_problem)
}

/// Create the OCB described by a fully specified C `OrderParmsCell`.
///
/// This mirrors `TOCreateOrdering`: allocate the concrete ordering, generate
/// or parse precedence, generate KBO weights when needed, and then install the
/// literal-comparison mode.
///
/// # Errors
///
/// Returns a diagnostic for incomplete order-parameter cells, unsupported
/// precedence or weight generation, predefined parser errors, or invalid raw
/// literal-comparison values.
///
/// # Panics
///
/// Panics for `RPO`, matching the C assertion that this ordering is not yet
/// implemented.
pub fn to_create_ordering(
    signature: &mut Signature,
    axioms: &ClauseSet,
    params: &OrderParmsCell,
    pre_precedence: Option<&str>,
    pre_weights: Option<&str>,
    higher_order_problem: bool,
) -> Result<OrderControlBlock, Diagnostic> {
    assert!(
        matches!(
            params.ho_order_kind,
            HoOrderKind::LfhoOrder | HoOrderKind::LambdaOrder
        ),
        "TOCreateOrdering requires LFHO or Lambda ordering kind"
    );

    let prec_by_weight = pre_precedence.is_none();
    let mut handle = match params.ordertype {
        TermOrdering::Lpo
        | TermOrdering::LpoCopy
        | TermOrdering::Lpo4
        | TermOrdering::Lpo4Copy
        | TermOrdering::Kbo
        | TermOrdering::Kbo6 => OrderControlBlock::alloc(
            params.ordertype,
            prec_by_weight,
            signature,
            params.ho_order_kind,
        ),
        TermOrdering::Rpo => panic!("RPO not yet implemented!"),
        TermOrdering::NoOrdering | TermOrdering::Optimize | TermOrdering::Empty => {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                format!(
                    "Incompletely specified OrderParamsCell: {}",
                    params.ordertype.name()
                ),
            ));
        }
    };

    let precedence_order = generate_precedence_into_ocb_with_order(
        signature,
        axioms,
        pre_precedence,
        params,
        &mut handle,
    )?;
    if matches!(params.ordertype, TermOrdering::Kbo | TermOrdering::Kbo6) {
        generate_weights_into_ocb(
            signature,
            axioms,
            params,
            WeightGenerationContext {
                precedence_order: precedence_order.as_deref(),
                pre_weights,
                higher_order_problem,
            },
            &mut handle,
        )?;
    }

    handle.lit_cmp = literal_cmp_from_raw(params.lit_cmp)?;
    Ok(handle)
}

/// Select and create the term ordering requested by C `HeuristicParmsCell`.
///
/// This ports the non-`Optimize` branch of `TOSelectOrdering`; the optimizing
/// search still needs `OrderFindOptimal` and axiom maximality scoring.
///
/// # Errors
///
/// Returns diagnostics from [`to_create_ordering`] or an explicit diagnostic
/// for the still-pending optimize search.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as
/// [`to_create_ordering`].
pub fn to_select_ordering(
    signature: &mut Signature,
    axioms: &ClauseSet,
    params: &HeuristicParmsCell,
    higher_order_problem: bool,
) -> Result<OrderControlBlock, Diagnostic> {
    let mut tmp = params.order_params.clone();

    if tmp.ordertype == TermOrdering::Optimize {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "OrderFindOptimal is not yet implemented",
        ));
    }
    if tmp.ordertype == TermOrdering::NoOrdering {
        tmp.ordertype = TermOrdering::Kbo;
    }
    if tmp.to_const_weight == W_CONST_NO_WEIGHT {
        tmp.to_const_weight = W_CONST_NO_SPECIAL_WEIGHT;
    }

    let mut result = to_create_ordering(
        signature,
        axioms,
        &tmp,
        params.order_params.to_pre_prec.as_deref(),
        params.order_params.to_pre_weights.as_deref(),
        higher_order_problem,
    )?;
    result.rewrite_strong_rhs_inst = params.order_params.rewrite_strong_rhs_inst;
    Ok(result)
}

fn literal_cmp_from_raw(value: i64) -> Result<LiteralCmp, Diagnostic> {
    i32::try_from(value)
        .ok()
        .and_then(LiteralCmp::from_c_value)
        .ok_or_else(|| {
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                format!("Invalid literal comparison value {value}"),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::{
        auto_ordering_analysis_string, auto_ordering_params, describe_auto_ordering,
        generate_auto_ordering, init_oparms, order_next_const_weight, order_next_ordering,
        order_next_prec_gen, order_next_type, order_next_weight_gen, print_oparms_string,
        to_create_ordering, to_select_ordering, AutoOrderingMode, KBO_BONUS, MAX_CONST_WEIGHT,
        MAX_LITERAL_PENALTY, MAX_TERM_PENALTY, UNORIENT_LITERAL_PENALTY,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::CompareResult;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clausesets::ClauseSet;
    use crate::heuristics::hcb::HeuristicParmsCell;
    use crate::heuristics::to_params::{
        LiteralCmp, OrderParmsCell, TOPrecGenMethod, TOWeightGenMethod, TermOrdering,
        DEFAULT_DB_WEIGHT, DEFAULT_LAMBDA_WEIGHT, W_CONST_NO_SPECIAL_WEIGHT, W_CONST_NO_WEIGHT,
    };
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE, SIG_TRUE_CODE};
    use crate::terms::simpletypes::alloc_arrow_type;
    use crate::terms::typebanks::TypeBank;

    fn signature() -> Signature {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        signature
    }

    fn typed_symbol(signature: &mut Signature, name: &str, arity: i32) -> FunCode {
        let code = signature.insert_id(name, arity, false);
        let individual = signature.type_bank().i_type();
        let type_ = if arity == 0 {
            individual
        } else {
            let mut args = Vec::new();
            for _ in 0..arity {
                args.push(individual.clone());
            }
            args.push(individual);
            alloc_arrow_type(args)
        };
        signature
            .declare_final_type(code, type_)
            .unwrap_or_else(|err| panic!("{err}"));
        code
    }

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
    fn auto_ordering_modes_provide_c_analysis_labels() {
        assert_eq!(AutoOrderingMode::Auto.analysis_label(), "Auto");
        assert_eq!(AutoOrderingMode::AutoSched0.analysis_label(), "AutoSched0");
        assert_eq!(AutoOrderingMode::AutoSched9.analysis_label(), "AutoSched9");
        assert_eq!(
            auto_ordering_analysis_string(AutoOrderingMode::AutoSched4.analysis_label()),
            "\n% AutoSched4-Ordering is analysing problem.\n"
        );
    }

    #[test]
    fn auto_ordering_params_match_initialized_c_auto_sched_variants() {
        for mode in [
            AutoOrderingMode::Auto,
            AutoOrderingMode::AutoSched0,
            AutoOrderingMode::AutoSched1,
            AutoOrderingMode::AutoSched2,
            AutoOrderingMode::AutoSched3,
            AutoOrderingMode::AutoSched4,
            AutoOrderingMode::AutoSched5,
            AutoOrderingMode::AutoSched6,
            AutoOrderingMode::AutoSched7,
            AutoOrderingMode::AutoSched8,
            AutoOrderingMode::AutoSched9,
        ] {
            let params = auto_ordering_params(mode, HoOrderKind::LambdaOrder);

            assert_eq!(params.ordertype, TermOrdering::Kbo6);
            assert_eq!(params.to_const_weight, W_CONST_NO_SPECIAL_WEIGHT);
            assert_eq!(params.to_weight_gen, TOWeightGenMethod::SelectMaximal);
            assert_eq!(params.to_prec_gen, TOPrecGenMethod::UnaryFirst);
            assert_eq!(params.lit_cmp, i64::from(LiteralCmp::Normal.c_value()));
            assert_eq!(params.ho_order_kind, HoOrderKind::LambdaOrder);
            assert_eq!(params.db_w, DEFAULT_DB_WEIGHT);
            assert_eq!(params.lam_w, DEFAULT_LAMBDA_WEIGHT);
            assert!(!params.force_kbo_var_weight);
        }
    }

    #[test]
    fn generate_auto_ordering_creates_default_kbo6_ocb() {
        let mut signature = signature();
        typed_symbol(&mut signature, "a", 0);
        typed_symbol(&mut signature, "f", 1);

        let ocb = generate_auto_ordering(
            &mut signature,
            &ClauseSet::new(),
            AutoOrderingMode::AutoSched3,
            HoOrderKind::LfhoOrder,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.ordering_type, TermOrdering::Kbo6);
        assert!(ocb.weights.is_some());
        assert!(ocb.prec_weights.is_some());
        assert_eq!(ocb.ho_order_kind, HoOrderKind::LfhoOrder);
    }

    #[test]
    fn to_create_ordering_builds_lpo_with_generated_precedence() {
        let mut signature = signature();
        let unary = typed_symbol(&mut signature, "f", 1);
        let binary = typed_symbol(&mut signature, "g", 2);
        let params = OrderParmsCell {
            ordertype: TermOrdering::Lpo,
            to_prec_gen: TOPrecGenMethod::UnaryFirst,
            lit_cmp: i64::from(LiteralCmp::TfoEqMax.c_value()),
            ..OrderParmsCell::default()
        };

        let ocb = to_create_ordering(
            &mut signature,
            &ClauseSet::new(),
            &params,
            None,
            None,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.ordering_type, TermOrdering::Lpo);
        assert!(ocb.weights.is_none());
        assert!(ocb.prec_weights.is_some());
        assert!(ocb.precedence.is_none());
        assert_eq!(ocb.lit_cmp, LiteralCmp::TfoEqMax);
        assert_eq!(
            ocb.fun_compare(&signature, unary, binary),
            CompareResult::Greater
        );
    }

    #[test]
    fn to_create_ordering_builds_kbo6_with_generated_weights() {
        let mut signature = signature();
        typed_symbol(&mut signature, "a", 0);
        typed_symbol(&mut signature, "f", 1);
        let mut params = OrderParmsCell::default();
        init_oparms(&mut params);

        let ocb = to_create_ordering(
            &mut signature,
            &ClauseSet::new(),
            &params,
            None,
            None,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.ordering_type, TermOrdering::Kbo6);
        assert!(ocb.weights.is_some());
        assert!(ocb.prec_weights.is_some());
        assert!(ocb.precedence.is_none());
        assert_eq!(ocb.fun_weight(SIG_TRUE_CODE), ocb.var_weight);
        assert_eq!(ocb.fun_weight(SIG_PHONY_APP_CODE), 0);
        assert_eq!(ocb.lam_weight, params.lam_w);
        assert_eq!(ocb.db_weight, params.db_w);
        assert_eq!(ocb.lit_cmp, LiteralCmp::Normal);
    }

    #[test]
    fn to_create_ordering_applies_predefined_precedence_and_weight_overrides() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let params = OrderParmsCell {
            ordertype: TermOrdering::Kbo,
            to_weight_gen: TOWeightGenMethod::ConstantWeight,
            to_prec_gen: TOPrecGenMethod::NoMethod,
            lit_cmp: i64::from(LiteralCmp::TfoEqMin.c_value()),
            ..OrderParmsCell::default()
        };

        let ocb = to_create_ordering(
            &mut signature,
            &ClauseSet::new(),
            &params,
            Some("f > a"),
            Some("a:9"),
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(ocb.prec_weights.is_none());
        assert!(ocb.precedence.is_some());
        assert_eq!(
            ocb.fun_compare(&signature, unary, constant),
            CompareResult::Greater
        );
        assert_eq!(ocb.fun_weight(constant), 9);
        assert_eq!(ocb.fun_weight(unary), 1);
        assert_eq!(ocb.lit_cmp, LiteralCmp::TfoEqMin);
    }

    #[test]
    fn to_create_ordering_reports_incomplete_or_invalid_parameter_cells() {
        let mut signature = signature();
        let incomplete = OrderParmsCell {
            ordertype: TermOrdering::NoOrdering,
            ..OrderParmsCell::default()
        };

        let error = to_create_ordering(
            &mut signature,
            &ClauseSet::new(),
            &incomplete,
            None,
            None,
            false,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(error.message().contains("Incompletely specified"));

        let invalid_literal_cmp = OrderParmsCell {
            ordertype: TermOrdering::Lpo,
            to_prec_gen: TOPrecGenMethod::UnaryFirst,
            lit_cmp: 99,
            ..OrderParmsCell::default()
        };
        let error = to_create_ordering(
            &mut signature,
            &ClauseSet::new(),
            &invalid_literal_cmp,
            None,
            None,
            false,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(error.message().contains("Invalid literal comparison"));
    }

    #[test]
    fn predefined_only_precedence_dependent_weights_stay_explicitly_unsupported() {
        let mut signature = signature();
        typed_symbol(&mut signature, "a", 0);
        typed_symbol(&mut signature, "f", 1);
        let params = OrderParmsCell {
            ordertype: TermOrdering::Kbo,
            to_weight_gen: TOWeightGenMethod::SelectMaximal,
            to_prec_gen: TOPrecGenMethod::NoMethod,
            ..OrderParmsCell::default()
        };

        let error = to_create_ordering(
            &mut signature,
            &ClauseSet::new(),
            &params,
            Some("f > a"),
            None,
            false,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(error.message().contains("requires precedence order"));
    }

    #[test]
    fn to_select_ordering_defaults_no_ordering_and_zero_const_weight_like_c() {
        let mut signature = signature();
        typed_symbol(&mut signature, "a", 0);
        typed_symbol(&mut signature, "f", 1);
        let params = HeuristicParmsCell {
            order_params: OrderParmsCell {
                ordertype: TermOrdering::NoOrdering,
                to_weight_gen: TOWeightGenMethod::ConstantWeight,
                to_prec_gen: TOPrecGenMethod::UnaryFirst,
                to_const_weight: W_CONST_NO_WEIGHT,
                rewrite_strong_rhs_inst: true,
                ..OrderParmsCell::default()
            },
            ..HeuristicParmsCell::default()
        };

        let ocb = to_select_ordering(&mut signature, &ClauseSet::new(), &params, false)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.ordering_type, TermOrdering::Kbo);
        assert!(ocb.weights.is_some());
        assert_eq!(ocb.var_weight, 1);
        assert!(ocb.rewrite_strong_rhs_inst);
    }

    #[test]
    fn to_select_ordering_uses_original_predefined_strings() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let params = HeuristicParmsCell {
            order_params: OrderParmsCell {
                ordertype: TermOrdering::Kbo,
                to_weight_gen: TOWeightGenMethod::ConstantWeight,
                to_prec_gen: TOPrecGenMethod::NoMethod,
                to_pre_prec: Some("f > a".to_owned()),
                to_pre_weights: Some("a:11".to_owned()),
                ..OrderParmsCell::default()
            },
            ..HeuristicParmsCell::default()
        };

        let ocb = to_select_ordering(&mut signature, &ClauseSet::new(), &params, false)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            ocb.fun_compare(&signature, unary, constant),
            CompareResult::Greater
        );
        assert_eq!(ocb.fun_weight(constant), 11);
    }

    #[test]
    fn to_select_ordering_reports_pending_optimize_branch() {
        let mut signature = signature();
        let params = HeuristicParmsCell {
            order_params: OrderParmsCell {
                ordertype: TermOrdering::Optimize,
                ..OrderParmsCell::default()
            },
            ..HeuristicParmsCell::default()
        };

        let error =
            to_select_ordering(&mut signature, &ClauseSet::new(), &params, false).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(error.message().contains("OrderFindOptimal"));
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
