use crate::basics::error::{Diagnostic, ErrorCode};
use std::fs;
use std::path::Path;

pub const MAX_LOOKAHEAD: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamType {
    File,
    InternalString,
    UserString,
    OptionString,
}

impl StreamType {
    #[must_use]
    pub const fn description(self) -> Option<&'static str> {
        match self {
            Self::File => None,
            Self::InternalString => Some(
                "Internal (programmer-defined) string - if you see this, you encountered a bug",
            ),
            Self::UserString => Some("Parsing a user provided string"),
            Self::OptionString => Some("Parsing a user given option argument"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputStream {
    source: Vec<u8>,
    data: Vec<u8>,
    stream_type: StreamType,
    string_pos: usize,
    eof_seen: bool,
    line: usize,
    column: usize,
    buffer: [Option<u8>; MAX_LOOKAHEAD],
    current: usize,
}

impl InputStream {
    #[must_use]
    pub fn from_user_string(source: &str) -> Self {
        Self::from_string(StreamType::UserString, source)
    }

    #[must_use]
    pub fn from_option_string(source: &str) -> Self {
        Self::from_string(StreamType::OptionString, source)
    }

    #[must_use]
    pub fn from_internal_string(source: &str) -> Self {
        Self::from_string(StreamType::InternalString, source)
    }

    #[must_use]
    pub fn from_string(stream_type: StreamType, source: &str) -> Self {
        let mut stream = Self {
            source: source.as_bytes().to_vec(),
            data: source.as_bytes().to_vec(),
            stream_type,
            string_pos: 0,
            eof_seen: false,
            line: 1,
            column: 1,
            buffer: [None; MAX_LOOKAHEAD],
            current: 0,
        };
        for index in 0..MAX_LOOKAHEAD {
            stream.buffer[index] = stream.read_char();
        }
        stream
    }

    pub fn from_file(path: &Path) -> Result<Self, Diagnostic> {
        let data = fs::read(path).map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot open file {} for reading: {error}", path.display()),
            )
        })?;
        let mut stream = Self {
            source: path.to_string_lossy().as_bytes().to_vec(),
            data,
            stream_type: StreamType::File,
            string_pos: 0,
            eof_seen: false,
            line: 1,
            column: 1,
            buffer: [None; MAX_LOOKAHEAD],
            current: 0,
        };
        for index in 0..MAX_LOOKAHEAD {
            stream.buffer[index] = stream.read_char();
        }
        Ok(stream)
    }

    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }

    #[must_use]
    pub const fn stream_type(&self) -> StreamType {
        self.stream_type
    }

    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    #[must_use]
    pub const fn column(&self) -> usize {
        self.column
    }

    #[must_use]
    pub fn current_char(&self) -> Option<u8> {
        self.buffer[self.current]
    }

    #[must_use]
    /// Returns the byte `look` positions after the current stream position.
    ///
    /// # Panics
    ///
    /// Panics when `look >= MAX_LOOKAHEAD`, matching the assertion in the C
    /// `StreamLookChar` macro.
    pub fn look_char(&self, look: usize) -> Option<u8> {
        assert!(look < MAX_LOOKAHEAD);
        self.buffer[real_pos(self.current + look)]
    }

    pub fn next_char(&mut self) -> Option<u8> {
        if self.current_char() == Some(b'\n') {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.current = real_pos(self.current + 1);
        let fill = real_pos(self.current + MAX_LOOKAHEAD - 1);
        self.buffer[fill] = self.read_char();
        self.current_char()
    }

    fn read_char(&mut self) -> Option<u8> {
        if self.eof_seen {
            return None;
        }

        let Some(byte) = self.data.get(self.string_pos).copied() else {
            self.eof_seen = true;
            return None;
        };
        if byte == 0 {
            self.eof_seen = true;
            None
        } else {
            self.string_pos += 1;
            Some(byte)
        }
    }
}

#[must_use]
pub const fn real_pos(pos: usize) -> usize {
    pos % MAX_LOOKAHEAD
}

#[cfg(test)]
mod tests {
    use super::InputStream;
    use std::path::{Path, PathBuf};

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("streams-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[test]
    fn stream_prefills_lookahead_and_tracks_columns_like_c_streams() {
        let mut stream = InputStream::from_user_string("ab\nc");
        assert_eq!(stream.current_char(), Some(b'a'));
        assert_eq!(stream.look_char(1), Some(b'b'));
        assert_eq!(stream.look_char(2), Some(b'\n'));
        assert_eq!(stream.look_char(3), Some(b'c'));
        assert_eq!((stream.line(), stream.column()), (1, 1));

        stream.next_char();
        assert_eq!(stream.current_char(), Some(b'b'));
        assert_eq!((stream.line(), stream.column()), (1, 2));

        stream.next_char();
        assert_eq!(stream.current_char(), Some(b'\n'));
        assert_eq!((stream.line(), stream.column()), (1, 3));

        stream.next_char();
        assert_eq!(stream.current_char(), Some(b'c'));
        assert_eq!((stream.line(), stream.column()), (2, 1));
    }

    #[test]
    fn stream_returns_infinite_eof_after_nul_or_end() {
        let mut stream = InputStream::from_user_string("a\0b");
        assert_eq!(stream.current_char(), Some(b'a'));
        stream.next_char();
        assert_eq!(stream.current_char(), None);
        stream.next_char();
        assert_eq!(stream.current_char(), None);
    }

    #[test]
    fn file_stream_uses_filename_as_source_label_and_reads_bytes() {
        let path = temp_path("file");
        remove_if_present(&path);
        std::fs::write(&path, b"ab\nc").unwrap();

        let mut stream = InputStream::from_file(&path).unwrap();

        assert_eq!(stream.source_bytes(), path.to_string_lossy().as_bytes());
        assert_eq!(stream.stream_type(), super::StreamType::File);
        assert_eq!(stream.current_char(), Some(b'a'));
        assert_eq!(stream.look_char(1), Some(b'b'));
        assert_eq!(stream.look_char(2), Some(b'\n'));
        stream.next_char();
        stream.next_char();
        stream.next_char();
        assert_eq!((stream.line(), stream.column()), (2, 1));
        assert_eq!(stream.current_char(), Some(b'c'));

        remove_if_present(&path);
    }
}
