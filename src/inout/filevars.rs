use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::scanner::{Scanner, TokenType};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileVarEntry {
    value: String,
    source: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FileVars {
    source_names: Vec<String>,
    vars: BTreeMap<String, FileVarEntry>,
}

impl FileVars {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }

    #[must_use]
    pub fn source_count(&self) -> usize {
        self.source_names.len()
    }

    pub fn parse_scanner(
        &mut self,
        scanner: &mut Scanner,
        source_name: impl Into<String>,
    ) -> Result<usize, Diagnostic> {
        let source_name = source_name.into();
        self.source_names.push(source_name.clone());
        let mut count = 0_usize;

        while !scanner.test_tok(TokenType::NO_TOKEN) {
            scanner.check_tok(TokenType::IDENTIFIER)?;
            let name = scanner.current_token().literal();
            scanner.next_token()?;
            scanner.accept_tok(TokenType::EQUAL_SIGN)?;

            let mut value = String::new();
            while !scanner.test_tok(TokenType::SEMICOLON) {
                if scanner.test_tok(TokenType::NO_TOKEN) {
                    return Err(Diagnostic::new(
                        ErrorCode::SYNTAX_ERROR,
                        format!("Semicolon expected while reading file variable {name}"),
                    ));
                }
                value.push_str(&scanner.current_token().literal());
                scanner.next_token()?;
            }
            scanner.accept_tok(TokenType::SEMICOLON)?;

            self.vars.insert(
                name,
                FileVarEntry {
                    value,
                    source: source_name.clone(),
                },
            );
            count += 1;
        }

        Ok(count)
    }

    pub fn read_from_file(&mut self, file: &Path) -> Result<usize, Diagnostic> {
        let mut scanner = Scanner::from_file(file, true)?;
        self.parse_scanner(&mut scanner, file.display().to_string())
    }

    #[must_use]
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.vars.get(name).map(|entry| entry.value != "true")
    }

    pub fn get_int(&self, name: &str) -> Result<Option<i64>, Diagnostic> {
        let Some(entry) = self.vars.get(name) else {
            return Ok(None);
        };
        entry
            .value
            .parse::<i64>()
            .map(Some)
            .map_err(|_| semantic_error("Integer", "Integer", name, &entry.source))
    }

    #[must_use]
    pub fn get_str(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(|entry| entry.value.as_str())
    }

    pub fn get_identifier(&self, name: &str) -> Result<Option<&str>, Diagnostic> {
        let Some(entry) = self.vars.get(name) else {
            return Ok(None);
        };
        let Ok(scanner) = Scanner::from_internal_string(&entry.value, true) else {
            return Err(semantic_error(
                "Identifier",
                "identifier",
                name,
                &entry.source,
            ));
        };
        if scanner.test_tok(TokenType::IDENTIFIER) {
            Ok(Some(entry.value.as_str()))
        } else {
            Err(semantic_error(
                "Identifier",
                "identifier",
                name,
                &entry.source,
            ))
        }
    }
}

fn semantic_error(requested: &str, present: &str, name: &str, source: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::INPUT_SEMANTIC_ERROR,
        format!(
            "{requested} value requested for file variable {name} read from \"{source}\", but no {present} value present"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::FileVars;
    use crate::basics::error::ErrorCode;
    use crate::inout::scanner::Scanner;
    use std::path::{Path, PathBuf};

    fn make_scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, true).unwrap()
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("filevars-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    #[test]
    fn parse_stores_values_without_whitespace_and_overwrites_old_values() {
        let mut scanner = make_scanner("alpha = true; beta = foo + 12; alpha = false; empty = ;");
        let mut vars = FileVars::new();

        assert_eq!(vars.parse_scanner(&mut scanner, "vars.cfg").unwrap(), 4);

        assert_eq!(vars.len(), 3);
        assert_eq!(vars.source_count(), 1);
        assert_eq!(vars.get_str("alpha"), Some("false"));
        assert_eq!(vars.get_str("beta"), Some("foo+12"));
        assert_eq!(vars.get_str("empty"), Some(""));
        assert_eq!(vars.get_str("missing"), None);
    }

    #[test]
    fn bool_getter_preserves_c_strcmp_bug() {
        let mut scanner = make_scanner("t = true; f = false; other = maybe;");
        let mut vars = FileVars::new();
        vars.parse_scanner(&mut scanner, "vars.cfg").unwrap();

        assert_eq!(vars.get_bool("t"), Some(false));
        assert_eq!(vars.get_bool("f"), Some(true));
        assert_eq!(vars.get_bool("other"), Some(true));
        assert_eq!(vars.get_bool("missing"), None);
    }

    #[test]
    fn int_getter_returns_none_or_semantic_error() {
        let mut scanner = make_scanner("neg = -42; plus = +7; bad = 12x;");
        let mut vars = FileVars::new();
        vars.parse_scanner(&mut scanner, "vars.cfg").unwrap();

        assert_eq!(vars.get_int("neg").unwrap(), Some(-42));
        assert_eq!(vars.get_int("plus").unwrap(), Some(7));
        assert_eq!(vars.get_int("missing").unwrap(), None);

        let error = vars.get_int("bad").unwrap_err();
        assert_eq!(error.code(), ErrorCode::INPUT_SEMANTIC_ERROR);
        assert!(error.message().contains("Integer value requested"));
        assert!(error.message().contains("vars.cfg"));
    }

    #[test]
    fn identifier_getter_checks_only_the_first_token_like_c() {
        let mut scanner = make_scanner("good = abc123 + tail; bad = + abc; idnum = x123;");
        let mut vars = FileVars::new();
        vars.parse_scanner(&mut scanner, "vars.cfg").unwrap();

        assert_eq!(vars.get_identifier("good").unwrap(), Some("abc123+tail"));
        assert_eq!(vars.get_identifier("idnum").unwrap(), Some("x123"));
        assert_eq!(vars.get_identifier("missing").unwrap(), None);

        let error = vars.get_identifier("bad").unwrap_err();
        assert_eq!(error.code(), ErrorCode::INPUT_SEMANTIC_ERROR);
        assert!(error.message().contains("Identifier value requested"));
    }

    #[test]
    fn read_from_file_uses_path_as_semantic_error_source() {
        let path = temp_path("read");
        remove_if_present(&path);
        std::fs::write(&path, b"answer = 42; bad = 1z;").unwrap();
        let mut vars = FileVars::new();

        assert_eq!(vars.read_from_file(&path).unwrap(), 2);
        assert_eq!(vars.get_int("answer").unwrap(), Some(42));

        let error = vars.get_int("bad").unwrap_err();
        assert!(error.message().contains(&path.display().to_string()));
        remove_if_present(&path);
    }

    #[test]
    fn read_from_file_uses_file_positions_for_syntax_errors() {
        let path = temp_path("syntax");
        remove_if_present(&path);
        std::fs::write(&path, b"= 1;").unwrap();
        let mut vars = FileVars::new();

        let error = vars.read_from_file(&path).unwrap_err();

        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains(&path.display().to_string()));
        assert!(error.message().contains("Column 1"));

        remove_if_present(&path);
    }

    #[test]
    fn parse_rejects_missing_semicolon_instead_of_looping_at_eof() {
        let mut scanner = make_scanner("answer = 42");
        let mut vars = FileVars::new();

        let error = vars.parse_scanner(&mut scanner, "vars.cfg").unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Semicolon expected"));
    }
}
