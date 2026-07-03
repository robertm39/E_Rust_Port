use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::set_verbose_level;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::inout::signals::{e_signal_setup, SignalOutcome, SIGINT_COMPAT, SIGTERM_COMPAT};
use crate::pcl2::proofcheck::{
    protocol_check_with_output_and_warnings, ProofcheckWarningOutput, ProverType,
};
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::PclStepParseOptions;
use crate::prover::version::{footer, VERSION};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "checkproof";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    Silent,
    OutputLevel,
    ProverType,
    Executable,
    TimeLimit,
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
        None,
        Some("version"),
        OptArgType::NoArg,
        None,
        "Print the version number of the program.",
    ),
    OptCell::new(
        OptionCode::Verbose,
        Some('v'),
        Some("verbose"),
        OptArgType::OptArg,
        Some("1"),
        "Verbose comments on the progress of the program.",
    ),
    OptCell::new(
        OptionCode::Output,
        Some('o'),
        Some("output-file"),
        OptArgType::ReqArg,
        None,
        "Redirect output into the named file.",
    ),
    OptCell::new(
        OptionCode::Silent,
        Some('s'),
        Some("silent"),
        OptArgType::NoArg,
        None,
        "Equivalent to --output-level=0.",
    ),
    OptCell::new(
        OptionCode::OutputLevel,
        Some('l'),
        Some("output-level"),
        OptArgType::ReqArg,
        None,
        "Select an output level, greater values imply more verbose output. At the moment, level 0 only prints the result, level 1 prints inference steps as they are verified, level 2 prints prover commands issued, and level 3 prints all prover output (which may be very little)",
    ),
    OptCell::new(
        OptionCode::ProverType,
        Some('p'),
        Some("prover-type"),
        OptArgType::ReqArg,
        None,
        "Set the type of the prover to use for proof verification. Determines problem syntax, options, and check for success. Supported options at are  'E' (the default),'Otter' 'SPASS', and 'scheme-setheo' (not yet implemented). SPASS support is only tested with SPASS 0.55 and may fail if the problem contains identifiers reserved by SPASS. There have been some supple syntax changes, so more recent SPASS versions will probably fail as well.",
    ),
    OptCell::new(
        OptionCode::Executable,
        Some('x'),
        Some("executable"),
        OptArgType::ReqArg,
        None,
        "Give the name under which the prover can be called. If no executable is given, checkproof will guess a name based on the type of the prover. This guess may be way off!",
    ),
    OptCell::new(
        OptionCode::TimeLimit,
        Some('t'),
        Some("prover-cpu-limit"),
        OptArgType::ReqArg,
        None,
        "Limit the CPU time prover may spend on a single step. Default is 10 seconds.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct CheckproofConfig {
    output_file: Option<PathBuf>,
    output_level: i64,
    prover: ProverType,
    executable: Option<String>,
    time_limit: i64,
    files: Vec<String>,
}

impl Default for CheckproofConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            output_level: 1,
            prover: ProverType::EProver,
            executable: None,
            time_limit: 10,
            files: Vec::new(),
        }
    }
}

enum RunCommand {
    Execute(CheckproofConfig),
    Exit(u8),
}

struct ProblemTypeRunGuard;

impl ProblemTypeRunGuard {
    fn new() -> Self {
        reset_problem_type();
        Self
    }
}

impl Drop for ProblemTypeRunGuard {
    fn drop(&mut self) {
        reset_problem_type();
    }
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
    let _problem_type_guard = ProblemTypeRunGuard::new();
    init_io(PROGRAM_NAME);
    setup_signal_handlers()?;
    set_problem_type(ProblemType::FirstOrder)?;
    set_verbose_level(0);
    let _ = set_output_level(1);
    let result = run_inner(argv, stdin, stdout, stderr);
    exit_io();
    result
}

fn run_inner<I, S>(
    argv: I,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_checkproof(&config, stdin, stdout, stderr),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = CheckproofConfig::default();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        match parsed.option().option_code {
            OptionCode::Verbose => {
                let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                set_verbose_level(i64_to_i32_saturating(level));
            }
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Version => {
                writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION}"))?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Output => {
                config.output_file = parsed.arg().map(PathBuf::from);
            }
            OptionCode::Silent => {
                config.output_level = 0;
                let _ = set_output_level(0);
            }
            OptionCode::OutputLevel => {
                let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                config.output_level = level;
                let _ = set_output_level(level);
            }
            OptionCode::ProverType => {
                let arg = parsed.arg().unwrap_or("");
                config.prover = parse_prover_type(arg)?;
            }
            OptionCode::Executable => {
                config.executable = parsed.arg().map(ToOwned::to_owned);
            }
            OptionCode::TimeLimit => {
                config.time_limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn execute_checkproof(
    config: &CheckproofConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = CheckproofOutput::open(config.output_file.as_deref(), stdout)?;
    let mut protocol = PclProtocol::new()?;
    let mut steps = 0_i64;

    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        steps += protocol.parse(&mut scanner, parse_options())?;
        scanner.check_tok(TokenType::NO_TOKEN)?;
    }

    let mut warning_output = ProofcheckWarningOutput::new(stderr, PROGRAM_NAME);
    let summary = protocol_check_with_output_and_warnings(
        &mut output,
        &mut warning_output,
        config.output_level,
        &mut protocol,
        config.prover,
        config.executable.as_deref(),
        config.time_limit,
    )?;

    write_all(
        &mut output,
        format!(
            "% Successfully checked {} of {} steps ({} unchecked): ",
            summary.checked, steps, summary.unchecked
        )
        .as_bytes(),
    )?;
    if summary.checked == steps {
        writeln_diag(&mut output, " Proof verified!")?;
    } else if summary.checked + summary.unchecked == steps {
        writeln_diag(&mut output, " Proof partially verified!")?;
    } else {
        writeln_diag(&mut output, " Failed to verify proof!")?;
    }

    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    stderr
        .flush()
        .map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(0)
}

fn parse_options() -> PclStepParseOptions {
    PclStepParseOptions {
        problem_type: ProblemType::FirstOrder,
        support_shell_pcl: true,
        ..PclStepParseOptions::default()
    }
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    let mut scanner = if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        Scanner::from_file_content("-", data, true)?
    } else {
        Scanner::from_file(Path::new(name), true).map_err(checkproof_scanner_open_diagnostic)?
    };
    scanner.set_format(IoFormat::Tptp);
    Ok(scanner)
}

fn parse_prover_type(arg: &str) -> Result<ProverType, Diagnostic> {
    match arg {
        "E" => Ok(ProverType::EProver),
        "Otter" => Ok(ProverType::Otter),
        "SPASS" => Ok(ProverType::Spass),
        "scheme-setheo" => Ok(ProverType::Setheo),
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Option -p (--prover-type) requires E, Otter, SPASS or scheme-setheo as an argument",
        )),
    }
}

fn setup_signal_handlers() -> Result<(), Diagnostic> {
    for signal in [SIGTERM_COMPAT, SIGINT_COMPAT] {
        if let SignalOutcome::HandlerInstallFailed { diagnostic, .. } = e_signal_setup(signal) {
            return Err(diagnostic);
        }
    }
    Ok(())
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
\n\
{PROGRAM_NAME} {VERSION}\n\
\n\
Usage: {PROGRAM_NAME} [options] [files]\n\
\n\
Read an UPCL2 protocol and verify the inferences using one of a\n\
varity of external provers.\n\
\n\
This is a _very_ experimental program. Passing checkproof does\n\
indicate that all inferences in an UPCL2 protocol are correct\n\
(i.e. that the conclusion is logically implied by the premisses) -\n\
that is, if you believe that the transformation process and the used\n\
prover are correct. However, checkproof will e.g. gladly show that the\n\
empty proof protocol does not contain any buggy steps.\n\
\n\
If a proof protocol fails to pass this test, the proof may still be\n\
correct. Due to e.g. incomplete strategies (this applies in particular\n\
to Otter), build-in limits (Otter), and bugs in the prover (potentially\n\
all systems, but observed in SPASS 0.55), a prover might fail to\n\
verify a correct step. Moreover, due to the different strategies,\n\
calculi, and in particular different term orderings chosen by the\n\
systems, a single UPCL2 inference may result in a proof problem that\n\
is very hard to verify for other provers. However, if a proof step is\n\
rejected by more than one system, you should probably look at this\n\
step in detail.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

enum CheckproofOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> CheckproofOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        File::create(path).map(Self::File).map_err(|error| {
            checkproof_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
    }
}

impl<W: Write> Write for CheckproofOutput<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Stdout(output) => output.write(buffer),
            Self::File(file) => file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Stdout(output) => output.flush(),
            Self::File(file) => file.flush(),
        }
    }
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

fn checkproof_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn checkproof_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
    if error.code() != ErrorCode::FILE_ERROR || !error.message().starts_with("Cannot open file ") {
        return error;
    }
    let Some((prefix, source_error)) = error.message().split_once(": ") else {
        return error;
    };
    Diagnostic::new(
        error.code(),
        format!("{prefix}\n{PROGRAM_NAME}: {source_error}"),
    )
}

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::{print_help, run, OUTPUT_CLOSE_ERROR, PROGRAM_NAME};
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::prover::version::VERSION;
    use crate::test_support::global_state_lock;
    use std::io::{self, Cursor, Write};
    use std::path::{Path, PathBuf};

    const ASSUMPTION_PROTOCOL: &str = "1 : : [++p(a)] : initial\n";
    const PARTIAL_PROTOCOL: &str = "\
1 : : [++p(a)] : initial
2 : : [++q(a)] : 1
3 : : [++r(a)] : split(2)
";

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("checkproof-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("checkproof run succeeds");
        (
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        )
    }

    #[test]
    fn help_and_version_exit_before_processing_input() {
        let _guard = global_state_lock();
        let (status, help, stderr) = run_with_stdin(&[PROGRAM_NAME, "--help"], "not pcl");
        assert_eq!(status, 0);
        assert!(help.starts_with(&format!("\n\n{PROGRAM_NAME} {VERSION}\n\n")));
        assert!(help.contains("Usage: checkproof [options] [files]"));
        assert!(help.contains("Read an UPCL2 protocol and verify the inferences"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_stdin(&[PROGRAM_NAME, "--version"], "not pcl");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn short_v_is_not_a_version_option() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME, "-V"], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("C checkproof has no -V shorthand");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("Unknown Option: -V"));
        assert!(stdout.is_empty());
    }

    #[test]
    fn default_eprover_checks_assumptions_without_external_process() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], ASSUMPTION_PROTOCOL);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("% Checking       1 :  : [++p(a)] : initial\n"));
        assert!(output.contains("% Checked (by assumption)\n\n"));
        assert!(output
            .ends_with("% Successfully checked 1 of 1 steps (0 unchecked):  Proof verified!\n"));
    }

    #[test]
    fn scheme_setheo_reports_unimplemented_steps_as_partial() {
        let _guard = global_state_lock();
        let (status, output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "-p", "scheme-setheo"], PARTIAL_PROTOCOL);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("% Checked (by assumption)\n\n"));
        assert!(output.contains("% Check not implemented, assuming true!\n\n"));
        assert!(output.ends_with(
            "% Successfully checked 1 of 3 steps (2 unchecked):  Proof partially verified!\n"
        ));
    }

    #[test]
    fn silent_output_level_keeps_only_final_result() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--silent", "-p", "scheme-setheo"],
            PARTIAL_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(!output.contains("% Checking"));
        assert_eq!(
            output,
            "% Successfully checked 1 of 3 steps (2 unchecked):  Proof partially verified!\n"
        );
    }

    #[test]
    fn output_file_receives_trace_and_summary() {
        let _guard = global_state_lock();
        let input_path = temp_path("input");
        let output_path = temp_path("output");
        remove_if_present(&input_path);
        remove_if_present(&output_path);
        std::fs::write(&input_path, PARTIAL_PROTOCOL).expect("input fixture is written");

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "-p",
                "scheme-setheo",
                "-o",
                output_path.to_str().expect("path is utf8"),
                input_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("file run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let output = std::fs::read_to_string(&output_path).expect("output file is readable");
        assert!(output.contains("% Checking       1 :  : [++p(a)] : initial\n"));
        assert!(output.contains("% Successfully checked 1 of 3 steps"));

        remove_if_present(&input_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn output_dash_routes_to_stdout_like_c() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "-p", "scheme-setheo", "-o", "-"],
            PARTIAL_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("% Checking       1 :  : [++p(a)] : initial\n"));
        assert!(output.ends_with(
            "% Successfully checked 1 of 3 steps (2 unchecked):  Proof partially verified!\n"
        ));
    }

    #[test]
    fn verbose_and_output_level_options_set_global_compatible_state() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--verbose=3", "-l", "0"],
            ASSUMPTION_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert_eq!(verbose_level(), 3);
        assert_eq!(
            output,
            "% Successfully checked 1 of 1 steps (0 unchecked):  Proof verified!\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn invalid_prover_type_reports_usage_error() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--prover-type=bad"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("invalid prover type is rejected");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error
            .message()
            .contains("requires E, Otter, SPASS or scheme-setheo"));
        assert!(stdout.is_empty());
    }

    #[test]
    fn trailing_input_reports_syntax_error() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"1 : : [++p] : initial\ntrailing\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("trailing input is rejected");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("No token"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn input_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_path = temp_path("missing-input");
        remove_if_present(&missing_path);
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, missing_path.to_str().expect("path is utf8")],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing input file is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_path.display()
        )));
        assert!(error.message().contains(&format!("\n{PROGRAM_NAME}: ")));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn output_file_is_created_before_later_input_open_failure() {
        let _guard = global_state_lock();
        let output_path = temp_path("early-output");
        let missing_path = temp_path("missing-after-output");
        remove_if_present(&output_path);
        remove_if_present(&missing_path);
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "-o",
                output_path.to_str().expect("path is utf8"),
                missing_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing input file is reported after output creation");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().starts_with(&format!(
            "Cannot open file {} for reading",
            missing_path.display()
        )));
        assert!(output_path.exists());
        assert_eq!(
            std::fs::read_to_string(&output_path).expect("output file is readable"),
            ""
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        remove_if_present(&output_path);
    }

    #[test]
    fn output_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let output_path = temp_path("output-dir");
        remove_if_present(&output_path);
        _ = std::fs::remove_dir(&output_path);
        std::fs::create_dir(&output_path).expect("output fixture directory is created");
        let mut stdin = Cursor::new(ASSUMPTION_PROTOCOL.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "-o",
                output_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("directory output path is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error
            .message()
            .starts_with(&format!("Cannot open file {}", output_path.display())));
        assert!(error.message().contains(&format!("\n{PROGRAM_NAME}: ")));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        std::fs::remove_dir(output_path).expect("output fixture directory is removed");
    }

    #[test]
    fn output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(ASSUMPTION_PROTOCOL.as_bytes().to_vec());
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    #[test]
    fn help_text_preserves_c_usage_summary() {
        let rendered = print_help();

        assert!(rendered.contains("varity of external provers."));
        assert!(rendered.contains("empty proof protocol does not contain any buggy steps."));
        assert!(rendered.contains("Copyright 1998-2026 by Stephan Schulz"));
    }
}
