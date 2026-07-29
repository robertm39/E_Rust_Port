use crate::basics::error::{Diagnostic, ErrorCode};
use crate::control::proc_ctrl::{EPCtrl, EPCtrlSet, MAX_CORES};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::prover::version::{footer, VERSION, VERSION_QUALIFIER};
use std::io::Write;

pub const PROGRAM_NAME: &str = "umlaut-stratpar";

const DEFAULT_PROVER: &str = "umlaut";
const DEFAULT_HARD_TIME_LIMIT: i64 = 3600;
const C_USAGE_ERROR: &str = "Usage: umlaut-ltb-runner <spec> [<path-to-umlaut>]";
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

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
                writeln_diag(
                    stdout,
                    &format!("{PROGRAM_NAME} {VERSION} {VERSION_QUALIFIER}"),
                )?;
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
    execute_process_set_with_poll(controls, stdout, |controls, output| {
        controls.get_result(false, output)
    })
}

fn execute_process_set_with_poll<W, F>(
    controls: &mut EPCtrlSet,
    stdout: &mut W,
    mut poll: F,
) -> Result<u8, Diagnostic>
where
    W: Write,
    F: FnMut(
        &mut EPCtrlSet,
        &mut W,
    ) -> Result<Option<crate::control::session::Descriptor>, Diagnostic>,
{
    let mut proof_descriptor = None;
    while !controls.is_empty() {
        proof_descriptor = poll(controls, stdout)?;
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
    stdout
        .flush()
        .map_err(|_error| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
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
{PROGRAM_NAME} {VERSION} {VERSION_QUALIFIER}\n\
\n\
Usage: {PROGRAM_NAME} [options] [file]\n\
\n\
Run 8 instances of Umlaut with different strategies in parallel.\n\
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
        execute_process_set_with_poll, execute_with_spawner, print_help, process_options, run,
        strategy_specs, RunCommand, StratparConfig, C_USAGE_ERROR, DEFAULT_PROVER,
        OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::control::proc_ctrl::{EPCtrl, EPCtrlSet};
    use crate::control::session::{Descriptor, DescriptorInterestSet};
    use crate::prover::version::{assert_help_matches_fixture, footer, VERSION, VERSION_QUALIFIER};
    use crate::test_support::global_state_lock;
    use std::io::{self, Write};
    use std::process::Command;

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }
    }

    fn expected_help() -> String {
        let mut expected = format!(
            concat!(
                "\n",
                "umlaut-stratpar {version} {nickname}\n",
                "\n",
                "Usage: umlaut-stratpar [options] [file]\n",
                "\n",
                "Run 8 instances of Umlaut with different strategies in parallel.\n",
                "\n",
                "Options:\n",
                "\n",
                "   -h\n",
                "  --help\n",
                "    Print a short description of program usage and options.\n",
                "\n",
                "   -V\n",
                "  --version\n",
                "    Print the version number of the prover. Please include this with all bug\n",
                "    reports (if any).\n",
                "\n",
                "  --cpu-limit[=<arg>]\n",
                "    Limit the cpu time the prover should run. The optional argument is the\n",
                "    CPU time in seconds. The option without the optional argument is\n",
                "    equivalent to --cpu-limit=300.\n",
                "\n",
                "\n",
                "\n",
            ),
            version = VERSION,
            nickname = VERSION_QUALIFIER,
        );
        expected.push_str(&footer());
        expected
    }

    #[test]
    fn help_and_version_exit_before_spawning_children() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let help_status = run([PROGRAM_NAME, "--help"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(help_status, ErrorCode::NO_ERROR.exit_status());
        assert_help_matches_fixture(
            &String::from_utf8(std::mem::take(&mut stdout)).unwrap(),
            &expected_help(),
        );

        let version_status = run([PROGRAM_NAME, "-V"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(version_status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!("{PROGRAM_NAME} {VERSION} {VERSION_QUALIFIER}\n")
        );
    }

    #[test]
    fn print_help_preserves_full_c_text() {
        assert_help_matches_fixture(&print_help(), &expected_help());
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
                "custom-umlaut",
            ],
            &mut stdout,
        )
        .unwrap();
        let RunCommand::Execute(config) = command else {
            panic!("valid stratpar arguments should execute");
        };
        let strategies = strategy_specs(&config);

        assert_eq!(DEFAULT_PROVER, "umlaut");
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
            process_options([PROGRAM_NAME, "a.p", "umlaut", "extra"], &mut stdout).unwrap_err();
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
    fn execute_with_no_proof_children_prints_child_failures_and_gaveup() {
        let _guard = global_state_lock();
        let config = StratparConfig {
            cpu_limit: 10,
            problem_file: "problem.p".to_owned(),
        };
        let mut stdout = Vec::new();

        let status = execute_with_spawner(&config, &mut stdout, |strategy| {
            EPCtrl::spawn_command(
                pid_status_command("% SZS status GaveUp"),
                strategy.name.clone(),
                None,
                strategy.cpu_limit,
            )
        })
        .unwrap();

        let printed = String::from_utf8(stdout).unwrap();
        let no_proof_messages = printed.matches("% No proof found by AutoSched").count();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert!((1..=8).contains(&no_proof_messages));
        assert!(!printed.contains("% SZS status Theorem"));
        assert!(printed.ends_with("% SZS status GaveUp\n"));
    }

    #[test]
    fn simultaneous_proofs_replay_highest_ready_descriptor_like_c() {
        let _guard = global_state_lock();
        let mut controls = EPCtrlSet::new();
        controls
            .add_proc(EPCtrl::with_descriptor("lower", Descriptor::new(2)))
            .unwrap();
        controls
            .add_proc(EPCtrl::with_descriptor("higher", Descriptor::new(7)))
            .unwrap();
        let mut stdout = Vec::new();
        let mut polls = 0;

        let status =
            execute_process_set_with_poll(&mut controls, &mut stdout, |controls, output| {
                polls += 1;
                let mut ready = DescriptorInterestSet::default();
                ready.set_read(Descriptor::new(2));
                ready.set_read(Descriptor::new(7));
                controls.get_result_from_ready(&ready, false, output, |proc, _buffer| {
                    let line = format!("% output from {}\n% SZS status Theorem\n", proc.name());
                    let _eof = proc.get_result_from_optional_line(Some(&line));
                    Ok(proc.get_result_from_optional_line(None))
                })
            })
            .unwrap();

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(polls, 1);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "% output from higher\n% SZS status Theorem\n"
        );
    }

    #[test]
    fn execute_reports_final_outclose_flush_failure_like_c() {
        let _guard = global_state_lock();
        let config = StratparConfig {
            cpu_limit: 10,
            problem_file: "problem.p".to_owned(),
        };
        let mut stdout = FlushFailWriter;

        let error = execute_with_spawner(&config, &mut stdout, |strategy| {
            EPCtrl::spawn_command(
                pid_status_command("% SZS status GaveUp"),
                strategy.name.clone(),
                None,
                strategy.cpu_limit,
            )
        })
        .expect_err("final flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
    }

    #[test]
    fn help_text_contains_current_version_and_footer() {
        let help = print_help();

        assert!(help.contains("umlaut-stratpar "));
        assert!(help.contains("Options:"));
        assert!(help.contains("E copyright 1998-2026 by Stephan Schulz"));
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
