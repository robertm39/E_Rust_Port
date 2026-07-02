use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::inferencedoc::ProofDocOutputFormat;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::pcl2::miniprotocol::PclMiniProtocol;
use crate::pcl2::ministeps::PclMiniStepParseOptions;
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::{PclStepParseOptions, PCL_IS_PROOF_STEP};
use crate::prover::version::{footer, VERSION};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "epclextract";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    FastExtract,
    ForwardComments,
    TstpPrint,
    Competition,
    NoExtract,
    Output,
    Silent,
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
        OptionCode::FastExtract,
        Some('f'),
        Some("fast-extract"),
        OptArgType::NoArg,
        None,
        "Do a fast extract. With this option the program understands only a subset of PCL and assumes that all \"proof\" and \"final\" steps are at the end of the protocoll.",
    ),
    OptCell::new(
        OptionCode::ForwardComments,
        Some('C'),
        Some("forward-comments"),
        OptArgType::NoArg,
        None,
        "Pass comments found in the input through to the output while reading input.",
    ),
    OptCell::new(
        OptionCode::Competition,
        Some('c'),
        Some("competition-framing"),
        OptArgType::NoArg,
        None,
        "Print special \"begin\" and \"end\"comments around the proof object, as required by the CASC MIX* class.",
    ),
    OptCell::new(
        OptionCode::NoExtract,
        Some('n'),
        Some("no-extract"),
        OptArgType::NoArg,
        None,
        "Don't extract, print back all steps (actually, it treats all steps as proof steps). Useful as a syntax checker, or if you want to convert PCL to TSTP with the next option.",
    ),
    OptCell::new(
        OptionCode::TstpPrint,
        None,
        Some("tstp-out"),
        OptArgType::NoArg,
        None,
        "Print proof protocol in TSTP syntax (default is PCL).",
    ),
    OptCell::new(
        OptionCode::TstpPrint,
        None,
        Some("tptp3-out"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-out.",
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
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExtractConfig {
    output_file: Option<PathBuf>,
    mode: ExtractMode,
    framing: FramingMode,
    selection: SelectionMode,
    comments: CommentMode,
    output_format: ProofDocOutputFormat,
    files: Vec<String>,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            mode: ExtractMode::Full,
            framing: FramingMode::Plain,
            selection: SelectionMode::ProofClosure,
            comments: CommentMode::Skip,
            output_format: ProofDocOutputFormat::Pcl,
            files: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExtractMode {
    Full,
    Fast,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FramingMode {
    Plain,
    Competition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionMode {
    ProofClosure,
    AllSteps,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommentMode {
    Skip,
    Forward,
}

impl CommentMode {
    const fn ignore_comments(self) -> bool {
        matches!(self, Self::Skip)
    }
}

enum RunCommand {
    Execute(ExtractConfig),
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
        RunCommand::Execute(config) => execute_extract(&config, stdin, stdout, stderr),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = ExtractConfig::default();

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
            OptionCode::FastExtract => {
                config.mode = ExtractMode::Fast;
            }
            OptionCode::ForwardComments => {
                config.comments = CommentMode::Forward;
            }
            OptionCode::Competition => {
                config.framing = FramingMode::Competition;
            }
            OptionCode::NoExtract => {
                config.selection = SelectionMode::AllSteps;
            }
            OptionCode::TstpPrint => {
                config.output_format = ProofDocOutputFormat::Tstp;
            }
            OptionCode::Output => {
                config.output_file = parsed.arg().map(PathBuf::from);
            }
            OptionCode::Silent => {}
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn execute_extract(
    config: &ExtractConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = ExtractOutput::open(config.output_file.as_deref(), stdout)?;
    match config.mode {
        ExtractMode::Fast => execute_fast_extract(config, stdin, &mut output)?,
        ExtractMode::Full => execute_full_extract(config, stdin, &mut output)?,
    }
    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    stderr
        .flush()
        .map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(0)
}

fn execute_fast_extract(
    config: &ExtractConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
) -> Result<(), Diagnostic> {
    let mut protocol = PclMiniProtocol::new()?;
    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin, config.comments.ignore_comments())?;
        if config.comments == CommentMode::Forward {
            protocol.parse_with_output(output, &mut scanner, mini_parse_options())?;
        } else {
            protocol.parse(&mut scanner, mini_parse_options())?;
        }
        scanner.check_tok(TokenType::NO_TOKEN)?;
    }

    let empty_clause = if config.selection == SelectionMode::AllSteps {
        protocol.set_clause_property(PCL_IS_PROOF_STEP);
        false
    } else {
        protocol.mark_proof_clauses(true)?
    };
    write_competition_start(config, output, empty_clause)?;
    write_all(
        output,
        protocol
            .print_proof_clauses_string(config.output_format, ProblemType::FirstOrder)?
            .as_bytes(),
    )?;
    write_competition_end(config, output, empty_clause)
}

fn execute_full_extract(
    config: &ExtractConfig,
    stdin: &mut impl Read,
    output: &mut impl Write,
) -> Result<(), Diagnostic> {
    let mut protocol = PclProtocol::new()?;
    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin, config.comments.ignore_comments())?;
        if config.comments == CommentMode::Forward {
            protocol.parse_with_output(output, &mut scanner, full_parse_options())?;
        } else {
            protocol.parse(&mut scanner, full_parse_options())?;
        }
        scanner.check_tok(TokenType::NO_TOKEN)?;
    }

    let empty_clause = if config.selection == SelectionMode::AllSteps {
        protocol.set_property(PCL_IS_PROOF_STEP);
        false
    } else {
        protocol.mark_proof_clauses()?
    };
    write_competition_start(config, output, empty_clause)?;
    write_all(
        output,
        protocol
            .print_property_steps_string(
                PCL_IS_PROOF_STEP,
                config.output_format,
                ProblemType::FirstOrder,
            )?
            .as_bytes(),
    )?;
    write_competition_end(config, output, empty_clause)
}

fn full_parse_options() -> PclStepParseOptions {
    PclStepParseOptions {
        problem_type: ProblemType::FirstOrder,
        support_shell_pcl: true,
    }
}

fn mini_parse_options() -> PclMiniStepParseOptions {
    PclMiniStepParseOptions {
        problem_type: ProblemType::FirstOrder,
        support_shell_pcl: true,
    }
}

fn write_competition_start(
    config: &ExtractConfig,
    output: &mut impl Write,
    empty_clause: bool,
) -> Result<(), Diagnostic> {
    if config.framing == FramingMode::Plain {
        return Ok(());
    }
    if config.selection == SelectionMode::AllSteps {
        writeln_diag(
            output,
            &format!("{DEFAULT_COMCHAR_RAW} SZS output start Derivation."),
        )
    } else if empty_clause {
        writeln_diag(
            output,
            &format!("{DEFAULT_COMCHAR_RAW} SZS output start CNFRefutation."),
        )
    } else {
        writeln_diag(
            output,
            &format!("{DEFAULT_COMCHAR_RAW} SZS output start Saturation."),
        )
    }
}

fn write_competition_end(
    config: &ExtractConfig,
    output: &mut impl Write,
    empty_clause: bool,
) -> Result<(), Diagnostic> {
    if config.framing == FramingMode::Plain {
        return Ok(());
    }
    if config.selection == SelectionMode::AllSteps {
        writeln_diag(
            output,
            &format!("{DEFAULT_COMCHAR_RAW} SZS output end Derivation."),
        )
    } else if empty_clause {
        writeln_diag(
            output,
            &format!("{DEFAULT_COMCHAR_RAW} SZS output end CNFRefutation"),
        )
    } else {
        writeln_diag(
            output,
            &format!("{DEFAULT_COMCHAR_RAW} SZS output end Saturation."),
        )
    }
}

fn scanner_for_input(
    name: &str,
    stdin: &mut impl Read,
    ignore_comments: bool,
) -> Result<Scanner, Diagnostic> {
    let mut scanner = if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        Scanner::from_file_content("-", data, ignore_comments)?
    } else {
        Scanner::from_file(Path::new(name), ignore_comments)
            .map_err(epclextract_scanner_open_diagnostic)?
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
Read an PCL2 protocol and print the steps necessary for proving the clauses in \"proof\", \"final\", or \"extract\" steps.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

enum ExtractOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> ExtractOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        File::create(path).map(Self::File).map_err(|error| {
            epclextract_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
    }
}

impl<W: Write> Write for ExtractOutput<'_, W> {
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

fn epclextract_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn epclextract_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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
    use crate::prover::version::VERSION;
    use crate::test_support::global_state_lock;
    use std::io::{self, Cursor, Write};
    use std::path::{Path, PathBuf};

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
        }
    }

    const SAMPLE_PROTOCOL: &str = "\
1 : : [++p] : initial
2 : lemma : [++q] : 1
3 : : [] : 2 : 'final'
4 : : [++r] : initial
";

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("epclextract-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("epclextract run succeeds");
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
        assert!(help.contains("Usage: epclextract [options] [files]"));
        assert!(help.contains("Options\n\n"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_stdin(&[PROGRAM_NAME, "-V"], "not pcl");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn full_extract_prints_proof_dependencies_from_stdin() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], SAMPLE_PROTOCOL);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(
            output,
            concat!(
                "      1 :  : [++p] : initial\n",
                "      2 : lemma : [++q] : 1 : 'lemma'\n",
                "      3 :  : [] : 2 : 'final'\n",
            )
        );
    }

    #[test]
    fn competition_framing_reports_cnf_refutation_for_empty_clause_extract() {
        let _guard = global_state_lock();
        let (status, output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--competition-framing"], SAMPLE_PROTOCOL);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.starts_with("% SZS output start CNFRefutation.\n"));
        assert!(output.ends_with("% SZS output end CNFRefutation\n"));
    }

    #[test]
    fn no_extract_marks_and_prints_all_steps_with_derivation_frame() {
        let _guard = global_state_lock();
        let (status, output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--no-extract", "-c"], SAMPLE_PROTOCOL);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.starts_with("% SZS output start Derivation.\n"));
        assert!(output.contains("      4 :  : [++r] : initial\n"));
        assert!(output.ends_with("% SZS output end Derivation.\n"));
    }

    #[test]
    fn no_extract_prints_formula_and_shell_steps() {
        let _guard = global_state_lock();
        let input = "\
1 : : p(a) : initial
2 : : : 1 : final
";
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME, "--no-extract"], input);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("      1 :  : p(a) : initial\n"));
        assert!(output.contains("      2 :  :  : 1 : final\n"));
    }

    #[test]
    fn fast_extract_uses_contiguous_extract_suffix() {
        let _guard = global_state_lock();
        let input = "\
1 : : [++p] : initial : 'final'
2 : : [++q] : initial
3 : : [++r] : initial : 'final'
";
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME, "--fast-extract"], input);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(output, "     3 :  : [++r] : initial : 'final'\n");
    }

    #[test]
    fn forward_comments_are_written_before_extracted_steps() {
        let _guard = global_state_lock();
        let input = format!("% lead\n{SAMPLE_PROTOCOL}% tail\n");
        let (status, output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--forward-comments"], &input);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.starts_with("% lead\n% tail\n"));
        assert!(output.contains("      3 :  : [] : 2 : 'final'\n"));
    }

    #[test]
    fn forward_comments_preserve_multi_file_input_order() {
        let _guard = global_state_lock();
        let first_path = temp_path("comments-first");
        let second_path = temp_path("comments-second");
        remove_if_present(&first_path);
        remove_if_present(&second_path);
        std::fs::write(&first_path, "% first\n1 : : [++p] : initial\n")
            .expect("first input fixture is written");
        std::fs::write(&second_path, "% second\n2 : : [] : 1 : 'final'\n")
            .expect("second input fixture is written");
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [
                PROGRAM_NAME,
                "--forward-comments",
                first_path.to_str().expect("path is utf8"),
                second_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("multi-file comment run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("stdout is utf8");
        assert!(output.starts_with("% first\n% second\n"));
        assert!(output.contains("      1 :  : [++p] : initial\n"));
        assert!(output.contains("      2 :  : [] : 1 : 'final'\n"));

        remove_if_present(&first_path);
        remove_if_present(&second_path);
    }

    #[test]
    fn tstp_alias_prints_tstp_protocol() {
        let _guard = global_state_lock();
        let (status, output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--tptp3-out"], SAMPLE_PROTOCOL);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("cnf(1,axiom,"));
        assert!(output.contains("cnf(3,plain,"));
        assert!(output.contains("['final']"));
    }

    #[test]
    fn output_file_receives_extract() {
        let _guard = global_state_lock();
        let input_path = temp_path("input");
        let output_path = temp_path("output");
        remove_if_present(&input_path);
        remove_if_present(&output_path);
        std::fs::write(&input_path, SAMPLE_PROTOCOL).expect("input fixture is written");

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
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
        assert!(output.contains("      3 :  : [] : 2 : 'final'\n"));

        remove_if_present(&input_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn output_dash_routes_to_stdout_like_c() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(SAMPLE_PROTOCOL.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "-o", "-"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("dash output run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("stdout is utf8");
        assert!(output.contains("      3 :  : [] : 2 : 'final'\n"));
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
    fn output_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let output_path = temp_path("output-dir");
        remove_if_present(&output_path);
        _ = std::fs::remove_dir(&output_path);
        std::fs::create_dir(&output_path).expect("output fixture directory is created");
        let mut stdin = Cursor::new(Vec::new());
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

        std::fs::remove_dir(&output_path).expect("output fixture directory is removed");
    }

    #[test]
    fn output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(SAMPLE_PROTOCOL.as_bytes().to_vec());
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
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
    fn help_text_preserves_c_usage_summary() {
        let rendered = print_help();

        assert!(rendered.contains(
            "Read an PCL2 protocol and print the steps necessary for proving the clauses"
        ));
        assert!(rendered.contains("Copyright 1998-2026 by Stephan Schulz"));
    }
}
