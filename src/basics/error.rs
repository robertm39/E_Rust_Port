use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErrorCode(u8);

impl ErrorCode {
    pub const NO_ERROR: Self = Self(0);
    pub const PROOF_FOUND: Self = Self(0);
    pub const SATISFIABLE: Self = Self(1);
    pub const OUT_OF_MEMORY: Self = Self(2);
    pub const SYNTAX_ERROR: Self = Self(3);
    pub const TYPE_ERROR: Self = Self(4);
    pub const USAGE_ERROR: Self = Self(5);
    pub const FILE_ERROR: Self = Self(6);
    pub const SYSTEM_ERROR: Self = Self(7);
    pub const CPU_LIMIT_ERROR: Self = Self(8);
    pub const RESOURCE_OUT: Self = Self(9);
    pub const INCOMPLETE_PROOFSTATE: Self = Self(10);
    pub const OTHER_ERROR: Self = Self(11);
    pub const INPUT_SEMANTIC_ERROR: Self = Self(12);
    pub const INTERFACE_ERROR: Self = Self(13);
    pub const PARENT_REQUEST: Self = Self(14);

    #[must_use]
    pub const fn exit_status(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    code: ErrorCode,
    message: String,
}

impl Diagnostic {
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn render_error(&self, program_name: &str) -> String {
        format!("{program_name}: {}\n", self.message)
    }

    #[must_use]
    pub fn render_warning(&self, program_name: &str) -> String {
        format!("{program_name}: Warning: {}\n", self.message)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Diagnostic {}

#[must_use]
pub fn test_letter_string(to_check: &str, options: &str) -> bool {
    to_check
        .bytes()
        .all(|candidate| options.bytes().any(|control| control == candidate))
}

pub fn check_option_letter_string(
    to_check: &str,
    options: &str,
    option: &str,
) -> Result<(), Diagnostic> {
    if test_letter_string(to_check, options) {
        Ok(())
    } else {
        Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("Illegal argument to option {option}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{check_option_letter_string, test_letter_string, ErrorCode};

    #[test]
    fn letter_string_accepts_only_known_letters() {
        assert!(test_letter_string("abc", "cadb"));
        assert!(test_letter_string("", ""));
        assert!(!test_letter_string("abcx", "abc"));
    }

    #[test]
    fn check_letter_string_reports_usage_error() {
        let error = check_option_letter_string("az", "abc", "--letters").unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(error.message(), "Illegal argument to option --letters");
    }
}
