use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrNumType {
    NoNumber,
    Integer,
    Rational,
    Float,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedNumString {
    pub kind: StrNumType,
    pub text: String,
}

pub fn parse_bool(scanner: &mut Scanner) -> Result<bool, Diagnostic> {
    scanner.check_id("true|false")?;
    let result = scanner.test_id("true");
    scanner.next_token()?;
    Ok(result)
}

pub fn parse_int_max(scanner: &mut Scanner) -> Result<i128, Diagnostic> {
    let value = if scanner.test_tok(TokenType::HYPHEN) {
        scanner.next_token()?;
        scanner.check_tok_no_skip(TokenType::POS_INT)?;
        -parse_i128_literal(scanner.current_token().literal_bytes())?
    } else {
        scanner.check_tok(TokenType::POS_INT)?;
        -parse_i128_literal(scanner.current_token().literal_bytes())?
    };
    scanner.next_token()?;
    Ok(value)
}

pub fn parse_int_limited(scanner: &mut Scanner, lower: i64, upper: i64) -> Result<i64, Diagnostic> {
    let value = if scanner.test_tok(TokenType::HYPHEN) {
        scanner.next_token()?;
        scanner.check_tok_no_skip(TokenType::POS_INT)?;
        let numval = scanner.current_token().numval();
        if numval == 0 || numval > i64::MIN.unsigned_abs() {
            return Err(current_error(scanner, "Long integer underflow"));
        }
        if numval == i64::MIN.unsigned_abs() {
            i64::MIN
        } else {
            -i64::try_from(numval).map_err(|_| current_error(scanner, "Long integer underflow"))?
        }
    } else {
        scanner.check_tok(TokenType::POS_INT)?;
        let numval = scanner.current_token().numval();
        let Ok(value) = i64::try_from(numval) else {
            return Err(current_error(scanner, "Long integer overflow"));
        };
        value
    };

    if value < lower || value > upper {
        return Err(current_error(scanner, "Long integer out of expected range"));
    }
    scanner.next_token()?;
    Ok(value)
}

pub fn parse_int(scanner: &mut Scanner) -> Result<i64, Diagnostic> {
    parse_int_limited(scanner, i64::MIN, i64::MAX)
}

pub fn parse_uint_max(scanner: &mut Scanner) -> Result<u64, Diagnostic> {
    scanner.check_tok(TokenType::POS_INT)?;
    let value = scanner.current_token().numval();
    scanner.next_token()?;
    Ok(value)
}

pub fn parse_float(scanner: &mut Scanner) -> Result<f64, Diagnostic> {
    let mut accumulator = String::new();

    if scanner.test_tok(TokenType::HYPHEN | TokenType::PLUS) {
        accumulator.push_str(&scanner.current_token().literal());
        scanner.next_token()?;
        scanner.check_tok_no_skip(TokenType::POS_INT)?;
    } else {
        scanner.check_tok(TokenType::POS_INT)?;
    }
    accumulator.push_str(&scanner.current_token().literal());
    scanner.next_token()?;

    if scanner.test_no_skip() && scanner.test_tok(TokenType::FULLSTOP) {
        accumulator.push('.');
        scanner.accept_tok_no_skip(TokenType::FULLSTOP)?;
        accumulator.push_str(&scanner.current_token().literal());
        scanner.accept_tok_no_skip(TokenType::POS_INT)?;
    }

    if scanner.test_no_skip() && scanner.test_id("e|E") {
        accumulator.push_str(&scanner.current_token().literal());
        scanner.next_token()?;
        accumulator.push_str(&scanner.current_token().literal());
        scanner.accept_tok_no_skip(TokenType::HYPHEN | TokenType::PLUS)?;
        accumulator.push_str(&scanner.current_token().literal());
        scanner.accept_tok_no_skip(TokenType::POS_INT)?;
    }

    accumulator
        .parse::<f64>()
        .map_err(|_| current_error(scanner, "Cannot translate double"))
}

pub fn parse_num_string(scanner: &mut Scanner) -> Result<ParsedNumString, Diagnostic> {
    let mut kind = StrNumType::Integer;
    let mut accumulator = String::new();

    if scanner.test_tok(TokenType::HYPHEN | TokenType::PLUS) {
        accumulator.push_str(&scanner.current_token().literal());
        scanner.next_token()?;
        scanner.check_tok_no_skip(TokenType::POS_INT)?;
    } else {
        scanner.check_tok(TokenType::POS_INT)?;
    }
    accumulator.push_str(&scanner.current_token().literal());
    scanner.next_token()?;

    if scanner.test_tok_no_skip(TokenType::SLASH) {
        accumulator.push('/');
        scanner.next_token()?;

        if scanner.test_tok(TokenType::HYPHEN | TokenType::PLUS) {
            accumulator.push_str(&scanner.current_token().literal());
            scanner.next_token()?;
        }

        if scanner.test_tok(TokenType::POS_INT)
            && is_zero_decimal(scanner.current_token().literal_bytes())
        {
            return Err(current_error(
                scanner,
                "Denominator in rational number cannot be 0",
            ));
        }
        accumulator.push_str(&scanner.current_token().literal());
        scanner.accept_tok_no_skip(TokenType::POS_INT)?;
        kind = StrNumType::Rational;
    } else {
        if scanner.test_tok_no_skip(TokenType::FULLSTOP)
            && scanner.look_token(1).kind().intersects(TokenType::POS_INT)
            && !scanner.look_token(1).skipped()
        {
            accumulator.push('.');
            scanner.accept_tok_no_skip(TokenType::FULLSTOP)?;
            accumulator.push_str(&scanner.current_token().literal());
            scanner.accept_tok_no_skip(TokenType::POS_INT)?;
            kind = StrNumType::Float;
        }

        if scanner.test_no_skip() {
            if scanner.test_id("e|E") {
                accumulator.push('e');
                scanner.next_token()?;
                accumulator.push_str(&scanner.current_token().literal());
                scanner.accept_tok_no_skip(TokenType::HYPHEN | TokenType::PLUS)?;
                accumulator.push_str(&scanner.current_token().literal());
                scanner.accept_tok_no_skip(TokenType::POS_INT)?;
                kind = StrNumType::Float;
            } else if scanner.test_idnum("e|E") {
                accumulator.push_str(&scanner.current_token().literal());
                scanner.accept_tok_no_skip(TokenType::IDNUM)?;
                kind = StrNumType::Float;
            }
        }
    }

    Ok(ParsedNumString {
        kind,
        text: accumulator,
    })
}

pub fn parse_double_array(scanner: &mut Scanner, brackets: bool) -> Result<Vec<f64>, Diagnostic> {
    let mut values = Vec::new();
    if brackets {
        scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    }

    if scanner.test_tok(TokenType::HYPHEN | TokenType::PLUS | TokenType::POS_INT) {
        values.push(parse_float(scanner)?);
        while scanner.test_tok(TokenType::COMMA) {
            scanner.next_token()?;
            values.push(parse_float(scanner)?);
        }
    }

    if brackets {
        scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    }
    Ok(values)
}

pub fn parse_filename(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    parse_filename_with_tokens(
        scanner,
        plain_file_tokens() | TokenType::SLASH | TokenType::MULT,
    )
}

pub fn parse_plain_filename(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    parse_filename_with_tokens(scanner, plain_file_tokens() | TokenType::SLASH)
}

pub fn parse_basic_include(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    scanner.accept_id("include")?;
    scanner.accept_tok(TokenType::OPEN_BRACKET)?;
    scanner.check_tok(TokenType::SQ_STRING)?;
    let result = strip_quote_core(scanner.current_token().literal_bytes())?;
    scanner.next_token()?;
    scanner.accept_tok(TokenType::CLOSE_BRACKET)?;
    scanner.accept_tok(TokenType::FULLSTOP)?;
    Ok(result)
}

pub fn parse_dotted_id(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    let mut result = String::new();

    result.push_str(&scanner.current_token().literal());
    scanner.accept_tok(TokenType::IDENTIFIER | TokenType::POS_INT)?;

    while scanner.test_no_skip() && scanner.test_tok(TokenType::FULLSTOP) {
        result.push_str(&scanner.current_token().literal());
        scanner.accept_tok(TokenType::FULLSTOP)?;
        result.push_str(&scanner.current_token().literal());
        scanner.accept_tok(TokenType::IDENTIFIER | TokenType::POS_INT)?;
    }

    Ok(result)
}

pub fn accept_dotted_id(scanner: &mut Scanner, expected: &str) -> Result<(), Diagnostic> {
    let position = token_pos_rep(scanner.current_token());
    let candidate = parse_dotted_id(scanner)?;
    if candidate == expected {
        Ok(())
    } else {
        Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!("{position} {expected} expected, but {candidate} read"),
        ))
    }
}

pub fn parse_continuous(scanner: &mut Scanner) -> Result<String, Diagnostic> {
    let mut result = String::new();
    result.push_str(&scanner.current_token().literal());
    scanner.next_token()?;

    while scanner.test_no_skip() {
        result.push_str(&scanner.current_token().literal());
        scanner.next_token()?;
    }
    Ok(result)
}

pub fn parse_skip_parenthesized_expr(scanner: &mut Scanner) -> Result<(), Diagnostic> {
    let mut stack = Vec::new();
    scanner.check_tok(open_tokens())?;
    stack.push(scanner.current_token().kind());
    scanner.next_token()?;

    while let Some(open) = stack.pop() {
        loop {
            if scanner.test_tok(open_tokens()) {
                stack.push(open);
                stack.push(scanner.current_token().kind());
                scanner.next_token()?;
                break;
            }
            if scanner.test_tok(close_tokens()) {
                match open {
                    TokenType::OPEN_BRACKET => scanner.accept_tok(TokenType::CLOSE_BRACKET)?,
                    TokenType::OPEN_CURLY => scanner.accept_tok(TokenType::CLOSE_CURLY)?,
                    TokenType::OPEN_SQUARE => scanner.accept_tok(TokenType::CLOSE_SQUARE)?,
                    _ => unreachable!("only open delimiters are pushed"),
                }
                break;
            }
            if scanner.test_tok(TokenType::NO_TOKEN) {
                return Err(current_error(
                    scanner,
                    "Unexpected end of input in parenthesized expression",
                ));
            }
            scanner.next_token()?;
        }
    }

    Ok(())
}

fn parse_filename_with_tokens(
    scanner: &mut Scanner,
    accepted: TokenType,
) -> Result<String, Diagnostic> {
    let mut first_token = true;
    let mut result = String::new();
    while (first_token || scanner.test_no_skip()) && scanner.test_tok(accepted) {
        result.push_str(&scanner.current_token().literal());
        scanner.next_token()?;
        first_token = false;
    }
    Ok(result)
}

fn plain_file_tokens() -> TokenType {
    TokenType::STRING
        | TokenType::NAME
        | TokenType::POS_INT
        | TokenType::FULLSTOP
        | TokenType::PLUS
        | TokenType::HYPHEN
        | TokenType::EQUAL_SIGN
}

fn open_tokens() -> TokenType {
    TokenType::OPEN_BRACKET | TokenType::OPEN_CURLY | TokenType::OPEN_SQUARE
}

fn close_tokens() -> TokenType {
    TokenType::CLOSE_BRACKET | TokenType::CLOSE_CURLY | TokenType::CLOSE_SQUARE
}

fn parse_i128_literal(bytes: &[u8]) -> Result<i128, Diagnostic> {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| text.parse::<i128>().ok())
        .ok_or_else(|| Diagnostic::new(ErrorCode::SYNTAX_ERROR, "Cannot translate integer"))
}

fn is_zero_decimal(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().all(|byte| *byte == b'0')
}

fn strip_quote_core(bytes: &[u8]) -> Result<String, Diagnostic> {
    if bytes.len() < 2 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Quoted string literal is too short",
        ));
    }
    Ok(String::from_utf8_lossy(&bytes[1..bytes.len() - 1]).into_owned())
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
    use super::{
        accept_dotted_id, parse_basic_include, parse_bool, parse_continuous, parse_dotted_id,
        parse_double_array, parse_filename, parse_float, parse_int, parse_int_limited,
        parse_int_max, parse_num_string, parse_plain_filename, parse_skip_parenthesized_expr,
        parse_uint_max, ParsedNumString, StrNumType,
    };
    use crate::basics::error::ErrorCode;
    use crate::inout::scanner::{Scanner, TokenType};

    fn make_scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap()
    }

    #[test]
    fn parses_booleans_and_consumes_one_token() {
        let mut scanner = make_scanner("true false");
        assert!(parse_bool(&mut scanner).unwrap());
        assert_eq!(scanner.current_token().literal(), "false");
        assert!(!parse_bool(&mut scanner).unwrap());
    }

    #[test]
    fn parses_limited_ints_and_preserves_c_parse_int_max_sign_quirk() {
        let mut scanner = make_scanner("-42 17");
        assert_eq!(parse_int(&mut scanner).unwrap(), -42);
        assert_eq!(parse_int_limited(&mut scanner, 0, 20).unwrap(), 17);

        let mut scanner = make_scanner("9");
        assert_eq!(parse_int_max(&mut scanner).unwrap(), -9);
    }

    #[test]
    fn parse_int_accepts_long_min_sentinel() {
        let mut scanner = make_scanner("-9223372036854775808 tail");
        assert_eq!(parse_int(&mut scanner).unwrap(), i64::MIN);
        assert_eq!(scanner.current_token().literal(), "tail");

        let mut below_min = make_scanner("-9223372036854775809");
        let error = parse_int(&mut below_min).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Long integer underflow"));
    }

    #[test]
    fn rejects_skipped_unsigned_part_after_minus() {
        let mut scanner = make_scanner("- 1");
        let error = parse_int(&mut scanner).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("White space"));
    }

    #[test]
    fn parses_uint_float_and_float_arrays() {
        let mut scanner = make_scanner("123 -4.5 6.7e-2 (1,2.5,+3)");
        assert_eq!(parse_uint_max(&mut scanner).unwrap(), 123);
        assert!((parse_float(&mut scanner).unwrap() + 4.5).abs() < f64::EPSILON);
        assert!((parse_float(&mut scanner).unwrap() - 0.067).abs() < 1.0e-12);
        assert_eq!(
            parse_double_array(&mut scanner, true).unwrap(),
            vec![1.0, 2.5, 3.0]
        );
    }

    #[test]
    fn parse_num_string_classifies_integer_rational_and_float_spellings() {
        let mut scanner = make_scanner("+12 -3/4 1.25 6e-7 8e9");
        assert_eq!(
            parse_num_string(&mut scanner).unwrap(),
            ParsedNumString {
                kind: StrNumType::Integer,
                text: "+12".to_owned()
            }
        );
        assert_eq!(
            parse_num_string(&mut scanner).unwrap(),
            ParsedNumString {
                kind: StrNumType::Rational,
                text: "-3/4".to_owned()
            }
        );
        assert_eq!(
            parse_num_string(&mut scanner).unwrap(),
            ParsedNumString {
                kind: StrNumType::Float,
                text: "1.25".to_owned()
            }
        );
        assert_eq!(parse_num_string(&mut scanner).unwrap().text, "6e-7");
        assert_eq!(parse_num_string(&mut scanner).unwrap().text, "8e9");
    }

    #[test]
    fn parse_num_string_rejects_zero_denominator() {
        let mut scanner = make_scanner("1/0");
        let error = parse_num_string(&mut scanner).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error
            .message()
            .contains("Denominator in rational number cannot be 0"));
    }

    #[test]
    fn parses_filename_variants_until_skipped_token() {
        let mut scanner = make_scanner("dir/file-1.* next");
        assert_eq!(parse_filename(&mut scanner).unwrap(), "dir/file-1.*");
        assert_eq!(scanner.current_token().literal(), "next");

        let mut scanner = make_scanner("plain/name* rest");
        assert_eq!(parse_plain_filename(&mut scanner).unwrap(), "plain/name");
        assert_eq!(scanner.current_token().kind(), TokenType::MULT);
    }

    #[test]
    fn parses_basic_include_and_dotted_identifiers() {
        let mut scanner = make_scanner("include('Axioms/SET001.ax'). foo.12.bar");
        assert_eq!(
            parse_basic_include(&mut scanner).unwrap(),
            "Axioms/SET001.ax"
        );
        assert_eq!(parse_dotted_id(&mut scanner).unwrap(), "foo.12.bar");

        let mut scanner = make_scanner("a.b");
        accept_dotted_id(&mut scanner, "a.b").unwrap();
        assert_eq!(scanner.current_token().kind(), TokenType::NO_TOKEN);
    }

    #[test]
    fn parse_continuous_stops_at_whitespace() {
        let mut scanner = make_scanner("abc:def rest");
        assert_eq!(parse_continuous(&mut scanner).unwrap(), "abc:def");
        assert_eq!(scanner.current_token().literal(), "rest");
    }

    #[test]
    fn skips_balanced_parenthesized_expressions_and_rejects_mismatch() {
        let mut scanner = make_scanner("(a,[b,{c}]) tail");
        parse_skip_parenthesized_expr(&mut scanner).unwrap();
        assert_eq!(scanner.current_token().literal(), "tail");

        let mut scanner = make_scanner("(]");
        let error = parse_skip_parenthesized_expr(&mut scanner).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Closing bracket"));
    }
}
