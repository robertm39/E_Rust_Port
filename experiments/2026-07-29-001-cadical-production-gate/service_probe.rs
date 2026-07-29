//! Experiment-only driver for Umlaut's production incremental SAT service.

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::time::{Duration, Instant};
use umlaut::clauses::cadical::CadicalSatService;
use umlaut::clauses::satservice::{
    IncrementalSatService, InternalSatService, SatSolveOptions, SatSolveOutcome,
};

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

fn run(backend: &str, path: &str) -> Result<(), String> {
    let mut service: Box<dyn IncrementalSatService> = match backend {
        "internal" => Box::new(InternalSatService::new()),
        "cadical" => Box::new(CadicalSatService::new().map_err(|error| error.to_string())?),
        _ => return Err(format!("unknown backend {backend:?}")),
    };
    let backend_name = service.backend_name();
    let input = File::open(path).map_err(|error| format!("could not open session: {error}"))?;
    let mut saw_header = false;
    let mut clause_count = 0_usize;
    let mut insertion_ns = 0_u128;

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
                let maximum = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing maximum variable"))?
                    .parse::<i32>()
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                if format != "isat" || maximum < 0 || saw_header || fields.next().is_some() {
                    return Err(format!("line {line_number}: invalid or duplicate header"));
                }
                saw_header = true;
            }
            "a" => {
                if !saw_header {
                    return Err(format!("line {line_number}: clause precedes header"));
                }
                let clause = parse_literals(fields, line_number)?;
                let started = Instant::now();
                service
                    .add_clause(&clause)
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                insertion_ns += started.elapsed().as_nanos();
                clause_count += 1;
            }
            "q" => {
                if !saw_header {
                    return Err(format!("line {line_number}: query precedes header"));
                }
                let query = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing query identifier"))?;
                let raw_limit = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing native limit"))?
                    .parse::<i64>()
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                let deadline_us = fields
                    .next()
                    .ok_or_else(|| format!("line {line_number}: missing deadline"))?
                    .parse::<u64>()
                    .map_err(|error| format!("line {line_number}: {error}"))?;
                let assumptions = parse_literals(fields, line_number)?;
                let decision_limit = if raw_limit < 0 {
                    None
                } else {
                    Some(
                        u64::try_from(raw_limit)
                            .map_err(|error| format!("line {line_number}: {error}"))?,
                    )
                };
                let options = SatSolveOptions {
                    decision_limit,
                    deadline: (deadline_us > 0).then(|| Duration::from_micros(deadline_us)),
                    ..SatSolveOptions::default()
                };
                let started = Instant::now();
                let outcome = service.solve(&assumptions, &options);
                let elapsed_ns = started.elapsed().as_nanos();
                let (status, model_len, core) = match outcome {
                    SatSolveOutcome::Sat { model } => ("sat", model.len(), Vec::new()),
                    SatSolveOutcome::Unsat {
                        failed_assumptions, ..
                    } => ("unsat", 0, failed_assumptions),
                    SatSolveOutcome::Unknown(_) => ("unknown", 0, Vec::new()),
                    SatSolveOutcome::Error(error) => {
                        return Err(format!("line {line_number}: {error}"));
                    }
                };
                println!(
                    concat!(
                        "{{\"backend\":{},\"session\":{},\"query\":{},",
                        "\"clauses\":{},\"assumptions\":{},\"status\":{},",
                        "\"elapsed_ns\":{},\"core_ns\":0,\"insertion_ns\":{},",
                        "\"model_len\":{},\"core\":{}}}"
                    ),
                    json_string(backend_name),
                    json_string(path),
                    json_string(query),
                    clause_count,
                    assumptions.len(),
                    json_string(status),
                    elapsed_ns,
                    insertion_ns,
                    model_len,
                    json_array(&core),
                );
                insertion_ns = 0;
            }
            _ => return Err(format!("line {line_number}: unknown opcode {opcode}")),
        }
    }
    if !saw_header {
        return Err("session has no header".to_owned());
    }
    service.reset().map_err(|error| error.to_string())
}

fn main() {
    let mut arguments = env::args();
    let _program = arguments.next();
    let Some(backend) = arguments.next() else {
        eprintln!("usage: umlaut-sat-service-probe BACKEND SESSION");
        std::process::exit(2);
    };
    let Some(path) = arguments.next() else {
        eprintln!("usage: umlaut-sat-service-probe BACKEND SESSION");
        std::process::exit(2);
    };
    if arguments.next().is_some() {
        eprintln!("usage: umlaut-sat-service-probe BACKEND SESSION");
        std::process::exit(2);
    }
    if let Err(error) = run(&backend, &path) {
        let _ = writeln!(io::stderr().lock(), "umlaut-sat-service-probe: {error}");
        std::process::exit(1);
    }
}
