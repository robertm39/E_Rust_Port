//! Port of `PCL2/pcl_positions`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::clauses::eqn_props::EqnSide;
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use std::fmt::Write as _;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pcl2Position {
    literal: i64,
    side: EqnSide,
    termpos: Vec<i64>,
}

impl Default for Pcl2Position {
    fn default() -> Self {
        Self::new()
    }
}

impl Pcl2Position {
    /// C `PCL2PosAlloc`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            literal: -1,
            side: EqnSide::NoSide,
            termpos: Vec::new(),
        }
    }

    #[must_use]
    pub const fn literal(&self) -> i64 {
        self.literal
    }

    #[must_use]
    pub const fn side(&self) -> EqnSide {
        self.side
    }

    #[must_use]
    pub fn termpos(&self) -> &[i64] {
        &self.termpos
    }

    /// C `pos->termposlen`.
    #[must_use]
    pub fn termpos_len(&self) -> usize {
        self.termpos.len()
    }

    /// C `PCL2PosParse`.
    ///
    /// # Errors
    ///
    /// Returns scanner diagnostics when the input does not match
    /// `<pos-int> [. L|R [ .<pos-int> ]*]`.
    pub fn parse(scanner: &mut Scanner) -> Result<Self, Diagnostic> {
        let mut position = Self::new();
        position.literal = parse_pos_int_as_long(scanner)?;
        if scanner.test_tok(TokenType::FULLSTOP) {
            scanner.next_token()?;
            scanner.check_id("L|R")?;
            position.side = if scanner.test_id("L") {
                EqnSide::LeftSide
            } else {
                EqnSide::RightSide
            };
            scanner.next_token()?;

            while scanner.test_tok(TokenType::FULLSTOP) {
                scanner.next_token()?;
                position.termpos.push(parse_pos_int_as_long(scanner)?);
            }
        }
        Ok(position)
    }

    /// C `PCL2PosPrint`.
    ///
    /// The C printer omits `.` separators before term-position components, so
    /// `3.L.4.5` renders as `3.L45`.
    ///
    /// # Panics
    ///
    /// Panics if the internal side is `BothSides`, matching the C assertion
    /// that a printed non-empty side is either left or right.
    #[must_use]
    pub fn print_string(&self) -> String {
        let mut output = String::new();
        let _ = write!(output, "{}", self.literal);
        match self.side {
            EqnSide::NoSide => {}
            EqnSide::LeftSide | EqnSide::RightSide => {
                let side = if self.side == EqnSide::LeftSide {
                    'L'
                } else {
                    'R'
                };
                let _ = write!(output, ".{side}");
                for component in &self.termpos {
                    let _ = write!(output, "{component}");
                }
            }
            EqnSide::BothSides => panic!("PCL2 position side must be left or right when printed"),
        }
        output
    }
}

fn parse_pos_int_as_long(scanner: &mut Scanner) -> Result<i64, Diagnostic> {
    scanner.check_tok(TokenType::POS_INT)?;
    let numval = scanner.current_token().numval();
    let value =
        i64::try_from(numval).map_err(|_| current_error(scanner, "Long integer overflow"))?;
    scanner.next_token()?;
    Ok(value)
}

fn current_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): {message}",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal()
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::Pcl2Position;
    use crate::clauses::eqn_props::EqnSide;
    use crate::inout::scanner::{Scanner, TokenType};

    fn parse(source: &str) -> Pcl2Position {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        Pcl2Position::parse(&mut scanner).unwrap()
    }

    #[test]
    fn alloc_initializes_c_defaults() {
        let position = Pcl2Position::new();
        assert_eq!(position.literal(), -1);
        assert_eq!(position.side(), EqnSide::NoSide);
        assert_eq!(position.termpos_len(), 0);
        assert!(position.termpos().is_empty());
    }

    #[test]
    fn parses_literal_only_position() {
        let position = parse("7");
        assert_eq!(position.literal(), 7);
        assert_eq!(position.side(), EqnSide::NoSide);
        assert!(position.termpos().is_empty());
        assert_eq!(position.print_string(), "7");
    }

    #[test]
    fn parses_sided_positions() {
        let left = parse("7.L");
        assert_eq!(left.literal(), 7);
        assert_eq!(left.side(), EqnSide::LeftSide);
        assert!(left.termpos().is_empty());
        assert_eq!(left.print_string(), "7.L");

        let right = parse("8.R");
        assert_eq!(right.literal(), 8);
        assert_eq!(right.side(), EqnSide::RightSide);
        assert!(right.termpos().is_empty());
        assert_eq!(right.print_string(), "8.R");
    }

    #[test]
    fn parses_term_path_and_preserves_c_print_separator_bug() {
        let position = parse("3.L.4.5");
        assert_eq!(position.literal(), 3);
        assert_eq!(position.side(), EqnSide::LeftSide);
        assert_eq!(position.termpos(), [4, 5]);
        assert_eq!(position.termpos_len(), 2);
        assert_eq!(position.print_string(), "3.L45");
    }

    #[test]
    fn rejects_invalid_side_without_consuming_the_side_token() {
        let mut scanner = Scanner::from_user_string("3.X.4", false).unwrap();
        let error = Pcl2Position::parse(&mut scanner).unwrap_err();
        assert!(error.message().contains("Identifier (L|R) expected"));
        assert_eq!(scanner.current_token().literal(), "X");
    }

    #[test]
    fn rejects_negative_literal_like_c_positive_integer_token_check() {
        let mut scanner = Scanner::from_user_string("-1.L", false).unwrap();
        let error = Pcl2Position::parse(&mut scanner).unwrap_err();
        assert!(error.message().contains("Integer"));
        assert!(scanner.test_tok(TokenType::HYPHEN));
    }
}
