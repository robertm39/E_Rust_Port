<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / eprover

## Source Files

- [PROVER/eprover.c](../../../eprover/PROVER/eprover.c)

## Purpose

Main program for the E equational theorem prover. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `NAME`

### Globals

- None found in the source scan.

### Exported Functions

- `CLState_p process_options(int argc, char* argv[])`
- `PERF_CTR_DEFINE(SatTimer)`
- `void print_help(FILE* out)`

## Implementation Notes

### Internal Functions

- `print_info`
- `print_proof_stats`

### Source-Level Behavior

- `set_limits`: Sets time and memory limits.
- `parse_spec`: Allocate proof state, parse input files into it, and check that requested properties are met. Factored out of main for reasons of readability and length.
- `print_info`: Check if pid and version should be printed, if yes, do so.
- `strategy_io`: Write and/or read the search strategy parameters. Moved here to declutter main.
- `handle_auto_mode_preproc`: Handle (raw) classification and preprocessing scheduling for auto-mode and auto-schedule mode. Moved here to declutter main().
- `print_proof_stats`: Print some statistics about the proof search. This is a pure service function to make main() smaller.
- `main`: Main entry point of the prover. This is where all the cruft accumulates - sorry!
- `check_fp_index_arg`: Check in arg is a valid term describing a FP-index function. If yes, return true. If no, print error (nominally return false).
- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.

### Dependencies

- `<ccl_formulafunc.h>`
- `<ccl_relevance.h>`
- `<ccl_unfold_defs.h>`
- `<cco_ho_inferences.h>`
- `<cco_preprocessing.h>`
- `<cco_proofproc.h>`
- `<cco_scheduling.h>`
- `<cco_sine.h>`
- `<che_new_autoschedule.h>`
- `<cio_commandline.h>`
- `<cio_output.h>`
- `<cio_signals.h>`
- `<clb_defines.h>`
- `<clb_regmem.h>`
- `<cte_lambda.h>`
- `<cte_simpletypes.h>`
- `<e_options.h>`
- `<e_version.h>`
- `<sys/mman.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
- `FAST_EXIT`
- `FULL_MEM_STATS`
- `GlobalOut`
- `MEASURE_UNIFICATION`
- `NDEBUG`
- `PDT_COUNT_NODES`
- `PRINT_INDEX_STATS`
- `PRINT_SOMEERRORS_STDOUT`
- `STACK_SIZE`

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

Source files reviewed: `PROVER/eprover.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 2242 lines, 3 scanned public declarations, 2 scanned internal function definitions, and 9 structured function-comment blocks.
- Primary executable. Option processing, input parsing, scheduling, proof-state setup, and output mode selection define drop-in compatibility.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Change-Later Observations

- `eprover.c` handles syntax-only parsing through the same proof-state scanner/parser setup used by ordinary input processing. The Rust executable now has a parse-only path for the currently supported clause syntax, but it should be rejoined with the full proof-state/formula parser once `WFormula`/formula-list parsing and the remaining scanner owners are ported.
- `CreateScanner(StreamTypeFile, "-", ...)` keeps stdin as a real scanner stream with file-typed source identity. Rust syntax-only stdin now keeps the source name `-` and file-shaped diagnostics, but it still reads stdin into memory before scanner construction; revisit streaming behavior once large-input and include compatibility are tested.
- `--lpo-recursion-limit` updates the process-wide LPO recursion limit and emits its large-value warning immediately during option parsing, before the missing `break` falls through into `--restrict-literal-comparisons`. Rust now applies the global limit and emits the warning for valid runs, but exact warning/error ordering for later invalid options should be handled with the final executable diagnostic layer.
- `eprover.c` mutates the global `h_parms` and `fvi_parms` structures directly throughout option parsing and later passes those objects into `ProofControlInit`. Rust keeps parsed executable state in typed config structs first and now has explicit conversions into `HeuristicParmsCell`, `FVIndexParms`-shaped data, and an initial `ProofControl`; keep the adapter boundary visible until proof-control ownership is complete, then decide whether the C global mutation model should remain only as a compatibility shim.
- `--cnf` is implemented by setting `outdesc = "teigEIG"`, enabling saturated output, setting `proc_limit = 0`, and short-circuiting final status to CNF success. Rust preserves that option-state and early-output behavior for supported clause-list input; full formula clausification still needs the formula/preprocessing pipeline.
- `strategy_io` applies a parsed strategy file first, then a named predefined strategy, and finally handles `--print-strategy` by printing all predefined strategies, all predefined names, the current parameter cell, or a named predefined cell before exiting. Rust now bridges those option effects for current/named/all strategy printing and proof-control parameter setup; exact C timing after parsing/preprocessing should be revisited once the full formula and scheduling pipeline is available.
- `--prune` exits after parsing plus SInE/relevance preprocessing with a pruning-success banner and `Unknown` SZS status, before clausification or proof search. Rust now preserves that control-flow exit for supported clause-list input after parsing; actual SInE/relevance pruning still awaits the full formula/preprocessing path.
- After saturation, C prints an unconditional result banner before optional saturated-state/statistics output: proof found, no proof found, restricted-calculus closure, incomplete out-of-unprocessed, watchlist empty, or user resource limit exceeded. With the reference `PRINT_TSTP_STATUS` setting, it also prints `# SZS status ...` immediately after the banner. Rust now emits those banners and status lines for the supported clause-list proof-search/CNF outcomes; proof-object/derivation output remains pending because it depends on full derivation ownership.
- `--filter-saturated` computes the pre-filter `out_of_clauses` flag, then mutates the unprocessed set and can still replace the final proof result when filtering extracts an empty clause. Rust preserves the configured empty-clause promotion for the supported clause-list path, while the CLI still preserves C's descriptor-validator mismatch; exact extraction-root/proof-object side effects should be revisited when derivation ownership is complete.
- C's final completeness gate combines process-global assumptions, proof-state completeness, selected-calculus checks, and `SigHasUnimplementedInterpretedSymbols`. Rust now covers those selected-calculus, proof-state-completeness, and signature-level unimplemented-interpreted-symbol branches for the supported clause-list path.
- In syntax-only mode, C prints formulas directly when `--print-formulas` is set, but otherwise emits a parsing-success banner plus `Unknown` SZS status. Rust now mirrors that success output for the supported clause-list syntax-only path; full formula-set pretty printing remains tied to the later formula parser.
- C `main` runs input parsing, optional preprocessing/scheduling, watchlist loading, proof-control initialization, saturation, and all selected proof/statistics output as one long stateful pipeline. Rust now wires the supported first-order clause-list subset through `ProofState`, configured watchlist-file loading/inline activation, `ProofControlInit`, `ProofStateInit`, `Saturate`, descriptor-selected saturated-state output, and maintained proof-state statistics output, but it deliberately leaves formula preprocessing, scheduling, inline watchlist clauses from the full input parser, proof-object banners, full `GlobalOut` routing, subsystem-global statistics counters, and state-owned global indexes outside this bridge until those owners are complete.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
