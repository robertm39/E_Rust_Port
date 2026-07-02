use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::verbose::{set_verbose_level, verbout};
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::network::{
    create_client_socket, tcp_string_recv_from_or_error, tcp_string_send_to_or_error,
};
use crate::prover::version::{E_NICKNAME, E_URL, STS_MAIL, VERSION};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const PROGRAM_NAME: &str = "e_client";
const DEFAULT_SERVER: &str = "localhost";
const DEFAULT_PORT: u16 = 3666;
const IPPORT_RESERVED: u16 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    Server,
    Port,
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
        OptionCode::Output,
        Some('o'),
        Some("output-file"),
        OptArgType::ReqArg,
        None,
        "Redirect output into the named file (this affects only some output, as most is written to automatically generated files based on the input and filter names.",
    ),
    OptCell::new(
        OptionCode::Server,
        Some('S'),
        Some("server"),
        OptArgType::ReqArg,
        None,
        "Specify the address of the server. The default is 'localhost'.",
    ),
    OptCell::new(
        OptionCode::Port,
        Some('P'),
        Some("service-port"),
        OptArgType::ReqArg,
        None,
        "Specify the port to use for the deduction service. The default is to use 3666",
    ),
    OptCell::new(
        OptionCode::Port,
        Some('P'),
        Some("port"),
        OptArgType::ReqArg,
        None,
        "Specify the port to use for the deduction service.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct EClientConfig {
    output_file: Option<PathBuf>,
    server: String,
    port: u16,
    files: Vec<String>,
}

impl Default for EClientConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            server: DEFAULT_SERVER.to_owned(),
            port: DEFAULT_PORT,
            files: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(EClientConfig),
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
        RunCommand::Execute(config) => execute_e_client(&config, stdin, stdout, stderr),
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
    let mut config = EClientConfig::default();

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
                writeln_diag(stdout, &format!("E {VERSION} {E_NICKNAME}"))?;
                return Ok(RunCommand::Exit(0));
            }
            OptionCode::Output => {
                config.output_file = parsed.arg().map(PathBuf::from);
            }
            OptionCode::Server => {
                parsed.arg().unwrap_or("").clone_into(&mut config.server);
            }
            OptionCode::Port => {
                config.port = parse_port(parsed.option(), parsed.arg().unwrap_or(""), stderr)?;
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(RunCommand::Execute(config))
}

fn parse_port<Code>(
    option: &OptCell<Code>,
    arg: &str,
    stderr: &mut impl Write,
) -> Result<u16, Diagnostic> {
    let port = get_int_arg(option, arg)?;
    if !(0..=i64::from(u16::MAX)).contains(&port) {
        return Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Port numbers must be between 0 and 65535",
        ));
    }
    let port = u16::try_from(port).map_err(|_| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Port numbers must be between 0 and 65535",
        )
    })?;
    if port < IPPORT_RESERVED {
        write_all(
            stderr,
            Diagnostic::new(
                ErrorCode::OTHER_ERROR,
                format!("Port numbers less than {IPPORT_RESERVED} require root level access"),
            )
            .render_warning(PROGRAM_NAME)
            .as_bytes(),
        )?;
    }
    Ok(port)
}

fn execute_e_client(
    config: &EClientConfig,
    stdin: &mut impl Read,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic> {
    let mut output_file = open_output_file(config.output_file.as_deref())?;
    let problem = load_problem_files(&config.files, stdin)?;
    verbout_diag(stderr, "Problem input read\n")?;
    let mut stream = create_client_socket(&config.server, config.port)?;
    let output: &mut dyn Write = match output_file.as_mut() {
        Some(file) => file,
        None => stdout,
    };
    execute_client_protocol(&mut stream, output, &problem)?;
    output
        .flush()
        .map_err(|_| io_diagnostic(OUTPUT_CLOSE_ERROR))?;
    Ok(0)
}

fn execute_client_protocol<S, W>(
    stream: &mut S,
    output: &mut W,
    problem: &str,
) -> Result<(), Diagnostic>
where
    S: Read + Write,
    W: Write + ?Sized,
{
    tcp_string_send_to_or_error(stream, "hello")?;
    tcp_msg_wait(stream, output, "ready")?;
    tcp_string_send_to_or_error(stream, "add")?;
    tcp_string_send_to_or_error(stream, problem)?;
    tcp_string_send_to_or_error(stream, "prove")?;
    tcp_msg_wait(stream, output, "result")
}

fn tcp_msg_wait<S, W>(stream: &mut S, output: &mut W, reply: &str) -> Result<(), Diagnostic>
where
    S: Read,
    W: Write + ?Sized,
{
    loop {
        let msg = tcp_string_recv_from_or_error(stream)?;
        writeln_diag(output, &format!("% Server: {msg}"))?;
        if msg == reply {
            return Ok(());
        }
    }
}

fn load_problem_files(files: &[String], stdin: &mut impl Read) -> Result<String, Diagnostic> {
    let mut result = Vec::new();
    for file in files {
        if file == "-" {
            stdin
                .read_to_end(&mut result)
                .map_err(|error| io_diagnostic(format!("Cannot read stdin: {error}")))?;
        } else {
            let mut data = std::fs::read(Path::new(file))
                .map_err(|error| io_diagnostic(format!("Cannot read file {file}: {error}")))?;
            result.append(&mut data);
        }
    }
    String::from_utf8(result)
        .map_err(|error| io_diagnostic(format!("Invalid UTF-8 input: {error}")))
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
E {VERSION} \"{E_NICKNAME}\"\n\
\n\
Usage: {PROGRAM_NAME} [options] [files]\n\
\n\
Read an problem specification, connect to the E deduction server, \n\
and try to have the problem solved.\n\
\n"
    );
    result.push_str(&print_options(OPTIONS, Some("Options:\n\n")));
    result.push('\n');
    result.push_str(&legacy_footer());
    result
}

fn legacy_footer() -> String {
    format!(
        "Copyright (C) 2011 by Stephan Schulz, {STS_MAIL}\n\
\n\
You can find the latest version of E and additional information at\n\
{E_URL}\n\
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

fn open_output_file(path: Option<&Path>) -> Result<Option<std::fs::File>, Diagnostic> {
    let Some(path) = path else {
        return Ok(None);
    };
    if path == Path::new("-") {
        return Ok(None);
    }
    std::fs::File::create(path)
        .map(Some)
        .map_err(|error| io_diagnostic(format!("Cannot open file {}: {error}", path.display())))
}

fn verbout_diag(output: &mut impl Write, message: &str) -> Result<(), Diagnostic> {
    let _ =
        verbout(output, PROGRAM_NAME, message).map_err(|error| io_diagnostic(error.to_string()))?;
    Ok(())
}

fn write_all(output: &mut (impl Write + ?Sized), bytes: &[u8]) -> Result<(), Diagnostic> {
    output
        .write_all(bytes)
        .map_err(|error| io_diagnostic(format!("Cannot write output: {error}")))
}

fn writeln_diag(output: &mut (impl Write + ?Sized), line: &str) -> Result<(), Diagnostic> {
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
    use super::{
        execute_client_protocol, load_problem_files, parse_port, print_help, process_options, run,
        EClientConfig, RunCommand, DEFAULT_PORT, DEFAULT_SERVER, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::inout::network::{
        tcp_string_recv_from_or_error, tcp_string_send_to_or_error, TcpMessage,
    };
    use crate::prover::version::{E_NICKNAME, VERSION};
    use crate::test_support::global_state_lock;
    use std::io::{Cursor, Read, Write};
    use std::path::{Path, PathBuf};

    #[derive(Debug)]
    struct Duplex {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl Duplex {
        fn new(messages: &[&str]) -> Self {
            let mut input = Vec::new();
            for message in messages {
                let packed = TcpMessage::pack(message).expect("message packs");
                input.extend_from_slice(packed.content_bytes());
            }
            Self {
                input: Cursor::new(input),
                output: Vec::new(),
            }
        }
    }

    impl Read for Duplex {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl Write for Duplex {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("e-client-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn sent_strings(bytes: &[u8]) -> Vec<String> {
        let mut cursor = Cursor::new(bytes.to_vec());
        let mut result = Vec::new();
        while usize::try_from(cursor.position()).expect("cursor position fits usize") < bytes.len()
        {
            result.push(tcp_string_recv_from_or_error(&mut cursor).expect("sent message decodes"));
        }
        result
    }

    fn run_with_stdin(args: &[&str], stdin_data: &str) -> (u8, String, String) {
        let mut stdin = Cursor::new(stdin_data.as_bytes().to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(args.iter().copied(), &mut stdin, &mut stdout, &mut stderr)
            .expect("e_client run succeeds");
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
        assert!(help.starts_with(&format!("\nE {VERSION} \"{E_NICKNAME}\"\n\n")));
        assert!(help.contains("Usage: e_client [options] [files]"));
        assert!(help.contains("Read an problem specification"));
        assert!(help.contains("Copyright (C) 2011 by Stephan Schulz"));
        assert!(stderr.is_empty());

        let (status, version, stderr) = run_with_stdin(&[PROGRAM_NAME, "-V"], "ignored");
        assert_eq!(status, 0);
        assert_eq!(version, format!("E {VERSION} {E_NICKNAME}\n"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn process_options_records_client_options() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose=2",
                "--output-file=client.out",
                "--server=example.invalid",
                "--service-port=3667",
                "problem.p",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("options parse");

        let RunCommand::Execute(EClientConfig {
            output_file,
            server,
            port,
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
            "client.out"
        );
        assert_eq!(server, "example.invalid");
        assert_eq!(port, 3667);
        assert_eq!(files, ["problem.p"]);
        assert_eq!(verbose_level(), 2);
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn defaults_insert_stdin_localhost_and_c_port() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command = process_options([PROGRAM_NAME], &mut stdout, &mut stderr).expect("options");

        let RunCommand::Execute(config) = command else {
            panic!("expected execute command");
        };
        assert_eq!(config.server, DEFAULT_SERVER);
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.files, ["-"]);
    }

    #[test]
    fn invalid_and_reserved_ports_match_c_surface() {
        let _guard = global_state_lock();
        let mut stderr = Vec::new();
        let error = parse_port(&super::OPTIONS[6], "70000", &mut stderr).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Port numbers must be between 0 and 65535");
        assert!(stderr.is_empty());

        let port = parse_port(&super::OPTIONS[6], "80", &mut stderr).expect("port accepted");
        assert_eq!(port, 80);
        assert_eq!(
            String::from_utf8(stderr).expect("stderr is utf8"),
            "e_client: Warning: Port numbers less than 1024 require root level access\n"
        );
    }

    #[test]
    fn client_protocol_sends_c_sequence_and_echoes_until_expected_replies() {
        let mut stream = Duplex::new(&["booting", "ready", "working", "result"]);
        let mut output = Vec::new();

        execute_client_protocol(&mut stream, &mut output, "cnf(c,axiom,p).\n")
            .expect("protocol succeeds");

        assert_eq!(
            sent_strings(&stream.output),
            ["hello", "add", "cnf(c,axiom,p).\n", "prove"]
        );
        assert_eq!(
            String::from_utf8(output).expect("output is utf8"),
            "% Server: booting\n% Server: ready\n% Server: working\n% Server: result\n"
        );
    }

    #[test]
    fn load_problem_files_concatenates_files_and_stdin_without_separators() {
        let file_a = temp_path("a");
        let file_b = temp_path("b");
        remove_if_present(&file_a);
        remove_if_present(&file_b);
        std::fs::write(&file_a, "left").expect("first file written");
        std::fs::write(&file_b, "right").expect("second file written");
        let mut stdin = Cursor::new(b"middle".to_vec());
        let files = vec![
            file_a.to_str().expect("utf8 path").to_owned(),
            "-".to_owned(),
            file_b.to_str().expect("utf8 path").to_owned(),
        ];

        let loaded = load_problem_files(&files, &mut stdin).expect("problem loads");
        assert_eq!(loaded, "leftmiddleright");

        remove_if_present(&file_a);
        remove_if_present(&file_b);
    }

    #[test]
    fn output_file_is_created_before_network_connection_attempt() {
        let _guard = global_state_lock();
        let output_path = temp_path("network-failure-output");
        remove_if_present(&output_path);
        let mut stdin = Cursor::new(b"cnf(c,axiom,p).\n".to_vec());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let result = run(
            [
                PROGRAM_NAME,
                "--server=127.0.0.1",
                "--port=1",
                "-o",
                output_path.to_str().expect("path is utf8"),
            ],
            &mut stdin,
            &mut stdout,
            &mut stderr,
        );

        assert!(result.is_err());
        assert!(output_path.exists());
        assert!(String::from_utf8(stdout).expect("stdout utf8").is_empty());
        remove_if_present(&output_path);
    }

    #[test]
    fn print_help_mentions_deduction_server() {
        assert!(print_help().contains("connect to the E deduction server"));
    }

    #[test]
    fn tcp_pack_helper_is_used_in_tests() {
        let mut data = Vec::new();
        tcp_string_send_to_or_error(&mut data, "ready").expect("pack via send works");
        assert_eq!(sent_strings(&data), ["ready"]);
    }
}
