<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / enormalizer

## Source Files

- [PROVER/enormalizer.c](../../../eprover/PROVER/enormalizer.c)

## Purpose

Read a set of unit clauses (and/or formulas) and a set of terms/clauses/formulas. The unit clauses/formulas are interpreted as rewrite rules. The terms are normalized using these rewrite rules. If the rule system is not confluent, the results are deterministic but unspecified. If the rule system is not terminating, rewriting might get stuck into an infinite loop.

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

- `build_rw_system`: Extract all positive unit clauses from spec, mark them as oriented in the natural direction (left to right), and insert them into demods. Free all other clauses and print a warning.
- `process_terms`: Open infile, read terms, compute and print their normal forms.
- `process_clauses`: Open infile, read clauses, and compute and print their normal forms.
- `process_formulas`: Open infile, read formulas, and compute and print their normal forms.
- `main`: Entry point of the program and driver of the processing.
- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_rewrite.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cio_signals.h>`
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
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/enormalizer.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 777 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Read a set of unit clauses (and/or formulas) and a set of terms/clauses/formulas. The unit clauses/formulas are interpreted as rewrite rules. The terms are normalized using these rewrite rules. If the rule system is not confluent, the results are deterministic but unspecified. If the rule system is not terminating, rewriting might get st...
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
