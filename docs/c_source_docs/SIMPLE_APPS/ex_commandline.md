<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# SIMPLE_APPS / ex_commandline

## Source Files

- [SIMPLE_APPS/ex_commandline.c](../../../eprover/SIMPLE_APPS/ex_commandline.c)

## Purpose

Example program for demonstrating the use of the cio_commandline module of CLIB. the GNU Lesser General Public License. <1> Tue Jan 20 00:34:12 MET 1998

Within the source tree, this unit belongs to `SIMPLE_APPS`. Small standalone example or conversion programs built against the E libraries.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OptionCodes`

### Macros And Constants

- `VERSION`

### Globals

- None found in the source scan.

### Exported Functions

- `CLState_p process_options(int argc, char* argv[])`
- `void print_help(FILE* out)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.

### Dependencies

- `<cio_commandline.h>`

### Compile-Time Conditions

- None found in the source scan.

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `SIMPLE_APPS/ex_commandline.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `SIMPLE_APPS` covering 1 source file(s), about 173 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Example program for demonstrating the use of the cio_commandline module of CLIB. the GNU Lesser General Public License. <1> Tue Jan 20 00:34:12 MET 1998
- Small application code. Useful as integration examples for command-line and term/formula APIs.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `src/simple_apps/ex_commandline.rs` and the `ex_commandline` Cargo binary now port the standalone CLIB command-line demo: exact C-shaped help text through the shared option renderer, required integer and optional floating-point example options, underscore long-option names, default stdin marker insertion, remaining-argument printing, option-output ordering, exact stable parser diagnostics, and C `SysError`-shaped two-line integer/float range diagnostics using the active CRT error string. The comparison matrix covers unknown/missing/malformed/range failures and canonicalizes only the CRT-owned `ERANGE` suffix; evidence is recorded in [`experiments/2026-07-16-038-ex-commandline-diagnostics/FINDINGS.md`](../../../experiments/2026-07-16-038-ex-commandline-diagnostics/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- This file is a parser demonstration, not a prover feature. Keep the executable visible while pursuing drop-in source-tree coverage, but decide later whether release packaging should install example binaries.
- The C option table uses underscore long-option names and gives `--int_example` a default string even though the option requires an argument. Rust preserves both for compatibility; a cleaned example would likely use hyphenated names and remove the unused default.
- `print_help(FILE* out)` writes the banner to `out` but calls `PrintOptions(stdout, ...)`, so non-stdout help streams would still receive the option table on stdout. Rust's executable path matches the observed `main` behavior; keep this split visible if a stream-parametric helper is added later.
<!-- END MANUAL REVIEW: c_source_docs -->
