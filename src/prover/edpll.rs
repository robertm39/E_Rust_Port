use crate::basics::defines::MEGA;
use crate::basics::error::{c_io_error_message, Diagnostic, ErrorCode};
use crate::basics::os_wrapper::{get_system_phys_memory, set_memory_limit, RLimResult};
use crate::basics::simple_stuff::{reset_problem_type, set_problem_type, ProblemType};
use crate::basics::verbose::{set_verbose_level, verbose_level};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{token_pos_rep, IoFormat, Scanner};
use crate::inout::signals::{configure_time_limits, RLIM_INFINITY_COMPAT};
use crate::propositional::dpll::DpllState;
use crate::propositional::dpllformula::DpllFormula;
use crate::prover::version::{footer, VERSION};
use crate::terms::signature::Signature;
use crate::terms::termbanks::TermBank;
use crate::terms::typebanks::TypeBank;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "umlaut-dpll";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    Silent,
    OutputLevel,
    TptpParse,
    DimacsPrint,
    MemoryLimit,
    CpuLimit,
    SoftCpuLimit,
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
        "Verbose comments on the progress of the program by printing technical information to stderr.",
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
        "Select an output level, greater values imply more verbose output. Level 0 produces nearly no output, level 1 produces minimal additional output.Higher levels are without meaning in umlaut-dpll (I think).",
    ),
    OptCell::new(
        OptionCode::TptpParse,
        None,
        Some("tptp-in"),
        OptArgType::NoArg,
        None,
        "Parse TPTP format instead of lop (does not understand includes, as TPTP include syntax is considered harmful).",
    ),
    OptCell::new(
        OptionCode::DimacsPrint,
        Some('d'),
        Some("dimacs"),
        OptArgType::NoArg,
        None,
        "Print output in the DIMACS format suitable for many propositional provers.",
    ),
    OptCell::new(
        OptionCode::MemoryLimit,
        Some('m'),
        Some("memory-limit"),
        OptArgType::ReqArg,
        None,
        "Limit the memory the system may use. The argument is the allowed amount of memory in MB. This option may not work everywhere, due to broken and/or strange behaviour of setrlimit() in some UNIX implementations. It does work under all tested versions of Solaris and GNU/Linux.",
    ),
    OptCell::new(
        OptionCode::CpuLimit,
        None,
        Some("cpu-limit"),
        OptArgType::OptArg,
        Some("300"),
        "Limit the cpu time the program should run. The optional argument is the CPU time in seconds. The program will terminate immediately after reaching the time limit, regardless of internal state. This option may not work everywhere, due to broken and/or strange behaviour of setrlimit() in some UNIX implementations. It does work under all tested versions of Solaris, HP-UX and GNU/Linux. As a side effect, this option will inhibit core file writing.",
    ),
    OptCell::new(
        OptionCode::SoftCpuLimit,
        None,
        Some("soft-cpu-limit"),
        OptArgType::OptArg,
        Some("310"),
        "Limit the cpu time spend in grounding. After the time expires, the prover will print an partial system.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct EdpllConfig {
    output_file: Option<PathBuf>,
    parse_format: IoFormat,
    memory_limit: u64,
    hard_cpu_limit: Option<i64>,
    soft_cpu_limit: Option<i64>,
    files: Vec<String>,
}

impl Default for EdpllConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            parse_format: IoFormat::Lop,
            memory_limit: 0,
            hard_cpu_limit: None,
            soft_cpu_limit: None,
            files: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(EdpllConfig),
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
    configure_time_limits(RLIM_INFINITY_COMPAT, RLIM_INFINITY_COMPAT, 0);
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
    match process_options(argv, stdout, stderr)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_edpll(&config, stdin, stdout, stderr),
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
    let mut config = EdpllConfig::default();

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
            OptionCode::OutputLevel => {
                let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                let _ = set_output_level(level);
            }
            OptionCode::TptpParse => {
                config.parse_format = IoFormat::Tptp;
            }
            OptionCode::DimacsPrint => {}
            OptionCode::MemoryLimit => {
                config.memory_limit = parse_memory_limit(parsed.option(), parsed.arg(), stderr)?;
            }
            OptionCode::CpuLimit => {
                let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if let Some(soft_limit) = config.soft_cpu_limit {
                    check_hard_soft_limits(limit, soft_limit, true)?;
                }
                config.hard_cpu_limit = Some(limit);
            }
            OptionCode::SoftCpuLimit => {
                let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
                if let Some(hard_limit) = config.hard_cpu_limit {
                    check_hard_soft_limits(hard_limit, limit, false)?;
                }
                config.soft_cpu_limit = Some(limit);
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn execute_edpll(
    config: &EdpllConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    apply_resource_config(config, stderr)?;
    let mut output = EdpllOutput::open(config.output_file.as_deref(), stdout)?;
    let mut bank = TermBank::new(Signature::new(TypeBank::new()))?;
    let mut formula = DpllFormula::new();

    for file in &config.files {
        let mut scanner = scanner_for_input(file, stdin, config.parse_format)?;
        let parse_result = formula.parse_lop_with_trace(
            &mut scanner,
            &mut bank,
            ProblemType::FirstOrder,
            |trace| write_all(&mut output, trace.as_bytes()),
        );
        if let Err(error) = parse_result {
            return Err(edpll_parse_diagnostic(&scanner, error));
        }
    }

    let _dpll_state = DpllState::new(formula);
    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
}

fn scanner_for_input(
    name: &str,
    stdin: &mut impl Read,
    format: IoFormat,
) -> Result<Scanner, Diagnostic> {
    let mut scanner = if name == "-" {
        let mut data = Vec::new();
        stdin
            .read_to_end(&mut data)
            .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        Scanner::from_file_content("<stdin>", data, true)?
    } else {
        Scanner::from_file(Path::new(name), true).map_err(edpll_scanner_open_diagnostic)?
    };
    scanner.set_format(format);
    Ok(scanner)
}

fn parse_memory_limit<Code>(
    option: &OptCell<Code>,
    arg: Option<&str>,
    stderr: &mut impl Write,
) -> Result<u64, Diagnostic> {
    let arg = arg.unwrap_or("");
    if arg == "Auto" {
        let system_memory = get_system_phys_memory();
        if system_memory == -1 {
            return Err(Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                "Cannot find physical memory automatically. Give explicit value to --memory-limit",
            ));
        }
        let memory_limit = memory_limit_bytes_from_mb(auto_memory_mb(system_memory));
        if verbose_level() > 0 {
            writeln_diag(
                stderr,
                &format!("Physical memory determined as {system_memory} MB"),
            )?;
            writeln_diag(stderr, &format!("Memory limit set to {memory_limit} MB"))?;
        }
        return Ok(memory_limit);
    }
    get_int_arg(option, arg).map(memory_limit_bytes_from_mb)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn auto_memory_mb(system_memory_mb: i64) -> i64 {
    (system_memory_mb as f64 * 0.8) as i64
}

fn apply_resource_config(config: &EdpllConfig, stderr: &mut impl Write) -> Result<(), Diagnostic> {
    let hard_limit = config
        .hard_cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    let soft_limit = config
        .soft_cpu_limit
        .map_or(RLIM_INFINITY_COMPAT, c_rlimit_from_arg);
    configure_time_limits(hard_limit, soft_limit, 0);
    for warning in memory_limit_warnings(set_memory_limit(config.memory_limit)) {
        write_all(stderr, warning.render_warning(PROGRAM_NAME).as_bytes())?;
    }
    Ok(())
}

fn memory_limit_warnings(result: RLimResult) -> Vec<Diagnostic> {
    if result != RLimResult::Reduced {
        return Vec::new();
    }
    ["RLIMIT_DATA", "RLIMIT_AS"]
        .into_iter()
        .map(|description| {
            Diagnostic::new(
                ErrorCode::SYSTEM_ERROR,
                format!("Had to reduce limit {description}"),
            )
        })
        .collect()
}

#[allow(clippy::cast_sign_loss)]
const fn c_rlimit_from_arg(value: i64) -> u64 {
    value as u64
}

const fn memory_limit_bytes_from_mb(memory_mb: i64) -> u64 {
    c_rlimit_from_arg(memory_mb).wrapping_mul(MEGA)
}

fn check_hard_soft_limits(
    hard: i64,
    soft: i64,
    hard_option_changed: bool,
) -> Result<(), Diagnostic> {
    if c_rlimit_from_arg(hard) > c_rlimit_from_arg(soft) {
        return Ok(());
    }
    let message = if hard_option_changed {
        "Hard time limit has to be larger than softtime limit"
    } else {
        "Soft time limit has to be smaller than hardtime limit"
    };
    Err(Diagnostic::new(ErrorCode::USAGE_ERROR, message))
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
Read a set of ground clauses and try to refute (or satisfy) it.\n\
Not completed yet!\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options\n\n")));
    result.push_str(&legacy_footer());
    result
}

fn legacy_footer() -> String {
    footer()
}

const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

enum EdpllOutput<'a, W: Write> {
    Stdout(&'a mut W),
    File(File),
}

impl<'a, W: Write> EdpllOutput<'a, W> {
    fn open(path: Option<&Path>, stdout: &'a mut W) -> Result<Self, Diagnostic> {
        let Some(path) = path else {
            return Ok(Self::Stdout(stdout));
        };
        if path == Path::new("-") {
            return Ok(Self::Stdout(stdout));
        }
        File::create(path).map(Self::File).map_err(|error| {
            edpll_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
        })
    }
}

impl<W: Write> Write for EdpllOutput<'_, W> {
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

fn edpll_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!(
            "{}\n{PROGRAM_NAME}: {}",
            prefix.into(),
            c_io_error_message(error)
        ),
    )
}

fn edpll_scanner_open_diagnostic(error: Diagnostic) -> Diagnostic {
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

fn edpll_parse_diagnostic(scanner: &Scanner, error: Diagnostic) -> Diagnostic {
    if error.code() != ErrorCode::SYNTAX_ERROR || error.message().contains("(just read '") {
        return error;
    }
    let token = scanner.current_token();
    Diagnostic::new(
        error.code(),
        format!(
            "{}(just read '{}'): {}",
            token_pos_rep(token),
            token.literal(),
            error.message()
        ),
    )
}

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use super::{
        auto_memory_mb, memory_limit_bytes_from_mb, memory_limit_warnings, print_help,
        process_options, run, EdpllConfig, RunCommand, OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::os_wrapper::{get_system_phys_memory, RLimResult};
    use crate::basics::verbose::verbose_level;
    use crate::inout::output::output_level;
    use crate::inout::scanner::IoFormat;
    use crate::prover::version::{assert_help_matches_fixture, VERSION};
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

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("umlaut-dpll-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[allow(clippy::too_many_lines)]
    fn expected_help() -> String {
        format!(
            concat!(
                "\n",
                "\n",
                "umlaut-dpll {version}\n",
                "\n",
                "Usage: umlaut-dpll [options] [files]\n",
                "\n",
                "Read a set of ground clauses and try to refute (or satisfy) it.\n",
                "Not completed yet!\n",
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
                "    Verbose comments on the progress of the program by printing technical\n",
                "    information to stderr. The short form or the long form without the\n",
                "    optional argument is equivalent to --verbose=1.\n",
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
                "    Select an output level, greater values imply more verbose output. Level 0\n",
                "    produces nearly no output, level 1 produces minimal additional\n",
                "    output.Higher levels are without meaning in umlaut-dpll (I think).\n",
                "\n",
                "  --tptp-in\n",
                "    Parse TPTP format instead of lop (does not understand includes, as TPTP\n",
                "    include syntax is considered harmful).\n",
                "\n",
                "   -d\n",
                "  --dimacs\n",
                "    Print output in the DIMACS format suitable for many propositional\n",
                "    provers.\n",
                "\n",
                "   -m <arg>\n",
                "  --memory-limit=<arg>\n",
                "    Limit the memory the system may use. The argument is the allowed amount\n",
                "    of memory in MB. This option may not work everywhere, due to broken\n",
                "    and/or strange behaviour of setrlimit() in some UNIX implementations. It\n",
                "    does work under all tested versions of Solaris and GNU/Linux.\n",
                "\n",
                "  --cpu-limit[=<arg>]\n",
                "    Limit the cpu time the program should run. The optional argument is the\n",
                "    CPU time in seconds. The program will terminate immediately after\n",
                "    reaching the time limit, regardless of internal state. This option may\n",
                "    not work everywhere, due to broken and/or strange behaviour of\n",
                "    setrlimit() in some UNIX implementations. It does work under all tested\n",
                "    versions of Solaris, HP-UX and GNU/Linux. As a side effect, this option\n",
                "    will inhibit core file writing. The option without the optional argument\n",
                "    is equivalent to --cpu-limit=300.\n",
                "\n",
                "  --soft-cpu-limit[=<arg>]\n",
                "    Limit the cpu time spend in grounding. After the time expires, the prover\n",
                "    will print an partial system. The option without the optional argument is\n",
                "    equivalent to --soft-cpu-limit=310.\n",
                "\n",
                "\n",
                "Copyright (C) 2003 by Stephan Schulz, schulz@eprover.org \n",
                "\n",
                "This program is a part of the support structure for the E equational\n",
                "theorem prover. You can find the latest version of the E distribution\n",
                "as well as additional information at\n",
                "http://www.eprover.org\n",
                "\n",
                "This program is free software; you can redistribute it and/or modify\n",
                "it under the terms of the GNU General Public License as published by\n",
                "the Free Software Foundation; either version 2 of the License, or\n",
                "(at your option) any later version.\n",
                "\n",
                "This program is distributed in the hope that it will be useful,\n",
                "but WITHOUT ANY WARRANTY; without even the implied warranty of\n",
                "MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the\n",
                "GNU General Public License for more details.\n",
                "\n",
                "You should have received a copy of the GNU General Public License\n",
                "along with this program (it should be contained in the top level\n",
                "directory of the distribution in the file COPYING); if not, write to\n",
                "the Free Software Foundation, Inc., 59 Temple Place, Suite 330,\n",
                "Boston, MA  02111-1307 USA\n",
                "\n",
                "The original copyright holder can be contacted as\n",
                "\n",
                "Stephan Schulz\n",
                "DHBW Stuttgart\n",
                "Fakultaet Technik\n",
                "Informatik\n",
                "Lerchenstrasse 1\n",
                "70174 Stuttgart\n",
                "Germany\n",
                "\n",
            ),
            version = VERSION,
        )
    }

    fn assert_help_matches_rebranded_footer(actual: &str) {
        assert_help_matches_fixture(actual, &expected_help());
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("umlaut-dpll run succeeds");
        (
            status,
            String::from_utf8(stdout).expect("stdout is utf8"),
            String::from_utf8(stderr).expect("stderr is utf8"),
        )
    }

    #[test]
    fn help_and_version_preserve_c_text_and_version_typo() {
        let _guard = global_state_lock();
        let (status, help, stderr) = run_with_stdin(&[PROGRAM_NAME, "--help"], "not lop");

        assert_eq!(status, 0);
        assert_help_matches_rebranded_footer(&help);
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_stdin(&[PROGRAM_NAME, "--version"], "not lop");
        assert_eq!(status, 0);
        assert_eq!(version, format!("{PROGRAM_NAME} {VERSION}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn short_v_is_verbose_not_version() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME, "-v"], "p <- q.");

        assert_eq!(status, 0);
        assert_eq!(verbose_level(), 1);
        assert_eq!(output, "New clause: p<-q....accepted\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn stdin_lop_run_prints_clause_acceptance_trace_and_keeps_dimacs_noop() {
        let _guard = global_state_lock();
        let (status, output, stderr) =
            run_with_stdin(&[PROGRAM_NAME, "--dimacs"], "p <- q. r <- r.");

        assert_eq!(status, 0);
        assert_eq!(
            output,
            "New clause: p<-q....accepted\nNew clause: r<-r....discarded (tautology)\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn contradictory_units_preserve_c_trace_only_no_solver_contract() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], "p.\n<- p.\n");

        assert_eq!(status, 0);
        assert_eq!(
            output,
            "New clause: p<-....accepted\nNew clause: <-p....accepted\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn tptp_input_clause_mode_uses_old_tptp_parser() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(
            &[PROGRAM_NAME, "--tptp-in"],
            "input_clause(c_0_1,axiom,[++p,--q]).",
        );

        assert_eq!(status, 0);
        assert_eq!(output, "New clause: p<-q....accepted\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn trailing_non_clause_tokens_are_ignored_like_c() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME], "p. ,");

        assert_eq!(status, 0);
        assert_eq!(output, "New clause: p<-....accepted\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn empty_procedural_tail_reports_c_diagnostic() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"p :- .\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error =
            run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr).expect_err("tail is empty");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(
            error.message(),
            "<stdin>:1:(Column 6):(just read '.'): Procedural rule or query needs at least one tail literal (Hey! I did not make this  syntax! -StS)"
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn malformed_later_clause_preserves_prior_c_trace_output() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"p.\nq(f(a).\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("malformed second clause is reported");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(
            error.message(),
            "<stdin>:2:(Column 7):(just read '.'): Closing bracket (')') expected, but Fullstop ('.') read "
        );
        assert_eq!(
            String::from_utf8(stdout).expect("stdout is utf8"),
            "New clause: p<-....accepted\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn malformed_equation_preserves_exact_term_expectation() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"p(a)=.\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("missing equation right side is reported");

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert_eq!(
            error.message(),
            "<stdin>:1:(Column 6):(just read '.'): Identifier not terminating in a number or Identifier terminating in a number or Interpreted function/predicate name ('$name') or String enclosed in double quotes (\"\") or String enclosed in single quote ('') or Integer (sequence of decimal digits) convertible to an 'unsigned long' or Hyphen ('-') or Plus sign ('+') expected, but Fullstop ('.') read "
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn output_file_receives_trace() {
        let _guard = global_state_lock();
        let input_path = temp_path("input");
        let output_path = temp_path("output");
        remove_if_present(&input_path);
        remove_if_present(&output_path);
        std::fs::write(&input_path, "p.").expect("input fixture is written");

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
        assert_eq!(output, "New clause: p<-....accepted\n");

        remove_if_present(&input_path);
        remove_if_present(&output_path);
    }

    #[test]
    fn output_dash_routes_trace_to_stdout_like_c() {
        let _guard = global_state_lock();
        let (status, output, stderr) = run_with_stdin(&[PROGRAM_NAME, "-o", "-"], "p.");

        assert_eq!(status, 0);
        assert_eq!(output, "New clause: p<-....accepted\n");
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
    fn non_clause_lop_input_is_ignored_like_c() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b",\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status =
            run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr).expect("run succeeds");

        assert_eq!(status, 0);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn output_close_failure_uses_c_outclose_diagnostic() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"p.\n".to_vec());
        let mut stdout = FlushFailWriter;
        let mut stderr = Vec::new();

        let error = run([PROGRAM_NAME], &mut stdin, &mut stdout, &mut stderr)
            .expect_err("flush failure is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
        assert!(stderr.is_empty());
    }

    #[test]
    fn silent_and_output_level_mutate_global_output_level() {
        let _guard = global_state_lock();
        let (status, _output, stderr) = run_with_stdin(&[PROGRAM_NAME, "--silent"], "");
        assert_eq!(status, 0);
        assert_eq!(output_level(), 0);
        assert!(stderr.is_empty());

        let (status, _output, stderr) = run_with_stdin(&[PROGRAM_NAME, "--output-level=3"], "");
        assert_eq!(status, 0);
        assert_eq!(output_level(), 3);
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_resource_and_format_options() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--tptp-in",
                "--dimacs",
                "--memory-limit=128",
                "--soft-cpu-limit=10",
                "--cpu-limit=30",
                "problem.p",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("options parse");

        let RunCommand::Execute(EdpllConfig {
            parse_format,
            memory_limit,
            hard_cpu_limit,
            soft_cpu_limit,
            files,
            ..
        }) = command
        else {
            panic!("expected execute command");
        };
        assert_eq!(parse_format, IoFormat::Tptp);
        assert_eq!(memory_limit, 128 * 1_048_576);
        assert_eq!(hard_cpu_limit, Some(30));
        assert_eq!(soft_cpu_limit, Some(10));
        assert_eq!(files, ["problem.p"]);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn invalid_hard_soft_limit_order_is_rejected_like_c() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let error = process_options(
            [PROGRAM_NAME, "--soft-cpu-limit=10", "--cpu-limit=10"],
            &mut stdout,
            &mut stderr,
        )
        .expect_err("hard limit must be larger");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Hard time limit has to be larger than softtime limit"
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());

        let error = process_options(
            [PROGRAM_NAME, "--cpu-limit=10", "--soft-cpu-limit=10"],
            &mut stdout,
            &mut stderr,
        )
        .expect_err("soft limit must be smaller");

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Soft time limit has to be smaller than hardtime limit"
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn memory_limit_auto_follows_available_system_memory_when_known() {
        let _guard = global_state_lock();
        let system_memory = get_system_phys_memory();
        if system_memory == -1 {
            return;
        }
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command = process_options(
            [PROGRAM_NAME, "--memory-limit=Auto"],
            &mut stdout,
            &mut stderr,
        )
        .expect("auto memory parses when system memory is known");

        let RunCommand::Execute(config) = command else {
            panic!("expected execute command");
        };
        assert_eq!(
            config.memory_limit,
            memory_limit_bytes_from_mb(auto_memory_mb(system_memory))
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn verbose_auto_memory_preserves_c_value_and_unit_text() {
        let _guard = global_state_lock();
        let system_memory = get_system_phys_memory();
        if system_memory == -1 {
            return;
        }
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command = process_options(
            [PROGRAM_NAME, "--verbose=1", "--memory-limit=Auto"],
            &mut stdout,
            &mut stderr,
        )
        .expect("verbose auto memory parses");

        let RunCommand::Execute(config) = command else {
            panic!("expected execute command");
        };
        let expected_limit = memory_limit_bytes_from_mb(auto_memory_mb(system_memory));
        assert_eq!(config.memory_limit, expected_limit);
        assert_eq!(
            String::from_utf8(stderr).expect("stderr is utf8"),
            format!(
                "Physical memory determined as {system_memory} MB\n\
                 Memory limit set to {expected_limit} MB\n"
            )
        );
        assert!(stdout.is_empty());
    }

    #[test]
    fn memory_limit_reduction_preserves_both_c_warning_descriptions() {
        assert!(memory_limit_warnings(RLimResult::Success).is_empty());
        assert!(memory_limit_warnings(RLimResult::Failed).is_empty());

        let warnings = memory_limit_warnings(RLimResult::Reduced);
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].message(), "Had to reduce limit RLIMIT_DATA");
        assert_eq!(warnings[1].message(), "Had to reduce limit RLIMIT_AS");
        assert_eq!(
            warnings
                .iter()
                .map(|warning| warning.render_warning(PROGRAM_NAME))
                .collect::<String>(),
            "umlaut-dpll: Warning: Had to reduce limit RLIMIT_DATA\n\
             umlaut-dpll: Warning: Had to reduce limit RLIMIT_AS\n"
        );
    }

    #[test]
    fn print_help_mentions_incomplete_c_tool_status() {
        assert_help_matches_rebranded_footer(&print_help());
    }
}
