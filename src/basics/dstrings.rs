use std::borrow::Cow;
use std::io::{self, BufRead};

pub const DSTR_GROW: usize = 64;
pub const DSTR_GETS_CHUNK: usize = 256;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DynamicString {
    bytes: Vec<u8>,
    mem: usize,
}

impl DynamicString {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: Vec::new(),
            mem: 0,
        }
    }

    pub fn append_str(&mut self, new_part: &str) {
        self.ensure_for_str_append(new_part.len());
        self.bytes.extend_from_slice(new_part.as_bytes());
    }

    pub fn append_byte(&mut self, new_byte: u8) {
        self.ensure_for_byte_append();
        self.bytes.push(new_byte);
    }

    pub fn append_buffer(&mut self, buffer: &[u8]) {
        for byte in buffer {
            self.append_byte(*byte);
        }
    }

    pub fn append_bytes_with_str_growth(&mut self, buffer: &[u8]) {
        self.ensure_for_str_append(buffer.len());
        self.bytes.extend_from_slice(buffer);
    }

    pub fn append_int(&mut self, new_part: i64) {
        self.append_str(&new_part.to_string());
    }

    pub fn append_str_array<'a>(
        &mut self,
        parts: impl IntoIterator<Item = &'a str>,
        separator: &str,
    ) {
        let mut iterator = parts.into_iter();
        if let Some(first) = iterator.next() {
            self.append_str(first);
            for part in iterator {
                self.append_str(separator);
                self.append_str(part);
            }
        }
    }

    pub fn delete_last_char(&mut self) -> u8 {
        self.bytes.pop().unwrap_or(0)
    }

    #[must_use]
    pub fn last_char(&self) -> u8 {
        self.bytes.last().copied().unwrap_or(0)
    }

    #[must_use]
    pub fn view(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.bytes)
    }

    #[must_use]
    pub fn view_bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn address(&self, index: usize) -> Option<u8> {
        self.bytes.get(index).copied()
    }

    #[must_use]
    pub fn copy(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    #[must_use]
    pub fn copy_core(&self) -> Option<Vec<u8>> {
        (self.bytes.len() >= 2).then(|| self.bytes[1..self.bytes.len() - 1].to_vec())
    }

    pub fn set(&mut self, value: &str) {
        self.reset();
        self.append_str(value);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    #[must_use]
    pub const fn allocated_mem(&self) -> usize {
        self.mem
    }

    pub fn reset(&mut self) {
        self.bytes.clear();
    }

    pub fn minimize(&mut self) {
        if self.bytes.is_empty() {
            self.bytes = Vec::new();
            self.mem = 0;
        } else {
            self.mem = self.bytes.len() + 1;
            self.bytes.shrink_to_fit();
            if self.bytes.capacity() < self.mem {
                self.bytes.reserve_exact(self.mem - self.bytes.capacity());
            }
        }
    }

    pub fn read_line<R: BufRead>(&mut self, reader: &mut R) -> io::Result<bool> {
        self.reset();
        let mut chunk = Vec::with_capacity(DSTR_GETS_CHUNK);
        let read = reader.read_until(b'\n', &mut chunk)?;
        if read == 0 {
            Ok(false)
        } else {
            self.append_buffer(&chunk);
            Ok(true)
        }
    }

    fn ensure_for_str_append(&mut self, additional: usize) {
        let mut new_mem = self.mem;
        while self.bytes.len() + additional >= new_mem {
            new_mem += DSTR_GROW;
        }
        self.set_allocated_mem(new_mem);
    }

    fn ensure_for_byte_append(&mut self) {
        if self.bytes.len() + 1 >= self.mem {
            self.set_allocated_mem(self.bytes.len() + DSTR_GROW);
        }
    }

    fn set_allocated_mem(&mut self, new_mem: usize) {
        if new_mem > self.mem {
            self.mem = new_mem;
            let capacity = self.bytes.capacity();
            if capacity < self.mem {
                self.bytes.reserve_exact(self.mem - capacity);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{DynamicString, DSTR_GROW};

    #[test]
    fn new_string_views_as_empty_without_allocation() {
        let string = DynamicString::new();
        assert_eq!(string.len(), 0);
        assert!(string.is_empty());
        assert_eq!(string.allocated_mem(), 0);
        assert_eq!(string.view(), "");
        assert_eq!(string.view_bytes(), b"");
    }

    #[test]
    fn append_str_uses_c_growth_rule() {
        let mut string = DynamicString::new();
        string.append_str("");
        assert_eq!(string.len(), 0);
        assert_eq!(string.allocated_mem(), DSTR_GROW);

        string.append_str(&"x".repeat(DSTR_GROW - 1));
        assert_eq!(string.len(), DSTR_GROW - 1);
        assert_eq!(string.allocated_mem(), DSTR_GROW);

        string.append_str("y");
        assert_eq!(string.len(), DSTR_GROW);
        assert_eq!(string.allocated_mem(), DSTR_GROW * 2);
    }

    #[test]
    fn append_byte_uses_c_byte_growth_rule() {
        let mut string = DynamicString::new();
        for _ in 0..DSTR_GROW - 1 {
            string.append_byte(b'x');
        }
        assert_eq!(string.len(), DSTR_GROW - 1);
        assert_eq!(string.allocated_mem(), DSTR_GROW);

        string.append_byte(b'y');
        assert_eq!(string.len(), DSTR_GROW);
        assert_eq!(string.allocated_mem(), (DSTR_GROW - 1) + DSTR_GROW);
    }

    #[test]
    fn append_helpers_preserve_order_and_bytes() {
        let mut string = DynamicString::new();
        string.append_str("a");
        string.append_byte(b'b');
        string.append_buffer(b"cd");
        string.append_bytes_with_str_growth(b"ef");
        string.append_int(-12);
        string.append_str_array(["x", "y", "z"], ",");
        assert_eq!(string.view_bytes(), b"abcdef-12x,y,z");
        assert_eq!(string.last_char(), b'z');
        assert_eq!(string.address(2), Some(b'c'));
        assert_eq!(string.address(99), None);
    }

    #[test]
    fn delete_copy_core_set_reset_and_minimize_match_c_contracts() {
        let mut string = DynamicString::new();
        assert_eq!(string.delete_last_char(), 0);
        string.set("\"core\"");
        assert_eq!(string.copy_core(), Some(b"core".to_vec()));
        assert_eq!(string.delete_last_char(), b'"');
        assert_eq!(string.copy(), b"\"core".to_vec());

        let allocated = string.allocated_mem();
        string.reset();
        assert!(string.is_empty());
        assert_eq!(string.allocated_mem(), allocated);

        string.minimize();
        assert_eq!(string.allocated_mem(), 0);
    }

    #[test]
    fn read_line_returns_false_only_on_empty_eof() {
        let mut cursor = Cursor::new(b"abc\ndef".to_vec());
        let mut string = DynamicString::new();

        assert!(string.read_line(&mut cursor).unwrap());
        assert_eq!(string.view_bytes(), b"abc\n");

        assert!(string.read_line(&mut cursor).unwrap());
        assert_eq!(string.view_bytes(), b"def");

        assert!(!string.read_line(&mut cursor).unwrap());
        assert_eq!(string.view_bytes(), b"");
    }
}
