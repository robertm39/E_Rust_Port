<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_axfilter

## Source Files

- [PROVER/e_axfilter.c](../../../eprover/PROVER/e_axfilter.c)

## Purpose

Parse a problem specification and a filter setup, and produce output the GNU Lesser General Public License. <1> Mon Feb 21 13:24:04 CET 2011 New (but borrowing from LTB runner)

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OptionCodes`
- `SubSampleMethod`

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

- `filter_problem`: Given a structured problem data structure, an axfilter, and the core name of the output, apply the filter to the problem and write the result into a properly named file (which is determined from the core name and the filter name).
- `all_filters_problem`: Apply all filters to problems.
- `find_seed_symbols`: Push all symbols in sig that correspond to the symbol types used for seeding onto result.
- `seeded_filter_all`: Generate seeded axiom selections based on all formulas with symbol seed_symbols (which are already delivered in symb_formulas).
- `seeded_filter_largest`: Generate seeded axiom selections based on the largest formula with symbol seed_symbols (candidates are already delivered in symb_formulas).
- `seeded_filter_diverse`: Generate seeded axiom selections based on the most diverse formula with symbol seed_symbols (candidates are already delivered in symb_formulas).
- `subsample_seed_symbols`: Optionally reduce the set of seed symbols, based on the value of the variables below.
- `decode_seed_symbols`: Parse the symbols from seedstr, find their encoding, and put them onto the provided stack. Terminate with error if there is an unknown symbol.
- `seeded_filters`: Run through all seeds, all seeding methods and generate all filtered files.
- `main`: Main function of the program.
- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_relevance.h>`
- `<ccl_sine.h>`
- `<cco_batch_spec.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cio_signals.h>`
- `<clb_defines.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_axfilter.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 1063 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 11 structured function-comment blocks.
- Parse a problem specification and a filter setup, and produce output the GNU Lesser General Public License. <1> Mon Feb 21 13:24:04 CET 2011 New (but borrowing from LTB runner)
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Notes

- `src/prover/e_axfilter.rs` and `src/bin/e_axfilter.rs` port the standalone executable wrapper for non-seeded and artificial seeded filter generation. The Rust wrapper preserves C-shaped help/version text, output-file creation before filter parsing and missing-problem usage errors, configured-output routing including `-o -`, C-shaped configured-output and filter-file open diagnostics, C `OutClose` wording for configured-output flush failure, filter dumping before the usage check, default/custom ax-filter parsing, Auto/LOP/TPTP/TSTP input-format options, structured problem parsing through the ported batch-spec loader, reset of the shared boundary after distribution initialization, first-input `FileNameStrip` corename derivation, generated `<corename>_<filter>.p` and seeded `<corename>_S[ALD]_<P|F><arity>_<symbol>_<filter>.p` output names, TSTP type-declaration emission, and selected clause/formula stack printing.
- `--seed-symbols`, `--seeds`, `--seed-subsample`, and `--seed-method` are parsed with the C option defaults and validation quirks, including `atol`-style prefix parsing for `--seed-subsample`. Seeded filtering now discovers eligible seed symbols, decodes explicit function-symbol names after problem parsing, preserves duplicate explicit seeds, optionally subsamples with the ported frequency distribution and process-global JKISS helper, temporarily mutates formula roles to hypotheses, emits C-shaped seed descriptors, and applies only hypothesis-aware filters.
- The corresponding status entry lives in [`../../rust-port-status.md`](../../rust-port-status.md) under `e_axfilter Executable`.

### Change Later

- `filter_problem` opens the generated `<corename>_<filter>.p` file with `fopen` and immediately calls `fprintf` on the result without checking for `NULL`. Rust reports a file diagnostic instead of reproducing the crash surface; a cleaned C API should report generated-file failures explicitly.
- `--output-file` affects `GlobalOut` messages and filter dumps only. The actual filtered problems are always written to generated names in the current working directory, and pipe-based output is intentionally unsupported. A modernized CLI should offer an output directory or manifest once drop-in behavior is covered.
- `main()` calls `OpenGlobalOut(outname)` before opening the optional filter file and before the missing-problem usage check, so configured output can be created or truncated even when no reduced problem file will be generated. Rust preserves this side effect for compatibility; a future user-facing mode could delay or atomically commit configured output.
- `process_options` keeps `app_encode` and `OPT_PRINT_STATISTICS`-style dead global/enum surfaces in the surrounding file shape even though this executable does not expose or consume them. Do not add cleaned Rust API surface for these unless another compatibility path proves them observable.
- `seeded_filter_largest` sets `largest` to hypothesis, but its restoration branch calls `FormulaSetType(handle, CPTypeAxiom)`, where `handle` is the last loop element, not necessarily `largest`. `seeded_filter_diverse` has a similar copy/paste hazard in the hypothesis-setting branch. Rust currently preserves these handle/last-candidate effects; a cleaned implementation should restore the actually selected formula after compatibility tests no longer require the bug surface.
- `seeded_filter_all` calls `FormulaStackCondSetType(..., CPTypeAxiom)` after output, which resets every non-conjecture seed candidate to axiom instead of restoring original roles. Rust preserves the behavior; a later API should snapshot and restore original roles if compatibility mode is not needed.
- The seeded `Name: ...` progress line is printed with `printf`, not `GlobalOut`, so it bypasses `--output-file` while filter-progress lines do not. Rust preserves that stdout/file split; a later API should make progress-output routing explicit rather than depending on mixed global/stdout channels.
- `decode_seed_symbols` reports unknown user seed symbols through the fatal usage-error path after the whole problem has been parsed and the signature is populated. A cleaned interface could validate explicit seeds transactionally, but compatibility mode should keep the current late validation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
