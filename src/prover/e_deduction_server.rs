use std::io::{BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::verbose::set_verbose_level;
use crate::control::batch_spec::{BatchOutputType, BatchProcCtrlRunnerSet, BatchSpec};
use crate::control::einteractive_mode::{
    run_command_with_runner_backend, start_deduction_server_tcp_with, InteractiveCommandOutput,
    InteractiveRunReport, InteractiveServerReport, InteractiveSpec,
};
use crate::control::sine::StructFofSpec;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell, ParsedOpt,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::network::{create_server_socket, listen};
use crate::inout::output::set_output_level;
use crate::inout::scanner::IoFormat;
use crate::prover::version::{footer, E_NICKNAME, VERSION};
use crate::terms::{signature::Signature, termbanks::TermBank, typebanks::TypeBank};

pub const PROGRAM_NAME: &str = "e_deduction_server";
const DEFAULT_PROVER: &str = "eprover";
const DEFAULT_TOTAL_WTC_LIMIT: i64 = 30;
const STDOUT_SERVER_UNIMPLEMENTED_MESSAGE: &str =
    "e_deduction_server: Server mode not implemented yet for stdout\n";
const OUTPUT_CLOSE_ERROR: &str =
    "Output stream to be closed reports error (probably broken pipe, file system full or quota exceeded)";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Port,
    Silent,
    OutputLevel,
    GlobalWtcLimit,
    ServerLib,
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
        OptionCode::Verbose,
        Some('v'),
        Some("verbose"),
        OptArgType::OptArg,
        Some("1"),
        "Verbose comments on the progress of the program. This differs from the output level (below) in that technical information is printed to stderr, while the output level determines which logical manipulations of the clauses are printed to stdout.",
    ),
    OptCell::new(
        OptionCode::Port,
        Some('p'),
        Some("port"),
        OptArgType::ReqArg,
        None,
        "The port on which the server will receive connections. Only effective when interactive mode is on. If not given stdin/stdout will be used.",
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
        "Select an output level, greater values imply more verbose output. Level 0 produces nearly no output, level 1 will output each clause as it is processed, level 2 will output generating inferences, level 3 will give a full protocol including rewrite steps and level 4 will include some internal clause renamings. Levels >= 2 also imply PCL2 or TSTP formats (which can be post-processed with suitable tools).",
    ),
    OptCell::new(
        OptionCode::GlobalWtcLimit,
        Some('w'),
        Some("wtc-limit"),
        OptArgType::ReqArg,
        None,
        "Set the global wall-clock limit for each batch (if any).",
    ),
    OptCell::new(
        OptionCode::ServerLib,
        Some('L'),
        Some("lib"),
        OptArgType::ReqArg,
        None,
        "Set the axioms library directory of the server.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeductionServerConfig {
    prover: String,
    port: Option<u16>,
    server_lib: String,
    total_wtc_limit: i64,
    verbose_level: i64,
    output_level: i64,
}

impl Default for DeductionServerConfig {
    fn default() -> Self {
        Self {
            prover: DEFAULT_PROVER.to_owned(),
            port: None,
            server_lib: String::new(),
            total_wtc_limit: DEFAULT_TOTAL_WTC_LIMIT,
            verbose_level: 0,
            output_level: 1,
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(DeductionServerConfig),
    Exit(u8),
}

pub fn run<I, S, R, W, E>(
    argv: I,
    stdin: &mut R,
    stdout: &mut W,
    _stderr: &mut E,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    R: BufRead,
    W: Write,
    E: Write,
{
    init_io(PROGRAM_NAME);
    set_verbose_level(0);
    let result = run_inner(argv, stdin, stdout);
    exit_io();
    result
}

fn run_inner<I, S, R, W>(argv: I, stdin: &mut R, stdout: &mut W) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    R: BufRead,
    W: Write,
{
    match process_options(argv, stdout)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_config(&config, stdin, stdout),
    }
}

fn process_options<I, S>(argv: I, stdout: &mut impl Write) -> Result<RunCommand, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = DeductionServerConfig::default();

    while let Some(parsed) = state.next_opt(OPTIONS)? {
        match parsed.option().option_code {
            OptionCode::Verbose => {
                config.verbose_level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            }
            OptionCode::Help => {
                write_all(stdout, print_help().as_bytes())?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::Version => {
                writeln_diag(stdout, &format!("{PROGRAM_NAME} {VERSION} {E_NICKNAME}"))?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::Port => {
                config.port = Some(parse_port(parsed.option(), required_arg(&parsed, "port")?)?);
            }
            OptionCode::Silent => config.output_level = 0,
            OptionCode::OutputLevel => {
                config.output_level =
                    get_int_arg(parsed.option(), required_arg(&parsed, "output-level")?)?;
            }
            OptionCode::GlobalWtcLimit => {
                config.total_wtc_limit =
                    get_int_arg(parsed.option(), required_arg(&parsed, "wtc-limit")?)?;
            }
            OptionCode::ServerLib => {
                config
                    .server_lib
                    .clone_from(&required_arg(&parsed, "lib")?.to_owned());
            }
        }
    }

    if let Some(prover) = state.remaining_args().first() {
        config.prover.clone_from(prover);
    }

    Ok(RunCommand::Execute(config))
}

fn execute_config<R, W>(
    config: &DeductionServerConfig,
    _stdin: &mut R,
    stdout: &mut W,
) -> Result<u8, Diagnostic>
where
    R: BufRead,
    W: Write,
{
    apply_global_options(config);
    let spec = deduction_batch_spec(&config.prover, config.total_wtc_limit);
    if let Some(port) = config.port {
        serve_tcp(port, &config.server_lib, &spec, stdout)?;
    } else {
        write_all(stdout, STDOUT_SERVER_UNIMPLEMENTED_MESSAGE.as_bytes())?;
    }
    stdout
        .flush()
        .map_err(|_error| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(ErrorCode::NO_ERROR.exit_status())
}

pub fn run_text_server_with<R, W, F>(
    input: &mut R,
    output: &mut W,
    server_lib: &str,
    spec: &BatchSpec,
    bank: &mut TermBank,
    ctrl: &mut StructFofSpec,
    mut run_command: F,
) -> Result<InteractiveServerReport, Diagnostic>
where
    R: BufRead,
    W: Write,
    F: FnMut(
        &BatchSpec,
        &mut TermBank,
        &mut StructFofSpec,
        &str,
        &str,
    ) -> Result<InteractiveCommandOutput, Diagnostic>,
{
    let mut interactive = InteractiveSpec::new(server_lib);
    let mut report = InteractiveServerReport {
        commands: 0,
        done: false,
    };

    while !report.done {
        let mut command = String::new();
        let read = input
            .read_line(&mut command)
            .map_err(|error| io_diagnostic(format!("Cannot read command: {error}")))?;
        if read == 0 {
            break;
        }

        let result = interactive.dispatch_text_command_with(
            &command,
            input,
            spec,
            bank,
            ctrl,
            &mut run_command,
        )?;
        report.commands += 1;
        report.done = result.done;
        if !result.output.is_empty() {
            write_all(output, result.output.as_bytes())?;
            output
                .flush()
                .map_err(|error| io_diagnostic(format!("Cannot flush output: {error}")))?;
        }
    }

    Ok(report)
}

fn serve_tcp(
    port: u16,
    server_lib: &str,
    spec: &BatchSpec,
    stdout: &mut impl Write,
) -> Result<(), Diagnostic> {
    let listener = create_server_socket(port)?;
    listen(&listener)?;
    loop {
        match listener.accept() {
            Ok((mut stream, _address)) => {
                writeln_diag(stdout, "Client connected ..")?;
                stdout
                    .flush()
                    .map_err(|error| io_diagnostic(format!("Cannot flush output: {error}")))?;
                let mut bank = new_term_bank()?;
                let mut ctrl = StructFofSpec::new(bank.signature());
                let mut backend = BatchProcCtrlRunnerSet::new();
                start_deduction_server_tcp_with(
                    &mut stream,
                    server_lib.to_owned(),
                    spec,
                    &mut bank,
                    &mut ctrl,
                    |spec, bank, ctrl, job_name, input_axioms| {
                        run_command_with_runner_backend(
                            job_name,
                            input_axioms,
                            spec,
                            bank,
                            ctrl,
                            current_time_seconds,
                            &mut backend,
                        )
                        .and_then(|report| emit_run_report_global_output(report, stdout))
                    },
                )?;
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::Interrupted
                ) => {}
            Err(error) => {
                return Err(Diagnostic::new(
                    ErrorCode::SYSTEM_ERROR,
                    format!("Unable to listen on socket {port}: {error}"),
                ));
            }
        }
    }
}

fn emit_run_report_global_output<W: Write>(
    report: InteractiveRunReport,
    stdout: &mut W,
) -> Result<InteractiveCommandOutput, Diagnostic> {
    if !report.global_output.is_empty() {
        write_all(stdout, report.global_output.as_bytes())?;
        stdout
            .flush()
            .map_err(|error| io_diagnostic(format!("Cannot flush output: {error}")))?;
    }
    Ok(report.command)
}

#[must_use]
fn deduction_batch_spec(prover: &str, total_wtc_limit: i64) -> BatchSpec {
    let mut spec = BatchSpec::new(prover, IoFormat::Tstp);
    spec.category = Some("dummy".to_owned());
    spec.total_wtc_limit = total_wtc_limit;
    spec.res_proof = BatchOutputType::Desired;
    spec
}

fn new_term_bank() -> Result<TermBank, Diagnostic> {
    let mut signature = Signature::new(TypeBank::new());
    signature.insert_internal_codes()?;
    TermBank::new(signature)
}

fn apply_global_options(config: &DeductionServerConfig) {
    set_verbose_level(i64_to_i32_saturating(config.verbose_level));
    let _old_output_level = set_output_level(config.output_level);
}

fn parse_port<Code>(option: &OptCell<Code>, arg: &str) -> Result<u16, Diagnostic> {
    let port = get_int_arg(option, arg)?;
    u16::try_from(port).map_err(|_| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Port numbers must be between 0 and 65535",
        )
    })
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
{PROGRAM_NAME} {VERSION} \"{E_NICKNAME}\"\n\
\n\
Usage: {PROGRAM_NAME} -p <port> [options] [files]\n\
\n\
The E deduction server offers deduction services based on local or\n\
uploaded axiom sets via network. See README.server.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options:\n\n")));
    result.push_str("\n\n");
    result.push_str(&footer());
    result
}

fn required_arg<'a>(
    parsed: &'a ParsedOpt<'a, OptionCode>,
    name: &str,
) -> Result<&'a str, Diagnostic> {
    parsed.arg().ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Option {name} requires an argument"),
        )
    })
}

fn current_time_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        })
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
    use std::io::{self, Cursor, Write};

    use super::{
        deduction_batch_spec, emit_run_report_global_output, parse_port, print_help,
        process_options, run, run_text_server_with, DeductionServerConfig, RunCommand,
        DEFAULT_PROVER, DEFAULT_TOTAL_WTC_LIMIT, OUTPUT_CLOSE_ERROR, PROGRAM_NAME,
        STDOUT_SERVER_UNIMPLEMENTED_MESSAGE,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::control::batch_spec::{BatchOutputType, BatchProcessProblemReport, BatchSpec};
    use crate::control::einteractive_mode::{
        InteractiveCommandOutput, InteractiveRunReport, END_OF_BLOCK_TOKEN, HELP_MESSAGE,
        OK_SUCCESS_MESSAGE,
    };
    use crate::control::sine::{StructFofSpec, StructFofSpecBacktrackReport};
    use crate::inout::output::output_level;
    use crate::inout::scanner::IoFormat;
    use crate::prover::version::{E_NICKNAME, VERSION};
    use crate::terms::{signature::Signature, termbanks::TermBank, typebanks::TypeBank};
    use crate::test_support::global_state_lock;

    struct FlushFailWriter;

    impl Write for FlushFailWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            Ok(buffer.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }
    }

    fn parser_bank() -> TermBank {
        let mut signature = Signature::new(TypeBank::new());
        signature.insert_internal_codes().unwrap();
        TermBank::new(signature).unwrap()
    }

    #[allow(clippy::too_many_lines)]
    fn expected_help() -> String {
        format!(
            concat!(
                "\n",
                "e_deduction_server {version} \"{nickname}\"\n",
                "\n",
                "Usage: e_deduction_server -p <port> [options] [files]\n",
                "\n",
                "The E deduction server offers deduction services based on local or\n",
                "uploaded axiom sets via network. See README.server.\n",
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
                "   -v\n",
                "  --verbose[=<arg>]\n",
                "    Verbose comments on the progress of the program. This differs from the\n",
                "    output level (below) in that technical information is printed to stderr,\n",
                "    while the output level determines which logical manipulations of the\n",
                "    clauses are printed to stdout. The short form or the long form without\n",
                "    the optional argument is equivalent to --verbose=1.\n",
                "\n",
                "   -p <arg>\n",
                "  --port=<arg>\n",
                "    The port on which the server will receive connections. Only effective\n",
                "    when interactive mode is on. If not given stdin/stdout will be used.\n",
                "\n",
                "   -s\n",
                "  --silent\n",
                "    Equivalent to --output-level=0.\n",
                "\n",
                "   -l <arg>\n",
                "  --output-level=<arg>\n",
                "    Select an output level, greater values imply more verbose output. Level 0\n",
                "    produces nearly no output, level 1 will output each clause as it is\n",
                "    processed, level 2 will output generating inferences, level 3 will give a\n",
                "    full protocol including rewrite steps and level 4 will include some\n",
                "    internal clause renamings. Levels >= 2 also imply PCL2 or TSTP formats\n",
                "    (which can be post-processed with suitable tools).\n",
                "\n",
                "   -w <arg>\n",
                "  --wtc-limit=<arg>\n",
                "    Set the global wall-clock limit for each batch (if any).\n",
                "\n",
                "   -L <arg>\n",
                "  --lib=<arg>\n",
                "    Set the axioms library directory of the server.\n",
                "\n",
                "\n",
                "\n",
                "Copyright 1998-2026 by Stephan Schulz, schulz@eprover.org,\n",
                "and the E contributors (see DOC/CONTRIBUTORS).\n",
                "\n",
                "This program is a part of the distribution of the equational theorem\n",
                "prover E. You can find the latest version of the E distribution\n",
                "as well as additional information at\n",
                "http://www.eprover.org\n",
                "\n",
                "This program is free software; you can redistribute it and/or modify\n",
                "it under the terms of the GNU General Public License as published by\n",
                "the Free Software Foundation; either version 2 of the License, or\n",
                "(at your option) any later version.\n",
                "\n",
                "Bug reports for the first-order prover should be sent to <schulz@eprover.org>.\n",
                "Bug reports with respect to the HO-version should be sent to or at least copied to\n",
                "<jasmin.blanchette@gmail.com>.\n",
            ),
            version = VERSION,
            nickname = E_NICKNAME,
        )
    }

    fn ok_run_command(
        jobs: &mut Vec<(String, String)>,
        _spec: &BatchSpec,
        _bank: &mut TermBank,
        _ctrl: &mut StructFofSpec,
        job_name: &str,
        input_axioms: &str,
    ) -> InteractiveCommandOutput {
        jobs.push((job_name.to_owned(), input_axioms.to_owned()));
        InteractiveCommandOutput {
            output: format!("proof for {job_name}\n"),
            status: OK_SUCCESS_MESSAGE,
        }
    }

    #[test]
    fn help_and_version_exit_before_server_start() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::<u8>::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let help_status = run(
            [PROGRAM_NAME, "--help"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("help");
        assert_eq!(help_status, ErrorCode::NO_ERROR.exit_status());
        let help = String::from_utf8(stdout).unwrap();
        assert_eq!(help, expected_help());

        let mut stdin = Cursor::new(Vec::<u8>::new());
        let mut stdout = Vec::new();
        let version_status =
            run([PROGRAM_NAME, "-V"], &mut stdin, &mut stdout, &mut stderr).expect("version");
        assert_eq!(version_status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!("{PROGRAM_NAME} {VERSION} {E_NICKNAME}\n")
        );
    }

    #[test]
    fn help_and_version_exit_before_outclose_like_c() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::<u8>::new());
        let mut stderr = Vec::new();

        let help_status = run(
            [PROGRAM_NAME, "--help"],
            &mut stdin,
            &mut FlushFailWriter,
            &mut stderr,
        )
        .expect("help does not final-flush stdout");
        assert_eq!(help_status, ErrorCode::NO_ERROR.exit_status());

        let mut stdin = Cursor::new(Vec::<u8>::new());
        let version_status = run(
            [PROGRAM_NAME, "--version"],
            &mut stdin,
            &mut FlushFailWriter,
            &mut stderr,
        )
        .expect("version does not final-flush stdout");
        assert_eq!(version_status, ErrorCode::NO_ERROR.exit_status());
    }

    #[test]
    fn process_options_preserves_c_defaults_and_positional_prover() {
        let mut stdout = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "-p",
                "3667",
                "-w",
                "0",
                "-L",
                "Axioms",
                "--verbose",
                "--output-level=2",
                "custom-eprover",
                "ignored-extra",
            ],
            &mut stdout,
        )
        .expect("options");
        let RunCommand::Execute(config) = command else {
            panic!("valid arguments should execute");
        };

        assert_eq!(
            config,
            DeductionServerConfig {
                prover: "custom-eprover".to_owned(),
                port: Some(3667),
                server_lib: "Axioms".to_owned(),
                total_wtc_limit: 0,
                verbose_level: 1,
                output_level: 2,
            }
        );
    }

    #[test]
    fn process_options_default_server_uses_stdio_and_eprover() {
        let mut stdout = Vec::new();
        let command = process_options([PROGRAM_NAME, "-s"], &mut stdout).expect("options");
        let RunCommand::Execute(config) = command else {
            panic!("valid arguments should execute");
        };

        assert_eq!(config.prover, DEFAULT_PROVER);
        assert_eq!(config.port, None);
        assert_eq!(config.server_lib, "");
        assert_eq!(config.total_wtc_limit, DEFAULT_TOTAL_WTC_LIMIT);
        assert_eq!(config.verbose_level, 0);
        assert_eq!(config.output_level, 0);
    }

    #[test]
    fn parse_port_rejects_values_outside_u16_range() {
        let option = &super::OPTIONS[3];
        assert_eq!(parse_port(option, "0").unwrap(), 0);
        assert_eq!(parse_port(option, "65535").unwrap(), 65_535);

        let error = parse_port(option, "65536").unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Port numbers must be between 0 and 65535");
    }

    #[test]
    fn deduction_spec_matches_c_server_defaults() {
        let spec = deduction_batch_spec("custom-e", 44);

        assert_eq!(spec.executable, "custom-e");
        assert_eq!(spec.format, IoFormat::Tstp);
        assert_eq!(spec.category.as_deref(), Some("dummy"));
        assert_eq!(spec.total_wtc_limit, 44);
        assert_eq!(spec.res_proof, BatchOutputType::Desired);
    }

    #[test]
    fn text_server_dispatches_immediate_commands_until_quit() {
        let mut input = Cursor::new(b"HELP\nLIST\nQUIT\n".to_vec());
        let mut output = Vec::new();
        let spec = deduction_batch_spec(DEFAULT_PROVER, DEFAULT_TOTAL_WTC_LIMIT);
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let mut unused_jobs = Vec::new();

        let report = run_text_server_with(
            &mut input,
            &mut output,
            "",
            &spec,
            &mut bank,
            &mut ctrl,
            |spec, bank, ctrl, job_name, input_axioms| {
                Ok(ok_run_command(
                    &mut unused_jobs,
                    spec,
                    bank,
                    ctrl,
                    job_name,
                    input_axioms,
                ))
            },
        )
        .expect("text session");

        assert_eq!(report.commands, 3);
        assert!(report.done);
        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with(HELP_MESSAGE));
        assert!(output.contains("No Axiom Sets currently in memory.\n"));
        assert!(output.contains("On Disk :\n"));
        assert!(output.ends_with(OK_SUCCESS_MESSAGE));
    }

    #[test]
    fn text_server_reads_run_block_from_line_transport() {
        let input_text =
            format!("RUN job1\nfof(job_formula, axiom, q(a)).\n{END_OF_BLOCK_TOKEN}QUIT\n");
        let mut input = Cursor::new(input_text.into_bytes());
        let mut output = Vec::new();
        let spec = deduction_batch_spec(DEFAULT_PROVER, DEFAULT_TOTAL_WTC_LIMIT);
        let mut bank = parser_bank();
        let mut ctrl = StructFofSpec::new(bank.signature());
        let mut jobs = Vec::new();

        let report = run_text_server_with(
            &mut input,
            &mut output,
            "",
            &spec,
            &mut bank,
            &mut ctrl,
            |spec, bank, ctrl, job_name, input_axioms| {
                Ok(ok_run_command(
                    &mut jobs,
                    spec,
                    bank,
                    ctrl,
                    job_name,
                    input_axioms,
                ))
            },
        )
        .expect("text session");

        assert_eq!(report.commands, 2);
        assert!(report.done);
        assert_eq!(
            jobs,
            vec![(
                "job1".to_owned(),
                "fof(job_formula, axiom, q(a)).\n".to_owned()
            )]
        );
        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("proof for job1\n{OK_SUCCESS_MESSAGE}")
        );
    }

    #[test]
    fn run_applies_output_and_verbose_globals_for_stdout_unimplemented_path() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(b"QUIT\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let status = run(
            [PROGRAM_NAME, "--verbose=3", "--output-level=4"],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        )
        .expect("run");

        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(verbose_level(), 3);
        assert_eq!(output_level(), 4);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            STDOUT_SERVER_UNIMPLEMENTED_MESSAGE
        );
    }

    #[test]
    fn run_reports_final_outclose_flush_failure_like_c() {
        let _guard = global_state_lock();
        let mut stdin = Cursor::new(Vec::<u8>::new());
        let mut stderr = Vec::new();

        let error = run(
            [PROGRAM_NAME],
            &mut stdin,
            &mut FlushFailWriter,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert_eq!(error.message(), OUTPUT_CLOSE_ERROR);
    }

    #[test]
    fn run_report_adapter_emits_c_stdout_side_channel_and_returns_socket_output() {
        let report = InteractiveRunReport {
            command: InteractiveCommandOutput {
                output: "socket output\n".to_owned(),
                status: OK_SUCCESS_MESSAGE,
            },
            process: BatchProcessProblemReport {
                solved: false,
                spawned: 0,
                completed: None,
                backtrack: StructFofSpecBacktrackReport {
                    removed_clause_sets: 0,
                    removed_formula_sets: 0,
                    signature_backtrack_to: 0,
                },
            },
            global_output: "job1% global proof output\n".to_owned(),
        };
        let mut stdout = Vec::new();

        let command = emit_run_report_global_output(report, &mut stdout).unwrap();

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "job1% global proof output\n"
        );
        assert_eq!(
            command,
            InteractiveCommandOutput {
                output: "socket output\n".to_owned(),
                status: OK_SUCCESS_MESSAGE,
            }
        );
    }

    #[test]
    fn print_help_preserves_full_c_text() {
        assert_eq!(print_help(), expected_help());
    }
}
