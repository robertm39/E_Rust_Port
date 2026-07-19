use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::simple_stuff::ProblemType;
use crate::basics::verbose::set_verbose_level;
use crate::external::csscpa::{csscpa_loop, CsscpaLoopResult, CsscpaState};
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
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

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
        "Tell the program how much you hate to include the 'Please'-sequence in the input. The optional argument is the  rant-intensity.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct FilterConfig {
    output_file: Option<PathBuf>,
    output_level: i64,
    files: Vec<String>,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            output_level: 1,
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
                config.output_level = 0;
            }
            OptionCode::OutputLevel => {
                let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if level > 1 {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        "Option -l (--output-level) accepts only 0 or 1for CSSCPA_filter",
                    ));
                }
                config.output_level = level;
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
        write_loop_trace(&mut output, &result)?;
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
        .map_err(|_error| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
}

fn scanner_for_input(name: &str, stdin: &mut impl Read) -> Result<Scanner, Diagnostic> {
    if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        return Scanner::from_file_content("<stdin>", data, true);
    }
    Scanner::from_file(Path::new(name), true).map_err(csscpa_scanner_open_diagnostic)
}

fn write_clause_set(
    output: &mut impl Write,
    terms: &crate::terms::termbanks::TermBank,
    clauses: &crate::clauses::clausesets::ClauseSet,
) -> Result<(), Diagnostic> {
    let rendered = clauses.tstp_print_string(terms, true, ProblemType::FirstOrder)?;
    write_all(output, rendered.as_bytes())
}

fn write_loop_trace(output: &mut impl Write, result: &CsscpaLoopResult) -> Result<(), Diagnostic> {
    let trace = result.trace();
    let mut start = 0;
    for &end in result.trace_flush_offsets() {
        debug_assert!(start <= end && end <= trace.len());
        let segment = trace
            .get(start..end)
            .expect("CSSCPA trace flush offset must be a valid string boundary");
        write_all(output, segment.as_bytes())?;
        _ = output.flush();
        start = end;
    }
    let remainder = trace
        .get(start..)
        .expect("CSSCPA trace flush offset must be a valid string boundary");
    write_all(output, remainder.as_bytes())
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
        File::create(path).map(Self::File).map_err(|error| {
            csscpa_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
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

fn csscpa_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn csscpa_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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
        csscpa_scanner_open_diagnostic, csscpa_sys_error_diagnostic, print_help, run,
        OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::{Diagnostic, ErrorCode};
    use crate::basics::verbose::verbose_level;
    use crate::prover::version::{footer, VERSION};
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

    #[derive(Default)]
    struct FlushCountingWriter {
        bytes: Vec<u8>,
        flushes: usize,
    }

    impl Write for FlushCountingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            self.bytes.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("csscpa-filter-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn expected_help() -> String {
        let mut expected = format!(
            concat!(
                "\n",
                "\n",
                "{program_name} {version}\n",
                "\n",
                "Usage: {program_name} [options] [files]\n",
                "\n",
                "Read a list of CSSCPA statements, print the resulting clause set on\n",
                "termination. A CSSCPA statement is either 'accept: <clause>' or\n",
                "'check: <clause>', where <clause> is a clause in TPTP format. Clauses\n",
                "prepended by 'accept' are always integrated into the current clause\n",
                "set unless they are subsumed or tautological. Clauses prepended by\n",
                "'check' are only integrated if they subsume clauses with a total\n",
                "weight that is higher than their own weight. Subsumed clauses are\n",
                "always removed from the clause set.\n",
                "\n",
                "After every statement, clause count, literal count and total clause\n",
                "weight are printed to the selected output channel (stdout by\n",
                "default). If you need these results immediately, you'll have to beg\n",
                "the progam by including the sequence\n",
                "\n",
                "Please process clauses now, I beg you, great shining CSSCPA,\n",
                "wonder of the world, most beautiful program ever written.\n",
                "\n",
                "to overcome CLIB's input buffering.\n",
                "\n",
                "\n",
                "Options\n",
                "\n",
                "   -h\n",
                "  --help\n",
                "    Print a short description of program usage and options.\n",
                "\n",
                "  --version\n",
                "    Print the version number of the program.\n",
                "\n",
                "   -v\n",
                "  --verbose[=<arg>]\n",
                "    Verbose comments on the progress of the program. The short form or the\n",
                "    long form without the optional argument is equivalent to --verbose=1.\n",
                "\n",
                "   -o <arg>\n",
                "  --output-file=<arg>\n",
                "    Redirect output into the named file.\n",
                "\n",
                "   -s\n",
                "  --silent\n",
                "    Equivalent to --output-level=0.\n",
                "\n",
                "   -l <arg>\n",
                "  --output-level=<arg>\n",
                "    Select an output level, greater values imply more verbose output. At the\n",
                "    moment, level 0 only prints the result of each statement, and level 1\n",
                "    also prints what happens to each clause.\n",
                "\n",
                "   -r\n",
                "  --rant-about-input-buffering[=<arg>]\n",
                "    Tell the program how much you hate to include the 'Please'-sequence in\n",
                "    the input. The optional argument is the  rant-intensity. The short form\n",
                "    or the long form without the optional argument is equivalent to\n",
                "    --rant-about-input-buffering=666.\n",
                "\n",
                "\n",
                "\n",
            ),
            program_name = PROGRAM_NAME,
            version = VERSION,
        );
        expected.push_str(&footer());
        expected
    }

    fn large_stateful_corpus() -> String {
        let mut lines = vec!["output_level 0".to_owned(), "state:".to_owned()];
        for index in 0..24 {
            let source = 2 + index % 14;
            lines.push(format!(
                "accept from {source}: cnf(csscpa_seed_{index},axiom,csscpa_seed_{index}(a))."
            ));
        }
        for index in 0..8 {
            lines.push(format!(
                "accept: cnf(csscpa_negative_{index},axiom,~csscpa_negative_{index}(a))."
            ));
        }
        for index in 0..8 {
            lines.push(format!(
                "accept: cnf(csscpa_wide_{index},axiom,(csscpa_wide_{index}(a)|csscpa_side_{index}(a)))."
            ));
        }
        lines.extend(["output_level 1".to_owned(), "state:".to_owned()]);
        for index in 0..12 {
            lines.push(format!(
                "check improve(0.0,0.0): cnf(csscpa_subsumed_{index},axiom,(csscpa_seed_{index}(a)|csscpa_extra_{index}(a)))."
            ));
        }
        for index in 0..4 {
            lines.push(format!(
                "check: cnf(csscpa_tautology_{index},axiom,(csscpa_taut_{index}(a)|~csscpa_taut_{index}(a)))."
            ));
        }
        for index in 0..8 {
            lines.push(format!(
                "check improve(0.0,1.0): cnf(csscpa_improved_{index},axiom,csscpa_wide_{index}(a))."
            ));
        }
        for index in 0..4 {
            lines.push(format!(
                "check improve(1.0,1.0): cnf(csscpa_contradiction_{index},axiom,csscpa_negative_{index}(a))."
            ));
        }
        for index in 0..4 {
            lines.push(format!(
                "check improve(1.0,1.0): cnf(csscpa_weighty_{index},axiom,(csscpa_heavy_{index}(f(a))|csscpa_other_{index}(g(a))))."
            ));
        }
        lines.extend([
            "Please process clauses now, I beg you, great shining CSSCPA,".to_owned(),
            "wonder of the world, most beautiful program ever written.".to_owned(),
            "state:".to_owned(),
        ]);
        lines.join("\n") + "\n"
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
        assert_eq!(help, expected_help());
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
    fn large_stateful_corpus_covers_all_clause_outcomes_and_final_buckets() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(large_stateful_corpus().into_bytes());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect("large CSSCPA corpus succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("output is utf8");
        assert_eq!(output.matches("rejected (subsumed by").count(), 12);
        assert_eq!(output.matches("rejected (Tautology)").count(), 4);
        assert_eq!(output.matches("accepted from 0 (improved)").count(), 8);
        assert_eq!(output.matches("accepted from 0 (contradicts)").count(), 4);
        assert_eq!(output.matches("rejected (weighty)").count(), 4);
        assert!(output.contains("% CSSCPAState: requested  by 0, 44, 44,"));
        let result = output
            .split_once("% Resulting clause set:\n")
            .map(|(_, result)| result)
            .expect("result clause-set marker is present");
        assert_eq!(result.matches("cnf(").count(), 44);
        assert!(result.contains("csscpa_seed_23(a)"));
        assert!(result.contains("csscpa_wide_7(a)"));
        assert!(result.contains("csscpa_negative_3(a)"));
        assert!(!result.contains("csscpa_side_0(a)"));
    }

    #[test]
    fn filter_accepts_old_tptp_input_clause_under_tstp_mode() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"accept: input_clause(c_0_1,axiom,[++p(a)]).\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect("filter old-TPTP clause run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("output is utf8");
        assert!(output.contains("accepted from 0 (forced)"));
        assert!(output.contains("% Resulting clause set:"));
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
    fn negative_output_level_keeps_c_truthy_trace_but_not_outprint_line() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(
            b"accept: cnf(csscpa_neg,axiom,~p(a)).\n\
check improve(1.0,0.0): cnf(csscpa_pos,axiom,p(a)).\n"
                .to_vec(),
        );
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--output-level=-1"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("negative output level run succeeds");

        assert_eq!(status, 0);
        assert!(stderr.is_empty());
        let output = String::from_utf8(stdout).expect("output is utf8");
        assert!(output.contains("accepted from 0 (forced)"));
        assert!(output.contains("accepted from 0 (contradicts)"));
        assert!(output.contains("% CSSCPAState: contradicts"));
        assert!(!output.contains("% Unit contradiction found!"));
        assert!(output.contains("% Resulting clause set:"));
    }

    #[test]
    fn missing_input_file_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let missing_path = temp_path("missing-input");
        remove_if_present(&missing_path);
        _ = std::fs::remove_dir(&missing_path);
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
    fn syserror_diagnostic_preserves_host_error_suffix_exactly() {
        let source = io::Error::other("host-specific suffix (os error 1234)");
        let diagnostic = csscpa_sys_error_diagnostic("Cannot open file output.csscpa", &source);

        assert_eq!(diagnostic.code(), ErrorCode::FILE_ERROR);
        assert_eq!(
            diagnostic.message(),
            "Cannot open file output.csscpa\n\
CSSCPA_filter: host-specific suffix (os error 1234)"
        );
    }

    #[test]
    fn scanner_open_diagnostic_preserves_path_and_host_suffix_exactly() {
        let source = Diagnostic::new(
            ErrorCode::FILE_ERROR,
            "Cannot open file C:\\csscpa cases\\missing.csscpa for reading: host suffix",
        );

        let diagnostic = csscpa_scanner_open_diagnostic(source);

        assert_eq!(diagnostic.code(), ErrorCode::FILE_ERROR);
        assert_eq!(
            diagnostic.message(),
            "Cannot open file C:\\csscpa cases\\missing.csscpa for reading\n\
CSSCPA_filter: host suffix"
        );
    }

    #[test]
    fn output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"accept: cnf(csscpa_unit,axiom,p(a)).\n".to_vec());
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    #[test]
    fn filter_flushes_after_csscpa_state_and_clause_events() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(
            b"state:
output_level 0
accept: cnf(csscpa_unit,axiom,p(a)).
Please process clauses now, I beg you, great shining CSSCPA,
wonder of the world, most beautiful program ever written.
"
            .to_vec(),
        );
        let mut stdout = FlushCountingWriter::default();
        let mut stderr = Vec::new();

        let status = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect("CSSCPA filter succeeds");

        assert_eq!(status, 0);
        assert_eq!(stdout.flushes, 3);
        let output = String::from_utf8(stdout.bytes).expect("CSSCPA output is utf8");
        assert!(output.starts_with("% CSSCPAState: requested  by 0, 0, 0, 0"));
        assert!(output.contains("% Resulting clause set:\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn help_text_contains_c_footer_and_options() {
        let rendered = print_help();

        assert_eq!(rendered, expected_help());
    }
}
