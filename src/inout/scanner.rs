use std::ops::BitOr;

use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::inout::streams::{InputStream, StreamType};
use std::path::Path;

pub const MAX_TOKEN_LOOKAHEAD: usize = 4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum IoFormat {
    #[default]
    Lop = 0,
    Tptp = 1,
    Tstp = 2,
    Auto = 3,
}

impl IoFormat {
    #[must_use]
    pub const fn c_value(self) -> i32 {
        self as i32
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TokenType(u64);

impl TokenType {
    pub const NO_TOKEN: Self = Self(1 << 0);
    pub const WHITE_SPACE: Self = Self(1 << 1);
    pub const COMMENT: Self = Self(1 << 2);
    pub const IDENT: Self = Self(1 << 3);
    pub const IDNUM: Self = Self(1 << 4);
    pub const SEM_IDENT: Self = Self(1 << 5);
    pub const STRING: Self = Self(1 << 6);
    pub const SQ_STRING: Self = Self(1 << 7);
    pub const POS_INT: Self = Self(1 << 8);
    pub const OPEN_BRACKET: Self = Self(1 << 9);
    pub const CLOSE_BRACKET: Self = Self(1 << 10);
    pub const OPEN_CURLY: Self = Self(1 << 11);
    pub const CLOSE_CURLY: Self = Self(1 << 12);
    pub const OPEN_SQUARE: Self = Self(1 << 13);
    pub const CLOSE_SQUARE: Self = Self(1 << 14);
    pub const LESSER_SIGN: Self = Self(1 << 15);
    pub const GREATER_SIGN: Self = Self(1 << 16);
    pub const EQUAL_SIGN: Self = Self(1 << 17);
    pub const NEG_EQUAL_SIGN: Self = Self(1 << 18);
    pub const TILDE_SIGN: Self = Self(1 << 19);
    pub const EXCLAMATION: Self = Self(1 << 20);
    pub const UNIV_QUANTOR: Self = Self::EXCLAMATION;
    pub const QUESTION_MARK: Self = Self(1 << 21);
    pub const EXIST_QUANTOR: Self = Self::QUESTION_MARK;
    pub const COMMA: Self = Self(1 << 22);
    pub const SEMICOLON: Self = Self(1 << 23);
    pub const COLON: Self = Self(1 << 24);
    pub const HYPHEN: Self = Self(1 << 25);
    pub const PLUS: Self = Self(1 << 26);
    pub const MULT: Self = Self(1 << 27);
    pub const FULLSTOP: Self = Self(1 << 28);
    pub const DOLLAR: Self = Self(1 << 29);
    pub const SLASH: Self = Self(1 << 30);
    pub const PIPE: Self = Self(1 << 31);
    pub const FOF_OR: Self = Self::PIPE;
    pub const AMPERSAND: Self = Self(1 << 32);
    pub const FOF_AND: Self = Self::AMPERSAND;
    pub const FOF_LR_IMPL: Self = Self(1 << 33);
    pub const FOF_RL_IMPL: Self = Self(1 << 34);
    pub const FOF_EQUIV: Self = Self(1 << 35);
    pub const FOF_XOR: Self = Self(1 << 36);
    pub const FOF_NAND: Self = Self(1 << 37);
    pub const FOF_NOR: Self = Self(1 << 38);
    pub const APPLICATION: Self = Self(1 << 39);
    pub const CARRET: Self = Self(1 << 40);
    pub const LAMBDA_QUANTOR: Self = Self::CARRET;
    pub const LET_TOKEN: Self = Self(1 << 41);
    pub const ITE_TOKEN: Self = Self(1 << 42);

    pub const SKIP_TOKEN: Self = Self(Self::WHITE_SPACE.0 | Self::COMMENT.0);
    pub const IDENTIFIER: Self = Self(Self::IDENT.0 | Self::IDNUM.0);
    pub const NAME: Self = Self(Self::IDENTIFIER.0 | Self::STRING.0);
    pub const FOF_BIN_OP: Self = Self(
        Self::FOF_AND.0
            | Self::FOF_OR.0
            | Self::FOF_LR_IMPL.0
            | Self::FOF_RL_IMPL.0
            | Self::FOF_EQUIV.0
            | Self::FOF_XOR.0
            | Self::FOF_NAND.0
            | Self::FOF_NOR.0
            | Self::EQUAL_SIGN.0
            | Self::NEG_EQUAL_SIGN.0,
    );
    pub const FOF_ASSOC_OP: Self = Self(Self::FOF_AND.0 | Self::FOF_OR.0);

    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

impl BitOr for TokenType {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    kind: TokenType,
    literal: DynamicString,
    numval: u64,
    comment: DynamicString,
    skipped: bool,
    source: Vec<u8>,
    stream_type: StreamType,
    line: usize,
    column: usize,
}

impl Default for Token {
    fn default() -> Self {
        Self {
            kind: TokenType::NO_TOKEN,
            literal: DynamicString::new(),
            numval: 0,
            comment: DynamicString::new(),
            skipped: false,
            source: Vec::new(),
            stream_type: StreamType::UserString,
            line: 1,
            column: 1,
        }
    }
}

impl Token {
    #[must_use]
    pub const fn kind(&self) -> TokenType {
        self.kind
    }

    #[must_use]
    pub fn literal_bytes(&self) -> &[u8] {
        self.literal.view_bytes()
    }

    #[must_use]
    pub fn literal(&self) -> String {
        self.literal.view().into_owned()
    }

    #[must_use]
    pub const fn numval(&self) -> u64 {
        self.numval
    }

    #[must_use]
    pub fn comment_bytes(&self) -> &[u8] {
        self.comment.view_bytes()
    }

    #[must_use]
    pub const fn skipped(&self) -> bool {
        self.skipped
    }

    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    #[must_use]
    pub const fn column(&self) -> usize {
        self.column
    }

    #[must_use]
    pub const fn stream_type(&self) -> StreamType {
        self.stream_type
    }

    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scanner {
    source: InputStream,
    ignore_comments: bool,
    format: IoFormat,
    tok_sequence: [Token; MAX_TOKEN_LOOKAHEAD],
    current: usize,
}

impl Scanner {
    pub fn from_user_string(source: &str, ignore_comments: bool) -> Result<Self, Diagnostic> {
        Self::from_stream(InputStream::from_user_string(source), ignore_comments)
    }

    pub fn from_option_string(source: &str, ignore_comments: bool) -> Result<Self, Diagnostic> {
        Self::from_stream(InputStream::from_option_string(source), ignore_comments)
    }

    pub fn from_internal_string(source: &str, ignore_comments: bool) -> Result<Self, Diagnostic> {
        Self::from_stream(InputStream::from_internal_string(source), ignore_comments)
    }

    pub fn from_file(path: &Path, ignore_comments: bool) -> Result<Self, Diagnostic> {
        Self::from_stream(InputStream::from_file(path)?, ignore_comments)
    }

    pub fn from_stream(source: InputStream, ignore_comments: bool) -> Result<Self, Diagnostic> {
        let mut scanner = Self {
            source,
            ignore_comments,
            format: IoFormat::Lop,
            tok_sequence: std::array::from_fn(|_| Token::default()),
            current: 0,
        };
        for index in 0..MAX_TOKEN_LOOKAHEAD {
            scanner.scan_real_token(index)?;
        }
        Ok(scanner)
    }

    #[must_use]
    pub fn current_token(&self) -> &Token {
        &self.tok_sequence[self.current]
    }

    #[must_use]
    pub const fn format(&self) -> IoFormat {
        self.format
    }

    pub fn set_format(&mut self, format: IoFormat) {
        self.format = if format == IoFormat::Auto {
            if self.test_id("fof|cnf|tff|thf|tcf|include") {
                IoFormat::Tstp
            } else if self.test_id("input_clause|input_formula") {
                IoFormat::Tptp
            } else {
                IoFormat::Lop
            }
        } else {
            format
        };
    }

    #[must_use]
    /// Returns the token `look` positions after the current scanner token.
    ///
    /// # Panics
    ///
    /// Panics when `look >= MAX_TOKEN_LOOKAHEAD`, matching the assertion in
    /// the C `LookToken` macro.
    pub fn look_token(&self, look: usize) -> &Token {
        assert!(look < MAX_TOKEN_LOOKAHEAD);
        &self.tok_sequence[token_real_pos(self.current + look)]
    }

    pub fn next_token(&mut self) -> Result<(), Diagnostic> {
        self.scan_real_token(self.current)?;
        self.current = token_real_pos(self.current + 1);
        Ok(())
    }

    #[must_use]
    pub fn test_tok(&self, toks: TokenType) -> bool {
        test_tok(self.current_token(), toks)
    }

    #[must_use]
    pub fn test_id(&self, ids: &str) -> bool {
        test_id(self.current_token(), ids)
    }

    #[must_use]
    pub fn test_idnum(&self, ids: &str) -> bool {
        test_idnum(self.current_token(), ids)
    }

    #[must_use]
    pub fn test_no_skip(&self) -> bool {
        !self.current_token().skipped
    }

    #[must_use]
    pub fn test_tok_no_skip(&self, toks: TokenType) -> bool {
        self.test_no_skip() && self.test_tok(toks)
    }

    pub fn check_tok(&self, toks: TokenType) -> Result<(), Diagnostic> {
        if self.test_tok(toks) {
            Ok(())
        } else {
            Err(self.current_token_error(&format!(
                "{} expected, but {} read ",
                describe_token(toks),
                describe_token(self.current_token().kind)
            )))
        }
    }

    pub fn check_tok_no_skip(&self, toks: TokenType) -> Result<(), Diagnostic> {
        if self.current_token().skipped {
            Err(self.current_token_error(&format!(
                "{} expected, but {} read ",
                describe_token(toks),
                describe_token(TokenType::SKIP_TOKEN)
            )))
        } else {
            self.check_tok(toks)
        }
    }

    pub fn check_id(&self, ids: &str) -> Result<(), Diagnostic> {
        if self.test_id(ids) {
            Ok(())
        } else {
            Err(self.current_token_error(&format!(
                "Identifier ({ids}) expected, but {}('{}') read ",
                describe_token(self.current_token().kind),
                self.current_token().literal()
            )))
        }
    }

    pub fn accept_tok(&mut self, toks: TokenType) -> Result<(), Diagnostic> {
        self.check_tok(toks)?;
        self.next_token()
    }

    pub fn accept_tok_no_skip(&mut self, toks: TokenType) -> Result<(), Diagnostic> {
        self.check_tok_no_skip(toks)?;
        self.next_token()
    }

    pub fn accept_id(&mut self, ids: &str) -> Result<(), Diagnostic> {
        self.check_id(ids)?;
        self.next_token()
    }

    fn scan_real_token(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.tok_sequence[index].skipped = false;
        self.tok_sequence[index].comment.reset();
        self.scan_token(index)?;

        while test_tok(&self.tok_sequence[index], TokenType::SKIP_TOKEN) {
            self.tok_sequence[index].skipped = true;
            if !self.ignore_comments && test_tok(&self.tok_sequence[index], TokenType::COMMENT) {
                let comment = self.tok_sequence[index].literal.copy();
                self.tok_sequence[index].comment.append_buffer(&comment);
            }
            self.scan_token(index)?;
        }
        Ok(())
    }

    fn scan_token(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.reset_scanned_token(index);
        match self.source.current_char() {
            None => self.tok_sequence[index].kind = TokenType::NO_TOKEN,
            Some(byte) if byte.is_ascii_whitespace() => self.scan_white(index),
            Some(byte) if is_start_id_char(byte) => self.scan_ident(index),
            Some(byte) if byte.is_ascii_digit() => self.scan_int(index),
            Some(b'#' | b'%') => self.scan_line_comment(index),
            Some(b'/') if self.source.look_char(1) == Some(b'*') => {
                self.scan_c_comment(index)?;
            }
            Some(delimiter @ (b'"' | b'\'')) => self.scan_string(index, delimiter)?,
            Some(b'$') if self.source.look_char(1).is_some_and(is_id_char) => {
                self.scan_semantic_identifier(index);
            }
            Some(_) => self.scan_punctuation(index)?,
        }
        Ok(())
    }

    fn reset_scanned_token(&mut self, index: usize) {
        let token = &mut self.tok_sequence[index];
        token.literal.reset();
        token.source = self.source.source_bytes().to_vec();
        token.stream_type = self.source.stream_type();
        token.line = self.source.line();
        token.column = self.source.column();
        token.numval = 0;
        token.kind = TokenType::NO_TOKEN;
    }

    fn scan_white(&mut self, index: usize) {
        self.tok_sequence[index].kind = TokenType::WHITE_SPACE;
        while let Some(byte) = self.source.current_char() {
            if !byte.is_ascii_whitespace() {
                break;
            }
            self.append_current_and_advance(index);
        }
    }

    fn scan_ident(&mut self, index: usize) {
        let mut numstart = 0_usize;
        for offset in 0_usize.. {
            let Some(byte) = self.source.current_char() else {
                break;
            };
            if !is_id_char(byte) {
                break;
            }
            if numstart == 0 && byte.is_ascii_digit() {
                numstart = offset;
            } else if !byte.is_ascii_digit() {
                numstart = 0;
            }
            self.append_current_and_advance(index);
        }

        if numstart != 0 {
            self.tok_sequence[index].kind = TokenType::IDNUM;
            self.tok_sequence[index].numval =
                parse_u64_saturating(&self.tok_sequence[index].literal.view_bytes()[numstart..]);
        } else {
            self.tok_sequence[index].kind = TokenType::IDENT;
            self.tok_sequence[index].numval = 0;
        }
    }

    fn scan_int(&mut self, index: usize) {
        self.tok_sequence[index].kind = TokenType::POS_INT;
        while let Some(byte) = self.source.current_char() {
            if !byte.is_ascii_digit() {
                break;
            }
            self.append_current_and_advance(index);
        }
        self.tok_sequence[index].numval =
            parse_u64_saturating(self.tok_sequence[index].literal.view_bytes());
    }

    fn scan_line_comment(&mut self, index: usize) {
        self.tok_sequence[index].kind = TokenType::COMMENT;
        while let Some(byte) = self.source.current_char() {
            if byte == b'\n' {
                break;
            }
            self.append_current_and_advance(index);
        }
        self.tok_sequence[index].literal.append_byte(b'\n');
        self.source.next_char();
    }

    fn scan_c_comment(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.tok_sequence[index].kind = TokenType::COMMENT;
        while !(self.source.current_char() == Some(b'*') && self.source.look_char(1) == Some(b'/'))
        {
            if self.source.current_char().is_none() {
                return Err(self.token_error(index, "Unterminated C-style comment"));
            }
            self.append_current_and_advance(index);
        }
        self.append_current_and_advance(index);
        self.append_current_and_advance(index);
        Ok(())
    }

    fn scan_string(&mut self, index: usize, delimiter: u8) -> Result<(), Diagnostic> {
        let token_kind = if delimiter == b'\'' {
            TokenType::SQ_STRING
        } else {
            TokenType::STRING
        };
        self.tok_sequence[index].kind = token_kind;
        self.append_current_and_advance(index);

        let mut escaped = false;
        loop {
            let Some(byte) = self.source.current_char() else {
                return Err(self.token_error(index, "Unterminated string constant"));
            };
            if !escaped && byte == delimiter {
                break;
            }
            if byte <= 127 && !byte.is_ascii_graphic() && byte != b' ' {
                return Err(self.token_error(index, "Non-printable character in string constant"));
            }
            if byte == b'\\' {
                escaped = !escaped;
            } else {
                escaped = false;
            }
            self.append_current_and_advance(index);
        }
        self.append_current_and_advance(index);
        Ok(())
    }

    fn scan_semantic_identifier(&mut self, index: usize) {
        self.append_current_and_advance(index);
        self.scan_ident(index);
        self.tok_sequence[index].kind = match self.tok_sequence[index].literal.view_bytes() {
            b"$let" => TokenType::LET_TOKEN,
            b"$ite" => TokenType::ITE_TOKEN,
            _ => TokenType::SEM_IDENT,
        };
    }

    fn scan_punctuation(&mut self, index: usize) -> Result<(), Diagnostic> {
        let kind = match self.source.current_char() {
            Some(b'(') => TokenType::OPEN_BRACKET,
            Some(b')') => TokenType::CLOSE_BRACKET,
            Some(b'{') => TokenType::OPEN_CURLY,
            Some(b'}') => TokenType::CLOSE_CURLY,
            Some(b'[') => TokenType::OPEN_SQUARE,
            Some(b']') => TokenType::CLOSE_SQUARE,
            Some(b'<') => self.scan_lesser_prefixed_operator(index),
            Some(b'>') => TokenType::GREATER_SIGN,
            Some(b'=') => self.scan_equal_prefixed_operator(index),
            Some(b'~') => self.scan_tilde_prefixed_operator(index),
            Some(b'!') => self.scan_exclamation_prefixed_operator(index),
            Some(b'?') => TokenType::QUESTION_MARK,
            Some(b',') => TokenType::COMMA,
            Some(b';') => TokenType::SEMICOLON,
            Some(b':') => TokenType::COLON,
            Some(b'-') => TokenType::HYPHEN,
            Some(b'+') => TokenType::PLUS,
            Some(b'*') => TokenType::MULT,
            Some(b'.') => TokenType::FULLSTOP,
            Some(b'|') => TokenType::PIPE,
            Some(b'/') => TokenType::SLASH,
            Some(b'&') => TokenType::AMPERSAND,
            Some(b'$') => TokenType::DOLLAR,
            Some(b'@') => TokenType::APPLICATION,
            Some(b'^') => TokenType::CARRET,
            Some(_) | None => {
                self.append_current_and_advance(index);
                return Err(self.token_error(index, "Illegal character"));
            }
        };
        self.tok_sequence[index].kind = kind;
        self.append_current_and_advance(index);
        Ok(())
    }

    fn scan_lesser_prefixed_operator(&mut self, index: usize) -> TokenType {
        if self.source.look_char(1) == Some(b'~') && self.source.look_char(2) == Some(b'>') {
            self.append_current_and_advance(index);
            self.append_current_and_advance(index);
            TokenType::FOF_XOR
        } else if self.source.look_char(1) == Some(b'=') {
            self.append_current_and_advance(index);
            if self.source.look_char(1) == Some(b'>') {
                self.append_current_and_advance(index);
                TokenType::FOF_EQUIV
            } else {
                TokenType::FOF_RL_IMPL
            }
        } else {
            TokenType::LESSER_SIGN
        }
    }

    fn scan_equal_prefixed_operator(&mut self, index: usize) -> TokenType {
        if self.source.look_char(1) == Some(b'>') {
            self.append_current_and_advance(index);
            TokenType::FOF_LR_IMPL
        } else {
            TokenType::EQUAL_SIGN
        }
    }

    fn scan_tilde_prefixed_operator(&mut self, index: usize) -> TokenType {
        match self.source.look_char(1) {
            Some(b'|') => {
                self.append_current_and_advance(index);
                TokenType::FOF_NOR
            }
            Some(b'&') => {
                self.append_current_and_advance(index);
                TokenType::FOF_NAND
            }
            _ => TokenType::TILDE_SIGN,
        }
    }

    fn scan_exclamation_prefixed_operator(&mut self, index: usize) -> TokenType {
        if self.source.look_char(1) == Some(b'=') {
            self.append_current_and_advance(index);
            TokenType::NEG_EQUAL_SIGN
        } else {
            TokenType::EXCLAMATION
        }
    }

    fn append_current_and_advance(&mut self, index: usize) {
        if let Some(byte) = self.source.current_char() {
            self.tok_sequence[index].literal.append_byte(byte);
        }
        self.source.next_char();
    }

    fn token_error(&self, index: usize, message: &str) -> Diagnostic {
        let token = &self.tok_sequence[index];
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{}(just read '{}'): {message}",
                token_pos_rep(token),
                token.literal()
            ),
        )
    }

    fn current_token_error(&self, message: &str) -> Diagnostic {
        let token = self.current_token();
        Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            format!(
                "{}(just read '{}'): {message}",
                token_pos_rep(token),
                token.literal()
            ),
        )
    }
}

#[must_use]
pub const fn token_real_pos(pos: usize) -> usize {
    pos % MAX_TOKEN_LOOKAHEAD
}

#[must_use]
pub fn test_tok(token: &Token, toks: TokenType) -> bool {
    token.kind.intersects(toks)
}

#[must_use]
pub fn test_id(token: &Token, ids: &str) -> bool {
    if !test_tok(token, TokenType::IDENTIFIER | TokenType::SEM_IDENT) {
        return false;
    }
    str_n_element(token.literal_bytes(), ids, token.literal_bytes().len())
}

#[must_use]
pub fn test_idnum(token: &Token, ids: &str) -> bool {
    if !test_tok(token, TokenType::IDNUM) {
        return false;
    }
    str_n_element(
        token.literal_bytes(),
        ids,
        idnum_prefix_len(token.literal_bytes()),
    )
}

#[must_use]
pub fn describe_token(token_type: TokenType) -> String {
    let mut parts = Vec::new();
    for (kind, description) in TOKEN_PRINT_REP {
        if token_type.intersects(*kind) {
            parts.push(*description);
        }
    }
    if parts.is_empty() {
        "Unknown token (this should not happen)".to_owned()
    } else {
        parts.join(" or ")
    }
}

#[must_use]
pub fn pos_rep(stream_type: StreamType, source: &[u8], line: usize, column: usize) -> String {
    let source = String::from_utf8_lossy(source);
    match stream_type.description() {
        None => format!("{source}:{line}:(Column {column}):"),
        Some(description) => {
            let mut shown = source.chars().take(1020).collect::<String>();
            if source.chars().count() > 1020 {
                shown.push_str("...");
            }
            format!("{description}: \"{shown}\":{line}:(Column {column}):")
        }
    }
}

#[must_use]
pub fn token_pos_rep(token: &Token) -> String {
    pos_rep(token.stream_type, &token.source, token.line, token.column)
}

const TOKEN_PRINT_REP: &[(TokenType, &str)] = &[
    (TokenType::NO_TOKEN, "No token (probably EOF)"),
    (
        TokenType::WHITE_SPACE,
        "White space (spaces, tabs, newlines...)",
    ),
    (TokenType::COMMENT, "Comment"),
    (TokenType::IDENT, "Identifier not terminating in a number"),
    (TokenType::IDNUM, "Identifier terminating in a number"),
    (
        TokenType::SEM_IDENT,
        "Interpreted function/predicate name ('$name')",
    ),
    (TokenType::STRING, "String enclosed in double quotes (\"\")"),
    (TokenType::SQ_STRING, "String enclosed in single quote ('')"),
    (
        TokenType::POS_INT,
        "Integer (sequence of decimal digits) convertible to an 'unsigned long'",
    ),
    (TokenType::OPEN_BRACKET, "Opening bracket ('(')"),
    (TokenType::CLOSE_BRACKET, "Closing bracket (')')"),
    (TokenType::OPEN_CURLY, "Opening curly brace ('{')"),
    (TokenType::CLOSE_CURLY, "Closing curly brace ('}')"),
    (TokenType::OPEN_SQUARE, "Opening square brace ('[')"),
    (TokenType::CLOSE_SQUARE, "Closing square brace (']')"),
    (TokenType::LESSER_SIGN, "\"Lesser than\" sign ('<')"),
    (TokenType::GREATER_SIGN, "\"Greater than\" sign ('>')"),
    (TokenType::EQUAL_SIGN, "Equal Predicate/Sign ('=')"),
    (TokenType::NEG_EQUAL_SIGN, "Negated Equal Predicate ('!=')"),
    (TokenType::TILDE_SIGN, "Tilde ('~')"),
    (TokenType::EXCLAMATION, "Exclamation mark ('!')"),
    (TokenType::QUESTION_MARK, "Question mark ('?')"),
    (TokenType::COMMA, "Comma (',')"),
    (TokenType::SEMICOLON, "Semicolon (';')"),
    (TokenType::COLON, "Colon (':')"),
    (TokenType::HYPHEN, "Hyphen ('-')"),
    (TokenType::PLUS, "Plus sign ('+')"),
    (TokenType::MULT, "Multiplication sign ('*')"),
    (TokenType::FULLSTOP, "Fullstop ('.')"),
    (TokenType::DOLLAR, "Dollar sign ('$')"),
    (TokenType::SLASH, "Slash ('/')"),
    (TokenType::PIPE, "Vertical bar ('|')"),
    (TokenType::AMPERSAND, "Ampersand ('&')"),
    (TokenType::FOF_LR_IMPL, "Implication/LRArrow ('=>')"),
    (TokenType::FOF_RL_IMPL, "Back Implicatin/RLArrow ('<=')"),
    (TokenType::FOF_EQUIV, "Equivalence/Double arrow ('<=>')"),
    (TokenType::FOF_XOR, "Negated Equivalence/Xor ('<~>')"),
    (TokenType::FOF_NAND, "Nand ('~&')"),
    (TokenType::FOF_NOR, "Nor ('~|')"),
    (TokenType::APPLICATION, "Application ('@')"),
    (TokenType::LAMBDA_QUANTOR, "Lambda ('^')"),
    (TokenType::LET_TOKEN, "Let ('$let')"),
    (TokenType::ITE_TOKEN, "Ite ('$ite')"),
];

fn is_start_id_char(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_id_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn parse_u64_saturating(bytes: &[u8]) -> u64 {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|text| text.parse::<u64>().ok())
        .unwrap_or(u64::MAX)
}

fn idnum_prefix_len(bytes: &[u8]) -> usize {
    let mut len = 0_usize;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if len == 0 && byte.is_ascii_digit() {
            len = index;
        } else if !byte.is_ascii_digit() {
            len = 0;
        }
    }
    len
}

fn str_n_element(candidate: &[u8], ids: &str, len: usize) -> bool {
    ids.split('|')
        .any(|id| id.len() == len && candidate.get(..len) == Some(id.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{describe_token, test_id, test_idnum, token_pos_rep, IoFormat, Scanner, TokenType};
    use crate::basics::error::ErrorCode;
    use std::path::{Path, PathBuf};

    fn temp_path(name: &str) -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("scanner-{name}-{}.tmp", std::process::id()))
    }

    fn remove_if_present(path: &Path) {
        _ = std::fs::remove_file(path);
    }

    fn collect_tokens(source: &str) -> Vec<(TokenType, String, bool)> {
        let mut scanner = Scanner::from_user_string(source, false).unwrap();
        let mut tokens = Vec::new();
        loop {
            let token = scanner.current_token().clone();
            tokens.push((token.kind(), token.literal(), token.skipped()));
            if token.kind() == TokenType::NO_TOKEN {
                break;
            }
            scanner.next_token().unwrap();
        }
        tokens
    }

    #[test]
    fn token_type_values_match_c_bit_layout() {
        assert_eq!(TokenType::NO_TOKEN.bits(), 1);
        assert_eq!(TokenType::WHITE_SPACE.bits(), 2);
        assert_eq!(TokenType::IDENT.bits(), 8);
        assert_eq!(TokenType::POS_INT.bits(), 256);
        assert_eq!(TokenType::ITE_TOKEN.bits(), 1 << 42);
        assert!(TokenType::FOF_BIN_OP.intersects(TokenType::NEG_EQUAL_SIGN));
        assert!(TokenType::IDENTIFIER.intersects(TokenType::IDNUM));
    }

    #[test]
    fn scanner_format_defaults_and_auto_detection_match_c() {
        assert_eq!(IoFormat::Lop.c_value(), 0);
        assert_eq!(IoFormat::Tptp.c_value(), 1);
        assert_eq!(IoFormat::Tstp.c_value(), 2);
        assert_eq!(IoFormat::Auto.c_value(), 3);

        let mut tstp_scanner = Scanner::from_user_string("fof(name, axiom, p).", false).unwrap();
        assert_eq!(tstp_scanner.format(), IoFormat::Lop);
        tstp_scanner.set_format(IoFormat::Auto);
        assert_eq!(tstp_scanner.format(), IoFormat::Tstp);
        assert_eq!(tstp_scanner.current_token().literal(), "fof");

        let mut old_tptp_scanner =
            Scanner::from_user_string("input_clause(c,axiom,[]).", false).unwrap();
        old_tptp_scanner.set_format(IoFormat::Auto);
        assert_eq!(old_tptp_scanner.format(), IoFormat::Tptp);

        let mut lop = Scanner::from_user_string("cnf_like_but_not_exact", false).unwrap();
        lop.set_format(IoFormat::Auto);
        assert_eq!(lop.format(), IoFormat::Lop);
        lop.set_format(IoFormat::Tstp);
        assert_eq!(lop.format(), IoFormat::Tstp);
    }

    #[test]
    fn scanner_skips_space_and_comments_but_marks_next_real_token() {
        let scanner = Scanner::from_user_string("  # one\n/*two*/abc", false).unwrap();
        let token = scanner.current_token();
        assert_eq!(token.kind(), TokenType::IDENT);
        assert_eq!(token.literal(), "abc");
        assert!(token.skipped());
        assert_eq!(token.comment_bytes(), b"# one\n/*two*/");
        assert_eq!((token.line(), token.column()), (2, 8));
    }

    #[test]
    fn scanner_can_ignore_comment_accumulation() {
        let scanner = Scanner::from_user_string("# hidden\nabc", true).unwrap();
        let token = scanner.current_token();
        assert!(token.skipped());
        assert_eq!(token.comment_bytes(), b"");
    }

    #[test]
    fn scanner_reads_file_sources_with_filename_positions() {
        let scanner_path = temp_path("source");
        remove_if_present(&scanner_path);
        std::fs::write(&scanner_path, b"  abc").unwrap();

        let scanner = Scanner::from_file(&scanner_path, false).unwrap();

        assert_eq!(scanner.current_token().literal(), "abc");
        assert_eq!(
            scanner.current_token().stream_type(),
            super::StreamType::File
        );
        assert_eq!(
            token_pos_rep(scanner.current_token()),
            format!("{}:1:(Column 3):", scanner_path.to_string_lossy())
        );

        remove_if_present(&scanner_path);

        let missing = temp_path("missing");
        let error = Scanner::from_file(&missing, false).unwrap_err();
        assert_eq!(error.code(), ErrorCode::FILE_ERROR);
        assert!(error.message().contains("Cannot open file"));
    }

    #[test]
    fn scanner_tracks_line_and_column_for_real_tokens() {
        let scanner = Scanner::from_user_string("a\n  b", false).unwrap();
        assert_eq!(scanner.current_token().literal(), "a");
        assert_eq!(
            (
                scanner.current_token().line(),
                scanner.current_token().column()
            ),
            (1, 1)
        );
        let look = scanner.look_token(1);
        assert_eq!(look.literal(), "b");
        assert_eq!((look.line(), look.column()), (2, 3));
        assert!(look.skipped());
    }

    #[test]
    fn scanner_classifies_identifiers_idnums_semantic_names_and_integers() {
        let tokens = collect_tokens("abc a12 a12b34 123 $sum $let $ite");
        assert_eq!(
            tokens.iter().map(|token| token.0).collect::<Vec<_>>(),
            vec![
                TokenType::IDENT,
                TokenType::IDNUM,
                TokenType::IDNUM,
                TokenType::POS_INT,
                TokenType::SEM_IDENT,
                TokenType::LET_TOKEN,
                TokenType::ITE_TOKEN,
                TokenType::NO_TOKEN,
            ]
        );
        assert_eq!(tokens[2].1, "a12b34");

        let scanner = Scanner::from_user_string("a12b34", false).unwrap();
        assert!(test_idnum(scanner.current_token(), "a12b"));
        assert!(!test_idnum(scanner.current_token(), "a"));
    }

    #[test]
    fn scanner_parses_operator_literals_like_c_switch() {
        let tokens = collect_tokens(
            "( ) { } [ ] < <= <=> <~> > = => ~ ~| ~& ! != ? , ; : - + * . | / & $ @ ^",
        );
        let kinds = tokens.iter().map(|token| token.0).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                TokenType::OPEN_BRACKET,
                TokenType::CLOSE_BRACKET,
                TokenType::OPEN_CURLY,
                TokenType::CLOSE_CURLY,
                TokenType::OPEN_SQUARE,
                TokenType::CLOSE_SQUARE,
                TokenType::LESSER_SIGN,
                TokenType::FOF_RL_IMPL,
                TokenType::FOF_EQUIV,
                TokenType::FOF_XOR,
                TokenType::GREATER_SIGN,
                TokenType::EQUAL_SIGN,
                TokenType::FOF_LR_IMPL,
                TokenType::TILDE_SIGN,
                TokenType::FOF_NOR,
                TokenType::FOF_NAND,
                TokenType::EXCLAMATION,
                TokenType::NEG_EQUAL_SIGN,
                TokenType::QUESTION_MARK,
                TokenType::COMMA,
                TokenType::SEMICOLON,
                TokenType::COLON,
                TokenType::HYPHEN,
                TokenType::PLUS,
                TokenType::MULT,
                TokenType::FULLSTOP,
                TokenType::PIPE,
                TokenType::SLASH,
                TokenType::AMPERSAND,
                TokenType::DOLLAR,
                TokenType::APPLICATION,
                TokenType::CARRET,
                TokenType::NO_TOKEN,
            ]
        );
        assert_eq!(tokens[7].1, "<=");
        assert_eq!(tokens[8].1, "<=>");
        assert_eq!(tokens[12].1, "=>");
        assert_eq!(tokens[17].1, "!=");
    }

    #[test]
    fn scanner_keeps_quoted_string_literals_and_escapes() {
        let tokens = collect_tokens(r#""a\"b" 'c\'d'"#);
        assert_eq!(
            tokens[0],
            (TokenType::STRING, r#""a\"b""#.to_owned(), false)
        );
        assert_eq!(
            tokens[1],
            (TokenType::SQ_STRING, r"'c\'d'".to_owned(), true)
        );
    }

    #[test]
    fn scanner_reports_syntax_errors_for_illegal_characters_and_bad_strings() {
        let error = Scanner::from_user_string("`", false).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Illegal character"));

        let error = Scanner::from_user_string("\"unterminated", false).unwrap_err();
        assert_eq!(error.code(), ErrorCode::SYNTAX_ERROR);
        assert!(error.message().contains("Unterminated string constant"));
    }

    #[test]
    fn token_description_and_position_helpers_match_c_shapes() {
        assert_eq!(
            describe_token(TokenType::IDENT | TokenType::POS_INT),
            "Identifier not terminating in a number or Integer (sequence of decimal digits) convertible to an 'unsigned long'"
        );

        let scanner = Scanner::from_user_string("abc", false).unwrap();
        assert_eq!(
            token_pos_rep(scanner.current_token()),
            "Parsing a user provided string: \"abc\":1:(Column 1):"
        );
        assert!(test_id(scanner.current_token(), "abc|def"));
    }
}
