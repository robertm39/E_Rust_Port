<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_pdtrees

## Source Files

- [CLAUSES/ccl_pdtrees.h](../../../eprover/CLAUSES/ccl_pdtrees.h)
- [CLAUSES/ccl_pdtrees.c](../../../eprover/CLAUSES/ccl_pdtrees.c)

## Purpose

Perfect discrimination trees for optimized rewriting and subsumption. PDTrees are machines and have a state - each new search must initialize a tree to a consistent state, and only one search may be conducted at any given time.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PDTNodeCell`
- `PDTNode_p`
- `PDTreeCell`
- `PDTree_p`
- `TraversalState`

### Macros And Constants

- `CCL_PDTREES`
- `PDNODE_FUN_GROW_ALT`
- `PDNODE_FUN_INIT_ALT`
- `PDNODE_VAR_GROW_ALT`
- `PDNODE_VAR_INIT_ALT`
- `PDTNODE_MEM`
- `PDTNodeCellAlloc()`
- `PDTNodeCellFree(junk)`
- `PDTNodeGetAgeConstraint(node)`
- `PDTNodeGetSizeConstraint(node)`
- `PDTREE_CELL_MEM`
- `PDTREE_IGNORE_NF_DATE`
- `PDTREE_IGNORE_TERM_WEIGHT`
- `PDT_COUNT_INC(x)`
- `PDT_NODE_CLOSED(tree,node)`
- `PDT_NODE_INIT_VAL(tree)`
- `PDTreeAlloc(bank)`
- `PDTreeCellAlloc()`
- `PDTreeCellFree(junk)`
- `PDTreeStorage(tree)`
- `TermPCompare`

### Globals

- `extern bool PDTreeUseSizeConstraints`
- `extern unsigned long PDTNodeCounter`

### Exported Functions

- `ClausePos_p PDTreeFindNextDemodulator(PDTree_p tree, Subst_p subst)`
- `PDTNode_p PDTNodeAlloc(void)`
- `PDTNode_p PDTreeFindNextIndexedLeaf(PDTree_p tree, Subst_p subst)`
- `PDTNode_p PDTreeMatchPrefix(PDTree_p tree, Term_p term, long* matched, long* remains)`
- `PDTree_p PDTreeAllocWDeleter(TB_p bank, Deleter deleter)`
- `Term_p TermLRTraverseNext(PStack_p stack)`
- `Term_p TermLRTraversePrev(PStack_p stack, Term_p term)`
- `bool PDTreeInsert(PDTree_p tree, ClausePos_p demod_side)`
- `bool PDTreeInsertTerm(PDTree_p tree, Term_p term, ClausePos_p demod_side, bool store_data)`
- `long PDTreeDelete(PDTree_p tree, Term_p term, Clause_p clause)`
- `void PDTNodeFree(PDTNode_p tree, Deleter deleter)`
- `void PDTreeFree(PDTree_p tree)`
- `void PDTreePrint(FILE* out, PDTree_p tree)`
- `void PDTreeSearchExit(PDTree_p tree)`
- `void PDTreeSearchInit(PDTree_p tree, Term_p term, SysDate age_constr, bool prefer_general)`
- `void TermLRTraverseInit(PStack_p stack, Term_p term)`

## Implementation Notes

### Internal Functions

- `delete_clause_entries`
- `pdt_compute_age_constraint`
- `pdt_compute_size_constraint`
- `pdt_node_succ_stack_create`
- `pdt_select_alt_ref`
- `pdt_select_next`
- `pdtree_backtrack`
- `pdtree_default_cell_free`
- `pdtree_forward`
- `pdtree_verify_node_constr`
- `pos_tree_compute_age_constraint`
- `pos_tree_compute_size_constraint`

### Source-Level Behavior

- `pdtree_default_cell_free`: Free a node cell (but not potential children et al.)
- `pdt_select_alt_ref`: Return a pointer to the position where the alternative to term is stored.
- `pdt_select_next`: Find an alternative node based on the term given. The difference between this function and pdt_select_alt_ref is that no node in the underlying data structures will be created.
- `pdt_node_succ_stack_create`: Create a stack of all children of node and return it (for convenient traversal).
- `pos_tree_compute_size_constraint`: Find the size of the smallest term at a position in tree.
- `pdt_compute_size_constraint`: Compute and set the size constraint of the current node in the PDT tree.
- `pdt_verify_size_constraint`: Verify the size constraint at node, and return the optimal value (or -1 if the tree is inconsistent)
- `pos_tree_compute_age_constraint`: Find the creation date of the youngst clauss at a position in tree.
- `pdt_compute_age_constraint`: Compute and set the age constraint (i.e. date stamp of the youngest clause in the subtree) of the current node in the PDT tree.
- `pdt_verify_age_constraint`: Verify the age constraint at node, and return the optimal value (or -1 if the tree is inconsistent)
- `delete_clause_entries`: Consider *root as a PTree of ClausePos_p and delete all entries from it that describe a position in clause. Return number of clauses.
- `pdtree_verify_node_constr`: Check if the current tree state is consistent with the query constraints stored in the tree.
- `pdtree_forward`: Find the next open possibility and advance to it. If none exists, indicate this by setting tree->tree_pos->var_traverse_stack to PDT_NODE_CLOSED.
- `pdtree_backtrack`: Backtrack to the predecessor node of the current state.
- `pdt_node_print`: Print a PDT node (and subtrees) for debugging.
- `PDTreeAllocWDeleter`: Allocate an empty, initialized PDTreeCell (including the initial PDTNodeCell().
- `PDTreeFree`: Completely free a PDTree.
- `PDTNodeAlloc`: Return an initialized node in a PDTree.
- `PDTNodeFree`: Free a PDTreeNode (including all referenced term positions.
- `TermLRTraverseInit`: Initialize a stack for term traversal.
- `TermLRTraverseNext`: Return the next term node in LR-ordering and update the stack. Return NULL if term traveral is complete.
- `TermLRTraversePrev`: Undo a TermLRTraverseNext() operation by replacing terms args on the stack with term.
- `PDTreeInsert`: Insert a new demodulator into the tree.
- `PDTreeInsertTerm`: Insert a new term into the tree, possibly storing data in the leaf.
- `PDTreeMatchPrefix`: Match the term against the tree and count matches/mismatches. Return the last matched node. The term is in the tree iff remains == 0.
- `PDTreeDelete`: Delete all index entries of clause indexed by term from tree. Return number of entries deleted.
- `PDTreeSearchInit`: Initialize a PDTree for searching for matching terms.
- `PDTreeSearchExit`: Mark a PDTree as not currently used in a search.
- `PDTreeFindNextIndexedLeaf`: Given a search state encoded in the tree and a (partial) substitution, find the next leaf node and return it. Extend subst to a suitable substitution.
- `PDTreeFindNextDemodulator`: Return the next matching clause position in the tree search represented by tree.
- `PDTreePrint`: Print a PD tree in human-readable form (for debugging).

### Dependencies

- `"ccl_derivation.h"`
- `"ccl_pdtrees.h"`
- `<ccl_clausepos.h>`
- `<clb_intmap.h>`
- `<clb_objmaps.h>`
- `<clb_ptrees.h>`
- `<clb_simple_stuff.h>`
- `<cte_lambda.h>`

### Compile-Time Conditions

- `CCL_PDTREES`
- `CONSTANT_MEM_ESTIMATE`
- `MEASURE_EXPENSIVE`
- `PDT_COUNT_NODES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_pdtrees.h`, `CLAUSES/ccl_pdtrees.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1749 lines, 24 scanned public declarations, 12 scanned internal function definitions, and 31 structured function-comment blocks.
- Perfect discrimination trees for optimized rewriting and subsumption. PDTrees are machines and have a state - each new search must initialize a tree to a consistent state, and only one search may be conducted at any given time.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- Initial Rust status: `src/clauses/pdtrees.rs` ports the `TermLRTraverseNext` key sequence plus an owned trie for `PDTreeInsertTerm`-style insertion, `PDTreeMatchPrefix` match/remains counting, C-shaped `PDTreeStorage` constant-memory accounting for the represented trie subset, code/term deletion with C-shaped prefix ref-count decrementing plus dead-child pruning, and the `match_count`/`visited_count` bookkeeping fields used by proof-state demodulator statistics. `che_prefixweight` now stores its lazy conjecture-prefix terms in that trie, and `che_tfidfweight` stores document-frequency terms in the same trie subset so IDF can use node ref-counts.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- C `PDTreeInsertTerm` eta-expands non-FO patterns or eta-reduces other terms before indexing, and full search mutates tree-local traversal state while relying on `ClausePos` leaves, substitution backtracking, age/size constraints, and a process-global traversal order switch. Keep those compatibility surfaces visible before replacing the remaining plain scans with full PDTree ownership.
- C `PDTreeDelete` deletes clause-position entries at the leaf, subtracts the number of removed entries from every traversed node, physically frees dead nodes, and invalidates cached size/age constraints for later recomputation. Rust now mirrors the ref-count and child-pruning shape for the current trie-only term/code subset, but leaf `ClausePos` payload deletion and constraint invalidation still belong with the full demodulator/search index owner.
- C `PDTreeDelete` removes function alternatives from the parent `IntMap`, but the `arr_storage_est` adjustment subtracts and re-adds the deleted child's function-alternative storage instead of the parent's changed storage. Rust preserves that stale estimate for `PDTreeStorage`; clean accounting should be considered only after compatibility tests cover demodulator-index memory reporting.
- C increments `visited_count` from the internal traversal paths and only reports it when compiled with `MEASURE_EXPENSIVE`. Rust represents the counter and exposes an explicit increment hook, but should wire automatic node-visit accounting only when `PDTreeFindNextIndexedLeaf`/`PDTreeFindNextDemodulator` traversal is ported.

<!-- END MANUAL REVIEW: c_source_docs -->
