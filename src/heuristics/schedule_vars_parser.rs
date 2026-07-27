//! Build-time parser for upstream's generated `schedule.vars` data.

use std::collections::HashSet;

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedScheduleData {
    pub(crate) strategies: Vec<ParsedStrategy>,
    pub(crate) schedules: Vec<ParsedSchedule>,
    pub(crate) preprocessing_map: Vec<ParsedScheduleClass>,
    pub(crate) search_map: Vec<ParsedScheduleClass>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ParsedStrategy {
    pub(crate) name: String,
    pub(crate) definition: String,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedSchedule {
    pub(crate) name: String,
    pub(crate) cells: Vec<ParsedScheduleCell>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedScheduleCell {
    pub(crate) heuristic_name: String,
    pub(crate) ordering: String,
    pub(crate) sine: Option<String>,
    pub(crate) time_fraction: f64,
    pub(crate) time_absolute: u64,
    pub(crate) cores: i32,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ParsedScheduleClass {
    pub(crate) key: String,
    pub(crate) schedule_name: String,
    pub(crate) class_size: i32,
}

pub(crate) fn parse_schedule_vars(input: &str) -> Result<ParsedScheduleData, String> {
    let strategies = parse_predefined_strategies(input)?;
    let schedules = parse_schedule_cell_arrays(input)?;
    let preprocessing_map = parse_schedule_class_map(input, "preproc_sched_map")?;
    let search_map = parse_schedule_class_map(input, "search_sched_map")?;
    validate_schedule_references(&schedules, &preprocessing_map)?;
    validate_schedule_references(&schedules, &search_map)?;
    if !schedules
        .iter()
        .any(|schedule| schedule.name == "_DEFAULT_SCHEDULE")
    {
        return Err("Cannot find _DEFAULT_SCHEDULE in schedule.vars".to_owned());
    }

    Ok(ParsedScheduleData {
        strategies,
        schedules,
        preprocessing_map,
        search_map,
    })
}

fn parse_predefined_strategies(input: &str) -> Result<Vec<ParsedStrategy>, String> {
    let start = input.find("StrStrPair conf_map[]").ok_or_else(|| {
        "Cannot find predefined strategy table conf_map in schedule.vars".to_owned()
    })?;
    let open_offset = input[start..]
        .find('{')
        .ok_or_else(|| "Cannot find opening brace for predefined strategy table".to_owned())?;
    let mut parser = ScheduleVarsParser::new(input, start + open_offset + 1);
    parser.parse_conf_map_entries()
}

fn parse_schedule_cell_arrays(input: &str) -> Result<Vec<ParsedSchedule>, String> {
    let mut parser = ScheduleVarsParser::new(input, 0);
    let mut arrays = Vec::new();
    let mut names = HashSet::new();
    while let Some(position) = parser.find_from("ScheduleCell ") {
        parser.position = position + "ScheduleCell ".len();
        let schedule = parser.parse_schedule_cell_array_after_keyword()?;
        if !names.insert(schedule.name.clone()) {
            return Err(format!("Duplicate schedule array {}", schedule.name));
        }
        arrays.push(schedule);
    }
    Ok(arrays)
}

fn parse_schedule_class_map(input: &str, name: &str) -> Result<Vec<ParsedScheduleClass>, String> {
    let start = input
        .find(&format!("StrSchedPair {name}[]"))
        .ok_or_else(|| format!("Cannot find {name} in schedule.vars"))?;
    let open_offset = input[start..]
        .find('{')
        .ok_or_else(|| format!("Cannot find opening brace for {name}"))?;
    let mut parser = ScheduleVarsParser::new(input, start + open_offset + 1);
    parser.parse_schedule_class_entries()
}

fn validate_schedule_references(
    schedules: &[ParsedSchedule],
    entries: &[ParsedScheduleClass],
) -> Result<(), String> {
    let names = schedules
        .iter()
        .map(|schedule| schedule.name.as_str())
        .collect::<HashSet<_>>();
    for entry in entries {
        if !names.contains(entry.schedule_name.as_str()) {
            return Err(format!(
                "Schedule map references unknown array {}",
                entry.schedule_name
            ));
        }
    }
    Ok(())
}

struct ScheduleVarsParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> ScheduleVarsParser<'a> {
    const fn new(input: &'a str, position: usize) -> Self {
        Self { input, position }
    }

    fn parse_conf_map_entries(&mut self) -> Result<Vec<ParsedStrategy>, String> {
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
            self.expect_comma()?;
            let definition = self.parse_c_string()?;
            self.skip_whitespace();
            self.expect_byte(b'}')?;
            result.push(ParsedStrategy { name, definition });
        }
        Ok(result)
    }

    fn parse_schedule_cell_array_after_keyword(&mut self) -> Result<ParsedSchedule, String> {
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
        Ok(ParsedSchedule { name, cells })
    }

    fn parse_schedule_cell_entry(&mut self) -> Result<Option<ParsedScheduleCell>, String> {
        let heuristic_name = self.parse_optional_c_string_or_null()?;
        self.expect_comma()?;
        let ordering = self.parse_identifier()?;
        self.expect_comma()?;
        let sine = self.parse_optional_c_string_or_null()?;
        self.expect_comma()?;
        let time_fraction = self.parse_float()?;
        self.expect_comma()?;
        let time_absolute = self.parse_u64()?;
        self.expect_comma()?;
        let cores = self.parse_i32()?;

        Ok(heuristic_name.map(|heuristic_name| ParsedScheduleCell {
            heuristic_name,
            ordering,
            sine,
            time_fraction,
            time_absolute,
            cores,
        }))
    }

    fn parse_schedule_class_entries(&mut self) -> Result<Vec<ParsedScheduleClass>, String> {
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
            result.push(ParsedScheduleClass {
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

    fn expect_comma(&mut self) -> Result<(), String> {
        self.skip_whitespace();
        self.expect_byte(b',')
    }

    fn skip_until_entry_close(&mut self) -> Result<(), String> {
        while let Some(byte) = self.current_byte() {
            self.position += 1;
            if byte == b'}' {
                return Ok(());
            }
        }
        Err("Unterminated generated-table terminator".to_owned())
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

    fn parse_identifier(&mut self) -> Result<String, String> {
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
            return Err("Expected identifier in generated schedule table".to_owned());
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_optional_c_string_or_null(&mut self) -> Result<Option<String>, String> {
        self.skip_whitespace();
        if self.consume_identifier("NULL") {
            Ok(None)
        } else {
            self.parse_c_string().map(Some)
        }
    }

    fn parse_float(&mut self) -> Result<f64, String> {
        let token = self.parse_number_token()?;
        token
            .parse::<f64>()
            .map_err(|_| format!("Invalid floating-point value {token} in schedule.vars"))
    }

    fn parse_i32(&mut self) -> Result<i32, String> {
        let token = self.parse_number_token()?;
        token
            .parse::<i32>()
            .map_err(|_| format!("Invalid signed integer value {token} in schedule.vars"))
    }

    fn parse_u64(&mut self) -> Result<u64, String> {
        let token = self.parse_number_token()?;
        token
            .parse::<u64>()
            .map_err(|_| format!("Invalid unsigned integer value {token} in schedule.vars"))
    }

    fn parse_number_token(&mut self) -> Result<String, String> {
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
            return Err("Expected numeric value in generated schedule table".to_owned());
        }
        Ok(self.input[start..self.position].to_owned())
    }

    fn parse_c_string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        self.expect_byte(b'"')?;
        let mut result = String::new();
        loop {
            let Some(byte) = self.take_byte() else {
                return Err("Unterminated C string in generated schedule table".to_owned());
            };
            match byte {
                b'"' => return Ok(result),
                b'\\' => result.push(self.parse_c_escape()?),
                _ => result.push(char::from(byte)),
            }
        }
    }

    fn parse_c_escape(&mut self) -> Result<char, String> {
        let Some(byte) = self.take_byte() else {
            return Err("Unterminated C escape in generated schedule table".to_owned());
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

    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        match self.take_byte() {
            Some(byte) if byte == expected => Ok(()),
            Some(byte) => Err(format!(
                "Expected '{}' in generated schedule table, read '{}'",
                char::from(expected),
                char::from(byte)
            )),
            None => Err(format!(
                "Expected '{}' in generated schedule table, reached end of file",
                char::from(expected)
            )),
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
