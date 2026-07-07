<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# SIMPLE_APPS / term2dag

## Source Files

- [SIMPLE_APPS/term2dag.c](../../../eprover/SIMPLE_APPS/term2dag.c)

## Purpose

Main program for a simple CLIB application: Read term set, write equivalent DAG. the GNU Lesser General Public License.

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

- `<cio_basicparser.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cte_termbanks.h>`
- `<stdio.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `SIMPLE_APPS/term2dag.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `SIMPLE_APPS` covering 1 source file(s), about 199 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Main program for a simple CLIB application: Read term set, write equivalent DAG. the GNU Lesser General Public License.
- Small application code. Useful as integration examples for command-line and term/formula APIs.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `src/simple_apps/term2dag.rs` and the `term2dag` Cargo binary now port the standalone wrapper: C-shaped help/verbosity/output/print-reference option parsing, default stdin input through `-`, sequential term parsing through one shared term bank with checked `TBTermParse`-style distinct-number/object argument-list rejection, `TPTopPos` marking, signature printing including C's stdout side channel for per-symbol newlines and missing-type markers when `-o` selects a file, entry-number-ordered DAG output with forced internal property comments, C-shaped two-line input/output file-open diagnostics, and `OutClose`-style final flush diagnostics.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `process_options()` accepts `-r`/`--print-reference-number` and mutates `TBPrintInternalInfo`, but `main()` unconditionally sets `TBPrintInternalInfo = true` after options have been processed. The Rust wrapper validates the option and keeps comments forced on for compatibility; after drop-in compatibility is secured, decide whether this option should be honored or removed.
- `term2dag` is a small DAG-dump utility, but C routes input through full `TBTermParse` rather than the looser `TBTermParseSimple`, so it inherits distinct-number/object argument-list diagnostics and the broader term-bank parser surface. Rust mirrors that for drop-in behavior; a cleaned CLI could make strict parser mode explicit.
- `TBPrintBankInOrder` builds a temporary numeric tree solely to sort term-bank cells by `entry_no`. The Rust port sorts the collected shared terms directly; if profiling ever identifies this path as hot, keep the observable order while choosing the simpler allocation strategy.
- `SigPrint` inherits `sig_print_operator`'s mixed-stream behavior: the signature line prefix and type text go to `out`, but the per-symbol newline and `(no type)` marker go to stdout. Rust preserves this only at the `term2dag` executable boundary; a cleaned signature-printing API should keep one destination unless compatibility mode asks for the split.
<!-- END MANUAL REVIEW: c_source_docs -->
