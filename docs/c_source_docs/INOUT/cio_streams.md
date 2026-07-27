<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_streams

## Source Files

- [INOUT/cio_streams.h](../../../eprover/INOUT/cio_streams.h)
- [INOUT/cio_streams.c](../../../eprover/INOUT/cio_streams.c)

## Purpose

Definitions for a stream type, i.e. an object associated with a file pointer (and possibly a file name), allowing read operations, arbitrary look-aheads, and maintaining line and column numbers for error messages.

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `Inpstack_p`
- `StreamCell`
- `StreamType`
- `Stream_p`

### Macros And Constants

- `CIO_STREAMS`
- `MAXLOOKAHEAD`
- `STREAMREALPOS(pos)`
- `StreamCellAlloc()`
- `StreamCellFree(junk)`
- `StreamCurrChar(stream)`
- `StreamCurrColumn(stream)`
- `StreamCurrLine(stream)`
- `StreamLookChar(stream, look)`

### Globals

- `extern const StreamType StreamTypeFile`
- `extern const StreamType StreamTypeInternalString`
- `extern const StreamType StreamTypeOptionString`
- `extern const StreamType StreamTypeUserString`

### Exported Functions

- `(assert((look)<MAXLOOKAHEAD),\ (stream)->buffer[STREAMREALPOS((stream)->current+(look))]) int StreamNextChar(Stream_p stream)`
- `Stream_p CreateStream(StreamType type, char* source, bool fail)`
- `Stream_p OpenStackedInput(Inpstack_p stack, StreamType type, char* source, bool fail)`
- `void CloseStackedInput(Inpstack_p stack)`
- `void DestroyStream(Stream_p stream)`

## Implementation Notes

### Internal Functions

- `read_char`

### Source-Level Behavior

- `read_char`: Read a character and return it. Return an infinite sequence of EOFs after the end of file.
- `CreateStream`: Create a stream associated with the file name. Both the NULL-pointer and the name "-" are taken to mean stdin.
- `DestroyStream`: Free all resources (memory, file handle) associated with the stream.
- `StreamNextChar`: Move the current window on the input stream one character forward. Return the new CurrChar().
- `OpenStackedInput`: Open a new input stream and put it on top of the stack. All further input from this stack is read from the new top of the stack.
- `CloseStackedInput`: Pop the top from the input stack and destroy the associated stream.

### Dependencies

- `"cio_streams.h"`
- `<cio_fileops.h>`
- `<cio_initio.h>`

### Compile-Time Conditions

- `CIO_STREAMS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_streams.h`, `INOUT/cio_streams.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 403 lines, 13 scanned public declarations, 1 scanned internal function definitions, and 6 structured function-comment blocks.
- Definitions for a stream type, i.e. an object associated with a file pointer (and possibly a file name), allowing read operations, arbitrary look-aheads, and maintaining line and column numbers for error messages.
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- `src/inout/streams.rs` ports the stream type discriminants, 64-character lookahead window, source label storage, string/file-backed stream constructors including `CreateStream`-style fail-or-null file opening with C's named-path `stat` preflight and `NULL`/`"-"` stdin labeling as `"<stdin>"`, C line/column update rules, NUL/end-of-input infinite EOF behavior, and `STREAMREALPOS` circular-buffer indexing. The complete live scanner-diagnostic comparison is retained in [`experiment 127`](../../../experiments/2026-07-18-127-support-tool-matrix-closure/FINDINGS.md).
- Rust now also represents `OpenStackedInput`/`CloseStackedInput` with an owned `InputStreamStack` that pushes a new top stream, exposes top access, pops back to the previous stream in LIFO order, and offers a C-shaped asserting close for nonempty-stack compatibility paths; `Scanner` uses this stack for automatic include splicing.
- Tests cover lookahead prefill, line/column movement, NUL-triggered EOF, file source labels, fail-or-null missing-file opening, C-shaped stdin source labeling, string-source construction, file-named in-memory sources for stdin-like data, stacked stream push/pop restoration, and the `CloseStackedInput` nonempty-stack assertion.

### Change Later

- C overloads both a null source pointer and the filename string `-` to mean stdin, then replaces either with the display label `<stdin>`. Rust preserves the accepted spellings and label at compatibility boundaries, but a cleaned stream API should use an explicit stdin/file source enum so path data and source kind cannot be confused.
- Rust file and stdin streams still load the bytes eagerly during construction. Revisit lazy streaming if large-problem parsing, interactive stdin use, or include-stack behavior makes the C `FILE*` window observable.
- C `CloseStackedInput` asserts that the stack is nonempty and destroys the popped stream. Rust keeps the optional pop for reusable callers and now exposes an asserting compatibility wrapper for direct C-shaped paths.
- C `DestroyStream` can report `fclose` failures for file streams. Rust owns file-backed stream bytes eagerly, so close-time diagnostics are not represented; revisit only if a lazy `FILE*`-style stream backend is introduced.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
