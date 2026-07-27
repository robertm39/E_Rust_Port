<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_ptrees

## Source Files

- [BASICS/clb_ptrees.h](../../../eprover/BASICS/clb_ptrees.h)
- [BASICS/clb_ptrees.c](../../../eprover/BASICS/clb_ptrees.c)

## Purpose

Data structures for the efficient management of pointer sets. I substituted this SPLAY tree version as it consumes less memory and may even be faster in the average case. As pointers are managed, all additional information can go into the pointed-to structures.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PTreeCell`
- `PTree_p`

### Macros And Constants

- `CLB_PTREES`
- `PCmp(p1, p2)`
- `PEqual(p1,p2)`
- `PGreater(p1,p2)`
- `PLesser(p1,p2)`
- `PTREE_CELL_MEM`
- `PTreeCellAlloc()`
- `PTreeCellFree(junk)`
- `PTreeTraverseExit(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `PTree_p PTreeCellAllocEmpty(void)`
- `PTree_p PTreeCopy(PTree_p tree1)`
- `PTree_p PTreeExtractEntry(PTree_p *root, void* key)`
- `PTree_p PTreeFind(PTree_p *root, void* key)`
- `PTree_p PTreeFindBinary(PTree_p root, void* key)`
- `PTree_p PTreeInsert(PTree_p *root, PTree_p newnode)`
- `PTree_p PTreeIntersection(PTree_p tree1, PTree_p tree2)`
- `bool PTreeDeleteEntry(PTree_p *root, void* key)`
- `bool PTreeEquiv(PTree_p t1, PTree_p t2)`
- `bool PTreeIsSubset(PTree_p sub, PTree_p *super)`
- `bool PTreeMerge(PTree_p *root, PTree_p add)`
- `bool PTreeStore(PTree_p *root, void* key)`
- `long PStackToPTree(PTree_p *root, PStack_p stack)`
- `long PTreeDebugPrint(FILE* out, PTree_p root)`
- `long PTreeDestrIntersection(PTree_p *tree1, PTree_p tree2)`
- `long PTreeNodes(PTree_p root)`
- `long PTreeToPStack(PStack_p target_stack, PTree_p root)`
- `static inline int PCmpFun(const void* p1, const void*p2)`
- `void PTreeFree(PTree_p junk)`
- `void PTreeInsertTree(PTree_p *root, PTree_p add)`
- `void PTreeVisitInOrder(PTree_p t, void (*visitor)(void*, void*), void* arg)`
- `void* PTreeExtractKey(PTree_p *root, void* key)`
- `void* PTreeExtractRootKey(PTree_p *root)`
- `void* PTreeSharedElement(PTree_p *tree1, PTree_p tree2)`

## Implementation Notes

### Internal Functions

- `PCmpFun`
- `splay_ptree`

### Source-Level Behavior

- `PCmpFun`: Compare two pointers, return 1 if the first one is bigger, 0 if both are equal, and -1 if the second one is bigger.
- `splay_tree`: Perform the splay operation on tree at node with key.
- `PTreeCellAllocEmpty`: Allocate a empty, initialized PTreeCell. Pointers to children are NULL, int values are 0 (and pointer values in ANSI-World undefined, in practice NULL on 32 bit machines)(This comment is superfluous!). The balance field is (correctly) set to 0.
- `PTreeFree`: Free a PTree (including the keys, but not potential objects pointed to in the val fields
- `PTreeInsert`: If an entry with key *newnode->key exists in the tree return a pointer to it. Otherwise insert *newnode in the tree and return NULL. Will splay the tree!
- `PTreeStore`: Insert a cell with given key into the tree. Return false if an entry for this key exists, true otherwise.
- `PTreeFind`: Find the entry with key key in the tree and return it. Return NULL if no such key exists.
- `PTreeFindBinary`: Find an entry by simple binary search. This does not reorganize the tree, otherwise it is inferior to PTreeFind()!
- `PTreeExtractEntry`: Find the entry with key key, remove it from the tree, rebalance the tree, and return the pointer to the removed element. Return NULL if no matching element exists.
- `PTreeExtractKey`: Extract the entry with key key, delete the PTree-Node and return the key.
- `PTreeExtractRootKey`: Extract the root node of the tree, delete it and return the key. Return NULL if the tree is empty.
- `PTreeDeleteEntry`: Delete the entry with key key from the tree. Return true, if the key existed, false otherwise.
- `PTreeMerge`: Merge the two trees, i.e. destroy the second one and add its elements to the first one. Return true if *root gains a new element.
- `PTreeInsertTree`: Insert the elements stored in add into *root. The tree at add remains unchanged.
- `PTreeNodes`: Return the number of nodes in the tree.
- `PTreeDebugPrint`: Print the keys stored in the tree. Returns number of nodes (why not ?).
- `PStackToPTree`: Interprete a stack as a list of pointers and insert these pointers into the tree at *root. Returns number of new elements in the tree.
- `PTreeToPStack`: Push all the keys in the tree onto the stack (in arbitrary order). Return number of values pushed.
- `PTreeSharedElement`: If there exists an element common in both trees, return the first one found. Otherwise return NULL. This iterates over the elements of the second tree and searches in the first, so make the second one smaller if you have a choice.
- `PTreeIntersection`: Compute the intersection of the two PTrees and return it.
- `PTreeCopy`: Return a Ptree that stores the same elements as tree.
- `PTreeEquiv`: Determin if two PTrees contain the same pointers.
- `PTreeIsSubset`: Determine if pointers stored in sub are a subset of pointers stored in super.
- `PTreeVisitInOrder`: Apply function visitor to every key stored in PTree t. Nodes will be visited as in inorder traversal. "arg" is an additional (first) arg to the visitor function.

### Dependencies

- `"clb_ptrees.h"`
- `<clb_avlgeneric.h>`
- `<clb_pstacks.h>`
- `<stdint.h>`

### Compile-Time Conditions

- `CLB_PTREES`
- `CONSTANT_MEM_ESTIMATE`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_ptrees.h`, `BASICS/clb_ptrees.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 1071 lines, 26 scanned public declarations, 2 scanned internal function definitions, and 25 structured function-comment blocks.
- Data structures for the efficient management of pointer sets. I substituted this SPLAY tree version as it consumes less memory and may even be faster in the average case. As pointers are managed, all additional information can go into the pointed-to structures.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `PTreeCell` stores `key` last to work around an old GCC/memory-manager interaction described in the header. Rust does not mirror that field-layout workaround; keep any future raw-pointer arena representation behind a documented compatibility boundary instead of carrying the historical layout into safe code.
- `PTreeToPStack` and `PTreeDebugPrint` use root-right-left explicit-stack traversal even though the C comment calls the order arbitrary. Rust now preserves the actual top-down splay topology with safe vector-index links and keeps sorted traversal separate. The resulting C order still depends on raw pointer values and temporary lookup history; a cleaned container API should distinguish unordered identity-set use from callers that deliberately consume compatibility traversal order.
- `PTreeMerge` destroys its `add` tree while `PTreeInsertTree` leaves the same pointer-shaped argument alive; that ownership distinction exists only in comments and caller convention. Rust preserves it with a by-value consuming merge and a borrowed insertion API. A later C API should make transfer explicit in the type/signature or replace the pair with separately named move/copy operations.
<!-- END MANUAL REVIEW: c_source_docs -->
