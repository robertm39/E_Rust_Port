//! Experiment-only streaming driver for Umlaut's production `CaDiCaL` service.

use std::io::{self, BufRead, Write};
use std::time::{Duration, Instant};
use umlaut::clauses::cadical::CadicalSatService;
use umlaut::clauses::satservice::{IncrementalSatService, SatSolveOptions, SatSolveOutcome};

fn parse_literals<'a>(
    fields: impl Iterator<Item = &'a str>,
    line_number: usize,
) -> Result<Vec<i32>, String> {
    let mut literals = Vec::new();
    for field in fields {
        let literal = field
            .parse::<i32>()
            .map_err(|error| format!("line {line_number}: invalid literal: {error}"))?;
        if literal == 0 {
            return Ok(literals);
        }
        literals.push(literal);
    }
    Err(format!(
        "line {line_number}: literal list is not zero-terminated"
    ))
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
            control if control.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(output, "\\u{:04x}", u32::from(control));
            }
            ordinary => output.push(ordinary),
        }
    }
    output.push('"');
    output
}

fn json_array(values: &[i32]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn run() -> Result<(), String> {
    let mut service = CadicalSatService::new().map_err(|error| error.to_string())?;
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut clause_count = 0_usize;
    let mut insertion_ns = 0_u128;

    for (offset, line) in stdin.lock().lines().enumerate() {
        let line_number = offset + 1;
        let line = line.map_err(|error| format!("line {line_number}: {error}"))?;
        let mut fields = line.split_ascii_whitespace();
        let Some(opcode) = fields.next() else {
            continue;
        };
        match opcode {
            "a" => {
                let clause = parse_literals(fields, line_number)?;
                let started = Instant::now();
                service
                    .add_clause(&clause)
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                insertion_ns += started.elapsed().as_nanos();
                clause_count += 1;
            }
            "q" => {
                let query = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing query identifier"))?;
                let deadline_us = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing deadline"))?
                    .parse::<u64>()
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                let assumptions = parse_literals(fields, line_number)?;
                let started = Instant::now();
                let outcome = service.solve(
                    &assumptions,
                    &SatSolveOptions {
                        deadline: Some(Duration::from_micros(deadline_us)),
                        ..SatSolveOptions::default()
                    },
                );
                let elapsed_ns = started.elapsed().as_nanos();
                let (status, reason, model) = match outcome {
                    SatSolveOutcome::Sat { model } => ("sat", String::new(), model),
                    SatSolveOutcome::Unsat { .. } => ("unsat", String::new(), Vec::new()),
                    SatSolveOutcome::Unknown(reason) => {
                        ("unknown", format!("{reason:?}"), Vec::new())
                    }
                    SatSolveOutcome::Error(error) => {
                        return Err(format!("line {line_number}: {error}"));
                    }
                };
                writeln!(
                    stdout,
                    concat!(
                        "{{\"backend\":{},\"query\":{},\"clauses\":{},",
                        "\"assumptions\":{},\"status\":{},\"reason\":{},",
                        "\"elapsed_ns\":{},\"insertion_ns\":{},\"model_len\":{},",
                        "\"model\":{}}}"
                    ),
                    json_string(service.backend_name()),
                    json_string(query),
                    clause_count,
                    assumptions.len(),
                    json_string(status),
                    json_string(&reason),
                    elapsed_ns,
                    insertion_ns,
                    model.len(),
                    json_array(&model),
                )
                .map_err(|error| error.to_string())?;
                stdout.flush().map_err(|error| error.to_string())?;
                insertion_ns = 0;
            }
            "x" => return Ok(()),
            _ => return Err(format!("line {line_number}: unknown opcode {opcode}")),
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("umlaut-fnt-sat-probe: {error}");
        std::process::exit(1);
    }
}
