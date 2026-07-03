use std::cmp::Ordering;

pub const PO_COMPARE_SYMBOLS: [&str; 5] = ["*u*", "=/=", " = ", " > ", " < "];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CompareResult {
    Unknown = 0,
    Uncomparable = 1,
    Equal = 2,
    Greater = 3,
    Lesser = 4,
    NotGreaterEqual = 5,
    NotLessEqual = 6,
}

impl CompareResult {
    #[must_use]
    pub const fn from_c_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Unknown),
            1 => Some(Self::Uncomparable),
            2 => Some(Self::Equal),
            3 => Some(Self::Greater),
            4 => Some(Self::Lesser),
            5 => Some(Self::NotGreaterEqual),
            6 => Some(Self::NotLessEqual),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub const fn inverse(self) -> Option<Self> {
        match self {
            Self::Equal => Some(Self::Equal),
            Self::Uncomparable => Some(Self::Uncomparable),
            Self::Greater => Some(Self::Lesser),
            Self::Lesser => Some(Self::Greater),
            Self::NotGreaterEqual => Some(Self::NotLessEqual),
            Self::NotLessEqual => Some(Self::NotGreaterEqual),
            Self::Unknown => None,
        }
    }

    /// Return the inverse relation with the C `POInverseRelation` assertion.
    ///
    /// # Panics
    ///
    /// Panics for `Unknown`, matching the C helper's default `assert(false)`
    /// branch.
    #[must_use]
    pub fn inverse_c(self) -> Self {
        match self.inverse() {
            Some(value) => value,
            None => panic!("POInverseRelation called with unknown relation"),
        }
    }

    #[must_use]
    pub fn symbol(self) -> Option<&'static str> {
        PO_COMPARE_SYMBOLS.get(usize::from(self.c_value())).copied()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum HoOrderKind {
    LfhoOrder = 0,
    LambdaOrder = 1,
}

impl HoOrderKind {
    #[must_use]
    pub const fn from_c_value(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::LfhoOrder),
            1 => Some(Self::LambdaOrder),
            _ => None,
        }
    }

    #[must_use]
    pub const fn c_value(self) -> u8 {
        self as u8
    }
}

#[must_use]
pub const fn q_to_part_i32(result: i32) -> CompareResult {
    if result < 0 {
        CompareResult::Lesser
    } else if result > 0 {
        CompareResult::Greater
    } else {
        CompareResult::Equal
    }
}

#[must_use]
pub const fn ordering_to_part(ordering: Ordering) -> CompareResult {
    match ordering {
        Ordering::Less => CompareResult::Lesser,
        Ordering::Equal => CompareResult::Equal,
        Ordering::Greater => CompareResult::Greater,
    }
}

#[cfg(test)]
mod tests {
    use super::{ordering_to_part, q_to_part_i32, CompareResult, HoOrderKind, PO_COMPARE_SYMBOLS};
    use std::cmp::Ordering;

    #[test]
    fn compare_result_discriminants_match_c_enum() {
        assert_eq!(CompareResult::Unknown.c_value(), 0);
        assert_eq!(CompareResult::Uncomparable.c_value(), 1);
        assert_eq!(CompareResult::Equal.c_value(), 2);
        assert_eq!(CompareResult::Greater.c_value(), 3);
        assert_eq!(CompareResult::Lesser.c_value(), 4);
        assert_eq!(CompareResult::NotGreaterEqual.c_value(), 5);
        assert_eq!(CompareResult::NotLessEqual.c_value(), 6);

        for value in 0_u8..=6 {
            assert_eq!(
                CompareResult::from_c_value(value).map(CompareResult::c_value),
                Some(value)
            );
        }
        assert_eq!(CompareResult::from_c_value(7), None);
    }

    #[test]
    fn inverse_relation_matches_c_switch_cases() {
        assert_eq!(CompareResult::Equal.inverse_c(), CompareResult::Equal);
        assert_eq!(
            CompareResult::Uncomparable.inverse_c(),
            CompareResult::Uncomparable
        );
        assert_eq!(CompareResult::Greater.inverse_c(), CompareResult::Lesser);
        assert_eq!(CompareResult::Lesser.inverse_c(), CompareResult::Greater);
        assert_eq!(
            CompareResult::NotGreaterEqual.inverse_c(),
            CompareResult::NotLessEqual
        );
        assert_eq!(
            CompareResult::NotLessEqual.inverse_c(),
            CompareResult::NotGreaterEqual
        );
        assert_eq!(CompareResult::Unknown.inverse(), None);
    }

    #[test]
    #[should_panic(expected = "POInverseRelation called with unknown relation")]
    fn inverse_c_panics_on_unknown_like_c_assertion() {
        let _value = CompareResult::Unknown.inverse_c();
    }

    #[test]
    fn compare_symbols_cover_only_c_array_entries() {
        assert_eq!(PO_COMPARE_SYMBOLS, ["*u*", "=/=", " = ", " > ", " < "]);
        assert_eq!(CompareResult::Unknown.symbol(), Some("*u*"));
        assert_eq!(CompareResult::Uncomparable.symbol(), Some("=/="));
        assert_eq!(CompareResult::Equal.symbol(), Some(" = "));
        assert_eq!(CompareResult::Greater.symbol(), Some(" > "));
        assert_eq!(CompareResult::Lesser.symbol(), Some(" < "));
        assert_eq!(CompareResult::NotGreaterEqual.symbol(), None);
        assert_eq!(CompareResult::NotLessEqual.symbol(), None);
    }

    #[test]
    fn quasi_ordering_conversion_matches_macro() {
        assert_eq!(q_to_part_i32(-10), CompareResult::Lesser);
        assert_eq!(q_to_part_i32(0), CompareResult::Equal);
        assert_eq!(q_to_part_i32(10), CompareResult::Greater);
        assert_eq!(ordering_to_part(Ordering::Less), CompareResult::Lesser);
        assert_eq!(ordering_to_part(Ordering::Equal), CompareResult::Equal);
        assert_eq!(ordering_to_part(Ordering::Greater), CompareResult::Greater);
    }

    #[test]
    fn higher_order_kind_discriminants_match_c_enum() {
        assert_eq!(HoOrderKind::LfhoOrder.c_value(), 0);
        assert_eq!(HoOrderKind::LambdaOrder.c_value(), 1);
        assert_eq!(HoOrderKind::from_c_value(0), Some(HoOrderKind::LfhoOrder));
        assert_eq!(HoOrderKind::from_c_value(1), Some(HoOrderKind::LambdaOrder));
        assert_eq!(HoOrderKind::from_c_value(2), None);
    }
}
