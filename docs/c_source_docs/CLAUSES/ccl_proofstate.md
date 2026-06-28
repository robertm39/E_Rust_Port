<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_proofstate

## Source Files

- [CLAUSES/ccl_proofstate.h](../../../eprover/CLAUSES/ccl_proofstate.h)
- [CLAUSES/ccl_proofstate.c](../../../eprover/CLAUSES/ccl_proofstate.c)

## Purpose

Proof objects describing the state of a proof attempt (i.e. all information relevant to the calculus, but not information describing control). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ProofStateCell`
- `ProofState_p`
- `TrainingSelector`

### Macros And Constants

- `CTO_PROOFSTATE`
- `ProofStateAxNo(state)`
- `ProofStateCardinality(state)`
- `ProofStateCellAlloc()`
- `ProofStateCellFree(junk)`
- `ProofStateProcCardinality(state)`
- `ProofStateStorage(state)`
- `ProofStateUnprocCardinality(state)`
- `WATCHLIST_INLINE_QSTRING`
- `WATCHLIST_INLINE_STRING`

### Globals

- None found in the source scan.

### Exported Functions

- `(ClauseSetStorage((state)->unprocessed)+ \ ClauseSetStorage((state)->processed_pos_rules)+ \ ClauseSetStorage((state)->processed_pos_eqns)+ \ ClauseSetStorage((state)->processed_neg_units)+ \ ClauseSetStorage((state)->processed_non_units)+ \ ClauseSetStorage((state)->archive)+ \ TBStorage((state)->terms)) (ClauseSetCardinality((state)->processed_pos_rules)+ \ ClauseSetCardinality((state)->processed_pos_eqns)+ \ ClauseSetCardinality((state)->processed_neg_units)+ \ ClauseSetCardinality((state)->processed_non_units)) ClauseSetCardinality((state)->unprocessed) (ProofStateProcCardinality(state)+ \ ProofStateUnprocCardinality(state)) bool ProofStateIsUntyped(ProofState_p state)`
- `(ProofStateCell*)SizeMalloc(sizeof(ProofStateCell)) SizeFree(junk, sizeof(ProofStateCell)) ProofState_p ProofStateAlloc(FunctionProperties free_symb_prop)`
- `long ProofStateProcessDistinct(ProofState_p state)`
- `void ProofStateAnalyseGC(ProofState_p state)`
- `void ProofStateFree(ProofState_p junk)`
- `void ProofStateInitWatchlist(ProofState_p state, OCB_p ocb)`
- `void ProofStateLoadWatchlist(ProofState_p state, char* watchlist_filename, IOFormat parse_format)`
- `void ProofStatePickTrainingExamples(ProofState_p state, PStack_p pos_examples, PStack_p neg_examples)`
- `void ProofStatePrint(FILE* out, ProofState_p state)`
- `void ProofStatePropDocQuote(FILE* out, int level, FormulaProperties prop, ProofState_p state, char* comment)`
- `void ProofStateResetClauseSets(ProofState_p state, bool term_gc)`
- `void ProofStateResetSATSolver(ProofState_p state)`
- `void ProofStateStatisticsPrint(FILE* out, ProofState_p state)`
- `void ProofStateTrain(ProofState_p state, bool print_pos, bool print_neg)`

## Implementation Notes

### Internal Functions

- `clause_set_analyse_gc`
- `clause_set_pick_training_examples`

### Source-Level Behavior

- `clause_set_analyse_gc`: Count number of clauses, given clauses, and used given clauses.
- `clause_set_pick_training_examples`: Find given clauses and classify them as positive (used in the proof) and negative (not used) examples. Return the two sets via the result-stacks provided.
- `ProofStateAlloc`: Return an empty, initialized proof state. The argument is: free_symb_prop: Which sub-properties of FPDistinctProp should be ignored (i.e. which classes with distinct object syntax should be treated as plain free symbols). Use FPIgnoreProps for default behaviour, FPDistinctProp for fully free (conventional) semantics.
- `ProofStateLoadWatchlist`: Load the watchlist (if requested and not inline), remove it if not requested.
- `ProofStateInitWatchlist`: Initialize the (preloaded) watchlist.
- `ProofStateResetClauseSets`: Empty _all_ clause and formula sets in proof state. Keep the signature and term bank. If term_gc is true, perform a garbage collection of term cells.
- `ProofStateFree`: Free a ProofStateCell.
- `ProofStateProcessDistinct`: Process $distinct directives in state->f_axioms. Return number of $distincts processed.
- `ProofStateIsUntyped`: Return true if all clauses in the proof state are untyped. Does not check formulas!
- `ProofStateAnalyseGC`: Run an analysis of the use of given clauses in the proof search: How many were used (i.e. useful) and how many were unused (i.e. useless).
- `ProofStatePickTrainingExamples`: Find positive and negative training examples in the proof state.
- `ProofStateTrain`: Perform some (yet to be specified ;-) training on the proof state.
- `ProofStateStatisticsPrint`: Print the statistics of the proof state.
- `ProofStatePrint`: Print the clause sets of the proof state.
- `ProofStatePropDocQuote`: Print all clauses in the main clause sets in state for which props is true (if outputlevel is large enough, as defined in ClauseSetPropDocQuote().

### Dependencies

- `"ccl_proofstate.h"`
- `<ccl_def_handling.h>`
- `<ccl_garbage_coll.h>`
- `<ccl_global_indices.h>`
- `<ccl_rewrite.h>`
- `<cio_output.h>`
- `<picosat.h>`

### Compile-Time Conditions

- `CTO_PROOFSTATE`
- `MEASURE_EXPENSIVE`
- `NEVER_DEFINED`

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

Source files reviewed: `CLAUSES/ccl_proofstate.h`, `CLAUSES/ccl_proofstate.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1056 lines, 17 scanned public declarations, 2 scanned internal function definitions, and 15 structured function-comment blocks.
- Global proof-state assembly point; changes here affect parsing, indexing, preprocessing, saturation, and proof extraction.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Compatibility Notes

- `ProofStateAlloc` constructs the type bank, signature, main term bank, temporary term bank, fresh variable bank, formula sets, clause sets, watchlist, global indices, FV-index metadata, demodulator aliases, definition store, and all counters in one allocation path. The Rust proof-state owner currently ports the internal signature/term-bank setup, a shadow-paired fresh variable bank, clause sets, optional watchlist, definition-store clause set, watchlist load/disable activation, watchlist maximal-term marking and FV-index rebuild, initial-clause watchlist-hit mutation, FV-index specs/anchors, C cardinality macros, untyped check domain, `ProofStatePrint` clause-set output order, zeroed statistics, and clause-side `ProofStateAnalyseGC`/`ProofStatePickTrainingExamples` over the same represented clause-owner domains; formula sets, global indices, demodulator trees, SAT state, and temporary term bank remain later proof-state slices.
- `ProofStateAlloc` applies `signature->distinct_props = signature->distinct_props & (~free_symb_prop)` after internal symbols and term banks are allocated. Rust keeps the same distinct-property mask in the proof-state constructor.
- `ProofStateIsUntyped` checks only processed positive rules, processed positive equations, processed negative units, processed non-units, and unprocessed clauses. It intentionally ignores axioms, watchlist clauses, and formulas.
- `ProofStateStorage` is a macro that sums `ClauseSetStorage` for unprocessed, processed, and archive sets plus `TBStorage(state->terms)`; it intentionally excludes axioms, watchlist, temporary, eval, definition, and formula stores. Rust now exposes a proof-control storage estimate over the same selected domains for cleanup gating, but exact C byte accounting remains tied to allocator and index-memory constants.
- C `fvi_param_init` is reached from proof-control initialization and installs anchors on processed clause sets, the watchlist if present, and `definition_store->def_clauses`. The Rust proof-state shell now owns those targets and installs the same anchor bundle into them.

### Change Later Candidates

- `ProofStateResetClauseSets` says it empties all clause and formula sets, but the implementation frees `f_ax_archive` twice and does not clear `definition_store`. Rust preserves the definition-store omission for now; revisit the reset contract only after definition handling and proof-state reuse callers are ported.
- `ProofStateAlloc` creates `terms` and `tmp_terms` with the same mutable `Sig_p`, and `ProofStateFree` nulls both term-bank signature pointers before freeing the shared signature. Rust `TermBank` currently owns its signature, so the eventual `tmp_terms` port should use an explicit shared proof-session signature handle rather than cloning mutable signatures through proof search.
- `ProofStateLoadWatchlist` uses the global `UseInlinedWatchList` sentinel to decide whether to parse from a filename, while any non-null filename, including the inline sentinel, still marks the watchlist as active. The Rust proof-state and executable bridge now model this as enum state internally; keep that shape for future configuration APIs instead of reintroducing sentinel filenames.
- `ProofStateLoadWatchlist` calls `ClauseSetDocInital` after marking watchlist clauses. Rust ports the parsing, EOF check, TPTP type/property marking, standard weighing, literal sorting, and disable path now, but leaves the output/documentation side effect for the later global-output/proof-documentation integration.
- `ProofStatePrint` prints both `processed_pos_rules` and `processed_pos_eqns` under the single heading `Processed positive unit clauses`. Rust preserves that output shape; a later diagnostic/UI layer can split orientable and unorientable positive units only if compatibility output is kept separate.
- `ProofStateStatisticsPrint` mixes proof-state-owned counters with formula-archive counts, proof-object GC counters, optional term-bank detail output, and global rewrite-cache accounting. Rust now prints the maintained proof-state counters and represented archive/given-clause counts; formula archives, rewrite-cache uncached counts, demodulator-index details, and final term-detail mode should be filled only when those owners are ported.
- `ProofStateAnalyseGC` adds to `gc_count` and `gc_used_count` without clearing them first. Rust preserves that accumulation, but later reporting APIs should make one-shot analysis versus cumulative statistics explicit.
- `ProofStateInitWatchlist` rebuilds the watchlist through indexed insertion and then inserts it into `wlindices`, but `GlobalIndicesInsertClauseSet` is a no-op unless the backward-rewrite index exists. Rust ports the maximal-term marking, owned FV-index rebuild, and initial-clause dynamic watchlist removal from the owned FV index now; the `ProofStateInit` global-index tail can initialize caller-owned `GlobalIndices`, but long-lived `wlindices` insertion/deletion remains pending until proof-state/global-index ownership is represented without a self-reference.
- `TBGCCollect` marks clause and formula sets registered in `terms->gc`; the current Rust proof state marks every represented clause owner directly before sweeping. Revisit this once formula sets and the bank-owned GC registry are ported so cleanup-time GC uses the same ownership registry as C.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
