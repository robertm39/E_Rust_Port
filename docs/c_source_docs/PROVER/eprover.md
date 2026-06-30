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

- `eprover.c` handles syntax-only parsing through the same proof-state scanner/parser setup used by ordinary input processing. The Rust executable now has a parse-only path for the currently supported clause syntax and applies the configured `--free-numbers`/`--free-objects` distinct-symbol mask there, but it should be rejoined with the full proof-state/formula parser once `WFormula`/formula-list parsing and the remaining scanner owners are ported.
- C handles TSTP include files through scanner/parser state that can collect selector names, consult a `skip_includes` tree, and fall back through the process-global `TPTP_dir` initialized by `InitIO`. In this checkout, the visible `eprover` path threads that tree into nested include parsing but does not appear to populate it, so repeated-include suppression should be made explicit later rather than assumed from the parameter. Rust now initializes the C-shaped I/O state before executable parsing and handles TSTP `include(...)` entries with `TPTP` fallback, name selectors, and missing-selector diagnostics in the supported `cnf(...)`/temporary-`fof(...)` bridge; revisit nested selector semantics, repeated-include policy, and whether `TPTP_dir` remains process-global when the full parser/proof-state input pipeline replaces the temporary bridge.
- C routes old TPTP `input_formula(...)` and TSTP `fof(...)`/`tff(...)`/`tcf(...)` input through formula owners and clausification before proof-state initialization. Rust's temporary executable bridge now also preserves old-TPTP formula role mapping, including C's default treatment of `lemma` and `unknown` as axioms, preserves supported `question` role output in syntax-only lowered-clause printing, annotates supported leading-existential `question` formulas with `$answer(esk(...))` literals before conjecture negation during proof/CNF/prune parsing, emits a C-style unsupported-HOL diagnostic for `thf(...)` until the higher-order formula pipeline is ported, converts first-order `tff(..., type, ...)`/`fof(..., type, ...)`/`tcf(..., type, ...)` declarations into signature mutations without C's temporary `$true` formula placeholder, preserves C's wrapper-specific role validation by permitting `watchlist` only under `tcf(...)`, rejects TSTP formula entries with free variables using C's diagnostic, routes first-order `tff(...)` and `tcf(...)` formula entries through the supported `fof(...)` subset, typed TFF/TCF quantified variable declarations into banked variables with C-style scoped name shadow/restore behavior, `$true`/`$false` formula constants into tautological or false-clause results, `$distinct(...)` formula pseudo-terms over same-type constants into pairwise disequality clauses, grouped or unparenthesized non-conjecture conjunction/disjunction fragments with supported connective precedence including supported existential conjuncts or disjuncts, conjunctive disjuncts containing supported existential conjuncts, grouped or unparenthesized conjecture conjunctions with supported existential conjuncts, implication antecedents/consequents over supported fragments including single supported existential operands, equivalence and `<~>` XOR left/right sides with single supported existential operands, `~&` NAND and `~|` NOR left/right sides with single supported existential operands including negated XOR/NAND/NOR polarity, grouped or unparenthesized conjecture conjunctions/disjunctions over supported fragments, simple universal scopes around supported atoms, direct positive atomic existential atoms plus supported unparenthesized unitary and parenthesized existential bodies including positive or negative quantified operands with banked Skolem terms over occurring variables from an active-universal dependency stack (TSTP rejects globally free variables first, while old TPTP bridge entries still seed that stack with globally free formula variables), supported negated universal scopes containing supported existential fragments, supported positive implication/equivalence consequents that are existential fragments, supported existential implication/equivalence antecedents, existential conjecture conjuncts, and negative-polarity NAND/NOR operands as universal negative literals, supported negated universal conjecture scopes, supported truth-constant, `$distinct`, unitary, or parenthesized existential conjecture atoms after conjecture negation, and their simple parenthesized negations directly into clauses with limited CNF distribution; keep that as a narrow compatibility bridge only, because general NNF conversion, distributive clausification, dependency-aware Skolemization, definitions, typed formula placeholders, formula-level `$distinct` expansion, formula-owner scoped variable handling, TCF-specific formula ownership, proof-documentation steps, and the exact `WFormulaAnnotateQuestion` derivation trail belong in the full formula/CNF pipeline. The mutable C variable-bank environment stack is now mirrored only for the supported bridge surface and should eventually become an explicit scoped binding object instead of parser-global state.
- The temporary bridge now also accepts top-level Boolean `$ite(...)` and `$let(...)` atoms by preserving the already typed Boolean term through predicate-literal preparation. This mirrors the C parser's literal-encoding path for the supported surface, but should become an explicit formula-node-to-literal lowering step when `WFormula` ownership replaces the bridge.
- `CreateScanner(StreamTypeFile, "-", ...)` keeps stdin as a real scanner stream with file-typed source identity. Rust syntax-only stdin now keeps the source name `-` and file-shaped diagnostics, but it still reads stdin into memory before scanner construction; revisit streaming behavior once large-input and include compatibility are tested.
- `--lpo-recursion-limit` updates the process-wide LPO recursion limit and emits its large-value warning immediately during option parsing, before the missing `break` falls through into `--restrict-literal-comparisons`. Rust now applies the global limit and emits the warning for valid runs, but exact warning/error ordering for later invalid options should be handled with the final executable diagnostic layer.
- `--term-ordering=RPO` is accepted by the C executable option handler even though the generic ordering dispatch later asserts that RPO is not implemented. Rust now preserves the accepted CLI surface and materializes `RPO` in the C-shaped ordering parameter cell; a future user-facing mode could reject it earlier with a clearer diagnostic only after compatibility tests decide that the C late-failure behavior can be relaxed.
- The C option table in `e_options.h` is the authoritative option-name and argument-metadata surface, but its help prose includes stale or misspelled text and the executable switch does not handle every advertised entry, notably `--fp-no-size-constr`. Rust now keeps regression tests for long-option, short-alias, argument-kind, and default-argument coverage while using shorter corrected descriptions; a future help-compatibility pass should decide whether user-facing help should be byte-compatible with C or intentionally cleaned up.
- `eprover.c` mutates the global `h_parms` and `fvi_parms` structures directly throughout option parsing and later passes those objects into `ProofControlInit`. Rust keeps parsed executable state in typed config structs first and now has explicit conversions into `HeuristicParmsCell`, `FVIndexParms`-shaped data, and an initial `ProofControl`; keep the adapter boundary visible until proof-control ownership is complete, then decide whether the C global mutation model should remain only as a compatibility shim.
- `--cnf` is implemented by setting `outdesc = "teigEIG"`, enabling saturated output, setting `proc_limit = 0`, and short-circuiting final status to CNF success. Rust preserves that option-state and early-output behavior for supported clause-list input and supported temporary formula-bridge input after lowering; full formula clausification still needs the formula/preprocessing pipeline.
- `strategy_io` applies a parsed strategy file first, then a named predefined strategy, and finally handles `--print-strategy` by printing all predefined strategies, all predefined names, the current parameter cell, or a named predefined cell before exiting. Rust now bridges those option effects for current/named/all strategy printing and proof-control parameter setup; exact C timing after parsing/preprocessing should be revisited once the full formula and scheduling pipeline is available.
- `handle_auto_modes_preproc` computes raw features before SInE/relevance processing, selects the first generated preprocessing schedule cell for plain `--auto`, prints the preprocessing class and configuration through stdout, then reprocesses the original option state so explicit CLI options override the generated strategy. After clause/formula preprocessing, `main` computes the search class with `DEFAULT_MASK` (`aaaaa-aaaaaa-aaaaaaaaa`, including the hyphen separator positions), selects the first generated search schedule cell for plain `--auto` unless `--cnf` is active, restores `inst_choice_max_depth`, and replays CLI options again before `strategy_io`. Rust now mirrors this for supported first-order clause owners and generated `schedule.vars` strategies, using explicit option provenance to overlay user-specified heuristic/order fields over the generated parameter cell and preserving the hyphenated search-class output. Full formula-aware classification and real preprocessing are still pending; a future cleanup should replace C's global reparse/replay pattern with an explicit layered configuration object once compatibility tests cover option precedence.
- After parsing and the unconditional preprocessing-configuration stdout line, `main` calls `ProofStateSinE` before relevance pruning. Rust now emits the C no-filter report for supported first-order `--sine=Auto`/generated-auto cases where the C raw-class lookup returns no filter, but destructive SInE pruning and the named-filter report/effects remain pending until full formula/clause SInE ownership is wired.
- `--prune` exits after parsing plus SInE/relevance preprocessing with a pruning-success banner and `Unknown` SZS status, before clausification or proof search. Rust now preserves that control-flow exit for supported clause-list input and supported temporary formula-bridge input after lowering, and applies the available clause-side relevance pruning for `--rel-pruning-level`; full SInE, formula relevance, and formula/preprocessing integration still await the full formula owner path.
- `parse_spec` couples input format auto-detection to later output policy: after `ScannerSetFormat(AutoFormat)`, auto-detected TSTP input mutates the global `OutputFormat` to TSTP and sets `DocOutputFormat` to TSTP only when documentation format was still unset; any remaining unset documentation format then defaults to PCL. This is order-sensitive across input files and is a reasonable future cleanup target once compatibility tests pin down the user-visible cases. Rust preserves the behavior for the supported clause-list paths by mutating per-run executable config instead of process globals.
- After `parse_spec` and any auto-mode preprocessing, `eprover.c` has a commented-out `#ifndef NDEBUG` guard around an unconditional `fprintf(stdout, COMCHAR" (lift_lambdas = ..., sine = ...)\n", ...)`. It bypasses `GlobalOut`, so it still writes to stdout when proof output is redirected with `--output-file`; on the reference glibc build, an unset `h_parms->sine` prints as `(null)`. Rust preserves this stdout side channel for supported non-syntax executable paths using the reference `%` comment prefix, but it is a clear cleanup candidate once output compatibility is locked down.
- `--app-encode` branches after parsing, SInE, and relevance pruning but before initial documentation, conjecture negation, pruning output, or proof search. It calls `FormulaSetAppEncode(stdout, proofstate->f_axioms)`, so formula output bypasses `GlobalOut`; during parsing, `FormulaAndClauseSetParse` also ignores `include(...)` entries when the global `app_encode` flag is set. Rust now preserves the print-and-exit mode for the supported main-file TPTP/TSTP formula bridge, including stdout-side output, ignored includes, parsed-but-unprinted `cnf(...)`/`input_clause(...)` entries that still mutate the shared parser signature and count for `--error-on-empty`, top-level `$true` omission, supported binary connective spelling preservation, and app-encoded type/symbol declarations. Full formula SInE/relevance effects, clause-backed wrapped formulas, exact `WFormula` ownership, and a cleaner declaration policy for symbols introduced only by skipped clauses remain pending; later cleanup should decide whether include ignoring, output bypass, and spelling-preserving rendering stay only in a compatibility layer.
- After saturation, C prints an unconditional result banner before optional proof-object/saturated-state/statistics output: proof found, no proof found, restricted-calculus closure, incomplete out-of-unprocessed, watchlist empty, or user resource limit exceeded. With the reference `PRINT_TSTP_STATUS` setting, it also prints `COMCHAR " SZS status ..."` immediately after the banner. Rust now emits those banners and status lines for the supported clause-list proof-search/CNF outcomes and has represented clause-side proof-object list/graph output for supported roots; full ordered proof-object extraction and formula-aware derivation output remain pending because they depend on full derivation ownership.
- For proof-found exits after formula conjecture preprocessing, C chooses `Theorem` versus `ContradictoryAxioms` by traversing the final derivation and checking whether a conjecture-type formula or clause appears in the proof tree. Rust now mirrors this for supported clause-side proofs by scanning direct compact clause parents and rewrite-demodulator references from the returned proof clause, but the C coupling between status reporting and full proof-object reconstruction should be reconsidered once formula parents, AC auxiliary parents, and stable derivation owners are fully ported.
- For saturated proof searches after formula conjecture preprocessing, C reports `CounterSatisfiable` instead of `Satisfiable`. Rust now mirrors that status split for supported `fof(...)` conjectures while preserving `Satisfiable` for explicit `negated_conjecture` inputs that were not created by conjecture negation.
- `--filter-saturated` computes the pre-filter `out_of_clauses` flag, then mutates the unprocessed set and can still replace the final proof result when filtering extracts an empty clause. Rust preserves the configured empty-clause promotion for the supported clause-list path, while the CLI still preserves C's descriptor-validator mismatch; exact extraction-root/proof-object side effects should be revisited when derivation ownership is complete.
- In the final `print_sat` block, C prints `COMCHAR " Saturated system contains the empty clause:"` whenever the `success` pointer is non-null, then prints that clause through `ClausePrint` before the descriptor-selected proof-state sections; `--print-sat-info` only toggles the `outinfo` argument passed to `ProofStatePrintSelective`, so printed saturated clauses receive same-line `ClauseInfoPrint` comments. Rust mirrors this for returned proof clauses in the supported clause-list path, including the selected output-format dispatch and represented saturated-clause `info(...)` comments; if future answer/proof-success clauses can be non-empty in this block, the message wording should be reconsidered only behind compatibility tests.
- On proof-found exits at output level 2 or higher, C calls `DocClauseQuoteDefault(2, success, "proof")` before the `COMCHAR " Proof found!"` banner. Rust mirrors the supported PCL/TSTP executable output with a cloned clause; broader proof-object work still needs a single decision on whether C's global documentation id stream is preserved exactly or represented as run-owned state.
- C prints formula/clause initial documentation before `ProofStateLoadWatchlist`, then the watchlist loader prints initial documentation for active watchlist clauses after marking them as watch-only. Rust preserves that ordering and documentation-id sequence for supported clause-list proof-search runs.
- On non-proof exits at output level 2 or higher, C quotes final proof-state clauses before the result banner: watchlist-empty exits use `CPSubsumesWatch` with comment `final_subsumes_wl`, while other stopped exits use `CPIgnoreProps` with comment `exists` unless the unprocessed set is empty and both the proof state and inference system are complete, in which case the comment is `final`. Rust mirrors those supported PCL/TSTP final quotes with cloned clauses for clause-list proof-search runs.
- When `PrintProofObject` is enabled for proof-found exits, C prints `DerivationPrintConditional(..., "CNFRefutation", ...)` after the proof banner and SZS status. For complete saturated no-proof exits, it instead sets `sat_status = "Saturation"`, pushes processed clause sets as extraction roots, and prints `DerivationComputeAndPrint(..., sat_status, ...)`; resource-out and incomplete exits do not get that block unless forced derivation output is active. `--proof-object=0` is accepted but leaves `PrintProofObject` false when no earlier proof-output option enabled it, even though the handler still raises the internal derivation-output selector to list mode. Rust now preserves the no-output level-0 surface, emits the `SZS output start/end CNFRefutation` framing plus represented ancestor/root list steps for supported clause-list proofs using display-only sequential ids, applies represented `--full-deriv` roots to proof-found list/DOT/statistics output, emits a conservative `Saturation` block for complete no-proof clause-list exits when proof-object output is enabled, renders represented stopped/list roots and reachable ancestors with C-shaped PCL/TSTP derivation or source-info payloads plus final/proof root markers only on extraction roots, emits represented `Derivation` proof-object blocks/statistics for forced incomplete/resource clause-list exits, adds unprocessed roots for `--force-deriv=2`, suppresses later saturated-state printing at force level 2, emits represented clause-side DOT for supported `--proof-graph` proof-found and stopped roots, includes C-shaped TSTP clause/derivation/source-info labels for represented graph levels above 1, applies represented clause-root marking before GC analysis/training examples for supported proof-found runs, and prints represented clause-side proof-object statistics for supported proof-found, complete-saturation, and forced stopped roots when `--proof-statistics` is combined with an enabled proof object. Exact ordered derivation expansion, extraction-root selection beyond the represented clause roots, exact C proof-object renumbering, formula-archive proof roots, formula-aware graph labels, AC auxiliary proof-stat/graph roots and graph-label parents, and pointer-stable proof identity remain pending; a later cleanup should keep C's root-selection policy separate from the core proof-state API.
- C's final completeness gate combines process-global assumptions, proof-state completeness, selected-calculus checks, and `SigHasUnimplementedInterpretedSymbols`. Rust now covers those selected-calculus, proof-state-completeness, and signature-level unimplemented-interpreted-symbol branches for the supported clause-list path.
- In syntax-only mode, C prints formulas directly when `--print-formulas` is set, using the selected output format, but otherwise emits a parsing-success banner plus `Unknown` SZS status. Rust now mirrors that success output for the supported clause-list syntax-only path and distinguishes LOP, old-TPTP, and TSTP clause output there; full formula-set pretty printing remains tied to the later formula parser.
- C executable parsing uses signatures whose internal symbol block has already reserved fixed codes such as `$@_var`, `$named_lam`, `$db_lam`, `$ite`, and `$let` before user symbols are inserted. Rust now applies the same internal-code reservation to the temporary syntax-only and app-encode parser banks, not just to proof-state allocation, so ordinary user predicates cannot collide with the phony-application code. This should become an explicit parser-bank constructor once the temporary executable bridges are replaced by the full proof-state/formula input owner.
- `--print-types` mutates the process-global `TermPrintTypes` flag during option parsing, so later term printing implicitly appends type suffixes in all full-term output paths that use `TermPrint`. Rust records this as executable configuration and now threads it explicitly through supported `--print-formulas` and saturated clause output; future proof/formula printers should keep the explicit option boundary unless a compatibility layer needs the C global.
- At normal cleanup, C prints `PrintRusage(GlobalOut)` when `--resources-info` is set and timeout handling did not suppress it. Rust now emits the same footer shape for successful configured exits; exact timeout suppression and Unix `getrusage` accounting remain tied to the later signal/resource-limit layer.
- During early limit setup, C uses `SetSoftRlimitErr` for CPU limits, warns if core-dump suppression fails, and then applies `SetMemoryLimit`. Rust now routes supported setup warnings through the executable warning stream before the main run, including `strerror(errno)` text for unmasked Linux CPU-limit failures, and preserves C's separate `perror("eprover")` line before the core-dump warning. Host-mutating resource-limit calls remain disabled in tests.
- C `main` runs input parsing, optional preprocessing/scheduling, watchlist loading, proof-control initialization, saturation, and all selected proof/statistics output as one long stateful pipeline. Rust now wires the supported first-order clause-list subset through `ProofState`, configured watchlist-file loading/inline activation with supported TPTP/TSTP inline watchlist clauses preloaded from normal input/includes, `ProofControlInit`, `ProofStateInit`, configured caller-owned global indexes over a cloned initialized signature, indexed `Saturate`, represented proof-object list/DOT/statistics output for supported clause roots, descriptor-selected saturated-state output, and maintained proof-state statistics output, but it deliberately leaves formula preprocessing, scheduling, full ordered proof-object extraction, full `GlobalOut` routing, subsystem-global statistics counters, and state-owned global indexes outside this bridge until those owners are complete. The cloned-signature global-index bridge preserves C's indexed processed-clause insertion/deletion and indexed selected-clause generation order for supported first-order runs; replace it with an explicit proof-session owner before emulating C's process-wide `state->gindices` ownership fully.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
