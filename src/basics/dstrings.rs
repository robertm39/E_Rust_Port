use std::borrow::Cow;
use std::io::{self, BufRead};

pub const DSTR_GROW: usize = 64;
pub const DSTR_GETS_CHUNK: usize = 256;

#[derive(Debug)]
pub struct DynamicString {
    bytes: Vec<u8>,
    mem: usize,
    refs: usize,
}

impl Clone for DynamicString {
    fn clone(&self) -> Self {
        Self {
            bytes: self.bytes.clone(),
            mem: self.mem,
            refs: 1,
        }
    }
}

impl Default for DynamicString {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for DynamicString {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes && self.mem == other.mem
    }
}

impl Eq for DynamicString {}

impl DynamicString {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: Vec::new(),
            mem: 0,
            refs: 1,
        }
    }

    #[must_use]
    pub const fn ref_count(&self) -> usize {
        self.refs
    }

    /// Increment the C reference count.
    ///
    /// # Panics
    ///
    /// Panics if the reference count would overflow.
    pub fn get_ref_c(&mut self) {
        assert!(
            self.refs < usize::MAX,
            "DStrGetRef reference count overflow"
        );
        self.refs += 1;
    }

    #[must_use]
    pub fn get_ref_option_c(strdes: Option<&mut Self>) -> Option<&mut Self> {
        match strdes {
            Some(string) => {
                string.get_ref_c();
                Some(string)
            }
            None => None,
        }
    }

    /// Release one C reference.
    ///
    /// C `DStrFree` decrements `refs` and frees the descriptor when the count
    /// reaches zero. Rust value methods cannot free `self`, so this returns
    /// `true` exactly when C would have freed the descriptor.
    ///
    /// # Panics
    ///
    /// Panics when the descriptor has already reached zero references,
    /// matching the C assertion that `refs >= 1`.
    #[must_use]
    pub fn release_ref_c(&mut self) -> bool {
        assert!(self.refs >= 1, "DStrFree requires at least one reference");
        self.refs -= 1;
        self.refs == 0
    }

    #[must_use]
    pub fn release_ref_option_c(strdes: Option<&mut Self>) -> bool {
        strdes.is_some_and(Self::release_ref_c)
    }

    pub fn append_str(&mut self, new_part: &str) {
        self.append_c_str_bytes(new_part.as_bytes());
    }

    pub fn append_c_str_bytes(&mut self, new_part: &[u8]) {
        let prefix = c_string_prefix(new_part);
        self.ensure_for_str_append(prefix.len());
        self.bytes.extend_from_slice(prefix);
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

fn c_string_prefix(bytes: &[u8]) -> &[u8] {
    let nul_pos = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    &bytes[..nul_pos]
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
        assert_eq!(string.ref_count(), 1);
        assert_eq!(string.view(), "");
        assert_eq!(string.view_bytes(), b"");
    }

    #[test]
    fn reference_helpers_preserve_c_counter_contract() {
        let mut string = DynamicString::new();

        string.get_ref_c();
        assert_eq!(string.ref_count(), 2);
        assert!(!string.release_ref_c());
        assert_eq!(string.ref_count(), 1);
        assert!(string.release_ref_c());
        assert_eq!(string.ref_count(), 0);
    }

    #[test]
    fn nullable_reference_helpers_preserve_null_noop() {
        assert!(DynamicString::get_ref_option_c(None).is_none());
        assert!(!DynamicString::release_ref_option_c(None));

        let mut string = DynamicString::new();
        assert!(DynamicString::get_ref_option_c(Some(&mut string)).is_some());
        assert_eq!(string.ref_count(), 2);
        assert!(!DynamicString::release_ref_option_c(Some(&mut string)));
        assert_eq!(string.ref_count(), 1);
    }

    #[test]
    fn clone_creates_independent_descriptor_refcount() {
        let mut string = DynamicString::new();
        string.append_str("abc");
        string.get_ref_c();

        let clone = string.clone();

        assert_eq!(clone, string);
        assert_eq!(string.ref_count(), 2);
        assert_eq!(clone.ref_count(), 1);
    }

    #[test]
    #[should_panic(expected = "DStrFree requires at least one reference")]
    fn releasing_after_final_reference_preserves_c_assertion() {
        let mut string = DynamicString::new();

        assert!(string.release_ref_c());
        let _ = string.release_ref_c();
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
    fn append_str_stops_at_first_nul_like_c_string() {
        let mut string = DynamicString::new();

        string.append_str("ab\0hidden");
        string.append_str("");

        assert_eq!(string.view_bytes(), b"ab");
        assert_eq!(string.allocated_mem(), DSTR_GROW);
    }

    #[test]
    fn append_c_str_bytes_accepts_non_utf8_and_stops_at_nul() {
        let mut string = DynamicString::new();

        string.append_c_str_bytes(&[0xff, b'a', 0, b'b']);

        assert_eq!(string.view_bytes(), &[0xff, b'a']);
        assert_eq!(string.allocated_mem(), DSTR_GROW);
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

        string.append_str_array_c(
            &[Some("alpha"), Some("beta\0suffix"), None, Some("ignored")],
            "::\0ignored",
        );

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
