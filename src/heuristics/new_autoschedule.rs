//! Predefined strategy lookup from C `che_new_autoschedule`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::heuristics::hcb::{
    heuristic_parms_parse_into, heuristic_parms_print_string, HeuristicParmsCell,
};
use crate::heuristics::to_params::{TermOrdering, TERM_ORDERING_NAMES};
use crate::inout::scanner::{Scanner, TokenType};

const SCHEDULE_VARS: &str = include_str!("../../eprover/HEURISTICS/schedule.vars");

#[derive(Clone, Debug, Eq, PartialEq)]
struct PredefinedStrategy {
    name: String,
    definition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleCell {
    pub heuristic_name: String,
    pub ordering: TermOrdering,
    pub sine: Option<String>,
    pub time_fraction: f64,
    pub time_absolute: u64,
    pub cores: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScheduleClass {
    key: String,
    schedule_name: String,
    class_size: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedSchedule {
    pub matched_class: String,
    pub distance: usize,
    pub class_size: i32,
    pub schedule: Vec<ScheduleCell>,
}

#[derive(Clone, Debug, PartialEq)]
struct AutoSchedules {
    schedules: HashMap<String, Vec<ScheduleCell>>,
    preprocessing_map: Vec<ScheduleClass>,
    search_map: Vec<ScheduleClass>,
    default_schedule: Vec<ScheduleCell>,
}

static PREDEFINED_STRATEGIES: OnceLock<Result<Vec<PredefinedStrategy>, Diagnostic>> =
    OnceLock::new();
static AUTO_SCHEDULES: OnceLock<Result<AutoSchedules, Diagnostic>> = OnceLock::new();

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

/// Returns C `GetPreprocessingSchedule(problem_category)`.
///
/// The returned `ResolvedSchedule` includes the selected class metadata so
/// callers can reproduce C's partial-match comment when `distance != 0`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded generated schedule table cannot be
/// parsed or references an unknown schedule array.
pub fn get_preprocessing_schedule(problem_category: &str) -> Result<ResolvedSchedule, Diagnostic> {
    with_auto_schedules(|schedules| {
        resolve_schedule(problem_category, &schedules.preprocessing_map, schedules)
    })?
}

/// Returns C `GetSearchSchedule(problem_category)`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded generated schedule table cannot be
/// parsed or references an unknown schedule array.
pub fn get_search_schedule(problem_category: &str) -> Result<ResolvedSchedule, Diagnostic> {
    with_auto_schedules(|schedules| {
        resolve_schedule(problem_category, &schedules.search_map, schedules)
    })?
}

/// Returns C `GetDefaultSchedule()`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded generated schedule table cannot be
/// parsed or the default schedule array is missing.
pub fn get_default_schedule() -> Result<Vec<ScheduleCell>, Diagnostic> {
    with_auto_schedules(|schedules| schedules.default_schedule.clone())
}

/// C `StrDistance`: positional character mismatches plus length difference.
#[must_use]
pub fn schedule_string_distance(left: &str, right: &str) -> usize {
    let mut distance = 0_usize;
    let mut left_chars = left.chars();
    let mut right_chars = right.chars();
    loop {
        match (left_chars.next(), right_chars.next()) {
            (Some(left_char), Some(right_char)) => {
                distance += usize::from(left_char != right_char);
            }
            (Some(_), None) => {
                distance += 1 + left_chars.count();
                break;
            }
            (None, Some(_)) => {
                distance += 1 + right_chars.count();
                break;
            }
            (None, None) => break,
        }
    }
    distance
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

fn with_auto_schedules<R>(callback: impl FnOnce(&AutoSchedules) -> R) -> Result<R, Diagnostic> {
    match AUTO_SCHEDULES.get_or_init(parse_auto_schedules) {
        Ok(schedules) => Ok(callback(schedules)),
        Err(error) => Err(error.clone()),
    }
}

fn parse_auto_schedules() -> Result<AutoSchedules, Diagnostic> {
    let schedules = parse_schedule_cell_arrays()?;
    let preprocessing_map = parse_schedule_class_map("preproc_sched_map")?;
    let search_map = parse_schedule_class_map("search_sched_map")?;
    validate_schedule_map(&schedules, &preprocessing_map)?;
    validate_schedule_map(&schedules, &search_map)?;
    let default_schedule = schedules
        .get("_DEFAULT_SCHEDULE")
        .cloned()
        .ok_or_else(|| schedule_parse_error("Cannot find _DEFAULT_SCHEDULE in schedule.vars"))?;

    Ok(AutoSchedules {
        schedules,
        preprocessing_map,
        search_map,
        default_schedule,
    })
}

fn parse_schedule_cell_arrays() -> Result<HashMap<String, Vec<ScheduleCell>>, Diagnostic> {
    let mut parser = ScheduleVarsParser::new(SCHEDULE_VARS, 0);
    let mut arrays = HashMap::new();
    while let Some(position) = parser.find_from("ScheduleCell ") {
        parser.position = position + "ScheduleCell ".len();
        let (name, cells) = parser.parse_schedule_cell_array_after_keyword()?;
        arrays.insert(name, cells);
    }
    Ok(arrays)
}

fn parse_schedule_class_map(name: &str) -> Result<Vec<ScheduleClass>, Diagnostic> {
    let start = SCHEDULE_VARS
        .find(&format!("StrSchedPair {name}[]"))
        .ok_or_else(|| schedule_parse_error(format!("Cannot find {name} in schedule.vars")))?;
    let open_offset = SCHEDULE_VARS[start..]
        .find('{')
        .ok_or_else(|| schedule_parse_error(format!("Cannot find opening brace for {name}")))?;
    let mut parser = ScheduleVarsParser::new(SCHEDULE_VARS, start + open_offset + 1);
    parser.parse_schedule_class_entries()
}

fn validate_schedule_map(
    schedules: &HashMap<String, Vec<ScheduleCell>>,
    entries: &[ScheduleClass],
) -> Result<(), Diagnostic> {
    for entry in entries {
        if !schedules.contains_key(&entry.schedule_name) {
            return Err(schedule_parse_error(format!(
                "Schedule map references unknown array {}",
                entry.schedule_name
            )));
        }
    }
    Ok(())
}

fn resolve_schedule(
    problem_category: &str,
    entries: &[ScheduleClass],
    schedules: &AutoSchedules,
) -> Result<ResolvedSchedule, Diagnostic> {
    let (entry, distance) = select_schedule_class(problem_category, entries)
        .ok_or_else(|| schedule_parse_error("Schedule class map is empty"))?;
    let schedule = schedules
        .schedules
        .get(&entry.schedule_name)
        .cloned()
        .ok_or_else(|| {
            schedule_parse_error(format!(
                "Schedule map references unknown array {}",
                entry.schedule_name
            ))
        })?;

    Ok(ResolvedSchedule {
        matched_class: entry.key.clone(),
        distance,
        class_size: entry.class_size,
        schedule,
    })
}

fn select_schedule_class<'a>(
    problem_category: &str,
    entries: &'a [ScheduleClass],
) -> Option<(&'a ScheduleClass, usize)> {
    let mut selected = None;
    let mut min_distance = usize::MAX;
    let mut max_class_size = i32::MIN;

    for entry in entries {
        let distance = schedule_string_distance(&entry.key, problem_category);
        if distance == 0 {
            return Some((entry, distance));
        }
        if distance < min_distance
            || (distance == min_distance && entry.class_size > max_class_size)
        {
            selected = Some(entry);
            min_distance = distance;
            max_class_size = entry.class_size;
        }
    }

    selected.map(|entry| (entry, min_distance))
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

    fn parse_schedule_cell_array_after_keyword(
        &mut self,
    ) -> Result<(String, Vec<ScheduleCell>), Diagnostic> {
        let name = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect_byte(b'[')?;
        self.expect_byte(b']')?;
        self.skip_whitespace();
        self.expect_byte(b'=')?;
        self.skip_whitespace();
        self.expect_byte(b'{')?;

        let mut cells = Vec::new();
        loop {
            self.skip_separators();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b'{')?;
            if let Some(cell) = self.parse_schedule_cell_entry()? {
                cells.push(cell);
            }
            self.skip_whitespace();
            self.expect_byte(b'}')?;
        }
        self.skip_whitespace();
        self.expect_byte(b';')?;
        Ok((name, cells))
    }

    fn parse_schedule_cell_entry(&mut self) -> Result<Option<ScheduleCell>, Diagnostic> {
        let heuristic_name = self.parse_optional_c_string_or_null()?;
        self.expect_comma()?;
        let ordering = self.parse_term_ordering()?;
        self.expect_comma()?;
        let sine = self.parse_optional_c_string_or_null()?;
        self.expect_comma()?;
        let time_fraction = self.parse_float()?;
        self.expect_comma()?;
        let time_absolute = self.parse_u64()?;
        self.expect_comma()?;
        let cores = self.parse_i32()?;

        Ok(heuristic_name.map(|heuristic_name| ScheduleCell {
            heuristic_name,
            ordering,
            sine,
            time_fraction,
            time_absolute,
            cores,
        }))
    }

    fn parse_schedule_class_entries(&mut self) -> Result<Vec<ScheduleClass>, Diagnostic> {
        let mut result = Vec::new();
        loop {
            self.skip_separators();
            self.expect_byte(b'{')?;
            self.skip_whitespace();
            if self.consume_identifier("NULL") {
                self.skip_until_entry_close()?;
                break;
            }
            let key = self.parse_c_string()?;
            self.expect_comma()?;
            let schedule_name = self.parse_identifier()?;
            self.expect_comma()?;
            let class_size = self.parse_i32()?;
            self.skip_whitespace();
            self.expect_byte(b'}')?;
            result.push(ScheduleClass {
                key,
                schedule_name,
                class_size,
            });
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

    fn expect_comma(&mut self) -> Result<(), Diagnostic> {
        self.skip_whitespace();
        self.expect_byte(b',')
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
            .is_some_and(|byte| is_identifier_continuation(*byte))
        {
            return false;
        }
        self.position = end;
        true
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        self.skip_whitespace();
        if self.current_byte() == Some(expected) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn parse_identifier(&mut self) -> Result<String, Diagnostic> {
        self.skip_whitespace();
        let start = self.position;
        while let Some(byte) = self.current_byte() {
            if is_identifier_continuation(byte) {
                self.position += 1;
            } else {
                break;
            }
        }
        if self.position == start {
            return Err(schedule_parse_error(
                "Expected identifier in predefined schedule table",
            ));
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_optional_c_string_or_null(&mut self) -> Result<Option<String>, Diagnostic> {
        self.skip_whitespace();
        if self.consume_identifier("NULL") {
            Ok(None)
        } else {
            self.parse_c_string().map(Some)
        }
    }

    fn parse_term_ordering(&mut self) -> Result<TermOrdering, Diagnostic> {
        let name = self.parse_identifier()?;
        let Some(index) = TERM_ORDERING_NAMES
            .iter()
            .position(|candidate| *candidate == name)
        else {
            return Err(schedule_parse_error(format!(
                "Unknown term ordering {name} in schedule.vars"
            )));
        };
        let value = i32::try_from(index)
            .map_err(|_| schedule_parse_error("Term-ordering index does not fit C enum"))?;
        TermOrdering::from_c_value(value)
            .ok_or_else(|| schedule_parse_error(format!("Invalid term ordering {name}")))
    }

    fn parse_float(&mut self) -> Result<f64, Diagnostic> {
        let token = self.parse_number_token()?;
        token.parse::<f64>().map_err(|_| {
            schedule_parse_error(format!(
                "Invalid floating-point value {token} in schedule.vars"
            ))
        })
    }

    fn parse_i32(&mut self) -> Result<i32, Diagnostic> {
        let token = self.parse_number_token()?;
        token.parse::<i32>().map_err(|_| {
            schedule_parse_error(format!(
                "Invalid signed integer value {token} in schedule.vars"
            ))
        })
    }

    fn parse_u64(&mut self) -> Result<u64, Diagnostic> {
        let token = self.parse_number_token()?;
        token.parse::<u64>().map_err(|_| {
            schedule_parse_error(format!(
                "Invalid unsigned integer value {token} in schedule.vars"
            ))
        })
    }

    fn parse_number_token(&mut self) -> Result<String, Diagnostic> {
        self.skip_whitespace();
        let start = self.position;
        while let Some(byte) = self.current_byte() {
            if byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'+' | b'e' | b'E') {
                self.position += 1;
            } else {
                break;
            }
        }
        if self.position == start {
            return Err(schedule_parse_error(
                "Expected numeric value in predefined schedule table",
            ));
        }
        Ok(self.input[start..self.position].to_owned())
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

    fn find_from(&self, needle: &str) -> Option<usize> {
        self.input[self.position..]
            .find(needle)
            .map(|offset| self.position + offset)
    }
}

const fn is_identifier_continuation(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Prints a single heuristic parameter block.
#[must_use]
pub fn heuristic_parms_strategy_print_string(handle: &HeuristicParmsCell) -> String {
    heuristic_parms_print_string(handle)
}

#[cfg(test)]
mod tests {
    use super::{
        get_default_schedule, get_heuristic_with_name, get_preprocessing_schedule,
        get_search_schedule, schedule_string_distance, select_schedule_class,
        strategies_print_predefined_string, with_predefined_strategies, ScheduleClass,
    };
    use crate::basics::error::ErrorCode;
    use crate::heuristics::hcb::HeuristicParmsCell;
    use crate::heuristics::to_params::TermOrdering;
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

    #[test]
    fn schedule_string_distance_matches_c_positional_difference() {
        assert_eq!(schedule_string_distance("ABC", "ABC"), 0);
        assert_eq!(schedule_string_distance("ABC", "AXC"), 1);
        assert_eq!(schedule_string_distance("ABC", "AXCD"), 2);
        assert_eq!(schedule_string_distance("ABCD", "AX"), 3);
    }

    #[test]
    fn class_selection_uses_c_exact_and_largest_same_distance_tie_breaks() {
        let classes = vec![
            ScheduleClass {
                key: "AAAA".to_owned(),
                schedule_name: "small".to_owned(),
                class_size: 1,
            },
            ScheduleClass {
                key: "AAAB".to_owned(),
                schedule_name: "large".to_owned(),
                class_size: 5,
            },
            ScheduleClass {
                key: "AAAC".to_owned(),
                schedule_name: "exact".to_owned(),
                class_size: 0,
            },
        ];

        let (partial, distance) =
            select_schedule_class("AAAD", &classes).expect("non-empty schedule map");
        assert_eq!(partial.schedule_name, "large");
        assert_eq!(distance, 1);

        let (exact, distance) =
            select_schedule_class("AAAC", &classes).expect("non-empty schedule map");
        assert_eq!(exact.schedule_name, "exact");
        assert_eq!(distance, 0);
    }

    #[test]
    fn generated_schedule_tables_resolve_preprocessing_search_and_default() {
        let default_schedule = get_default_schedule().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(default_schedule.len(), 8);
        assert_eq!(
            default_schedule
                .first()
                .map(|cell| cell.heuristic_name.as_str()),
            Some("G-E--_208_C18C--_F1_SE_CS_SP_PS_S5PRR_RG_S04AN")
        );
        assert_eq!(
            default_schedule.first().map(|cell| cell.ordering),
            Some(TermOrdering::NoOrdering)
        );

        let preprocessing =
            get_preprocessing_schedule("FSLMSMSLSSSNFFN").unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(preprocessing.matched_class, "FSLMSMSLSSSNFFN");
        assert_eq!(preprocessing.distance, 0);
        assert_eq!(preprocessing.class_size, 456);
        assert_eq!(preprocessing.schedule.len(), 4);
        assert_eq!(
            preprocessing.schedule[0].heuristic_name,
            "G-E--_008_C45_F1_PI_SE_Q4_CS_SP_S4SI"
        );

        let search =
            get_search_schedule("FGHSF-FSLM21-MFFFFFNN").unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(search.matched_class, "FGHSF-FSLM21-MFFFFFNN");
        assert_eq!(search.distance, 0);
        assert_eq!(search.class_size, 523);
        assert_eq!(search.schedule.len(), 11);
        assert_eq!(
            search.schedule[10].heuristic_name, "<placeholder>",
            "search schedules preserve the C placeholder cell for later insertion"
        );
    }

    #[test]
    fn generated_schedule_partial_match_reports_selected_class() {
        let resolved =
            get_search_schedule("FGHSF-FSLM21-MFFFFFNX").unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(resolved.matched_class, "FGHSF-FSLM21-MFFFFFNN");
        assert_eq!(resolved.distance, 1);
        assert_eq!(resolved.class_size, 523);
    }
}
