<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_basicparser

## Source Files

- [INOUT/cio_basicparser.h](../../../eprover/INOUT/cio_basicparser.h)
- [INOUT/cio_basicparser.c](../../../eprover/INOUT/cio_basicparser.c)

## Purpose

Parsing routines for useful C build-in ans some general CLIB datatypes not covered by the scanner. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `StrNumType`

### Macros And Constants

- `CIO_BASICPARSER`
- `DECIMAL_DOT`
- `PLAIN_FILE_TOKENS`

### Globals

- None found in the source scan.

### Exported Functions

- `StrNumType ParseNumString(Scanner_p in)`
- `bool ParseBool(Scanner_p in)`
- `char* ParseBasicInclude(Scanner_p in)`
- `char* ParseContinous(Scanner_p in)`
- `char* ParseDottedId(Scanner_p in)`
- `char* ParseFilename(Scanner_p in)`
- `char* ParsePlainFilename(Scanner_p in)`
- `double ParseFloat(Scanner_p in)`
- `intmax_t ParseIntMax(Scanner_p in)`
- `long DDArrayParse(Scanner_p in, DDArray_p array, bool brackets)`
- `long ParseInt(Scanner_p in)`
- `long ParseIntLimited(Scanner_p in, long lower, long upper)`
- `uintmax_t ParseUIntMax(Scanner_p in)`
- `void AcceptDottedId(Scanner_p in, char* expected)`
- `void ParseSkipParenthesizedExpr(Scanner_p in)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ParseBool`: Parse and return a Boolean value (true/false).
- `ParseIntMax`: Parses a (possibly negative) Integer, defined as an optional "-", followed by a sequence of digits. Returns the value or gives an error on overflow.
- `ParseIntLimited`: Parses a (possibly negative) Integer, defined as an optional "-", followed by a sequence of digits. Returns the value or gives an error on overflow.
- `ParseInt`: Parses a (possibly negative) (long) Integer, defined as an optional "-", // followed by a sequence of digits. Returns the value or gives an error on overflow.
- `ParseUIntMax`: Parses an uintmax-Integer, a sequence of digits. Returns the value or gives an error on overflow.
- `ParseFloat`: Parse a float in x.yEz format (optional negative and so on...)
- `ParseNumString`: Parse a (possibly signed) number (Integer, Rational, or Float) and return the most specific type compatible with it. The number is not evaluated, but its ASCII representation is stored in in->accu.
- `DDArrayParse`: Parse a coma-delimited list of double values into array. If brackets is true, expect the list to be enclosed into (). Return the number of values parsed.
- `ParseFilename`: Parse a filename and return it. Note that we only allow reasonably "normal" filenames or strings, i.e. not spaces, non-printables, most meta-charachters, or quotes.
- `ParsePlainFileName`: Parse a local file name (without /) and return it. The caller has to free the allocated memory!
- `ParseBasicInclude`: Parse a basic TPTP-3 include (without optional selector), return the file name (which the caller has to free).
- `ParseDottedId`: Parse a sequence id1.id2.id2 ... and return it as a string.
- `AcceptDottedId`: Parse a sequence id1.id2.id2..., check it against an expected value, and skip it. Print error and terminate on mismatch.
- `ParseContinous`: Parse a sequence of tokens with no whitespace and return the result as a string.
- `ParseSkipParenthesizedExpr`: Skip any expression containing balanced (), [], {}. Print error on missmatch. Note that no full syntax check is performed, we are only interested in the different braces.

### Dependencies

- `"cio_basicparser.h"`
- `<cio_scanner.h>`
- `<clb_ddarrays.h>`
- `<clb_pstacks.h>`
- `<stdlib.h>`

### Compile-Time Conditions

- `ALLOW_COMMA_AS_DECIMAL_DOT`
- `CIO_BASICPARSER`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_basicparser.h`, `INOUT/cio_basicparser.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 724 lines, 16 scanned public declarations, 0 scanned internal function definitions, and 15 structured function-comment blocks.
- Shared parser helpers. Token acceptance/checking behavior is intentionally fatal on malformed input.
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `ParseIntLimited` accepts the exact LP64 `LONG_MIN` spelling `-9223372036854775808` via unsigned-token arithmetic, while still rejecting `-0` as underflow. Rust preserves that sentinel behavior so generated strategy files can parse C's `LONG_MIN` fields.

### Rust Port Status Notes

- `src/inout/basicparser.rs` ports the shared parser helpers for booleans, signed and unsigned integers, floats, number-string classification, double arrays, filename token spans, basic includes, dotted identifiers, continuous no-whitespace token spans, and balanced delimiter skipping.
- The Rust parser keeps token-consumption behavior explicit with `Scanner` methods and returns diagnostics instead of terminating directly, while callers that model C fatal parse paths can still surface those diagnostics as fatal errors.
- Tests cover the C `ParseIntMax` sign quirk, `ParseIntLimited`'s LP64 `LONG_MIN` boundary, missing whitespace after a sign, numeric spelling classification, filename token stopping, include/dotted-id parsing, continuous spans, and delimiter mismatch diagnostics.

### Change-Later Observations

- C `ParseIntMax` negates the parsed magnitude in both the signed and unsigned branches. Rust preserves that surprising behavior for compatibility; after reference coverage proves no caller depends on it, this should be audited as a likely C bug.
- C `ParseNumString` normalizes exponent markers to lowercase `e` for separated exponent tokens but preserves the raw `Idnum` spelling for compact forms such as `8e9`. Rust mirrors that split, but a future numeric token API could expose normalized and raw spellings separately.
- C delimiter skipping is intentionally not a full syntax parser; it only tracks balanced `()`, `[]`, and `{}`. Keep the Rust helper as a syntax-skipping compatibility shim rather than reusing it for semantic parsing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
