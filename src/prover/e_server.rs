use std::fs::File;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::verbose::set_verbose_level;
use crate::control::batch_spec::BatchSpec;
use crate::control::esession::descriptor_from_tcp_stream;
use crate::control::sine::StructFofSpec;
use crate::heuristics::axfilter::AxFilterSet;
use crate::inout::commandline::{
    get_int_arg, print_options, CommandLineState, OptArgType, OptCell, ParsedOpt,
};
use crate::inout::initio::{exit_io, init_io};
use crate::inout::network::{
    create_server_socket, listen, tcp_msg_read_from, tcp_msg_try_read_from, tcp_string_recv_from,
    tcp_string_send_to_or_error, MsgStatus, TcpMessage,
};
use crate::inout::output::set_output_level;
use crate::inout::scanner::{IoFormat, Scanner};
use crate::prover::version::{E_NICKNAME, E_URL, STS_MAIL, VERSION};
use crate::terms::{signature::Signature, termbanks::TermBank, typebanks::TypeBank};

pub const PROGRAM_NAME: &str = "e_server";
const DEFAULT_PROVER: &str = "eprover";
const DEFAULT_PORT: u16 = 3666;
const IPPORT_RESERVED: u16 = 1024;
const C_USAGE_ERROR: &str = "Usage: e_server <domain-spec> [<options>]\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OptionCode {
    Help,
    Version,
    Verbose,
    Output,
    Filter,
    Prover,
    ServicePort,
    Silent,
    OutputLevel,
    LopParse,
    LopFormat,
    TptpParse,
    TptpFormat,
    TstpParse,
    TstpFormat,
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
        "Select an output level, greater values imply more verbose output.",
    ),
    OptCell::new(
        OptionCode::Filter,
        Some('f'),
        Some("filter"),
        OptArgType::ReqArg,
        None,
        "Specify the filter definition file. If not set, the system will uses the built-in default.",
    ),
    OptCell::new(
        OptionCode::Prover,
        Some('p'),
        Some("prover"),
        OptArgType::ReqArg,
        None,
        "Specify the prover binary to use. The default is 'eprover', and initially, only E is supported. This option does accept absolute and relative paths.",
    ),
    OptCell::new(
        OptionCode::ServicePort,
        Some('P'),
        Some("service-port"),
        OptArgType::ReqArg,
        None,
        "Specify the port to use for the deduction service.",
    ),
    OptCell::new(
        OptionCode::LopParse,
        None,
        Some("lop-in"),
        OptArgType::NoArg,
        None,
        "Parse input in E-LOP, not the default TPTP-3 format.",
    ),
    OptCell::new(
        OptionCode::LopFormat,
        None,
        Some("lop-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --lop-in.",
    ),
    OptCell::new(
        OptionCode::TptpParse,
        None,
        Some("tptp-in"),
        OptArgType::NoArg,
        None,
        "Parse TPTP-2 format instead of E-LOP (but note that includes are handled according to TPTP-3 semantics).",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TptpParse,
        None,
        Some("tptp2-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TptpFormat,
        None,
        Some("tptp2-format"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tptp-in.",
    ),
    OptCell::new(
        OptionCode::TstpParse,
        None,
        Some("tstp-in"),
        OptArgType::NoArg,
        None,
        "Parse TPTP-3 format instead of E-LOP (Note that TPTP-3 syntax is still under development, and the version in E may not be fully conforming at all times. E works on all TPTP 4.1.0 input files (including includes).",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tstp-format"),
        OptArgType::NoArg,
        None,
        "Equivalent to --tstp-in.",
    ),
    OptCell::new(
        OptionCode::TstpParse,
        None,
        Some("tptp3-in"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-in.",
    ),
    OptCell::new(
        OptionCode::TstpFormat,
        None,
        Some("tptp3-format"),
        OptArgType::NoArg,
        None,
        "Synonymous with --tstp-in.",
    ),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct EServerConfig {
    output_file: Option<PathBuf>,
    filter_file: Option<PathBuf>,
    prover: String,
    port: u16,
    parse_format: IoFormat,
    verbose_level: i64,
    output_level: i64,
    files: Vec<String>,
}

impl Default for EServerConfig {
    fn default() -> Self {
        Self {
            output_file: None,
            filter_file: None,
            prover: DEFAULT_PROVER.to_owned(),
            port: DEFAULT_PORT,
            parse_format: IoFormat::Tstp,
            verbose_level: 0,
            output_level: 1,
            files: Vec::new(),
        }
    }
}

#[derive(Debug)]
enum RunCommand {
    Execute(EServerConfig),
    Exit(u8),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LegacyServerReport {
    pub received: usize,
    pub closed: bool,
    pub read_error: bool,
}

#[derive(Debug)]
struct LegacyActiveConnection {
    stream: TcpStream,
    descriptor: u64,
    message: TcpMessage,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct LegacyServerStepReport {
    active_read: bool,
    accepted: bool,
    rejected: bool,
}

impl LegacyServerStepReport {
    #[must_use]
    const fn made_progress(self) -> bool {
        self.active_read || self.accepted || self.rejected
    }
}

pub fn run<I, S>(
    argv: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    init_io(PROGRAM_NAME);
    set_verbose_level(0);
    let result = run_inner(argv, stdout, stderr);
    exit_io();
    stdout
        .flush()
        .map_err(|error| io_diagnostic(format!("Cannot flush output: {error}")))?;
    stderr
        .flush()
        .map_err(|error| io_diagnostic(format!("Cannot flush stderr: {error}")))?;
    result
}

fn run_inner<I, S>(
    argv: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<u8, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv, stdout, stderr)? {
        RunCommand::Exit(status) => Ok(status),
        RunCommand::Execute(config) => execute_config(&config, stdout),
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
    let mut config = EServerConfig::default();

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
                writeln_diag(stdout, &format!("E {VERSION} {E_NICKNAME}"))?;
                return Ok(RunCommand::Exit(ErrorCode::NO_ERROR.exit_status()));
            }
            OptionCode::Output => {
                config.output_file = parsed.arg().map(PathBuf::from);
            }
            OptionCode::Silent => config.output_level = 0,
            OptionCode::OutputLevel => {
                config.output_level =
                    get_int_arg(parsed.option(), required_arg(&parsed, "output-level")?)?;
            }
            OptionCode::Filter => {
                config.filter_file = Some(PathBuf::from(required_arg(&parsed, "filter")?));
            }
            OptionCode::Prover => {
                required_arg(&parsed, "prover")?.clone_into(&mut config.prover);
            }
            OptionCode::ServicePort => {
                config.port = parse_port(
                    parsed.option(),
                    required_arg(&parsed, "service-port")?,
                    stderr,
                )?;
            }
            OptionCode::LopParse | OptionCode::LopFormat => {
                config.parse_format = IoFormat::Lop;
            }
            OptionCode::TptpParse | OptionCode::TptpFormat => {}
            OptionCode::TstpParse | OptionCode::TstpFormat => {
                config.parse_format = IoFormat::Tstp;
            }
        }
    }

    config.files = state.remaining_args().to_vec();
    Ok(RunCommand::Execute(config))
}

fn execute_config(config: &EServerConfig, stdout: &mut impl Write) -> Result<u8, Diagnostic> {
    apply_global_options(config);
    let mut output_file = open_output_file(config.output_file.as_deref())?;
    if config.files.is_empty() {
        return Err(Diagnostic::new(ErrorCode::USAGE_ERROR, C_USAGE_ERROR));
    }

    let _filters = load_filters(config.filter_file.as_deref())?;
    if let Some(output_file) = output_file.as_mut() {
        let (_bank, _ctrl, _parsed) = init_domain_spec(
            &config.prover,
            config.parse_format,
            &config.files,
            output_file,
        )?;
        output_file
            .flush()
            .map_err(|error| io_diagnostic(format!("Cannot flush output file: {error}")))?;
    } else {
        let (_bank, _ctrl, _parsed) =
            init_domain_spec(&config.prover, config.parse_format, &config.files, stdout)?;
    }

    serve_legacy_server(config.port, stdout)?;
    Ok(ErrorCode::NO_ERROR.exit_status())
}

pub fn process_legacy_connection<S, W>(
    stream: &mut S,
    output: &mut W,
) -> Result<LegacyServerReport, Diagnostic>
where
    S: Read + Write,
    W: Write,
{
    let mut report = LegacyServerReport::default();
    loop {
        let (message, status) = tcp_string_recv_from(stream, false)?;
        let Some(message) = message else {
            match status {
                MsgStatus::Error => {
                    writeln_diag(output, "Read error")?;
                    report.read_error = true;
                }
                MsgStatus::ConnClosed => {
                    writeln_diag(output, "Connection closed")?;
                    report.closed = true;
                }
                MsgStatus::Incomplete | MsgStatus::Success => {}
            }
            return Ok(report);
        };

        writeln_diag(output, &format!("Received: {message}"))?;
        tcp_string_send_to_or_error(stream, "wait")?;
        tcp_string_send_to_or_error(stream, "ready")?;
        report.received += 1;
    }
}

fn serve_legacy_server(port: u16, stdout: &mut impl Write) -> Result<(), Diagnostic> {
    let listener = create_server_socket(port)?;
    listen(&listener)?;
    listener.set_nonblocking(true).map_err(|error| {
        Diagnostic::new(
            ErrorCode::SYSTEM_ERROR,
            format!("Cannot set server socket nonblocking: {error}"),
        )
    })?;
    let mut active = None;
    let mut printed_loop_marker = false;
    loop {
        if !printed_loop_marker {
            writeln_diag(stdout, "Main loop")?;
            stdout.flush().map_err(|error| {
                io_diagnostic(format!("Cannot flush legacy server output: {error}"))
            })?;
            printed_loop_marker = true;
        }
        if poll_legacy_server_once(&listener, &mut active, stdout)?.made_progress() {
            printed_loop_marker = false;
        } else {
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn poll_legacy_server_once(
    listener: &TcpListener,
    active: &mut Option<LegacyActiveConnection>,
    output: &mut impl Write,
) -> Result<LegacyServerStepReport, Diagnostic> {
    let mut report = LegacyServerStepReport::default();
    if process_active_connection_once(active, output)? {
        report.active_read = true;
    }
    match listener.accept() {
        Ok((stream, _addr)) => {
            if active.is_none() {
                *active = Some(accept_legacy_connection(stream, output)?);
                report.accepted = true;
            } else {
                drop(stream);
                report.rejected = true;
            }
        }
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
        Err(error) => {
            return Err(Diagnostic::new(
                ErrorCode::SYSTEM_ERROR,
                format!("Failure to accept connection: {error}"),
            ));
        }
    }
    Ok(report)
}

fn accept_legacy_connection(
    stream: TcpStream,
    output: &mut impl Write,
) -> Result<LegacyActiveConnection, Diagnostic> {
    let descriptor = descriptor_from_tcp_stream(&stream)?.value();
    stream.set_nonblocking(true).map_err(|error| {
        Diagnostic::new(
            ErrorCode::SYSTEM_ERROR,
            format!("Cannot set accepted socket nonblocking: {error}"),
        )
    })?;
    writeln_diag(output, &format!("Accepted {descriptor}"))?;
    Ok(LegacyActiveConnection {
        stream,
        descriptor,
        message: TcpMessage::new(),
    })
}

fn process_active_connection_once(
    active: &mut Option<LegacyActiveConnection>,
    output: &mut impl Write,
) -> Result<bool, Diagnostic> {
    let Some(connection) = active.as_mut() else {
        return Ok(false);
    };
    let before = connection.message.transmission_count();
    let mut status = tcp_msg_try_read_from(&mut connection.stream, &mut connection.message);
    if status == MsgStatus::Incomplete && connection.message.transmission_count() > before {
        status = finish_active_message_read(connection)?;
    }
    match status {
        MsgStatus::Success => {
            let message = std::mem::take(&mut connection.message).unpack_string_lossy();
            writeln_diag(output, &format!("Received: {message}"))?;
            send_legacy_responses(connection)?;
            Ok(true)
        }
        MsgStatus::Error => {
            writeln_diag(output, "Read error")?;
            *active = None;
            Ok(true)
        }
        MsgStatus::ConnClosed => {
            writeln_diag(output, "Connection closed")?;
            *active = None;
            Ok(true)
        }
        MsgStatus::Incomplete => Ok(false),
    }
}

fn finish_active_message_read(
    connection: &mut LegacyActiveConnection,
) -> Result<MsgStatus, Diagnostic> {
    connection.stream.set_nonblocking(false).map_err(|error| {
        Diagnostic::new(
            ErrorCode::SYSTEM_ERROR,
            format!(
                "Cannot set accepted socket {} blocking: {error}",
                connection.descriptor
            ),
        )
    })?;
    let status = loop {
        match tcp_msg_read_from(&mut connection.stream, &mut connection.message) {
            MsgStatus::Incomplete => {}
            status => break status,
        }
    };
    connection.stream.set_nonblocking(true).map_err(|error| {
        Diagnostic::new(
            ErrorCode::SYSTEM_ERROR,
            format!(
                "Cannot restore accepted socket {} nonblocking: {error}",
                connection.descriptor
            ),
        )
    })?;
    Ok(status)
}

fn send_legacy_responses(connection: &mut LegacyActiveConnection) -> Result<(), Diagnostic> {
    connection.stream.set_nonblocking(false).map_err(|error| {
        Diagnostic::new(
            ErrorCode::SYSTEM_ERROR,
            format!(
                "Cannot set accepted socket {} blocking: {error}",
                connection.descriptor
            ),
        )
    })?;
    let send_result = (|| {
        tcp_string_send_to_or_error(&mut connection.stream, "wait")?;
        tcp_string_send_to_or_error(&mut connection.stream, "ready")
    })();
    let restore_result = connection.stream.set_nonblocking(true).map_err(|error| {
        Diagnostic::new(
            ErrorCode::SYSTEM_ERROR,
            format!(
                "Cannot restore accepted socket {} nonblocking: {error}",
                connection.descriptor
            ),
        )
    });
    send_result?;
    restore_result
}

fn load_filters(filter_file: Option<&Path>) -> Result<AxFilterSet, Diagnostic> {
    let Some(path) = filter_file else {
        return AxFilterSet::default_set();
    };
    let mut scanner = Scanner::from_file(path, true)?;
    let mut filters = AxFilterSet::new();
    filters.parse(&mut scanner)?;
    Ok(filters)
}

fn init_domain_spec<W: Write + ?Sized>(
    prover: &str,
    parse_format: IoFormat,
    files: &[String],
    output: &mut W,
) -> Result<(TermBank, StructFofSpec, i64), Diagnostic> {
    let mut spec = BatchSpec::new(prover, parse_format);
    spec.includes = files.to_vec();
    let mut bank = new_term_bank()?;
    let mut ctrl = StructFofSpec::new(bank.signature());
    let parsed = spec.init_struct_fof_spec_from_files(&mut bank, &mut ctrl, None, output)?;
    ctrl.reset_shared();
    Ok((bank, ctrl, parsed))
}

fn new_term_bank() -> Result<TermBank, Diagnostic> {
    let mut signature = Signature::new(TypeBank::new());
    signature.insert_internal_codes()?;
    TermBank::new(signature)
}

fn apply_global_options(config: &EServerConfig) {
    set_verbose_level(i64_to_i32_saturating(config.verbose_level));
    let _old_output_level = set_output_level(config.output_level);
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

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\n\
E {VERSION} \"{E_NICKNAME}\"\n\
\n\
Usage: {PROGRAM_NAME} [options] [files]\n\
\n\
Read an problem specification and offer deduction in the the structure\n\
described by the specification as a service.  All input formats (LOP,\n\
TPTP-2 and TPTP-3 are supported for the original specification, \n\
however, only TPTP-3 is used for the service. TPTP-3 is also the \n\
default format. Important options allow specificatio of the filters\n\
to use for proof attemtps, the dervice port, and the binary of the\n\
prover to use.\n\
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

fn open_output_file(path: Option<&Path>) -> Result<Option<File>, Diagnostic> {
    let Some(path) = path else {
        return Ok(None);
    };
    if path == Path::new("-") {
        return Ok(None);
    }
    File::create(path).map(Some).map_err(|error| {
        e_server_sys_error_diagnostic(format!("Cannot open file {}", path.display()), &error)
    })
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

fn e_server_sys_error_diagnostic(prefix: impl Into<String>, error: &io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("{}\n{PROGRAM_NAME}: {error}", prefix.into()),
    )
}

fn i64_to_i32_saturating(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::path::{Path, PathBuf};

    use super::{
        init_domain_spec, load_filters, open_output_file, parse_port, poll_legacy_server_once,
        print_help, process_legacy_connection, process_options, run, EServerConfig,
        LegacyServerReport, RunCommand, C_USAGE_ERROR, DEFAULT_PORT, DEFAULT_PROVER, PROGRAM_NAME,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::verbose_level;
    use crate::inout::network::{
        tcp_string_recv_from_or_error, tcp_string_send_to_or_error, TcpMessage,
    };
    use crate::inout::output::output_level;
    use crate::inout::scanner::IoFormat;
    use crate::prover::version::{E_NICKNAME, VERSION};
    use crate::test_support::global_state_lock;

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
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl Write for Duplex {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .expect("current directory is available")
            .join("target")
            .join(format!("e-server-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn remove_dir_if_present(path: &Path) {
        _ = std::fs::remove_dir(path);
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

    fn loopback_listener() -> (TcpListener, SocketAddr) {
        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("loopback listener binds");
        listener
            .set_nonblocking(true)
            .expect("listener can be nonblocking");
        let address = listener.local_addr().expect("listener has address");
        (listener, address)
    }

    #[allow(clippy::too_many_lines)]
    fn expected_help() -> String {
        format!(
            concat!(
                "\n",
                "E {version} \"{nickname}\"\n",
                "\n",
                "Usage: e_server [options] [files]\n",
                "\n",
                "Read an problem specification and offer deduction in the the structure\n",
                "described by the specification as a service.  All input formats (LOP,\n",
                "TPTP-2 and TPTP-3 are supported for the original specification, \n",
                "however, only TPTP-3 is used for the service. TPTP-3 is also the \n",
                "default format. Important options allow specificatio of the filters\n",
                "to use for proof attemtps, the dervice port, and the binary of the\n",
                "prover to use.\n",
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
                "   -o <arg>\n",
                "  --output-file=<arg>\n",
                "    Redirect output into the named file (this affects only some output, as\n",
                "    most is written to automatically generated files based on the input and\n",
                "    filter names.\n",
                "\n",
                "   -s\n",
                "  --silent\n",
                "    Equivalent to --output-level=0.\n",
                "\n",
                "   -l <arg>\n",
                "  --output-level=<arg>\n",
                "    Select an output level, greater values imply more verbose output.\n",
                "\n",
                "   -f <arg>\n",
                "  --filter=<arg>\n",
                "    Specify the filter definition file. If not set, the system will uses the\n",
                "    built-in default.\n",
                "\n",
                "   -p <arg>\n",
                "  --prover=<arg>\n",
                "    Specify the prover binary to use. The default is 'eprover', and\n",
                "    initially, only E is supported. This option does accept absolute and\n",
                "    relative paths.\n",
                "\n",
                "   -P <arg>\n",
                "  --service-port=<arg>\n",
                "    Specify the port to use for the deduction service.\n",
                "\n",
                "  --lop-in\n",
                "    Parse input in E-LOP, not the default TPTP-3 format.\n",
                "\n",
                "  --lop-format\n",
                "    Equivalent to --lop-in.\n",
                "\n",
                "  --tptp-in\n",
                "    Parse TPTP-2 format instead of E-LOP (but note that includes are handled\n",
                "    according to TPTP-3 semantics).\n",
                "\n",
                "  --tptp-format\n",
                "    Equivalent to --tptp-in.\n",
                "\n",
                "  --tptp2-in\n",
                "    Synonymous with --tptp-in.\n",
                "\n",
                "  --tptp2-format\n",
                "    Synonymous with --tptp-in.\n",
                "\n",
                "  --tstp-in\n",
                "    Parse TPTP-3 format instead of E-LOP (Note that TPTP-3 syntax is still\n",
                "    under development, and the version in E may not be fully conforming at\n",
                "    all times. E works on all TPTP 4.1.0 input files (including includes).\n",
                "\n",
                "  --tstp-format\n",
                "    Equivalent to --tstp-in.\n",
                "\n",
                "  --tptp3-in\n",
                "    Synonymous with --tstp-in.\n",
                "\n",
                "  --tptp3-format\n",
                "    Synonymous with --tstp-in.\n",
                "\n",
                "\n",
                "Copyright (C) 2011 by Stephan Schulz, schulz@eprover.org\n",
                "\n",
                "You can find the latest version of E and additional information at\n",
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
            nickname = E_NICKNAME,
        )
    }

    #[test]
    fn help_and_version_preserve_c_text() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let help_status = run([PROGRAM_NAME, "--help"], &mut stdout, &mut stderr).expect("help");
        assert_eq!(help_status, ErrorCode::NO_ERROR.exit_status());
        let help = String::from_utf8(stdout).expect("help is utf8");
        assert_eq!(help, expected_help());
        assert!(stderr.is_empty());

        let mut stdout = Vec::new();
        let version_status = run([PROGRAM_NAME, "-V"], &mut stdout, &mut stderr).expect("version");
        assert_eq!(version_status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).expect("version utf8"),
            format!("E {VERSION} {E_NICKNAME}\n")
        );
    }

    #[test]
    fn process_options_records_server_options_and_format_quirks() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command = process_options(
            [
                PROGRAM_NAME,
                "--verbose=2",
                "--output-file=server.out",
                "--filter=filters.axf",
                "--prover=custom-e",
                "--service-port=3667",
                "--lop-in",
                "--tptp-in",
                "--output-level=3",
                "domain.p",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("options");

        let RunCommand::Execute(config) = command else {
            panic!("expected execute command");
        };
        assert_eq!(
            config,
            EServerConfig {
                output_file: Some(PathBuf::from("server.out")),
                filter_file: Some(PathBuf::from("filters.axf")),
                prover: "custom-e".to_owned(),
                port: 3667,
                parse_format: IoFormat::Lop,
                verbose_level: 2,
                output_level: 3,
                files: vec!["domain.p".to_owned()],
            }
        );
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn defaults_match_c_globals() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let command =
            process_options([PROGRAM_NAME, "domain.p"], &mut stdout, &mut stderr).expect("options");
        let RunCommand::Execute(config) = command else {
            panic!("expected execute command");
        };

        assert_eq!(config.prover, DEFAULT_PROVER);
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.parse_format, IoFormat::Tstp);
        assert_eq!(config.output_level, 1);
        assert_eq!(config.files, ["domain.p"]);
    }

    #[test]
    fn invalid_and_reserved_ports_match_c_surface() {
        let mut stderr = Vec::new();
        let error = parse_port(&super::OPTIONS[8], "70000", &mut stderr).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Port numbers must be between 0 and 65535");
        assert!(stderr.is_empty());

        let port = parse_port(&super::OPTIONS[8], "80", &mut stderr).expect("port accepted");
        assert_eq!(port, 80);
        assert_eq!(
            String::from_utf8(stderr).expect("stderr is utf8"),
            "e_server: Warning: Port numbers less than 1024 require root level access\n"
        );
    }

    #[test]
    fn missing_domain_spec_opens_output_file_before_usage_error() {
        let _guard = global_state_lock();
        let output_path = temp_path("usage-output");
        remove_if_present(&output_path);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            [
                PROGRAM_NAME,
                "-o",
                output_path.to_str().expect("test path is utf8"),
            ],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), C_USAGE_ERROR);
        assert!(output_path.exists());
        assert!(stdout.is_empty());
        remove_if_present(&output_path);
    }

    #[test]
    fn output_dash_uses_stdout_route_like_c() {
        let _guard = global_state_lock();

        assert!(open_output_file(Some(Path::new("-")))
            .expect("- output opens")
            .is_none());
    }

    #[test]
    fn output_file_open_failure_uses_c_syserror_shape() {
        let _guard = global_state_lock();
        let output_path = temp_path("output-dir");
        remove_if_present(&output_path);
        remove_dir_if_present(&output_path);
        std::fs::create_dir(&output_path).expect("output fixture directory is created");

        let error =
            open_output_file(Some(&output_path)).expect_err("directory output path is reported");

        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error
            .message()
            .starts_with(&format!("Cannot open file {}", output_path.display())));
        assert!(error.message().contains(&format!("\n{PROGRAM_NAME}: ")));

        remove_dir_if_present(&output_path);
    }

    #[test]
    fn run_applies_verbose_and_output_globals_before_usage_error() {
        let _guard = global_state_lock();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let _error = run(
            [PROGRAM_NAME, "--verbose=4", "--output-level=5"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(verbose_level(), 4);
        assert_eq!(output_level(), 5);
    }

    #[test]
    fn custom_filter_file_is_parsed() {
        let filter_path = temp_path("filters");
        remove_if_present(&filter_path);
        std::fs::write(&filter_path, "tiny=Threshold(3)\n").expect("filter written");

        let filters = load_filters(Some(&filter_path)).expect("filters parse");

        assert_eq!(filters.elements(), 1);
        assert!(filters.find_filter("tiny").is_some());
        remove_if_present(&filter_path);
    }

    #[test]
    fn domain_spec_parses_files_and_resets_shared_boundary() {
        let domain_path = temp_path("domain");
        remove_if_present(&domain_path);
        std::fs::write(&domain_path, "fof(a, axiom, p(a)).\n").expect("domain written");
        let mut output = Vec::new();

        let (_bank, ctrl, parsed) = init_domain_spec(
            DEFAULT_PROVER,
            IoFormat::Tstp,
            &[domain_path.to_string_lossy().into_owned()],
            &mut output,
        )
        .expect("domain parses");

        assert_eq!(parsed, 1);
        assert_eq!(ctrl.formula_set_count(), 1);
        assert_eq!(ctrl.clause_set_count(), 1);
        assert_eq!(ctrl.shared_ax_sp(), 0);
        assert!(String::from_utf8(output)
            .expect("parse output utf8")
            .contains("% Parsing "));
        remove_if_present(&domain_path);
    }

    #[test]
    fn legacy_connection_replies_wait_ready_for_each_message_until_close() {
        let mut stream = Duplex::new(&["hello", "add"]);
        let mut output = Vec::new();

        let report = process_legacy_connection(&mut stream, &mut output).expect("session");

        assert_eq!(
            report,
            LegacyServerReport {
                received: 2,
                closed: true,
                read_error: false,
            }
        );
        assert_eq!(
            sent_strings(&stream.output),
            ["wait", "ready", "wait", "ready"]
        );
        assert_eq!(
            String::from_utf8(output).expect("output utf8"),
            "Received: hello\nReceived: add\nConnection closed\n"
        );
    }

    #[test]
    fn legacy_poll_keeps_one_active_connection_and_rejects_second_client() {
        let (listener, address) = loopback_listener();
        let _first_client = TcpStream::connect(address).expect("first client connects");
        let mut active = None;
        let mut output = Vec::new();

        let accepted =
            poll_legacy_server_once(&listener, &mut active, &mut output).expect("accept first");

        assert!(accepted.accepted);
        assert!(!accepted.rejected);
        assert!(active.is_some());

        let _second_client = TcpStream::connect(address).expect("second client connects");
        let rejected =
            poll_legacy_server_once(&listener, &mut active, &mut output).expect("reject second");

        assert!(!rejected.accepted);
        assert!(rejected.rejected);
        assert!(active.is_some());
        let output = String::from_utf8(output).expect("output utf8");
        assert_eq!(output.matches("Accepted ").count(), 1);
    }

    #[test]
    fn legacy_poll_processes_active_message_before_rejecting_pending_client() {
        let (listener, address) = loopback_listener();
        let mut first_client = TcpStream::connect(address).expect("first client connects");
        let mut active = None;
        let mut output = Vec::new();

        poll_legacy_server_once(&listener, &mut active, &mut output).expect("accept first");
        tcp_string_send_to_or_error(&mut first_client, "hello").expect("client sends message");
        let _second_client = TcpStream::connect(address).expect("second client connects");

        let report = poll_legacy_server_once(&listener, &mut active, &mut output)
            .expect("read active and reject second");

        assert!(report.active_read);
        assert!(report.rejected);
        assert_eq!(
            tcp_string_recv_from_or_error(&mut first_client).expect("wait response"),
            "wait"
        );
        assert_eq!(
            tcp_string_recv_from_or_error(&mut first_client).expect("ready response"),
            "ready"
        );
        assert_eq!(
            String::from_utf8(output)
                .expect("output utf8")
                .lines()
                .last(),
            Some("Received: hello")
        );
    }

    #[test]
    fn print_help_preserves_full_c_text() {
        assert_eq!(print_help(), expected_help());
    }
}
