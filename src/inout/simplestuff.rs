use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::network::{tcp_msg_recv_from, MsgStatus};
use std::io::{self, BufRead, Read};

pub const READ_TEXT_BLOCK_CHUNK: usize = 255;

fn read_fgets_chunk(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut chunk = Vec::new();
    while chunk.len() < READ_TEXT_BLOCK_CHUNK {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }

        let remaining = READ_TEXT_BLOCK_CHUNK - chunk.len();
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len().min(remaining), |newline| {
                (newline + 1).min(remaining)
            });
        chunk.extend_from_slice(&available[..take]);
        reader.consume(take);
        if chunk.last() == Some(&b'\n') || take == remaining {
            break;
        }
    }

    if chunk.is_empty() {
        Ok(None)
    } else {
        Ok(Some(chunk))
    }
}

fn c_string_prefix(bytes: &[u8]) -> &[u8] {
    let nul_pos = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    &bytes[..nul_pos]
}

fn c_strings_equal(left: &[u8], right: &[u8]) -> bool {
    c_string_prefix(left) == c_string_prefix(right)
}

pub fn read_text_block(
    result: &mut DynamicString,
    reader: &mut impl BufRead,
    terminator: &[u8],
) -> io::Result<bool> {
    while let Some(chunk) = read_fgets_chunk(reader)? {
        if c_strings_equal(&chunk, terminator) {
            return Ok(true);
        }
        result.append_bytes_with_str_growth(c_string_prefix(&chunk));
    }
    Ok(false)
}

pub fn tcp_read_text_block<I, B>(
    result: &mut DynamicString,
    received_strings: I,
    terminator: &[u8],
) -> bool
where
    I: IntoIterator<Item = B>,
    B: AsRef<[u8]>,
{
    for received in received_strings {
        let bytes = received.as_ref();
        if c_strings_equal(bytes, terminator) {
            return true;
        }
        result.append_bytes_with_str_growth(c_string_prefix(bytes));
    }
    false
}

pub fn tcp_read_text_block_from(
    result: &mut DynamicString,
    reader: &mut impl Read,
    terminator: &[u8],
) -> Result<bool, Diagnostic> {
    loop {
        let (message, status) = tcp_msg_recv_from(reader);
        if status != MsgStatus::Success {
            return Err(Diagnostic::new(
                ErrorCode::SYSTEM_ERROR,
                "Could not receive string message",
            ));
        }
        let received = message.unpack();
        if c_strings_equal(&received, terminator) {
            return Ok(true);
        }
        result.append_bytes_with_str_growth(c_string_prefix(&received));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        read_text_block, tcp_read_text_block, tcp_read_text_block_from, READ_TEXT_BLOCK_CHUNK,
    };
    use crate::basics::dstrings::DynamicString;
    use crate::inout::network::TcpMessage;
    use std::io::Cursor;

    #[test]
    fn read_text_block_appends_until_terminator_without_clearing() {
        let mut result = DynamicString::new();
        result.append_str("prefix:");
        let mut input = Cursor::new(b"one\ntwo\nEND\nignored\n".to_vec());

        assert!(read_text_block(&mut result, &mut input, b"END\n").unwrap());
        assert_eq!(result.view_bytes(), b"prefix:one\ntwo\n");
    }

    #[test]
    fn read_text_block_returns_false_on_eof_after_appending() {
        let mut result = DynamicString::new();
        let mut input = Cursor::new(b"one\ntwo".to_vec());

        assert!(!read_text_block(&mut result, &mut input, b"END\n").unwrap());
        assert_eq!(result.view_bytes(), b"one\ntwo");
    }

    #[test]
    fn read_text_block_uses_c_fgets_sized_chunks() {
        let mut result = DynamicString::new();
        let mut input_bytes = vec![b'a'; READ_TEXT_BLOCK_CHUNK + 10];
        input_bytes.extend_from_slice(b"\nEND\n");
        let mut input = Cursor::new(input_bytes);

        assert!(read_text_block(&mut result, &mut input, b"END\n").unwrap());
        assert_eq!(result.len(), READ_TEXT_BLOCK_CHUNK + 11);
        assert_eq!(result.view_bytes()[READ_TEXT_BLOCK_CHUNK], b'a');
        assert_eq!(result.last_char(), b'\n');
    }

    #[test]
    fn read_text_block_uses_c_string_semantics_for_nul_bytes() {
        let mut result = DynamicString::new();
        result.append_str("prefix:");
        let mut input = Cursor::new(b"one\0hidden\nTERM\0ignored\nunread\n".to_vec());

        assert!(read_text_block(&mut result, &mut input, b"TERM\0other").unwrap());
        assert_eq!(result.view_bytes(), b"prefix:one");
    }

    #[test]
    fn tcp_read_text_block_stops_at_matching_received_string() {
        let mut result = DynamicString::new();
        assert!(tcp_read_text_block(
            &mut result,
            [
                b"alpha\n".as_slice(),
                b"END\n".as_slice(),
                b"ignored\n".as_slice()
            ],
            b"END\n"
        ));
        assert_eq!(result.view_bytes(), b"alpha\n");

        assert!(!tcp_read_text_block(
            &mut result,
            [b"tail\n".as_slice()],
            b"END\n"
        ));
        assert_eq!(result.view_bytes(), b"alpha\ntail\n");
    }

    #[test]
    fn tcp_read_text_block_uses_c_string_semantics_for_nul_bytes() {
        let mut result = DynamicString::new();

        assert!(tcp_read_text_block(
            &mut result,
            [b"one\0hidden\n".as_slice(), b"TERM\0ignored\n".as_slice()],
            b"TERM\0other"
        ));
        assert_eq!(result.view_bytes(), b"one");
    }

    #[test]
    fn tcp_read_text_block_from_reads_network_messages_until_terminator() {
        let mut bytes = TcpMessage::pack("one\n").unwrap().content_bytes().to_vec();
        bytes.extend_from_slice(TcpMessage::pack("two\n").unwrap().content_bytes());
        bytes.extend_from_slice(TcpMessage::pack("END\n").unwrap().content_bytes());
        bytes.extend_from_slice(TcpMessage::pack("ignored\n").unwrap().content_bytes());
        let mut reader = Cursor::new(bytes);
        let mut result = DynamicString::new();
        result.append_str("prefix:");

        assert!(tcp_read_text_block_from(&mut result, &mut reader, b"END\n").unwrap());
        assert_eq!(result.view_bytes(), b"prefix:one\ntwo\n");
    }

    #[test]
    fn tcp_read_text_block_from_reports_receive_failure() {
        let mut reader = Cursor::new(Vec::new());
        let mut result = DynamicString::new();

        let error = tcp_read_text_block_from(&mut result, &mut reader, b"END\n").unwrap_err();
        assert_eq!(error.code(), crate::basics::error::ErrorCode::SYSTEM_ERROR);
        assert_eq!(error.message(), "Could not receive string message");
    }
}
