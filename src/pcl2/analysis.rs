//! Port of `PCL2/pcl_analysis`.

use std::cmp::Ordering;

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::pcl2::expressions::{PclExpression, PclExpressionData, PclOpCode, PclQuote};
use crate::pcl2::idents::PclId;
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{
    PclStep, PCL_IS_EXAMPLE, PCL_IS_FOF_STEP, PCL_IS_PROOF_STEP, PCL_PROOF_DIST_DEFAULT,
    PCL_PROOF_DIST_INFINITY, PCL_PROOF_DIST_UNKNOWN,
};

/// C `PCLExprProofDistance`.
///
/// # Errors
///
/// Returns a syntax diagnostic if a quoted parent is missing or if a mini
/// identifier appears in a full-protocol expression.
pub fn expr_proof_distance(
    protocol: &mut PclProtocol,
    expr: &PclExpression,
) -> Result<i64, Diagnostic> {
    match expr.data() {
        PclExpressionData::None => Err(analysis_error("Unknown PCL expression in analysis")),
        PclExpressionData::Initial(_) => Ok(PCL_PROOF_DIST_DEFAULT),
        PclExpressionData::Quote {
            quote: PclQuote::Full(id),
            ..
        } => step_proof_distance(protocol, id),
        PclExpressionData::Quote {
            quote: PclQuote::Mini(_),
            ..
        } => Err(analysis_error(
            "Mini PCL identifier found in full protocol expression",
        )),
        PclExpressionData::Compound(args) => {
            let mut distance = 0;
            for argument in args {
                distance = distance.max(expr_proof_distance(protocol, argument.expr())?);
            }
            if distance != PCL_PROOF_DIST_INFINITY {
                distance += 1;
            }
            Ok(distance)
        }
    }
}

/// C `PCLStepProofDistance`.
///
/// # Errors
///
/// Returns a syntax diagnostic if the step id or one of its ancestors is
/// missing.
pub fn step_proof_distance(protocol: &mut PclProtocol, id: &PclId) -> Result<i64, Diagnostic> {
    let Some(step) = protocol.find_step(id) else {
        return Err(analysis_error("Dangling reference in PCL protocol!"));
    };
    let cached = step.tree_data().proof_distance;
    if cached != PCL_PROOF_DIST_UNKNOWN {
        return Ok(cached);
    }

    let distance = if step.properties().query(PCL_IS_PROOF_STEP) {
        0
    } else {
        let just = step.just().clone();
        expr_proof_distance(protocol, &just)?
    };
    let Some(step) = protocol.find_step_mut(id) else {
        return Err(analysis_error("Dangling reference in PCL protocol!"));
    };
    step.tree_data_mut().proof_distance = distance;
    Ok(distance)
}

/// C `PCLProtProofDistance`.
///
/// # Errors
///
/// Returns a syntax diagnostic if a referenced parent is missing.
pub fn protocol_proof_distance(protocol: &mut PclProtocol) -> Result<(), Diagnostic> {
    for id in protocol.step_ids() {
        let _ = step_proof_distance(protocol, &id)?;
    }
    Ok(())
}

/// C `PCLExprUpdateGRefs`.
pub fn expr_update_grefs(protocol: &mut PclProtocol, expr: &PclExpression, proofstep: bool) {
    match expr.data() {
        PclExpressionData::Initial(_)
        | PclExpressionData::Quote { .. }
        | PclExpressionData::None => {}
        PclExpressionData::Compound(args) => match expr.op() {
            PclOpCode::Paramod
            | PclOpCode::SimParamod
            | PclOpCode::EResolution
            | PclOpCode::EFactoring
            | PclOpCode::SplitClause => {
                for argument in args {
                    update_generation_arg(protocol, argument.expr(), proofstep);
                }
            }
            PclOpCode::SimplifyReflect
            | PclOpCode::ContextSimplifyReflect
            | PclOpCode::Rewrite
            | PclOpCode::URewrite
            | PclOpCode::ACResolution => {
                if let Some((first, rest)) = args.split_first() {
                    expr_update_grefs(protocol, first.expr(), proofstep);
                    for argument in rest {
                        update_simplification_arg(protocol, argument.expr(), proofstep);
                    }
                }
            }
            _ => {}
        },
    }
}

/// C `PCLStepUpdateGRefs`.
pub fn step_update_grefs(protocol: &mut PclProtocol, id: &PclId) {
    let Some(step) = protocol.find_step(id) else {
        return;
    };
    let just = step.just().clone();
    let proofstep = step.properties().query(PCL_IS_PROOF_STEP);
    expr_update_grefs(protocol, &just, proofstep);
}

/// C `PCLProtUpdateGRefs`.
pub fn protocol_update_grefs(protocol: &mut PclProtocol) {
    for id in protocol.step_ids() {
        step_update_grefs(protocol, &id);
    }
}

/// C `PCLProtSelectExamples`.
#[must_use]
pub fn protocol_select_examples(protocol: &mut PclProtocol, mut neg_examples: i64) -> i64 {
    let mut ids = protocol.step_ids();
    ids.sort_by(|left, right| {
        compare_examples(protocol.find_step(left), protocol.find_step(right))
    });

    let mut visited = 0;
    for id in ids {
        if neg_examples <= 0 {
            break;
        }
        visited += 1;
        let Some(step) = protocol.find_step(&id) else {
            continue;
        };
        if step.properties().query(PCL_IS_FOF_STEP) {
            continue;
        }
        let was_proof = step.properties().query(PCL_IS_PROOF_STEP);
        if let Some(step) = protocol.find_step_mut(&id) {
            step.set_property(PCL_IS_EXAMPLE);
        }
        if !was_proof {
            neg_examples -= 1;
        }
    }
    visited
}

fn update_generation_arg(protocol: &mut PclProtocol, expr: &PclExpression, proofstep: bool) {
    if let Some(id) = full_quote_id(expr) {
        if let Some(parent) = protocol.find_step_mut(id) {
            if proofstep {
                parent.tree_data_mut().contrib_gen_refs += 1;
            } else {
                parent.tree_data_mut().useless_gen_refs += 1;
            }
        }
    } else {
        expr_update_grefs(protocol, expr, proofstep);
    }
}

fn update_simplification_arg(protocol: &mut PclProtocol, expr: &PclExpression, proofstep: bool) {
    if let Some(id) = full_quote_id(expr) {
        if let Some(parent) = protocol.find_step_mut(id) {
            if proofstep {
                parent.tree_data_mut().contrib_simpl_refs += 1;
            } else {
                parent.tree_data_mut().useless_simpl_refs += 1;
            }
        }
    } else {
        expr_update_grefs(protocol, expr, proofstep);
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

fn compare_examples(left: Option<&PclStep>, right: Option<&PclStep>) -> Ordering {
    let Some(left) = left else {
        return Ordering::Equal;
    };
    let Some(right) = right else {
        return Ordering::Equal;
    };

    let left_proof = left.properties().query(PCL_IS_PROOF_STEP);
    let right_proof = right.properties().query(PCL_IS_PROOF_STEP);
    match (left_proof, right_proof) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        (false, false) => {}
    }

    let left_data = left.tree_data();
    let right_data = right.tree_data();
    let left_weight = c_example_weight(left_data.useless_gen_refs, left_data.useless_simpl_refs);
    let right_weight = c_example_weight(right_data.useless_gen_refs, right_data.useless_simpl_refs);
    right_weight
        .partial_cmp(&left_weight)
        .unwrap_or(Ordering::Equal)
}

#[allow(clippy::cast_precision_loss)]
fn c_example_weight(gen_refs: i64, simpl_refs: i64) -> f32 {
    gen_refs as f32 / (simpl_refs + 1) as f32
}

fn analysis_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        c_example_weight, protocol_proof_distance, protocol_select_examples, protocol_update_grefs,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::simple_stuff::ProblemType;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::pcl2::idents::PclId;
    use crate::pcl2::protocol::PclProtocol;
    use crate::pcl2::steps::{
        PclStepParseOptions, PCL_IS_EXAMPLE, PCL_IS_PROOF_STEP, PCL_PROOF_DIST_DEFAULT,
    };

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
    fn proof_distance_marks_proofs_initials_quotes_and_compound_chains() {
        let mut protocol = parse_protocol(
            "1 : : [++p] : initial : 'final'\n2 : : [++q] : 1\n3 : : [++r] : pm(2,2)\n4 : : [++s] : initial",
        );
        assert!(!protocol.mark_proof_clauses().unwrap());

        protocol_proof_distance(&mut protocol).unwrap();

        assert_eq!(
            protocol
                .find_step(&parse_id("1"))
                .unwrap()
                .tree_data()
                .proof_distance,
            0
        );
        assert_eq!(
            protocol
                .find_step(&parse_id("2"))
                .unwrap()
                .tree_data()
                .proof_distance,
            0
        );
        assert_eq!(
            protocol
                .find_step(&parse_id("3"))
                .unwrap()
                .tree_data()
                .proof_distance,
            1
        );
        assert_eq!(
            protocol
                .find_step(&parse_id("4"))
                .unwrap()
                .tree_data()
                .proof_distance,
            PCL_PROOF_DIST_DEFAULT
        );
    }

    #[test]
    fn generation_and_simplification_reference_counters_follow_c_operator_classes() {
        let mut protocol = parse_protocol(
            "1 : : [++p] : initial\n2 : : [++q] : initial\n3 : : [++r] : pm(1,2) : 'final'\n4 : : [++s] : sr(3,2)",
        );
        assert!(!protocol.mark_proof_clauses().unwrap());

        protocol_update_grefs(&mut protocol);

        let first = protocol.find_step(&parse_id("1")).unwrap();
        assert_eq!(first.tree_data().contrib_gen_refs, 1);
        assert_eq!(first.tree_data().useless_gen_refs, 0);

        let second = protocol.find_step(&parse_id("2")).unwrap();
        assert_eq!(second.tree_data().contrib_gen_refs, 1);
        assert_eq!(second.tree_data().useless_simpl_refs, 1);

        let third = protocol.find_step(&parse_id("3")).unwrap();
        assert_eq!(third.tree_data().contrib_simpl_refs, 0);
        assert_eq!(third.tree_data().useless_simpl_refs, 0);
    }

    #[test]
    fn select_examples_prefers_proofs_then_high_generation_to_simplification_ratio() {
        let mut protocol = parse_protocol(
            "1 : : [++p] : initial : 'proof'\n2 : : [++q] : initial\n3 : : [++r] : initial",
        );
        assert!(!protocol.mark_proof_clauses().unwrap());
        protocol
            .find_step_mut(&parse_id("2"))
            .unwrap()
            .tree_data_mut()
            .useless_gen_refs = 1;
        protocol
            .find_step_mut(&parse_id("3"))
            .unwrap()
            .tree_data_mut()
            .useless_gen_refs = 5;

        assert_eq!(protocol_select_examples(&mut protocol, 1), 2);

        assert!(protocol
            .find_step(&parse_id("1"))
            .unwrap()
            .properties()
            .query(PCL_IS_EXAMPLE | PCL_IS_PROOF_STEP));
        assert!(!protocol
            .find_step(&parse_id("2"))
            .unwrap()
            .properties()
            .query(PCL_IS_EXAMPLE));
        assert!(protocol
            .find_step(&parse_id("3"))
            .unwrap()
            .properties()
            .query(PCL_IS_EXAMPLE));
    }

    #[test]
    fn select_examples_with_zero_negative_budget_matches_c_loop_condition() {
        let mut protocol = parse_protocol("1 : : [++p] : initial : 'proof'\n2 : : [++q] : initial");
        assert!(!protocol.mark_proof_clauses().unwrap());

        assert_eq!(protocol_select_examples(&mut protocol, 0), 0);
        assert_eq!(protocol.count_property(PCL_IS_EXAMPLE), 0);
    }

    #[test]
    fn dangling_proof_distance_parent_is_a_diagnostic_not_c_null_dereference() {
        let mut protocol = parse_protocol("1 : : [++p] : 99");

        let error = protocol_proof_distance(&mut protocol)
            .expect_err("a missing proof-distance parent must be rejected safely");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(error.message(), "Dangling reference in PCL protocol!");
    }

    #[test]
    fn dangling_generation_parent_remains_a_silent_c_shaped_no_op() {
        let mut protocol = parse_protocol("1 : : [++p] : initial\n2 : : [++q] : pm(99,1)");

        protocol_update_grefs(&mut protocol);

        assert_eq!(
            protocol
                .find_step(&parse_id("1"))
                .expect("live parent remains present")
                .tree_data()
                .useless_gen_refs,
            1
        );
        assert_eq!(
            protocol
                .find_step(&parse_id("2"))
                .expect("derived step remains present")
                .tree_data()
                .useless_gen_refs,
            0
        );
    }

    #[test]
    fn equal_example_scores_use_deterministic_pcl_id_order() {
        let mut protocol = parse_protocol(
            "2 : : [++p2] : initial\n1 : : [++p1] : initial\n100 : : [++goal] : initial : 'final'",
        );
        assert!(!protocol.mark_proof_clauses().unwrap());

        assert_eq!(protocol_select_examples(&mut protocol, 1), 2);

        assert!(protocol
            .find_step(&parse_id("1"))
            .expect("first tied id remains present")
            .properties()
            .query(PCL_IS_EXAMPLE));
        assert!(!protocol
            .find_step(&parse_id("2"))
            .expect("second tied id remains present")
            .properties()
            .query(PCL_IS_EXAMPLE));
    }

    #[test]
    fn example_score_rounding_matches_c_float_division() {
        let exactly_representable = c_example_weight(16_777_216, 0);
        let rounded = c_example_weight(16_777_217, 0);

        assert_eq!(exactly_representable.to_bits(), rounded.to_bits());
        assert_eq!(rounded.to_bits(), 16_777_216.0_f32.to_bits());
    }
}
