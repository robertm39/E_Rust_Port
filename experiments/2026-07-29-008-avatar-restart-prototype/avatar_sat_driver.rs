//! Persistent experiment-only bridge to Umlaut's incremental SAT service.

use std::io::{self, BufRead, Write};
use std::time::Instant;

use umlaut::clauses::satservice::{
    IncrementalSatService, InternalSatService, SatSolveOptions, SatSolveOutcome,
};

enum Command {
    Add(Vec<i32>),
    Count,
    Quit,
    Solve,
}

fn parse_command(line: &str) -> Result<Command, String> {
    let fields: Vec<&str> = line.split_ascii_whitespace().collect();
    let Some(operator) = fields.first() else {
        return Err("empty command".to_owned());
    };
    match *operator {
        "a" => {
            if fields.len() < 2 || fields.last() != Some(&"0") {
                return Err("add requires literals followed by zero".to_owned());
            }
            let mut clause = Vec::with_capacity(fields.len() - 2);
            for field in &fields[1..fields.len() - 1] {
                let literal = field
                    .parse::<i32>()
                    .map_err(|_| format!("invalid literal: {field}"))?;
                if literal == 0 {
                    return Err("zero is only allowed as the terminator".to_owned());
                }
                clause.push(literal);
            }
            Ok(Command::Add(clause))
        }
        "c" if fields.len() == 1 => Ok(Command::Count),
        "q" if fields.len() == 1 => Ok(Command::Quit),
        "s" if fields.len() == 1 => Ok(Command::Solve),
        _ => Err("unknown or malformed command".to_owned()),
    }
}

fn safe_message(message: &str) -> String {
    message
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:".contains(character) {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let mut service = InternalSatService::new();
    writeln!(stdout, "ready 1 {}", service.backend_name())?;
    stdout.flush()?;

    for line_result in stdin.lock().lines() {
        let line = line_result?;
        let command = match parse_command(&line) {
            Ok(command) => command,
            Err(error) => {
                writeln!(stdout, "error protocol {}", safe_message(&error))?;
                stdout.flush()?;
                continue;
            }
        };
        match command {
            Command::Add(clause) => match service.add_clause(&clause) {
                Ok(()) => {
                    writeln!(stdout, "ok {}", service.permanent_clause_count())?;
                }
                Err(error) => {
                    writeln!(stdout, "error sat {}", safe_message(&error.to_string()))?;
                }
            },
            Command::Count => {
                writeln!(stdout, "count {}", service.permanent_clause_count())?;
            }
            Command::Quit => {
                writeln!(stdout, "bye")?;
                stdout.flush()?;
                return Ok(());
            }
            Command::Solve => {
                let started = Instant::now();
                let outcome = service.solve(&[], &SatSolveOptions::default());
                let elapsed_ns = started.elapsed().as_nanos();
                match outcome {
                    SatSolveOutcome::Sat { model } => {
                        write!(stdout, "sat {elapsed_ns}")?;
                        for literal in model {
                            write!(stdout, " {literal}")?;
                        }
                        writeln!(stdout, " 0")?;
                    }
                    SatSolveOutcome::Unsat { .. } => {
                        writeln!(stdout, "unsat {elapsed_ns}")?;
                    }
                    SatSolveOutcome::Unknown(reason) => {
                        writeln!(
                            stdout,
                            "unknown {elapsed_ns} {}",
                            safe_message(&format!("{reason:?}"))
                        )?;
                    }
                    SatSolveOutcome::Error(error) => {
                        writeln!(stdout, "error solve {}", safe_message(&error.to_string()))?;
                    }
                }
            }
        }
        stdout.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_command, safe_message, Command};

    #[test]
    fn add_command_requires_a_dimacs_terminator() {
        assert!(parse_command("a 1 -2").is_err());
        assert!(matches!(
            parse_command("a 0"),
            Ok(Command::Add(clause)) if clause.is_empty()
        ));
        assert!(matches!(
            parse_command("a 1 -2 0"),
            Ok(Command::Add(clause)) if clause == vec![1, -2]
        ));
    }

    #[test]
    fn singleton_commands_reject_trailing_fields() {
        assert!(matches!(parse_command("s"), Ok(Command::Solve)));
        assert!(parse_command("s unexpected").is_err());
    }

    #[test]
    fn error_messages_are_one_field() {
        assert_eq!(safe_message("bad value!"), "bad_value_");
    }
}
