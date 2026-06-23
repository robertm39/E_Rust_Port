use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::scanner::Scanner;
use crate::terms::functypes::{func_symb_parse, FuncSymbType};
use std::collections::BTreeMap;
use std::io::{self, Write};

pub type SortType = i32;

pub const ST_NO_SORT: SortType = 0;
pub const ST_BOOL: SortType = 1;
pub const ST_INDIVIDUALS: SortType = 2;
pub const ST_KIND: SortType = 3;
pub const ST_INTEGER: SortType = 4;
pub const ST_RATIONAL: SortType = 5;
pub const ST_REAL: SortType = 6;
pub const ST_PREDEFINED: SortType = ST_REAL;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SortTable {
    default_type: SortType,
    sort_index: BTreeMap<String, SortType>,
    back_index: Vec<String>,
}

impl Default for SortTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SortTable {
    #[must_use]
    pub fn new() -> Self {
        Self {
            default_type: ST_INDIVIDUALS,
            sort_index: BTreeMap::new(),
            back_index: Vec::new(),
        }
    }

    #[must_use]
    pub fn default_table() -> Self {
        let mut table = Self::new();
        table.init_defaults();
        table
    }

    #[must_use]
    pub const fn default_type(&self) -> SortType {
        self.default_type
    }

    pub fn set_default_type(&mut self, default_type: SortType) {
        self.default_type = default_type;
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.back_index.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.back_index.is_empty()
    }

    pub fn insert(&mut self, sort_name: &str) -> SortType {
        if let Some(sort) = self.sort_index.get(sort_name) {
            return *sort;
        }
        let sort = i32::try_from(self.back_index.len()).unwrap_or(i32::MAX);
        self.sort_index.insert(sort_name.to_owned(), sort);
        self.back_index.push(sort_name.to_owned());
        sort
    }

    #[must_use]
    pub fn get_rep(&self, sort: SortType) -> Option<&str> {
        usize::try_from(sort)
            .ok()
            .and_then(|index| self.back_index.get(index))
            .map(String::as_str)
    }

    pub fn parse_tstp(&mut self, scanner: &mut Scanner) -> Result<SortType, Diagnostic> {
        let mut id = DynamicString::new();
        let func_type = func_symb_parse(scanner, &mut id)?;
        if matches!(
            func_type,
            FuncSymbType::IdentFreeFun | FuncSymbType::IdentInterpreted
        ) {
            Ok(self.insert(&id.view()))
        } else {
            Err(Diagnostic::new(
                ErrorCode::SYNTAX_ERROR,
                "Expected TSTP sort",
            ))
        }
    }

    pub fn print_tstp(&self, output: &mut impl Write, sort: SortType) -> io::Result<bool> {
        let Some(name) = self.get_rep(sort) else {
            return Ok(false);
        };
        output.write_all(name.as_bytes())?;
        Ok(true)
    }

    pub fn print_table(&self, output: &mut impl Write) -> io::Result<()> {
        writeln!(output, "Sort table in order of sort creation:")?;
        writeln!(output, "=====================================")?;
        for (sort, name) in self.back_index.iter().enumerate() {
            writeln!(output, "Type {sort:4}: {name}")?;
        }
        writeln!(output)?;
        writeln!(output, "Sort table in alphabetic order:")?;
        writeln!(output, "=====================================")?;
        for (name, sort) in &self.sort_index {
            writeln!(output, "Type {sort:4}: {name}")?;
        }
        writeln!(output)?;
        Ok(())
    }

    fn init_defaults(&mut self) {
        debug_assert_eq!(self.insert("$no_type"), ST_NO_SORT);
        debug_assert_eq!(self.insert("$o"), ST_BOOL);
        debug_assert_eq!(self.insert("$i"), ST_INDIVIDUALS);
        debug_assert_eq!(self.insert("$tType"), ST_KIND);
        debug_assert_eq!(self.insert("$int"), ST_INTEGER);
        debug_assert_eq!(self.insert("$rat"), ST_RATIONAL);
        debug_assert_eq!(self.insert("$real"), ST_REAL);
    }
}

#[must_use]
pub fn sort_is_user_defined(sort: SortType) -> bool {
    sort > ST_PREDEFINED
}

#[must_use]
pub fn sort_is_interpreted(sort: SortType) -> bool {
    (ST_INTEGER..=ST_REAL).contains(&sort)
}

#[cfg(test)]
mod tests {
    use super::{
        sort_is_interpreted, sort_is_user_defined, SortTable, ST_BOOL, ST_INDIVIDUALS, ST_INTEGER,
        ST_KIND, ST_NO_SORT, ST_PREDEFINED, ST_RATIONAL, ST_REAL,
    };
    use crate::basics::error::ErrorCode;
    use crate::inout::scanner::Scanner;

    fn scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).unwrap()
    }

    #[test]
    fn constants_and_sort_predicates_match_c_header() {
        assert_eq!(ST_NO_SORT, 0);
        assert_eq!(ST_BOOL, 1);
        assert_eq!(ST_INDIVIDUALS, 2);
        assert_eq!(ST_KIND, 3);
        assert_eq!(ST_INTEGER, 4);
        assert_eq!(ST_RATIONAL, 5);
        assert_eq!(ST_REAL, 6);
        assert_eq!(ST_PREDEFINED, ST_REAL);
        assert!(sort_is_user_defined(ST_PREDEFINED + 1));
        assert!(!sort_is_user_defined(ST_REAL));
        assert!(sort_is_interpreted(ST_INTEGER));
        assert!(sort_is_interpreted(ST_RATIONAL));
        assert!(sort_is_interpreted(ST_REAL));
        assert!(!sort_is_interpreted(ST_BOOL));
    }

    #[test]
    fn empty_table_defaults_to_individuals_and_inserts_in_order() {
        let mut table = SortTable::new();
        assert_eq!(table.default_type(), ST_INDIVIDUALS);
        assert!(table.is_empty());

        assert_eq!(table.insert("alpha"), 0);
        assert_eq!(table.insert("beta"), 1);
        assert_eq!(table.insert("alpha"), 0);
        assert_eq!(table.len(), 2);
        assert_eq!(table.get_rep(0), Some("alpha"));
        assert_eq!(table.get_rep(1), Some("beta"));
        assert_eq!(table.get_rep(2), None);

        table.set_default_type(1);
        assert_eq!(table.default_type(), 1);
    }

    #[test]
    fn default_table_inserts_predefined_sorts_in_reserved_order() {
        let table = SortTable::default_table();

        assert_eq!(table.len(), 7);
        assert_eq!(table.get_rep(ST_NO_SORT), Some("$no_type"));
        assert_eq!(table.get_rep(ST_BOOL), Some("$o"));
        assert_eq!(table.get_rep(ST_INDIVIDUALS), Some("$i"));
        assert_eq!(table.get_rep(ST_KIND), Some("$tType"));
        assert_eq!(table.get_rep(ST_INTEGER), Some("$int"));
        assert_eq!(table.get_rep(ST_RATIONAL), Some("$rat"));
        assert_eq!(table.get_rep(ST_REAL), Some("$real"));
    }

    #[test]
    fn parse_tstp_accepts_free_and_interpreted_function_symbols() {
        let mut table = SortTable::default_table();
        let mut input = scanner("$o custom $custom");

        assert_eq!(table.parse_tstp(&mut input).unwrap(), ST_BOOL);
        let custom = table.parse_tstp(&mut input).unwrap();
        assert!(sort_is_user_defined(custom));
        assert_eq!(table.get_rep(custom), Some("custom"));
        let interpreted = table.parse_tstp(&mut input).unwrap();
        assert!(sort_is_user_defined(interpreted));
        assert_eq!(table.get_rep(interpreted), Some("$custom"));
    }

    #[test]
    fn parse_tstp_rejects_variables_objects_and_numbers() {
        for source in ["X", "\"object\"", "123"] {
            let mut table = SortTable::default_table();
            let mut input = scanner(source);
            let error = table.parse_tstp(&mut input).unwrap_err();
            assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
            assert!(error.message().contains("Expected TSTP sort"));
        }
    }

    #[test]
    fn print_helpers_match_c_debug_shape() {
        let mut table = SortTable::default_table();
        let custom = table.insert("custom");

        let mut output = Vec::new();
        assert!(table.print_tstp(&mut output, custom).unwrap());
        assert_eq!(String::from_utf8(output).unwrap(), "custom");

        let mut missing = Vec::new();
        assert!(!table.print_tstp(&mut missing, 99).unwrap());
        assert!(missing.is_empty());

        let mut debug = Vec::new();
        table.print_table(&mut debug).unwrap();
        let debug = String::from_utf8(debug).unwrap();
        assert!(debug.contains("Sort table in order of sort creation:"));
        assert!(debug.contains("Type    0: $no_type"));
        assert!(debug.contains("Sort table in alphabetic order:"));
        assert!(debug.contains(&format!("Type {custom:4}: custom")));
    }
}
