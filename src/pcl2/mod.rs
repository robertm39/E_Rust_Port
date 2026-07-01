//! PCL2 proof-object support ported from E's `PCL2` units.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};

pub mod idents;
pub mod positions;

pub(crate) fn parse_pos_int_as_long(scanner: &mut Scanner) -> Result<i64, Diagnostic> {
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
