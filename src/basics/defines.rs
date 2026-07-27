use std::cmp::Ordering;
use std::mem::size_of;

pub type IntOrPInt = i64;

pub const DEFAULT_COMCHAR_RAW: &str = "%";
pub const DEFAULT_COMCHAR_DIRECT: &str = "%%";
pub const KILO: u64 = 1024;
pub const MEGA: u64 = KILO * KILO;
pub const LONG_MEM: usize = size_of::<IntOrPInt>();
pub const INT_OR_P_MEM: usize = if LONG_MEM > size_of::<usize>() {
    LONG_MEM
} else {
    size_of::<usize>()
};

#[must_use]
pub const fn bool_to_str(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[must_use]
pub fn c_cmp<T: Ord + ?Sized>(left: &T, right: &T) -> i32 {
    match left.cmp(right) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

/// Return the C `MAX` macro result.
///
/// The C macro evaluates both operands once and returns the right operand on
/// ties because it uses a strict `>` comparison.
#[must_use]
pub fn c_max<T: PartialOrd>(left: T, right: T) -> T {
    if left > right {
        left
    } else {
        right
    }
}

/// Return the C `MIN` macro result.
///
/// The C macro evaluates both operands once and returns the right operand on
/// ties because it uses a strict `<` comparison.
#[must_use]
pub fn c_min<T: PartialOrd>(left: T, right: T) -> T {
    if left < right {
        left
    } else {
        right
    }
}

/// Swap two same-typed Rust values with the C `SWAP` macro's value exchange.
pub fn c_swap<T>(left: &mut T, right: &mut T) {
    std::mem::swap(left, right);
}

/// Return the C `ABS` macro value for a signed `long`-shaped integer.
///
/// # Panics
///
/// Panics for `i64::MIN`. The C macro negates that value with signed overflow,
/// which is undefined behavior.
#[must_use]
pub fn c_abs(value: IntOrPInt) -> IntOrPInt {
    if value > 0 {
        value
    } else {
        value
            .checked_neg()
            .unwrap_or_else(|| panic!("ABS overflow for minimum signed value"))
    }
}

/// Write a C string-shaped message to a raw file descriptor.
///
/// This mirrors `clb_defines.h`'s `WriteStr`: it computes the message length
/// up to the first NUL byte, performs one low-level write call, and returns the
/// raw C result converted to `usize`. A failed write therefore returns
/// `usize::MAX`, matching C's signed-to-unsigned return conversion.
#[must_use]
pub fn write_str_to_fd(fd: i32, message: &str) -> usize {
    fd_write::write(fd, c_string_prefix(message))
}

/// Return the C `TSTPOUT(file, msg)` line shape for formatted output.
#[must_use]
pub fn tstp_out_string(status: &str) -> String {
    format!(
        "{} SZS status {}\n",
        DEFAULT_COMCHAR_RAW,
        c_string_prefix_str(status)
    )
}

/// Write the C `TSTPOUTFD(fd, msg)` shape directly to a raw descriptor.
///
/// The default C build uses `COMCHAR == "%%"` for printf-format strings.
/// `TSTPOUTFD` writes that string through `WriteStr`, so the direct descriptor
/// output intentionally contains two percent signs.
pub fn tstp_out_fd(fd: i32, status: &str) {
    for chunk in tstp_out_fd_chunks(status) {
        let _ = write_str_to_fd(fd, chunk);
    }
}

fn tstp_out_fd_chunks(status: &str) -> [&str; 4] {
    [
        DEFAULT_COMCHAR_DIRECT,
        " SZS status ",
        c_string_prefix_str(status),
        "\n",
    ]
}

#[cfg(test)]
fn tstp_out_fd_string(status: &str) -> String {
    tstp_out_fd_chunks(status).concat()
}

fn c_string_prefix(message: &str) -> &[u8] {
    let bytes = message.as_bytes();
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    &bytes[..end]
}

fn c_string_prefix_str(message: &str) -> &str {
    let end = message.find('\0').unwrap_or(message.len());
    &message[..end]
}

#[must_use]
pub const fn logical_xor(left: bool, right: bool) -> bool {
    left != right
}

#[must_use]
pub const fn logical_equiv(left: bool, right: bool) -> bool {
    left == right
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntOrP<P> {
    Int(IntOrPInt),
    Pointer(P),
}

impl<P> IntOrP<P> {
    #[must_use]
    pub const fn int(value: IntOrPInt) -> Self {
        Self::Int(value)
    }

    #[must_use]
    pub const fn pointer(value: P) -> Self {
        Self::Pointer(value)
    }

    #[must_use]
    pub const fn is_int(&self) -> bool {
        matches!(self, Self::Int(_))
    }

    #[must_use]
    pub const fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer(_))
    }

    #[must_use]
    pub const fn as_int(&self) -> Option<IntOrPInt> {
        match self {
            Self::Int(value) => Some(*value),
            Self::Pointer(_) => None,
        }
    }

    #[must_use]
    pub const fn as_pointer(&self) -> Option<&P> {
        match self {
            Self::Int(_) => None,
            Self::Pointer(value) => Some(value),
        }
    }

    pub fn into_int(self) -> Option<IntOrPInt> {
        match self {
            Self::Int(value) => Some(value),
            Self::Pointer(_) => None,
        }
    }

    pub fn into_pointer(self) -> Option<P> {
        match self {
            Self::Int(_) => None,
            Self::Pointer(value) => Some(value),
        }
    }
}

// Allowed external shared-library boundary: `WriteStr` is a raw descriptor
// helper, so Umlaut calls the platform C runtime's one-shot write ABI
// behind a safe, C-shaped wrapper.
#[cfg(unix)]
#[allow(unsafe_code)]
mod fd_write {
    use std::ffi::c_void;

    unsafe extern "C" {
        #[link_name = "write"]
        fn libc_write(fd: i32, buffer: *const c_void, count: usize) -> isize;
    }

    pub(super) fn write(fd: i32, bytes: &[u8]) -> usize {
        // SAFETY: bytes points to a live buffer for exactly bytes.len() bytes.
        // The fd is intentionally raw to match C `WriteStr`.
        let result = unsafe { libc_write(fd, bytes.as_ptr().cast::<c_void>(), bytes.len()) };
        usize::try_from(result).unwrap_or(usize::MAX)
    }
}

// Allowed external DLL boundary: on MSVC Windows, the compatibility fd surface
// is a UCRT file descriptor, so `WriteStr` uses UCRT `_write`.
#[cfg(all(windows, target_env = "msvc"))]
#[allow(unsafe_code)]
mod fd_write {
    use std::ffi::c_void;

    #[link(name = "ucrt")]
    unsafe extern "C" {
        fn _write(fd: i32, buffer: *const c_void, count: u32) -> i32;
    }

    pub(super) fn write(fd: i32, bytes: &[u8]) -> usize {
        if fd < 0 {
            return usize::MAX;
        }
        let count = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        let capped_len = usize::try_from(count).unwrap_or(bytes.len());
        let capped = &bytes[..capped_len];
        // SAFETY: capped points to a live buffer for exactly count bytes. The
        // fd is intentionally raw to match C `WriteStr`.
        let result = unsafe { _write(fd, capped.as_ptr().cast::<c_void>(), count) };
        usize::try_from(result).unwrap_or(usize::MAX)
    }
}

// Allowed external DLL boundary: on GNU Windows, the compatibility fd surface
// is a MinGW/MSVCRT file descriptor, so `WriteStr` uses MSVCRT `_write`.
#[cfg(all(windows, target_env = "gnu"))]
#[allow(unsafe_code)]
mod fd_write {
    use std::ffi::c_void;

    #[link(name = "msvcrt")]
    unsafe extern "C" {
        fn _write(fd: i32, buffer: *const c_void, count: u32) -> i32;
    }

    pub(super) fn write(fd: i32, bytes: &[u8]) -> usize {
        if fd < 0 {
            return usize::MAX;
        }
        let count = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        let capped_len = usize::try_from(count).unwrap_or(bytes.len());
        let capped = &bytes[..capped_len];
        // SAFETY: capped points to a live buffer for exactly count bytes. The
        // fd is intentionally raw to match C `WriteStr`.
        let result = unsafe { _write(fd, capped.as_ptr().cast::<c_void>(), count) };
        usize::try_from(result).unwrap_or(usize::MAX)
    }
}

#[cfg(all(windows, not(any(target_env = "msvc", target_env = "gnu"))))]
mod fd_write {
    pub(super) fn write(_fd: i32, bytes: &[u8]) -> usize {
        if bytes.is_empty() {
            0
        } else {
            usize::MAX
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod fd_write {
    pub(super) fn write(_fd: i32, bytes: &[u8]) -> usize {
        if bytes.is_empty() {
            0
        } else {
            usize::MAX
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bool_to_str, c_abs, c_cmp, c_max, c_min, c_string_prefix, c_swap, logical_equiv,
        logical_xor, tstp_out_fd, tstp_out_fd_string, tstp_out_string, write_str_to_fd, IntOrP,
        IntOrPInt, INT_OR_P_MEM, KILO, LONG_MEM, MEGA,
    };
    use std::mem::size_of;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TaggedValue {
        key: i32,
        tag: &'static str,
    }

    impl PartialOrd for TaggedValue {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.key.cmp(&other.key))
        }
    }

    #[test]
    fn constants_match_c_default_defines() {
        assert_eq!(KILO, 1024);
        assert_eq!(MEGA, 1_048_576);
        assert_eq!(LONG_MEM, size_of::<IntOrPInt>());
        assert_eq!(INT_OR_P_MEM, LONG_MEM.max(size_of::<usize>()));
    }

    #[test]
    fn c_macro_helpers_match_boolean_and_comparison_shapes() {
        assert_eq!(bool_to_str(true), "true");
        assert_eq!(bool_to_str(false), "false");
        assert_eq!(c_cmp(&1, &2), -1);
        assert_eq!(c_cmp(&2, &2), 0);
        assert_eq!(c_cmp(&3, &2), 1);
        assert_eq!(c_max(1, 2), 2);
        assert_eq!(c_max(3, 2), 3);
        assert_eq!(c_min(1, 2), 1);
        assert_eq!(c_min(3, 2), 2);
        assert_eq!(c_abs(5), 5);
        assert_eq!(c_abs(0), 0);
        assert_eq!(c_abs(-5), 5);
        assert!(logical_xor(true, false));
        assert!(!logical_xor(true, true));
        assert!(logical_equiv(false, false));
        assert!(!logical_equiv(false, true));
    }

    #[test]
    fn c_max_and_min_return_right_operand_on_ties_like_strict_c_macros() {
        let left = TaggedValue {
            key: 7,
            tag: "left",
        };
        let right = TaggedValue {
            key: 7,
            tag: "right",
        };

        assert_eq!(c_max(left.clone(), right.clone()).tag, "right");
        assert_eq!(c_min(left, right).tag, "right");
    }

    #[test]
    fn c_max_and_min_preserve_c_nan_branch_shape() {
        assert!(c_max(f64::NAN, 1.0).is_finite());
        assert!(c_min(f64::NAN, 1.0).is_finite());
        assert!(c_max(1.0, f64::NAN).is_nan());
        assert!(c_min(1.0, f64::NAN).is_nan());
    }

    #[test]
    fn c_swap_exchanges_values_like_c_macro() {
        let mut left = "left";
        let mut right = "right";

        c_swap(&mut left, &mut right);

        assert_eq!(left, "right");
        assert_eq!(right, "left");
    }

    #[test]
    #[should_panic(expected = "ABS overflow for minimum signed value")]
    fn c_abs_panics_on_minimum_signed_value_instead_of_importing_undefined_behavior() {
        let _value = c_abs(IntOrPInt::MIN);
    }

    #[test]
    fn int_or_p_keeps_checked_tagged_payloads() {
        let int_value = IntOrP::<&str>::int(7);
        assert!(int_value.is_int());
        assert!(!int_value.is_pointer());
        assert_eq!(int_value.as_int(), Some(7));
        assert_eq!(int_value.as_pointer(), None);
        assert_eq!(int_value.into_int(), Some(7));

        let pointer_value = IntOrP::pointer("payload");
        assert!(!pointer_value.is_int());
        assert!(pointer_value.is_pointer());
        assert_eq!(pointer_value.as_int(), None);
        assert_eq!(pointer_value.as_pointer(), Some(&"payload"));
        assert_eq!(pointer_value.into_pointer(), Some("payload"));
    }

    #[test]
    fn c_string_prefix_stops_at_first_nul_like_strlen() {
        assert_eq!(c_string_prefix("abc"), b"abc");
        assert_eq!(c_string_prefix("abc\0def"), b"abc");
    }

    #[test]
    fn write_str_to_fd_reports_failed_raw_descriptor_like_c_unsigned_return() {
        assert_eq!(write_str_to_fd(-1, "x"), usize::MAX);
    }

    #[test]
    fn tstp_status_helpers_preserve_formatted_and_direct_comment_prefixes() {
        assert_eq!(tstp_out_string("Theorem"), "% SZS status Theorem\n");
        assert_eq!(
            tstp_out_fd_string("ResourceOut"),
            "%% SZS status ResourceOut\n"
        );
        assert_eq!(tstp_out_string("Sat\0tail"), "% SZS status Sat\n");
        assert_eq!(tstp_out_fd_string("Sat\0tail"), "%% SZS status Sat\n");
    }

    #[test]
    fn tstp_out_fd_ignores_failed_raw_descriptor_like_c_macro() {
        tstp_out_fd(-1, "ResourceOut");
    }
}
