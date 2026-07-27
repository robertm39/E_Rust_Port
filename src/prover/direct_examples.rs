use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{c_io_error_message, Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::{set_verbose_level, verbout2};
use crate::clauses::clause::ClauseParseOptions;
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::inout::commandline::{
    get_float_arg, get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::pcl2::analysis::{
    protocol_proof_distance, protocol_select_examples, protocol_update_grefs,
};
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStepParseOptions, PCL_IS_INITIAL, PCL_IS_PROOF_STEP};
use crate::prover::version::{footer, VERSION};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "umlaut-direct-examples";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    NegativeNumber,
    NegativeProportion,
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
        OptionCode::NegativeNumber,
        Some('n'),
        Some("negative-example-number"),
        OptArgType::ReqArg,
        None,
        "Set the (maximum) number of negative examples to pick if the proof listing does not describe a successful proof.",
    ),
    OptCell::new(
        OptionCode::NegativeProportion,
        Some('p'),
        Some("negative-example-proportion"),
        OptArgType::ReqArg,
        None,
        "Set the maximum number of negative examples (expressed as a proportion of the positive examples) to pick if the proof listing does describe a successful proof",
    ),
];

#[derive(Clone, Debug, PartialEq)]
struct DirectExamplesConfig {
    output_file: Option<PathBuf>,
    neg_proportion: f64,
    neg_examples: i64,
    files: Vec<String>,
}

impl Default for DirectExamplesConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            neg_proportion: 1.0,
            neg_examples: 200,
            files: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(DirectExamplesConfig),
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
    set_problem_type(ProblemType::FirstOrder)?;
    set_verbose_level(0);
    let result = run_inner(argv, stdin, stdout, stderr);
    exit_io();
    stderr
        .flush()
        .map_err(|error| io_diagnostic(error.to_string()))?;
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
        RunCommand::Execute(config) => execute_direct_examples(&config, stdin, stdout, stderr),
    }
}

fn process_options<I, S>(command_line: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(command_line);
    let mut config = DirectExamplesConfig::default();

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
            OptionCode::NegativeNumber => {
                config.neg_examples = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
            OptionCode::NegativeProportion => {
                config.neg_proportion = get_float_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if config.neg_proportion < 0.0 {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        "Option -p (--negative-example-proportion)requires positive argument.}",
                    ));
                }
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn execute_direct_examples(
    config: &DirectExamplesConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output_file = open_output_file(config.output_file.as_deref())?;
    let mut protocol = PclProtocol::new()?;
    for input_file in &config.files {
        let mut scanner = scanner_for_input(input_file, stdin)?;
        protocol.parse(&mut scanner, parse_options())?;
        scanner.check_tok(TokenType::NO_TOKEN)?;
    }
    verbout2_diag(stderr, "PCL input read\n")?;

    let rendered =
        generate_direct_examples_output(&mut protocol, config.neg_proportion, config.neg_examples)?;
    write_output(output_file.as_mut(), stdout, &rendered)?;
    flush_output(output_file.as_mut(), stdout)?;
    Ok(0)
}

fn generate_direct_examples_output(
    protocol: &mut PclProtocol,
    neg_proportion: f64,
    neg_examples: i64,
) -> Result<String, Diagnostic> {
    protocol.strip_fof()?;
    protocol.reset_tree_data(false);
    protocol.mark_proof_clauses()?;
    protocol_proof_distance(protocol)?;
    protocol_update_grefs(protocol);
    let proof_steps = protocol.count_property(PCL_IS_PROOF_STEP);
    let neg_steps = negative_example_budget(neg_proportion, neg_examples, proof_steps);
    let _visited = protocol_select_examples(protocol, neg_steps);

    let mut generated = String::new();
    generated.push_str(DEFAULT_COMCHAR_RAW);
    generated.push_str(" Axioms:\n");
    generated.push_str(&protocol.print_property_steps_string(
        PCL_IS_INITIAL,
        ProofDocOutputFormat::Lop,
        ProblemType::FirstOrder,
    )?);
    generated.push_str(".\n\n");
    generated.push_str(DEFAULT_COMCHAR_RAW);
    generated.push_str(" Examples:\n");
    generated.push_str(&protocol.print_examples_string()?);
    Ok(generated)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn negative_example_budget(neg_proportion: f64, neg_examples: i64, proof_steps: i64) -> i64 {
    if proof_steps == 0 {
        neg_examples
    } else {
        (neg_proportion * proof_steps as f64) as i64
    }
}

fn parse_options() -> PclStepParseOptions {
    PclStepParseOptions {
        problem_type: ProblemType::FirstOrder,
        support_shell_pcl: false,
        clause_parse_options: ClauseParseOptions {
            clauses_have_local_variables: false,
            ..ClauseParseOptions::default()
        },
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
        Scanner::from_file(Path::new(name), true).map_err(direct_examples_input_open_diagnostic)?
    };
    scanner.set_format(IoFormat::Tptp);
    Ok(scanner)
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
Parse a full PCL listing (possibly\n\
spread over multiple files), and generate training examples\n\
corresponding to the selected clauses.\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

fn open_output_file(path: Option<&Path>) -> Result<Option<std::fs::File>, Diagnostic> {
    let Some(path) = path else {
        return Ok(None);
    };
    if path == Path::new("-") {
        return Ok(None);
    }
    std::fs::File::create(path).map(Some).map_err(|error| {
        direct_examples_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
    })
}

fn write_output(
    file: Option<&mut std::fs::File>,
    stdout: &mut impl Write,
    contents: &str,
) -> Result<(), Diagnostic> {
    if let Some(file) = file {
        write_all(file, contents.as_bytes())
    } else {
        write_all(stdout, contents.as_bytes())
    }
}

fn flush_output(
    file: Option<&mut std::fs::File>,
    stdout: &mut impl Write,
) -> Result<(), Diagnostic> {
    let result = if let Some(file) = file {
        file.flush()
    } else {
        stdout.flush()
    };
    result.map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))
}

fn verbout2_diag(output: &mut impl Write, message: &str) -> Result<(), Diagnostic> {
    let _ = verbout2(output, PROGRAM_NAME, message)
        .map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(())
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

fn direct_examples_sys_error_diagnostic(
    prefix: impl Into<String>,
    error: &io::Error,
) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!(
            "{}\n{PROGRAM_NAME}: {}",
            prefix.into(),
            c_io_error_message(error)
        ),
    )
}

fn direct_examples_input_open_diagnostic(error: Diagnostic) -> Diagnostic {
    if error.code() != ErrorCode::FILE_ERROR
        || !(error.message().starts_with("Cannot stat file ")
            || error.message().starts_with("Cannot open file "))
    {
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
    use super::{
        negative_example_budget, parse_options, print_help, process_options, run,
        DirectExamplesConfig, RunCommand, OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::prover::version::VERSION;
    use crate::test_support::global_state_lock;
    use std::io::{self, Cursor, Write};
    use std::path::{Path, PathBuf};

    const NO_PROOF_PROTOCOL: &str = "\
1 : : [++p(a)] : initial
2 : : [++q(a)] : 1
";

    const PROOF_PROTOCOL: &str = "\
1 : : [++p(a)] : initial
2 : : [++q(a)] : 1
3 : : [] : 2 : 'final'
";

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("direct-examples-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
        }
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("umlaut-direct-examples run succeeds");
        (
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        )
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let (status, help, stderr) = run_with_stdin(&[PROGRAM_NAME, "--help"], "ignored");

        assert_eq!(status, 0);
        assert!(help.starts_with(&format!("\n\n{PROGRAM_NAME} {VERSION}\n\n")));
        assert!(help.contains("Usage: umlaut-direct-examples [options] [files]"));
        assert!(help.contains("generate training examples"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_stdin(&[PROGRAM_NAME, "-V"], "ignored");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_negative_selection_options() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose=2",
                "--output-file=examples.out",
                "--negative-example-number=7",
                "--negative-example-proportion=2.5",
                "proof.pcl",
            ],
            &mut stdout,
        )
        .expect("options parse");

        let RunCommand::Execute(DirectExamplesConfig {
            output_file,
            neg_proportion,
            neg_examples,
            files,
        }) = command
        else {
            panic!("expected execute command");
        };
        assert_eq!(
            output_file
                .as_ref()
                .and_then(|path| path.to_str())
                .expect("output path utf8"),
            "examples.out"
        );
        assert_eq!(neg_proportion.to_bits(), 2.5_f64.to_bits());
        assert_eq!(neg_examples, 7);
        assert_eq!(files, ["proof.pcl"]);
        assert_eq!(verbose_level(), 2);
        assert!(stdout.is_empty());
    }

    #[test]
    fn invalid_negative_proportion_keeps_c_typo() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let error = process_options([PROGRAM_NAME, "-p", "-0.1"], &mut stdout).unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option -p (--negative-example-proportion)requires positive argument.}"
        );
        assert!(stdout.is_empty());
    }

    #[test]
    fn stdin_run_prints_axioms_and_examples() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], NO_PROOF_PROTOCOL);

        assert_eq!(status, 0);
        assert!(output.starts_with("% Axioms:\n"));
        assert!(output.contains("p(a) <- .\n.\n\n% Examples:\n"));
        assert!(output.contains("0:(10, 0.000000,0.000000,0.000000,0.000000):p(a) <- ."));
        assert!(output.contains("1:(10, 0.000000,0.000000,0.000000,0.000000):q(a) <- ."));
        assert!(stderr.is_empty());
    }

    #[test]
    fn proof_run_uses_proof_step_proportion() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--negative-example-proportion=0"],
            PROOF_PROTOCOL,
        );

        assert_eq!(status, 0);
        assert!(output.contains("% Axioms:\n"));
        assert!(output.contains("% Examples:\n"));
        assert!(!output.contains("0:("));
        assert!(stderr.is_empty());
    }

    #[test]
    fn output_file_receives_generated_examples_after_concatenated_inputs() {
        let _guard = global_state_lock();
        let input_a_path = temp_path("input-a");
        let input_b_path = temp_path("input-b");
        let output_path = temp_path("output");
        remove_if_present(&input_a_path);
        remove_if_present(&input_b_path);
        remove_if_present(&output_path);
        std::fs::write(&input_a_path, "1 : : [++p(a)] : initial\n")
            .expect("first input fixture is written");
        std::fs::write(&input_b_path, "2 : : [++q(a)] : 1\n")
            .expect("second input fixture is written");

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "-o",
                output_path.to_str().expect("path is utf8"),
                input_a_path.to_str().expect("path is utf8"),
                input_b_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("file run succeeds");

        assert_eq!(status, 0);
        assert!(String::from_utf8(stdout)
            .expect("stdout is utf8")
            .is_empty());
        assert!(String::from_utf8(stderr)
            .expect("stderr is utf8")
            .is_empty());
        let output = std::fs::read_to_string(&output_path).expect("output file is readable");
        assert!(output.starts_with("% Axioms:\n"));
        assert!(output.contains("% Examples:\n"));

        remove_if_present(&input_a_path);
        remove_if_present(&input_b_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn output_dash_routes_to_stdout_like_c() {
        let _guard = global_state_lock();
        let (status, output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "-o", "-"], NO_PROOF_PROTOCOL);

        assert_eq!(status, 0);
        assert!(output.starts_with("% Axioms:\n"));
        assert!(output.contains("% Examples:\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn missing_input_file_uses_c_stat_syserror_shape() {
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
        assert!(error
            .message()
            .starts_with(&format!("Cannot stat file {}", missing_path.display())));
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
        assert!(error
            .message()
            .starts_with(&format!("Cannot stat file {}", missing_path.display())));
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
        let mut stdin = Cursor::new(NO_PROOF_PROTOCOL.as_bytes().to_vec());
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
        let mut stdin = Cursor::new(NO_PROOF_PROTOCOL.as_bytes().to_vec());
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    #[test]
    fn verbose_run_reports_pcl_read_message_at_level_two() {
        let _guard = global_state_lock();
        let (status, _output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--verbose=2"], NO_PROOF_PROTOCOL);

        assert_eq!(status, 0);
        assert_eq!(stderr, "umlaut-direct-examples: PCL input read\n");
    }

    #[test]
    fn compressed_pcl_input_shares_external_variable_names_like_c() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[PROGRAM_NAME],
            "1 : : [++p(X,Y)] : initial\n2 : : [++q(Y,X)] : initial\n",
        );

        assert_eq!(status, 0);
        assert!(output.contains("p(X1,X2) <- ."));
        assert!(output.contains("q(X2,X1) <- ."));
        assert!(stderr.is_empty());
    }

    #[test]
    fn parse_options_disable_local_clause_variables_like_c() {
        assert!(
            !parse_options()
                .clause_parse_options
                .clauses_have_local_variables
        );
    }

    #[test]
    fn helper_preserves_c_negative_budget_branch() {
        assert_eq!(negative_example_budget(3.5, 11, 0), 11);
        assert_eq!(negative_example_budget(1.5, 11, 3), 4);
    }

    #[test]
    fn print_help_mentions_training_examples() {
        assert!(print_help().contains("generate training examples"));
    }
}
