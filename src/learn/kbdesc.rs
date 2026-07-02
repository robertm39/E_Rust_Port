use crate::basics::defines::DEFAULT_COMCHAR_RAW;
use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::basicparser::parse_float;
use crate::inout::scanner::{token_pos_rep, Scanner, TokenType};
use crate::prover::version::E_URL;
use std::fmt::Write as _;

pub const KB_VERSION: &str = "0.20dev";
pub const KB_ANNOTATION_NO: i64 = 7;

#[derive(Clone, Debug, PartialEq)]
pub struct KbDesc {
    version: String,
    neg_proportion: f64,
    fail_neg_examples: i64,
}

impl KbDesc {
    #[must_use]
    pub fn new(version: impl Into<String>, neg_proportion: f64, fail_neg_examples: i64) -> Self {
        Self {
            version: version.into(),
            neg_proportion,
            fail_neg_examples,
        }
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub const fn neg_proportion(&self) -> f64 {
        self.neg_proportion
    }

    #[must_use]
    pub const fn fail_neg_examples(&self) -> i64 {
        self.fail_neg_examples
    }

    #[must_use]
    pub fn print_string(&self) -> String {
        let mut result = String::new();
        let write_result = writeln!(
            &mut result,
            "{DEFAULT_COMCHAR_RAW} E theorem prover knowledge base description\n\
Version     : \"{}\"\n\
NegProp     : {:8.6}  {DEFAULT_COMCHAR_RAW} Negative example proportion (successful proof search)\n\
FailExamples: {:8}  {DEFAULT_COMCHAR_RAW} Number of clauses from a failed proof search",
            self.version, self.neg_proportion, self.fail_neg_examples
        );
        debug_assert!(write_result.is_ok());
        result
    }

    pub fn parse(scanner: &mut Scanner) -> Result<Self, Diagnostic> {
        scanner.accept_id("Version")?;
        scanner.accept_tok(TokenType::COLON)?;
        scanner.check_tok(TokenType::STRING)?;
        let version = strip_double_quote_core(scanner.current_token().literal_bytes())?;
        if version.as_str() > KB_VERSION {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                format!("Knowledge base is younger than your tool set. Please update from{E_URL}"),
            ));
        }
        scanner.next_token()?;

        scanner.accept_id("NegProp")?;
        scanner.accept_tok(TokenType::COLON)?;
        let neg_proportion = parse_float(scanner)?;
        scanner.accept_id("FailExamples")?;
        scanner.accept_tok(TokenType::COLON)?;
        let fail_neg_examples = i64::try_from(scanner.current_token().numval())
            .map_err(|_| current_error(scanner, "Long integer overflow"))?;
        scanner.accept_tok(TokenType::POS_INT)?;

        Ok(Self::new(version, neg_proportion, fail_neg_examples))
    }
}

#[must_use]
pub fn kb_desc_alloc(version: &str, neg_proportion: f64, fail_neg_examples: i64) -> KbDesc {
    KbDesc::new(version, neg_proportion, fail_neg_examples)
}

#[must_use]
pub fn kb_desc_print_string(desc: &KbDesc) -> String {
    desc.print_string()
}

pub fn kb_desc_parse(scanner: &mut Scanner) -> Result<KbDesc, Diagnostic> {
    KbDesc::parse(scanner)
}

#[must_use]
pub fn kb_file_name(name: &mut DynamicString, basename: &str, file: &str) -> String {
    name.reset();
    name.append_str(basename);
    name.append_str("/");
    name.append_str(file);
    name.view().into_owned()
}

fn strip_double_quote_core(bytes: &[u8]) -> Result<String, Diagnostic> {
    if bytes.len() < 2 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Quoted string literal is too short",
        ));
    }
    Ok(String::from_utf8_lossy(&bytes[1..bytes.len() - 1]).into_owned())
}

fn current_error(scanner: &Scanner, message: &str) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::SYNTAX_ERROR,
        format!("{} {message}", token_pos_rep(scanner.current_token())),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        kb_desc_alloc, kb_desc_parse, kb_desc_print_string, kb_file_name, KB_ANNOTATION_NO,
        KB_VERSION,
    };
    use crate::basics::dstrings::DynamicString;
    use crate::basics::error::ErrorCode;
    use crate::inout::scanner::Scanner;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn make_scanner(source: &str) -> Scanner {
        Scanner::from_user_string(source, false).expect("scanner allocation")
    }

    #[test]
    fn constants_match_c_header() {
        assert_eq!(KB_VERSION, "0.20dev");
        assert_eq!(KB_ANNOTATION_NO, 7);
    }

    #[test]
    fn allocation_and_printing_match_c_shape() {
        let desc = kb_desc_alloc(KB_VERSION, 0.25, 42);

        assert_eq!(desc.version(), KB_VERSION);
        assert_close(desc.neg_proportion(), 0.25);
        assert_eq!(desc.fail_neg_examples(), 42);
        assert_eq!(
            kb_desc_print_string(&desc),
            "% E theorem prover knowledge base description\n\
Version     : \"0.20dev\"\n\
NegProp     : 0.250000  % Negative example proportion (successful proof search)\n\
FailExamples:       42  % Number of clauses from a failed proof search\n"
        );
    }

    #[test]
    fn parse_reads_version_neg_prop_and_failed_examples() {
        let mut scanner =
            make_scanner("Version: \"0.20dev\" NegProp: 0.125 FailExamples: 17 trailing");

        let desc = kb_desc_parse(&mut scanner).expect("KB description parse");

        assert_eq!(desc.version(), KB_VERSION);
        assert_close(desc.neg_proportion(), 0.125);
        assert_eq!(desc.fail_neg_examples(), 17);
        assert_eq!(scanner.current_token().literal(), "trailing");
    }

    #[test]
    fn parse_rejects_lexicographically_younger_versions() {
        let mut scanner = make_scanner("Version: \"1.0\" NegProp: 0.1 FailExamples: 1");

        let error = kb_desc_parse(&mut scanner).unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error
            .message()
            .contains("Knowledge base is younger than your tool set"));
    }

    #[test]
    fn kb_file_name_resets_buffer_and_uses_forward_slash() {
        let mut name = DynamicString::new();
        name.append_str("old");

        let result = kb_file_name(&mut name, "kb/base", "signature");

        assert_eq!(result, "kb/base/signature");
        assert_eq!(name.view(), "kb/base/signature");
    }
}
