<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_scanner

## Source Files

- [INOUT/cio_scanner.h](../../../eprover/INOUT/cio_scanner.h)
- [INOUT/cio_scanner.c](../../../eprover/INOUT/cio_scanner.c)

## Purpose

Datatypes for the scanner: TokenType, TokenCell, TokenRepCell the GNU Lesser General Public License. <1> Thu Aug 28 01:48:03 MET DST 1997 New

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `IOFormat`
- `ScannerCell`
- `Scanner_p`
- `TokenCell`
- `TokenRepCell`
- `TokenRep_p`
- `TokenType`
- `Token_p`

### Macros And Constants

- `APP_VAR_MULT_DEFAULT`
- `AcceptInpId(in, ids)`
- `AcceptInpTok(in, toks)`
- `AcceptInpTokNoSkip(in, toks)`
- `AktToken(in)`
- `AktTokenType(in)`
- `Ampersand`
- `Application`
- `CIO_SCANNER`
- `Carret`
- `CloseBracket`
- `CloseCurly`
- `CloseSquare`
- `Colon`
- `Comma`
- `Comment`
- `CurrChar(scanner)`
- `CurrColumn(scanner)`
- `CurrLine(scanner)`
- `Dollar`
- `EqualSign`
- `Exclamation`
- `ExistQuantor`
- `FOFAnd`
- `FOFAssocOp`
- `FOFBinOp`
- `FOFEquiv`
- `FOFLRImpl`
- `FOFNand`
- `FOFNor`
- `FOFOr`
- `FOFRLImpl`
- `FOFXor`
- `Fullstop`
- `GreaterSign`
- `Hyphen`
- `Ident`
- `Identifier`
- `Idnum`
- `IteToken`
- `LambdaQuantor`
- `LesserSign`
- `LetToken`
- `LookChar(scanner, look)`
- `LookToken(in,look)`
- `MAXTOKENLOOKAHEAD`
- `Mult`
- `Name`
- `NegEqualSign`
- `NextChar(scanner)`
- `NoToken`
- `OpenBracket`
- `OpenCurly`
- `OpenSquare`
- `PARSE_OPTIONAL_AV_PENALTY(in, var_name)`
- `Pipe`
- `Plus`
- `PosInt`
- `QuestionMark`
- `SQString`
- `ScannerCellAlloc()`
- `ScannerCellFree(junk)`
- `ScannerGetDefaultDir(scanner)`
- `ScannerGetFormat(scanner)`
- `SemIdent`
- `Semicolon`
- `SkipToken`
- `Slash`
- `Source(scanner)`
- `SourceType(scanner)`
- `String`
- `TOKENREALPOS(pos)`
- `TestInpId(in, ids)`
- `TestInpIdnum(in, ids)`
- `TestInpNoSkip(in)`
- `TestInpTok(in, toks)`
- `TestInpTokNoSkip(in, toks)`
- `TildeSign`
- `TokenCellAlloc()`
- `TokenCellFree(junk)`
- ... 6 more

### Globals

- None found in the source scan.

### Exported Functions

- `(&((in)->tok_sequence[TOKENREALPOS((in)->current+(look))])) bool TestTok(Token_p akt, TokenType toks)`
- `(TestInpNoSkip(in) && TestInpTok(in, toks)) void AktTokenError(Scanner_p in, char* msg, bool syserr)`
- `NextToken(in) CheckInpTokNoSkip((in), (toks));\ NextToken(in) NextToken(in) void NextToken(Scanner_p in)`
- `Scanner_p CreateScanner(StreamType type, char *name, bool ignore_comments, char *default_dir, bool fail)`
- `Scanner_p ScannerParseInclude(Scanner_p in, StrTree_p *name_selector, StrTree_p *skip_includes)`
- `bool TestId(Token_p akt, char* ids)`
- `bool TestIdnum(Token_p akt, char* ids)`
- `char* DescribeToken(TokenType token)`
- `char* PosRep(StreamType type, DStr_p file, long line, long column)`
- `char* TokenPosRep(Token_p token)`
- `void AktTokenWarning(Scanner_p in, char* msg)`
- `void CheckInpId(Scanner_p in, char* ids)`
- `void CheckInpTok(Scanner_p in, TokenType toks)`
- `void CheckInpTokNoSkip(Scanner_p in, TokenType toks)`
- `void DestroyScanner(Scanner_p junk)`
- `void PrintToken(FILE* out, Token_p token)`
- `void ScannerSetFormat(Scanner_p scanner, IOFormat fmt)`

## Implementation Notes

### Internal Functions

- `compose_errmsg`
- `scan_C_comment`
- `scan_ident`
- `scan_int`
- `scan_line_comment`
- `scan_real_token`
- `scan_string`
- `scan_token`
- `scan_white`

### Source-Level Behavior

- `scan_white`: Scan a continous sequence of white space characters.
- `scan_ident`: Scan an identifier, d.h. an ident or an idnum. Also used for completing SemIdents.
- `scan_line_comment`: Scan a comment starting with # or %.
- `scan_string`: Scan a string (enclosed in "" or '').
- `scan_token`: Scans a token into AktToken(in). Does _not_ move the AktToken-pointer - this is done only for real (i.e. non white, non-comment) tokens in the function NextToken(). The function assumes that *AktToken(in) is an initialized TokenCell which does not contain any outside references.
- `scan_token_follow_includes`: Scan a token, follow include directives and pop back empty input / streams.
- `scan_real_token`: Scan tokens until a real token (i.e. not a SkipToken has been scanned.
- `compose_errmsg`: Compose position of current token and message into a DStr for futher processing.
- `str_n_element`: Test whether the len lenght start of str is contained in the set id of strings (encoded in a single string with elements separated by |).
- `PosRep`: Return a pointer to a description of a position in a file. The description is valid until the function is called the next time.
- `TokenPosRep`: Return a pointer to a description of the position of a token. The description is valid until the function or PosRep() is called the next time.
- `DescribeToken`: Return a pointer to a description of the set of tokens described by tok. The caller has to free the space of this description!
- `PrintToken`: Print a token (probably only for debugging purposes...
- `CreateScanner`: Create a new, initialized scanner from which tokens can be read immediately.
- `DestroyScanner`: Ensure that the scanner is disposed of cleanly, all files are closed and all memory/references are released.
- `ScannerSetFormat`: Set the format of the scanner (in particular, guess a format if
- `TestTok`: Compares the type of the given token with a list of possible tokens. Possibilities are values of type TokenType, possibly combined with the bitwise or operator '|'. The test is true if the given token matches at least one type from the list.
- `TestId`: Test whether a given token is of type identifier and is one of a set of possible alternatives. This set is given as a single C-String, alternatives are separated by the '|' character.
- `TestIdNum`: As TestId(), but take only the non-numerical-part into account.
- `AktTokenError`: Produce a syntax error at the current token with the given message. If syserror is true, this will also print the C library error message corresponding to the current value of errno.
- `AktTokenWarning`: Produce a warning at the current token with the given message.
- `CheckInpTok`: Check whether AktTok(in) is of one of the desired types. Produce error if not.
- `CheckInpTokNoSkip`: As CheckInpTok(), but produce an error if SkipTokens were present.
- `CheckInpId`: Check whether AktToken(in) is an identifier with the desired value. Produce error if not.
- `NextToken`: Read a new token, switch to the next token in the queue.
- `ScannerParseInclude`: Parse a TPTP-Style include statement. Return a scanner for the included file, and put (optional) selected names into name_selector. If the file name is in skip_includes, skip the rest and return NULL.

### Dependencies

- `"cio_scanner.h"`
- `<cio_streams.h>`
- `<clb_stringtrees.h>`
- `<ctype.h>`
- `<limits.h>`

### Compile-Time Conditions

- `CIO_SCANNER`
- `ENABLE_LFHO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_scanner.h`, `INOUT/cio_scanner.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 1621 lines, 25 scanned public declarations, 9 scanned internal function definitions, and 26 structured function-comment blocks.
- Scanner/tokenizer core. Buffer ownership, include stacks, position tracking, and token lookahead are parser contracts.
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.

### Rust Port Status Notes

- `src/inout/scanner.rs` ports the core token bit layout, token lookahead queue, token descriptions, position formatting, `PrintToken`-style debug rendering, identifier/idnum tests, format auto-detection, string/file/in-memory source construction, C-style default-directory/include lookup with a Windows-native top-level path normalization shim, automatic `include_key` file splicing, and explicit `ScannerParseInclude`-style include parsing with selector and skip-name trees.
- The scanner now stores its active input sources in `InputStreamStack`, so automatic include splicing pushes included files as the current top stream and pops back to the parent stream at EOF like C `OpenStackedInput`/`CloseStackedInput`.
- Tests cover token classification, lookahead, comments and skipped-token state, syntax diagnostics, format auto-detection, default-dir plus `TPTP` include lookup, explicit include parsing, selector handling, skip-tree handling, and nested automatic include splicing back to parent streams.

### Change Later

- C automatic include splicing is driven by `include_key`, but this checkout initializes that field to `NULL` and exposes no public setter. Rust keeps an explicit constructor for compatibility tests and supported callers; decide later whether a broader public switch is needed when every parser path uses the same include owner.
- C scanner file streams hold live `FILE*` handles and close them through `DestroyScanner`; Rust loads file bytes eagerly into `InputStream`. This avoids close-time ownership hazards but should be benchmarked and possibly replaced with a lazy backend before large-file parser parity is claimed.
- C scanner default-directory handling composes include paths with the slash-only `FileNameDirName`/`FileNameBaseName` helpers. Rust preserves that composition for scanner names but normalizes Windows-native top-level file paths before deriving `default_dir`, so executable callers can still resolve TPTP-style relative includes such as `Axioms/...` when invoked from a Windows shell. A full path policy should keep this boundary explicit rather than making every C-shaped filename helper platform-aware.
- C `PrintToken` writes directly to a `FILE*`. Rust exposes the same text shape as both a string renderer and an `io::Write` helper so callers can choose their output owner; keep this wrapper split unless a byte-level C FILE bridge is required.
- `ScannerParseInclude` represents `include(file,[])` by inserting the magic selector name `"** Not a legal name**"` with a nonzero dummy value. Rust preserves that sentinel so empty selector lists select no entries without triggering missing-selector diagnostics; a future cleaned parser API should model the empty-selection state explicitly instead of exposing an impossible-name marker.
- `ScannerParseInclude` and automatic include splicing are separate C paths with slightly different state flow. Rust keeps both represented; the full parser should choose one policy per caller rather than mixing implicit and explicit include behavior accidentally.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
