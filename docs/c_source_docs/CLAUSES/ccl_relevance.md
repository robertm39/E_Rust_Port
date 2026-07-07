<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_relevance

## Source Files

- [CLAUSES/ccl_relevance.h](../../../eprover/CLAUSES/ccl_relevance.h)
- [CLAUSES/ccl_relevance.c](../../../eprover/CLAUSES/ccl_relevance.c)

## Purpose

Code implementing some limited relevance analysis for function symbols and clauses/formulas. the GNU Lesser General Public License. <1> Sun May 31 11:20:27 CEST 2009

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RelevanceCell`
- `Relevance_p`

### Macros And Constants

- `CCL_RELEVANCE`
- `RelevanceCellAlloc()`
- `RelevanceCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `Relevance_p RelevanceAlloc(void)`
- `Relevance_p RelevanceDataCompute(ProofState_p state)`
- `long ProofStateRelevancyProcess(ProofState_p state, long level)`
- `long RelevanceDataInit(ProofState_p state, Relevance_p data)`
- `void ClausePListPrint(FILE* out, PList_p list)`
- `void FormulaPListPrint(FILE* out, PList_p list)`
- `void RelevanceFree(Relevance_p junk)`

## Implementation Notes

### Internal Functions

- `move_clauses`
- `move_formulas`
- `proofstate_rel_prune`

### Source-Level Behavior

- `find_level_fcodes`: Find all (non-special) function symbols in the relevance cores and assign their relevance level. Push them onto the new_codes stack (once).
- `extract_new_core`: Find the formulas and clauses in the the "rest" part and put them into the core.
- `move_clauses`: Given a plist of clauses, move them into the clauseset.
- `move_formulas`: Given a plist of formulas, move them into the formulaset.
- `proofstate_rel_prune`: Use the relevance data to prune axioms to those with a relevancy <= level.
- `RelevanceAlloc`: Allocate a relevancy data structure - mostly used to be able to clearly state invariants. After initialization: - Core contains the newly found relevant clauses and formulas - Rest contains the remainder of clauses and formulas - new_codes is the set of newly found relevant function symbols. - f_code_relevance contains for all f_codes the relevance level (i...
- `RelevanceFree`: Free a RelevanceCell data structure.
- `ClausePListPrint`: Print a plist of clauses.
- `FormulaPListPrint`: Print a plist of WFormulas.
- `RelevanceDataInit`: Initialize a relevancy data structure - Split conjectures and non-conjectures, and index the non-conjectures.
- `RelevanceDataCompute`: Compute the relevance levels.
- `ProofStateRelevancyProcess`: Perform proof state preprocssing, in particular compute relevancy data and perform relevancy pruning.

### Dependencies

- `"ccl_relevance.h"`
- `<ccl_findex.h>`
- `<ccl_proofstate.h>`
- `<clb_plist.h>`

### Compile-Time Conditions

- `CCL_RELEVANCE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_relevance.h`, `CLAUSES/ccl_relevance.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 605 lines, 9 scanned public declarations, 3 scanned internal function definitions, and 12 structured function-comment blocks.
- Code implementing some limited relevance analysis for function symbols and clauses/formulas. the GNU Lesser General Public License. <1> Sun May 31 11:20:27 CEST 2009
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `RelevanceDataInit` splits conjecture and non-conjecture clauses/formulas into `PList` buckets with `PListStoreP(anchor, ...)`; because insertion is after the anchor, each bucket is reversed relative to source set traversal.
- `RelevanceDataCompute` records clause and formula core lists before allocating fresh empty cores, then `extract_new_core` moves matching rest-list cells into the new cores through the function-symbol index.
- `proofstate_rel_prune` treats level `0` outside the helper: `ProofStateRelevancyProcess` returns before pruning. For requested levels beyond computed relevance levels, it moves all remaining rest clauses/formulas into the new axiom sets.
- Rust now shares the C-shaped relevance traversal over represented clause and formula axiom owners, including formula `PList` indexing and axiom-count pruning deltas. Supported proof-search coverage now includes represented FOF formula owners pruned before CNF emits clauses. `che_funweights` relevance-level weights can consume the same represented formula axiom context; parser-owned formula population remains separate follow-up work.

### Change Later

- The C relevance implementation exposes raw `PList` list order in pruning results. If reference tests show the order does not matter, later Rust code could prefer source-order stable vectors for readability, but the current port should preserve the C-shaped reversed buckets.
- `extract_new_core` repeatedly consumes the root of a `PTree` bucket keyed by raw `PList` cell addresses. That root depends on allocation addresses and splay-tree history; replacing it with deterministic handle order is a good Rust cleanup candidate only if proof/pruning behavior tests allow it.
- The clause and formula relevance paths are duplicated structurally in C. Rust now shares the traversal/indexing scaffolding for represented owners while keeping compatibility-visible mutation order explicit; revisit the remaining duplication only after parser-owned formula handles and reference output tests are in place.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
