//! PCL2 proof-object support ported from E's `PCL2` units.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};

pub mod expressions;
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

pub(crate) fn current_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!(
            "{}(just read '{}'): {message}",
            token_pos_rep(scanner.current_token()),
            scanner.current_token().literal()
        ),
    )
}

pub(crate) fn strip_quote_core(bytes: &[u8]) -> Result<String, Diagnostic> {
    if bytes.len() < 2 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Quoted string literal is too short",
        ));
    }
    Ok(String::from_utf8_lossy(&bytes[1..bytes.len() - 1]).into_owned())
}
