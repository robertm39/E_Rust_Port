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
- `src/prover/enormalizer.rs` and `src/bin/enormalizer.rs` port the standalone executable wrapper. The Rust path preserves the long-only `--version`, default stdin rule file, LOP/TPTP/TSTP parse/print aliases, represented formula-owner preprocessing/CNF for rule files, left-to-right demodulator orientation, non-rule warnings, term/clause/formula normalization including old-TPTP `input_formula` targets, output-file routing including `-o -`, two-line `SysError`-style scanner/output open diagnostics, C `OutClose` wording on final flush failure, resource-limit parsing, and resource-usage printing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `--print-statistics`, `print_result`, `app_encode`, `give_up`, `initial_literals`, and `initial_clauses` are parsed or initialized but not used by `enormalizer.c`. Rust accepts the visible flag as a no-op for compatibility; a cleaned CLI should either remove these surfaces or attach real behavior.
- If no positional rule files are provided, C inserts `"-"` as the rule source. Target options can also use `-`, so some option combinations compete for stdin and depend on read order. A modernized mode should reject ambiguous stdin use.
- `OpenGlobalOut(outname)` runs before the default `-` rule file is inserted and before rule or target scanners are created, so output paths can be created or truncated even if later input opening or parsing fails. Rust preserves this order; a cleanup mode could stage output before replacing the destination.
- `build_rw_system()` treats any positive unit as a demodulator and force-sets `EPIsOriented` in the natural left-to-right direction, independent of ordering. Keep this for compatibility, but a future rewrite-rule API should make trusted orientation explicit.
- Missing `--terms`, `--clauses`, and `--formulas` targets are not an error; the program can parse rules and exit without normalizing anything. A user-facing cleanup should require at least one target outside drop-in compatibility mode.
- The C driver allocates dummy clause/formula/watchlist containers and runs the broad `FormulaAndClauseSetParse`/preprocess/CNF path to extract rewrite rules. Rust now mirrors that path through represented formula-owner preprocessing/CNF for supported inputs; once formula ownership is stable, a direct rewrite-rule loader would make this boundary clearer.
- Old-TPTP `input_formula` equality records supplied as rule files are clausified before `build_rw_system()` extracts positive unit demodulators, so they can turn into ignored non-rule implication clauses rather than trusted rewrite rules. A cleaner rule-loading API should diagnose or document that distinction before CNF.
<!-- END MANUAL REVIEW: c_source_docs -->
