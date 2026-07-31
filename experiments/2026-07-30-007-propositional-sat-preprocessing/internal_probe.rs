//! Experiment-only adapter for Umlaut's production internal SAT service.

use std::env;
use std::fs;
use std::time::{Duration, Instant};
use umlaut::clauses::satservice::{
    IncrementalSatService, InternalSatService, SatSolveOptions, SatSolveOutcome, SatUnknownReason,
};

fn parse_dimacs(path: &str) -> Result<(i32, Vec<Vec<i32>>), String> {
    let input = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut variables = None;
    let mut declared_clauses = None;
    let mut clauses = Vec::new();
    for (offset, line) in input.lines().enumerate() {
        let line_number = offset + 1;
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.is_empty() || fields[0] == "c" {
            continue;
        }
        if fields[0] == "p" {
            if fields.len() != 4 || fields[1] != "cnf" || variables.is_some() {
                return Err(format!("line {line_number}: invalid header"));
            }
            variables = Some(
                fields[2]
                    .parse::<i32>()
                    .map_err(|error| format!("line {line_number}: {error}"))?,
            );
            declared_clauses = Some(
                fields[3]
                    .parse::<usize>()
                    .map_err(|error| format!("line {line_number}: {error}"))?,
            );
            continue;
        }
        let mut clause = Vec::new();
        for field in fields {
            let literal = field
                .parse::<i32>()
                .map_err(|error| format!("line {line_number}: {error}"))?;
            if literal == 0 {
                break;
            }
            clause.push(literal);
        }
        clauses.push(clause);
    }
    let variables = variables.ok_or_else(|| "missing DIMACS header".to_owned())?;
    if declared_clauses != Some(clauses.len()) {
        return Err("DIMACS clause count does not match header".to_owned());
    }
    Ok((variables, clauses))
}

fn unknown_name(reason: SatUnknownReason) -> &'static str {
    match reason {
        SatUnknownReason::DecisionLimit => "decision_limit",
        SatUnknownReason::Deadline => "deadline",
        SatUnknownReason::Cancelled => "cancelled",
        SatUnknownReason::ExternalStop => "external_stop",
        SatUnknownReason::Backend => "backend",
    }
}

fn json_array(values: &[i32]) -> String {
    let body = values
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

fn complete_model(variables: i32, partial: &[i32]) -> Result<Vec<i32>, String> {
    let model_length = usize::try_from(variables)
        .map_err(|_| "DIMACS variable count must be nonnegative".to_owned())?;
    let mut values = vec![false; model_length + 1];
    let mut assigned = vec![false; model_length + 1];
    for &literal in partial {
        let variable = usize::try_from(
            literal
                .checked_abs()
                .ok_or_else(|| "model literal is out of range".to_owned())?,
        )
        .map_err(|_| "model literal is out of range".to_owned())?;
        if variable == 0 || variable > model_length || assigned[variable] {
            return Err("model is duplicate or out of range".to_owned());
        }
        assigned[variable] = true;
        values[variable] = literal > 0;
    }
    (1..=model_length)
        .map(|variable| {
            let literal = i32::try_from(variable)
                .map_err(|_| "DIMACS variable count is out of range".to_owned())?;
            Ok(if values[variable] { literal } else { -literal })
        })
        .collect()
}

fn run(path: &str, decision_limit: Option<u64>) -> Result<(), String> {
    let (variables, clauses) = parse_dimacs(path)?;
    let mut solver = InternalSatService::new();
    let insertion_start = Instant::now();
    for clause in &clauses {
        solver
            .add_clause(clause)
            .map_err(|error| error.to_string())?;
    }
    let insertion_ns = insertion_start.elapsed().as_nanos();
    let solve_start = Instant::now();
    let outcome = solver.solve(
        &[],
        &SatSolveOptions {
            decision_limit,
            deadline: Some(Duration::from_secs(1)),
            ..SatSolveOptions::default()
        },
    );
    let solve_ns = solve_start.elapsed().as_nanos();
    let (status, model, unknown) = match outcome {
        SatSolveOutcome::Sat { model } => ("sat", complete_model(variables, &model)?, ""),
        SatSolveOutcome::Unsat { .. } => ("unsat", Vec::new(), ""),
        SatSolveOutcome::Unknown(reason) => ("unknown", Vec::new(), unknown_name(reason)),
        SatSolveOutcome::Error(error) => return Err(error.to_string()),
    };
    println!(
        concat!(
            "{{\"backend\":\"internal\",\"status\":\"{status}\",",
            "\"variables\":{variables},\"active_before\":{variables},",
            "\"active_after\":{variables},\"clauses_before\":{clauses_before},",
            "\"clauses_after\":{clauses_after},\"insertion_ns\":{insertion_ns},",
            "\"simplify_ns\":0,\"solve_ns\":{solve_ns},\"unknown\":\"{unknown}\",",
            "\"model\":{model}}}"
        ),
        status = status,
        variables = variables,
        clauses_before = clauses.len(),
        clauses_after = clauses.len(),
        insertion_ns = insertion_ns,
        solve_ns = solve_ns,
        unknown = unknown,
        model = json_array(&model)
    );
    Ok(())
}

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    if arguments.len() != 3 {
        eprintln!("usage: internal-probe INPUT DECISIONS");
        std::process::exit(2);
    }
    let decision_limit = arguments[2].parse::<i64>().unwrap_or_else(|error| {
        eprintln!("internal-probe: invalid decision limit: {error}");
        std::process::exit(2);
    });
    if decision_limit < -1 {
        eprintln!("internal-probe: decision limit must be -1 or nonnegative");
        std::process::exit(2);
    }
    let decision_limit = u64::try_from(decision_limit).ok();
    if let Err(error) = run(&arguments[1], decision_limit) {
        eprintln!("internal-probe: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::complete_model;

    #[test]
    fn totalizes_unmentioned_declared_variables_as_false() {
        assert_eq!(complete_model(3, &[1]).unwrap(), vec![1, -2, -3]);
    }

    #[test]
    fn rejects_duplicate_or_out_of_range_model_literals() {
        assert!(complete_model(2, &[1, -1]).is_err());
        assert!(complete_model(2, &[3]).is_err());
    }
}
