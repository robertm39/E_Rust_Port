<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_verbose

## Source Files

- [BASICS/clb_verbose.h](../../../eprover/BASICS/clb_verbose.h)
- [BASICS/clb_verbose.c](../../../eprover/BASICS/clb_verbose.c)

## Purpose

Declarations for the Verbose variable and macros for verbose reporting on certain operations. the GNU Lesser General Public License. <1> Mon Sep 15 14:41:33 MET DST 1997

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLB_VERBOSE`
- `VERBOSE(arg)`
- `VERBOSE10(arg)`
- `VERBOSE2(arg)`
- `VERBOUT(arg)`
- `VERBOUT10(arg)`
- `VERBOUT2(arg)`
- `VERBOUTARG(arg1,arg2)`
- `VERBOUTARG2(arg1,arg2)`

### Globals

- `extern int Verbose`

### Exported Functions

- None found in the source scan.

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `"clb_verbose.h"`
- `<clb_error.h>`
- `<clb_simple_stuff.h>`

### Compile-Time Conditions

- `CLB_VERBOSE`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_verbose.h`, `BASICS/clb_verbose.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 126 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- Declarations for the Verbose variable and macros for verbose reporting on certain operations. the GNU Lesser General Public License. <1> Mon Sep 15 14:41:33 MET DST 1997
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `Verbose` is a process-global integer gate. `VERBOSE` accepts any nonzero value, while `VERBOSE2` and `VERBOSE10` require levels `>= 2` and `>= 10`.
- `VERBOUT*`/`VERBOUTARG*` close over the global `ProgName` and `stderr`, prepend `<ProgName>: `, and flush the output stream after writing.

### Rust Port Status Notes

- `src/basics/verbose.rs` ports the global verbosity level, threshold helpers, macro-shaped closure gates, exact `VERBOUT*`/`VERBOUTARG*` message formatting, writer-injected helpers for tests, and global-`ProgName` stderr-backed wrappers for direct C macro call-site ports.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `Verbose` is a process-global `int`, and the `VERBOSE*`/`VERBOUT*` macros close over that global plus `ProgName` and `stderr`. Rust preserves the threshold, formatting, global-name, and stderr behavior for compatibility, but future Rust-only call paths should prefer explicit per-run verbosity configuration and writer injection instead of hidden global output.
<!-- END MANUAL REVIEW: c_source_docs -->
