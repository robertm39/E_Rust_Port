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
    let (sign, digit_start) = match chars.next() {
        Some((_, '+')) => (1_i64, 1),
        Some((_, '-')) => (-1_i64, 1),
        Some((index, character)) if character.is_ascii_digit() => (1_i64, index),
        _ => return None,
    };

    let mut value = 0_i64;
    let mut consumed_digit = false;
    let mut end = digit_start;
    for (index, character) in trimmed[digit_start..].char_indices() {
        if !character.is_ascii_digit() {
            end = digit_start + index;
            break;
        }
        consumed_digit = true;
        end = digit_start + index + character.len_utf8();
        let digit = i64::from(character as u8 - b'0');
        value = value.checked_mul(10)?.checked_add(digit)?;
    }

    if !consumed_digit || !trimmed[end..].is_empty() {
        return None;
    }
    value.checked_mul(sign)
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
    for (index, character) in text.char_indices() {
        if count >= width {
            break;
        }
        if character == ' ' || character == '\n' {
            last_blank = Some(index);
            if character == '\n' {
                break;
            }
        }
        count += 1;
    }

    if count < width {
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
        get_bool_arg, get_int_arg, get_int_arg_check_range, print_options, CommandLineState,
        OptArgType, OptCell,
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
    fn option_printing_includes_optional_argument_default_note() {
        let output = print_options(&OPTIONS[..2], Some("Options:\n\n"));
        assert!(output.contains("Options:"));
        assert!(output.contains("--verbose[=<arg>]"));
        assert!(output.contains("equivalent to --verbose=1."));
    }
}
