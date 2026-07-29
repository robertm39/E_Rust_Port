//! Experiment-only SAT encoding for first-order subsumption variants.
//!
//! This module is compiled into a disposable runner worktree by
//! `capture.patch`. It is not part of the production module graph.

use crate::basics::simple_stuff::{problem_type, ProblemType};
use crate::clauses::clause::Clause;
use crate::clauses::eqn::Eqn;
use crate::clauses::satservice::{
    IncrementalSatService, InternalSatService, SatSolveOptions, SatSolveOutcome,
};
use crate::terms::match_mgu::subst_match_complete;
use crate::terms::subst::Substitution;
use crate::terms::termtypes::Term;
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Binding {
    variable: i64,
    target: Term,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Choice {
    variable: i32,
    source: usize,
    target: usize,
    negative: bool,
    reversed: bool,
    bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evaluation {
    pub ordinary: bool,
    pub resolution: bool,
    pub match_ns: u128,
    pub ordinary_solve_ns: u128,
    pub resolution_solve_ns: u128,
    pub positive_choices: usize,
    pub negative_choices: usize,
    pub binding_count: usize,
    pub ordinary_clause_count: usize,
    pub ordinary_literal_count: usize,
    pub resolution_clause_count: usize,
    pub resolution_literal_count: usize,
}

fn snapshot_bindings(substitution: &Substitution) -> Vec<Binding> {
    let mut bindings = substitution
        .bindings()
        .iter()
        .filter_map(|variable| {
            variable.binding().map(|target| Binding {
                variable: variable.f_code(),
                target,
            })
        })
        .collect::<Vec<_>>();
    bindings.sort_by_key(|binding| binding.variable);
    bindings
}

fn directed_match(pattern: &Eqn, candidate: &Eqn, reverse_candidate: bool) -> Option<Vec<Binding>> {
    let mut substitution = Substitution::new();
    let (candidate_left, candidate_right) = if reverse_candidate {
        (candidate.right(), candidate.left())
    } else {
        (candidate.left(), candidate.right())
    };
    let matched = subst_match_complete(pattern.left(), candidate_left, &mut substitution)
        && subst_match_complete(pattern.right(), candidate_right, &mut substitution);
    let bindings = matched.then(|| snapshot_bindings(&substitution));
    substitution.backtrack();
    bindings
}

fn literal_choices(
    source: usize,
    target: usize,
    pattern: &Eqn,
    candidate: &Eqn,
    bank: &crate::terms::termbanks::TermBank,
) -> Vec<Choice> {
    if pattern.is_equ_lit(bank) != candidate.is_equ_lit(bank)
        || (pattern.is_oriented() && !candidate.is_oriented())
    {
        return Vec::new();
    }
    let negative = pattern.is_positive() != candidate.is_positive();
    let mut choices = Vec::new();
    for reversed in [false, true] {
        if reversed && pattern.is_oriented() {
            continue;
        }
        let Some(bindings) = directed_match(pattern, candidate, reversed) else {
            continue;
        };
        let duplicate = choices.iter().any(|choice: &Choice| {
            choice.negative == negative && choice.target == target && choice.bindings == bindings
        });
        if !duplicate {
            choices.push(Choice {
                variable: 0,
                source,
                target,
                negative,
                reversed,
                bindings,
            });
        }
    }
    choices
}

fn build_choices(
    side: &Clause,
    main: &Clause,
    bank: &crate::terms::termbanks::TermBank,
) -> Vec<Choice> {
    let mut choices = Vec::new();
    for (source, pattern) in side.literals().as_slice().iter().enumerate() {
        for (target, candidate) in main.literals().as_slice().iter().enumerate() {
            choices.extend(literal_choices(source, target, pattern, candidate, bank));
        }
    }
    for (index, choice) in choices.iter_mut().enumerate() {
        choice.variable = i32::try_from(index + 1).expect("choice count exceeds SAT literal range");
    }
    choices
}

fn compatible(left: &Choice, right: &Choice) -> bool {
    left.bindings.iter().all(|left_binding| {
        right
            .bindings
            .iter()
            .find(|right_binding| right_binding.variable == left_binding.variable)
            .is_none_or(|right_binding| right_binding.target == left_binding.target)
    })
}

fn add_at_most_one(clauses: &mut Vec<Vec<i32>>, variables: &[i32]) {
    for left in 0..variables.len() {
        for right in (left + 1)..variables.len() {
            clauses.push(vec![-variables[left], -variables[right]]);
        }
    }
}

fn add_compatibility(clauses: &mut Vec<Vec<i32>>, choices: &[Choice]) {
    for left in 0..choices.len() {
        for right in (left + 1)..choices.len() {
            if !compatible(&choices[left], &choices[right]) {
                clauses.push(vec![-choices[left].variable, -choices[right].variable]);
            }
        }
    }
}

fn ordinary_cnf(source_count: usize, target_count: usize, choices: &[Choice]) -> Vec<Vec<i32>> {
    let mut clauses = Vec::new();
    for source in 0..source_count {
        let variables = choices
            .iter()
            .filter(|choice| choice.source == source && !choice.negative)
            .map(|choice| choice.variable)
            .collect::<Vec<_>>();
        clauses.push(variables.clone());
        add_at_most_one(&mut clauses, &variables);
    }
    for target in 0..target_count {
        let variables = choices
            .iter()
            .filter(|choice| choice.target == target && !choice.negative)
            .map(|choice| choice.variable)
            .collect::<Vec<_>>();
        add_at_most_one(&mut clauses, &variables);
    }
    add_compatibility(&mut clauses, choices);
    clauses
}

fn resolution_cnf(source_count: usize, target_count: usize, choices: &[Choice]) -> Vec<Vec<i32>> {
    let mut clauses = Vec::new();
    for source in 0..source_count {
        let variables = choices
            .iter()
            .filter(|choice| choice.source == source)
            .map(|choice| choice.variable)
            .collect::<Vec<_>>();
        clauses.push(variables.clone());
        add_at_most_one(&mut clauses, &variables);
    }

    clauses.push(
        choices
            .iter()
            .filter(|choice| choice.negative)
            .map(|choice| choice.variable)
            .collect(),
    );

    for left in choices.iter().filter(|choice| choice.negative) {
        for right in choices
            .iter()
            .filter(|choice| choice.negative && choice.target != left.target)
        {
            if left.variable < right.variable {
                clauses.push(vec![-left.variable, -right.variable]);
            }
        }
    }

    for target in 0..target_count {
        let positive = choices
            .iter()
            .filter(|choice| choice.target == target && !choice.negative);
        let negative = choices
            .iter()
            .filter(|choice| choice.target == target && choice.negative)
            .collect::<Vec<_>>();
        for positive_choice in positive {
            for negative_choice in &negative {
                clauses.push(vec![-positive_choice.variable, -negative_choice.variable]);
            }
        }
    }
    add_compatibility(&mut clauses, choices);
    clauses
}

fn solve_cnf(clauses: &[Vec<i32>]) -> bool {
    let mut solver = InternalSatService::default();
    for clause in clauses {
        solver
            .add_clause(clause)
            .expect("experiment encoder emitted an invalid SAT literal");
    }
    match solver.solve(&[], &SatSolveOptions::default()) {
        SatSolveOutcome::Sat { .. } => true,
        SatSolveOutcome::Unsat { .. } => false,
        SatSolveOutcome::Unknown(reason) => {
            panic!("unlimited internal SAT solve returned unknown: {reason:?}")
        }
        SatSolveOutcome::Error(error) => panic!("internal SAT solve failed: {error}"),
    }
}

fn cnf_literal_count(clauses: &[Vec<i32>]) -> usize {
    clauses.iter().map(Vec::len).sum()
}

/// Evaluates the experimental SAT encodings for a clause pair.
///
/// # Panics
///
/// Panics outside first-order mode or if the internal SAT service returns an
/// unknown result or an error.
#[must_use]
pub fn evaluate(
    side: &Clause,
    main: &Clause,
    bank: &crate::terms::termbanks::TermBank,
) -> Evaluation {
    assert_eq!(
        problem_type(),
        ProblemType::FirstOrder,
        "experiment encoding supports first-order clauses only"
    );
    let match_started = Instant::now();
    let choices = build_choices(side, main, bank);
    let match_ns = match_started.elapsed().as_nanos();

    let ordinary_started = Instant::now();
    let ordinary_clauses = ordinary_cnf(side.literal_number(), main.literal_number(), &choices);
    let ordinary = solve_cnf(&ordinary_clauses);
    let ordinary_solve_ns = ordinary_started.elapsed().as_nanos();

    let resolution_started = Instant::now();
    let resolution_clauses = resolution_cnf(side.literal_number(), main.literal_number(), &choices);
    let resolution = solve_cnf(&resolution_clauses);
    let resolution_solve_ns = resolution_started.elapsed().as_nanos();

    Evaluation {
        ordinary,
        resolution,
        match_ns,
        ordinary_solve_ns,
        resolution_solve_ns,
        positive_choices: choices.iter().filter(|choice| !choice.negative).count(),
        negative_choices: choices.iter().filter(|choice| choice.negative).count(),
        binding_count: choices.iter().map(|choice| choice.bindings.len()).sum(),
        ordinary_clause_count: ordinary_clauses.len(),
        ordinary_literal_count: cnf_literal_count(&ordinary_clauses),
        resolution_clause_count: resolution_clauses.len(),
        resolution_literal_count: cnf_literal_count(&resolution_clauses),
    }
}

static ELIGIBLE_CALLS: AtomicU64 = AtomicU64::new(0);
static WRITTEN_CALLS: AtomicU64 = AtomicU64::new(0);
static CAPTURE_FILE: OnceLock<Option<Mutex<File>>> = OnceLock::new();

fn capture_file() -> Option<&'static Mutex<File>> {
    CAPTURE_FILE
        .get_or_init(|| {
            let path = std::env::var_os("UMLAUT_SAT_SUBSUMPTION_CAPTURE")?;
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("cannot open SAT-subsumption capture path");
            Some(Mutex::new(file))
        })
        .as_ref()
}

fn capture_limit() -> u64 {
    std::env::var("UMLAUT_SAT_SUBSUMPTION_CAPTURE_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2_048)
}

fn should_capture(side_literals: usize, main_literals: usize) -> Option<u64> {
    capture_file()?;
    let ordinal = ELIGIBLE_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    let selected = ordinal <= 256
        || ordinal.is_multiple_of(997)
        || (side_literals >= 4 && main_literals >= 6 && ordinal.is_multiple_of(31));
    if !selected {
        return None;
    }
    let written = WRITTEN_CALLS.fetch_add(1, Ordering::Relaxed);
    (written < capture_limit()).then_some(ordinal)
}

fn term_code(term: &Term, output: &mut String) {
    if term.is_free_var() {
        output.push('v');
        output.push_str(&term.f_code().to_string());
        return;
    }
    if term.is_db_var() {
        output.push('d');
        output.push_str(&term.f_code().to_string());
        return;
    }
    output.push('f');
    output.push_str(&term.f_code().to_string());
    if term.arity() == 0 {
        return;
    }
    output.push('(');
    for index in 0..term.arity() {
        if index != 0 {
            output.push(',');
        }
        let argument = term
            .argument(index)
            .unwrap_or_else(|| panic!("captured term argument {index} is uninitialized"));
        term_code(&argument, output);
    }
    output.push(')');
}

fn clause_code(clause: &Clause) -> String {
    let mut output = String::new();
    for (index, literal) in clause.literals().as_slice().iter().enumerate() {
        if index != 0 {
            output.push('|');
        }
        output.push(if literal.is_positive() { '+' } else { '-' });
        output.push(if literal.is_oriented() { '>' } else { '=' });
        term_code(literal.left(), &mut output);
        output.push(':');
        term_code(literal.right(), &mut output);
    }
    output
}

fn variable_count(clause: &Clause) -> usize {
    fn collect(term: &Term, variables: &mut BTreeSet<i64>) {
        if term.is_free_var() {
            variables.insert(term.f_code());
            return;
        }
        for index in 0..term.arity() {
            if let Some(argument) = term.argument(index) {
                collect(&argument, variables);
            }
        }
    }

    let mut variables = BTreeSet::new();
    for literal in clause.literals().as_slice() {
        collect(literal.left(), &mut variables);
        collect(literal.right(), &mut variables);
    }
    variables.len()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write as _;
                write!(output, "\\u{:04x}", u32::from(character))
                    .expect("writing to String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

/// Records one sampled comparison between the baseline and experimental checks.
///
/// # Panics
///
/// Panics if the configured capture file cannot be opened, locked, written, or
/// flushed, or if experimental SAT evaluation fails.
pub fn record_capture(
    side: &Clause,
    main: &Clause,
    bank: &crate::terms::termbanks::TermBank,
    baseline: bool,
    baseline_ns: u128,
) {
    if problem_type() != ProblemType::FirstOrder
        || side.literal_number() < 2
        || main.literal_number() < 2
    {
        return;
    }
    let Some(ordinal) = should_capture(side.literal_number(), main.literal_number()) else {
        return;
    };
    let evaluation = evaluate(side, main, bank);
    let side_code = clause_code(side);
    let main_code = clause_code(main);
    let payload = format!("{side_code}\n{main_code}");
    let digest = format!("{:016x}", fnv1a64(payload.as_bytes()));
    let problem =
        std::env::var("UMLAUT_SAT_SUBSUMPTION_PROBLEM").unwrap_or_else(|_| "unknown".to_owned());
    let line = format!(
        concat!(
            "{{\"schema_version\":1,\"problem\":{},\"ordinal\":{},",
            "\"digest\":\"{}\",\"side\":{},\"main\":{},",
            "\"side_literals\":{},\"main_literals\":{},",
            "\"side_variables\":{},\"main_variables\":{},",
            "\"baseline\":{},\"ordinary\":{},\"resolution\":{},",
            "\"baseline_ns\":{},\"match_ns\":{},\"ordinary_solve_ns\":{},",
            "\"resolution_solve_ns\":{},\"positive_choices\":{},",
            "\"negative_choices\":{},\"binding_count\":{},",
            "\"ordinary_clause_count\":{},\"ordinary_literal_count\":{},",
            "\"resolution_clause_count\":{},\"resolution_literal_count\":{}}}\n"
        ),
        json_string(&problem),
        ordinal,
        digest,
        json_string(&side_code),
        json_string(&main_code),
        side.literal_number(),
        main.literal_number(),
        variable_count(side),
        variable_count(main),
        baseline,
        evaluation.ordinary,
        evaluation.resolution,
        baseline_ns,
        evaluation.match_ns,
        evaluation.ordinary_solve_ns,
        evaluation.resolution_solve_ns,
        evaluation.positive_choices,
        evaluation.negative_choices,
        evaluation.binding_count,
        evaluation.ordinary_clause_count,
        evaluation.ordinary_literal_count,
        evaluation.resolution_clause_count,
        evaluation.resolution_literal_count,
    );
    let mut file = capture_file()
        .expect("capture file disappeared")
        .lock()
        .expect("capture file mutex is poisoned");
    file.write_all(line.as_bytes())
        .expect("cannot write SAT-subsumption capture");
    file.flush().expect("cannot flush SAT-subsumption capture");
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use crate::basics::simple_stuff::{set_problem_type, ProblemType};
    use crate::clauses::clause::Clause;
    use crate::clauses::eqn::Eqn;
    use crate::clauses::eqnlist::EqnList;
    use crate::clauses::subsumption::{clause_subsume_order_sort_lits, clause_subsumes_clause};
    use crate::terms::signature::Signature;
    use crate::terms::termbanks::TermBank;
    use crate::terms::termtypes::Term;
    use crate::terms::typebanks::TypeBank;
    use crate::test_support::global_state_lock;

    fn test_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    fn constant(bank: &mut TermBank, name: &str) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        let code = bank.signature_mut().insert_id(name, 0, false);
        if bank.signature().get_type(code).is_none() {
            bank.signature_mut()
                .declare_final_type(code, type_)
                .unwrap();
        }
        bank.create_const_term(code).unwrap()
    }

    fn variable(bank: &TermBank, code: i64) -> Term {
        let type_ = bank.signature().type_bank().default_type();
        bank.vars().var_assert_alloc(code, &type_)
    }

    fn literal(bank: &mut TermBank, left: &Term, right: &Term, positive: bool) -> Eqn {
        Eqn::alloc(left.clone(), right.clone(), bank, positive).unwrap()
    }

    fn clause(literals: Vec<Eqn>) -> Clause {
        Clause::alloc(EqnList::from_vec(literals))
    }

    fn prepare(clause: &mut Clause, bank: &TermBank) {
        clause.set_weight(clause.standard_weight());
        clause_subsume_order_sort_lits(clause, bank);
    }

    #[test]
    fn ordinary_encoding_agrees_on_shared_bindings_and_multiplicity() {
        let _guard = global_state_lock();
        set_problem_type(ProblemType::FirstOrder).unwrap();
        let mut bank = test_bank();
        let x = variable(&bank, -10);
        let y = variable(&bank, -12);
        let a = constant(&mut bank, "sat_sub_a");
        let b = constant(&mut bank, "sat_sub_b");
        let c = constant(&mut bank, "sat_sub_c");
        let d = constant(&mut bank, "sat_sub_d");

        let mut side = clause(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &x, &b, true),
        ]);
        let mut matching = clause(vec![
            literal(&mut bank, &c, &a, true),
            literal(&mut bank, &c, &b, true),
        ]);
        let mut conflicting = clause(vec![
            literal(&mut bank, &c, &a, true),
            literal(&mut bank, &d, &b, true),
        ]);
        prepare(&mut side, &bank);
        prepare(&mut matching, &bank);
        prepare(&mut conflicting, &bank);

        assert!(clause_subsumes_clause(&side, &matching, &bank));
        assert!(evaluate(&side, &matching, &bank).ordinary);
        assert!(!clause_subsumes_clause(&side, &conflicting, &bank));
        assert!(!evaluate(&side, &conflicting, &bank).ordinary);

        let collapsed_side = clause(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &y, &a, true),
        ]);
        let singleton_main = clause(vec![literal(&mut bank, &c, &a, true)]);
        assert!(!evaluate(&collapsed_side, &singleton_main, &bank).ordinary);
    }

    #[test]
    fn resolution_encoding_accepts_one_complementary_target() {
        let _guard = global_state_lock();
        set_problem_type(ProblemType::FirstOrder).unwrap();
        let mut bank = test_bank();
        let x = variable(&bank, -20);
        let a = constant(&mut bank, "sat_sr_a");
        let b = constant(&mut bank, "sat_sr_b");
        let c = constant(&mut bank, "sat_sr_c");
        let side = clause(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &x, &b, true),
        ]);
        let main = clause(vec![
            literal(&mut bank, &c, &a, false),
            literal(&mut bank, &c, &b, true),
        ]);
        let evaluation = evaluate(&side, &main, &bank);
        assert!(!evaluation.ordinary);
        assert!(evaluation.resolution);
    }

    #[test]
    fn resolution_encoding_enforces_uniqueness_and_coherence() {
        let _guard = global_state_lock();
        set_problem_type(ProblemType::FirstOrder).unwrap();
        let mut bank = test_bank();
        let x = variable(&bank, -30);
        let a = constant(&mut bank, "sat_sr_unique_a");
        let b = constant(&mut bank, "sat_sr_unique_b");
        let c = constant(&mut bank, "sat_sr_unique_c");
        let side = clause(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &x, &b, true),
        ]);
        let different_negative_targets = clause(vec![
            literal(&mut bank, &c, &a, false),
            literal(&mut bank, &c, &b, false),
        ]);
        assert!(!evaluate(&side, &different_negative_targets, &bank).resolution);

        let coherent_side = clause(vec![
            literal(&mut bank, &x, &a, true),
            literal(&mut bank, &x, &a, false),
        ]);
        let one_target = clause(vec![literal(&mut bank, &c, &a, false)]);
        assert!(!evaluate(&coherent_side, &one_target, &bank).resolution);
    }
}
