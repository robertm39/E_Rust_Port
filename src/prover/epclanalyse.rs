use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::set_verbose_level;
use crate::clauses::clause::ClauseParseOptions;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner, TokenType};
use crate::pcl2::propanalysis::{protocol_prop_analyse, protocol_prop_data_print_string};
use crate::pcl2::protocol::PclProtocol;
use crate::pcl2::steps::PclStepParseOptions;
use crate::prover::version::{STS_MAIL, VERSION};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "epclanalyse";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
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
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AnalyseConfig {
    output_file: Option<PathBuf>,
    files: Vec<String>,
}

enum RunCommand {
    Execute(AnalyseConfig),
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
    let _ = set_output_level(1);
    let result = run_inner(argv, stdin, stdout);
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
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_analyse(&config, stdin, stdout),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = AnalyseConfig::default();

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
                let _ = set_output_level(0);
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn execute_analyse(
    config: &AnalyseConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output = AnalyseOutput::open(config.output_file.as_deref(), stdout)?;
    let mut protocol = PclProtocol::new()?;

    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin)?;
        protocol.parse(&mut scanner, parse_options())?;
        scanner.check_tok(TokenType::NO_TOKEN)?;
    }

    let data = protocol_prop_analyse(&mut protocol);
    write_all(
        &mut output,
        protocol_prop_data_print_string(&mut protocol, &data, ProblemType::FirstOrder)?.as_bytes(),
    )?;
    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
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
        Scanner::from_file(Path::new(name), true).map_err(epclanalyse_scanner_open_diagnostic)?
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
Read an PCL2 protocol and print a number of statistics about the\n\
protocol and its clauses.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str(&legacy_footer());
    result
}

fn legacy_footer() -> String {
    format!(
        "\n\
Copyright (C) 2002-2009 by Stephan Schulz, {STS_MAIL}\n\
\n\
This program is a part of the support structure for the E equational\n\
theorem prover. You can find the latest version of the E distribution\n\
as well as additional information at\n\
http://wwwjessen.informatik.tu-muenchen.de/~schulz/WORK/eprover.html.\n\
\n\
This program is free software; you can redistribute it and/or modify\n\
it under the terms of the GNU General Public License as published by\n\
the Free Software Foundation; either version 2 of the License, or\n\
(at your option) any later version.\n\
\n\
This program is distributed in the hope that it will be useful,\n\
but WITHOUT ANY WARRANTY; without even the implied warranty of\n\
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the\n\
GNU General Public License for more details.\n\
\n\
You should have received a copy of the GNU General Public License\n\
along with this program (it should be contained in the top level\n\
directory of the distribution in the file COPYING); if not, write to\n\
the Free Software Foundation, Inc., 59 Temple Place, Suite 330,\n\
Boston, MA  02111-1307 USA\n\
\n\
The original copyright holder can be contacted as\n\
\n\
Stephan Schulz\n\
DHBW Stuttgart\n\
Fakultaet Technik\n\
Informatik\n\
Lerchenstrasse 1\n\
70174 Stuttgart\n\
Germany\n\
\n"
    )
}

const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

enum AnalyseOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> AnalyseOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        File::create(path).map(Self::File).map_err(|error| {
            epclanalyse_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
    }
}

impl<W: Write> Write for AnalyseOutput<'_, W> {
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

fn epclanalyse_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn epclanalyse_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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
    use super::{parse_options, print_help, run, OUTPUT_CLOSE_ERROR, PROGRAM_NAME};
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::inout::output::output_level;
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
1 : : [++p(a)] : initial
2 : : [++q(a),--r(X)] : 1 : 'derived'
3 : : [] : 2
";

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("epclanalyse-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("epclanalyse run succeeds");
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
        assert!(help.contains("Usage: epclanalyse [options] [files]"));
        assert!(help.contains("Copyright (C) 2002-2009 by Stephan Schulz"));
        assert!(help.contains("DHBW Stuttgart"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_stdin(&[PROGRAM_NAME, "--version"], "not pcl");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
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
    fn short_v_is_not_a_version_option() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME, "-V"], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("C epclanalyse has no -V shorthand");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("Unknown Option: -V"));
        assert!(stdout.is_empty());
    }

    #[test]
    fn verbose_option_sets_global_level() {
        let _guard = global_state_lock();
        let (status, _output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--verbose=3"], SAMPLE_PROTOCOL);

        assert_eq!(status, 0);
        assert_eq!(verbose_level(), 3);
        assert!(stderr.is_empty());
    }

    #[test]
    fn silent_option_sets_global_output_level_like_c() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME, "--silent"], SAMPLE_PROTOCOL);

        assert_eq!(status, 0);
        assert_eq!(output_level(), 0);
        assert!(output.contains("% Protocol properties\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_run_prints_protocol_statistics() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], SAMPLE_PROTOCOL);

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("% Protocol properties\n% ===================\n"));
        assert!(output.contains("% Number of clauses                  :      2\n"));
        assert!(output.contains("% Average number of literals         : 1.5000\n"));
        assert!(output.contains("% Longest Clause (if any): \n"));
        assert!(output.contains("      2 :  : [++q(a),--r(X1)] : 1 : 'derived'"));
        assert!(output.contains("% Heaviest Clause (if any): \n"));
        assert!(output.contains("% Standardweight:"));
    }

    #[test]
    fn empty_input_preserves_c_zero_denominator_summary_shape() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], "");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("% Number of clauses                  :      0\n"));
        assert!(output.contains("% Average number of literals         :    NaN\n"));
        assert!(output.contains("% Longest Clause (if any): \n"));
        assert!(output.contains("% Heaviest Clause (if any): \n"));
        assert!(!output.contains("% Standardweight:"));
    }

    #[test]
    fn formula_only_protocol_prints_formula_representatives_without_clause_metrics() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], "1 : : p(a) : initial\n");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        assert!(output.contains("% Number of clauses                  :      0\n"));
        assert_eq!(output.matches("      1 :  : ").count(), 4);
        assert!(output.contains("p(a)"));
        assert!(!output.contains("% Standardweight:"));
    }

    #[test]
    fn output_file_receives_protocol_statistics() {
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
        assert!(output.contains("% Protocol properties\n"));
        assert!(output.contains("% Number of clauses                  :      2\n"));

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
        assert!(output.contains("% Protocol properties\n"));
        assert!(output.contains("% Number of clauses                  :      2\n"));
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
            "Read an PCL2 protocol and print a number of statistics about the\nprotocol and its clauses."
        ));
        assert!(rendered.contains("http://wwwjessen.informatik.tu-muenchen.de"));
    }
}
