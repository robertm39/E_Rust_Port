//! Initial port of `PCL2/pcl_proofcheck`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::clause::Clause;
use crate::clauses::clause_props::CP_TYPE_HYPOTHESIS;
use crate::clauses::clausesets::ClauseSet;
use crate::clauses::eqn::{eqn_string, Eqn, EqnPrintOptions};
use crate::clauses::eqnlist::EqnList;
use crate::inout::scanner::IoFormat;
use crate::pcl2::expressions::PclOpCode;
use crate::pcl2::idents::PclId;
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStep, PclStepLogic};
use crate::terms::functypes::FunCode;
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fmt::Write as _;

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

/// C `pcl_verify_eprover` problem-file body.
#[must_use]
pub fn eprover_problem_string(problem: &ClauseSet, bank: &TermBank) -> String {
    problem.print_tptp_format_string(bank)
}

/// C `clause_set_print_otter`.
#[must_use]
pub fn otter_clause_set_string(problem: &ClauseSet, bank: &TermBank) -> String {
    let mut output = String::new();
    for clause in problem.iter() {
        output.push_str(&otter_clause_string(clause, bank));
        output.push('\n');
    }
    output
}

/// C `pcl_verify_otter` problem-file body.
#[must_use]
pub fn otter_problem_string(problem: &ClauseSet, bank: &TermBank, time_limit: i64) -> String {
    let mut output = format!(
        "set(prolog_style_variables).\n\
         clear(print_kept).\n\
         clear(print_new_demod).\n\
         clear(print_back_demod).\n\
         clear(print_back_sub).\n\
         set(auto).\n\
         set(input_sos_first).\n\
         assign(max_seconds, {time_limit}).\n\n\
         assign(max_mem, 100000).\n\n\
         list(usable).\n\n\
         equal(X,X).\n",
    );
    output.push_str(&otter_clause_set_string(problem, bank));
    output.push_str("end_of_list.\n");
    output
}

/// C `sig_print_dfg`.
#[must_use]
pub fn dfg_signature_string(problem: &ClauseSet, signature: &Signature) -> String {
    let symbol_distribution = symbol_distribution(problem, signature);
    let mut output = String::from("list_of_symbols.\nfunctions[(spass_hack,0)");
    append_dfg_symbol_list(&mut output, signature, &symbol_distribution, false);
    output.push_str("].\npredicates[(spass_pred_dummy,0)");
    append_dfg_symbol_list(&mut output, signature, &symbol_distribution, true);
    output.push_str("].\nend_of_list.\n");
    output
}

/// C `clause_set_print_dfg`.
#[must_use]
pub fn dfg_clause_set_string(problem: &ClauseSet, bank: &TermBank) -> String {
    let mut output = String::new();
    for clause in problem.iter() {
        output.push_str(&dfg_clause_string(clause, bank));
        output.push('\n');
    }
    output
}

/// C `pcl_verify_spass` problem-file body.
#[must_use]
pub fn spass_problem_string(problem: &ClauseSet, bank: &TermBank, time_limit: i64) -> String {
    let mut output = String::from("begin_problem(Unknown).\n");
    output.push_str(&dfg_signature_string(problem, bank.signature()));
    output.push_str("list_of_clauses(axioms,cnf).\n");
    output.push_str(&dfg_clause_set_string(problem, bank));
    let _ = write!(
        output,
        "end_of_list.\n\
         list_of_settings(SPASS).\n\
         set_flag(TimeLimit, {time_limit}).\n\
         end_of_list.\n\
         end_problem.\n"
    );
    output
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

fn otter_clause_string(clause: &Clause, bank: &TermBank) -> String {
    if clause.is_empty() {
        return "$F.".to_owned();
    }

    let mut output = String::new();
    let mut literals = clause.literals().as_slice().iter();
    if let Some(first) = literals.next() {
        output.push_str(&otter_eqn_string(first, bank));
        for literal in literals {
            output.push_str("|\n");
            output.push_str(&otter_eqn_string(literal, bank));
        }
        output.push_str(".\n");
    }
    output
}

fn otter_eqn_string(literal: &Eqn, bank: &TermBank) -> String {
    if literal.is_equ_lit(bank) {
        if literal.is_positive() {
            return eqn_string(bank, literal, false, true, EqnPrintOptions::default());
        }
        return format!(
            "-{}",
            eqn_string(bank, literal, true, true, EqnPrintOptions::default())
        );
    }

    if literal.left() == bank.true_term() {
        debug_assert_eq!(literal.right(), bank.true_term());
        if literal.is_positive() {
            return "$T".to_owned();
        }
        return "$F".to_owned();
    }

    let mut output = String::new();
    output.push(if literal.is_negative() { '-' } else { ' ' });
    output.push_str(&bank.term_string(literal.left(), true));
    output
}

fn dfg_clause_string(clause: &Clause, bank: &TermBank) -> String {
    let mut output = String::from("clause(");
    let mut variables = BTreeMap::new();
    let variable_count = clause.collect_variables(&mut variables);
    if variable_count != 0 {
        let mut variables = variables.into_values().collect::<Vec<_>>();
        variables.sort_by_key(|variable| Reverse(variable.f_code()));
        output.push_str("forall([");
        for (index, variable) in variables.iter().enumerate() {
            if index != 0 {
                output.push(',');
            }
            output.push_str(&bank.term_string(variable, true));
        }
        output.push_str("],");
    }

    output.push_str("or(");
    let mut literals = clause.literals().as_slice().iter();
    if let Some(first) = literals.next() {
        output.push_str(&dfg_eqn_string(first, bank));
        for literal in literals {
            output.push(',');
            output.push_str(&dfg_eqn_string(literal, bank));
        }
    } else {
        output.push_str("not(equal(spass_hack,spass_hack))");
    }
    output.push(')');
    output.push(if variable_count != 0 { ')' } else { ' ' });
    let _ = write!(output, ", c{} ).", clause.ident());
    output
}

fn dfg_eqn_string(literal: &Eqn, bank: &TermBank) -> String {
    let mut output = String::new();
    if literal.is_negative() {
        output.push_str("not(");
    }
    if literal.left() == bank.true_term() {
        debug_assert_eq!(literal.right(), bank.true_term());
        output.push_str("equal(spass_hack,spass_hack)");
    } else {
        output.push_str(&eqn_string(
            bank,
            literal,
            literal.is_negative(),
            true,
            EqnPrintOptions {
                output_format: IoFormat::Lop,
                use_infix: false,
                ..EqnPrintOptions::default()
            },
        ));
    }
    if literal.is_negative() {
        output.push(')');
    }
    output
}

fn symbol_distribution(problem: &ClauseSet, signature: &Signature) -> Vec<i64> {
    let len = usize::try_from(signature.f_count() + 1).expect("signature size fits usize");
    let mut distribution = vec![0; len];
    for clause in problem.iter() {
        clause.add_symbol_distribution(&mut distribution);
    }
    distribution
}

fn append_dfg_symbol_list(
    output: &mut String,
    signature: &Signature,
    symbol_distribution: &[i64],
    predicates: bool,
) {
    for f_code in (signature.internal_symbols() + 1)..=signature.f_count() {
        if symbol_is_used(symbol_distribution, f_code)
            && signature.is_predicate(f_code) == predicates
        {
            let name = signature
                .find_name(f_code)
                .expect("valid f-code has a printable name");
            let arity = signature
                .find_arity(f_code)
                .expect("valid f-code has an arity");
            let _ = write!(output, ",({name},{arity})");
        }
    }
}

fn symbol_is_used(symbol_distribution: &[i64], f_code: FunCode) -> bool {
    usize::try_from(f_code)
        .ok()
        .and_then(|index| symbol_distribution.get(index))
        .is_some_and(|count| *count != 0)
}

fn proofcheck_error(message: &str) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYNTAX_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        collect_preconditions, dfg_clause_set_string, dfg_signature_string, eprover_problem_string,
        generate_check, neg_skolemize_clause, otter_clause_set_string, otter_problem_string,
        protocol_check, spass_problem_string, step_check, PclCheckType, ProverType,
    };
    use crate::basics::simple_stuff::ProblemType;
    use crate::clauses::clause::Clause;
    use crate::clauses::clausesets::ClauseSet;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
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
    fn eprover_problem_string_delegates_to_tptp_clause_set_rendering() {
        let mut protocol = parse_protocol("1 : : [++p(a),--q(a)] : initial\n2 : : [++r(a)] : 1");

        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        assert_eq!(
            eprover_problem_string(&problem, protocol.term_bank()),
            problem.print_tptp_format_string(protocol.term_bank())
        );
    }

    #[test]
    fn otter_problem_string_matches_c_header_and_clause_layout() {
        let mut protocol = parse_protocol("1 : : [++p(a),--q(a)] : initial\n2 : : [++r(a)] : 1");
        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        let rendered = otter_problem_string(&problem, protocol.term_bank(), 7);

        assert!(rendered.starts_with("set(prolog_style_variables).\nclear(print_kept).\n"));
        assert!(rendered.contains("assign(max_seconds, 7).\n\nassign(max_mem, 100000).\n\n"));
        assert!(rendered.contains("list(usable).\n\nequal(X,X).\n"));
        assert!(rendered.contains(" p(a)|\n-q(a).\n\n"));
        assert!(rendered.contains("-r(a).\n\n"));
        assert!(rendered.ends_with("end_of_list.\n"));
    }

    #[test]
    fn spass_problem_string_matches_c_dfg_wrapper_and_symbol_lists() {
        let mut protocol = parse_protocol("1 : : [++p(X),--q(X)] : initial\n2 : : [++r(a)] : 1");
        let problem = generate_check(&mut protocol, &parse_id("2"))
            .unwrap()
            .unwrap();

        let signature = dfg_signature_string(&problem, protocol.term_bank().signature());
        let clauses = dfg_clause_set_string(&problem, protocol.term_bank());
        let rendered = spass_problem_string(&problem, protocol.term_bank(), 11);

        assert!(signature.starts_with("list_of_symbols.\nfunctions[(spass_hack,0)"));
        assert!(signature.contains(",(a,0)"));
        assert!(signature.contains("predicates[(spass_pred_dummy,0)"));
        assert!(signature.contains(",(p,1)"));
        assert!(signature.contains(",(q,1)"));
        assert!(signature.contains(",(r,1)"));
        assert!(clauses.contains("forall([X1],or(p(X1),not(q(X1))))"));
        assert!(clauses.contains("or(not(r(a)))"));
        assert!(rendered.starts_with("begin_problem(Unknown).\nlist_of_symbols.\n"));
        assert!(rendered.contains("list_of_clauses(axioms,cnf).\n"));
        assert!(rendered.contains("set_flag(TimeLimit, 11).\n"));
        assert!(rendered.ends_with("end_problem.\n"));
    }

    #[test]
    fn otter_and_dfg_render_c_truth_literal_hacks() {
        let mut protocol = PclProtocol::new().unwrap();
        let true_term = protocol.term_bank().true_term().clone();
        let positive = Eqn::alloc(
            true_term.clone(),
            true_term.clone(),
            protocol.term_bank_mut(),
            true,
        )
        .unwrap();
        let negative = Eqn::alloc(
            true_term.clone(),
            true_term,
            protocol.term_bank_mut(),
            false,
        )
        .unwrap();
        let set =
            ClauseSet::from_clauses([Clause::alloc(EqnList::from_vec(vec![positive, negative]))]);

        assert_eq!(
            otter_clause_set_string(&set, protocol.term_bank()),
            "$T|\n$F.\n\n"
        );
        assert!(dfg_clause_set_string(&set, protocol.term_bank())
            .contains("or(equal(spass_hack,spass_hack),not(equal(spass_hack,spass_hack)))"));
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
