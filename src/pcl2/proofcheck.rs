//! Initial port of `PCL2/pcl_proofcheck`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_HYPOTHESIS;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::Eqn;
use crate::clauses::eqnlist::EqnList;
use crate::pcl2::expressions::PclOpCode;
use crate::pcl2::idents::PclId;
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStep, PclStepLogic};

pub const E_EXEC_DEFAULT: &str = "eprover";
pub const OTTER_EXEC_DEFAULT: &str = "otter";
pub const SPASS_EXEC_DEFAULT: &str = "SPASS-0.55";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PclCheckType {
    Fail,
    Ok,
    ByAssumption,
    NotImplemented,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProverType {
    NoProver,
    EProver,
    Spass,
    Setheo,
    Otter,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PclCheckSummary {
    pub checked: i64,
    pub unchecked: i64,
}

/// C `PCLCollectPreconds`.
///
/// # Errors
///
/// Returns diagnostics for dangling full-protocol references, mini identifiers,
/// or clause copy failures.
pub fn collect_preconditions(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    set: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    let Some(step) = protocol.find_step(step_id) else {
        return Err(proofcheck_error("PCL proofcheck step not found"));
    };
    let parent_ids = protocol.collect_preconditions(step.just())?;
    let mut count = 0;

    for parent_id in parent_ids {
        let Some(clause) = protocol
            .find_step(&parent_id)
            .and_then(step_clause)
            .cloned()
        else {
            continue;
        };
        let copied = clause.copy_to_bank(protocol.term_bank_mut())?;
        set.insert(copied);
        count += 1;
    }
    Ok(count)
}

/// C `PCLNegSkolemizeClause`.
///
/// # Errors
///
/// Returns diagnostics from skolemization or literal allocation.
pub fn neg_skolemize_clause(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    set: &mut ClauseSet,
) -> Result<i64, Diagnostic> {
    let Some(clause) = protocol.find_step(step_id).and_then(step_clause).cloned() else {
        return Ok(0);
    };
    let skolemized = clause.skolemize(protocol.term_bank_mut())?;
    let mut count = 0;

    for literal in skolemized.literals().as_slice() {
        let flipped = Eqn::alloc(
            literal.left().clone(),
            literal.right().clone(),
            protocol.term_bank_mut(),
            !literal.is_positive(),
        )?;
        let mut new_clause = Clause::alloc(EqnList::from_vec(vec![flipped]));
        new_clause.set_tptp_type(CP_TYPE_HYPOTHESIS);
        set.insert(new_clause);
        count += 1;
    }
    Ok(count)
}

/// C `PCLGenerateCheck`.
///
/// Returns `Ok(None)` for assumption/initial steps with no clausal
/// preconditions.
///
/// # Errors
///
/// Returns diagnostics from precondition collection, clause copying, or
/// skolemization.
pub fn generate_check(
    protocol: &mut PclProtocol,
    step_id: &PclId,
) -> Result<Option<ClauseSet>, Diagnostic> {
    let mut set = ClauseSet::new();
    if collect_preconditions(protocol, step_id, &mut set)? == 0 {
        return Ok(None);
    }
    let _ = neg_skolemize_clause(protocol, step_id, &mut set)?;
    Ok(Some(set))
}

/// Initial C `PCLStepCheck` port.
///
/// External prover execution is intentionally reported as not implemented in
/// this slice; generated check-problem construction is available through
/// [`generate_check`].
///
/// # Errors
///
/// Returns diagnostics from check-problem generation.
pub fn step_check(
    protocol: &mut PclProtocol,
    step_id: &PclId,
    prover: ProverType,
    _executable: Option<&str>,
    _time_limit: i64,
) -> Result<PclCheckType, Diagnostic> {
    let Some(step) = protocol.find_step(step_id) else {
        return Err(proofcheck_error("PCL proofcheck step not found"));
    };
    if step.just().op() == PclOpCode::SplitClause {
        return Ok(PclCheckType::NotImplemented);
    }

    if generate_check(protocol, step_id)?.is_none() {
        return Ok(PclCheckType::ByAssumption);
    }

    match prover {
        ProverType::NoProver
        | ProverType::EProver
        | ProverType::Spass
        | ProverType::Setheo
        | ProverType::Otter => Ok(PclCheckType::NotImplemented),
    }
}

/// Initial C `PCLProtCheck` port.
///
/// # Errors
///
/// Returns diagnostics from step-level check generation.
pub fn protocol_check(
    protocol: &mut PclProtocol,
    prover: ProverType,
    executable: Option<&str>,
    time_limit: i64,
) -> Result<PclCheckSummary, Diagnostic> {
    let mut summary = PclCheckSummary::default();
    for id in protocol.step_ids() {
        match step_check(protocol, &id, prover, executable, time_limit)? {
            PclCheckType::ByAssumption | PclCheckType::Ok => summary.checked += 1,
            PclCheckType::NotImplemented => summary.unchecked += 1,
            PclCheckType::Fail => {}
        }
    }
    Ok(summary)
}

fn step_clause(step: &PclStep) -> Option<&Clause> {
    match step.logic() {
        PclStepLogic::Clause(clause) => Some(clause),
        PclStepLogic::Shell | PclStepLogic::Formula(_) => None,
    }
}

fn proofcheck_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_preconditions, generate_check, neg_skolemize_clause, protocol_check, step_check,
        PclCheckType, ProverType,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::inout::scanner::{IoFormat, Scanner};
    use crate::pcl2::idents::PclId;
    use crate::pcl2::protocol::PclProtocol;
    use crate::pcl2::steps::PclStepParseOptions;

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
                },
            )
            .unwrap();
        protocol
    }

    #[test]
    fn collect_preconditions_copies_unique_clausal_parent_clauses() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n\
             2 : : q(a) : initial\n\
             3 : : [++r(a)] : pm(1,2)\n\
             4 : : [++s(a)] : pm(1,3)",
        );
        let mut set = ClauseSet::new();

        let count = collect_preconditions(&mut protocol, &parse_id("4"), &mut set).unwrap();

        assert_eq!(count, 2);
        assert_eq!(set.members(), 2);
        assert!(set.iter().all(|clause| !clause.is_empty()));
    }

    #[test]
    fn neg_skolemize_clause_adds_one_flipped_hypothesis_unit_per_literal() {
        let mut protocol = parse_protocol("1 : : [++p(X),--q(a)] : initial\n2 : : [++r(a)] : 1");
        let mut set = ClauseSet::new();

        let count = neg_skolemize_clause(&mut protocol, &parse_id("1"), &mut set).unwrap();

        assert_eq!(count, 2);
        assert_eq!(set.members(), 2);
        let clauses = set.iter().collect::<Vec<_>>();
        assert!(clauses.iter().all(|clause| clause.is_hypothesis()));
        assert_eq!(
            clauses
                .iter()
                .map(|clause| clause.literal_number())
                .collect::<Vec<_>>(),
            [1, 1]
        );
        assert_eq!(
            clauses
                .iter()
                .filter(|clause| clause.literals().as_slice()[0].is_positive())
                .count(),
            1
        );
    }

    #[test]
    fn generate_check_combines_copied_preconditions_and_negated_goal_units() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : initial\n2 : : [++q(a),--r(a)] : 1");

        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        assert_eq!(problem.members(), 3);
        assert_eq!(
            problem
                .iter()
                .map(Clause::literal_number)
                .collect::<Vec<_>>(),
            [1, 1, 1]
        );
    }

    #[test]
    fn generate_check_returns_none_for_assumption_without_clausal_preconditions() {
        let mut protocol = parse_protocol("1 : : [++p(a)] : initial");

        assert!(generate_check(&mut protocol, &parse_id("1"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn step_and_protocol_check_report_assumptions_and_unimplemented_external_checks() {
        let mut protocol = parse_protocol(
            "1 : : [++p(a)] : initial\n2 : : [++q(a)] : 1\n3 : : [++r(a)] : split(2)",
        );

        assert_eq!(
            step_check(
                &mut protocol,
                &parse_id("1"),
                ProverType::NoProver,
                None,
                10
            )
            .unwrap(),
            PclCheckType::ByAssumption
        );
        assert_eq!(
            step_check(
                &mut protocol,
                &parse_id("2"),
                ProverType::NoProver,
                None,
                10
            )
            .unwrap(),
            PclCheckType::NotImplemented
        );
        assert_eq!(
            step_check(
                &mut protocol,
                &parse_id("3"),
                ProverType::NoProver,
                None,
                10
            )
            .unwrap(),
            PclCheckType::NotImplemented
        );

        let summary = protocol_check(&mut protocol, ProverType::NoProver, None, 10).unwrap();
        assert_eq!(summary.checked, 1);
        assert_eq!(summary.unchecked, 2);
    }
}
