<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / edpll

## Source Files

- [PROVER/edpll.c](../../../eprover/PROVER/edpll.c)

## Purpose

Read a ground problem and try to refute (or satisfy) it. the GNU Lesser General Public License. <1> Thu May 1 20:40:24 CEST 2003 New

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OptionCodes`

### Macros And Constants

- `NAME`

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
- `<cio_output.h>`
- `<cio_signals.h>`
- `<cpr_dpll.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
- `FAST_EXIT`
- `STACK_SIZE`

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

Source files reviewed: `PROVER/edpll.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 402 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Read a ground problem and try to refute (or satisfy) it. the GNU Lesser General Public License. <1> Thu May 1 20:40:24 CEST 2003 New
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Notes

- `src/prover/edpll.rs` and `src/bin/edpll.rs` port the standalone executable wrapper over the existing Rust `propositional::{dpllformula,dpll}` state-shell modules, including exact C-shaped full help text with the legacy support-tool footer.
- The C executable parses input clauses, prints the `DPLLFormulaParseLOP` `New clause: ...accepted` / `...discarded (tautology)` trace, allocates a `DPLLState`, and exits without calling a solver. The Rust executable intentionally preserves that incomplete behavior.
- `--dimacs` sets C's `dimacs_format` global but no later code reads it; Rust accepts the flag as a parsed no-op and keeps output identical to the default trace.
- `--version` prints `classify_problem VERSION` in C even though the executable name is `edpll`; Rust keeps that visible typo for drop-in CLI compatibility.
- The Rust wrapper preserves default stdin through `-`, output-file routing including `-o -` as stdout, two-line `SysError`-style scanner/output open diagnostics, early output-file creation before later input-open failures, C `OutClose` wording on final flush failure, the C loop's loose treatment of non-clause trailing input, and the historical empty procedural-tail diagnostic text from `ClauseParse`.

### Change Later

- Decide whether `edpll` should become a working standalone DPLL driver or remain a legacy parser/state-construction helper. Completing it will require changing user-visible behavior from the current "Not completed yet!" C path.
- If a completed driver is desired, wire `--dimacs` to actual `DPLLFormulaPrint`/DIMACS output deliberately instead of treating the currently unused flag as a hidden output mode.
- `OpenGlobalOut(outname)` runs before the default `-` input is inserted and before any scanner is created, so output paths can be created or truncated even if later input opening or parsing fails. Rust preserves this order; a cleanup mode could stage output before replacing the destination.
- `DPLLFormulaParseLOP()` stops when `ClauseStartsMaybe()` is false and does not require end-of-file, so trailing non-clause tokens are silently ignored. Rust preserves this parser boundary; strict validation should be a deliberate non-compatibility behavior.
- Resource-limit handling is copied into this small C program even though the current executable does not run a search loop. A cleaned CLI could share one resource-limit owner with `eprover` after compatibility mode is separated from modernized behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
