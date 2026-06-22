<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_output

## Source Files

- [INOUT/cio_output.h](../../../eprover/INOUT/cio_output.h)
- [INOUT/cio_output.c](../../../eprover/INOUT/cio_output.c)

## Purpose

Simple functions for secure opening of output files with - convention and error checking. Much simpler than the input, because much less can go wrong with output... the GNU Lesser General Public License.

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CIO_OUTPUT`
- `InitOutput()`
- `OUTPRINT(level, message)`

### Globals

- `extern FILE* GlobalOut`
- `extern int GlobalOutFD`
- `extern long OutputLevel`

### Exported Functions

- `FILE* OutOpen(char* name)`
- `void OpenGlobalOut(char* outname)`
- `void OutClose(FILE* file)`
- `void PrintDashedStatuses(FILE* out, char *stat1, char *stat2, char *fallback)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `OpenGlobalOut`: Set GobalOut to a FILE* connected to file outname, set GlobalOutFD accordingly.
- `OutOpen`: Open a file for writing and return it, with error checking. "-" and NULL are both taken to mean stdout.
- `OutClose`: Close the file, checking for errors. If stdout, just flush it. Error messages are bound to be short, but errors should only result from program errors or extremely obscure circumstances.
- `PrintDashedStatuses`: This is a weird simple thing needed far above. If stat1 and stat2 are NULL, print fallback. If either is non-NULL, print it. If both are non-NULL, print them both separated by a dash.

### Dependencies

- `<cio_output.h>`
- `<clb_dstrings.h>`
- `<stdio.h>`

### Compile-Time Conditions

- `CIO_OUTPUT`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_output.h`, `INOUT/cio_output.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 260 lines, 7 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Output-format selection and printing helpers; TSTP/PCL compatibility depends on small formatting details.
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
