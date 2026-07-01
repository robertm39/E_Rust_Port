//! Port of `PCL2/pcl_idents`.

use crate::basics::error::Diagnostic;
use crate::inout::scanner::{Scanner, TokenType};
use crate::pcl2::parse_pos_int_as_long;
use std::fmt::Write as _;

pub const NO_PCL_ID_ELEMENT: i64 = -1;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PclId {
    elements: Vec<i64>,
}

impl PclId {
    /// C `PCLIdAlloc`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    #[must_use]
    pub fn elements(&self) -> &[i64] {
        &self.elements
    }

    /// C `PCLIdParse`.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics when the input is not a fullstop-separated
    /// sequence of positive-integer tokens.
    pub fn parse(scanner: &mut Scanner) -> Result<Self, Diagnostic> {
        scanner.check_tok(TokenType::POS_INT)?;
        let mut id = Self::new();
        id.elements.push(parse_pos_int_as_long(scanner)?);
        while scanner.test_tok(TokenType::FULLSTOP) {
            scanner.next_token()?;
            id.elements.push(parse_pos_int_as_long(scanner)?);
        }
        Ok(id)
    }

    /// C `PCLIdPrintFormatted`.
    ///
    /// # Panics
    ///
    /// Panics when called for an empty allocated-but-uninitialized identifier,
    /// matching the C assertion that element zero is present and non-negative.
    #[must_use]
    pub fn print_formatted_string(&self, formatted: bool) -> String {
        let mut output = String::new();
        let mut iter = self.elements.iter();
        let first = *iter
            .next()
            .unwrap_or_else(|| panic!("PCL identifier must have at least one element"));
        assert!(
            first != NO_PCL_ID_ELEMENT && first >= 0,
            "PCL identifier first element must be non-negative"
        );
        if formatted {
            let _ = write!(output, "{first:7}");
        } else {
            let _ = write!(output, "{first}");
        }
        for element in iter {
            let _ = write!(output, ".{element}");
        }
        output
    }

    /// C `PCLIdPrint`.
    ///
    /// # Panics
    ///
    /// Panics when called for an empty allocated-but-uninitialized identifier,
    /// matching the C assertion that element zero is present and non-negative.
    #[must_use]
    pub fn print_string(&self) -> String {
        self.print_formatted_string(false)
    }

    /// C `PCLIdPrintTSTP`.
    ///
    /// # Panics
    ///
    /// Panics when called for an empty allocated-but-uninitialized identifier,
    /// matching the C assertion that element zero is present and non-negative.
    #[must_use]
    pub fn print_tstp_string(&self) -> String {
        let mut iter = self.elements.iter();
        let first = *iter
            .next()
            .unwrap_or_else(|| panic!("PCL identifier must have at least one element"));
        assert!(
            first != NO_PCL_ID_ELEMENT && first >= 0,
            "PCL identifier first element must be non-negative"
        );
        let Some(second) = iter.next() else {
            return first.to_string();
        };

        let mut output = format!("pclid{first}_{second}");
        for element in iter {
            let _ = write!(output, "_{element}");
        }
        output
    }

    /// C `PCLIdCompare`.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn compare_c_value(&self, other: &Self) -> i32 {
        for index in 0.. {
            let left = self
                .elements
                .get(index)
                .copied()
                .unwrap_or(NO_PCL_ID_ELEMENT);
            let right = other
                .elements
                .get(index)
                .copied()
                .unwrap_or(NO_PCL_ID_ELEMENT);
            if left == NO_PCL_ID_ELEMENT && right == NO_PCL_ID_ELEMENT {
                return 0;
            }
            let result = left.wrapping_sub(right);
            if result != 0 {
                // C returns `(int)(e1-e2)` here; keep that truncating surface.
                return result as i32;
            }
        }
        unreachable!("PCL identifier comparison must return at shared sentinel");
    }
}

#[cfg(test)]
mod tests {
    use super::PclId;
    use crate::inout::scanner::{Scanner, TokenType};

    fn parse(source: &str) -> PclId {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        PclId::parse(&mut scanner).unwrap()
    }

    #[test]
    fn alloc_matches_empty_pdarray_shape() {
        let id = PclId::new();
        assert!(id.elements().is_empty());
    }

    #[test]
    fn parses_fullstop_separated_positive_integer_list() {
        let mut scanner = Scanner::from_user_string("12.3.45 tail", false).unwrap();
        let id = PclId::parse(&mut scanner).unwrap();
        assert_eq!(id.elements(), [12, 3, 45]);
        assert_eq!(scanner.current_token().literal(), "tail");
    }

    #[test]
    fn prints_plain_and_formatted_identifiers() {
        let id = parse("12.3.45");
        assert_eq!(id.print_string(), "12.3.45");
        assert_eq!(id.print_formatted_string(true), "     12.3.45");
    }

    #[test]
    fn prints_tstp_singletons_and_compound_ids() {
        assert_eq!(parse("7").print_tstp_string(), "7");
        assert_eq!(parse("7.8.9").print_tstp_string(), "pclid7_8_9");
    }

    #[test]
    fn compares_ids_with_c_sentinel_lexicographic_shape() {
        assert_eq!(parse("1.2").compare_c_value(&parse("1.2")), 0);
        assert!(parse("1.2").compare_c_value(&parse("1.3")) < 0);
        assert!(parse("1.3").compare_c_value(&parse("1.2")) > 0);
        assert_eq!(parse("1").compare_c_value(&parse("1.0")), -1);
    }

    #[test]
    fn rejects_negative_first_element_like_c_positive_integer_check() {
        let mut scanner = Scanner::from_user_string("-1", false).unwrap();
        let error = PclId::parse(&mut scanner).unwrap_err();
        assert!(error.message().contains("Integer"));
        assert!(scanner.test_tok(TokenType::HYPHEN));
    }
}
