use crate::basics::error::{Diagnostic, ErrorCode};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};

pub const TCP_BACKLOG: usize = 10;
pub const TCP_BUF_SIZE: usize = 1025;
pub const TCP_HEADER_SIZE: usize = size_of::<u32>();

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MsgStatus {
    #[default]
    Incomplete,
    Error,
    ConnClosed,
    Success,
}

impl MsgStatus {
    #[must_use]
    pub const fn c_value(self) -> u8 {
        match self {
            Self::Incomplete => 0,
            Self::Error => 1,
            Self::ConnClosed => 2,
            Self::Success => 3,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TcpMessage {
    content: Vec<u8>,
    len: Option<usize>,
    transmission_count: usize,
    len_buf: [u8; TCP_HEADER_SIZE],
}

impl Default for TcpMessage {
    fn default() -> Self {
        Self::new()
    }
}

impl TcpMessage {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            content: Vec::new(),
            len: None,
            transmission_count: 0,
            len_buf: [0; TCP_HEADER_SIZE],
        }
    }

    pub fn pack(text: &str) -> Result<Self, Diagnostic> {
        Self::pack_payload(c_string_prefix(text.as_bytes()))
    }

    pub fn pack_payload(payload: &[u8]) -> Result<Self, Diagnostic> {
        let Some(total_len) = payload.len().checked_add(TCP_HEADER_SIZE) else {
            return Err(network_error("TCP message length overflow"));
        };
        let Ok(total_len_u32) = u32::try_from(total_len) else {
            return Err(network_error("TCP message length exceeds 32-bit header"));
        };

        let mut content = Vec::with_capacity(total_len);
        content.extend_from_slice(&total_len_u32.to_be_bytes());
        content.extend_from_slice(payload);

        Ok(Self {
            content,
            len: Some(total_len),
            transmission_count: 0,
            len_buf: [0; TCP_HEADER_SIZE],
        })
    }

    #[must_use]
    pub fn message_len(&self) -> Option<usize> {
        self.len
    }

    #[must_use]
    pub fn c_len(&self) -> isize {
        self.len.map_or(-1, |len| match isize::try_from(len) {
            Ok(value) => value,
            Err(_) => isize::MAX,
        })
    }

    #[must_use]
    pub const fn transmission_count(&self) -> usize {
        self.transmission_count
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.len == Some(self.transmission_count)
    }

    #[must_use]
    pub fn content_bytes(&self) -> &[u8] {
        &self.content
    }

    #[must_use]
    pub fn raw_payload_bytes(&self) -> &[u8] {
        if self.content.len() < TCP_HEADER_SIZE {
            &[]
        } else {
            &self.content[TCP_HEADER_SIZE..]
        }
    }

    #[must_use]
    pub fn unpack(self) -> Vec<u8> {
        c_string_prefix(self.raw_payload_bytes()).to_vec()
    }

    #[must_use]
    pub fn unpack_string_lossy(self) -> String {
        String::from_utf8_lossy(&self.unpack()).into_owned()
    }
}

fn c_string_prefix(bytes: &[u8]) -> &[u8] {
    let nul_pos = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    &bytes[..nul_pos]
}

fn network_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::SYSTEM_ERROR, message)
}

fn append_header(message: &mut TcpMessage) -> MsgStatus {
    let encoded = u32::from_be_bytes(message.len_buf);
    let Ok(len) = usize::try_from(encoded) else {
        return MsgStatus::Error;
    };
    if len < TCP_HEADER_SIZE {
        return MsgStatus::Error;
    }
    message.len = Some(len);
    message.content.extend_from_slice(&message.len_buf);
    MsgStatus::Success
}

pub fn tcp_msg_write_to(writer: &mut impl Write, message: &mut TcpMessage) -> MsgStatus {
    let Some(len) = message.len else {
        return MsgStatus::Error;
    };
    if message.transmission_count > len || message.content.len() < len {
        return MsgStatus::Error;
    }

    let remaining = len - message.transmission_count;
    match writer.write(&message.content[message.transmission_count..][..remaining]) {
        Ok(written) => {
            message.transmission_count += written;
            if message.is_complete() {
                MsgStatus::Success
            } else {
                MsgStatus::Incomplete
            }
        }
        Err(_) => MsgStatus::Error,
    }
}

pub fn tcp_msg_read_from(reader: &mut impl Read, message: &mut TcpMessage) -> MsgStatus {
    if message.transmission_count < TCP_HEADER_SIZE {
        let target = &mut message.len_buf[message.transmission_count..TCP_HEADER_SIZE];
        match reader.read(target) {
            Ok(0) => return MsgStatus::ConnClosed,
            Ok(read) => {
                message.transmission_count += read;
                if message.transmission_count < TCP_HEADER_SIZE {
                    return MsgStatus::Incomplete;
                }
                if append_header(message) == MsgStatus::Error {
                    return MsgStatus::Error;
                }
            }
            Err(_) => return MsgStatus::Error,
        }
    }

    let Some(len) = message.len else {
        return MsgStatus::Error;
    };
    if message.transmission_count > len {
        return MsgStatus::Error;
    }

    let remaining = len - message.transmission_count;
    let chunk_len = remaining.min(TCP_BUF_SIZE - 1);
    let mut buffer = [0; TCP_BUF_SIZE];
    match reader.read(&mut buffer[..chunk_len]) {
        Ok(0) => MsgStatus::ConnClosed,
        Ok(read) => {
            message
                .content
                .extend_from_slice(c_string_prefix(&buffer[..read]));
            message.transmission_count += read;
            if message.is_complete() {
                MsgStatus::Success
            } else {
                MsgStatus::Incomplete
            }
        }
        Err(_) => MsgStatus::Error,
    }
}

pub fn tcp_msg_send_to(writer: &mut impl Write, message: &mut TcpMessage) -> MsgStatus {
    loop {
        let before = message.transmission_count;
        let status = tcp_msg_write_to(writer, message);
        match status {
            MsgStatus::Incomplete => {
                if message.transmission_count == before {
                    return MsgStatus::Incomplete;
                }
            }
            MsgStatus::Success | MsgStatus::Error | MsgStatus::ConnClosed => return status,
        }
    }
}

pub fn tcp_msg_recv_from(reader: &mut impl Read) -> (TcpMessage, MsgStatus) {
    let mut message = TcpMessage::new();
    loop {
        let status = tcp_msg_read_from(reader, &mut message);
        match status {
            MsgStatus::Success | MsgStatus::Error | MsgStatus::ConnClosed => {
                return (message, status);
            }
            MsgStatus::Incomplete => {}
        }
    }
}

pub fn tcp_string_send_to(
    writer: &mut impl Write,
    text: &str,
    fail_on_error: bool,
) -> Result<MsgStatus, Diagnostic> {
    let mut message = TcpMessage::pack(text)?;
    let status = tcp_msg_send_to(writer, &mut message);
    if fail_on_error && status != MsgStatus::Success {
        Err(network_error("Could not send string message"))
    } else {
        Ok(status)
    }
}

pub fn tcp_string_send_to_or_error(writer: &mut impl Write, text: &str) -> Result<(), Diagnostic> {
    tcp_string_send_to(writer, text, true).map(|_| ())
}

pub fn tcp_string_recv_from(
    reader: &mut impl Read,
    fail_on_error: bool,
) -> Result<(Option<String>, MsgStatus), Diagnostic> {
    let (message, status) = tcp_msg_recv_from(reader);
    if status == MsgStatus::Success {
        Ok((Some(message.unpack_string_lossy()), status))
    } else if fail_on_error {
        Err(network_error("Could not receive string message"))
    } else {
        Ok((None, status))
    }
}

pub fn tcp_string_recv_from_or_error(reader: &mut impl Read) -> Result<String, Diagnostic> {
    let (message, _) = tcp_string_recv_from(reader, true)?;
    message.ok_or_else(|| network_error("Could not receive string message"))
}

#[must_use]
pub fn create_server_socket_no_fail(port: u16) -> Option<TcpListener> {
    platform_server_socket::create_server_socket_no_fail(port)
}

pub fn create_server_socket(port: u16) -> Result<TcpListener, Diagnostic> {
    create_server_socket_no_fail(port)
        .ok_or_else(|| network_error(format!("Cannot create socket for port {port}")))
}

pub fn listen(_listener: &TcpListener) -> Result<(), Diagnostic> {
    Ok(())
}

pub fn create_client_socket_no_fail(host: &str, port: u16) -> Result<TcpStream, Diagnostic> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| network_error(format!("Could not resolve address ({error})")))?;

    connect_client_like_c(addresses, TcpStream::connect).map_err(|last_error| {
        network_error(match last_error {
            Some(error) => format!("Could not create connected socket: {error}"),
            None => "Could not resolve address".to_owned(),
        })
    })
}

pub fn create_client_socket(host: &str, port: u16) -> Result<TcpStream, Diagnostic> {
    create_client_socket_no_fail(host, port)
}

fn connect_client_like_c<I, F, T, E>(addresses: I, mut connect: F) -> Result<T, Option<E>>
where
    I: IntoIterator<Item = SocketAddr>,
    F: FnMut(SocketAddr) -> Result<T, E>,
{
    let mut result = None;
    for address in addresses {
        result = Some(connect(address));
    }

    match result {
        Some(Ok(stream)) => Ok(stream),
        Some(Err(error)) => Err(Some(error)),
        None => Err(None),
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod platform_server_socket {
    use super::TCP_BACKLOG;
    use std::ffi::c_void;
    use std::mem::size_of_val;
    use std::net::TcpListener;
    use std::os::fd::FromRawFd;
    use std::os::raw::c_int;

    const AF_INET: c_int = 2;
    const SOCK_STREAM: c_int = 1;
    const IPPROTO_TCP: c_int = 6;
    const SOL_SOCKET: c_int = 1;
    const SO_REUSEADDR: c_int = 2;
    const SOCKADDR_IN_LEN: SockLen = 16;

    type SockLen = u32;

    #[repr(C)]
    struct InAddr {
        s_addr: u32,
    }

    #[repr(C)]
    struct SockAddr {
        family: u16,
        data: [u8; 14],
    }

    #[repr(C)]
    struct SockAddrIn {
        family: u16,
        port: u16,
        address: InAddr,
        zero: [u8; 8],
    }

    extern "C" {
        fn socket(domain: c_int, socket_type: c_int, protocol: c_int) -> c_int;
        fn setsockopt(
            socket: c_int,
            level: c_int,
            option_name: c_int,
            option_value: *const c_void,
            option_len: SockLen,
        ) -> c_int;
        fn bind(socket: c_int, address: *const SockAddr, address_len: SockLen) -> c_int;
        fn listen(socket: c_int, backlog: c_int) -> c_int;
        fn close(fd: c_int) -> c_int;
    }

    pub(super) fn create_server_socket_no_fail(port: u16) -> Option<TcpListener> {
        // SAFETY: socket is called with C constants matching the AF_INET TCP
        // stream socket shape used by cio_network.c. On success, the returned
        // fd is either closed on error paths or transferred to TcpListener.
        let fd = unsafe { socket(AF_INET, SOCK_STREAM, IPPROTO_TCP) };
        if fd == -1 {
            return None;
        }

        if set_reuse_addr(fd).is_none() || bind_any(fd, port).is_none() || listen_fd(fd).is_none() {
            close_fd(fd);
            return None;
        }

        // SAFETY: fd is a live listening TCP socket created by socket, bound
        // and switched to listening mode above. Ownership moves into
        // TcpListener, so this module must not close fd after this point.
        Some(unsafe { TcpListener::from_raw_fd(fd) })
    }

    fn set_reuse_addr(fd: c_int) -> Option<()> {
        let yes: c_int = 1;
        let option_len = SockLen::try_from(size_of_val(&yes)).ok()?;
        // SAFETY: &yes points to a valid c_int option value for the duration
        // of the call, and fd is owned by this module until success wrapping.
        if unsafe {
            setsockopt(
                fd,
                SOL_SOCKET,
                SO_REUSEADDR,
                (&raw const yes).cast::<c_void>(),
                option_len,
            )
        } == -1
        {
            None
        } else {
            Some(())
        }
    }

    fn bind_any(fd: c_int, port: u16) -> Option<()> {
        let address = SockAddrIn {
            family: u16::try_from(AF_INET).ok()?,
            port: port.to_be(),
            address: InAddr { s_addr: 0 },
            zero: [0; 8],
        };
        // SAFETY: address is a properly initialized sockaddr_in with the C ABI
        // layout used by bind for AF_INET. fd is a live socket owned here.
        if unsafe { bind(fd, (&raw const address).cast::<SockAddr>(), SOCKADDR_IN_LEN) } == -1 {
            None
        } else {
            Some(())
        }
    }

    fn listen_fd(fd: c_int) -> Option<()> {
        let backlog = c_int::try_from(TCP_BACKLOG).ok()?;
        // SAFETY: fd is a bound TCP socket owned by this module, and backlog
        // is the C TCP_BACKLOG value represented as c_int.
        if unsafe { listen(fd, backlog) } == -1 {
            None
        } else {
            Some(())
        }
    }

    fn close_fd(fd: c_int) {
        // SAFETY: fd is still owned by this module on all call sites and has
        // not been transferred to TcpListener.
        let _ = unsafe { close(fd) };
    }
}

#[cfg(not(target_os = "linux"))]
mod platform_server_socket {
    use std::net::{Ipv4Addr, TcpListener};

    pub(super) fn create_server_socket_no_fail(port: u16) -> Option<TcpListener> {
        TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        connect_client_like_c, create_server_socket, tcp_msg_read_from, tcp_msg_recv_from,
        tcp_msg_send_to, tcp_msg_write_to, tcp_string_recv_from, tcp_string_send_to, MsgStatus,
        TcpMessage, TCP_HEADER_SIZE,
    };
    use std::io::{self, Cursor, Read, Write};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    struct LimitedReader {
        inner: Cursor<Vec<u8>>,
        max_read: usize,
    }

    impl LimitedReader {
        fn new(bytes: Vec<u8>, max_read: usize) -> Self {
            Self {
                inner: Cursor::new(bytes),
                max_read,
            }
        }
    }

    impl Read for LimitedReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let limit = buffer.len().min(self.max_read);
            self.inner.read(&mut buffer[..limit])
        }
    }

    struct LimitedWriter {
        bytes: Vec<u8>,
        max_write: usize,
    }

    impl LimitedWriter {
        fn new(max_write: usize) -> Self {
            Self {
                bytes: Vec::new(),
                max_write,
            }
        }
    }

    impl Write for LimitedWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            let write_len = buffer.len().min(self.max_write);
            self.bytes.extend_from_slice(&buffer[..write_len]);
            Ok(write_len)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn packed_bytes(text: &str) -> Vec<u8> {
        TcpMessage::pack(text).unwrap().content_bytes().to_vec()
    }

    fn loopback_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    #[test]
    fn message_status_values_match_c_enum() {
        assert_eq!(MsgStatus::Incomplete.c_value(), 0);
        assert_eq!(MsgStatus::Error.c_value(), 1);
        assert_eq!(MsgStatus::ConnClosed.c_value(), 2);
        assert_eq!(MsgStatus::Success.c_value(), 3);
    }

    #[test]
    fn new_message_matches_c_allocated_shape() {
        let message = TcpMessage::new();
        assert_eq!(message.c_len(), -1);
        assert_eq!(message.message_len(), None);
        assert_eq!(message.transmission_count(), 0);
        assert!(!message.is_complete());
        assert!(message.content_bytes().is_empty());
    }

    #[test]
    fn pack_uses_total_message_length_in_network_order() {
        let message = TcpMessage::pack("abc").unwrap();
        assert_eq!(message.message_len(), Some(TCP_HEADER_SIZE + 3));
        assert_eq!(message.transmission_count(), 0);
        assert_eq!(message.content_bytes(), &[0, 0, 0, 7, b'a', b'b', b'c']);
    }

    #[test]
    fn pack_and_unpack_use_c_string_prefixes() {
        let message = TcpMessage::pack("ab\0cd").unwrap();
        assert_eq!(message.content_bytes(), &[0, 0, 0, 6, b'a', b'b']);

        let mut raw_message = TcpMessage::pack_payload(b"a\0b").unwrap();
        raw_message.transmission_count = raw_message.message_len().unwrap_or_default();
        assert_eq!(raw_message.unpack(), b"a");
    }

    #[test]
    fn write_advances_one_write_call_at_a_time() {
        let mut message = TcpMessage::pack("abcdef").unwrap();
        let mut writer = LimitedWriter::new(5);

        assert_eq!(
            tcp_msg_write_to(&mut writer, &mut message),
            MsgStatus::Incomplete
        );
        assert_eq!(message.transmission_count(), 5);
        assert_eq!(
            tcp_msg_write_to(&mut writer, &mut message),
            MsgStatus::Success
        );
        assert_eq!(writer.bytes, packed_bytes("abcdef"));
    }

    #[test]
    fn send_loops_until_message_is_written() {
        let mut message = TcpMessage::pack("abcdef").unwrap();
        let mut writer = LimitedWriter::new(2);

        assert_eq!(
            tcp_msg_send_to(&mut writer, &mut message),
            MsgStatus::Success
        );
        assert_eq!(writer.bytes, packed_bytes("abcdef"));
    }

    #[test]
    fn read_accumulates_partial_header_and_payload() {
        let mut reader = LimitedReader::new(packed_bytes("hello"), 2);
        let mut message = TcpMessage::new();

        assert_eq!(
            tcp_msg_read_from(&mut reader, &mut message),
            MsgStatus::Incomplete
        );
        assert_eq!(message.transmission_count(), 2);
        assert_eq!(
            tcp_msg_read_from(&mut reader, &mut message),
            MsgStatus::Incomplete
        );
        assert_eq!(message.transmission_count(), 6);
        assert_eq!(
            tcp_msg_read_from(&mut reader, &mut message),
            MsgStatus::Incomplete
        );
        assert_eq!(message.transmission_count(), 8);
        assert_eq!(
            tcp_msg_read_from(&mut reader, &mut message),
            MsgStatus::Success
        );
        assert!(message.is_complete());
        assert_eq!(message.unpack(), b"hello");
    }

    #[test]
    fn read_accumulates_payload_with_c_string_truncation() {
        let bytes = TcpMessage::pack_payload(b"a\0b")
            .unwrap()
            .content_bytes()
            .to_vec();
        let mut reader = LimitedReader::new(bytes, usize::MAX);
        let mut message = TcpMessage::new();

        assert_eq!(
            tcp_msg_read_from(&mut reader, &mut message),
            MsgStatus::Success
        );
        assert_eq!(message.transmission_count(), TCP_HEADER_SIZE + 3);
        assert_eq!(message.raw_payload_bytes(), b"a");
        assert_eq!(message.unpack(), b"a");
    }

    #[test]
    fn read_reports_closed_connection_before_any_header_bytes() {
        let mut reader = Cursor::new(Vec::new());
        let mut message = TcpMessage::new();

        assert_eq!(
            tcp_msg_read_from(&mut reader, &mut message),
            MsgStatus::ConnClosed
        );
    }

    #[test]
    fn read_reports_c_empty_payload_as_closed_after_header() {
        let mut reader = Cursor::new(
            u32::try_from(TCP_HEADER_SIZE)
                .unwrap()
                .to_be_bytes()
                .to_vec(),
        );
        let mut message = TcpMessage::new();

        assert_eq!(
            tcp_msg_read_from(&mut reader, &mut message),
            MsgStatus::ConnClosed
        );
        assert_eq!(message.message_len(), Some(TCP_HEADER_SIZE));
    }

    #[test]
    fn recv_blocks_until_full_message_or_terminal_status() {
        let mut reader = LimitedReader::new(packed_bytes("ready"), 3);

        let (message, status) = tcp_msg_recv_from(&mut reader);

        assert_eq!(status, MsgStatus::Success);
        assert_eq!(message.unpack(), b"ready");
    }

    #[test]
    fn string_send_and_recv_use_message_protocol() {
        let mut writer = LimitedWriter::new(usize::MAX);
        assert_eq!(
            tcp_string_send_to(&mut writer, "hello", false).unwrap(),
            MsgStatus::Success
        );

        let mut reader = Cursor::new(writer.bytes);
        let (message, status) = tcp_string_recv_from(&mut reader, false).unwrap();
        assert_eq!(status, MsgStatus::Success);
        assert_eq!(message.as_deref(), Some("hello"));
    }

    #[test]
    fn server_socket_wrapper_binds_to_ephemeral_port() {
        let listener = create_server_socket(0).unwrap();
        assert_ne!(listener.local_addr().unwrap().port(), 0);
    }

    #[test]
    fn client_socket_loop_uses_final_address_outcome_like_c() {
        let addresses = [loopback_addr(1), loopback_addr(2), loopback_addr(3)];
        let mut attempts = Vec::new();
        let result = connect_client_like_c(addresses, |address| {
            attempts.push(address.port());
            if address.port() == 2 {
                Ok(format!("connected:{}", address.port()))
            } else {
                Err(format!("failed:{}", address.port()))
            }
        });

        assert_eq!(attempts, [1, 2, 3]);
        assert_eq!(result, Err(Some("failed:3".to_owned())));

        let addresses = [loopback_addr(4), loopback_addr(5)];
        let result = connect_client_like_c(addresses, |address| {
            if address.port() == 5 {
                Ok(format!("connected:{}", address.port()))
            } else {
                Err(format!("failed:{}", address.port()))
            }
        });

        assert_eq!(result.as_deref(), Ok("connected:5"));
    }
}
