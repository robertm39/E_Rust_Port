//! Port of `PCL2/pcl_lemmas`.

use std::cmp::Ordering;

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::pcl2::expressions::{PclExpression, PclExpressionData, PclOpCode, PclQuote};
use crate::pcl2::idents::PclId;
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStep, PclStepLogic, PCL_IS_FOF_STEP, PCL_IS_LEMMA, PCL_NO_WEIGHT};

pub const LEMMA_TREE_BASE_W: i64 = 1;
pub const LEMMA_ACT_PM_W: f32 = 2.0;
pub const LEMMA_O_GEN_W: f32 = 1.0;
pub const LEMMA_ACT_SIMPL_W: f32 = 2.0;
pub const LEMMA_PAS_SIMPL_W: f32 = 1.0;
pub const LEMMA_PROOF_TREE_W: f32 = 1.0;
pub const LEMMA_PROOF_DAG_W: f32 = 0.0;
pub const LEMMA_SIZE_BASE_W: i64 = 1;
pub const LEMMA_HORN_BONUS_W: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LemmaParams {
    pub tree_base_weight: i64,
    pub act_pm_w: f32,
    pub o_gen_w: f32,
    pub act_simpl_w: f32,
    pub pas_simpl_w: f32,
    pub proof_tree_w: f32,
    pub proof_dag_w: f32,
    pub size_base_weight: i64,
    pub horn_bonus: f32,
}

impl Default for LemmaParams {
    fn default() -> Self {
        Self {
            tree_base_weight: LEMMA_TREE_BASE_W,
            act_pm_w: LEMMA_ACT_PM_W,
            o_gen_w: LEMMA_O_GEN_W,
            act_simpl_w: LEMMA_ACT_SIMPL_W,
            pas_simpl_w: LEMMA_PAS_SIMPL_W,
            proof_tree_w: LEMMA_PROOF_TREE_W,
            proof_dag_w: LEMMA_PROOF_DAG_W,
            size_base_weight: LEMMA_SIZE_BASE_W,
            horn_bonus: LEMMA_HORN_BONUS_W,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InferenceWeights {
    weights: [i64; PclOpCode::MaxOp as usize],
}

impl Default for InferenceWeights {
    fn default() -> Self {
        let mut weights = [0; PclOpCode::MaxOp as usize];
        weights[PclOpCode::NoOp as usize] = 0;
        weights[PclOpCode::Initial as usize] = 1;
        weights[PclOpCode::Quote as usize] = 0;
        weights[PclOpCode::EvalGc as usize] = 0;
        weights[PclOpCode::Paramod as usize] = 1;
        weights[PclOpCode::SimParamod as usize] = 1;
        weights[PclOpCode::EResolution as usize] = 1;
        weights[PclOpCode::EFactoring as usize] = 1;
        weights[PclOpCode::SimplifyReflect as usize] = 1;
        weights[PclOpCode::ContextSimplifyReflect as usize] = 1;
        weights[PclOpCode::ACResolution as usize] = 2;
        weights[PclOpCode::Rewrite as usize] = 1;
        weights[PclOpCode::URewrite as usize] = 1;
        weights[PclOpCode::ClauseNormalize as usize] = 1;
        weights[PclOpCode::SplitClause as usize] = 1;
        Self { weights }
    }
}

impl InferenceWeights {
    #[must_use]
    pub const fn get(self, op: PclOpCode) -> i64 {
        self.weights[op as usize]
    }

    pub fn set(&mut self, op: PclOpCode, weight: i64) {
        self.weights[op as usize] = weight;
    }
}

/// C `PCLExprUpdateRefs`.
pub fn expr_update_refs(protocol: &mut PclProtocol, expr: &PclExpression) {
    match expr.data() {
        PclExpressionData::None
        | PclExpressionData::Initial(_)
        | PclExpressionData::Quote { .. } => {}
        PclExpressionData::Compound(args) => match expr.op() {
            PclOpCode::Paramod | PclOpCode::SimParamod => {
                if let Some(first) = args.first() {
                    update_ref_or_recurse(protocol, first.expr(), |step| {
                        step.tree_data_mut().other_generating_refs += 1;
                    });
                }
                if let Some(second) = args.get(1) {
                    update_ref_or_recurse(protocol, second.expr(), |step| {
                        step.tree_data_mut().active_pm_refs += 1;
                    });
                }
            }
            PclOpCode::EResolution | PclOpCode::EFactoring | PclOpCode::SplitClause => {
                if let Some(first) = args.first() {
                    update_ref_or_recurse(protocol, first.expr(), |step| {
                        step.tree_data_mut().other_generating_refs += 1;
                    });
                }
            }
            PclOpCode::SimplifyReflect
            | PclOpCode::ACResolution
            | PclOpCode::Rewrite
            | PclOpCode::URewrite
            | PclOpCode::ClauseNormalize => {
                if let Some(first) = args.first() {
                    update_ref_or_recurse(protocol, first.expr(), |step| {
                        step.tree_data_mut().passive_simpl_refs += 1;
                    });
                }
                for argument in args.iter().skip(1) {
                    update_ref_or_recurse(protocol, argument.expr(), |step| {
                        step.tree_data_mut().active_simpl_refs += 1;
                    });
                }
            }
            _ => {}
        },
    }
}

/// C `PCLStepUpdateRefs`.
pub fn step_update_refs(protocol: &mut PclProtocol, id: &PclId) {
    let Some(step) = protocol.find_step(id) else {
        return;
    };
    let just = step.just().clone();
    if just.op() == PclOpCode::Quote {
        if let Some(parent) = full_quote_id(&just).and_then(|id| protocol.find_step_mut(id)) {
            parent.tree_data_mut().pure_quote_refs += 1;
        }
    }
    expr_update_refs(protocol, &just);
}

/// C `PCLProtUpdateRefs`.
pub fn protocol_update_refs(protocol: &mut PclProtocol) {
    for id in protocol.step_ids() {
        step_update_refs(protocol, &id);
    }
}

/// C `PCLStepLemmaCmp`.
#[must_use]
pub fn step_lemma_cmp(left: &PclStep, right: &PclStep) -> Ordering {
    left.tree_data()
        .lemma_quality
        .partial_cmp(&right.tree_data().lemma_quality)
        .unwrap_or(Ordering::Equal)
}

/// C `PCLExprProofSize`.
///
/// # Errors
///
/// Returns a syntax diagnostic if a quoted parent is missing or if a mini
/// identifier appears in a full-protocol expression.
pub fn expr_proof_size(
    protocol: &mut PclProtocol,
    expr: &PclExpression,
    weights: InferenceWeights,
    use_lemmas: bool,
) -> Result<i64, Diagnostic> {
    match expr.data() {
        PclExpressionData::Quote {
            quote: PclQuote::Full(id),
            ..
        } => step_proof_size(protocol, id, weights, use_lemmas),
        PclExpressionData::Quote {
            quote: PclQuote::Mini(_),
            ..
        } => Err(lemma_error(
            "Mini PCL identifier found in full protocol expression",
        )),
        PclExpressionData::Initial(_) => Ok(weights.get(PclOpCode::Initial)),
        PclExpressionData::None => Ok(weights.get(expr.op())),
        PclExpressionData::Compound(args) => {
            let mut result = weights.get(expr.op());
            for argument in args {
                result += expr_proof_size(protocol, argument.expr(), weights, use_lemmas)?;
            }
            Ok(result)
        }
    }
}

/// C `PCLStepProofSize`.
///
/// # Errors
///
/// Returns a syntax diagnostic if the step id or one of its ancestors is
/// missing.
pub fn step_proof_size(
    protocol: &mut PclProtocol,
    id: &PclId,
    weights: InferenceWeights,
    use_lemmas: bool,
) -> Result<i64, Diagnostic> {
    let Some(step) = protocol.find_step(id) else {
        return Err(lemma_error("Reference to non-existing step"));
    };
    let cached = step.tree_data().proof_tree_size;
    if cached == PCL_NO_WEIGHT {
        let just = step.just().clone();
        let proof_size = expr_proof_size(protocol, &just, weights, use_lemmas)?;
        let Some(step) = protocol.find_step_mut(id) else {
            return Err(lemma_error("Reference to non-existing step"));
        };
        step.tree_data_mut().proof_tree_size = proof_size;
    }

    let Some(step) = protocol.find_step(id) else {
        return Err(lemma_error("Reference to non-existing step"));
    };
    if use_lemmas && step.properties().query(PCL_IS_LEMMA) {
        Ok(0)
    } else {
        Ok(step.tree_data().proof_tree_size)
    }
}

/// C `PCLProtComputeProofSize`.
///
/// # Errors
///
/// Returns a syntax diagnostic if a referenced parent is missing.
pub fn protocol_compute_proof_size(
    protocol: &mut PclProtocol,
    weights: InferenceWeights,
    use_lemmas: bool,
) -> Result<(), Diagnostic> {
    for id in protocol.step_ids() {
        let _ = step_proof_size(protocol, &id, weights, use_lemmas)?;
    }
    Ok(())
}

/// C `PCLStepComputeLemmaWeight`.
#[must_use]
pub fn step_compute_lemma_weight(
    protocol: &mut PclProtocol,
    id: &PclId,
    params: LemmaParams,
) -> f32 {
    let Some(step) = protocol.find_step(id) else {
        return 0.0;
    };
    let quality = if step.properties().query(PCL_IS_FOF_STEP) {
        0.0
    } else if let Some(clause) = step_clause(step) {
        c_lemma_quality(protocol, step, clause, params)
    } else {
        0.0
    };

    if let Some(step) = protocol.find_step_mut(id) {
        step.tree_data_mut().lemma_quality = quality;
    }
    quality
}

/// C `PCLProtComputeLemmaWeights`.
#[must_use]
pub fn protocol_compute_lemma_weights(
    protocol: &mut PclProtocol,
    params: LemmaParams,
) -> Option<PclId> {
    let mut best = None;
    let mut best_rating = -1.0_f32;
    for id in protocol.step_ids() {
        let current_rating = step_compute_lemma_weight(protocol, &id, params);
        let is_lemma = protocol
            .find_step(&id)
            .is_some_and(|step| step.properties().query(PCL_IS_LEMMA));
        if current_rating > best_rating && !is_lemma {
            best_rating = current_rating;
            best = Some(id);
        }
    }
    best
}

/// C `PCLProtSeqFindLemmas`.
///
/// # Errors
///
/// Returns a syntax diagnostic if a referenced parent is missing.
pub fn protocol_seq_find_lemmas(
    protocol: &mut PclProtocol,
    params: LemmaParams,
    weights: InferenceWeights,
    max_number: i64,
    quality_limit: f32,
) -> Result<i64, Diagnostic> {
    protocol.reset_tree_data(false);
    protocol_update_refs(protocol);

    let mut selected = 0;
    for id in protocol.step_ids() {
        let _ = step_proof_size(protocol, &id, weights, true)?;
        if step_compute_lemma_weight(protocol, &id, params) >= quality_limit {
            if let Some(step) = protocol.find_step_mut(&id) {
                step.set_property(PCL_IS_LEMMA);
            }
            selected += 1;
            if selected > max_number {
                break;
            }
        }
    }
    Ok(selected)
}

/// C `PCLProtRecFindLemmas`.
///
/// # Errors
///
/// Returns a syntax diagnostic if a referenced parent is missing.
pub fn protocol_rec_find_lemmas(
    protocol: &mut PclProtocol,
    params: LemmaParams,
    weights: InferenceWeights,
    max_number: i64,
    quality_limit: f32,
) -> Result<i64, Diagnostic> {
    protocol.reset_tree_data(false);
    protocol_update_refs(protocol);

    let mut selected = 0;
    while selected < max_number {
        protocol.reset_tree_data(true);
        protocol_compute_proof_size(protocol, weights, true)?;
        let Some(lemma) = protocol_compute_lemma_weights(protocol, params) else {
            break;
        };
        let quality = protocol
            .find_step(&lemma)
            .map_or(0.0, |step| step.tree_data().lemma_quality);
        if quality < quality_limit {
            break;
        }
        if let Some(step) = protocol.find_step_mut(&lemma) {
            step.set_property(PCL_IS_LEMMA);
        }
        selected += 1;
    }
    Ok(selected)
}

/// C `PCLProtFlatFindLemmas`.
///
/// # Errors
///
/// Returns a syntax diagnostic if a referenced parent is missing.
pub fn protocol_flat_find_lemmas(
    protocol: &mut PclProtocol,
    params: LemmaParams,
    weights: InferenceWeights,
    max_number: i64,
    quality_limit: f32,
) -> Result<i64, Diagnostic> {
    protocol.reset_tree_data(false);
    protocol_update_refs(protocol);
    protocol_compute_proof_size(protocol, weights, true)?;
    let _ = protocol_compute_lemma_weights(protocol, params);

    let mut ids = protocol.step_ids();
    ids.sort_by(|left, right| {
        let left = protocol.find_step(left);
        let right = protocol.find_step(right);
        match (left, right) {
            (Some(left), Some(right)) => step_lemma_cmp(left, right),
            _ => Ordering::Equal,
        }
    });

    let limit = max_number.min(ids.len().try_into().unwrap_or(i64::MAX));
    let mut selected = 0;
    while selected < limit {
        let Some(id) = ids.pop() else {
            break;
        };
        let quality = protocol
            .find_step(&id)
            .map_or(0.0, |step| step.tree_data().lemma_quality);
        if quality < quality_limit {
            break;
        }
        if let Some(step) = protocol.find_step_mut(&id) {
            step.set_property(PCL_IS_LEMMA);
        }
        selected += 1;
    }
    Ok(selected)
}

fn update_ref_or_recurse(
    protocol: &mut PclProtocol,
    expr: &PclExpression,
    update: impl FnOnce(&mut PclStep),
) {
    if let Some(id) = full_quote_id(expr) {
        if let Some(step) = protocol.find_step_mut(id) {
            update(step);
        }
    } else {
        expr_update_refs(protocol, expr);
    }
}

fn full_quote_id(expr: &PclExpression) -> Option<&PclId> {
    match expr.data() {
        PclExpressionData::Quote {
            quote: PclQuote::Full(id),
            ..
        } => Some(id),
        _ => None,
    }
}

fn step_clause(step: &PclStep) -> Option<&Clause> {
    match step.logic() {
        PclStepLogic::Clause(clause) => Some(clause),
        PclStepLogic::Shell | PclStepLogic::Formula(_) => None,
    }
}

#[allow(clippy::cast_precision_loss)]
fn c_lemma_quality(
    protocol: &PclProtocol,
    step: &PclStep,
    clause: &Clause,
    params: LemmaParams,
) -> f32 {
    let data = step.tree_data();
    let mut result = (1.0
        + params.tree_base_weight as f32
        + data.active_pm_refs as f32 * params.act_pm_w
        + data.other_generating_refs as f32 * params.o_gen_w
        + data.active_simpl_refs as f32 * params.act_simpl_w
        + data.passive_simpl_refs as f32 * params.pas_simpl_w)
        * (1.0 + data.proof_tree_size as f32)
        / (params.size_base_weight as f32 + clause.standard_weight() as f32);

    if clause.is_horn() {
        result *= params.horn_bonus;
    }

    let passive_only = (data.passive_simpl_refs != 0 || data.pure_quote_refs != 0)
        && data.active_pm_refs + data.other_generating_refs + data.active_simpl_refs == 0;
    if passive_only || clause.is_trivial(protocol.term_bank()) {
        result = 0.0;
    }

    result
}

fn lemma_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        expr_proof_size, protocol_flat_find_lemmas, protocol_rec_find_lemmas,
        protocol_seq_find_lemmas, protocol_update_refs, step_compute_lemma_weight, step_proof_size,
        InferenceWeights, LemmaParams,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::pcl2::idents::PclId;
    use crate::pcl2::protocol::PclProtocol;
    use crate::pcl2::steps::{PclStepParseOptions, PCL_IS_LEMMA, PCL_NO_WEIGHT};

    fn parse_id(source: &str) -> PclId {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclId::parse(&mut scanner).unwrap()
    }

    fn parse_protocol(source: &str) -> PclProtocol {
        let mut protocol = PclProtocol::new().unwrap();
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        scanner.set_format(IoFormat::Tptp);
        protocol
            .parse(
                &mut scanner,
                PclStepParseOptions {
                    problem_type: ProblemType::FirstOrder,
                    support_shell_pcl: true,
                    ..PclStepParseOptions::default()
                },
            )
            .unwrap();
        protocol
    }

    #[test]
    fn reference_counters_follow_c_operator_classes() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n\
             2 : : [++q(a)] : initial\n\
             3 : : [++r(a)] : pm(1,2)\n\
             4 : : [++s(a)] : rw(3,1)\n\
             5 : : [++t(a)] : cn(4)\n\
             6 : : [++u(a)] : 5",
        );

        protocol_update_refs(&mut protocol);

        let one = protocol.find_step(&parse_id("1")).unwrap().tree_data();
        assert_eq!(one.other_generating_refs, 1);
        assert_eq!(one.active_simpl_refs, 1);
        let two = protocol.find_step(&parse_id("2")).unwrap().tree_data();
        assert_eq!(two.active_pm_refs, 1);
        let three = protocol.find_step(&parse_id("3")).unwrap().tree_data();
        assert_eq!(three.passive_simpl_refs, 1);
        let four = protocol.find_step(&parse_id("4")).unwrap().tree_data();
        assert_eq!(four.passive_simpl_refs, 1);
        let five = protocol.find_step(&parse_id("5")).unwrap().tree_data();
        assert_eq!(five.pure_quote_refs, 1);
    }

    #[test]
    fn proof_size_uses_inference_weights_caches_and_zeroes_marked_lemmas() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n\
             2 : lemma : [++q(a)] : er(1)\n\
             3 : : [++r(a)] : pm(1,2)",
        );
        let weights = InferenceWeights::default();

        assert_eq!(
            step_proof_size(&mut protocol, &parse_id("3"), weights, false).unwrap(),
            4
        );
        assert_eq!(
            protocol
                .find_step(&parse_id("3"))
                .unwrap()
                .tree_data()
                .proof_tree_size,
            4
        );
        protocol.reset_tree_data(true);
        assert_eq!(
            step_proof_size(&mut protocol, &parse_id("3"), weights, true).unwrap(),
            2
        );
        assert_eq!(
            protocol
                .find_step(&parse_id("2"))
                .unwrap()
                .tree_data()
                .proof_tree_size,
            2
        );
    }

    #[test]
    fn missing_proof_size_reference_reports_diagnostic() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : 9");
        let just = protocol.find_step(&parse_id("1")).unwrap().just().clone();

        let error =
            expr_proof_size(&mut protocol, &just, InferenceWeights::default(), false).unwrap_err();

        assert!(error.message().contains("Reference to non-existing step"));
    }

    #[test]
    fn lemma_weight_applies_reference_proof_size_horn_and_simplification_gates() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n\
             2 : : [++q(a)] : er(1)\n\
             3 : : [++r(a)] : rw(2,1)",
        );
        protocol_update_refs(&mut protocol);
        let weights = InferenceWeights::default();
        let id = parse_id("1");
        let _ = step_proof_size(&mut protocol, &id, weights, true).unwrap();

        let quality = step_compute_lemma_weight(&mut protocol, &id, LemmaParams::default());

        assert!(quality > 0.0);
        assert_eq!(
            protocol
                .find_step(&id)
                .unwrap()
                .tree_data()
                .lemma_quality
                .to_bits(),
            quality.to_bits()
        );

        let passive_only = parse_id("2");
        let _ = step_proof_size(&mut protocol, &passive_only, weights, true).unwrap();
        assert_eq!(
            step_compute_lemma_weight(&mut protocol, &passive_only, LemmaParams::default())
                .to_bits(),
            0.0_f32.to_bits()
        );
    }

    #[test]
    fn sequential_selection_preserves_c_off_by_one_limit() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n\
             2 : : [++q(a)] : er(1)",
        );

        let selected = protocol_seq_find_lemmas(
            &mut protocol,
            LemmaParams::default(),
            InferenceWeights::default(),
            0,
            0.0,
        )
        .unwrap();

        assert_eq!(selected, 1);
        assert!(protocol
            .find_step(&parse_id("1"))
            .unwrap()
            .properties()
            .query(PCL_IS_LEMMA));
    }

    #[test]
    fn recursive_and_flat_selection_mark_highest_quality_steps() {
        let source = "1 : : [++p(a)] : initial\n\
                      2 : : [++q(a)] : initial\n\
                      3 : : [++r(a)] : pm(1,2)\n\
                      4 : : [++s(a)] : pm(1,3)\n\
                      5 : : [++t(a)] : er(4)";
        let mut rec_protocol = parse_protocol(source);
        let mut flat_protocol = parse_protocol(source);

        assert_eq!(
            protocol_rec_find_lemmas(
                &mut rec_protocol,
                LemmaParams::default(),
                InferenceWeights::default(),
                2,
                0.1,
            )
            .unwrap(),
            2
        );
        assert_eq!(
            protocol_flat_find_lemmas(
                &mut flat_protocol,
                LemmaParams::default(),
                InferenceWeights::default(),
                2,
                0.1,
            )
            .unwrap(),
            2
        );
        assert_eq!(rec_protocol.count_property(PCL_IS_LEMMA), 2);
        assert_eq!(flat_protocol.count_property(PCL_IS_LEMMA), 2);
        assert_ne!(
            rec_protocol
                .find_step(&parse_id("1"))
                .unwrap()
                .tree_data()
                .proof_tree_size,
            PCL_NO_WEIGHT
        );
    }
}
