//! Opt-in command-line surface for typed base VIRAS elimination.

use crate::arithmetic::typed_lira::{
    import_document_with_max_rational_bits, render_tff_document, ImportError, ImportErrorCode,
    ImportedDocument,
};
use crate::arithmetic::viras::{eliminate_formula, FormulaQeOutcome, Limits, QeStatus};
use crate::basics::error::{Diagnostic, ErrorCode};
use std::fmt::Write as _;
use std::fs;
use std::io::{Read, Write};

pub const PROGRAM_NAME: &str = "umlaut-viras-qe";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Json,
    Tff,
}

#[derive(Clone, Debug)]
struct Config {
    input: String,
    format: OutputFormat,
    limits: Limits,
}

enum Command {
    Help,
    Execute(Config),
}

fn usage_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::USAGE_ERROR, message)
}

fn io_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, message)
}

fn parse_usize(name: &str, value: &str) -> Result<usize, Diagnostic> {
    value
        .parse::<usize>()
        .map_err(|_| usage_error(format!("{name} requires a nonnegative integer")))
}

fn parse_u64(name: &str, value: &str) -> Result<u64, Diagnostic> {
    value
        .parse::<u64>()
        .map_err(|_| usage_error(format!("{name} requires a nonnegative integer")))
}

fn split_option(argument: &str) -> Option<(&str, &str)> {
    argument
        .strip_prefix("--")
        .and_then(|option| option.split_once('='))
}

fn parse_command<I, S>(argv: I) -> Result<Command, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut arguments = argv.into_iter().map(Into::into);
    let _program = arguments.next();
    let mut format = OutputFormat::Json;
    let mut limits = Limits::default();
    let mut input = None;
    for argument in arguments {
        match argument.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "--json" => format = OutputFormat::Json,
            "--tff" => format = OutputFormat::Tff,
            "-" => {
                if input.replace(argument).is_some() {
                    return Err(usage_error("only one input document is accepted"));
                }
            }
            _ if argument.starts_with("--") => {
                let (name, value) = split_option(&argument)
                    .ok_or_else(|| usage_error(format!("unknown option {argument}")))?;
                match name {
                    "format" => {
                        format = match value {
                            "json" => OutputFormat::Json,
                            "tff" => OutputFormat::Tff,
                            _ => {
                                return Err(usage_error("--format must be either json or tff"));
                            }
                        };
                    }
                    "max-steps" => limits.max_steps = parse_usize(name, value)?,
                    "max-candidates" => {
                        limits.max_candidates = parse_usize(name, value)?;
                    }
                    "max-grids" => limits.max_grids = parse_usize(name, value)?,
                    "max-grid-points" => {
                        limits.max_grid_points = parse_usize(name, value)?;
                    }
                    "max-dnf-branches" => {
                        limits.max_dnf_branches = parse_usize(name, value)?;
                    }
                    "max-formula-nodes" => {
                        limits.max_formula_nodes = parse_usize(name, value)?;
                    }
                    "max-rational-bits" => {
                        limits.max_rational_bits = parse_u64(name, value)?;
                    }
                    _ => return Err(usage_error(format!("unknown option --{name}"))),
                }
            }
            _ => {
                if input.replace(argument).is_some() {
                    return Err(usage_error("only one input document is accepted"));
                }
            }
        }
    }
    Ok(Command::Execute(Config {
        input: input.unwrap_or_else(|| "-".to_owned()),
        format,
        limits,
    }))
}

#[must_use]
pub fn help() -> String {
    format!(
        "{PROGRAM_NAME} - opt-in typed base VIRAS quantifier elimination\n\n\
         Usage: {PROGRAM_NAME} [OPTIONS] [FILE|-]\n\n\
         Options:\n\
           --json, --format=json              Canonical JSON output (default)\n\
           --tff, --format=tff                Transformed real-sorted TFF\n\
           --max-steps=N                      Total traversal/kernel steps\n\
           --max-candidates=N                 Total virtual candidates\n\
           --max-grids=N                      Total generated grids\n\
           --max-grid-points=N                Total materialized grid points\n\
           --max-dnf-branches=N               Bounded Boolean expansion\n\
           --max-formula-nodes=N              Result formula size\n\
           --max-rational-bits=N              Exact rational bit width\n\
           -h, --help                         Show this help\n"
    )
}

fn read_input(config: &Config, stdin: &mut impl Read) -> Result<String, Diagnostic> {
    if config.input == "-" {
        let mut source = String::new();
        stdin
            .read_to_string(&mut source)
            .map_err(|error| io_error(format!("cannot read stdin: {error}")))?;
        Ok(source)
    } else {
        fs::read_to_string(&config.input)
            .map_err(|error| io_error(format!("cannot read {}: {error}", config.input)))
    }
}

fn json_escape(value: &str) -> String {
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
                let _ = write!(output, "\\u{:04x}", u32::from(control));
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn render_trace(document: &ImportedDocument) -> String {
    document
        .import
        .trace
        .iter()
        .map(|step| {
            format!(
                "{{\"kind\":{},\"source\":{},\"target\":{}}}",
                json_escape(step.kind),
                json_escape(&step.source),
                json_escape(&step.target)
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn render_eliminations(outcome: &FormulaQeOutcome) -> String {
    outcome
        .derivation
        .eliminations
        .iter()
        .map(|elimination| {
            let candidates = elimination
                .candidates
                .iter()
                .map(|candidate| {
                    format!(
                        concat!("{{\"literal_index\":{},\"origin\":{},", "\"virtual\":{}}}"),
                        candidate.literal_index,
                        json_escape(candidate.origin_kind),
                        candidate.virtual_term.canonical_json()
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let flattening = elimination
                .grid_flattening
                .iter()
                .map(|record| {
                    let output = record
                        .output
                        .iter()
                        .map(crate::arithmetic::viras::VirtualTerm::canonical_json)
                        .collect::<Vec<_>>()
                        .join(",");
                    format!(
                        concat!(
                            "{{\"case\":{},\"common_period\":{},\"input\":{},",
                            "\"output\":[{}]}}"
                        ),
                        json_escape(record.case),
                        json_escape(&record.common_period.to_string()),
                        record.input.canonical_json(),
                        output
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let usage = elimination.resource_usage;
            format!(
                concat!(
                    "{{\"calculus\":{},\"eliminated\":{},",
                    "\"candidates\":[{}],\"grid_flattening\":[{}],",
                    "\"resource_usage\":{{\"steps\":{},\"candidates\":{},",
                    "\"grids\":{},\"grid_points\":{}}}}}"
                ),
                json_escape(elimination.calculus),
                json_escape(&elimination.eliminated),
                candidates,
                flattening,
                usage.steps,
                usage.candidates,
                usage.grids,
                usage.grid_points
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[must_use]
pub fn render_json(document: &ImportedDocument, outcome: &FormulaQeOutcome) -> String {
    let status = match outcome.status {
        QeStatus::Success => "success",
        QeStatus::Unknown => "unknown",
    };
    let result_formula = outcome.formula.as_ref().map_or_else(
        || "null".to_owned(),
        crate::arithmetic::viras::Formula::canonical_json,
    );
    let transformed_tff = outcome.formula.as_ref().map_or_else(
        || "null".to_owned(),
        |formula| {
            json_escape(&render_tff_document(
                &document.source_name,
                &document.source_role,
                formula,
            ))
        },
    );
    let unknown_kind = outcome.unknown_kind.map_or_else(
        || "null".to_owned(),
        |kind| json_escape(&format!("{kind:?}")),
    );
    let usage = outcome.derivation.resource_usage;
    format!(
        concat!(
            "{{\"schema\":\"umlaut-viras-qe-v1\",\"status\":{},",
            "\"source_name\":{},\"source_role\":{},",
            "\"imported_formula\":{},\"result_formula\":{},",
            "\"transformed_tff\":{},\"unknown_kind\":{},\"reason\":{},",
            "\"trace\":[{}],\"derivation\":{{\"calculus\":{},",
            "\"eliminations\":[{}],\"replay_validated\":{},\"resource_usage\":{{",
            "\"steps\":{},\"candidates\":{},\"grids\":{},\"grid_points\":{},",
            "\"dnf_branches\":{},\"quantifiers\":{}}}}}}}\n"
        ),
        json_escape(status),
        json_escape(&document.source_name),
        json_escape(&document.source_role),
        document.import.formula.canonical_json(),
        result_formula,
        transformed_tff,
        unknown_kind,
        json_escape(&outcome.reason),
        render_trace(document),
        json_escape(outcome.derivation.calculus),
        render_eliminations(outcome),
        outcome.derivation.replay_validated,
        usage.kernel.steps,
        usage.kernel.candidates,
        usage.kernel.grids,
        usage.kernel.grid_points,
        usage.dnf_branches,
        usage.quantifiers
    )
}

fn render_import_resource_unknown(error: &ImportError) -> String {
    format!(
        concat!(
            "{{\"schema\":\"umlaut-viras-qe-v1\",\"status\":\"unknown\",",
            "\"source_name\":null,\"source_role\":null,\"imported_formula\":null,",
            "\"result_formula\":null,\"transformed_tff\":null,",
            "\"unknown_kind\":\"ResourceLimit\",\"reason\":{},",
            "\"trace\":[],\"derivation\":null}}\n"
        ),
        json_escape(&error.message)
    )
}

pub fn run<I, S>(
    argv: I,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let command = parse_command(argv)?;
    let Command::Execute(config) = command else {
        stdout
            .write_all(help().as_bytes())
            .map_err(|error| io_error(format!("cannot write help: {error}")))?;
        return Ok(0);
    };
    let source = read_input(&config, stdin)?;
    let document =
        match import_document_with_max_rational_bits(&source, config.limits.max_rational_bits) {
            Ok(document) => document,
            Err(error) if error.code == ImportErrorCode::ResourceLimit => {
                if config.format == OutputFormat::Json {
                    stdout
                        .write_all(render_import_resource_unknown(&error).as_bytes())
                        .map_err(|write_error| {
                            io_error(format!("cannot write resource-limit JSON: {write_error}"))
                        })?;
                } else {
                    writeln!(stderr, "ResourceLimit: {}", error.message).map_err(
                        |write_error| {
                            io_error(format!(
                                "cannot write resource-limit diagnostic: {write_error}"
                            ))
                        },
                    )?;
                }
                return Ok(2);
            }
            Err(error) => {
                writeln!(stderr, "{error}").map_err(|write_error| {
                    io_error(format!("cannot write error: {write_error}"))
                })?;
                return Ok(2);
            }
        };
    let outcome = eliminate_formula(document.import.formula.clone(), config.limits);
    match config.format {
        OutputFormat::Json => stdout
            .write_all(render_json(&document, &outcome).as_bytes())
            .map_err(|error| io_error(format!("cannot write JSON: {error}")))?,
        OutputFormat::Tff => {
            let Some(formula) = outcome.formula.as_ref() else {
                writeln!(
                    stderr,
                    "{}: {}",
                    outcome
                        .unknown_kind
                        .map_or_else(|| "UNKNOWN".to_owned(), |kind| format!("{kind:?}")),
                    outcome.reason
                )
                .map_err(|error| io_error(format!("cannot write error: {error}")))?;
                return Ok(2);
            };
            stdout
                .write_all(
                    render_tff_document(&document.source_name, &document.source_role, formula)
                        .as_bytes(),
                )
                .map_err(|error| io_error(format!("cannot write TFF: {error}")))?;
        }
    }
    Ok(if outcome.status == QeStatus::Success {
        0
    } else {
        2
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::global_state_lock;

    #[test]
    fn cli_emits_canonical_json_and_tff() {
        let _global = global_state_lock();
        let source = b"tff(case,conjecture,? [I:$int] : (I = $to_int(-1.5))).";
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run(
            [PROGRAM_NAME, "--json", "-"],
            &mut source.as_slice(),
            &mut output,
            &mut error,
        )
        .expect("JSON run");
        assert_eq!(status, 0);
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("UTF-8 JSON");
        assert!(output.contains("\"schema\":\"umlaut-viras-qe-v1\""));
        assert!(output.contains("\"result_formula\":[\"bool\",true]"));
        assert!(output.contains("\"eliminations\":["));
        assert!(output.contains("\"replay_validated\":true"));
        assert!(output.ends_with('\n'));

        let mut output = Vec::new();
        let status = run(
            [PROGRAM_NAME, "--tff", "-"],
            &mut source.as_slice(),
            &mut output,
            &mut Vec::new(),
        )
        .expect("TFF run");
        assert_eq!(status, 0);
        assert_eq!(
            String::from_utf8(output).expect("UTF-8 TFF"),
            "tff(umlaut_viras_case,conjecture,\n    $true ).\n"
        );
    }

    #[test]
    fn cli_reports_stable_import_and_resource_failures() {
        let _global = global_state_lock();
        let unsupported = b"fof(case,axiom,1 = 1).";
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run(
            [PROGRAM_NAME, "-"],
            &mut unsupported.as_slice(),
            &mut output,
            &mut error,
        )
        .expect("unsupported run");
        assert_eq!(status, 2);
        assert!(output.is_empty());
        assert!(String::from_utf8(error)
            .expect("UTF-8 error")
            .starts_with("UNSUPPORTED_DIALECT:"));

        let source = b"tff(case,axiom,? [R:$real] : (R = 0.0)).";
        let mut output = Vec::new();
        let status = run(
            [PROGRAM_NAME, "--max-steps=0", "-"],
            &mut source.as_slice(),
            &mut output,
            &mut Vec::new(),
        )
        .expect("resource run");
        assert_eq!(status, 2);
        assert!(String::from_utf8(output)
            .expect("UTF-8 JSON")
            .contains("\"status\":\"unknown\""));

        let source = b"tff(case,axiom,256 = 256).";
        let mut output = Vec::new();
        let status = run(
            [PROGRAM_NAME, "--max-rational-bits=8", "-"],
            &mut source.as_slice(),
            &mut output,
            &mut Vec::new(),
        )
        .expect("import resource run");
        assert_eq!(status, 2);
        let output = String::from_utf8(output).expect("UTF-8 JSON");
        assert!(output.contains("\"status\":\"unknown\""));
        assert!(output.contains("\"unknown_kind\":\"ResourceLimit\""));
        assert!(output.contains("\"imported_formula\":null"));

        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run(
            [PROGRAM_NAME, "--tff", "--max-rational-bits=8", "-"],
            &mut source.as_slice(),
            &mut output,
            &mut error,
        )
        .expect("TFF import resource run");
        assert_eq!(status, 2);
        assert!(output.is_empty());
        assert!(String::from_utf8(error)
            .expect("UTF-8 resource diagnostic")
            .starts_with("ResourceLimit:"));
    }

    #[test]
    fn cli_help_is_feature_specific() {
        let mut output = Vec::new();
        let status = run(
            [PROGRAM_NAME, "--help"],
            &mut &b""[..],
            &mut output,
            &mut Vec::new(),
        )
        .expect("help");
        assert_eq!(status, 0);
        assert!(String::from_utf8(output)
            .expect("UTF-8 help")
            .contains("opt-in typed base VIRAS"));
    }
}
