use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::multiplexer::TcpChannel;
use crate::inout::network::MsgStatus;
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Descriptor(u64);

impl Descriptor {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DescriptorInterestSet {
    read: BTreeSet<Descriptor>,
    write: BTreeSet<Descriptor>,
}

impl DescriptorInterestSet {
    pub fn clear(&mut self) {
        self.read.clear();
        self.write.clear();
    }

    pub fn set_read(&mut self, descriptor: Descriptor) {
        let _ = self.read.insert(descriptor);
    }

    pub fn set_write(&mut self, descriptor: Descriptor) {
        let _ = self.write.insert(descriptor);
    }

    #[must_use]
    pub fn contains_read(&self, descriptor: Descriptor) -> bool {
        self.read.contains(&descriptor)
    }

    #[must_use]
    pub fn contains_write(&self, descriptor: Descriptor) -> bool {
        self.write.contains(&descriptor)
    }

    pub fn read_descriptors(&self) -> impl Iterator<Item = Descriptor> + '_ {
        self.read.iter().copied()
    }

    pub fn write_descriptors(&self) -> impl Iterator<Item = Descriptor> + '_ {
        self.write.iter().copied()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ESessionState {
    #[default]
    NoState,
    Waiting,
    Active,
    Stale,
}

impl ESessionState {
    #[must_use]
    pub const fn c_value(self) -> u8 {
        match self {
            Self::NoState => 0,
            Self::Waiting => 1,
            Self::Active => 2,
            Self::Stale => 3,
        }
    }
}

pub trait SessionProcessSet {
    fn init_read_fd_set(&self, interests: &mut DescriptorInterestSet) -> Descriptor;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NoProcessControlSet;

impl SessionProcessSet for NoProcessControlSet {
    fn init_read_fd_set(&self, _interests: &mut DescriptorInterestSet) -> Descriptor {
        Descriptor::ZERO
    }
}

#[derive(Debug)]
pub struct ESession<S, P = NoProcessControlSet> {
    state: ESessionState,
    descriptor: Descriptor,
    channel: TcpChannel<S>,
    running: Option<P>,
}

impl<S, P> ESession<S, P>
where
    S: Read + Write,
{
    #[must_use]
    pub fn new(stream: S, descriptor: Descriptor) -> Self {
        Self {
            state: ESessionState::NoState,
            descriptor,
            channel: TcpChannel::new(stream),
            running: None,
        }
    }
}

impl ESession<TcpStream, NoProcessControlSet> {
    pub fn from_tcp_stream(stream: TcpStream) -> Result<Self, Diagnostic> {
        let descriptor = descriptor_from_tcp_stream(&stream)?;
        Ok(Self::new(stream, descriptor))
    }
}

impl<S, P> ESession<S, P>
where
    S: Read + Write,
{
    #[must_use]
    pub const fn state(&self) -> ESessionState {
        self.state
    }

    pub fn set_state(&mut self, state: ESessionState) {
        self.state = state;
    }

    #[must_use]
    pub const fn descriptor(&self) -> Descriptor {
        self.descriptor
    }

    #[must_use]
    pub const fn running(&self) -> Option<&P> {
        self.running.as_ref()
    }

    pub fn set_running(&mut self, running: Option<P>) {
        self.running = running;
    }

    #[must_use]
    pub const fn channel(&self) -> &TcpChannel<S> {
        &self.channel
    }

    pub fn channel_mut(&mut self) -> &mut TcpChannel<S> {
        &mut self.channel
    }

    #[must_use]
    pub fn into_channel(self) -> TcpChannel<S> {
        self.channel
    }

    pub fn init_fd_set(&self, interests: &mut DescriptorInterestSet) -> Descriptor
    where
        P: SessionProcessSet,
    {
        if matches!(self.state, ESessionState::NoState | ESessionState::Stale) {
            return Descriptor::ZERO;
        }
        debug_assert!(!self.channel.is_closed());

        interests.set_read(self.descriptor);
        if self.channel.has_out_msg() {
            interests.set_write(self.descriptor);
        }
        let process_max = self.running.as_ref().map_or(Descriptor::ZERO, |running| {
            running.init_read_fd_set(interests)
        });
        self.descriptor.max(process_max)
    }

    pub fn do_io(&mut self, interests: &DescriptorInterestSet) -> Result<(), Diagnostic> {
        // C `ESessionDoIO` tests `session->running` here, but the subprocess
        // I/O block is intentionally empty. Process descriptors participate in
        // readiness collection without being consumed by the session loop.
        if matches!(self.state, ESessionState::NoState | ESessionState::Stale) {
            return Ok(());
        }
        debug_assert!(!self.channel.is_closed());

        if interests.contains_read(self.descriptor) {
            match self.channel.read() {
                MsgStatus::ConnClosed | MsgStatus::Error => self.close_as_stale(),
                MsgStatus::Success => self.channel.send_str("wait")?,
                MsgStatus::Incomplete => {}
            }
        }
        if matches!(self.state, ESessionState::NoState | ESessionState::Stale) {
            return Ok(());
        }
        if interests.contains_write(self.descriptor) {
            match self.channel.write() {
                MsgStatus::ConnClosed | MsgStatus::Error => self.close_as_stale(),
                MsgStatus::Incomplete | MsgStatus::Success => {}
            }
        }
        Ok(())
    }

    pub fn process_cmds(&mut self, output: &mut impl Write) -> Result<usize, Diagnostic> {
        let mut processed = 0;
        while self.channel.has_in_msg() {
            let Some(message) = self.channel.get_in_msg() else {
                break;
            };
            writeln!(output, "Received: {}", message.unpack_string_lossy()).map_err(|error| {
                session_error(format!("Could not write session command: {error}"))
            })?;
            processed += 1;
        }
        Ok(processed)
    }

    fn close_as_stale(&mut self) {
        let _ = self.channel.close();
        self.state = ESessionState::Stale;
    }
}

#[cfg(unix)]
pub fn descriptor_from_tcp_stream(stream: &TcpStream) -> Result<Descriptor, Diagnostic> {
    use std::os::fd::AsRawFd;

    let raw = stream.as_raw_fd();
    u64::try_from(raw)
        .map(Descriptor::new)
        .map_err(|_| session_error(format!("Invalid TCP stream descriptor: {raw}")))
}

#[cfg(windows)]
pub fn descriptor_from_tcp_stream(stream: &TcpStream) -> Result<Descriptor, Diagnostic> {
    use std::os::windows::io::AsRawSocket;

    Ok(Descriptor::new(stream.as_raw_socket()))
}

fn session_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::INTERFACE_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::{Descriptor, DescriptorInterestSet, ESession, ESessionState, SessionProcessSet};
    use crate::inout::network::{MsgStatus, TcpMessage};
    use std::io::{self, Cursor, Read, Write};

    #[derive(Debug)]
    struct Duplex {
        incoming: Cursor<Vec<u8>>,
        written: Vec<u8>,
        max_read: usize,
        max_write: usize,
    }

    impl Duplex {
        fn new(incoming: Vec<u8>, max_read: usize, max_write: usize) -> Self {
            Self {
                incoming: Cursor::new(incoming),
                written: Vec::new(),
                max_read,
                max_write,
            }
        }
    }

    impl Read for Duplex {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let limit = buffer.len().min(self.max_read);
            self.incoming.read(&mut buffer[..limit])
        }
    }

    impl Write for Duplex {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            let limit = buffer.len().min(self.max_write);
            self.written.extend_from_slice(&buffer[..limit]);
            Ok(limit)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn packed(text: &str) -> Vec<u8> {
        TcpMessage::pack(text).unwrap().content_bytes().to_vec()
    }

    fn session(stream: Duplex, descriptor: Descriptor) -> ESession<Duplex> {
        ESession::new(stream, descriptor)
    }

    #[derive(Debug)]
    struct DescriptorOnlyProcessSet {
        descriptor: Descriptor,
    }

    impl SessionProcessSet for DescriptorOnlyProcessSet {
        fn init_read_fd_set(&self, interests: &mut DescriptorInterestSet) -> Descriptor {
            interests.set_read(self.descriptor);
            self.descriptor
        }
    }

    #[test]
    fn session_state_values_match_c_enum() {
        assert_eq!(ESessionState::NoState.c_value(), 0);
        assert_eq!(ESessionState::Waiting.c_value(), 1);
        assert_eq!(ESessionState::Active.c_value(), 2);
        assert_eq!(ESessionState::Stale.c_value(), 3);
    }

    #[test]
    fn new_session_starts_without_readiness() {
        let session = session(Duplex::new(Vec::new(), 1, 1), Descriptor::new(7));
        let mut interests = DescriptorInterestSet::default();

        assert_eq!(session.state(), ESessionState::NoState);
        assert_eq!(session.init_fd_set(&mut interests), Descriptor::ZERO);
        assert!(!interests.contains_read(Descriptor::new(7)));
        assert!(!interests.contains_write(Descriptor::new(7)));
    }

    #[test]
    fn active_session_sets_read_and_pending_write_interest() {
        let mut session = session(Duplex::new(Vec::new(), 1, 1), Descriptor::new(8));
        session.set_state(ESessionState::Waiting);
        session.channel_mut().send_str("queued").unwrap();
        let mut interests = DescriptorInterestSet::default();

        assert_eq!(session.init_fd_set(&mut interests), Descriptor::new(8));
        assert!(interests.contains_read(Descriptor::new(8)));
        assert!(interests.contains_write(Descriptor::new(8)));
    }

    #[test]
    fn running_process_readiness_is_registered_but_io_is_a_c_compatible_no_op() {
        let socket_descriptor = Descriptor::new(8);
        let process_descriptor = Descriptor::new(12);
        let mut session = ESession::new(
            Duplex::new(packed("unread"), usize::MAX, usize::MAX),
            socket_descriptor,
        );
        session.set_state(ESessionState::Active);
        session.set_running(Some(DescriptorOnlyProcessSet {
            descriptor: process_descriptor,
        }));
        let mut interests = DescriptorInterestSet::default();

        assert_eq!(session.init_fd_set(&mut interests), process_descriptor);
        assert!(interests.contains_read(socket_descriptor));
        assert!(interests.contains_read(process_descriptor));

        let mut process_ready = DescriptorInterestSet::default();
        process_ready.set_read(process_descriptor);
        session.do_io(&process_ready).unwrap();

        assert_eq!(session.state(), ESessionState::Active);
        assert!(session.running().is_some());
        assert!(!session.channel().has_in_msg());
        assert!(!session.channel().has_out_msg());
    }

    #[test]
    fn do_io_reads_command_queues_wait_and_writes_reply() {
        let mut session = session(
            Duplex::new(packed("run"), usize::MAX, usize::MAX),
            Descriptor::new(9),
        );
        session.set_state(ESessionState::Active);
        let mut read_ready = DescriptorInterestSet::default();
        read_ready.set_read(Descriptor::new(9));

        session.do_io(&read_ready).unwrap();
        assert!(session.channel().has_in_msg());
        assert!(session.channel().has_out_msg());

        let mut output = Vec::new();
        assert_eq!(session.process_cmds(&mut output).unwrap(), 1);
        assert_eq!(String::from_utf8(output).unwrap(), "Received: run\n");

        let mut write_ready = DescriptorInterestSet::default();
        write_ready.set_write(Descriptor::new(9));
        session.do_io(&write_ready).unwrap();

        let stream = session.into_channel().into_inner().unwrap();
        assert_eq!(stream.written, packed("wait"));
    }

    #[test]
    fn do_io_marks_session_stale_on_closed_input() {
        let mut session = session(
            Duplex::new(Vec::new(), usize::MAX, usize::MAX),
            Descriptor::new(10),
        );
        session.set_state(ESessionState::Waiting);
        let mut interests = DescriptorInterestSet::default();
        interests.set_read(Descriptor::new(10));

        session.do_io(&interests).unwrap();

        assert_eq!(session.state(), ESessionState::Stale);
        assert_eq!(session.channel_mut().read(), MsgStatus::Error);
    }
}
