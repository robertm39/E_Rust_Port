use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::partial_orderings::HoOrderKind;
use crate::clauses::clausesets::ClauseSet;
use crate::heuristics::clausesetfeatures::{
    clause_set_count_maximal_literals, clause_set_count_maximal_terms,
    clause_set_count_unorientable_literals,
};
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
use crate::terms::termbanks::TermBank;

pub const KBO_BONUS: i64 = 1;
pub const MAX_TERM_PENALTY: i64 = 2;
pub const MAX_LITERAL_PENALTY: i64 = 1;
pub const UNORIENT_LITERAL_PENALTY: i64 = 1;
pub const MAX_CONST_WEIGHT: i64 = 2;
pub const DEFAULT_COMCHAR_RAW: &str = "%";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoOrderingMode {
    Auto,
    AutoCasc,
    AutoDev,
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
            Self::Auto | Self::AutoCasc | Self::AutoDev => "Auto",
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

    #[must_use]
    pub const fn selected_ordering_label(self) -> &'static str {
        match self {
            Self::AutoDev => "Auto-mode (Dev)",
            Self::Auto
            | Self::AutoCasc
            | Self::AutoSched0
            | Self::AutoSched1
            | Self::AutoSched2
            | Self::AutoSched3
            | Self::AutoSched4
            | Self::AutoSched5
            | Self::AutoSched6
            | Self::AutoSched7
            | Self::AutoSched8
            | Self::AutoSched9 => "Auto-mode",
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
    print_oparms_for_mode_string(AutoOrderingMode::Auto, oparms, output_level)
}

#[must_use]
pub fn print_oparms_for_mode_string(
    mode: AutoOrderingMode,
    oparms: &OrderParmsCell,
    output_level: i64,
) -> String {
    if output_level == 0 {
        return String::new();
    }

    let mut result = format!(
        concat!(
            "{comment} {label} selected ordering type {ordertype}\n",
            "{comment} {label} selected ordering precedence scheme <{prec}>\n"
        ),
        comment = DEFAULT_COMCHAR_RAW,
        label = mode.selected_ordering_label(),
        ordertype = oparms.ordertype.name(),
        prec = oparms.to_prec_gen.name().unwrap_or("")
    );

    if matches!(oparms.ordertype, TermOrdering::Kbo | TermOrdering::Kbo6) {
        result.push_str(DEFAULT_COMCHAR_RAW);
        result.push(' ');
        result.push_str(mode.selected_ordering_label());
        result.push_str(" selected weight ordering scheme <");
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

/// Evaluates an ordering on the current axiom clause set.
///
/// This is C `OrderEvaluate` without the surrounding `ProofState` wrapper. It
/// deliberately marks maximal terms on `axioms` as a side effect before reading
/// the clause-set feature counters.
#[allow(clippy::cast_precision_loss)]
pub fn order_evaluate(ocb: &mut OrderControlBlock, bank: &TermBank, axioms: &mut ClauseSet) -> f64 {
    axioms.mark_maximal_terms(ocb, bank);
    let mut result = 0;
    result += clause_set_count_maximal_terms(axioms) * MAX_TERM_PENALTY;
    result += clause_set_count_maximal_literals(axioms) * MAX_LITERAL_PENALTY;
    result += clause_set_count_unorientable_literals(axioms) * UNORIENT_LITERAL_PENALTY;
    if ocb.ordering_type == TermOrdering::Kbo {
        result *= KBO_BONUS;
    }
    result as f64
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
        | AutoOrderingMode::AutoCasc
        | AutoOrderingMode::AutoDev
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
/// This covers C `generate_auto_ordering`, `generate_autocasc_ordering`,
/// `generate_autodev_ordering`, and `generate_autosched0_ordering` through
/// `generate_autosched9_ordering`. Rust initializes the full parameter cell
/// for all modes before applying the visible C field assignments.
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
                precedence_ocb: None,
                pre_weights,
                higher_order_problem,
            },
            &mut handle,
        )?;
    }

    handle.lit_cmp = literal_cmp_from_raw(params.lit_cmp)?;
    Ok(handle)
}

/// Search the C `OrderFindOptimal` candidate space and return the best OCB.
///
/// C's `TOSelectOrdering` has a latent bug when passing `OPTIMIZE_AX` directly
/// as the mask ordering type. Rust treats `Optimize` as the wildcard ordering
/// mask that the helper comment describes, while copying the remaining mask
/// fields from the initialized parameter cell instead of reading indeterminate
/// stack data.
///
/// # Errors
///
/// Returns diagnostics from candidate ordering creation, including unsupported
/// precedence/weight methods.
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as
/// [`to_create_ordering`] and [`order_next_ordering`].
pub fn order_find_optimal(
    bank: &mut TermBank,
    axioms: &mut ClauseSet,
    mask: &OrderParmsCell,
    higher_order_problem: bool,
) -> Result<OrderControlBlock, Diagnostic> {
    order_find_optimal_with_params(bank, axioms, mask, higher_order_problem)
        .map(|(ocb, _params)| ocb)
}

fn order_find_optimal_with_params(
    bank: &mut TermBank,
    axioms: &mut ClauseSet,
    mask: &OrderParmsCell,
    higher_order_problem: bool,
) -> Result<(OrderControlBlock, OrderParmsCell), Diagnostic> {
    let mut search_mask = mask.clone();
    if search_mask.ordertype == TermOrdering::Optimize {
        search_mask.ordertype = TermOrdering::NoOrdering;
    }

    let mut local = search_mask.clone();
    local.ordertype = match search_mask.ordertype {
        TermOrdering::NoOrdering => TermOrdering::Kbo,
        other => other,
    };
    local.to_weight_gen = if search_mask.to_weight_gen == TOWeightGenMethod::NoMethod {
        TOWeightGenMethod::SelectMaximal
    } else {
        search_mask.to_weight_gen
    };
    local.to_prec_gen = if search_mask.to_prec_gen == TOPrecGenMethod::NoMethod {
        TOPrecGenMethod::UnaryFirst
    } else {
        search_mask.to_prec_gen
    };
    local.to_const_weight = if search_mask.to_const_weight == W_CONST_NO_WEIGHT {
        1
    } else {
        search_mask.to_const_weight
    };

    let mut best_params = local.clone();
    let mut best_ocb = to_create_ordering(
        bank.signature_mut(),
        axioms,
        &local,
        None,
        None,
        higher_order_problem,
    )?;
    let mut best_eval = order_evaluate(&mut best_ocb, bank, axioms);

    while order_next_ordering(&mut local, &search_mask) {
        let mut next_ocb = to_create_ordering(
            bank.signature_mut(),
            axioms,
            &local,
            None,
            None,
            higher_order_problem,
        )?;
        let next_eval = order_evaluate(&mut next_ocb, bank, axioms);
        if next_eval < best_eval {
            best_ocb = next_ocb;
            best_eval = next_eval;
            best_params = local.clone();
        }
    }

    Ok((best_ocb, best_params))
}

/// Select and create the term ordering requested by C `HeuristicParmsCell`.
///
/// This ports `TOSelectOrdering`, including the optimizing `OrderFindOptimal`
/// branch over the explicit Rust term bank and axiom set.
///
/// # Errors
///
/// Returns diagnostics from [`to_create_ordering`] or [`order_find_optimal`].
///
/// # Panics
///
/// Panics under the same internal-invariant conditions as
/// [`to_create_ordering`].
pub fn to_select_ordering(
    bank: &mut TermBank,
    axioms: &mut ClauseSet,
    params: &HeuristicParmsCell,
    higher_order_problem: bool,
) -> Result<OrderControlBlock, Diagnostic> {
    let mut tmp = params.order_params.clone();

    let mut result = if tmp.ordertype == TermOrdering::Optimize {
        order_find_optimal(bank, axioms, &tmp, higher_order_problem)?
    } else {
        if tmp.ordertype == TermOrdering::NoOrdering {
            tmp.ordertype = TermOrdering::Kbo;
        }
        if tmp.to_const_weight == W_CONST_NO_WEIGHT {
            tmp.to_const_weight = W_CONST_NO_SPECIAL_WEIGHT;
        }

        to_create_ordering(
            bank.signature_mut(),
            axioms,
            &tmp,
            params.order_params.to_pre_prec.as_deref(),
            params.order_params.to_pre_weights.as_deref(),
            higher_order_problem,
        )?
    };
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
        generate_auto_ordering, init_oparms, order_evaluate, order_find_optimal_with_params,
        order_next_const_weight, order_next_ordering, order_next_prec_gen, order_next_type,
        order_next_weight_gen, print_oparms_for_mode_string, print_oparms_string,
        to_create_ordering, to_select_ordering, AutoOrderingMode, KBO_BONUS, MAX_CONST_WEIGHT,
        MAX_LITERAL_PENALTY, MAX_TERM_PENALTY, UNORIENT_LITERAL_PENALTY,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::partial_orderings::CompareResult;
    use crate::basics::partial_orderings::HoOrderKind;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::heuristics::clausesetfeatures::{
        clause_set_count_maximal_literals, clause_set_count_maximal_terms,
        clause_set_count_unorientable_literals,
    };
    use crate::heuristics::hcb::HeuristicParmsCell;
    use crate::heuristics::to_params::{
        LiteralCmp, OrderParmsCell, TOPrecGenMethod, TOWeightGenMethod, TermOrdering,
        DEFAULT_DB_WEIGHT, DEFAULT_LAMBDA_WEIGHT, W_CONST_NO_SPECIAL_WEIGHT, W_CONST_NO_WEIGHT,
    };
    use crate::orderings::ocb::OrderControlBlock;
    use crate::terms::functypes::FunCode;
    use crate::terms::signature::{Signature, SIG_PHONY_APP_CODE, SIG_TRUE_CODE};
    use crate::terms::simpletypes::{alloc_arrow_type, Type};
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::{DerefType, Term};
    use crate::terms::typebanks::TypeBank;

    fn signature() -> Signature {
        let mut signature = Signature::new(TypeBank::new());
        signature
            .insert_internal_codes()
            .unwrap_or_else(|err| panic!("{err}"));
        signature
    }

    fn term_bank() -> TermBank {
        TermBank::new(signature()).unwrap_or_else(|err| panic!("{err}"))
    }

    fn individual(bank: &TermBank) -> Type {
        bank.signature().type_bank().i_type()
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

    fn typed_const(bank: &mut TermBank, name: &str, type_: &Type) -> Term {
        let code = bank.signature_mut().insert_id(name, 0, false);
        bank.signature_mut()
            .declare_final_type(code, type_.clone())
            .unwrap_or_else(|err| panic!("{err}"));
        bank.create_const_term(code)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn typed_unary(bank: &mut TermBank, name: &str, arg: &Term) -> Term {
        let individual = individual(bank);
        let code = bank.signature_mut().insert_id(name, 1, false);
        bank.signature_mut()
            .declare_final_type(
                code,
                alloc_arrow_type(vec![individual.clone(), individual.clone()]),
            )
            .unwrap_or_else(|err| panic!("{err}"));
        let term = Term::top_alloc(code, 1);
        term.set_type(Some(individual));
        term.set_argument(0, arg.clone());
        bank.insert(&term, DerefType::Never)
            .unwrap_or_else(|err| panic!("{err}"))
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, true).unwrap_or_else(|err| panic!("{err}"))
    }

    fn clause(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
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
    #[expect(
        clippy::cast_precision_loss,
        clippy::float_cmp,
        reason = "test asserts exact C-shaped integer penalty accumulation as double"
    )]
    fn order_evaluate_marks_axioms_and_scores_c_penalty_sum() {
        let mut bank = term_bank();
        let individual = individual(&bank);
        let a = typed_const(&mut bank, "order_eval_a", &individual);
        let b = typed_const(&mut bank, "order_eval_b", &individual);
        let fa = typed_unary(&mut bank, "order_eval_f", &a);
        let mut axioms = ClauseSet::from_clauses([clause(vec![
            literal(&mut bank, &fa, &a),
            literal(&mut bank, &b, &a),
        ])]);
        let mut ocb = OrderControlBlock::alloc(
            TermOrdering::Kbo,
            true,
            bank.signature(),
            HoOrderKind::LfhoOrder,
        );

        assert_eq!(clause_set_count_maximal_terms(&axioms), 0);
        assert_eq!(clause_set_count_maximal_literals(&axioms), 0);

        let score = order_evaluate(&mut ocb, &bank, &mut axioms);
        let expected = (clause_set_count_maximal_terms(&axioms) * MAX_TERM_PENALTY
            + clause_set_count_maximal_literals(&axioms) * MAX_LITERAL_PENALTY
            + clause_set_count_unorientable_literals(&axioms) * UNORIENT_LITERAL_PENALTY)
            * KBO_BONUS;

        assert!(expected > 0);
        assert_eq!(score, expected as f64);
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

        params.ordertype = TermOrdering::Kbo6;
        assert_eq!(
            print_oparms_for_mode_string(AutoOrderingMode::AutoDev, &params, 1),
            concat!(
                "% Auto-mode (Dev) selected ordering type KBO6\n",
                "% Auto-mode (Dev) selected ordering precedence scheme <unary_first>\n",
                "% Auto-mode (Dev) selected weight ordering scheme <firstmaximal0>\n",
                "%\n"
            )
        );
    }

    #[test]
    fn auto_ordering_modes_provide_c_analysis_labels() {
        assert_eq!(AutoOrderingMode::Auto.analysis_label(), "Auto");
        assert_eq!(AutoOrderingMode::AutoCasc.analysis_label(), "Auto");
        assert_eq!(AutoOrderingMode::AutoDev.analysis_label(), "Auto");
        assert_eq!(AutoOrderingMode::AutoSched0.analysis_label(), "AutoSched0");
        assert_eq!(AutoOrderingMode::AutoSched9.analysis_label(), "AutoSched9");
        assert_eq!(
            AutoOrderingMode::AutoCasc.selected_ordering_label(),
            "Auto-mode"
        );
        assert_eq!(
            AutoOrderingMode::AutoDev.selected_ordering_label(),
            "Auto-mode (Dev)"
        );
        assert_eq!(
            auto_ordering_analysis_string(AutoOrderingMode::AutoSched4.analysis_label()),
            "\n% AutoSched4-Ordering is analysing problem.\n"
        );
    }

    #[test]
    fn auto_ordering_params_match_initialized_c_auto_sched_variants() {
        for mode in [
            AutoOrderingMode::Auto,
            AutoOrderingMode::AutoCasc,
            AutoOrderingMode::AutoDev,
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
    fn casc_and_dev_auto_orderings_use_initialized_kbo6_defaults() {
        for mode in [AutoOrderingMode::AutoCasc, AutoOrderingMode::AutoDev] {
            let mut signature = signature();
            typed_symbol(&mut signature, "a", 0);
            typed_symbol(&mut signature, "f", 1);

            let ocb = generate_auto_ordering(
                &mut signature,
                &ClauseSet::new(),
                mode,
                HoOrderKind::LambdaOrder,
                false,
            )
            .unwrap_or_else(|err| panic!("{err}"));

            assert_eq!(ocb.ordering_type, TermOrdering::Kbo6);
            assert_eq!(ocb.lam_weight, DEFAULT_LAMBDA_WEIGHT);
            assert_eq!(ocb.db_weight, DEFAULT_DB_WEIGHT);
            assert_eq!(ocb.ho_order_kind, HoOrderKind::LambdaOrder);
            assert_eq!(ocb.lit_cmp, LiteralCmp::Normal);
        }
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
    fn predefined_only_precedence_dependent_weights_use_parsed_matrix() {
        let mut signature = signature();
        let constant = typed_symbol(&mut signature, "a", 0);
        let unary = typed_symbol(&mut signature, "f", 1);
        let params = OrderParmsCell {
            ordertype: TermOrdering::Kbo,
            to_weight_gen: TOWeightGenMethod::Precedence,
            to_prec_gen: TOPrecGenMethod::NoMethod,
            ..OrderParmsCell::default()
        };

        let ocb = to_create_ordering(
            &mut signature,
            &ClauseSet::new(),
            &params,
            Some("f > a"),
            None,
            false,
        )
        .unwrap_or_else(|err| panic!("{err}"));

        assert!(ocb.prec_weights.is_none());
        assert!(ocb.precedence.is_some());
        assert_eq!(
            ocb.fun_compare(&signature, unary, constant),
            CompareResult::Greater
        );
        assert_eq!(ocb.fun_weight(constant), 1);
        assert_eq!(ocb.fun_weight(unary), 2);
    }

    #[test]
    fn to_select_ordering_defaults_no_ordering_and_zero_const_weight_like_c() {
        let mut bank = term_bank();
        typed_symbol(bank.signature_mut(), "a", 0);
        typed_symbol(bank.signature_mut(), "f", 1);
        let mut axioms = ClauseSet::new();
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

        let ocb = to_select_ordering(&mut bank, &mut axioms, &params, false)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(ocb.ordering_type, TermOrdering::Kbo);
        assert!(ocb.weights.is_some());
        assert_eq!(ocb.var_weight, 1);
        assert!(ocb.rewrite_strong_rhs_inst);
    }

    #[test]
    fn to_select_ordering_uses_original_predefined_strings() {
        let mut bank = term_bank();
        let constant = typed_symbol(bank.signature_mut(), "a", 0);
        let unary = typed_symbol(bank.signature_mut(), "f", 1);
        let mut axioms = ClauseSet::new();
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

        let ocb = to_select_ordering(&mut bank, &mut axioms, &params, false)
            .unwrap_or_else(|err| panic!("{err}"));

        assert_eq!(
            ocb.fun_compare(bank.signature(), unary, constant),
            CompareResult::Greater
        );
        assert_eq!(ocb.fun_weight(constant), 11);
    }

    #[test]
    fn order_find_optimal_treats_optimize_as_wildcard_order_type() {
        let mut bank = term_bank();
        typed_symbol(bank.signature_mut(), "a", 0);
        typed_symbol(bank.signature_mut(), "f", 1);
        let mut axioms = ClauseSet::new();
        let mask = OrderParmsCell {
            ordertype: TermOrdering::Optimize,
            to_weight_gen: TOWeightGenMethod::ConstantWeight,
            to_prec_gen: TOPrecGenMethod::UnaryFirst,
            to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
            ..OrderParmsCell::default()
        };

        let (ocb, selected) = order_find_optimal_with_params(&mut bank, &mut axioms, &mask, false)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(matches!(
            selected.ordertype,
            TermOrdering::Kbo | TermOrdering::Lpo
        ));
        assert_eq!(ocb.ordering_type, selected.ordertype);
        assert_eq!(selected.to_weight_gen, TOWeightGenMethod::ConstantWeight);
        assert_eq!(selected.to_prec_gen, TOPrecGenMethod::UnaryFirst);
        assert_eq!(selected.lit_cmp, i64::from(LiteralCmp::Normal.c_value()));
        assert_eq!(selected.ho_order_kind, HoOrderKind::LfhoOrder);
    }

    #[test]
    fn to_select_ordering_uses_optimized_branch_and_propagates_rewrite_flag() {
        let mut bank = term_bank();
        typed_symbol(bank.signature_mut(), "a", 0);
        typed_symbol(bank.signature_mut(), "f", 1);
        let mut axioms = ClauseSet::new();
        let params = HeuristicParmsCell {
            order_params: OrderParmsCell {
                ordertype: TermOrdering::Optimize,
                to_weight_gen: TOWeightGenMethod::ConstantWeight,
                to_prec_gen: TOPrecGenMethod::UnaryFirst,
                to_const_weight: W_CONST_NO_SPECIAL_WEIGHT,
                rewrite_strong_rhs_inst: true,
                ..OrderParmsCell::default()
            },
            ..HeuristicParmsCell::default()
        };

        let ocb = to_select_ordering(&mut bank, &mut axioms, &params, false)
            .unwrap_or_else(|err| panic!("{err}"));

        assert!(matches!(
            ocb.ordering_type,
            TermOrdering::Kbo | TermOrdering::Lpo
        ));
        assert!(ocb.rewrite_strong_rhs_inst);
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
