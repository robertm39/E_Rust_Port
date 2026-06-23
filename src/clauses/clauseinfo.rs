#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseInfo {
    name: Option<String>,
    source: Option<String>,
    line: i64,
    column: i64,
}

impl ClauseInfo {
    #[must_use]
    pub fn new(name: Option<&str>, source: Option<&str>, line: i64, column: i64) -> Self {
        Self {
            name: name.map(str::to_owned),
            source: source.map(str::to_owned),
            line,
            column,
        }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self {
            name: None,
            source: None,
            line: -1,
            column: -1,
        }
    }

    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    #[must_use]
    pub const fn line(&self) -> i64 {
        self.line
    }

    #[must_use]
    pub const fn column(&self) -> i64 {
        self.column
    }

    /// Formats source metadata in the C `ClauseSourceInfoPrint` shape.
    ///
    /// # Panics
    ///
    /// Panics when the clause has no name, a non-negative line, and a negative
    /// column. The C implementation encodes this as an assertion.
    #[must_use]
    pub fn source_info_string(&self, inf_lit: &str, delim: &str) -> String {
        let source = self.source.as_ref().map_or_else(
            || "unknown".to_owned(),
            |source| format!("{delim}{source}{delim}"),
        );
        let name = self.name.clone().unwrap_or_else(|| {
            if self.line < 0 {
                "unknown".to_owned()
            } else {
                assert!(
                    self.column >= 0,
                    "clause source column must be non-negative when line is known"
                );
                format!("at_line_{}_column_{}", self.line, self.column)
            }
        });

        format!("{inf_lit}({source}, {name})")
    }

    #[must_use]
    pub fn source_info_tstp_string(&self) -> String {
        self.source_info_string("file", "'")
    }

    #[must_use]
    pub fn source_info_pcl_string(&self) -> String {
        self.source_info_string("initial", "\"")
    }

    #[must_use]
    pub fn id_namespace(&self) -> i64 {
        let Some(name) = self.name.as_deref() else {
            return -1;
        };
        get_id_namespace(name)
    }

    #[must_use]
    pub fn id_counter(&self) -> i64 {
        let Some(name) = self.name.as_deref() else {
            return -1;
        };
        get_id_counter(name)
    }
}

#[must_use]
pub fn source_info_string(info: Option<&ClauseInfo>, inf_lit: &str, delim: &str) -> String {
    info.map_or_else(String::new, |info| info.source_info_string(inf_lit, delim))
}

#[must_use]
pub fn source_info_tstp_string(info: Option<&ClauseInfo>) -> String {
    source_info_string(info, "file", "'")
}

#[must_use]
pub fn source_info_pcl_string(info: Option<&ClauseInfo>) -> String {
    source_info_string(info, "initial", "\"")
}

fn get_id_namespace(name: &str) -> i64 {
    let Some(tail) = generated_id_tail(name) else {
        return -1;
    };
    let (namespace, rest) = c_strtol_prefix(tail);
    if rest.starts_with('_') {
        namespace
    } else {
        -1
    }
}

fn get_id_counter(name: &str) -> i64 {
    let Some(tail) = generated_id_tail(name) else {
        return -1;
    };
    let (_, rest) = c_strtol_prefix(tail);
    let Some(counter_text) = rest.strip_prefix('_') else {
        return -1;
    };
    let (counter, rest) = c_strtol_prefix(counter_text);
    if rest.is_empty() {
        counter
    } else {
        -1
    }
}

fn generated_id_tail(name: &str) -> Option<&str> {
    if !(name.starts_with("i_") || name.starts_with("c_")) {
        return None;
    }
    let third = *name.as_bytes().get(2)?;
    if !third.is_ascii_digit() {
        return None;
    }
    Some(&name[3..])
}

fn c_strtol_prefix(input: &str) -> (i64, &str) {
    let bytes = input.as_bytes();
    let mut index = 0;
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }

    let mut negative = false;
    if let Some(sign) = bytes.get(index) {
        if *sign == b'-' || *sign == b'+' {
            negative = *sign == b'-';
            index += 1;
        }
    }

    let digit_start = index;
    let mut value = 0_u64;
    let limit = if negative {
        I64_MIN_ABS_U64
    } else {
        I64_MAX_U64
    };
    while let Some(digit) = bytes.get(index).filter(|byte| byte.is_ascii_digit()) {
        value = value
            .saturating_mul(10)
            .saturating_add(u64::from(*digit - b'0'))
            .min(limit);
        index += 1;
    }

    if index == digit_start {
        return (0, input);
    }

    let value = if negative {
        if value == I64_MIN_ABS_U64 {
            i64::MIN
        } else {
            -i64::try_from(value).expect("negative parsed value is in i64 range")
        }
    } else {
        i64::try_from(value).expect("positive parsed value is in i64 range")
    };
    (value, &input[index..])
}

const I64_MAX_U64: u64 = 9_223_372_036_854_775_807;
const I64_MIN_ABS_U64: u64 = 9_223_372_036_854_775_808;

#[cfg(test)]
mod tests {
    use super::{source_info_pcl_string, source_info_string, source_info_tstp_string, ClauseInfo};

    #[test]
    fn allocation_copies_optional_name_source_and_positions() {
        let mut name = String::from("input_name");
        let mut source = String::from("problem.p");
        let info = ClauseInfo::new(Some(&name), Some(&source), 12, 4);

        name.clear();
        source.clear();

        assert_eq!(info.name(), Some("input_name"));
        assert_eq!(info.source(), Some("problem.p"));
        assert_eq!(info.line(), 12);
        assert_eq!(info.column(), 4);
        assert_eq!(ClauseInfo::empty(), ClauseInfo::new(None, None, -1, -1));
    }

    #[test]
    fn source_info_formatting_matches_tstp_pcl_and_null_shapes() {
        let named = ClauseInfo::new(Some("ax1"), Some("file.p"), 2, 3);
        assert_eq!(named.source_info_tstp_string(), "file('file.p', ax1)");
        assert_eq!(named.source_info_pcl_string(), "initial(\"file.p\", ax1)");
        assert_eq!(
            named.source_info_string("custom", "|"),
            "custom(|file.p|, ax1)"
        );

        let located = ClauseInfo::new(None, Some("file.p"), 2, 3);
        assert_eq!(
            located.source_info_tstp_string(),
            "file('file.p', at_line_2_column_3)"
        );

        let unknown = ClauseInfo::empty();
        assert_eq!(unknown.source_info_tstp_string(), "file(unknown, unknown)");
        assert_eq!(source_info_tstp_string(None), "");
        assert_eq!(source_info_pcl_string(None), "");
        assert_eq!(source_info_string(None, "ignored", "'"), "");
    }

    #[test]
    #[should_panic(expected = "clause source column must be non-negative")]
    fn source_info_asserts_when_line_has_no_column() {
        let info = ClauseInfo::new(None, Some("file.p"), 2, -1);

        let _ = info.source_info_tstp_string();
    }

    #[test]
    fn generated_id_parsing_preserves_c_offset_quirk() {
        assert_eq!(
            ClauseInfo::new(Some("c_1_2"), None, -1, -1).id_namespace(),
            0
        );
        assert_eq!(ClauseInfo::new(Some("c_1_2"), None, -1, -1).id_counter(), 2);
        assert_eq!(
            ClauseInfo::new(Some("c_12_34"), None, -1, -1).id_namespace(),
            2
        );
        assert_eq!(
            ClauseInfo::new(Some("i_987_654"), None, -1, -1).id_namespace(),
            87
        );
        assert_eq!(
            ClauseInfo::new(Some("i_987_654"), None, -1, -1).id_counter(),
            654
        );
    }

    #[test]
    fn generated_id_parsing_uses_c_strtol_tail_acceptance() {
        assert_eq!(ClauseInfo::new(None, None, -1, -1).id_namespace(), -1);
        assert_eq!(
            ClauseInfo::new(Some("x_12_34"), None, -1, -1).id_namespace(),
            -1
        );
        assert_eq!(
            ClauseInfo::new(Some("c_x_34"), None, -1, -1).id_counter(),
            -1
        );
        assert_eq!(
            ClauseInfo::new(Some("c_12x_34"), None, -1, -1).id_counter(),
            -1
        );
        assert_eq!(ClauseInfo::new(Some("c_1_"), None, -1, -1).id_counter(), 0);
        assert_eq!(
            ClauseInfo::new(Some("c_1_-2"), None, -1, -1).id_counter(),
            -2
        );
        assert_eq!(
            ClauseInfo::new(Some("c_1_999999999999999999999999"), None, -1, -1).id_counter(),
            i64::MAX
        );
    }
}
