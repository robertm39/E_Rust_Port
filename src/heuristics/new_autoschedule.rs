//! Predefined strategy lookup from C `che_new_autoschedule`.

use std::sync::OnceLock;

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::heuristics::hcb::{
    heuristic_parms_parse_into, heuristic_parms_print_string, HeuristicParmsCell,
};
use crate::inout::scanner::{Scanner, TokenType};

const SCHEDULE_VARS: &str = include_str!("../../eprover/HEURISTICS/schedule.vars");

#[derive(Clone, Debug, Eq, PartialEq)]
struct PredefinedStrategy {
    name: String,
    definition: String,
}

static PREDEFINED_STRATEGIES: OnceLock<Result<Vec<PredefinedStrategy>, Diagnostic>> =
    OnceLock::new();

/// Parses a named predefined strategy into `target`, matching C
/// `GetHeuristicWithName`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded C schedule table cannot be parsed, the
/// requested name is not present, or the selected parameter block is malformed.
pub fn get_heuristic_with_name(
    name: &str,
    target: &mut HeuristicParmsCell,
) -> Result<(), Diagnostic> {
    let Some(definition) = predefined_strategy_definition(name)? else {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            format!("Error: Configuration name {name} not found."),
        ));
    };
    let mut scanner = Scanner::from_internal_string(&definition, true)?;
    heuristic_parms_parse_into(&mut scanner, target, false)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

/// Renders predefined strategies like C `StrategiesPrintPredefined`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded C schedule table cannot be parsed.
pub fn strategies_print_predefined_string(names_only: bool) -> Result<String, Diagnostic> {
    with_predefined_strategies(|strategies| {
        let mut result = String::new();
        for strategy in strategies {
            if names_only {
                result.push_str(&strategy.name);
                result.push('\n');
            } else {
                result.push_str(&strategy.name);
                result.push_str(" = \n");
                result.push_str(&strategy.definition);
                result.push('\n');
            }
        }
        result
    })
}

fn predefined_strategy_definition(name: &str) -> Result<Option<String>, Diagnostic> {
    with_predefined_strategies(|strategies| {
        strategies
            .iter()
            .find(|strategy| strategy.name == name)
            .map(|strategy| strategy.definition.clone())
    })
}

fn with_predefined_strategies<R>(
    callback: impl FnOnce(&[PredefinedStrategy]) -> R,
) -> Result<R, Diagnostic> {
    match PREDEFINED_STRATEGIES.get_or_init(parse_predefined_strategies) {
        Ok(strategies) => Ok(callback(strategies)),
        Err(error) => Err(error.clone()),
    }
}

fn parse_predefined_strategies() -> Result<Vec<PredefinedStrategy>, Diagnostic> {
    let start = SCHEDULE_VARS.find("StrStrPair conf_map[]").ok_or_else(|| {
        schedule_parse_error("Cannot find predefined strategy table conf_map in schedule.vars")
    })?;
    let open_offset = SCHEDULE_VARS[start..].find('{').ok_or_else(|| {
        schedule_parse_error("Cannot find opening brace for predefined strategy table")
    })?;
    let mut parser = ScheduleVarsParser::new(SCHEDULE_VARS, start + open_offset + 1);
    parser.parse_conf_map_entries()
}

fn schedule_parse_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, message)
}

struct ScheduleVarsParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> ScheduleVarsParser<'a> {
    const fn new(input: &'a str, position: usize) -> Self {
        Self { input, position }
    }

    fn parse_conf_map_entries(&mut self) -> Result<Vec<PredefinedStrategy>, Diagnostic> {
        let mut result = Vec::new();
        loop {
            self.skip_separators();
            self.expect_byte(b'{')?;
            self.skip_whitespace();
            if self.consume_identifier("NULL") {
                self.skip_until_entry_close()?;
                break;
            }
            let name = self.parse_c_string()?;
            self.skip_whitespace();
            self.expect_byte(b',')?;
            self.skip_whitespace();
            let definition = self.parse_c_string()?;
            self.skip_whitespace();
            self.expect_byte(b'}')?;
            result.push(PredefinedStrategy { name, definition });
        }
        Ok(result)
    }

    fn skip_separators(&mut self) {
        while let Some(byte) = self.current_byte() {
            if byte.is_ascii_whitespace() || byte == b',' {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.current_byte() {
            if byte.is_ascii_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn skip_until_entry_close(&mut self) -> Result<(), Diagnostic> {
        while let Some(byte) = self.current_byte() {
            self.position += 1;
            if byte == b'}' {
                return Ok(());
            }
        }
        Err(schedule_parse_error(
            "Unterminated predefined strategy table terminator",
        ))
    }

    fn consume_identifier(&mut self, expected: &str) -> bool {
        let remaining = &self.input[self.position..];
        if !remaining.starts_with(expected) {
            return false;
        }
        let end = self.position + expected.len();
        if self
            .input
            .as_bytes()
            .get(end)
            .is_some_and(u8::is_ascii_alphanumeric)
        {
            return false;
        }
        self.position = end;
        true
    }

    fn parse_c_string(&mut self) -> Result<String, Diagnostic> {
        self.expect_byte(b'"')?;
        let mut result = String::new();
        loop {
            let Some(byte) = self.take_byte() else {
                return Err(schedule_parse_error(
                    "Unterminated C string in predefined strategy table",
                ));
            };
            match byte {
                b'"' => return Ok(result),
                b'\\' => result.push(self.parse_c_escape()?),
                _ => result.push(char::from(byte)),
            }
        }
    }

    fn parse_c_escape(&mut self) -> Result<char, Diagnostic> {
        let Some(byte) = self.take_byte() else {
            return Err(schedule_parse_error(
                "Unterminated C escape in predefined strategy table",
            ));
        };
        Ok(match byte {
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'\\' => '\\',
            b'"' => '"',
            b'0' => '\0',
            _ => char::from(byte),
        })
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), Diagnostic> {
        match self.take_byte() {
            Some(byte) if byte == expected => Ok(()),
            Some(byte) => Err(schedule_parse_error(format!(
                "Expected '{}' in predefined strategy table, read '{}'",
                char::from(expected),
                char::from(byte)
            ))),
            None => Err(schedule_parse_error(format!(
                "Expected '{}' in predefined strategy table, reached end of file",
                char::from(expected)
            ))),
        }
    }

    fn current_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.position).copied()
    }

    fn take_byte(&mut self) -> Option<u8> {
        let byte = self.current_byte()?;
        self.position += 1;
        Some(byte)
    }
}

/// Prints a single heuristic parameter block.
#[must_use]
pub fn heuristic_parms_strategy_print_string(handle: &HeuristicParmsCell) -> String {
    heuristic_parms_print_string(handle)
}

#[cfg(test)]
mod tests {
    use super::{
        get_heuristic_with_name, strategies_print_predefined_string, with_predefined_strategies,
    };
    use crate::basics::error::ErrorCode;
    use crate::heuristics::hcb::HeuristicParmsCell;
    use crate::terms::termtypes::RewriteLevel;

    const FIRST_STRATEGY: &str = "G-E--_208_C12_11_nc_F1_SE_CS_SP_PS_S5PRR_S04BN";

    #[test]
    fn predefined_strategy_table_reads_conf_map_only() {
        let names = with_predefined_strategies(|strategies| {
            strategies
                .iter()
                .map(|strategy| strategy.name.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(names.first().map(String::as_str), Some(FIRST_STRATEGY));
        assert!(names.len() > 400);
        assert!(!names.iter().any(|name| name == "HGHSM-FSLF31-MHSFFSBC"));
    }

    #[test]
    fn predefined_strategy_name_print_matches_c_shape() {
        let printed =
            strategies_print_predefined_string(true).unwrap_or_else(|error| panic!("{error}"));

        assert!(printed.starts_with(FIRST_STRATEGY));
        assert!(printed.ends_with('\n'));
        assert!(printed.lines().count() > 400);
        assert!(!printed.contains(" = "));
    }

    #[test]
    fn predefined_strategy_full_print_includes_definition() {
        let printed =
            strategies_print_predefined_string(false).unwrap_or_else(|error| panic!("{error}"));

        assert!(printed.starts_with(&format!("{FIRST_STRATEGY} = \n#{FIRST_STRATEGY}\n")));
        assert!(printed.contains("selection_strategy: PSelectComplexExceptUniqMaxHorn"));
    }

    #[test]
    fn get_heuristic_with_name_parses_predefined_strategy() {
        let mut params = HeuristicParmsCell::default();

        get_heuristic_with_name(FIRST_STRATEGY, &mut params)
            .unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(params.heuristic_name, "Default");
        assert_eq!(params.selection_strategy, "PSelectComplexExceptUniqMaxHorn");
        assert_eq!(params.forward_demod, RewriteLevel::FullRewrite);
    }

    #[test]
    fn get_heuristic_with_name_rejects_unknown_strategy() {
        let mut params = HeuristicParmsCell::default();
        let error = get_heuristic_with_name("Missing", &mut params).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            error.message(),
            "Error: Configuration name Missing not found."
        );
    }
}
