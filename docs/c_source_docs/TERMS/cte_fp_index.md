<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_fp_index

## Source Files

- [TERMS/cte_fp_index.h](../../../eprover/TERMS/cte_fp_index.h)
- [TERMS/cte_fp_index.c](../../../eprover/TERMS/cte_fp_index.c)

## Purpose

Fingerprint based indexing of terms. A fingerprint is a extor of samples of symbols at different positions. The index is a try build over these vectors. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FPIndexCell`
- `FPIndex_p`
- `FPLeafPayloadPrint`
- `FPLeafPrintFun`
- `FPTreeCell`
- `FPTreeFreeFun`
- `FPTree_p`

### Macros And Constants

- `CTE_FP_INDEX`
- `FPIndexCellAlloc()`
- `FPIndexCellFree(junk)`
- `FPTreeCellAlloc()`
- `FPTreeCellFree(junk)`
- `GET_SYMBOL_ARITY(sig, f_code)`

### Globals

- None found in the source scan.

### Exported Functions

- `FPIndex_p FPIndexAlloc(FPIndexFunction fp_fun, Sig_p sig, FPTreeFreeFun payload_free)`
- `FPTree_p FPIndexFind(FPIndex_p index, Term_p term)`
- `FPTree_p FPIndexInsert(FPIndex_p index, Term_p term)`
- `FPTree_p FPTreeAlloc(void)`
- `FPTree_p FPTreeFind(FPTree_p root, IndexFP_p key)`
- `FPTree_p FPTreeInsert(FPTree_p root, IndexFP_p key)`
- `PERF_CTR_DECL(IndexMatchTimer)`
- `PERF_CTR_DECL(IndexUnifTimer)`
- `long FPIndexCollectLeaves(FPIndex_p index, PStack_p result)`
- `long FPIndexFindMatchable(FPIndex_p index, Term_p term, PStack_p collect)`
- `long FPIndexFindUnifiable(FPIndex_p index, Term_p term, PStack_p collect)`
- `long FPTreeFindMatchable(FPTree_p root, IndexFP_p key, Sig_p sig, PStack_p collect)`
- `long FPTreeFindUnifiable(FPTree_p root, IndexFP_p key, Sig_p sig, PStack_p collect)`
- `void FPIndexDelete(FPIndex_p index, Term_p term)`
- `void FPIndexDistribDataPrint(FILE* out, FPIndex_p index)`
- `void FPIndexDistribPrint(FILE* out, FPIndex_p index)`
- `void FPIndexFree(FPIndex_p index)`
- `void FPIndexPrint(FILE* out, FPIndex_p index, FPLeafPrintFun prtfun)`
- `void FPIndexPrintDot(FILE* out, char* name, FPIndex_p index, FPLeafPayloadPrint prt_sig, Sig_p sig)`
- `void FPTreeDelete(FPTree_p root, IndexFP_p key)`
- `void FPTreeFree(FPTree_p index, FPTreeFreeFun payload_free)`

## Implementation Notes

### Internal Functions

- `dt_index_rek_find_unifiable`
- `fp_index_rek_find_matchable`
- `fp_index_rek_find_unif`
- `fp_index_tree_print`
- `fp_symbol`
- `fpindex_alternative`
- `fpindex_alternative_ref`
- `fpindex_extract_alt`
- `fpindex_rek_delete`

### Source-Level Behavior

- `fpindex_alternative`: Return the child indexed by key f_code in index if it exists, NULL otherwise.
- `fpindex_alternative_ref`: Return the address of the child pointer indexed by key f_code in index (create it if its not already there).
- `fpindex_extract_alt`: Return the the child pointer indexed by key f_code in index and remove it from the index.
- `fpindex_rek_delete`: Delete the branches leading (only) to the leaf identified to the given key, _if_ that node has no payload. Return true if the current node should be deleted.
- `fpindex_rek_find_unif`: Find (and push) all payloads from terms unification-compatible with key.
- `fpindex_rek_find_matchable`: Find (and push) all payloads from terms match-compatible with key.
- `fp_index_leaf_prt_size`: Print a leaf as the path leading to it and the number of direct entries in the subterm.
- `fp_index_tree_print`: Print an FP index tree. Return the number of leaves and (via *entries), the number of entries.
- `fp_index_tree_collect_distrib`: Collect distribution information for an fp-tree. Return number of nodes.
- `fp_symbol`: Return the symbol of a given fingerprint sample.
- `fp_index_tree_print_node`: Print a tree node in DOT notation. See below.
- `fp_index_tree_print_nodes`: Print all the nodes in the FP-Tree in DOT notation, using the symbols (and symbol-codings) on the stack for the label.
- `fp_index_tree_print_edges`: Print all the edges in the fp-tree in DOT notation.
- `fp_index_collect_leaves`: Push all the leaves in index onto result.
- `fp_index_find_all`: Push all payloads in index onto stack. Return number of payloads found.
- `GET_SYMBOL_ARITY`: Local macro for getting the effective arity of any top-level symbol in a term (including variables).
- `dt_index_rek_find_matchable`: Find all leaves in index that are potentially matchable with the term that is represented by key (key is the flat term version of the query term). Push all payloads of leaves onto collect. skip_term indicates how many complete (sub-)terms need to be skipped to complete a term that corresponds to a variable in the query. If skip_term = 0: consume next symbol...
- `dt_index_rek_find_unifiable`: Find all leaves in index that are potentially unifiable with the term that is represented by key (key is the flat term version of the query term). Push all payloads of leaves onto collect.
- `FPTreeAlloc`: Allocate an initialized FPTreeCell.
- `FPTreeFree`: Free an FPTree tree.
- `FPTreeFind`: Find the leaf node corresponding to key in the index at root. Return NULL if no such node exists.
- `FPTreeInsert`: Insert a node corrsponding to key into the index (if necessary) and return a pointer to it.
- `FPTreeDelete`: Delete a node corresponding to a key if it does not carry any payload.
- `FPTreeFindUnifiable`: Push all the payloads of nodes unification-compatible with the given key onto the stack. Return number of payloads pushed.
- `FPTreeFindMatchable`: Push all the payloads of nodes match-compatible with the given key onto the stack. Return number of payloads pushed.
- `FPIndexAlloc`: Alloc an FPIndex.
- `FPIndexFree`: Free an FPIndex.
- `FPIndexFind`: Find the index tree node representing term (if any) and return it (or NULL).
- `FPIndexInsert`: Return a node representing term, creating it if necessary.
- `FPIndexDelete`: Delete the node representing term, unless it's still in use (per the payload field).
- `FPIndexFindUnifiable`: Return (via collect) all payloads of nodes representing potentially unifiable terms.
- `FPIndexFindMatchable`: Return (via collect) all payloads of nodes representing potentially matchable terms.
- `FPIndexDistribPrint`: Print the pathes in the index and the number of stored terms at each leaf of the FPTree.
- `FPIndexCollectDistrib`: Collect statistics for the node number and leaf term distribution. Returns number of nodes directly, leaves and average and standard deviation of terms/leaf via OUT parameters.
- `FPIndexDistribDataPrint`: Collect and print statistics about the FP-Index.
- `FPIndexPrint`: Print an FP-Index.
- `FPIndexCollectLeaves`: Push all leaves of an FPIndex onto the result stack. Return number of values pushed.
- `FPIndexPrintDot`: Print an FP-Index as a dot graph.

### Dependencies

- `"cte_fp_index.h"`
- `"cte_termfunc.h"`
- `<clb_intmap.h>`
- `<clb_objtrees.h>`
- `<cte_idx_fp.h>`

### Compile-Time Conditions

- `CTE_FP_INDEX`
- `NEVER_DEFINED`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_fp_index.h`, `TERMS/cte_fp_index.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 1629 lines, 30 scanned public declarations, 9 scanned internal function definitions, and 38 structured function-comment blocks.
- Fingerprint based indexing of terms. A fingerprint is a extor of samples of symbols at different positions. The index is a try build over these vectors. the GNU Lesser General Public License.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `FPIndexPrintDot` uses raw pointer addresses as DOT node identifiers and does not escape symbol labels; this is useful for C-debug parity but should not become the final reproducible user-facing graph format without a compatibility decision.
- `FPIndexPrintDot` connects payload boxes only for structural leaves collected by `FPIndexCollectLeaves`, while `FPIndexDistribPrint`/`FPIndexPrint` visit every node with a payload. Preserve that split for compatibility, but consider a clearer diagnostic renderer after the clause/subterm payload printers are integrated.
- `FPIndexDistribPrint` computes `entries/leaves` directly, so an empty index is an unguarded floating-point division. A cleaned wrapper should handle empty indexes explicitly once callers are known.
- Rust now uses the generic DOT scaffolding plus a term-bank-backed flattened subterm payload renderer for `eprover`'s optional `PRINT_INDEX_STATS`/`print-index-stats` path; other payload renderers should still be added only when a C diagnostic path needs them.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
