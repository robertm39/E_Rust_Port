use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type};
use crate::basics::verbose::set_verbose_level;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::{Scanner, TokenType};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::termfunc::{term_depth, term_weight_compute};
use crate::terms::termtypes::Term;
use crate::terms::typebanks::TypeBank;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "termprops";
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Verbose,
    Output,
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
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct TermpropsConfig {
    output_file: Option<PathBuf>,
    files: Vec<String>,
}

enum RunCommand {
    Execute(TermpropsConfig),
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct TermStats {
    count: i64,
    size_sum: i64,
    depth_sum: i64,
    size_max: i64,
    depth_max: i64,
}

impl TermStats {
    fn record(&mut self, size: i64, depth: i64) {
        self.count += 1;
        self.size_sum += size;
        self.depth_sum += depth;
        self.size_max = self.size_max.max(size);
        self.depth_max = self.depth_max.max(depth);
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
    set_problem_type(crate::basics::simple_stuff::ProblemType::FirstOrder)?;
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
        RunCommand::Execute(config) => execute_termprops(&config, stdin, stdout),
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
    let mut config = TermpropsConfig::default();

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
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn execute_termprops(
    config: &TermpropsConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = TermpropsOutput::open(config.output_file.as_deref(), stdout)?;
    let mut bank = TermBank::new(Signature::new(TypeBank::new()))?;
    let mut stats = TermStats::default();

    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        while !scanner.test_tok(TokenType::NO_TOKEN) {
            let term = bank.parse_term_with_distinct_checks(&mut scanner)?;
            let size = term_weight_compute(&term, 1, 1);
            let depth = term_depth(&term);
            let symmetry = termprops_symmetry(&term);
            let commutativity = termprops_commutativity(&term);
            writeln_diag(
                &mut output,
                &format!(
                    "{}  : {size} : {depth} : {} : {}",
                    bank.term_string(&term, true),
                    if symmetry { 's' } else { 'n' },
                    if commutativity { 's' } else { 'n' }
                ),
            )?;
            stats.record(size, depth);
        }
    }

    writeln_diag(&mut output, &termprops_summary(stats))?;
    output
        .flush()
        .map_err(|_error| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
}

fn termprops_symmetry(term: &Term) -> bool {
    term.arity() == 2 && term.argument(0) == term.argument(1)
}

fn termprops_commutativity(term: &Term) -> bool {
    if term.arity() != 2 {
        return false;
    }
    let Some(first) = term.argument(0) else {
        return false;
    };
    if first.arity() != 1 {
        return false;
    }
    let Some(nested) = first.argument(1) else {
        return false;
    };
    term.argument(1).is_some_and(|second| second == nested)
}

fn termprops_summary(stats: TermStats) -> String {
    format!(
        "{DEFAULT_COMCHAR_RAW} Terms: {}  ASize: {} MSize: {}, ADepth: {} MDepth: {}",
        stats.count,
        c_float(stats.size_sum, stats.count),
        stats.size_max,
        c_float(stats.depth_sum, stats.count),
        stats.depth_max
    )
}

#[allow(clippy::cast_precision_loss)]
fn c_float(sum: i64, count: i64) -> String {
    if count == 0 {
        return "nan".to_owned();
    }
    format!("{:.6}", sum as f64 / count as f64)
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        return Scanner::from_file_content("-", data, true);
    }
    Scanner::from_file(Path::new(name), true).map_err(termprops_scanner_open_diagnostic)
}

#[must_use]
pub fn print_help() -> String {
    let mut result = "\n\
\n\
cl_test\n\
\n\
Usage: termprops [options] [files]\n\
\n\
Read a set of terms and print it with size and depth information.\n\
\n"
    .to_owned();
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result
}

enum TermpropsOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> TermpropsOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        File::create(path).map(Self::File).map_err(|error| {
            termprops_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
    }
}

impl<W: Write> Write for TermpropsOutput<'_, W> {
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

fn termprops_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn termprops_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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
            .join(format!("termprops-{name}-{}.tmp", std::process::id()))
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
        assert!(help.contains("cl_test"));
        assert!(help.contains("Usage: termprops [options] [files]"));
        assert!(help.contains("Options\n\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_run_prints_term_properties_and_summary() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"a f(a,a) g(f(a),a)\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect("termprops stdin run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("output is utf8");
        assert!(output.contains("a  : 1 : 1 : n : n\n"));
        assert!(output.contains("f(a,a)  : 3 : 2 : s : n\n"));
        assert!(output.contains("g(f(a),a)  : 4 : 3 : n : n\n"));
        assert!(
            output.contains("% Terms: 3  ASize: 2.666667 MSize: 4, ADepth: 2.000000 MDepth: 3\n")
        );
    }

    #[test]
    fn rejects_distinct_number_argument_list_like_tbtermparse() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"1(a)\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("number argument list is rejected");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Number cannot have argument list"));
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn empty_input_preserves_c_nan_summary_shape() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status =
            run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr).expect("empty run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert_eq!(
            String::from_utf8(stdout).expect("output is utf8"),
            "% Terms: 0  ASize: nan MSize: 0, ADepth: nan MDepth: 0\n"
        );
    }

    #[test]
    fn output_file_receives_term_properties() {
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
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let output = std::fs::read_to_string(&output_path).expect("output file is readable");
        assert!(output.contains("f(a)  : 2 : 2 : n : n\n"));

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
        assert!(output.contains("f(a)  : 2 : 2 : n : n\n"));
        assert!(output.contains("% Terms: 1  ASize: 2.000000 MSize: 2"));
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

        assert!(rendered.starts_with("\n\ncl_test\n\n"));
        assert!(
            rendered.contains("Read a set of terms and print it with size and depth information.")
        );
    }
}
