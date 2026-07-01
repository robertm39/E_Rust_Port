use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::basics::verbose::set_verbose_level;
use crate::external::csscpa::{csscpa_loop, CsscpaState};
use crate::inout::commandline::{
    get_int_arg, get_int_arg_check_range, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::scanner::{IoFormat, Scanner};
use crate::prover::version::{footer, VERSION};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "CSSCPA_filter";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    Silent,
    OutputLevel,
    Rant,
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
        "Select an output level, greater values imply more verbose output. At the moment, level 0 only prints the result of each statement, and level 1 also prints what happens to each clause.",
    ),
    OptCell::new(
        OptionCode::Rant,
        Some('r'),
        Some("rant-about-input-buffering"),
        OptArgType::OptArg,
        Some("666"),
        "Tell the program how much you hate to include the 'Please'-sequence in the input. The optional argument is the rant-intensity.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct FilterConfig {
    output_file: Option<PathBuf>,
    output_level: bool,
    files: Vec<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            output_level: true,
            files: Vec::new(),
        }
    }
}

enum RunCommand {
    Execute(FilterConfig),
    Exit(u8),
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
    init_io(PROGRAM_NAME);
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
        RunCommand::Execute(config) => execute_filter(&config, stdin, stdout),
    }
}

fn process_options<I, S>(
    argv: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = FilterConfig::default();

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
                config.output_level = false;
            }
            OptionCode::OutputLevel => {
                let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if level > 1 {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        "Option -l (--output-level) accepts only 0 or 1for CSSCPA_filter",
                    ));
                }
                config.output_level = level != 0;
            }
            OptionCode::Rant => {
                let intensity = get_int_arg_check_range(
                    parsed.option(),
                    parsed.arg().unwrap_or(""),
                    i64::MIN,
                    i64::MAX,
                )?;
                if intensity != 0 {
                    write_all(stderr, b"Improve it yourself, mate. The code is free.\n")?;
                } else {
                    write_all(stderr, b"You call that a rant????\n")?;
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

fn execute_filter(
    config: &FilterConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = FilterOutput::open(config.output_file.as_deref(), stdout)?;
    let mut state = CsscpaState::new()?;
    let mut output_level = config.output_level;

    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        scanner.set_format(IoFormat::Tstp);
        let result = csscpa_loop(&mut scanner, &mut state, output_level)?;
        output_level = result.output_level();
        write_all(&mut output, result.trace().as_bytes())?;
    }

    write_all(&mut output, b"\n")?;
    writeln_diag(
        &mut output,
        &format!("{DEFAULT_COMCHAR_RAW} Resulting clause set:"),
    )?;
    write_clause_set(&mut output, state.terms(), state.pos_units())?;
    write_clause_set(&mut output, state.terms(), state.neg_units())?;
    write_clause_set(&mut output, state.terms(), state.non_units())?;
    output
        .flush()
        .map_err(|error| io_diagnostic(format!("Cannot flush output: {error}")))?;
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
    Scanner::from_file(Path::new(name), true)
}

fn write_clause_set(
    output: &mut impl Write,
    terms: &crate::terms::termbanks::TermBank,
    clauses: &crate::clauses::clausesets::ClauseSet,
) -> Result<(), Diagnostic> {
    let rendered = clauses.tstp_print_string(terms, true, ProblemType::FirstOrder)?;
    write_all(output, rendered.as_bytes())
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
Read a list of CSSCPA statements, print the resulting clause set on\n\
termination. A CSSCPA statement is either 'accept: <clause>' or\n\
'check: <clause>', where <clause> is a clause in TPTP format. Clauses\n\
prepended by 'accept' are always integrated into the current clause\n\
set unless they are subsumed or tautological. Clauses prepended by\n\
'check' are only integrated if they subsume clauses with a total\n\
weight that is higher than their own weight. Subsumed clauses are\n\
always removed from the clause set.\n\
\n\
After every statement, clause count, literal count and total clause\n\
weight are printed to the selected output channel (stdout by\n\
default). If you need these results immediately, you'll have to beg\n\
the progam by including the sequence\n\
\n\
Please process clauses now, I beg you, great shining CSSCPA,\n\
wonder of the world, most beautiful program ever written.\n\
\n\
to overcome CLIB's input buffering.\n\
\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

enum FilterOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> FilterOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        File::create(path)
            .map(Self::File)
            .map_err(|error| io_diagnostic(format!("Cannot open file {}: {error}", path.display())))
    }
}

impl<W: Write> Write for FilterOutput<'_, W> {
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

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::{print_help, run, PROGRAM_NAME};
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::test_support::global_state_lock;
    use std::io::Cursor;
    use std::path::{Path, PathBuf};

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("csscpa-filter-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[test]
    fn help_and_version_exit_before_filter_execution() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let help_status = run(
            [PROGRAM_NAME, "--help"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("help succeeds");
        assert_eq!(help_status, 0);
        let help = String::from_utf8(stdout).expect("help is utf8");
        assert!(help.contains("Usage: CSSCPA_filter [options] [files]"));
        assert!(help.contains("Please process clauses now"));
        assert!(stderr.is_empty());

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let version_status = run(
            [PROGRAM_NAME, "--version"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("version succeeds");
        assert_eq!(version_status, 0);
        assert!(String::from_utf8(stdout)
            .expect("version is utf8")
            .starts_with("CSSCPA_filter "));
    }

    #[test]
    fn default_stdin_run_processes_csscpa_commands_and_prints_result() {
        let _guard = global_state_lock();
        let mut stdin =
            Cursor::new(b"accept from 2: cnf(csscpa_unit,axiom,p(a)).\nstate:\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect("filter stdin run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("output is utf8");
        assert!(output.contains("% Clause "));
        assert!(output.contains("accepted from 2 (forced)"));
        assert!(output.contains("% CSSCPAState: requested"));
        assert!(output.contains("\n% Resulting clause set:\n"));
        assert!(output.contains("cnf("));
        assert!(output.contains("p(a)"));
    }

    #[test]
    fn silent_output_file_redirects_filter_output() {
        let _guard = global_state_lock();
        let input_path = temp_path("input");
        let output_path = temp_path("output");
        remove_if_present(&input_path);
        remove_if_present(&output_path);
        std::fs::write(&input_path, b"accept: cnf(csscpa_unit,axiom,p(a)).\n")
            .expect("input fixture is written");

        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(
            [
                PROGRAM_NAME,
                "--silent",
                "-o",
                output_path.to_str().expect("path is utf8"),
                input_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("filter file run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let output = std::fs::read_to_string(&output_path).expect("output file is readable");
        assert!(!output.contains("accepted from"));
        assert!(output.contains("% Resulting clause set:"));
        assert!(output.contains("p(a)"));

        remove_if_present(&input_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn rant_and_verbose_options_follow_c_shapes() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"accept: cnf(csscpa_unit,axiom,p(a)).\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--rant-about-input-buffering=0", "-v"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("filter rant run succeeds");

        assert_eq!(status, 0);
        assert_eq!(verbose_level(), 1);
        assert_eq!(
            String::from_utf8(stderr).expect("stderr is utf8"),
            "You call that a rant????\n"
        );
    }

    #[test]
    fn output_level_rejects_values_greater_than_one_like_c() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME, "--output-level=2"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect_err("output level 2 is rejected");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("accepts only 0 or 1"));
    }

    #[test]
    fn help_text_contains_c_footer_and_options() {
        let rendered = print_help();

        assert!(rendered.contains("Options\n\n"));
        assert!(rendered.contains("--rant-about-input-buffering[=<arg>]"));
        assert!(rendered.contains("This program is free software"));
    }
}
