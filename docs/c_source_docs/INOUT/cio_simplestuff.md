<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_simplestuff

## Source Files

- [INOUT/cio_simplestuff.h](../../../eprover/INOUT/cio_simplestuff.h)
- [INOUT/cio_simplestuff.c](../../../eprover/INOUT/cio_simplestuff.c)

## Purpose

Simple functions for simple operations that don't quite fit elsewhere. <1> Fri Jul 27 01:33:21 CEST 2012 New

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CIO_SIMPLESTUFF`

### Globals

- None found in the source scan.

### Exported Functions

- `bool ReadTextBlock(DStr_p result, FILE* fp, char* terminator)`
- `bool TCPReadTextBlock(DStr_p result, int fd, char* terminator)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ReadTextBlock`: Read lines from fp until terminator is encountered (on a line by itself). Note that termiantor has to end in \n for this to ever work. The read text, up to, but not including, terminator, is appended to result (which is not cleared!). Returns success/failure.
- `TCPReadTextBlock`: Read lines from network socket until terminator is encountered (on a line by itself). Note that termiantor has to end in \n for this to ever work. The read text, up to, but not including, terminator, is appended to result (which is not cleared!). Returns success/failure.

### Dependencies

- `"cio_simplestuff.h"`
- `<cio_network.h>`
- `<cio_output.h>`

### Compile-Time Conditions

- `CIO_SIMPLESTUFF`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_simplestuff.h`, `INOUT/cio_simplestuff.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 181 lines, 2 scanned public declarations, 0 scanned internal function definitions, and 2 structured function-comment blocks.
- Simple functions for simple operations that don't quite fit elsewhere. <1> Fri Jul 27 01:33:21 CEST 2012 New
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- `src/inout/simplestuff.rs` ports `ReadTextBlock` with C-shaped `fgets(buf, 256, ...)` chunking, append-without-clearing semantics, exact terminator-line comparison, and false-on-EOF behavior.
- `TCPReadTextBlock` is represented both as an iterator-backed helper for already received message strings and as a network-backed helper over the ported `TcpMessage` receive loop.
- Tests cover append preservation, EOF after partial append, 255-byte chunk boundaries, iterator-backed TCP text blocks, network-message text blocks, and receive-failure diagnostics.

### Change-Later Observations

- Both C functions require the caller-supplied terminator to include the trailing newline for line-based input to stop. Rust keeps byte-exact terminator matching for compatibility; a later higher-level API could make the line terminator policy explicit instead of relying on callers to remember the newline.
- C `TCPReadTextBlock` calls `TCPStringRecvX`, so receive errors are fatal and the function returns `true` once a terminator is seen. Rust's network-backed helper returns a diagnostic on receive failure; keep any future executable-facing compatibility wrapper responsible for converting that diagnostic back into C's fatal-error surface.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
