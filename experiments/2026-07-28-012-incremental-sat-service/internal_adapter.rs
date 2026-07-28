//! Experiment-only adapter for Umlaut's current recursive DPLL solver.
//!
//! `internal-adapter.patch` makes the existing private solver entry point
//! visible to this temporary binary in the remote experiment workspace. The
//! production worktree is not modified by the patch.

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::time::Instant;
use umlaut::clauses::satinterface::{solve_sat, SolverStatus};

fn parse_literals<'a>(
    fields: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<Vec<i32>, String> {
    let mut literals = Vec::new();
    let mut terminated = false;
    for field in fields {
        let literal = field
            .parse::<i32>()
            .map_err(|error| format!("line {line_number}: invalid literal: {error}"))?;
        if literal == 0 {
            terminated = true;
            break;
        }
        literals.push(literal);
    }
    if !terminated {
        return Err(format!(
            "line {line_number}: literal list is not zero-terminated"
        ));
    }
    Ok(literals)
}

fn json_array(values: &[i32]) -> String {
    let values = values
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn status_name(status: SolverStatus) -> &'static str {
    match status {
        SolverStatus::Sat => "sat",
        SolverStatus::Unsat => "unsat",
        SolverStatus::GaveUp => "unknown",
    }
}

fn solve_with_assumptions(
    clauses: &[Vec<i32>],
    assumptions: &[i32],
    max_variable: i32,
    decision_limit: i32,
) -> SolverStatus {
    let mut query_clauses = Vec::with_capacity(clauses.len() + assumptions.len());
    query_clauses.extend_from_slice(clauses);
    query_clauses.extend(assumptions.iter().map(|literal| vec![*literal]));
    solve_sat(&query_clauses, max_variable, decision_limit)
}

fn failed_core(
    clauses: &[Vec<i32>],
    assumptions: &[i32],
    max_variable: i32,
    decision_limit: i32,
) -> Vec<i32> {
    let mut core = assumptions.to_vec();
    let mut index = 0;
    while index < core.len() {
        let mut trial = core.clone();
        trial.remove(index);
        if solve_with_assumptions(clauses, &trial, max_variable, decision_limit)
            == SolverStatus::Unsat
        {
            core = trial;
        } else {
            index += 1;
        }
    }
    core
}

fn run(path: &str) -> Result<(), String> {
    let input = File::open(path).map_err(|error| format!("could not open session: {error}"))?;
    let mut max_variable = None;
    let mut clauses = Vec::<Vec<i32>>::new();

    for (offset, line) in BufReader::new(input).lines().enumerate() {
        let line_number = offset + 1;
        let line = line.map_err(|error| format!("line {line_number}: {error}"))?;
        if line.is_empty() || line.starts_with('c') {
            continue;
        }
        let mut fields = line.split_ascii_whitespace();
        let opcode = fields
            .next()
            .ok_or_else(|| format!("line {line_number}: missing opcode"))?;
        match opcode {
            "p" => {
                let format = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing format"))?;
                let parsed_max = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing maximum variable"))?
                    .parse::<i32>()
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                if format != "isat" || parsed_max < 0 || max_variable.is_some() {
                    return Err(format!("line {line_number}: invalid or duplicate header"));
                }
                max_variable = Some(parsed_max);
            }
            "a" => {
                if max_variable.is_none() {
                    return Err(format!("line {line_number}: clause precedes header"));
                }
                clauses.push(parse_literals(fields, line_number)?);
            }
            "q" => {
                let Some(max_variable) = max_variable else {
                    return Err(format!("line {line_number}: query precedes header"));
                };
                let query = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing query identifier"))?;
                let limit = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing native limit"))?
                    .parse::<i32>()
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                let _deadline_us = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing deadline"))?
                    .parse::<u64>()
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                let assumptions = parse_literals(fields, line_number)?;
                let started = Instant::now();
                let result = solve_with_assumptions(&clauses, &assumptions, max_variable, limit);
                let elapsed_ns = started.elapsed().as_nanos();
                let core_started = Instant::now();
                let core = if result == SolverStatus::Unsat {
                    failed_core(&clauses, &assumptions, max_variable, limit)
                } else {
                    Vec::new()
                };
                let core_ns = core_started.elapsed().as_nanos();
                println!(
                    concat!(
                        "{{\"backend\":\"internal-dpll\",\"version\":\"umlaut\",",
                        "\"session\":\"{}\",\"query\":\"{}\",\"clauses\":{},",
                        "\"assumptions\":{},\"status\":\"{}\",\"elapsed_ns\":{},",
                        "\"core_ns\":{},\"insertion_ns\":0,",
                        "\"native_limit_kind\":\"decisions\",",
                        "\"native_deadline\":false,\"proof_capable\":false,",
                        "\"decisions\":0,\"conflicts\":0,\"propagations\":0,",
                        "\"model\":[],\"core\":{}}}"
                    ),
                    path,
                    query,
                    clauses.len(),
                    assumptions.len(),
                    status_name(result),
                    elapsed_ns,
                    core_ns,
                    json_array(&core)
                );
            }
            _ => return Err(format!("line {line_number}: unknown opcode {opcode}")),
        }
    }
    if max_variable.is_none() {
        return Err("session has no header".to_owned());
    }
    Ok(())
}

fn main() {
    let mut arguments = env::args();
    let _program = arguments.next();
    let Some(path) = arguments.next() else {
        eprintln!("usage: sat-service-probe SESSION");
        std::process::exit(2);
    };
    if arguments.next().is_some() {
        eprintln!("usage: sat-service-probe SESSION");
        std::process::exit(2);
    }
    if let Err(error) = run(&path) {
        let _ = writeln_stderr(&format!("sat-service-probe: {error}"));
        std::process::exit(1);
    }
}

fn writeln_stderr(message: &str) -> io::Result<()> {
    use std::io::Write;
    writeln!(io::stderr().lock(), "{message}")
}
