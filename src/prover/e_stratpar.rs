use crate::basics::error::{Diagnostic, ErrorCode};
use crate::control::proc_ctrl::{EPCtrl, EPCtrlSet, MAX_CORES};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::prover::version::{footer, E_NICKNAME, VERSION};
use std::io::Write;
use std::time::Duration;

pub const PROGRAM_NAME: &str = "e_stratpar";

const DEFAULT_PROVER: &str = "eprover";
const DEFAULT_HARD_TIME_LIMIT: i64 = 3600;
const C_USAGE_ERROR: &str = "Usage: e_ltb_runner <spec> [<path-to-eprover>]";
const PROCESS_POLL_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    CpuLimit,
}

const OPTIONS: &[OptCell<OptionCode>] = &[
    OptCell::new(
        OptionCode::Help,
        Some('h'),
        Some("help"),
        OptArgType::NoArg,
        None,
        "Print a short description of program usage and options.",
    ),
    OptCell::new(
        OptionCode::Version,
        Some('V'),
        Some("version"),
        OptArgType::NoArg,
        None,
        "Print the version number of the prover. Please include this with all bug reports (if any).",
    ),
    OptCell::new(
        OptionCode::CpuLimit,
        None,
        Some("cpu-limit"),
        OptArgType::OptArg,
        Some("300"),
        "Limit the cpu time the prover should run. The optional argument is the CPU time in seconds.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct StratparConfig {
    cpu_limit: i64,
    problem_file: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StratparStrategy {
    name: String,
    options: String,
    cpu_limit: i64,
    problem_file: String,
}

#[derive(Debug)]
enum RunCommand {
    Execute(StratparConfig),
    Exit(u8),
}

pub fn run<I, S>(
    argv: I,
    stdout: &mut impl Write,
    _stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    init_io(PROGRAM_NAME);
    let result = run_inner(argv, stdout);
    exit_io();
    result
}

fn run_inner<I, S>(argv: I, stdout: &mut impl Write) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout)? {
        RunCommand::Execute(config) => execute_config(&config, stdout),
        RunCommand::Exit(status) => Ok(status),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut cpu_limit = DEFAULT_HARD_TIME_LIMIT;

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        match parsed.option().option_code {
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::Version => {
                writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION} {E_NICKNAME}"))?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::CpuLimit => {
                cpu_limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
        }
    }

    let positional = state.remaining_args();
    if positional.is_empty() || positional.len() > 2 {
        return Err(Diagnostic::new(ErrorCode::USAGE_ERROR, C_USAGE_ERROR));
    }

    Ok(RunCommand::Execute(StratparConfig {
        cpu_limit,
        problem_file: positional[0].clone(),
    }))
}

fn execute_config(config: &StratparConfig, stdout: &mut impl Write) -> Result<u8, Diagnostic> {
    execute_with_spawner(config, stdout, |strategy| {
        EPCtrl::create_generic(
            DEFAULT_PROVER,
            &strategy.name,
            &strategy.options,
            "",
            strategy.cpu_limit,
            strategy.problem_file.clone(),
        )
    })
}

fn execute_with_spawner<F>(
    config: &StratparConfig,
    stdout: &mut impl Write,
    mut spawn: F,
) -> Result<u8, Diagnostic>
where
    F: FnMut(&StratparStrategy) -> Result<EPCtrl, Diagnostic>,
{
    let mut controls = EPCtrlSet::new();
    for strategy in strategy_specs(config) {
        let proc = spawn(&strategy)?;
        let _previous = controls.add_proc(proc)?;
    }
    execute_process_set(&mut controls, stdout)
}

fn execute_process_set(
    controls: &mut EPCtrlSet,
    stdout: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut proof_descriptor = None;
    while !controls.is_empty() {
        proof_descriptor =
            controls.get_result_from_pipes_timeout(PROCESS_POLL_TIMEOUT, false, stdout)?;
        if proof_descriptor.is_some() {
            break;
        }
    }

    if let Some(descriptor) = proof_descriptor {
        let proof_output = controls
            .find_proc(descriptor)
            .map(|proc| proc.output().view_bytes().to_vec())
            .ok_or_else(|| Diagnostic::new(ErrorCode::INTERFACE_ERROR, "Missing proof process"))?;
        write_all(stdout, &proof_output)?;
    } else {
        writeln_diag(stdout, "% SZS status GaveUp")?;
    }
    controls.clear(false)?;
    Ok(ErrorCode::NO_ERROR.exit_status())
}

fn strategy_specs(config: &StratparConfig) -> Vec<StratparStrategy> {
    (0..MAX_CORES)
        .map(|index| StratparStrategy {
            name: format!("AutoSched{index}"),
            options: format!("-xAutoSched{index} -tAutoSched{index} --sine"),
            cpu_limit: config.cpu_limit / 2,
            problem_file: config.problem_file.clone(),
        })
        .collect()
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
{PROGRAM_NAME} {VERSION} \"{E_NICKNAME}\"\n\
\n\
Usage: {PROGRAM_NAME} [options] [file]\n\
\n\
Run 8 instances of E with different strategies in parallel.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options:\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

fn write_all(output: &mut impl Write, bytes: &[u8]) -> Result<(), Diagnostic> {
    output
        .write_all(bytes)
        .map_err(|error| io_diagnostic(format!("Cannot write output: {error}")))
}

fn writeln_diag(output: &mut impl Write, line: &str) -> Result<(), Diagnostic> {
    write_all(output, line.as_bytes())?;
    write_all(output, b"\n")
}

fn io_diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{
        execute_with_spawner, print_help, process_options, run, strategy_specs, RunCommand,
        StratparConfig, C_USAGE_ERROR, DEFAULT_PROVER, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::control::proc_ctrl::EPCtrl;
    use crate::test_support::global_state_lock;
    use std::process::Command;

    #[test]
    fn help_and_version_exit_before_spawning_children() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let help_status = run([PROGRAM_NAME, "--help"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(help_status, ErrorCode::NO_ERROR.exit_status());
        let help = String::from_utf8(stdout).unwrap();
        assert!(help.contains("Usage: e_stratpar [options] [file]"));
        assert!(help.contains("Run 8 instances of E with different strategies in parallel."));

        let mut stdout = Vec::new();
        let version_status = run([PROGRAM_NAME, "-V"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(version_status, ErrorCode::NO_ERROR.exit_status());
        assert!(String::from_utf8(stdout)
            .unwrap()
            .starts_with("e_stratpar "));
    }

    #[test]
    fn cpu_limit_optional_argument_uses_c_default() {
        let mut stdout = Vec::new();
        let command =
            process_options([PROGRAM_NAME, "--cpu-limit", "problem.p"], &mut stdout).unwrap();
        let RunCommand::Execute(config) = command else {
            panic!("cpu-limit should execute");
        };
        let strategies = strategy_specs(&config);

        assert_eq!(config.cpu_limit, 300);
        assert_eq!(strategies[0].cpu_limit, 150);
    }

    #[test]
    fn strategy_plan_uses_eight_autosched_commands_and_ignores_optional_prover() {
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--cpu-limit=301",
                "problem.p",
                "custom-eprover",
            ],
            &mut stdout,
        )
        .unwrap();
        let RunCommand::Execute(config) = command else {
            panic!("valid stratpar arguments should execute");
        };
        let strategies = strategy_specs(&config);

        assert_eq!(DEFAULT_PROVER, "eprover");
        assert_eq!(config.problem_file, "problem.p");
        assert_eq!(strategies.len(), 8);
        assert_eq!(strategies[0].name, "AutoSched0");
        assert_eq!(strategies[0].options, "-xAutoSched0 -tAutoSched0 --sine");
        assert_eq!(strategies[0].cpu_limit, 150);
        assert_eq!(strategies[7].name, "AutoSched7");
        assert_eq!(strategies[7].options, "-xAutoSched7 -tAutoSched7 --sine");
    }

    #[test]
    fn usage_rejects_missing_and_extra_arguments() {
        let mut stdout = Vec::new();
        let missing = process_options([PROGRAM_NAME], &mut stdout).unwrap_err();
        assert_eq!(missing.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(missing.message(), C_USAGE_ERROR);

        let extra =
            process_options([PROGRAM_NAME, "a.p", "eprover", "extra"], &mut stdout).unwrap_err();
        assert_eq!(extra.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(extra.message(), C_USAGE_ERROR);
    }

    #[test]
    fn execute_with_fake_children_prints_proof_output() {
        let _guard = global_state_lock();
        let config = StratparConfig {
            cpu_limit: 10,
            problem_file: "problem.p".to_owned(),
        };
        let mut stdout = Vec::new();

        let status = execute_with_spawner(&config, &mut stdout, |strategy| {
            EPCtrl::spawn_command(
                pid_status_command("% SZS status Theorem"),
                strategy.name.clone(),
                None,
                strategy.cpu_limit,
            )
        })
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!(printed.contains("% SZS status Theorem"));
        assert!(!printed.contains("% SZS status GaveUp"));
    }

    #[test]
    fn help_text_contains_current_version_and_footer() {
        let help = print_help();

        assert!(help.contains("e_stratpar "));
        assert!(help.contains("Options:"));
        assert!(help.contains("Copyright 1998-2026 by Stephan Schulz"));
    }

    #[cfg(windows)]
    fn pid_status_command(status: &str) -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", &format!("echo % Pid: 123& echo {status}")]);
        command
    }

    #[cfg(unix)]
    fn pid_status_command(status: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", &format!("printf '%s\\n' '% Pid: 123' '{status}'")]);
        command
    }
}
