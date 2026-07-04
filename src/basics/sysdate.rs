use std::fmt;

pub type SysDateRaw = i64;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct SysDate(SysDateRaw);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SysDateIncrement {
    Advanced,
    CAssertionWouldFail,
    Overflow,
}

impl SysDate {
    pub const CREATION_RAW: SysDateRaw = 0;
    pub const INVALID_RAW: SysDateRaw = -1;

    #[must_use]
    pub const fn creation_time() -> Self {
        Self(Self::CREATION_RAW)
    }

    #[must_use]
    pub const fn invalid_time() -> Self {
        Self(Self::INVALID_RAW)
    }

    #[must_use]
    pub const fn from_raw(raw: SysDateRaw) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn raw(self) -> SysDateRaw {
        self.0
    }

    #[must_use]
    pub const fn is_invalid(self) -> bool {
        self.0 == Self::INVALID_RAW
    }

    #[must_use]
    pub const fn is_creation_date(self) -> bool {
        self.0 == Self::CREATION_RAW
    }

    #[must_use]
    pub const fn is_earlier_than(self, other: Self) -> bool {
        self.0 < other.0
    }

    #[must_use]
    pub const fn is_equal_to(self, other: Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub const fn maximum(self, other: Self) -> Self {
        if self.0 >= other.0 {
            self
        } else {
            other
        }
    }

    #[must_use]
    pub fn increment(&mut self) -> SysDateIncrement {
        let Some(next) = self.0.checked_add(1) else {
            return SysDateIncrement::Overflow;
        };
        self.0 = next;
        if self.is_creation_date() {
            SysDateIncrement::CAssertionWouldFail
        } else {
            SysDateIncrement::Advanced
        }
    }

    /// Increment with the C `SysDateInc` assertion-shaped contract.
    ///
    /// # Panics
    ///
    /// Panics if the incremented date is the creation-time sentinel, matching
    /// the C macro's post-increment `assert(*(sd))`. Also panics on signed
    /// overflow instead of importing C undefined behavior.
    pub fn increment_c(&mut self) {
        match self.increment() {
            SysDateIncrement::Advanced => {}
            SysDateIncrement::CAssertionWouldFail => {
                panic!("SysDateInc advanced to creation time")
            }
            SysDateIncrement::Overflow => panic!("SysDateInc date overflow"),
        }
    }

    #[must_use]
    pub fn unsigned_c_long_bits(self) -> u64 {
        u64::from_ne_bytes(self.0.to_ne_bytes())
    }

    #[must_use]
    pub fn print_string(self) -> String {
        format!("{:>5}", self.unsigned_c_long_bits())
    }

    pub fn write_to(self, output: &mut impl fmt::Write) -> fmt::Result {
        write!(output, "{:>5}", self.unsigned_c_long_bits())
    }
}

impl From<SysDateRaw> for SysDate {
    fn from(value: SysDateRaw) -> Self {
        Self::from_raw(value)
    }
}

impl From<SysDate> for SysDateRaw {
    fn from(value: SysDate) -> Self {
        value.raw()
    }
}

#[cfg(test)]
mod tests {
    use super::{SysDate, SysDateIncrement};

    #[test]
    fn sentinel_values_and_comparisons_match_c_macros() {
        let creation = SysDate::creation_time();
        let invalid = SysDate::invalid_time();
        let later = SysDate::from_raw(7);

        assert_eq!(creation.raw(), 0);
        assert_eq!(invalid.raw(), -1);
        assert!(invalid.is_invalid());
        assert!(creation.is_creation_date());
        assert!(invalid.is_earlier_than(creation));
        assert!(creation.is_equal_to(SysDate::from_raw(0)));
        assert_eq!(creation.maximum(later), later);
    }

    #[test]
    fn increment_mutates_before_reporting_c_assertion_failure() {
        let mut date = SysDate::creation_time();
        assert_eq!(date.increment(), SysDateIncrement::Advanced);
        assert_eq!(date.raw(), 1);

        let mut invalid = SysDate::invalid_time();
        assert_eq!(invalid.increment(), SysDateIncrement::CAssertionWouldFail);
        assert_eq!(invalid, SysDate::creation_time());
    }

    #[test]
    fn increment_c_matches_c_macro_for_ordinary_dates() {
        let mut date = SysDate::creation_time();

        date.increment_c();

        assert_eq!(date.raw(), 1);
    }

    #[test]
    #[should_panic(expected = "SysDateInc advanced to creation time")]
    fn increment_c_panics_after_invalid_sentinel_increment_like_c_assertion() {
        let mut date = SysDate::invalid_time();

        date.increment_c();
    }

    #[test]
    #[should_panic(expected = "SysDateInc date overflow")]
    fn increment_c_panics_on_overflow_instead_of_importing_c_undefined_behavior() {
        let mut date = SysDate::from_raw(i64::MAX);

        date.increment_c();
    }

    #[test]
    fn increment_reports_overflow_without_wrapping() {
        let mut date = SysDate::from_raw(i64::MAX);
        assert_eq!(date.increment(), SysDateIncrement::Overflow);
        assert_eq!(date.raw(), i64::MAX);
    }

    #[test]
    fn print_string_matches_unsigned_long_formatting_shape() {
        assert_eq!(SysDate::creation_time().print_string(), "    0");
        assert_eq!(SysDate::from_raw(42).print_string(), "   42");
        assert_eq!(
            SysDate::invalid_time().print_string(),
            "18446744073709551615"
        );

        let mut output = String::new();
        SysDate::from_raw(5).write_to(&mut output).unwrap();
        assert_eq!(output, "    5");
    }
}
