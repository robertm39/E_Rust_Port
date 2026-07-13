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

- `ProofStateAlloc` constructs the type bank, signature, main term bank, temporary term bank, fresh variable bank, formula sets, clause sets, watchlist, global indices, FV-index metadata, demodulator aliases, definition store, extraction-root stack, and all counters in one allocation path. The Rust proof-state owner currently ports the internal signature/main-and-temporary-term-bank setup, a shadow-paired fresh variable bank, clause sets, processed-set demodulator indexes, formula sets, optional watchlist, owned main and watchlist global indexes, definition-store clause set plus its separate definition formula archive, represented clause and formula extraction roots for currently ported proof-control exits plus proof-success proof-object/statistics root selection through those stacks, watchlist load/disable activation, bank-backed watchlist maximal-term marking and FV-index rebuild, initial-clause watchlist-hit mutation, FV-index specs/anchors, C cardinality macros, untyped check domain, `$distinct` formula-axiom processing, represented threshold/GSinE/LambdaDef SInE filtering over `axioms` and `f_axioms`, represented relevance pruning over `axioms` and `f_axioms`, `ProofStatePrint` clause-set output order, zeroed statistics including demodulator match-attempt counters, mixed clause/formula represented proof-object analysis, and clause-side `ProofStateAnalyseGC`/`ProofStatePickTrainingExamples`/`ProofStateTrain` over the same represented clause-owner domains. Clause preprocessing hands its populated scratch bank to the proof state, forward tautology checks continue using it through saturation, and proof control performs the C-position 256-node GC check; state-owned SAT solver integration, parser-owned formula population, and formula-producing extraction-root call sites remain later proof-state slices.
- `ProofStateAlloc` applies `signature->distinct_props = signature->distinct_props & (~free_symb_prop)` after internal symbols and term banks are allocated. Rust keeps the same distinct-property mask in the proof-state constructor.
- `ProofStateIsUntyped` checks only processed positive rules, processed positive equations, processed negative units, processed non-units, and unprocessed clauses. It intentionally ignores axioms, watchlist clauses, and formulas.
- `ProofStateStorage` is a macro that sums `ClauseSetStorage` for unprocessed, processed, and archive sets plus `TBStorage(state->terms)`; it intentionally excludes axioms, watchlist, temporary, eval, definition, and formula stores. Rust now exposes a proof-control storage estimate over the same selected domains, including optional FV and demodulator-index storage, for cleanup gating; exact C byte accounting remains tied to allocator and index-memory constants.
- C `fvi_param_init` is reached from proof-control initialization and installs anchors on processed clause sets, the watchlist if present, and `definition_store->def_clauses`. The Rust proof-state shell now owns those targets and installs the same anchor bundle into them.
- `ProofStatePropDocQuote` quotes the five main clause sets in this order: processed positive rules, processed positive equations, processed negative units, processed non-units, then unprocessed. Rust mirrors that order for supported final proof-search documentation quotes, including the `CPSubsumesWatch`/`final_subsumes_wl` watchlist-empty exit and the `CPIgnoreProps` `exists`/`final` exits.
- `ProofStateLoadWatchlist` calls `ClauseSetDocInital` after parsing/marking/weighing/sorting active watchlist clauses. Rust mirrors the supported executable output by printing watchlist initial documentation after parsed-clause initial documentation and before proof-control initialization.

### Change Later

- `ProofStateResetClauseSets` says it empties all clause and formula sets, but the implementation frees `f_ax_archive` twice and does not clear `definition_store` (including its definition formula archive) or `f_archive`. Rust preserves those omissions for now; revisit the reset contract only after definition handling, formula archives, and proof-state reuse callers are ported.
- `ProofStateAlloc` creates `terms` and `tmp_terms` with the same mutable `Sig_p`, and `ProofStateFree` nulls both term-bank signature pointers before freeing the shared signature. Rust `TermBank` currently owns its signature, so `ProofState::tmp_terms_mut` synchronizes a cloned signature only when the function/type counts change. This preserves generated-symbol visibility without unsafe self-references, but an explicit shared proof-session signature handle would remove the synchronization check and better encode C's ownership after compatibility and performance baselines are stable.
- `ProofStatePropDocQuote` accepts an `out` stream parameter but passes `GlobalOut` into each `ClauseSetPropDocQuote` call. Rust's supported final-result path writes through the configured executable output stream; the full proof-documentation port should decide whether to preserve the ignored-parameter quirk only in compatibility shims.
- `ProofStateLoadWatchlist` uses the global `UseInlinedWatchList` sentinel to decide whether to parse from a filename, while any non-null filename, including the inline sentinel, still marks the watchlist as active. The Rust proof-state and executable bridge now model this as enum state internally; keep that shape for future configuration APIs instead of reintroducing sentinel filenames.
- `ProofStateLoadWatchlist` parses file watchlists into the existing `state->watchlist` set and inline mode only activates the already populated set, so inline watchlist clauses parsed from the normal input stream and file-backed watchlist clauses can accumulate in the same set. Rust preserves this append/preload behavior for supported input; a later proof-session API should decide whether merge-vs-replace is compatibility output policy or core watchlist ownership behavior.
- `ProofStateLoadWatchlist` mixes source selection, parsing, activation, watchlist normalization, and `GlobalOut` proof-documentation output. Rust keeps the lower-level state loader output-free and mirrors the output side effect in the executable compatibility layer; keep that split unless full proof-session ownership needs C's exact coupling.
- `ProofStateProcessDistinct` extracts a `$distinct(...)` formula owner, archives the original wrapper, allocates the expanded disequality as a fresh default `WFormula`, and attaches only a `DCExpandDistinct` derivation back to the archived source. This intentionally drops the input name, source info, and role/properties before syntax-only formula printing and before later question/conjecture preprocessing. Rust now preserves that generated-plain-wrapper behavior for direct top-level executable `$distinct(...)`; a cleaned formula owner should decide whether user-facing metadata preservation belongs behind a non-compatibility mode.
- `ProofStatePrint` prints both `processed_pos_rules` and `processed_pos_eqns` under the single heading `Processed positive unit clauses`. Rust preserves that output shape; a later diagnostic/UI layer can split orientable and unorientable positive units only if compatibility output is kept separate.
- `ProofStateStatisticsPrint` mixes proof-state-owned counters with formula-archive counts, proof-object GC counters, optional term-bank detail output, and global rewrite-cache accounting. Rust now prints the maintained proof-state counters, represented formula/archive/given-clause counts, C-shaped cached rewrite steps via the global `RewriteUncached` correction, `PrintDetailedStatistics`-gated generated-literal, shared/unshared term-node, and demodulator-index match-attempt counts, plus `measure-expensive` feature-gated `MEASURE_EXPENSIVE` demodulator node-visit lines over the currently maintained counters, while `eprover` maps the separate global `PDT_COUNT_NODES` line to the non-default `pdt-count-nodes` feature. Supported proof-success runs mark represented clause ancestors before updating the GC counters when C would analyze the proof object; automatic PDTree traversal node-visit accounting and full pointer-stable proof marking should be filled only when those owners are ported.
- `ProofStateStatisticsPrint` computes `generated_count - backward_rewritten_count` and the detailed `generated_lit_count - backward_rewritten_lit_count` value with unsigned counters but prints the results with `%ld`. Rust preserves that signed display interpretation for statistics output; the saturation-limit helper separately keeps the unsigned wrapping value used by C comparisons.
- `ProofStateAnalyseGC` adds to `gc_count` and `gc_used_count` without clearing them first. Rust preserves that accumulation, but later reporting APIs should make one-shot analysis versus cumulative statistics explicit.
- `ProofStateTrain` prints the count line and optional positive/negative clause lists after GC analysis. Rust preserves the represented clause selection after supported proof-success ancestor marking, the C suffix strings (`% trainpos` and `%trainneg`), and explicit LOP/TPTP/TSTP output-format dispatch for both executable training clauses and the reusable lower-level `PStackClausePrint` compatibility helper.
- C proof-object analysis depends on the ordered `Derivation_p`, raw clause/formula pointer identity, `ClauseDerivFindFirst` dummy-quote collapse, `DerivStackExtractParents` expansion of `sig->ac_axioms`, and `DerivStackCountSearchInferences`. Rust now exposes represented proof-object analysis over selected roots, using local pointer identity for clauses/formulas while falling back to compact reference resolution between sets, following represented formula parents from active formulas, the formula axiom archive, the definition formula archive, and the general formula archive plus signature AC axiom parents for analysis and graph traversal. Exact ordered-derivation counts remain pending until the ordered proof-object owner is ported.
- C keeps extraction roots as raw derived-node pointers that can point at either clauses or formulas. Rust currently keeps separate cloned clause/formula root stacks until stable proof-state handles can represent the same mixed identity without value copies.
- `ProofStateInitWatchlist` rebuilds the watchlist through indexed insertion and then inserts it into `wlindices`, but `GlobalIndicesInsertClauseSet` is a no-op unless the backward-rewrite index exists. Rust ports the bank-backed maximal-term marking, owned FV-index rebuild, initial-clause dynamic watchlist removal from the owned FV index, and state-owned watchlist `GlobalIndices` insertion/deletion/reinsertion through executable proof search. Rust avoids C's raw signature pointer by passing the live signature into fingerprint queries, so `wlindices` can remain an ordinary owned `ProofState` field rather than a self-reference.
- `TBGCCollect` marks clause and formula sets registered in `terms->gc`; Rust now registers represented proof-state owners, including the separate definition formula archive, through stable term-bank GC handles and resolves those handles before sweeping. Revisit the registry once clause/formula owners have typed stable handles, so the Rust port can represent C's raw-pointer registry without hard-coded proof-state handle constants or a split `DefStore` representation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
