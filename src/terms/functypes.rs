use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::basicparser::{parse_num_string, StrNumType};
use crate::inout::scanner::{Scanner, TokenType};

pub type FunCode = i64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum FuncSymbType {
    #[default]
    None = 0,
    IdentVar = 1,
    IdentFreeFun = 2,
    IdentInt = 3,
    IdentFloat = 4,
    IdentRational = 5,
    IdentInterpreted = 6,
    IdentObject = 7,
}

impl FuncSymbType {
    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[must_use]
pub fn atomic_func_sym_tok() -> TokenType {
    TokenType::IDENTIFIER | TokenType::SEM_IDENT | TokenType::SQ_STRING | TokenType::STRING
}

#[must_use]
pub fn func_symb_token() -> TokenType {
    atomic_func_sym_tok()
}

#[must_use]
pub fn func_symb_start_token() -> TokenType {
    atomic_func_sym_tok()
        | TokenType::POS_INT
        | TokenType::STRING
        | TokenType::PLUS
        | TokenType::HYPHEN
}

pub fn func_symb_parse(
    scanner: &mut Scanner,
    id: &mut DynamicString,
) -> Result<FuncSymbType, Diagnostic> {
    scanner.check_tok(func_symb_start_token())?;

    if scanner.test_tok(func_symb_token()) {
        let literal = scanner.current_token().literal();
        id.append_str(&literal);
        let result = if scanner.test_tok(TokenType::IDENTIFIER) {
            match literal.as_bytes().first().copied() {
                Some(first) if first.is_ascii_uppercase() || first == b'_' => {
                    FuncSymbType::IdentVar
                }
                _ => FuncSymbType::IdentFreeFun,
            }
        } else if scanner.test_tok(TokenType::SEM_IDENT) {
            FuncSymbType::IdentInterpreted
        } else if scanner.test_tok(TokenType::SQ_STRING) {
            FuncSymbType::IdentFreeFun
        } else if scanner.test_tok(TokenType::STRING) {
            FuncSymbType::IdentObject
        } else {
            return Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Unexpected token in function symbol",
            ));
        };
        scanner.accept_tok(func_symb_token())?;
        Ok(result)
    } else {
        scanner.check_tok(TokenType::POS_INT | TokenType::PLUS | TokenType::HYPHEN)?;
        let parsed = parse_num_string(scanner)?;
        match parsed.kind {
            StrNumType::Integer => {
                id.append_str(&normalize_int_rep(&parsed.text));
                Ok(FuncSymbType::IdentInt)
            }
            StrNumType::Rational => {
                let Some(normalized) = normalize_rational_rep(&parsed.text) else {
                    return Err(Diagnostic::new(
                        ErrorCode::SYNTAX_ERROR,
                        "Cannot normalize rational number",
                    ));
                };
                id.append_str(&normalized);
                Ok(FuncSymbType::IdentRational)
            }
            StrNumType::Float => {
                id.append_str(&normalize_float_rep(&parsed.text)?);
                Ok(FuncSymbType::IdentFloat)
            }
            StrNumType::NoNumber => Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Expected numeric function symbol",
            )),
        }
    }
}

#[must_use]
pub fn normalize_int_rep(int_rep: &str) -> String {
    let mut work = int_rep;
    let mut sign = "";
    if let Some(rest) = work.strip_prefix('+') {
        work = rest;
    } else if let Some(rest) = work.strip_prefix('-') {
        sign = "-";
        work = rest;
    }

    let work = work.trim_start_matches('0');
    if work.is_empty() {
        "0".to_owned()
    } else {
        format!("{sign}{work}")
    }
}

#[must_use]
pub fn normalize_rational_rep(rational_rep: &str) -> Option<String> {
    let (numerator_text, denominator_text) = rational_rep.split_once('/')?;
    let numerator = parse_lp64_strtoll_saturating_decimal(numerator_text)?;
    let denominator = parse_lp64_strtoll_saturating_decimal(denominator_text)?;
    if denominator == 0 {
        return None;
    }
    if numerator == i64::MIN || denominator == i64::MIN {
        return None;
    }

    let negative = (numerator < 0) ^ (denominator < 0);
    let numerator = numerator.unsigned_abs();
    let denominator = denominator.unsigned_abs();
    let gcd = gcd_u64(numerator, denominator);
    if gcd == 0 {
        return None;
    }

    let numerator = numerator / gcd;
    let denominator = denominator / gcd;
    let signed_numerator = if negative && numerator != 0 {
        format!("-{numerator}")
    } else {
        numerator.to_string()
    };
    Some(format!("{signed_numerator}/{denominator}"))
}

pub fn normalize_float_rep(float_rep: &str) -> Result<String, Diagnostic> {
    let value = float_rep
        .parse::<f64>()
        .map_err(|_| Diagnostic::new(ErrorCode::SYNTAX_ERROR, "Cannot translate double"))?;
    if value.is_nan() {
        return Ok("nan".to_owned());
    }
    if value.is_infinite() {
        return Ok(if value.is_sign_negative() {
            "-inf".to_owned()
        } else {
            "inf".to_owned()
        });
    }

    if value.abs() >= 1000.0 {
        Ok(c_exp_format(value))
    } else {
        Ok(format!("{value:.6}"))
    }
}

fn c_exp_format(value: f64) -> String {
    let raw = format!("{value:.6e}");
    let Some((mantissa, exponent)) = raw.split_once('e') else {
        return raw;
    };
    let Ok(exponent) = exponent.parse::<i32>() else {
        return raw;
    };
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("{mantissa}e{sign}{:02}", exponent.abs())
}

fn parse_lp64_strtoll_saturating_decimal(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let (negative, digits) = match bytes.first() {
        Some(b'+') => (false, &bytes[1..]),
        Some(b'-') => (true, &bytes[1..]),
        Some(_) => (false, bytes),
        None => return None,
    };
    if digits.is_empty() {
        return None;
    }

    let limit = if negative {
        u128::from(i64::MIN.unsigned_abs())
    } else {
        u128::from(i64::MAX.unsigned_abs())
    };
    let mut value = 0_u128;
    let mut overflowed = false;

    for byte in digits {
        let digit = match byte {
            b'0'..=b'9' => u128::from(byte - b'0'),
            _ => return None,
        };
        if value > (limit - digit) / 10 {
            overflowed = true;
            value = limit;
        } else if !overflowed {
            value = value * 10 + digit;
        }
    }

    if overflowed {
        return Some(if negative { i64::MIN } else { i64::MAX });
    }

    let magnitude = u64::try_from(value).ok()?;
    if negative {
        if magnitude == i64::MIN.unsigned_abs() {
            Some(i64::MIN)
        } else {
            Some(-i64::try_from(magnitude).ok()?)
        }
    } else {
        i64::try_from(magnitude).ok()
    }
}

fn gcd_u64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left
}

#[cfg(test)]
mod tests {
    use super::{
        atomic_func_sym_tok, func_symb_parse, func_symb_start_token, func_symb_token,
        normalize_float_rep, normalize_int_rep, normalize_rational_rep, FuncSymbType,
    };
    use crate::basics::dstrings::DynamicString;
    use crate::inout::scanner::{Scanner, TokenType};

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap()
    }

    fn parse_one(source: &str) -> (FuncSymbType, String) {
        let mut scanner = scanner(source);
        let mut id = DynamicString::new();
        let kind = func_symb_parse(&mut scanner, &mut id).unwrap();
        (kind, id.view().into_owned())
    }

    #[test]
    fn enum_values_match_c_order() {
        assert_eq!(FuncSymbType::None.c_value(), 0);
        assert_eq!(FuncSymbType::IdentVar.c_value(), 1);
        assert_eq!(FuncSymbType::IdentFreeFun.c_value(), 2);
        assert_eq!(FuncSymbType::IdentInt.c_value(), 3);
        assert_eq!(FuncSymbType::IdentFloat.c_value(), 4);
        assert_eq!(FuncSymbType::IdentRational.c_value(), 5);
        assert_eq!(FuncSymbType::IdentInterpreted.c_value(), 6);
        assert_eq!(FuncSymbType::IdentObject.c_value(), 7);
    }

    #[test]
    fn token_masks_match_c_global_shapes() {
        let atomic =
            TokenType::IDENTIFIER | TokenType::SEM_IDENT | TokenType::SQ_STRING | TokenType::STRING;
        assert_eq!(atomic_func_sym_tok(), atomic);
        assert_eq!(func_symb_token(), atomic);
        assert!(func_symb_start_token().intersects(TokenType::POS_INT));
        assert!(func_symb_start_token().intersects(TokenType::PLUS));
        assert!(func_symb_start_token().intersects(TokenType::HYPHEN));
    }

    #[test]
    fn integer_normalization_drops_plus_and_leading_zeros() {
        assert_eq!(normalize_int_rep("+00012"), "12");
        assert_eq!(normalize_int_rep("-00012"), "-12");
        assert_eq!(normalize_int_rep("000"), "0");
        assert_eq!(normalize_int_rep("-000"), "0");
    }

    #[test]
    fn rational_normalization_reduces_and_moves_sign_to_front() {
        assert_eq!(normalize_rational_rep("+06/-008").as_deref(), Some("-3/4"));
        assert_eq!(normalize_rational_rep("-6/-8").as_deref(), Some("3/4"));
        assert_eq!(normalize_rational_rep("0/-10").as_deref(), Some("0/1"));
        assert_eq!(normalize_rational_rep("1/0"), None);
    }

    #[test]
    fn rational_normalization_matches_c_strtoll_overflow_shape() {
        assert_eq!(
            normalize_rational_rep("9223372036854775808/2").as_deref(),
            Some("9223372036854775807/2")
        );
        assert_eq!(
            normalize_rational_rep("2/9223372036854775808").as_deref(),
            Some("2/9223372036854775807")
        );
        assert_eq!(
            normalize_rational_rep("+18446744073709551616/+3").as_deref(),
            Some("9223372036854775807/3")
        );
        assert_eq!(normalize_rational_rep("-9223372036854775808/1"), None);
        assert_eq!(normalize_rational_rep("1/-9223372036854775808"), None);
    }

    #[test]
    fn float_normalization_matches_c_printf_shape() {
        assert_eq!(normalize_float_rep("1.5").unwrap(), "1.500000");
        assert_eq!(normalize_float_rep("1000.0").unwrap(), "1.000000e+03");
        assert_eq!(normalize_float_rep("-1000.0").unwrap(), "-1.000000e+03");
        assert_eq!(normalize_float_rep("6e-7").unwrap(), "0.000001");
    }

    #[test]
    fn parses_atomic_function_symbols_and_appends_to_id() {
        let mut scanner = scanner("Foo");
        let mut id = DynamicString::new();
        id.append_str("prefix:");
        assert_eq!(
            func_symb_parse(&mut scanner, &mut id).unwrap(),
            FuncSymbType::IdentVar
        );
        assert_eq!(id.view(), "prefix:Foo");

        assert_eq!(parse_one("_X"), (FuncSymbType::IdentVar, "_X".to_owned()));
        assert_eq!(
            parse_one("foo123"),
            (FuncSymbType::IdentFreeFun, "foo123".to_owned())
        );
        assert_eq!(
            parse_one("$true"),
            (FuncSymbType::IdentInterpreted, "$true".to_owned())
        );
        assert_eq!(
            parse_one("'quoted'"),
            (FuncSymbType::IdentFreeFun, "'quoted'".to_owned())
        );
        assert_eq!(
            parse_one("\"object\""),
            (FuncSymbType::IdentObject, "\"object\"".to_owned())
        );
    }

    #[test]
    fn parses_and_normalizes_numeric_function_symbols() {
        assert_eq!(
            parse_one("+00012"),
            (FuncSymbType::IdentInt, "12".to_owned())
        );
        assert_eq!(parse_one("-000"), (FuncSymbType::IdentInt, "0".to_owned()));
        assert_eq!(
            parse_one("6/8"),
            (FuncSymbType::IdentRational, "3/4".to_owned())
        );
        assert_eq!(
            parse_one("-6/-8"),
            (FuncSymbType::IdentRational, "3/4".to_owned())
        );
        assert_eq!(
            parse_one("1.5"),
            (FuncSymbType::IdentFloat, "1.500000".to_owned())
        );
        assert_eq!(
            parse_one("1000.0"),
            (FuncSymbType::IdentFloat, "1.000000e+03".to_owned())
        );
    }
}
