<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / termprops

## Source Files

- [PROVER/termprops.c](../../../eprover/PROVER/termprops.c)

## Purpose

Read a set of terms and print term, number of symbols and depth for each term the GNU Lesser General Public License. <1> Fri Nov 28 00:27:40 MET 1997

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OptionCodes`

### Macros And Constants

- None found in the source scan.

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

Source files reviewed: `PROVER/termprops.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 229 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Read a set of terms and print term, number of symbols and depth for each term the GNU Lesser General Public License. <1> Fri Nov 28 00:27:40 MET 1997
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/prover/termprops.rs` and `src/bin/termprops.rs` port the standalone `termprops` executable over the existing Rust term bank, including `-h`/`--help`, `-v`/`--verbose`, `-o`/`--output-file`, default stdin input through `-`, sequential file processing through one shared term bank, per-term simple printing, C `TermWeight(term,1,1)`-style size, `TermDepth`-style depth, pointer-identity symmetry detection for binary terms, and the final count/average/max summary line.

### Change Later

- The C `com` flag checks `term->args[0]->arity == 1` and then reads `term->args[0]->args[1]`, which is past the unary child argument list. Rust treats that missing second nested argument as `false` instead of reproducing undefined memory access; if reference traces ever show the flag is consumed by users, decide whether the intended test was `args[0]` or an old internal term-layout artifact.
- `termprops` divides by the term count when printing averages, so an empty input can print a platform-shaped NaN. Rust emits `nan` for the zero-count case; byte-compatible empty-input output should be rechecked against a built C executable before treating the spelling as final.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
