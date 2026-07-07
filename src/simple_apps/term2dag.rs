use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::set_verbose_level;
use crate::inout::commandline::{
    get_bool_arg, get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termtypes::TP_TOP_POS;
use crate::terms::typebanks::TypeBank;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "term2dag";
const VERSION: &str = "0.1 - Sat Nov 29 16:39:20 MET 1997";
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";
const HELP_SPACE_46: &str = "                                              ";
const HELP_SPACE_28: &str = "                            ";
const HELP_SPACE_13: &str = "             ";
const HELP_SPACE_54: &str = "                                                      ";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Verbose,
    Output,
    PrintRefs,
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
        OptionCode::PrintRefs,
        Some('r'),
        Some("print-reference-number"),
        OptArgType::OptArg,
        Some("true"),
        "Print number of references for each DAG node as a comment.",
    ),
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Term2dagConfig {
    output_file: Option<PathBuf>,
    files: Vec<String>,
}

enum RunCommand {
    Execute(Term2dagConfig),
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
    match process_options(argv, stdout, stderr)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_term2dag(&config, stdin, stdout),
    }
}

fn process_options<I, S>(
    argv: I,
    stdout: &mut impl Write,
    _stderr: &mut impl Write,
) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = Term2dagConfig::default();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        match parsed.option().option_code {
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Verbose => {
                let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                set_verbose_level(i64_to_i32_saturating(level));
            }
            OptionCode::Output => {
                config.output_file = parsed.arg().map(PathBuf::from);
            }
            OptionCode::PrintRefs => {
                let _ = get_bool_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn execute_term2dag(
    config: &Term2dagConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = Term2dagOutput::open(config.output_file.as_deref())?;
    let mut bank = TermBank::new(Signature::new(TypeBank::new()))?;

    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        while !scanner.test_tok(TokenType::NO_TOKEN) {
            let term = bank.parse_term_with_distinct_checks(&mut scanner)?;
            term.set_prop(TP_TOP_POS);
        }
    }

    output
        .write_signature(bank.signature(), stdout)
        .map_err(|error| io_diagnostic(format!("Cannot write output: {error}")))?;
    let dag = bank.bank_in_order_string_with_internal_info(true);
    output
        .write_all(stdout, dag.as_bytes())
        .map_err(|error| io_diagnostic(format!("Cannot write output: {error}")))?;
    output
        .flush(stdout)
        .map_err(|_error| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        return Scanner::from_file_content("-", data, true);
    }
    Scanner::from_file(Path::new(name), true).map_err(term2dag_scanner_open_diagnostic)
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\n{HELP_SPACE_46}term2dag {VERSION}\n\
         {HELP_SPACE_28}\n\
         {HELP_SPACE_46}Usage: term2dag [options] [files]\n\
         {HELP_SPACE_13}\n\
         {HELP_SPACE_54}Read a set of terms and print a DAG representing it.\n\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result
}

enum Term2dagOutput {
    Stdout,
    File(File),
}

impl Term2dagOutput {
    fn open(path: Option<&Path>) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout);
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout);
        }
        File::create(path).map(Self::File).map_err(|error| {
            term2dag_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
    }

    fn write_signature(
        &mut self,
        signature: &Signature,
        stdout: &mut impl Write,
    ) -> io::Result<()> {
        match self {
            Self::Stdout => signature.print(stdout),
            Self::File(file) => signature.print_with_c_stdout_side_channel(file, stdout),
        }
    }

    fn write_all(&mut self, stdout: &mut impl Write, buffer: &[u8]) -> io::Result<()> {
        match self {
            Self::Stdout => stdout.write_all(buffer),
            Self::File(file) => file.write_all(buffer),
        }
    }

    fn flush(&mut self, stdout: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Stdout => stdout.flush(),
            Self::File(file) => file.flush(),
        }
    }
}

fn write_all(output: &mut impl Write, bytes: &[u8]) -> Result<(), Diagnostic> {
    output
        .write_all(bytes)
        .map_err(|error| io_diagnostic(format!("Cannot write output: {error}")))
}

fn io_diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::FILE_ERROR, message)
}

fn term2dag_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn term2dag_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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
    use crate::test_support::global_state_lock;
    use std::io::{self, Cursor, Write};
    use std::path::{Path, PathBuf};

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("term2dag-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[test]
    fn help_exits_before_term_processing() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--help"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("help succeeds");

        assert_eq!(status, 0);
        let help = String::from_utf8(stdout).expect("help is utf8");
        assert!(help.starts_with(
            "\n\n                                              term2dag 0.1 - Sat Nov 29 16:39:20 MET 1997\n"
        ));
        assert!(help.contains(
            "\n                                              Usage: term2dag [options] [files]\n"
        ));
        assert!(help.contains(
            "\n                                                      Read a set of terms and print a DAG representing it.\n"
        ));
        assert!(help.contains("--print-reference-number[=<arg>]"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_run_prints_signature_and_entry_ordered_dag() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"f(a,a) g(f(a,a))\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect("term2dag stdin run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("output is utf8");
        assert!(output.starts_with("% Signature ("));
        assert!(output.contains("   f             :  2"));
        assert!(output.contains("*1 : $true   =   $true\t/*  Properties:"));
        assert!(output.contains("*3 : a   =   a\t/*  Properties:"));
        assert!(output.contains("*4 : f(*3,*3)   =   f(a,a)\t/*  Properties:"));
        assert!(output.contains("*5 : g(*4)   =   g(f(a,a))\t/*  Properties:"));
    }

    #[test]
    fn distinct_number_with_arguments_is_rejected_like_tb_term_parse() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"1(a)\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error =
            run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr).expect_err("bad term fails");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Number cannot have argument list"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn print_reference_number_false_is_overwritten_like_c_main() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"a\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--print-reference-number=false"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("term2dag run succeeds");

        assert_eq!(status, 0);
        let output = String::from_utf8(stdout).expect("output is utf8");
        assert!(output.contains("\t/*  Properties:"));
    }

    #[test]
    fn invalid_print_reference_number_is_still_rejected() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"a\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--print-reference-number=maybe"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("bad bool is rejected during option parsing");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("expects 'true' or 'false'"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn output_file_receives_dag_output() {
        let _guard = global_state_lock();
        let input_path = temp_path("input");
        let output_path = temp_path("output");
        remove_if_present(&input_path);
        remove_if_present(&output_path);
        std::fs::write(&input_path, b"f(a)\n").expect("input fixture is written");

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
        assert!(stderr.is_empty());
        let stdout = String::from_utf8(stdout).expect("stdout is utf8");
        assert_eq!(stdout, "\n\n\n\n");
        let output = std::fs::read_to_string(&output_path).expect("output file is readable");
        assert!(!output.contains("(no type)"));
        assert!(output.contains("*4 : f(*3)   =   f(a)\t/*  Properties:"));

        remove_if_present(&input_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn output_dash_routes_to_stdout_like_c() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"f(a)\n".to_vec());
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
        assert!(output.contains("*4 : f(*3)   =   f(a)\t/*  Properties:"));
    }

    #[test]
    fn verbose_option_sets_global_verbose_level() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"a\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--verbose=3"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("verbose run succeeds");

        assert_eq!(status, 0);
        assert_eq!(verbose_level(), 3);
    }

    #[test]
    fn missing_input_file_reports_file_error() {
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
        let mut stdin = Cursor::new(b"a\n".to_vec());
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    #[test]
    fn help_text_preserves_c_banner() {
        let rendered = print_help();

        assert!(rendered.starts_with(
            "\n\n                                              term2dag 0.1 - Sat Nov 29 16:39:20 MET 1997\n"
        ));
        assert!(rendered.contains(
            "\n                                                      Read a set of terms and print a DAG representing it.\n"
        ));
    }
}
