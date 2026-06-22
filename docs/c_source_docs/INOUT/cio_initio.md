<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_initio

## Source Files

- [INOUT/cio_initio.h](../../../eprover/INOUT/cio_initio.h)
- [INOUT/cio_initio.c](../../../eprover/INOUT/cio_initio.c)

## Purpose

Rather trivial code for initializing all I/O related stuff once and in one go. the GNU Lesser General Public License. <1> Thu Mar 17 11:20:32 UYT 2005

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CIO_INITIO`

### Globals

- `extern char* TPTP_dir`

### Exported Functions

- `void ExitIO(void)`
- `void InitIO(char* progname)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `InitIO`: Initialize I/O. Bundles a number of other initializations in one call.
- `ExitIO`: Clear up (variables)

### Dependencies

- `"cio_initio.h"`
- `<cio_output.h>`

### Compile-Time Conditions

- `CIO_INITIO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_initio.h`, `INOUT/cio_initio.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 174 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 2 structured function-comment blocks.
- Rather trivial code for initializing all I/O related stuff once and in one go. the GNU Lesser General Public License. <1> Thu Mar 17 11:20:32 UYT 2005
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
