use crate::basics::error::{Diagnostic, ErrorCode};

pub const FORMAT_WIDTH: usize = 78;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptArgType {
    NoArg,
    OptArg,
    ReqArg,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptCell<Code> {
    pub option_code: Code,
    pub shortopt: Option<char>,
    pub longopt: Option<&'static str>,
    pub arg_type: OptArgType,
    pub arg_default: Option<&'static str>,
    pub desc: &'static str,
}

impl<Code> OptCell<Code> {
    #[must_use]
    pub const fn new(
        option_code: Code,
        shortopt: Option<char>,
        longopt: Option<&'static str>,
        arg_type: OptArgType,
        arg_default: Option<&'static str>,
        desc: &'static str,
    ) -> Self {
        Self {
            option_code,
            shortopt,
            longopt,
            arg_type,
            arg_default,
            desc,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedOpt<'a, Code> {
    option: &'a OptCell<Code>,
    arg: Option<String>,
}

impl<'a, Code> ParsedOpt<'a, Code> {
    #[must_use]
    pub const fn option(&self) -> &'a OptCell<Code> {
        self.option
    }

    #[must_use]
    pub fn arg(&self) -> Option<&str> {
        self.arg.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandLineState {
    argv: Vec<String>,
    sc_opt_c: usize,
    argi: usize,
}

impl CommandLineState {
    #[must_use]
    pub fn new<I, S>(argv: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut argument_iter = argv.into_iter();
        let _program_name = argument_iter.next();
        Self {
            argv: argument_iter.map(Into::into).collect(),
            sc_opt_c: 0,
            argi: 0,
        }
    }

    #[must_use]
    pub fn remaining_args(&self) -> &[String] {
        &self.argv
    }

    pub fn insert_arg(&mut self, arg: impl Into<String>) -> usize {
        self.argv.push(arg.into());
        self.argv.len()
    }

    pub fn next_opt<'a, Code: Copy>(
        &mut self,
        options: &'a [OptCell<Code>],
    ) -> Result<Option<ParsedOpt<'a, Code>>, Diagnostic> {
        while let Some(current) = self.argv.get(self.argi) {
            if is_option_candidate(current) {
                break;
            }
            self.argi += 1;
        }

        let Some(current) = self.argv.get(self.argi) else {
            return Ok(None);
        };

        if current == "--" {
            self.argv.remove(self.argi);
            self.argi = self.argv.len();
            return Ok(None);
        }

        if current.starts_with("--") {
            self.process_long_option(options).map(Some)
        } else {
            if self.sc_opt_c == 0 {
                self.sc_opt_c = 1;
            }
            self.process_short_option(options).map(Some)
        }
    }

    fn process_long_option<'a, Code: Copy>(
        &mut self,
        options: &'a [OptCell<Code>],
    ) -> Result<ParsedOpt<'a, Code>, Diagnostic> {
        let option_text = self.argv[self.argi].clone();
        let Some(handle) = find_long_opt(&option_text, options) else {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                format!("Unknown Option: {option_text} (Use -h for a list of valid options)"),
            ));
        };

        let eq_sign = option_text.find('=');
        let arg = match handle.arg_type {
            OptArgType::NoArg => {
                if eq_sign.is_some() {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        format!("{option_text} does not accept an argument!"),
                    ));
                }
                None
            }
            OptArgType::OptArg => match eq_sign {
                Some(index) => Some(option_text[index + 1..].to_owned()),
                None => Some(
                    handle
                        .arg_default
                        .map_or_else(String::new, ToOwned::to_owned),
                ),
            },
            OptArgType::ReqArg => match eq_sign {
                Some(index) => Some(option_text[index + 1..].to_owned()),
                None => {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        format!("{option_text} requires an argument!"),
                    ));
                }
            },
        };
        self.argv.remove(self.argi);
        Ok(ParsedOpt {
            option: handle,
            arg,
        })
    }

    fn process_short_option<'a, Code: Copy>(
        &mut self,
        options: &'a [OptCell<Code>],
    ) -> Result<ParsedOpt<'a, Code>, Diagnostic> {
        let option_text = self.argv[self.argi].clone();
        let option_bytes = option_text.as_bytes();
        let option_char = option_bytes
            .get(self.sc_opt_c)
            .map(|byte| char::from(*byte))
            .ok_or_else(|| {
                Diagnostic::new(
                    ErrorCode::USAGE_ERROR,
                    format!("Unknown Option: - (processing {option_text}) (Use -h for a list of valid options)"),
                )
            })?;

        let Some(handle) = find_short_opt(option_char, options) else {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                format!(
                    "Unknown Option: -{option_char} (processing {option_text}) (Use -h for a list of valid options)"
                ),
            ));
        };

        match handle.arg_type {
            OptArgType::NoArg | OptArgType::OptArg => {
                let arg = if handle.arg_type == OptArgType::OptArg {
                    Some(
                        handle
                            .arg_default
                            .map_or_else(String::new, ToOwned::to_owned),
                    )
                } else {
                    None
                };
                self.sc_opt_c += 1;
                if self.sc_opt_c >= option_text.len() {
                    self.sc_opt_c = 0;
                    self.argv.remove(self.argi);
                }
                Ok(ParsedOpt {
                    option: handle,
                    arg,
                })
            }
            OptArgType::ReqArg => {
                if self.sc_opt_c != 1 {
                    return Err(Diagnostic::new(
                        ErrorCode::USAGE_ERROR,
                        format!(
                            "{option_text}: POSIX forbids the aggregation of options which take arguments (but you probably only forgot the second hyphen for a long GNU-style option)"
                        ),
                    ));
                }

                let attached_start = self.sc_opt_c + 1;
                let arg = if attached_start < option_text.len() {
                    option_text[attached_start..].to_owned()
                } else {
                    self.argv.remove(self.argi);
                    let Some(next_arg) = self.argv.get(self.argi).cloned() else {
                        return Err(Diagnostic::new(
                            ErrorCode::USAGE_ERROR,
                            format!("-{option_char} requires an argument"),
                        ));
                    };
                    next_arg
                };

                self.sc_opt_c = 0;
                self.argv.remove(self.argi);
                Ok(ParsedOpt {
                    option: handle,
                    arg: Some(arg),
                })
            }
        }
    }
}

#[must_use]
pub fn append_option_desc<Code>(option: &OptCell<Code>) -> String {
    let mut result = String::new();
    if let Some(shortopt) = option.shortopt {
        result.push('-');
        result.push(shortopt);
    }
    if option.shortopt.is_some() && option.longopt.is_some() {
        result.push_str(" or ");
    }
    if let Some(longopt) = option.longopt {
        result.push_str("--");
        result.push_str(longopt);
    }
    result
}

pub fn get_int_arg<Code>(option: &OptCell<Code>, arg: &str) -> Result<i64, Diagnostic> {
    parse_c_long(arg).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "{} expects integer instead of '{arg}'",
                append_option_desc(option)
            ),
        )
    })
}

pub fn get_int_arg_check_range<Code>(
    option: &OptCell<Code>,
    arg: &str,
    lower: i64,
    upper: i64,
) -> Result<i64, Diagnostic> {
    let value = get_int_arg(option, arg)?;
    if (lower..=upper).contains(&value) {
        Ok(value)
    } else {
        Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "Option {} expects integer argument from {{{lower}...{upper}}} but got {arg}",
                append_option_desc(option)
            ),
        ))
    }
}

pub fn get_float_arg<Code>(option: &OptCell<Code>, arg: &str) -> Result<f64, Diagnostic> {
    parse_c_double(arg).ok_or_else(|| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "{} expects float instead of '{arg}'",
                append_option_desc(option)
            ),
        )
    })
}

pub fn get_bool_arg<Code>(option: &OptCell<Code>, arg: &str) -> Result<bool, Diagnostic> {
    match arg {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!(
                "{} expects 'true' or 'false' instead of '{arg}'",
                append_option_desc(option)
            ),
        )),
    }
}

#[must_use]
pub fn print_option<Code>(option: &OptCell<Code>) -> String {
    let mut result = String::new();
    let long_arg_desc = match option.arg_type {
        OptArgType::NoArg => "",
        OptArgType::OptArg => "[=<arg>]",
        OptArgType::ReqArg => "=<arg>",
    };

    if let Some(shortopt) = option.shortopt {
        result.push_str("   -");
        result.push(shortopt);
        if option.arg_type == OptArgType::ReqArg {
            result.push_str(" <arg>");
        }
        result.push('\n');
    }
    if let Some(longopt) = option.longopt {
        result.push_str("  --");
        result.push_str(longopt);
        result.push_str(long_arg_desc);
        result.push('\n');
    }

    let mut desc = String::from(option.desc);
    if option.arg_type == OptArgType::OptArg {
        if option.shortopt.is_some() {
            desc.push_str(" The short form or the long form without the optional argument is equivalent to --");
        } else {
            desc.push_str(" The option without the optional argument is equivalent to --");
        }
        if let (Some(longopt), Some(default)) = (option.longopt, option.arg_default) {
            desc.push_str(longopt);
            desc.push('=');
            desc.push_str(default);
            desc.push('.');
        }
    }

    for line in wrap_c_style(&desc, FORMAT_WIDTH - 4) {
        result.push_str("    ");
        result.push_str(&line);
        result.push('\n');
    }
    result.push('\n');
    result
}

#[must_use]
pub fn print_options<Code>(options: &[OptCell<Code>], header: Option<&str>) -> String {
    let mut result = String::new();
    if let Some(header) = header {
        result.push_str(header);
    }
    for option in options {
        result.push_str(&print_option(option));
    }
    result
}

fn is_option_candidate(argument: &str) -> bool {
    argument.starts_with('-') && argument.len() > 1
}

fn find_long_opt<'a, Code>(
    option: &str,
    options: &'a [OptCell<Code>],
) -> Option<&'a OptCell<Code>> {
    let option = &option[2..];
    let name_len = option.find('=').unwrap_or(option.len());
    let name = &option[..name_len];
    options
        .iter()
        .find(|candidate| candidate.longopt.is_some_and(|longopt| longopt == name))
}

fn find_short_opt<Code>(option: char, options: &[OptCell<Code>]) -> Option<&OptCell<Code>> {
    options
        .iter()
        .find(|candidate| candidate.shortopt == Some(option))
}

fn parse_c_long(argument: &str) -> Option<i64> {
    if argument.is_empty() {
        return Some(0);
    }

    let trimmed = argument.trim_start_matches(|character: char| character.is_ascii_whitespace());
    let mut chars = trimmed.char_indices();
    let (negative, digit_start) = match chars.next() {
        Some((_, '+')) => (false, 1),
        Some((_, '-')) => (true, 1),
        Some((index, character)) if character.is_ascii_digit() => (false, index),
        _ => return None,
    };

    let limit = if negative {
        i64::MIN.unsigned_abs()
    } else {
        i64::MAX.unsigned_abs()
    };
    let mut value = 0_u64;
    let mut consumed_digit = false;
    let mut end = digit_start;
    for (index, character) in trimmed[digit_start..].char_indices() {
        if !character.is_ascii_digit() {
            end = digit_start + index;
            break;
        }
        consumed_digit = true;
        end = digit_start + index + character.len_utf8();
        let digit = u64::from(character as u8 - b'0');
        value = value.checked_mul(10)?.checked_add(digit)?;
        if value > limit {
            return None;
        }
    }

    if !consumed_digit || !trimmed[end..].is_empty() {
        return None;
    }
    if negative {
        if value == i64::MIN.unsigned_abs() {
            Some(i64::MIN)
        } else {
            i64::try_from(value).ok().map(|value| -value)
        }
    } else {
        i64::try_from(value).ok()
    }
}

fn parse_c_double(argument: &str) -> Option<f64> {
    if argument.is_empty() {
        return Some(0.0);
    }

    let trimmed = argument.trim_start_matches(|character: char| character.is_ascii_whitespace());
    if trimmed.is_empty() {
        return None;
    }

    if let Some(value) = parse_c_named_double(trimmed) {
        return Some(value);
    }
    if let Some(value) = parse_c_hex_double(trimmed) {
        return Some(value);
    }

    let value = trimmed.parse::<f64>().ok()?;
    if value.is_infinite() && !is_named_infinite(trimmed) {
        return None;
    }
    if decimal_double_underflowed(trimmed, value) {
        return None;
    }
    Some(value)
}

fn parse_c_named_double(argument: &str) -> Option<f64> {
    let (negative, unsigned) = split_c_float_sign(argument);
    let lower = unsigned.to_ascii_lowercase();

    let value = if matches!(lower.as_str(), "inf" | "infinity") {
        f64::INFINITY
    } else if lower == "nan" || is_c_nan_payload(&lower) {
        f64::NAN
    } else {
        return None;
    };

    Some(if negative { -value } else { value })
}

fn parse_c_hex_double(argument: &str) -> Option<f64> {
    let (negative, unsigned) = split_c_float_sign(argument);
    let rest = unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))?;
    let bytes = rest.as_bytes();
    let mut index = 0_usize;
    let mut significand = 0.0_f64;
    let mut hex_digits = 0_usize;
    let mut fractional_digits = 0_usize;
    let mut seen_dot = false;

    while let Some(byte) = bytes.get(index).copied() {
        if let Some(digit) = hex_digit_value(byte) {
            significand = significand.mul_add(16.0, f64::from(digit));
            hex_digits += 1;
            if seen_dot {
                fractional_digits += 1;
            }
            index += 1;
        } else if byte == b'.' && !seen_dot {
            seen_dot = true;
            index += 1;
        } else {
            break;
        }
    }

    let has_exponent_marker = bytes
        .get(index)
        .copied()
        .is_some_and(|byte| matches!(byte, b'p' | b'P'));
    if hex_digits == 0 || !has_exponent_marker || !significand.is_finite() {
        return None;
    }
    index += 1;

    let (exponent_negative, exponent, next_index) = parse_decimal_exponent(&rest[index..])?;
    index += next_index;
    if index != bytes.len() {
        return None;
    }

    let signed_exponent = if exponent_negative {
        exponent.checked_neg()?
    } else {
        exponent
    };
    let fractional_offset = i64::try_from(fractional_digits).ok()?.checked_mul(4)?;
    let binary_exponent = signed_exponent.checked_sub(fractional_offset)?;
    if significand == 0.0 {
        return Some(if negative { -0.0 } else { 0.0 });
    }
    let binary_exponent = i32::try_from(binary_exponent).ok()?;
    let value = significand * 2.0_f64.powi(binary_exponent);
    if matches!(
        value.classify(),
        std::num::FpCategory::Infinite
            | std::num::FpCategory::Zero
            | std::num::FpCategory::Subnormal
    ) {
        return None;
    }

    Some(if negative { -value } else { value })
}

fn decimal_double_underflowed(argument: &str, value: f64) -> bool {
    decimal_significand_has_nonzero_digit(argument)
        && matches!(
            value.classify(),
            std::num::FpCategory::Zero | std::num::FpCategory::Subnormal
        )
}

fn decimal_significand_has_nonzero_digit(argument: &str) -> bool {
    let (_, unsigned) = split_c_float_sign(argument);
    unsigned
        .bytes()
        .take_while(|byte| !matches!(byte, b'e' | b'E'))
        .any(|byte| matches!(byte, b'1'..=b'9'))
}

fn split_c_float_sign(argument: &str) -> (bool, &str) {
    match argument.as_bytes().first() {
        Some(b'+') => (false, &argument[1..]),
        Some(b'-') => (true, &argument[1..]),
        _ => (false, argument),
    }
}

fn hex_digit_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_decimal_exponent(argument: &str) -> Option<(bool, i64, usize)> {
    let bytes = argument.as_bytes();
    let (negative, mut index) = match bytes.first() {
        Some(b'+') => (false, 1_usize),
        Some(b'-') => (true, 1_usize),
        _ => (false, 0_usize),
    };
    let mut value = 0_i64;
    let digit_start = index;

    while let Some(byte @ b'0'..=b'9') = bytes.get(index).copied() {
        value = value.checked_mul(10)?.checked_add(i64::from(byte - b'0'))?;
        index += 1;
    }

    if index == digit_start {
        None
    } else {
        Some((negative, value, index))
    }
}

fn is_named_infinite(argument: &str) -> bool {
    matches!(
        argument.to_ascii_lowercase().as_str(),
        "inf" | "+inf" | "-inf" | "infinity" | "+infinity" | "-infinity"
    )
}

fn is_c_nan_payload(argument: &str) -> bool {
    let Some(payload) = argument
        .strip_prefix("nan(")
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        return false;
    };
    payload
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn wrap_c_style(text: &str, width: usize) -> Vec<String> {
    let mut remaining = text;
    let mut lines = Vec::new();
    while !remaining.is_empty() {
        let (line, rest) = split_c_style(remaining, width);
        lines.push(line.to_owned());
        remaining = rest;
    }
    lines
}

fn split_c_style(text: &str, width: usize) -> (&str, &str) {
    let mut last_blank = None;
    let mut count = 0_usize;
    let mut forced_newline = false;
    for (index, character) in text.char_indices() {
        if count >= width {
            break;
        }
        if character == ' ' || character == '\n' {
            last_blank = Some(index);
            if character == '\n' {
                forced_newline = true;
                break;
            }
        }
        count += 1;
    }

    if count < width && !forced_newline {
        return (text, "");
    }
    if let Some(blank) = last_blank {
        let next = blank + 1;
        return (&text[..blank], text.get(next..).unwrap_or(""));
    }

    let split_at = text
        .char_indices()
        .nth(width)
        .map_or(text.len(), |(index, _)| index);
    (&text[..split_at], &text[split_at..])
}

#[cfg(test)]
mod tests {
    use super::{
        get_bool_arg, get_float_arg, get_int_arg, get_int_arg_check_range, print_options,
        CommandLineState, OptArgType, OptCell,
    };
    use crate::basics::error::ErrorCode;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Code {
        Help,
        Verbose,
        Output,
        Silent,
    }

    const OPTIONS: &[OptCell<Code>] = &[
        OptCell::new(
            Code::Help,
            Some('h'),
            Some("help"),
            OptArgType::NoArg,
            None,
            "Help text.",
        ),
        OptCell::new(
            Code::Verbose,
            Some('v'),
            Some("verbose"),
            OptArgType::OptArg,
            Some("1"),
            "Verbose text.",
        ),
        OptCell::new(
            Code::Output,
            Some('o'),
            Some("output-file"),
            OptArgType::ReqArg,
            None,
            "Output text.",
        ),
        OptCell::new(
            Code::Silent,
            Some('s'),
            Some("silent"),
            OptArgType::NoArg,
            None,
            "Silent text.",
        ),
    ];

    #[test]
    fn long_required_arguments_must_use_equals() {
        let mut state = CommandLineState::new(["eprover", "--output-file", "out"]);
        let error = state.next_opt(OPTIONS).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "--output-file requires an argument!");
    }

    #[test]
    fn long_optional_argument_uses_default_without_equals() {
        let mut state = CommandLineState::new(["eprover", "--verbose"]);
        let parsed = state.next_opt(OPTIONS).unwrap().unwrap();
        assert_eq!(parsed.option().option_code, Code::Verbose);
        assert_eq!(parsed.arg(), Some("1"));
    }

    #[test]
    fn short_required_argument_accepts_attached_value() {
        let mut state = CommandLineState::new(["eprover", "-omy.out", "problem.p"]);
        let parsed = state.next_opt(OPTIONS).unwrap().unwrap();
        assert_eq!(parsed.option().option_code, Code::Output);
        assert_eq!(parsed.arg(), Some("my.out"));
        assert_eq!(state.remaining_args(), &["problem.p".to_owned()]);
    }

    #[test]
    fn short_required_argument_accepts_next_argv_even_when_it_starts_with_dash() {
        let mut state = CommandLineState::new(["eprover", "-o", "-"]);
        let parsed = state.next_opt(OPTIONS).unwrap().unwrap();
        assert_eq!(parsed.option().option_code, Code::Output);
        assert_eq!(parsed.arg(), Some("-"));
        assert!(state.remaining_args().is_empty());
    }

    #[test]
    fn short_optional_argument_uses_default_and_keeps_aggregating() {
        let mut state = CommandLineState::new(["eprover", "-vs"]);
        let first = state.next_opt(OPTIONS).unwrap().unwrap();
        let second = state.next_opt(OPTIONS).unwrap().unwrap();
        assert_eq!(first.option().option_code, Code::Verbose);
        assert_eq!(first.arg(), Some("1"));
        assert_eq!(second.option().option_code, Code::Silent);
    }

    #[test]
    fn double_dash_stops_option_processing() {
        let mut state = CommandLineState::new(["eprover", "--", "-h"]);
        assert!(state.next_opt(OPTIONS).unwrap().is_none());
        assert_eq!(state.remaining_args(), &["-h".to_owned()]);
    }

    #[test]
    fn int_arg_matches_c_empty_string_edge_case() {
        let option = &OPTIONS[1];
        assert_eq!(get_int_arg(option, "").unwrap(), 0);
        assert_eq!(get_int_arg(option, " 42").unwrap(), 42);
        assert!(get_int_arg(option, "42x").is_err());
    }

    #[test]
    fn int_arg_accepts_long_min_and_rejects_overflow_boundaries() {
        let option = &OPTIONS[1];
        assert_eq!(
            get_int_arg(option, "-9223372036854775808").unwrap(),
            i64::MIN
        );
        assert_eq!(
            get_int_arg(option, "9223372036854775807").unwrap(),
            i64::MAX
        );
        assert!(get_int_arg(option, "-9223372036854775809").is_err());
        assert!(get_int_arg(option, "9223372036854775808").is_err());
    }

    #[test]
    fn int_arg_range_and_bool_errors_are_usage_errors() {
        let option = &OPTIONS[1];
        assert_eq!(get_int_arg_check_range(option, "3", 1, 5).unwrap(), 3);
        assert_eq!(
            get_int_arg_check_range(option, "6", 1, 5)
                .unwrap_err()
                .code(),
            ErrorCode::USAGE_ERROR
        );
        assert!(get_bool_arg(option, "true").unwrap());
        assert_eq!(
            get_bool_arg(option, "yes").unwrap_err().code(),
            ErrorCode::USAGE_ERROR
        );
    }

    #[test]
    fn float_arg_matches_c_strtod_shape() {
        let option = &OPTIONS[1];
        assert_eq!(
            get_float_arg(option, "").unwrap().to_bits(),
            0.0_f64.to_bits()
        );
        assert_eq!(
            get_float_arg(option, " 1.25e2").unwrap().to_bits(),
            125.0_f64.to_bits()
        );
        assert!(get_float_arg(option, "1.25x").is_err());
        assert!(get_float_arg(option, " ").is_err());
        assert_eq!(
            get_float_arg(option, "1e9999").unwrap_err().code(),
            ErrorCode::USAGE_ERROR
        );
        assert!(get_float_arg(option, "inf").unwrap().is_infinite());
        assert!(get_float_arg(option, "INFINITY").unwrap().is_infinite());
        assert!(get_float_arg(option, "-INF").unwrap().is_sign_negative());
        assert!(get_float_arg(option, "nan").unwrap().is_nan());
        assert!(get_float_arg(option, "NAN(payload_1)").unwrap().is_nan());
        assert!(get_float_arg(option, "nan(payload-)").is_err());
        assert_eq!(
            get_float_arg(option, "0x1p2").unwrap().to_bits(),
            4.0_f64.to_bits()
        );
        assert_eq!(
            get_float_arg(option, "0x1.8p+2").unwrap().to_bits(),
            6.0_f64.to_bits()
        );
        assert_eq!(
            get_float_arg(option, "-0X1p-1").unwrap().to_bits(),
            (-0.5_f64).to_bits()
        );
        assert_eq!(
            get_float_arg(option, "0e-9999").unwrap().to_bits(),
            0.0_f64.to_bits()
        );
        assert_eq!(
            get_float_arg(option, "0x.8p1").unwrap().to_bits(),
            1.0_f64.to_bits()
        );
        assert_eq!(
            get_float_arg(option, "0x0p1024").unwrap().to_bits(),
            0.0_f64.to_bits()
        );
        assert_eq!(
            get_float_arg(option, "-0x0p1024").unwrap().to_bits(),
            (-0.0_f64).to_bits()
        );
        assert_eq!(
            get_float_arg(option, "0x0p-1074").unwrap().to_bits(),
            0.0_f64.to_bits()
        );
        assert!(get_float_arg(option, "0x1.2").is_err());
        assert!(get_float_arg(option, "0x1p1024").is_err());
        assert!(get_float_arg(option, "1e-309").is_err());
        assert!(get_float_arg(option, "5e-324").is_err());
        assert!(get_float_arg(option, "0x1p-1074").is_err());
    }

    #[test]
    fn option_printing_includes_optional_argument_default_note() {
        let output = print_options(&OPTIONS[..2], Some("Options:\n\n"));
        assert!(output.contains("Options:"));
        assert!(output.contains("--verbose[=<arg>]"));
        assert!(output.contains("equivalent to --verbose=1."));
    }

    #[test]
    fn option_printing_splits_embedded_newlines_like_c() {
        const MULTILINE_OPTIONS: &[OptCell<Code>] = &[OptCell::new(
            Code::Help,
            Some('m'),
            Some("multi"),
            OptArgType::NoArg,
            None,
            "First line.\nSecond line.",
        )];

        let output = print_options(MULTILINE_OPTIONS, None);

        assert!(output.contains("    First line.\n    Second line.\n"));
    }
}
