use crate::basics::error::{Diagnostic, ErrorCode};
use crate::control::esession::{
    descriptor_from_tcp_stream, Descriptor, DescriptorInterestSet, ESession, NoProcessControlSet,
    SessionProcessSet,
};
use crate::inout::network::{create_server_socket_no_fail, listen};
use std::collections::VecDeque;
use std::net::{SocketAddr, TcpListener, TcpStream};

#[derive(Debug)]
pub struct EServer<P = NoProcessControlSet> {
    listener: Option<TcpListener>,
    listening_descriptor: Option<Descriptor>,
    sessions: VecDeque<ESession<TcpStream, P>>,
}

impl<P> Default for EServer<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P> EServer<P> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            listener: None,
            listening_descriptor: None,
            sessions: VecDeque::new(),
        }
    }

    #[must_use]
    pub const fn is_listening(&self) -> bool {
        self.listener.is_some()
    }

    #[must_use]
    pub const fn listening_descriptor(&self) -> Option<Descriptor> {
        self.listening_descriptor
    }

    #[must_use]
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    #[must_use]
    pub fn sessions(&self) -> &VecDeque<ESession<TcpStream, P>> {
        &self.sessions
    }

    pub fn sessions_mut(&mut self) -> &mut VecDeque<ESession<TcpStream, P>> {
        &mut self.sessions
    }

    pub fn reset(&mut self) {}

    pub fn listen(&mut self, port: u16) -> Result<bool, Diagnostic> {
        if self.listener.is_some() {
            return Err(server_error("E server is already listening"));
        }

        let Some(listener) = create_server_socket_no_fail(port) else {
            return Ok(false);
        };
        listen(&listener)?;
        let descriptor = descriptor_from_tcp_listener(&listener)?;
        self.listener = Some(listener);
        self.listening_descriptor = Some(descriptor);
        Ok(true)
    }

    pub fn local_addr(&self) -> Result<SocketAddr, Diagnostic> {
        self.listener
            .as_ref()
            .ok_or_else(|| server_error("E server is not listening"))?
            .local_addr()
            .map_err(|error| server_error(format!("Could not read server socket address: {error}")))
    }

    pub fn accept(&mut self) -> Result<bool, Diagnostic> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| server_error("E server is not listening"))?;
        match listener.accept() {
            Ok((stream, _addr)) => {
                let descriptor = descriptor_from_tcp_stream(&stream)?;
                self.sessions.push_back(ESession::new(stream, descriptor));
                Ok(true)
            }
            Err(error) => Err(server_error(format!(
                "Failure to accept connection: {error}"
            ))),
        }
    }

    pub fn init_fd_set(&self, interests: &mut DescriptorInterestSet) -> Descriptor
    where
        P: SessionProcessSet,
    {
        let mut max_descriptor = Descriptor::ZERO;
        for session in &self.sessions {
            max_descriptor = max_descriptor.max(session.init_fd_set(interests));
        }
        if let Some(descriptor) = self.listening_descriptor {
            max_descriptor = max_descriptor.max(descriptor);
            interests.set_read(descriptor);
        }
        max_descriptor
    }
}

#[cfg(unix)]
pub fn descriptor_from_tcp_listener(listener: &TcpListener) -> Result<Descriptor, Diagnostic> {
    use std::os::fd::AsRawFd;

    let raw = listener.as_raw_fd();
    u64::try_from(raw)
        .map(Descriptor::new)
        .map_err(|_| server_error(format!("Invalid TCP listener descriptor: {raw}")))
}

#[cfg(windows)]
pub fn descriptor_from_tcp_listener(listener: &TcpListener) -> Result<Descriptor, Diagnostic> {
    use std::os::windows::io::AsRawSocket;

    Ok(Descriptor::new(listener.as_raw_socket()))
}

fn server_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::INTERFACE_ERROR, message)
}

#[cfg(test)]
mod tests {
    use super::EServer;
    use crate::control::esession::{Descriptor, DescriptorInterestSet, NoProcessControlSet};
    use std::net::TcpStream;

    #[test]
    fn new_server_is_not_listening_and_has_no_sessions() {
        let server = EServer::<NoProcessControlSet>::new();
        let mut interests = DescriptorInterestSet::default();

        assert!(!server.is_listening());
        assert_eq!(server.session_count(), 0);
        assert_eq!(server.init_fd_set(&mut interests), Descriptor::ZERO);
        assert_eq!(interests.read_descriptors().count(), 0);
    }

    #[test]
    fn server_listens_accepts_and_registers_descriptors() {
        let mut server = EServer::<NoProcessControlSet>::new();
        assert!(server.listen(0).unwrap());
        let mut address = server.local_addr().unwrap();
        if address.ip().is_unspecified() {
            address.set_ip(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
        }
        let _client = TcpStream::connect(address).unwrap();

        assert!(server.accept().unwrap());

        assert_eq!(server.session_count(), 1);
        let listening = server.listening_descriptor().unwrap();
        let session = server.sessions().front().unwrap();
        let mut interests = DescriptorInterestSet::default();
        let max_descriptor = server.init_fd_set(&mut interests);

        assert!(max_descriptor >= listening);
        assert!(interests.contains_read(listening));
        assert!(!interests.contains_read(session.descriptor()));
    }
}
