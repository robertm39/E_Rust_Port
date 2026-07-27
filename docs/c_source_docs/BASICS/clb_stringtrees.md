<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_stringtrees

## Source Files

- [BASICS/clb_stringtrees.h](../../../eprover/BASICS/clb_stringtrees.h)
- [BASICS/clb_stringtrees.c](../../../eprover/BASICS/clb_stringtrees.c)

## Purpose

Definitions for AVL trees with string keys and up to two int or pointer values. Part of the implementation is based on public domain code by D.D. Sleator. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `StrTreeCell`
- `StrTree_p`

### Macros And Constants

- `CLB_STRINGTREES`
- `StrTreeCellAlloc()`
- `StrTreeCellFree(junk)`
- `StrTreeTraverseExit(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `StrTree_p StrTreeCellAllocEmpty(void)`
- `StrTree_p StrTreeExtractEntry(StrTree_p *root, const char* key)`
- `StrTree_p StrTreeFind(StrTree_p *root, const char* key)`
- `StrTree_p StrTreeInsert(StrTree_p *root, StrTree_p newnode)`
- `StrTree_p StrTreeStore(StrTree_p *root, char* key, IntOrP val1, IntOrP val2)`
- `bool StrTreeDeleteEntry(StrTree_p *root, const char* key)`
- `void StrTreeFree(StrTree_p junk)`

## Implementation Notes

### Internal Functions

- `splay_tree`

### Source-Level Behavior

- `splay_tree`: Perform the splay operation on tree at node with key.
- `StrTreeCellAllocEmpty`: Allocate a empty, initialized StrTreeCell. Pointers to children are NULL, int values are 0 (and pointer values in ANSI-World undefined, in practice NULL on 32 bit machines)(This comment is superfluous!).
- `StrTreeFree`: Free a stringtree (including the keys, but not potential objects pointed to in the val fields
- `StrTreeInsert`: If an entry with key *newnode->key exists in the tree return a pointer to it. Otherwise insert *newnode in the tree and return NULL.
- `StrTreeStore`: Insert a cell associating key with val1 and val2 into the tree. Return NULL if an entry for this key exists, address of the new node otherwise.
- `StrTreeFind`: Find the entry with key key in the tree and return it. Return NULL if no such key exists.
- `StrTreeExtractEntry`: Find the entry with key key, remove it from the tree, rebalance the tree, and return the pointer to the removed element. Return NULL if no matching element exists.
- `StrTreeDeleteEntry`: Delete the entry with key key from the tree.

### Dependencies

- `"clb_stringtrees.h"`
- `<clb_avlgeneric.h>`
- `<clb_dstrings.h>`

### Compile-Time Conditions

- `CLB_STRINGTREES`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-07-18.

Source files reviewed: `BASICS/clb_stringtrees.h`, `BASICS/clb_stringtrees.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 465 lines, 9 scanned public declarations, 1 scanned internal function definitions, and 8 structured function-comment blocks.
- Definitions for AVL trees with string keys and up to two int or pointer values. Part of the implementation is based on public domain code by D.D. Sleator. the GNU Lesser General Public License.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- The top-down splay routine reorganizes the tree on duplicate insertion, successful lookup, nearest miss, and successful or failed extraction. Rust now preserves the exact root/child topology with safe arena indices and free-slot reuse.
- `StrTreeStore` duplicates the caller's key and the tree owns that copy. Rust likewise stores an owned `String`; extraction transfers the stored key and values without cloning them.
- C `strcmp` compares unsigned bytes and terminates at the first NUL. Rust preserves that ordering for valid UTF-8 owner strings, truncates stored keys at an embedded NUL, and ignores query suffixes after NUL. The safe `&str` API intentionally excludes arbitrary invalid-UTF-8 C byte strings; none of the four direct Rust owners supplies such keys.
- Exact unchanged-C topology, embedded-NUL, non-ASCII byte-order, and C/Rust owner evidence is retained in [`experiments/2026-07-18-119-strtree-splay-topology`](../../../experiments/2026-07-18-119-strtree-splay-topology/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- The generic safe Rust API accepts UTF-8 `&str`, while raw C `char*` keys can contain arbitrary nonzero bytes. If a future direct Rust owner needs opaque filesystem or protocol bytes, add a byte-key boundary for that owner without changing the proven `strcmp` topology.
<!-- END MANUAL REVIEW: c_source_docs -->
