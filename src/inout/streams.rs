use crate::basics::error::{Diagnostic, ErrorCode};
use std::fs;
use std::io::Read;
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
        Self::from_data(
            stream_type,
            source.as_bytes().to_vec(),
            source.as_bytes().to_vec(),
        )
    }

    #[must_use]
    pub fn from_file_content(source_name: &str, data: Vec<u8>) -> Self {
        Self::from_data(StreamType::File, source_name.as_bytes().to_vec(), data)
    }

    fn from_data(stream_type: StreamType, source: Vec<u8>, data: Vec<u8>) -> Self {
        let mut stream = Self {
            source,
            data,
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
        Ok(Self::from_file_content(&path.to_string_lossy(), data))
    }

    pub fn from_file_optional(path: &Path, fail: bool) -> Result<Option<Self>, Diagnostic> {
        match Self::from_file(path) {
            Ok(stream) => Ok(Some(stream)),
            Err(error) if fail => Err(error),
            Err(_) => Ok(None),
        }
    }

    pub fn create_stream(
        stream_type: StreamType,
        source: Option<&str>,
        fail: bool,
    ) -> Result<Option<Self>, Diagnostic> {
        let mut stdin = std::io::stdin().lock();
        Self::create_stream_with_stdin(stream_type, source, fail, &mut stdin)
    }

    pub fn create_stream_with_stdin(
        stream_type: StreamType,
        source: Option<&str>,
        fail: bool,
        stdin: &mut impl Read,
    ) -> Result<Option<Self>, Diagnostic> {
        if stream_type != StreamType::File {
            return Ok(Some(Self::from_string(stream_type, source.unwrap_or(""))));
        }

        let Some(source) = source else {
            return Self::from_stdin_reader(stdin);
        };
        if source == "-" {
            return Self::from_stdin_reader(stdin);
        }

        Self::from_file_optional(Path::new(source), fail)
    }

    fn from_stdin_reader(reader: &mut impl Read) -> Result<Option<Self>, Diagnostic> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot read <stdin> for stream: {error}"),
            )
        })?;
        Ok(Some(Self::from_file_content("<stdin>", data)))
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InputStreamStack {
    streams: Vec<InputStream>,
}

impl InputStreamStack {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.streams.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.streams.is_empty()
    }

    #[must_use]
    pub fn top(&self) -> Option<&InputStream> {
        self.streams.last()
    }

    #[must_use]
    pub fn top_mut(&mut self) -> Option<&mut InputStream> {
        self.streams.last_mut()
    }

    pub fn open_stacked_input(&mut self, stream: InputStream) -> &mut InputStream {
        self.streams.push(stream);
        let top = self.streams.len() - 1;
        &mut self.streams[top]
    }

    pub fn close_stacked_input(&mut self) -> Option<InputStream> {
        self.streams.pop()
    }

    /// Pops the top stream, asserting the C `CloseStackedInput` precondition.
    ///
    /// # Panics
    ///
    /// Panics when the stack is empty, matching the C assertion.
    pub fn close_stacked_input_asserting(&mut self) -> InputStream {
        self.close_stacked_input()
            .expect("CloseStackedInput requires a nonempty stack")
    }
}

#[must_use]
pub const fn real_pos(pos: usize) -> usize {
    pos % MAX_LOOKAHEAD
}

#[cfg(test)]
mod tests {
    use super::{InputStream, InputStreamStack};
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

    #[test]
    fn optional_file_stream_preserves_create_stream_fail_false_shape() {
        let path = temp_path("missing");
        remove_if_present(&path);

        assert!(InputStream::from_file_optional(&path, false)
            .unwrap()
            .is_none());
        assert!(InputStream::from_file_optional(&path, true).is_err());

        std::fs::write(&path, b"ok").unwrap();
        let stream = InputStream::from_file_optional(&path, false)
            .unwrap()
            .unwrap();
        assert_eq!(stream.current_char(), Some(b'o'));

        remove_if_present(&path);
    }

    #[test]
    fn create_stream_maps_stdin_sources_to_c_label() {
        let mut stdin = std::io::Cursor::new(b"stdin-bytes".to_vec());
        let stream =
            InputStream::create_stream_with_stdin(super::StreamType::File, None, true, &mut stdin)
                .unwrap()
                .unwrap();

        assert_eq!(stream.source_bytes(), b"<stdin>");
        assert_eq!(stream.current_char(), Some(b's'));

        let mut stdin = std::io::Cursor::new(b"dash".to_vec());
        let stream = InputStream::create_stream_with_stdin(
            super::StreamType::File,
            Some("-"),
            false,
            &mut stdin,
        )
        .unwrap()
        .unwrap();

        assert_eq!(stream.source_bytes(), b"<stdin>");
        assert_eq!(stream.current_char(), Some(b'd'));
    }

    #[test]
    fn create_stream_preserves_string_source_shape() {
        let mut stdin = std::io::Cursor::new(Vec::new());
        let stream = InputStream::create_stream_with_stdin(
            super::StreamType::OptionString,
            Some("abc"),
            true,
            &mut stdin,
        )
        .unwrap()
        .unwrap();

        assert_eq!(stream.stream_type(), super::StreamType::OptionString);
        assert_eq!(stream.source_bytes(), b"abc");
        assert_eq!(stream.current_char(), Some(b'a'));
    }

    #[test]
    fn file_content_stream_keeps_file_identity_without_opening_path() {
        let mut stream = InputStream::from_file_content("-", b"ab\nc".to_vec());

        assert_eq!(stream.source_bytes(), b"-");
        assert_eq!(stream.stream_type(), super::StreamType::File);
        assert_eq!(stream.current_char(), Some(b'a'));
        assert_eq!(stream.look_char(1), Some(b'b'));
        assert_eq!(stream.look_char(2), Some(b'\n'));
        stream.next_char();
        stream.next_char();
        stream.next_char();
        assert_eq!((stream.line(), stream.column()), (2, 1));
        assert_eq!(stream.current_char(), Some(b'c'));
    }

    #[test]
    fn stream_stack_opens_new_top_and_restores_previous_input() {
        let mut stack = InputStreamStack::new();

        stack.open_stacked_input(InputStream::from_user_string("outer"));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.top().and_then(InputStream::current_char), Some(b'o'));

        stack.open_stacked_input(InputStream::from_internal_string("inner"));
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.top().and_then(InputStream::current_char), Some(b'i'));

        stack.top_mut().unwrap().next_char();
        assert_eq!(stack.top().and_then(InputStream::current_char), Some(b'n'));

        let closed = stack.close_stacked_input().unwrap();
        assert_eq!(closed.current_char(), Some(b'n'));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.top().and_then(InputStream::current_char), Some(b'o'));

        let closed = stack.close_stacked_input_asserting();
        assert_eq!(closed.current_char(), Some(b'o'));
        assert!(stack.is_empty());
        assert!(stack.close_stacked_input().is_none());
    }

    #[test]
    #[should_panic(expected = "CloseStackedInput requires a nonempty stack")]
    fn stream_stack_asserting_close_matches_c_precondition() {
        InputStreamStack::new().close_stacked_input_asserting();
    }
}
