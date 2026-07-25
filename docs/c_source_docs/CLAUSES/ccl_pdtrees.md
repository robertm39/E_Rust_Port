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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for live-substitution traversal, iterative query construction, reusable query storage, one-pass query metadata, root query-weight reuse, variable-edge metadata, eta-normalized indexing, and demodulator leaf ordering on 2026-07-14; updated for split function alternatives and direct first-order symbol traversal on 2026-07-21; updated for allocation-free prefix matching on 2026-07-25.

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
- Initial Rust status: `src/clauses/pdtrees.rs` ports the `TermLRTraverseNext` key sequence plus an owned trie for `PDTreeInsertTerm`-style insertion, `PDTreeMatchPrefix` match/remains counting, C-shaped `PDTreeStorage` constant-memory accounting for the represented trie subset, code/term deletion with C-shaped prefix ref-count decrementing plus dead-child pruning, cached minimum term-weight and youngest-clause-date constraints with the C pruning predicate at each traversed node, variable type-UID checks, C-style variable-edge weight adjustment, repeated-variable consistency, and exact terminal clause-derivation-reference/side payloads. Like C, the cheap clause-set precheck now tests only root constraints and leaves path rejection to the live cursor. First-order demodulator lookup uses an incremental search-local frame cursor and compact variable-edge arena with invariant type/weight snapshots, validates each child constraint and initializes terminal state before pushing the child frame, stores speculative bindings as variable-child/query-cell indices instead of cloned term handles, leaves accepted bindings live in the caller's substitution, and follows the recorded symbols-first or variables-first branch order; the bank-aware higher-order path retains materialized candidate recovery. Prefix-token classification reads the function code once and borrows the term's type UID without cloning its type handle. Bank-aware proof-state insertion and rewrite/unit search eta-expand non-FO patterns and eta-reduce other lambda-bearing terms before building the PDTree path, while changed insertion paths are retained for extraction-time deletion without a second mutable term-bank borrow. The port also carries `match_count`; per-tree `visited_count` instrumentation is enabled only by `measure-expensive`, while the feature-gated global `PDTNodeCounter` remains independently available. Query code/weight/date/subtree spans, `prefer_general` side effects, active-query clear with C's stale temporary weight/date fields, and scoped equivalents of C's process-global constraint switches are preserved. `che_prefixweight` and `che_tfidfweight` use the represented trie subset for prefix and document-frequency terms.
- Query construction now reuses an owned query-cell vector between searches while preserving code, term, weight, type, and subtree-span data; search exit clears active handles but retains capacity for the next search.
- `PDTreeMatchPrefix` now follows C's incremental `TermLRTraverseNext` shape instead of materializing a complete prefix-token vector before matching. A tree-owned term stack is empty outside the call and retains its allocation for the next prefix-weight evaluation. This removes SWV851's observed unguarded 2,048-byte allocation under the exact 2 GiB limit; the GDB trace, resource outcomes, proof hash, and exact-work cost are retained in [`experiment 304`](../../../experiments/2026-07-25-003-linux-fallible-clause-admission/FINDINGS.md).
- C stores ordinary function edges as child pointers in `f_alternatives` and keeps free-variable/DB-like edges in separate object maps. Rust now mirrors that split: its `IntMap` carries function-code-to-node indices, while the ordered token map contains only object alternatives. This removes the earlier duplicate function edge and its hot ordered lookup; exact proof, constrained-resource, and 50-case evidence are retained in [`experiment 166`](../../../experiments/2026-07-21-166-pdt-function-intmap/FINDINGS.md).
- In first-order mode, C's symbol branch rejects top-level free variables before reading `term->f_code` from `f_alternatives`; it does not construct the identity/type/weight tuple needed by variable-edge traversal. Rust now uses the same direct negative-code test and integer-map lookup in its first-order cursor, while retaining complete token classification for higher-order and uninitialized modes. The exact 0.5389% whole-prover reduction and compatibility evidence are retained in [`experiment 168`](../../../experiments/2026-07-21-168-pdt-first-order-symbol/FINDINGS.md).
- Once that first-order symbol lookup succeeds, C traverses the ordinary term's argument array directly; the variable, lambda, and applied-DB alternatives have already been excluded. Rust now uses a first-order-only expansion helper at the same boundary, borrowing the argument slice once and preserving reverse push/left-to-right visit order, while higher-order and uninitialized searches retain the general expansion path. Exact LUSK6 work falls another 1.0617% and the dominant cursor falls 4.5868%; proof, resource, and full-matrix evidence are retained in [`experiment 180`](../../../experiments/2026-07-21-180-pdt-first-order-expansion/FINDINGS.md).
- The Rust cursor's traversal-step field represents only symbols, variables, and exhaustion. It is now one byte, keeping each 64-bit traversal frame at 40 bytes without narrowing node or substitution positions. This compaction was originally rejected when faster BOO020 search reached an infallible allocator boundary, but the fallible evaluated-clause admission boundary from experiment 165 now preserves C-compatible `ResourceOut`; exact proof, constrained-resource, full-matrix, and 0.2556% Callgrind evidence are retained in [`experiment 170`](../../../experiments/2026-07-21-170-compact-pdt-frame-revisit/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- C `PDTreeInsertTerm`, `PDTreeDelete`, and `PDTreeSearchInit` independently repeat the same classification and eta-normalization dispatch: non-FO patterns expand and every other term reduces. This preserves shared-term-bank side effects and guarantees all three operations derive the same key, but deletion rebuilds a key already established at insertion and the API does not make that invariant explicit. After compatibility is secured, consider returning a normalized index handle/path from insertion and consuming it at deletion, with one shared classification helper for ad hoc searches; preserve the current non-FO-pattern-first branch and benchmark the extra handle storage before changing this hot path.
- Rust's bank-aware proof-state path now applies that eta rule and retains prefix codes only for occurrences whose normalized key changed. Clause-set occurrences carry the exact generational clause reference and side returned to rewrite/unit consumers, so duplicate visible identifiers remain distinct through normalized insertion, search, resolution, and deletion. The auxiliary path map is not included in the C-shaped `PDTreeStorage` estimate; raw first-order/standalone APIs intentionally remain normalization-free until their ownership and bank contracts are explicit.
- C `PDTreeSearchExit` nulls the active `term` and `store_stack`, but it leaves the temporary `term_weight` and `term_date` fields unchanged from the last search. Rust mirrors that split by clearing the active query state while preserving the stale temporary values; consider resetting those temporaries only after reference tests show no statistics, pruning, or debugging surface relies on the C lifetime quirk.
- C stores the active branch order in the file-global `trav_order` array while storing the rest of a search cursor in the tree and its nodes. Initializing a search on one tree can therefore change traversal policy for an overlapping search on another tree, and even two searches with the same policy cannot safely share one tree. Rust keeps traversal policy and query state per tree; a later C API should move all cursor state into a dedicated search object after single-threaded proof-order compatibility is secured.
- C `PDTreeSearchInit` traverses the query through the tree-owned `term_stack` and `term_proc` callback, reusing the stack allocation across searches but making query construction part of the same non-reentrant tree state. Rust preserves the useful allocation behavior with an owned query-cell vector that is cleared on exit and reused by the next search, while keeping traversal continuation out of trie nodes. A later C cleanup should retain reusable storage in an explicit search object and keep `PDTreeSearchExit`'s observable active-state lifetime separate from buffer capacity.
- `TermLRTraverseNext` embeds the lambda/applied-DB-variable first-argument decision in `i >= (predicate) ? 1 : 0`; C precedence makes the loop work, but the expression is easy to misread or alter incorrectly. `TermLRTraversePrev` reverses a step by popping raw child pointers and validates the required stack shape only with assertions. A later C cleanup should compute the lower bound once and encapsulate reversible traversal in the explicit search object, while retaining direct argument-array access and exact left-to-right order. Rust's reusable enter/exit stack makes this state typed and testable; it adds 0.54% LUSK6 Callgrind instructions but improves measured LUSK6, HEN011, and GEO288 CPU time by reducing repeated dynamic argument borrows.
- `TermLRTraverseNext`, `pdt_select_alt_ref`, and `pdt_select_next` independently expand overlapping `TermIsTopLevelFreeVar`/`TermIsAppliedDBVar`/`TermIsLambda` predicates for the same term. These are cheap direct-field macros in optimized C, but higher-order head/property tests are still repeated and the classification contract is spread across traversal and edge selection. A later C search-step API could snapshot token class, DB/lambda head, and first visible argument once while retaining direct argument-array access, exact branch order, and zero per-cell allocation. Rust's equivalent one-pass snapshot reduces matched LUSK6 Callgrind instructions by 0.49% and improves its seven-pair median user and wall time.
- `PDTreeInsertTerm` computes `TermStandardWeight(term)` before traversal and then recomputes the unchanged root weight for every inserted node; `PDTreeSearchInit` also evaluates standard weight in an assertion and again for assignment. Optimized shared-term builds usually reduce these macros to cached fields, but assertion builds recursively validate weight and can turn the insertion loop into avoidable repeated subtree walks. Cache and validate the invariant root weight once if the C path is cleaned up; do not change the per-node size-constraint value. Rust now sources search state from the weight already stored in the root query cell, removing 2,632,017 matched LUSK6 Callgrind instructions from `record_search_init` without changing the query or pruning value.
- C variable alternatives retain the indexed `Term_p` and reread its type and standard weight during each successful variable traversal. Direct C field/macros make those reads cheap, but correctness implicitly requires shared indexed terms not to change type or weight before deletion. Make that immutability contract explicit if term storage is redesigned; an edge metadata snapshot becomes attractive only if access is encapsulated or reference-counted, because duplicating the fields in current C would increase every variable edge for little measured benefit. Rust snapshots the two invariants in its compact arena, refreshes them on free-slot reuse, and reduces matched LUSK6 traversal instructions by almost 10%.
- C `PDTreeDelete` deletes clause-position entries at the leaf, subtracts the number of removed entries from every traversed node, physically frees dead nodes, and lazily invalidates cached size/age constraints for later recomputation. Rust now mirrors the ref-count and child-pruning shape for the current trie-only term/code subset and eagerly recomputes represented terminal metadata after deletion; the lazy invalidation shape can be revisited only after full `ClausePos` payload ownership and traversal reference traces show whether the intermediate sentinel state is externally observable.
- C `PDTreeDelete` removes function alternatives from the parent `IntMap`, but the `arr_storage_est` adjustment subtracts and re-adds the deleted child's function-alternative storage instead of the parent's changed storage. Rust preserves that stale estimate for `PDTreeStorage`; clean accounting should be considered only after compatibility tests cover demodulator-index memory reporting.
- C increments `visited_count` on successful traversal advances and, under `PDT_COUNT_NODES`, increments the global `PDTNodeCounter` during node-constraint verification. Rust records successful advances in both materialized and live-substitution traversal through the shared hook, but compiles per-tree counting only with `measure-expensive`; `pdt-count-nodes` still controls the global counter independently. Revisit exact optional-counter parity only if those diagnostic builds become an externally compared surface.
- `pdtree_forward`, `pdtree_backtrack`, and `PDTreeFindNextIndexedLeaf` overwrite `trav_state`, `prev_subst`, and a reusable `var_traverse_stack` inside shared nodes, while `trav_order` is process-global. Every node therefore permanently owns a traversal stack even while idle, and searches are non-reentrant and thread-hostile. Rust's explicit search-local frames and separate compact variable-edge arena preserve branch order and the live-substitution contract while reducing exact LUSK6 Callgrind instructions by 1.98%. A later C cleanup should move all continuation state and reusable traversal storage into a dedicated search object, but it must continue returning candidates with accepted bindings live; dropping that coupling caused measured 30-38% LUSK6 and roughly 10% GEO288 regressions in rejected Rust experiments.
- `PDTreeFindNextDemodulator` traverses each leaf's `ClausePos*` entries through an in-order `PTree`, so candidate priority is ascending raw address. Those positions come from the process-global `SizeMalloc` size-class free list shared with unrelated object types; deletion and allocator traffic can therefore change which equivalent demodulator is tried first and can alter retained clauses and proofs. Rust uses reverse terminal insertion order as the stable surrogate observed for the canonical `LUSK6ext.lop` reference. A cleaned C index should use an explicit semantic or insertion-sequence key, but changing this before compatibility is secured changes search behavior.

<!-- END MANUAL REVIEW: c_source_docs -->
