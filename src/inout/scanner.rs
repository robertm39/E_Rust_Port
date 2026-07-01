use std::ops::BitOr;

use crate::basics::dstrings::DynamicString;
use crate::basics::error::{Diagnostic, ErrorCode};
use crate::basics::stringtrees::StrTree;
use crate::inout::fileops::{file_name_base_name, file_name_dir_name, file_name_is_absolute};
use crate::inout::initio::tptp_dir;
use crate::inout::streams::{InputStream, InputStreamStack, StreamType};
use std::io;
use std::path::Path;

pub const MAX_TOKEN_LOOKAHEAD: usize = 4;
pub const EMPTY_INCLUDE_SELECTOR_SENTINEL: &str = "** Not a legal name**";

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
    source_stack: InputStreamStack,
    default_dir: String,
    ignore_comments: bool,
    format: IoFormat,
    include_key: Option<String>,
    include_pos: Option<String>,
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
        Self::from_file_with_default_dir(path, ignore_comments, None)
    }

    pub fn from_file_content(
        source_name: &str,
        data: Vec<u8>,
        ignore_comments: bool,
    ) -> Result<Self, Diagnostic> {
        Self::from_stream(
            InputStream::from_file_content(source_name, data),
            ignore_comments,
        )
    }

    pub fn from_file_following_includes(
        path: &Path,
        ignore_comments: bool,
        include_key: &str,
    ) -> Result<Self, Diagnostic> {
        Self::from_file_with_options(path, ignore_comments, None, Some(include_key.to_owned()))
    }

    pub fn from_file_with_default_dir(
        path: &Path,
        ignore_comments: bool,
        default_dir: Option<&str>,
    ) -> Result<Self, Diagnostic> {
        Self::from_file_with_options(path, ignore_comments, default_dir, None)
    }

    fn from_file_with_options(
        path: &Path,
        ignore_comments: bool,
        default_dir: Option<&str>,
        include_key: Option<String>,
    ) -> Result<Self, Diagnostic> {
        let name = path.to_string_lossy();
        let (stream, resolved_default_dir) = create_file_stream(&name, default_dir)?;
        let mut scanner = Self::from_stream_with_include_key(stream, ignore_comments, include_key)?;
        scanner.default_dir = resolved_default_dir;
        Ok(scanner)
    }

    pub fn from_stream(source: InputStream, ignore_comments: bool) -> Result<Self, Diagnostic> {
        Self::from_stream_with_include_key(source, ignore_comments, None)
    }

    fn from_stream_with_include_key(
        source: InputStream,
        ignore_comments: bool,
        include_key: Option<String>,
    ) -> Result<Self, Diagnostic> {
        let mut source_stack = InputStreamStack::new();
        source_stack.open_stacked_input(source);
        let mut scanner = Self {
            source_stack,
            default_dir: String::new(),
            ignore_comments,
            format: IoFormat::Lop,
            include_key,
            include_pos: None,
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

    /// Return and clear the comments accumulated before the current token.
    ///
    /// This mirrors C callers that print `AktToken(in)->comment` and then
    /// reset the dynamic string.
    #[must_use]
    pub fn take_current_comment_bytes(&mut self) -> Vec<u8> {
        let token = &mut self.tok_sequence[self.current];
        let comment = token.comment.view_bytes().to_vec();
        token.comment.reset();
        comment
    }

    #[must_use]
    pub const fn format(&self) -> IoFormat {
        self.format
    }

    #[must_use]
    pub fn default_dir(&self) -> &str {
        &self.default_dir
    }

    #[must_use]
    pub fn include_pos(&self) -> Option<&str> {
        self.include_pos.as_deref()
    }

    fn source(&self) -> &InputStream {
        self.source_stack
            .top()
            .unwrap_or_else(|| panic!("scanner source stack must not be empty"))
    }

    fn source_mut(&mut self) -> &mut InputStream {
        self.source_stack
            .top_mut()
            .unwrap_or_else(|| panic!("scanner source stack must not be empty"))
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

    pub fn parse_include(
        &mut self,
        name_selector: &mut StrTree<i64, i64>,
        skip_includes: &StrTree<i64, i64>,
    ) -> Result<Option<Self>, Diagnostic> {
        let include_pos = token_pos_rep(self.current_token());
        self.accept_id("include")?;
        self.accept_tok(TokenType::OPEN_BRACKET)?;
        self.check_tok(TokenType::SQ_STRING)?;
        let name = strip_quote_core(self.current_token().literal_bytes())?;

        let included = if skip_includes.find(&name).is_none() {
            let mut scanner = Self::from_file_with_default_dir(
                Path::new(&name),
                self.ignore_comments,
                Some(&self.default_dir),
            )?;
            scanner.set_format(self.format);
            scanner.include_pos = Some(include_pos);
            Some(scanner)
        } else {
            None
        };

        self.next_token()?;
        if self.test_tok(TokenType::COMMA) {
            self.next_token()?;
            self.check_tok(TokenType::NAME | TokenType::POS_INT | TokenType::OPEN_SQUARE)?;
            if self.test_tok(TokenType::NAME | TokenType::POS_INT) {
                name_selector.store(&self.current_token().literal(), 0, 0);
                self.next_token()?;
            } else {
                self.accept_tok(TokenType::OPEN_SQUARE)?;
                if self.test_tok(TokenType::CLOSE_SQUARE) {
                    name_selector.store(EMPTY_INCLUDE_SELECTOR_SENTINEL, 1, 0);
                } else {
                    name_selector.store(&self.current_token().literal(), 0, 0);
                    self.accept_tok(TokenType::NAME | TokenType::POS_INT)?;
                    while self.test_tok(TokenType::COMMA) {
                        self.next_token()?;
                        name_selector.store(&self.current_token().literal(), 0, 0);
                        self.accept_tok(TokenType::NAME | TokenType::POS_INT)?;
                    }
                }
                self.accept_tok(TokenType::CLOSE_SQUARE)?;
            }
        }
        self.accept_tok(TokenType::CLOSE_BRACKET)?;
        self.accept_tok(TokenType::FULLSTOP)?;

        Ok(included)
    }

    fn scan_real_token(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.tok_sequence[index].skipped = false;
        self.tok_sequence[index].comment.reset();
        self.scan_token_follow_includes(index)?;

        while test_tok(&self.tok_sequence[index], TokenType::SKIP_TOKEN) {
            self.tok_sequence[index].skipped = true;
            if !self.ignore_comments && test_tok(&self.tok_sequence[index], TokenType::COMMENT) {
                let comment = self.tok_sequence[index].literal.copy();
                self.tok_sequence[index].comment.append_buffer(&comment);
            }
            self.scan_token_follow_includes(index)?;
        }
        Ok(())
    }

    fn scan_token_follow_includes(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.scan_token(index)?;
        let follows_include = self
            .include_key
            .as_deref()
            .is_some_and(|include_key| test_id(&self.tok_sequence[index], include_key));

        if follows_include {
            let mut name = automatic_include_prefix();
            self.scan_token(index)?;
            self.check_scanned_token(index, TokenType::OPEN_BRACKET)?;
            self.scan_token(index)?;
            self.check_scanned_token(
                index,
                TokenType::IDENT | TokenType::STRING | TokenType::SQ_STRING,
            )?;
            if test_tok(&self.tok_sequence[index], TokenType::IDENT) {
                name.push_str(&self.tok_sequence[index].literal());
            } else {
                name.push_str(&strip_quote_core(self.tok_sequence[index].literal_bytes())?);
            }
            self.push_file_source(Path::new(&name))?;
            self.scan_token_follow_includes(index)?;
        } else if self.include_key.is_some()
            && test_tok(&self.tok_sequence[index], TokenType::NO_TOKEN)
            && self.pop_source()
        {
            self.scan_token(index)?;
            self.check_scanned_token(index, TokenType::CLOSE_BRACKET)?;
            self.scan_token(index)?;
            self.check_scanned_token(index, TokenType::FULLSTOP)?;
            self.scan_token_follow_includes(index)?;
        }
        Ok(())
    }

    fn scan_token(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.reset_scanned_token(index);
        match self.source().current_char() {
            None => self.tok_sequence[index].kind = TokenType::NO_TOKEN,
            Some(byte) if byte.is_ascii_whitespace() => self.scan_white(index),
            Some(byte) if is_start_id_char(byte) => self.scan_ident(index),
            Some(byte) if byte.is_ascii_digit() => self.scan_int(index),
            Some(b'#' | b'%') => self.scan_line_comment(index),
            Some(b'/') if self.source().look_char(1) == Some(b'*') => {
                self.scan_c_comment(index)?;
            }
            Some(delimiter @ (b'"' | b'\'')) => self.scan_string(index, delimiter)?,
            Some(b'$') if self.source().look_char(1).is_some_and(is_id_char) => {
                self.scan_semantic_identifier(index);
            }
            Some(_) => self.scan_punctuation(index)?,
        }
        Ok(())
    }

    fn reset_scanned_token(&mut self, index: usize) {
        let source = self.source();
        let source_bytes = source.source_bytes().to_vec();
        let stream_type = source.stream_type();
        let line = source.line();
        let column = source.column();
        let token = &mut self.tok_sequence[index];
        token.literal.reset();
        token.source = source_bytes;
        token.stream_type = stream_type;
        token.line = line;
        token.column = column;
        token.numval = 0;
        token.kind = TokenType::NO_TOKEN;
    }

    fn scan_white(&mut self, index: usize) {
        self.tok_sequence[index].kind = TokenType::WHITE_SPACE;
        while let Some(byte) = self.source().current_char() {
            if !byte.is_ascii_whitespace() {
                break;
            }
            self.append_current_and_advance(index);
        }
    }

    fn scan_ident(&mut self, index: usize) {
        let mut numstart = 0_usize;
        for offset in 0_usize.. {
            let Some(byte) = self.source().current_char() else {
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
        while let Some(byte) = self.source().current_char() {
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
        while let Some(byte) = self.source().current_char() {
            if byte == b'\n' {
                break;
            }
            self.append_current_and_advance(index);
        }
        self.tok_sequence[index].literal.append_byte(b'\n');
        self.source_mut().next_char();
    }

    fn scan_c_comment(&mut self, index: usize) -> Result<(), Diagnostic> {
        self.tok_sequence[index].kind = TokenType::COMMENT;
        while !(self.source().current_char() == Some(b'*')
            && self.source().look_char(1) == Some(b'/'))
        {
            if self.source().current_char().is_none() {
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
            let Some(byte) = self.source().current_char() else {
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
        let kind = match self.source().current_char() {
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
        if self.source().look_char(1) == Some(b'~') && self.source().look_char(2) == Some(b'>') {
            self.append_current_and_advance(index);
            self.append_current_and_advance(index);
            TokenType::FOF_XOR
        } else if self.source().look_char(1) == Some(b'=') {
            self.append_current_and_advance(index);
            if self.source().look_char(1) == Some(b'>') {
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
        if self.source().look_char(1) == Some(b'>') {
            self.append_current_and_advance(index);
            TokenType::FOF_LR_IMPL
        } else {
            TokenType::EQUAL_SIGN
        }
    }

    fn scan_tilde_prefixed_operator(&mut self, index: usize) -> TokenType {
        match self.source().look_char(1) {
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
        if self.source().look_char(1) == Some(b'=') {
            self.append_current_and_advance(index);
            TokenType::NEG_EQUAL_SIGN
        } else {
            TokenType::EXCLAMATION
        }
    }

    fn append_current_and_advance(&mut self, index: usize) {
        if let Some(byte) = self.source().current_char() {
            self.tok_sequence[index].literal.append_byte(byte);
        }
        self.source_mut().next_char();
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

    fn check_scanned_token(&self, index: usize, toks: TokenType) -> Result<(), Diagnostic> {
        if test_tok(&self.tok_sequence[index], toks) {
            Ok(())
        } else {
            Err(self.token_error(
                index,
                &format!(
                    "{} expected, but {} read ",
                    describe_token(toks),
                    describe_token(self.tok_sequence[index].kind)
                ),
            ))
        }
    }

    fn push_file_source(&mut self, path: &Path) -> Result<(), Diagnostic> {
        let stream = InputStream::from_file(path)?;
        self.source_stack.open_stacked_input(stream);
        Ok(())
    }

    fn pop_source(&mut self) -> bool {
        if self.source_stack.len() <= 1 {
            return false;
        }
        self.source_stack.close_stacked_input().is_some()
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

#[must_use]
pub fn token_print_string(token: &Token) -> String {
    format!(
        "Token:    {} = {}\nPosition: {}   Literal:  {}\nNumval:   {:6}   Skipped:  {}\nComment:  {}\n",
        token.kind.bits(),
        describe_token(token.kind),
        token_pos_rep(token),
        token.literal(),
        token.numval(),
        if token.skipped() { "true" } else { "false" },
        String::from_utf8_lossy(token.comment_bytes())
    )
}

pub fn print_token(output: &mut impl io::Write, token: &Token) -> io::Result<()> {
    output.write_all(token_print_string(token).as_bytes())
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

fn create_file_stream(
    name: &str,
    default_dir: Option<&str>,
) -> Result<(InputStream, String), Diagnostic> {
    if file_name_is_absolute(name) {
        let stream = InputStream::from_file(Path::new(name))?;
        return Ok((stream, file_name_dir_name(name)));
    }

    let mut local_default_dir = String::new();
    if let Some(default_dir) = default_dir {
        local_default_dir.push_str(default_dir);
        debug_assert!(local_default_dir.is_empty() || local_default_dir.ends_with('/'));
    }
    local_default_dir.push_str(&file_name_dir_name(name));
    let local_name = format!("{}{}", local_default_dir, file_name_base_name(name));

    match InputStream::from_file(Path::new(&local_name)) {
        Ok(stream) => Ok((stream, local_default_dir)),
        Err(local_error) => {
            let Some(mut fallback_default_dir) = tptp_dir() else {
                return Err(local_error);
            };
            fallback_default_dir.push_str(&file_name_dir_name(name));
            let fallback_name = format!("{}{}", fallback_default_dir, file_name_base_name(name));
            let stream = InputStream::from_file(Path::new(&fallback_name))?;
            Ok((stream, fallback_default_dir))
        }
    }
}

fn strip_quote_core(bytes: &[u8]) -> Result<String, Diagnostic> {
    if bytes.len() < 2 {
        return Err(Diagnostic::new(
            ErrorCode::SYNTAX_ERROR,
            "Quoted string literal is too short",
        ));
    }
    Ok(String::from_utf8_lossy(&bytes[1..bytes.len() - 1]).into_owned())
}

fn automatic_include_prefix() -> String {
    let Some(value) = std::env::var_os("TPTP") else {
        return String::new();
    };
    let mut prefix = value.to_string_lossy().into_owned();
    if !prefix.is_empty() && !prefix.ends_with('/') {
        prefix.push('/');
    }
    prefix
}

#[cfg(test)]
mod tests {
    use super::{
        describe_token, print_token, test_id, test_idnum, token_pos_rep, token_print_string,
        IoFormat, Scanner, TokenType, EMPTY_INCLUDE_SELECTOR_SENTINEL,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::stringtrees::StrTree;
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

    fn remove_dir_if_present(path: &Path) {
        _ = std::fs::remove_dir_all(path);
    }

    fn temp_dir(name: &str) -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("scanner-{name}-{}", std::process::id()))
    }

    fn slash_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
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
    fn scanner_can_read_file_named_in_memory_content() {
        let scanner = Scanner::from_file_content("-", b"  abc".to_vec(), false).unwrap();

        assert_eq!(scanner.current_token().literal(), "abc");
        assert_eq!(
            scanner.current_token().stream_type(),
            super::StreamType::File
        );
        assert_eq!(scanner.current_token().source_bytes(), b"-");
        assert_eq!(token_pos_rep(scanner.current_token()), "-:1:(Column 3):");
    }

    #[test]
    fn token_print_string_matches_c_debug_shape() {
        let scanner = Scanner::from_file_content("-", b"  abc".to_vec(), false).unwrap();
        let token = scanner.current_token();
        let expected = format!(
            "Token:    8 = Identifier not terminating in a number\n\
             Position: {}   Literal:  abc\n\
             Numval:   {:6}   Skipped:  true\n\
             Comment:  \n",
            token_pos_rep(token),
            0
        );

        assert_eq!(token_print_string(token), expected);

        let mut printed = Vec::new();
        print_token(&mut printed, token).unwrap();
        assert_eq!(printed, expected.as_bytes());
    }

    #[test]
    fn file_scanner_resolves_c_style_default_directories() {
        let dir = temp_dir("default-dir");
        remove_dir_if_present(&dir);
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        let root_path = dir.join("main.p");
        let nested_path = dir.join("nested").join("child.ax");
        std::fs::write(&root_path, b"root").unwrap();
        std::fs::write(&nested_path, b"child").unwrap();

        let root_name = slash_path(&root_path);
        let scanner = Scanner::from_file(Path::new(&root_name), false).unwrap();
        assert_eq!(scanner.current_token().literal(), "root");
        assert_eq!(scanner.default_dir(), format!("{}/", slash_path(&dir)));

        let nested = Scanner::from_file_with_default_dir(
            Path::new("nested/child.ax"),
            false,
            Some(scanner.default_dir()),
        )
        .unwrap();
        assert_eq!(nested.current_token().literal(), "child");
        assert_eq!(
            nested.default_dir(),
            format!("{}/nested/", slash_path(&dir))
        );

        remove_dir_if_present(&dir);
    }

    #[test]
    fn parse_include_opens_relative_file_and_collects_selectors() {
        let dir = temp_dir("include");
        remove_dir_if_present(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root_path = dir.join("main.p");
        let child_path = dir.join("child.ax");
        std::fs::write(&root_path, b"include('child.ax',[foo,12]). tail").unwrap();
        std::fs::write(&child_path, b"cnf(child,axiom,p).").unwrap();

        let root_name = slash_path(&root_path);
        let mut scanner = Scanner::from_file(Path::new(&root_name), false).unwrap();
        scanner.set_format(IoFormat::Tstp);
        let mut selectors = StrTree::new();
        let skip = StrTree::new();

        let included = scanner
            .parse_include(&mut selectors, &skip)
            .unwrap()
            .unwrap();

        assert_eq!(scanner.current_token().literal(), "tail");
        assert_eq!(included.current_token().literal(), "cnf");
        assert_eq!(included.format(), IoFormat::Tstp);
        let expected_include_pos = format!("{root_name}:1:(Column 1):");
        assert_eq!(included.include_pos(), Some(expected_include_pos.as_str()));
        assert!(selectors.find("foo").is_some());
        assert!(selectors.find("12").is_some());

        remove_dir_if_present(&dir);
    }

    #[test]
    fn parse_include_honors_skip_tree_and_empty_selector_sentinel() {
        let mut scanner = Scanner::from_user_string("include('skip.ax',[]). done", false).unwrap();
        let mut selectors = StrTree::new();
        let mut skip = StrTree::new();
        assert!(skip.store("skip.ax", 0, 0));

        let included = scanner.parse_include(&mut selectors, &skip).unwrap();

        assert!(included.is_none());
        assert_eq!(scanner.current_token().literal(), "done");
        assert_eq!(
            selectors
                .find(EMPTY_INCLUDE_SELECTOR_SENTINEL)
                .map(|entry| entry.val1),
            Some(1)
        );
    }

    #[test]
    fn include_key_splices_included_files_and_resumes_parent_stream() {
        let dir = temp_dir("include-key");
        remove_dir_if_present(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root_path = dir.join("root.p");
        let child_path = dir.join("child.ax");
        let grand_path = dir.join("grand.ax");
        let child_name = slash_path(&child_path);
        let grand_name = slash_path(&grand_path);
        std::fs::write(
            &root_path,
            format!("include('{child_name}'). root_tail").as_bytes(),
        )
        .unwrap();
        std::fs::write(
            &child_path,
            format!("include('{grand_name}'). child_tail").as_bytes(),
        )
        .unwrap();
        std::fs::write(&grand_path, b"grand").unwrap();

        let root_name = slash_path(&root_path);
        let mut scanner =
            Scanner::from_file_following_includes(Path::new(&root_name), false, "include").unwrap();

        assert_eq!(scanner.current_token().literal(), "grand");
        scanner.next_token().unwrap();
        assert_eq!(scanner.current_token().literal(), "child_tail");
        scanner.next_token().unwrap();
        assert_eq!(scanner.current_token().literal(), "root_tail");
        scanner.next_token().unwrap();
        assert_eq!(scanner.current_token().kind(), TokenType::NO_TOKEN);

        remove_dir_if_present(&dir);
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
