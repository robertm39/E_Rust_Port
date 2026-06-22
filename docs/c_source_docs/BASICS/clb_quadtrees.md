<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_quadtrees

## Source Files

- [BASICS/clb_quadtrees.h](../../../eprover/BASICS/clb_quadtrees.h)
- [BASICS/clb_quadtrees.c](../../../eprover/BASICS/clb_quadtrees.c)

## Purpose

Trees indexed by 4 words (two pointers and two integers). See clb_ptrees.h (and below) for details. the GNU Lesser General Public License. <1> Tue Jan 4 00:53:38 MET 2000

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `QuadKey`
- `QuadKeyCell`
- `QuadKey_p`
- `QuadTreeCell`
- `QuadTree_p`

### Macros And Constants

- `CLB_QUADTREES`
- `QuadTreeCellAlloc()`
- `QuadTreeCellFree(junk)`
- `QuadTreeTraverseExit(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `QuadTree_p QuadTreeExtractEntry(QuadTree_p *root, QuadKey_p key)`
- `QuadTree_p QuadTreeFind(QuadTree_p *root, QuadKey_p key)`
- `QuadTree_p QuadTreeInsert(QuadTree_p *root, QuadTree_p newnode)`
- `bool QuadTreeDeleteEntry(QuadTree_p *root, QuadKey_p key)`
- `bool QuadTreeStore(QuadTree_p *root, QuadKey_p key, IntOrP val)`
- `int DoubleKeyCmp(void* p1, int i1, void *p2, int i2)`
- `int QuadKeyCmp(QuadKey_p p1, QuadKey_p p2)`
- `void QuadTreeFree(QuadTree_p junk)`

## Implementation Notes

### Internal Functions

- `splay_tree`

### Source-Level Behavior

- `splay_tree`: Perform the splay operation on tree at node with key.
- `DoubleKeyCmp`: Compare two pointer/integer pairs.
- `QuadKeyCmp`: Compare two QuadKeys.
- `QuadTreeFree`: Free a QuadTree (including the keys, but not potential objects pointed to in the val fields
- `QuadTreeInsert`: If an entry with key *newnode->key exists in the tree return a pointer to it. Otherwise insert *newnode in the tree and return NULL. Will splay the tree!
- `QuadTreeStore`: Insert a cell with given key into the tree. Return false if an entry for this key exists, true otherwise. The key is never freed!
- `QuadTreeFind`: Find the entry with key key in the tree and return it. Return NULL if no such key exists.
- `QuadTreeExtractEntry`: Find the entry with key key, remove it from the tree, rebalance the tree, and return the pointer to the removed element. Return NULL if no matching element exists.
- `QuadTreeDeleteEntry`: Delete the entry with key key from the tree. Return true, if the key existed, false otherwise.

### Dependencies

- `"clb_quadtrees.h"`
- `<clb_ptrees.h>`

### Compile-Time Conditions

- `CLB_QUADTREES`

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

Source files reviewed: `BASICS/clb_quadtrees.h`, `BASICS/clb_quadtrees.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 499 lines, 13 scanned public declarations, 1 scanned internal function definitions, and 9 structured function-comment blocks.
- Trees indexed by 4 words (two pointers and two integers). See clb_ptrees.h (and below) for details. the GNU Lesser General Public License. <1> Tue Jan 4 00:53:38 MET 2000
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
