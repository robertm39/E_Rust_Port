use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::network::{tcp_msg_read_from, tcp_msg_write_to, MsgStatus, TcpMessage};
use std::collections::VecDeque;
use std::io::{Read, Write};

#[derive(Debug)]
pub struct TcpChannel<S> {
    stream: Option<S>,
    in_queue: VecDeque<TcpMessage>,
    out_queue: VecDeque<TcpMessage>,
}

impl<S> TcpChannel<S>
where
    S: Read + Write,
{
    #[must_use]
    pub fn new(stream: S) -> Self {
        Self {
            stream: Some(stream),
            in_queue: VecDeque::new(),
            out_queue: VecDeque::new(),
        }
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.stream.is_none()
    }

    pub fn close(&mut self) -> Result<(), Diagnostic> {
        if self.stream.take().is_some() {
            Ok(())
        } else {
            Err(channel_error("TCP channel is already closed"))
        }
    }

    #[must_use]
    pub fn into_inner(self) -> Option<S> {
        self.stream
    }

    #[must_use]
    pub fn in_len(&self) -> usize {
        self.in_queue.len()
    }

    #[must_use]
    pub fn out_len(&self) -> usize {
        self.out_queue.len()
    }

    #[must_use]
    pub fn has_out_msg(&self) -> bool {
        !self.out_queue.is_empty()
    }

    #[must_use]
    pub fn has_in_msg(&self) -> bool {
        self.in_queue.front().is_some_and(TcpMessage::is_complete)
    }

    pub fn get_in_msg(&mut self) -> Option<TcpMessage> {
        self.in_queue.pop_front()
    }

    pub fn send_msg(&mut self, message: TcpMessage) {
        self.out_queue.push_back(message);
    }

    pub fn send_str(&mut self, text: &str) -> Result<(), Diagnostic> {
        self.send_msg(TcpMessage::pack(text)?);
        Ok(())
    }

    pub fn read(&mut self) -> MsgStatus {
        let Some(stream) = self.stream.as_mut() else {
            return MsgStatus::Error;
        };

        if self.in_queue.back().is_none_or(TcpMessage::is_complete) {
            self.in_queue.push_back(TcpMessage::new());
        }

        let Some(current) = self.in_queue.back_mut() else {
            return MsgStatus::Error;
        };
        tcp_msg_read_from(stream, current)
    }

    pub fn write(&mut self) -> MsgStatus {
        let Some(stream) = self.stream.as_mut() else {
            return MsgStatus::Error;
        };
        let Some(current) = self.out_queue.front_mut() else {
            return MsgStatus::Success;
        };

        let status = tcp_msg_write_to(stream, current);
        if status == MsgStatus::Success {
            let _ = self.out_queue.pop_front();
        }
        status
    }

    /// Mirrors the C `TCPChannelWrite` queue choice exactly.
    ///
    /// The C source enqueues outbound messages in `channel->out`, but
    /// `TCPChannelWrite` checks and drains `channel->in`. Keep this as an
    /// explicit compatibility path; ordinary Rust callers should use
    /// [`TcpChannel::write`].
    pub fn write_c_in_queue(&mut self) -> MsgStatus {
        let Some(stream) = self.stream.as_mut() else {
            return MsgStatus::Error;
        };
        let Some(current) = self.in_queue.front_mut() else {
            return MsgStatus::Success;
        };

        let status = tcp_msg_write_to(stream, current);
        if status == MsgStatus::Success {
            let _ = self.in_queue.pop_front();
        }
        status
    }
}

fn channel_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::INTERFACE_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::TcpChannel;
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

    #[test]
    fn new_channel_starts_with_empty_queues_and_open_stream() {
        let channel = TcpChannel::new(Duplex::new(Vec::new(), 1, 1));

        assert!(!channel.is_closed());
        assert_eq!(channel.in_len(), 0);
        assert_eq!(channel.out_len(), 0);
        assert!(!channel.has_in_msg());
        assert!(!channel.has_out_msg());
    }

    #[test]
    fn read_uses_latest_incomplete_message_then_appends_after_complete() {
        let mut incoming = packed("one");
        incoming.extend_from_slice(&packed("two"));
        let mut channel = TcpChannel::new(Duplex::new(incoming, 2, usize::MAX));

        assert_eq!(channel.read(), MsgStatus::Incomplete);
        assert_eq!(channel.in_len(), 1);
        assert!(!channel.has_in_msg());

        while channel.read() == MsgStatus::Incomplete {}
        assert!(channel.has_in_msg());
        assert_eq!(channel.in_len(), 1);

        while channel.read() == MsgStatus::Incomplete {}
        assert_eq!(channel.in_len(), 2);

        assert_eq!(
            channel.get_in_msg().map(TcpMessage::unpack),
            Some(b"one".to_vec())
        );
        assert_eq!(
            channel.get_in_msg().map(TcpMessage::unpack),
            Some(b"two".to_vec())
        );
    }

    #[test]
    fn send_queues_messages_and_write_drains_out_queue() {
        let mut channel = TcpChannel::new(Duplex::new(Vec::new(), usize::MAX, 5));
        channel.send_str("abcdef").unwrap();

        assert!(channel.has_out_msg());
        assert_eq!(channel.out_len(), 1);
        assert_eq!(channel.write(), MsgStatus::Incomplete);
        assert!(channel.has_out_msg());
        assert_eq!(channel.write(), MsgStatus::Success);
        assert!(!channel.has_out_msg());

        let stream = channel.into_inner().unwrap();
        assert_eq!(stream.written, packed("abcdef"));
    }

    #[test]
    fn write_does_not_consume_complete_inbound_messages() {
        let incoming = packed("cmd");
        let mut channel = TcpChannel::new(Duplex::new(incoming, usize::MAX, usize::MAX));

        assert_eq!(channel.read(), MsgStatus::Success);
        assert!(channel.has_in_msg());
        channel.send_str("wait").unwrap();
        assert_eq!(channel.write(), MsgStatus::Success);

        assert!(channel.has_in_msg());
        assert_eq!(
            channel.get_in_msg().map(TcpMessage::unpack),
            Some(b"cmd".to_vec())
        );
        let stream = channel.into_inner().unwrap();
        assert_eq!(stream.written, packed("wait"));
    }

    #[test]
    fn c_in_queue_write_ignores_queued_outbound_messages() {
        let mut channel = TcpChannel::new(Duplex::new(Vec::new(), usize::MAX, usize::MAX));
        channel.send_str("queued").unwrap();

        assert_eq!(channel.write_c_in_queue(), MsgStatus::Success);
        assert!(channel.has_out_msg());
        assert_eq!(channel.out_len(), 1);

        let stream = channel.into_inner().unwrap();
        assert!(stream.written.is_empty());
    }

    #[test]
    fn c_in_queue_write_consumes_complete_inbound_message() {
        let incoming = packed("cmd");
        let mut channel = TcpChannel::new(Duplex::new(incoming, usize::MAX, usize::MAX));

        assert_eq!(channel.read(), MsgStatus::Success);
        assert!(channel.has_in_msg());
        assert_eq!(channel.write_c_in_queue(), MsgStatus::Success);

        assert_eq!(channel.in_len(), 0);
        assert!(!channel.has_in_msg());
        let stream = channel.into_inner().unwrap();
        assert!(stream.written.is_empty());
    }

    #[test]
    fn close_drops_stream_and_rejects_double_close() {
        let mut channel = TcpChannel::new(Duplex::new(Vec::new(), 1, 1));

        assert!(channel.close().is_ok());
        assert!(channel.is_closed());
        assert!(channel.close().is_err());
        assert_eq!(channel.read(), MsgStatus::Error);
        assert_eq!(channel.write(), MsgStatus::Error);
    }
}
