<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_neweval

## Source Files

- [CLAUSES/ccl_neweval.h](../../../eprover/CLAUSES/ccl_neweval.h)
- [CLAUSES/ccl_neweval.c](../../../eprover/CLAUSES/ccl_neweval.c)

## Purpose

Data type for representing evaluations of clauses. the GNU Lesser General Public License. <1> Thu Apr 9 02:00:51 MET DST 1998 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `EvalCell`
- `EvalPriority`
- `Eval_p`
- `SimpleEvalCell`
- `SimpleEval_p`

### Macros And Constants

- `CCL_NEWEVAL`
- `EVAL_MEM(eval_no)`
- `EVAL_SIZE(eval_no)`
- `EvalCellAlloc(eval_no)`
- `EvalCellFree(junk, eval_no)`
- `EvalTreeTraverseExit(stack)`
- `PrioBest`
- `PrioDefer`
- `PrioLargestReasonable`
- `PrioNormal`
- `PrioPrefer`

### Globals

- `extern long EvaluationCounter`

### Exported Functions

- `Eval_p EvalTreeExtractEntry(Eval_p *root, Eval_p key, int pos)`
- `Eval_p EvalTreeFind(Eval_p *root, Eval_p key, int pos)`
- `Eval_p EvalTreeFindSmallest(Eval_p root, int pos)`
- `Eval_p EvalTreeInsert(Eval_p *root, Eval_p newnode, int pos)`
- `Eval_p EvalTreeTraverseNext(PStack_p state, int pos)`
- `Eval_p EvalsAlloc(int eval_no)`
- `PStack_p EvalTreeTraverseInit(Eval_p root, int pos)`
- `bool EvalGreater(Eval_p ev1, Eval_p ev2, int pos)`
- `bool EvalTreeDeleteEntry(Eval_p *root, Eval_p key, int pos)`
- `long EvalCompare(Eval_p ev1, Eval_p ev2, int pos)`
- `void EvalListChangePriority(Eval_p list, EvalPriority diff)`
- `void EvalListPrint(FILE* out, Eval_p list)`
- `void EvalListPrintComment(FILE* out, Eval_p list)`
- `void EvalPrint(FILE* out, Eval_p list, int pos)`
- `void EvalPrintComment(FILE* out, Eval_p list, int pos)`
- `void EvalSetPriority(Eval_p list, EvalPriority priority)`
- `void EvalTreePrintInOrder(FILE* out, Eval_p tree, int pos)`
- `void EvalsFree(Eval_p junk)`

## Implementation Notes

### Internal Functions

- `evals_alloc_raw`
- `splay_tree`

### Source-Level Behavior

- `splay_tree`: Perform the splay operation on tree at node with key.
- `EvalsFree`: Free a list of evaluations. Does _not_ check for tree dependencies.
- `EvalPrint`: Print an evaluation to the given channel.
- `EvalPrintComment`: Print an evaluation (as a comment) to the given channel.
- `EvalListPrint`: Print an evaluation list.
- `EvalListPrintComment`: Print an evaluation list as a comment.
- `EvalListSetPriority`: Set the priority in all elements of the list.
- `EvalListChangePriority`: Change the priority in all elements of the list.
- `EvalGreater`: Compare two evaluations, return true if the first one is greater.
- `EvalCompare`: Compare two evaluations, return a value <0, =0 or >0 if the first one is smaller than, equal two, or bigger than the second one.
- `EvalTreeInsert`: If an entry with newnode exists in the tree return a pointer to it. Otherwise insert newnode in the tree and return NULL.
- `EvalTreeFind`: Find the entry with key key in the tree and return it. Return NULL if no such key exists.
- `EvalTreeExtractEntry`: Find the entry with key key and remove it from the tree. Return NULL if no matching element exists.
- `EvalTreeDeleteEntry`: Delete the entry with key key from the tree.
- `EvalTreeFindSmallest`: Find the smallest evaluation.
- `EvalTreeTraverseInit`: Return a stack containing the path to the smallest element in the tree.
- `EvalTreeTraverseNext`: Given a stack describing a traversal state, return the next node and update the stack.
- `EvalTreePrintInOrder`: Print an evaluation tree in ascending order to stdout (mainly for debugging and to test the traversal functions ;-)

### Dependencies

- `"ccl_neweval.h"`
- `<clb_avlgeneric.h>`
- `<clb_ptrees.h>`
- `<clb_sysdate.h>`

### Compile-Time Conditions

- `CCL_NEWEVAL`
- `CONSTANT_MEM_ESTIMATE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; reconciled with production clause-set ownership on 2026-07-17.

Source files reviewed: `CLAUSES/ccl_neweval.h`, `CLAUSES/ccl_neweval.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 810 lines, 24 scanned public declarations, 2 scanned internal function definitions, and 18 structured function-comment blocks.
- Data type for representing evaluations of clauses. the GNU Lesser General Public License. <1> Thu Apr 9 02:00:51 MET DST 1998 New
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- Rust retains the complete exported splay-tree surface in `EvalTree`, using arena handles in place of raw `EvalCell*` links. Insertion, hit/miss splaying, extraction, deletion, smallest lookup, traversal, duplicate handling, and debug rendering preserve the C-shaped standalone behavior.
- Production clause ownership does not put one evaluation cell into several intrusive trees. `ClauseSet` owns one safe ordered root per evaluation position plus an exact evaluation-object-to-sparse-slot map. Insertion snapshots C's priority/heuristic/FIFO comparison key, extraction removes every root entry before moving the clause, and bounded sparse compaction rebuilds the object map without changing evaluation order.
- `ClauseSetFindBest`, standard/single-weight HCB selection, orphan deletion, `HCBClauseSetDelProp`, axiom initialization in `Uniq` order, processed-clause reset, and generated-clause evaluation all use the owned roots. Priority changes that affect ordering occur before insertion, or reweighting removes evaluations and rebuilds every root, preserving C's requirement that an in-tree key not be mutated silently.
- The safe roots use `BTreeSet`, giving logarithmic insertion/removal and direct smallest-entry access, comparable to the amortized logarithmic C splay operations without raw back-pointers. A final evaluation-object tie-breaker retains deliberately cloned cells that C's global `eval_count` uniqueness assumption would otherwise collapse; unique production cells keep exact C order.
- C's unsafe `EvalsFree` contract remains available only through explicit arena-node removal in the standalone compatibility adapter. Clause-set owners remove root entries before dropping cells, so production cannot leave dangling evaluation pointers.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
