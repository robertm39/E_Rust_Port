<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / epatternize

## Source Files

- [PROVER/epatternize.c](../../../eprover/PROVER/epatternize.c)

## Purpose

Read a logic file and convert it to pattern form. the GNU Lesser General Public License.

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

- `CLState_p process_options(int argc, char* argv[], SpecLimits_p limits)`
- `char* parse_feature_line(Scanner_p in, SpecFeature_p features)`
- `void print_help(FILE* out)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `main`: The main function and entry point of the program.
- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.
- `print_help`: Print the help text.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_unfold_defs.h>`
- `<cco_sine.h>`
- `<che_clausesetfeatures.h>`
- `<che_hcb.h>`
- `<che_rawspecfeatures.h>`
- `<che_specsigfeatures.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cle_clauseenc.h>`
- `<cle_patterns.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
- `STACK_SIZE`
- `term`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/epatternize.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 864 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Read a logic file and convert it to pattern form. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
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
