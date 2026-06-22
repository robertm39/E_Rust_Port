<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_numxtrees

## Source Files

- [BASICS/clb_numxtrees.h](../../../eprover/BASICS/clb_numxtrees.h)
- [BASICS/clb_numxtrees.c](../../../eprover/BASICS/clb_numxtrees.c)

## Purpose

Definitions for SPLAY trees with long integer keys and vectors of IntOrPs as values. Copied from clb_numtrees.h the GNU Lesser General Public License. <1> Mon Aug 1 11:04:53 CEST 2011

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `NumXTreeCell`
- `NumXTree_p`

### Macros And Constants

- `CLB_NUMXTREES`
- `NUMXTREEVALUES`
- `NumXTreeCellAlloc()`
- `NumXTreeCellFree(junk)`
- `NumXTreeMaxKey(tree)`
- `NumXTreeTraverseExit(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `NumXTree_p NumXTreeCellAllocEmpty(void)`
- `NumXTree_p NumXTreeExtractEntry(NumXTree_p *root, long key)`
- `NumXTree_p NumXTreeExtractRoot(NumXTree_p *root)`
- `NumXTree_p NumXTreeFind(NumXTree_p *root, long key)`
- `NumXTree_p NumXTreeInsert(NumXTree_p *root, NumXTree_p newnode)`
- `NumXTree_p NumXTreeMaxNode(NumXTree_p root)`
- `PStack_p NumXTreeLimitedTraverseInit(NumXTree_p root, long limit)`
- `bool NumXTreeDeleteEntry(NumXTree_p *root, long key)`
- `bool NumXTreeStore(NumXTree_p *root, long key, IntOrP val1, IntOrP val2)`
- `long NumXTreeNodes(NumXTree_p root)`
- `void NumXTreeFree(NumXTree_p junk)`

## Implementation Notes

### Internal Functions

- `splay_tree`

### Source-Level Behavior

- `splay_tree`: Perform the splay operation on tree at node with key.
- `NumXTreeCellAllocEmpty`: Allocate a empty, initialized NumXTreeCell. Pointers to children are NULL, int values are 0 (and pointer values in ANSI-World undefined, in practice NULL on 32 bit machines)(This comment is superfluous!). The balance field is (correctly) set to 0.
- `NumXTreeFree`: Free a numtree (including the keys, but not potential objects pointed to in the val fields
- `NumXTreeInsert`: If an entry with key *newnode->key exists in the tree return a pointer to it. Otherwise insert *newnode in the tree and return NULL.
- `NumXTreeStore`: Insert a cell associating key with val1 and val2 into the tree. Return false if an entry for this key exists, true otherwise. Values beyond the second are zero.
- `NumXTreeFind`: Find the entry with key key in the tree and return it. Return NULL if no such key exists.
- `NumXTreeExtractEntry`: Find the entry with key key, remove it from the tree, rebalance the tree, and return the pointer to the removed element. Return NULL if no matching element exists.
- `NumXTreeExtractRoot`: Extract the NumXTreeCell at the root of the tree and return it (or NULL if the tree is empty).
- `NumXTreeDeleteEntry`: Delete the entry with key key from the tree.
- `NumXTreeNodes`: Return the number of nodes in the tree.
- `NumXTreeMaxNode`: Return the node with the largest key in the tree (or NULL if tree is empty). Non-destructive/non-reorganizing.
- `NumXTreeLimitedTraverseInit`: Return a stack containing the path to the smallest element smaller than or equal to limit in the tree.

### Dependencies

- `"clb_numxtrees.h"`
- `<clb_avlgeneric.h>`
- `<clb_dstrings.h>`

### Compile-Time Conditions

- `CLB_NUMXTREES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_numxtrees.h`, `BASICS/clb_numxtrees.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 610 lines, 13 scanned public declarations, 1 scanned internal function definitions, and 12 structured function-comment blocks.
- Definitions for SPLAY trees with long integer keys and vectors of IntOrPs as values. Copied from clb_numtrees.h the GNU Lesser General Public License. <1> Mon Aug 1 11:04:53 CEST 2011
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
