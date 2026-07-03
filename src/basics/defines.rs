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

#[cfg(test)]
mod tests {
    use super::{
        bool_to_str, c_cmp, logical_equiv, logical_xor, IntOrP, IntOrPInt, INT_OR_P_MEM, KILO,
        LONG_MEM, MEGA,
    };
    use std::mem::size_of;

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
        assert!(logical_xor(true, false));
        assert!(!logical_xor(true, true));
        assert!(logical_equiv(false, false));
        assert!(!logical_equiv(false, true));
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
}
