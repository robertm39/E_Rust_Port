<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_numtrees

## Source Files

- [BASICS/clb_numtrees.h](../../../eprover/BASICS/clb_numtrees.h)
- [BASICS/clb_numtrees.c](../../../eprover/BASICS/clb_numtrees.c)

## Purpose

Definitions for SPLAY trees with long integer keys and up to two long or pointer values. Copied from clb_stringtrees.h the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `NumTreeCell`
- `NumTree_p`

### Macros And Constants

- `CLB_NUMTREES`
- `NUMTREECELL_MEM`
- `NumTreeCellAlloc()`
- `NumTreeCellFree(junk)`
- `NumTreeMaxKey(tree)`
- `NumTreeTraverseExit(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `NumTree_p NumTreeCellAllocEmpty(void)`
- `NumTree_p NumTreeExtractEntry(NumTree_p *root, long key)`
- `NumTree_p NumTreeExtractRoot(NumTree_p *root)`
- `NumTree_p NumTreeFind(NumTree_p *root, long key)`
- `NumTree_p NumTreeInsert(NumTree_p *root, NumTree_p newnode)`
- `NumTree_p NumTreeMaxNode(NumTree_p root)`
- `PStack_p NumTreeLimitedTraverseInit(NumTree_p root, long limit)`
- `bool NumTreeDeleteEntry(NumTree_p *root, long key)`
- `bool NumTreeStore(NumTree_p *root, long key, IntOrP val1, IntOrP val2)`
- `long NumTreeDebugPrint(FILE* out, NumTree_p tree, bool keys_only)`
- `long NumTreeNodes(NumTree_p root)`
- `void NumTreeFree(NumTree_p junk)`

## Implementation Notes

### Internal Functions

- `numtree_print`
- `splay_tree`

### Source-Level Behavior

- `numtree_print`: Print the tree with the appropriate indent level and return the number of nodes.
- `splay_tree`: Perform the splay operation on tree at node with key.
- `NumTreeCellAllocEmpty`: Allocate a empty, initialized NumTreeCell. Pointers to children are NULL, int values are 0 (and pointer values in ANSI-World undefined, in practice NULL on 32 bit machines)(This comment is superfluous!). The balance field is (correctly) set to 0.
- `NumTreeFree`: Free a numtree (including the keys, but not potential objects pointed to in the val fields
- `NumTreeInsert`: If an entry with key *newnode->key exists in the tree return a pointer to it. Otherwise insert *newnode in the tree and return NULL.
- `NumTreeStore`: Insert a cell associating key with val1 and val2 into the tree. Return false if an entry for this key exists, true otherwise.
- `NumTreeDebugPrint`: Print the tree in an unattractive but debug-friendly way.
- `NumTreeFind`: Find the entry with key key in the tree and return it. Return NULL if no such key exists.
- `NumTreeExtractEntry`: Find the entry with key key, remove it from the tree, rebalance the tree, and return the pointer to the removed element. Return NULL if no matching element exists.
- `NumTreeExtractRoot`: Extract the NumTreeCell at the root of the tree and return it (or NULL if the tree is empty).
- `NumTreeDeleteEntry`: Delete the entry with key key from the tree.
- `NumTreeNodes`: Return the number of nodes in the tree.
- `NumTreeMaxNode`: Return the node with the largest key in the tree (or NULL if tree is empty). Non-destructive/non-reorganizing.
- `NumTreeLimitedTraverseInit`: Return a stack containing the path to the smallest element smaller than or equal to limit in the tree.

### Dependencies

- `"clb_numtrees.h"`
- `"clb_simple_stuff.h"`
- `<clb_avlgeneric.h>`
- `<clb_dstrings.h>`

### Compile-Time Conditions

- `CLB_NUMTREES`
- `CONSTANT_MEM_ESTIMATE`

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

Source files reviewed: `BASICS/clb_numtrees.h`, `BASICS/clb_numtrees.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 669 lines, 14 scanned public declarations, 2 scanned internal function definitions, and 14 structured function-comment blocks.
- Definitions for SPLAY trees with long integer keys and up to two long or pointer values. Copied from clb_stringtrees.h the GNU Lesser General Public License.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- The top-down splay routine reorganizes the tree on duplicate insertion, successful lookup, nearest miss, and successful or failed extraction. Rust now preserves the exact root/child topology with safe arena indices; read-only `find_binary` is reserved for Rust APIs that only expose a shared reference.
- `NumTreeMaxNode` does not reorganize the tree. Full traversal remains ascending, while limited traversal initializes a logarithmic path to the first key greater than or equal to the supplied limit.
- `NumTreeDebugPrint` is a preorder topology dump rather than a sorted listing. It emits explicit `[]` children only when their parent has at least one child and advances by four visible spaces per tree level. Rust preserves that shape and prints implementation-native node addresses for the diagnostic pointer fields; exact pointer text is intentionally platform- and allocation-dependent.
- C compares `long` keys with signed subtraction, which has undefined behavior on overflow. Rust uses total `i64` comparison, matching the pinned LP64 reference for defined inputs while remaining defined at extreme keys.
- Exact unchanged-C topology/debug evidence and the direct owner inventory are retained in [`experiments/2026-07-18-117-numtree-splay-topology`](../../../experiments/2026-07-18-117-numtree-splay-topology/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `NumTreeLimitedTraverseInit` comments say it returns a path to the smallest element smaller than or equal to the limit, but the implementation skips keys below the limit and initializes traversal at the first key greater than or equal to it. Rust preserves the implemented behavior; the C comment should be corrected only after compatibility tests confirm no caller relied on the wording.
- `splay_tree` and insertion/extraction equality checks subtract signed `long` keys. Do not copy that undefined-overflow comparison idiom into Rust; if the C source is changed later, replace it with relational comparisons without changing ordinary-key topology.
<!-- END MANUAL REVIEW: c_source_docs -->
