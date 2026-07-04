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

    /// Append the first `len` bytes of `buffer` using the C `int len` loop shape.
    ///
    /// C `DStrAppendBuffer` uses `for(i=0; i<len; i++)`, so zero and negative
    /// lengths perform no work. The C helper trusts the raw pointer/length
    /// pair; Rust treats a length beyond the provided slice as an invariant
    /// failure instead of reading past the buffer.
    ///
    /// # Panics
    ///
    /// Panics when `len` is larger than the provided buffer.
    pub fn append_buffer_c(&mut self, buffer: &[u8], len: i32) {
        if len <= 0 {
            return;
        }
        let Ok(len) = usize::try_from(len) else {
            return;
        };
        assert!(
            len <= buffer.len(),
            "DStrAppendBuffer length exceeds buffer"
        );
        self.append_buffer(&buffer[..len]);
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

    /// Append a C-shaped NULL-terminated string array.
    ///
    /// `DStrAppendStrArray` stops at the first NULL entry. A first NULL entry
    /// leaves the descriptor untouched.
    pub fn append_str_array_c(&mut self, parts: &[Option<&str>], separator: &str) {
        let mut iterator = parts.iter().copied().map_while(|part| part);
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
        self.bytes
            .get(index)
            .copied()
            .or_else(|| (index == self.bytes.len() && self.mem > 0).then_some(0))
    }

    #[must_use]
    pub fn copy(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// Copy the bytes excluding the first and last byte.
    ///
    /// # Panics
    ///
    /// Panics when the string has fewer than two bytes, matching the C
    /// `DStrCopyCore` assertion.
    #[must_use]
    pub fn copy_core(&self) -> Vec<u8> {
        assert!(
            self.bytes.len() >= 2,
            "DStrCopyCore requires at least two bytes"
        );
        self.bytes[1..self.bytes.len() - 1].to_vec()
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
        assert_eq!(string.address(string.len()), Some(0));
        assert_eq!(string.address(99), None);
    }

    #[test]
    fn append_str_array_c_stops_at_first_null_sentinel() {
        let mut string = DynamicString::new();

        string.append_str_array_c(&[Some("alpha"), Some("beta"), None, Some("ignored")], "::");

        assert_eq!(string.view_bytes(), b"alpha::beta");
    }

    #[test]
    fn append_str_array_c_first_null_leaves_descriptor_untouched() {
        let mut string = DynamicString::new();

        string.append_str_array_c(&[None, Some("ignored")], ",");

        assert_eq!(string.view_bytes(), b"");
        assert_eq!(string.allocated_mem(), 0);
    }

    #[test]
    fn signed_append_buffer_preserves_c_nonpositive_len_noop() {
        let mut string = DynamicString::new();

        string.append_buffer_c(b"ignored", 0);
        string.append_buffer_c(b"ignored", -3);

        assert_eq!(string.view_bytes(), b"");
        assert_eq!(string.allocated_mem(), 0);

        string.append_buffer_c(b"abcdef", 3);

        assert_eq!(string.view_bytes(), b"abc");
        assert_eq!(string.allocated_mem(), DSTR_GROW);
    }

    #[test]
    #[should_panic(expected = "DStrAppendBuffer length exceeds buffer")]
    fn signed_append_buffer_rejects_read_past_buffer_boundary() {
        let mut string = DynamicString::new();

        string.append_buffer_c(b"abc", 4);
    }

    #[test]
    fn address_exposes_allocated_c_nul_slot_at_len() {
        let mut string = DynamicString::new();
        assert_eq!(string.address(0), None);

        string.append_str("");
        assert_eq!(string.len(), 0);
        assert_eq!(string.address(0), Some(0));

        string.append_str("abc");
        assert_eq!(string.address(3), Some(0));

        string.reset();
        assert_eq!(string.address(0), Some(0));

        string.minimize();
        assert_eq!(string.address(0), None);
    }

    #[test]
    fn delete_copy_core_set_reset_and_minimize_match_c_contracts() {
        let mut string = DynamicString::new();
        assert_eq!(string.delete_last_char(), 0);
        string.set("\"core\"");
        assert_eq!(string.copy_core(), b"core".to_vec());
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
    #[should_panic(expected = "DStrCopyCore requires at least two bytes")]
    fn copy_core_panics_on_short_string_like_c_assertion() {
        let string = DynamicString::new();

        let _ = string.copy_core();
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
