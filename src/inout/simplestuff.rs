use crate::basics::dstrings::DynamicString;
use std::io::{self, BufRead};

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

pub fn read_text_block(
    result: &mut DynamicString,
    reader: &mut impl BufRead,
    terminator: &[u8],
) -> io::Result<bool> {
    while let Some(chunk) = read_fgets_chunk(reader)? {
        if chunk == terminator {
            return Ok(true);
        }
        result.append_bytes_with_str_growth(&chunk);
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
        if bytes == terminator {
            return true;
        }
        result.append_bytes_with_str_growth(bytes);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{read_text_block, tcp_read_text_block, READ_TEXT_BLOCK_CHUNK};
    use crate::basics::dstrings::DynamicString;
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
}
