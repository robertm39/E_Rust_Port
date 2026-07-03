<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_objtrees

## Source Files

- [BASICS/clb_objtrees.h](../../../eprover/BASICS/clb_objtrees.h)
- [BASICS/clb_objtrees.c](../../../eprover/BASICS/clb_objtrees.c)

## Purpose

Data structures for the efficient management of objects represented by pointers. This inherits the ptree structure, but uses comparison on objects (by a user-provided comparison function) instead of pointer comparisons.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ObjDelFun`
- `PObjTree_p`

### Macros And Constants

- `CLB_OBJTREES`

### Globals

- None found in the source scan.

### Exported Functions

- `PObjTree_p PTreeObjExtractEntry(PObjTree_p *root, void* key, ComparisonFunctionType cmpfun)`
- `PObjTree_p PTreeObjFind(PObjTree_p *root, void* key, ComparisonFunctionType cmpfun)`
- `PObjTree_p PTreeObjFindBinary(PObjTree_p root, void* key, ComparisonFunctionType cmpfun)`
- `PObjTree_p PTreeObjInsert(PObjTree_p *root, PObjTree_p newnode, ComparisonFunctionType cmpfun)`
- `long PObjTreeNodes(PObjTree_p root)`
- `void DummyObjDelFun(void* Junk)`
- `void PObjTreeFree(PObjTree_p root, ObjDelFun del_fun)`
- `void PTreeObjMerge(PObjTree_p *root, PObjTree_p add, ComparisonFunctionType cmpfun)`
- `void* PTreeObjExtractObject(PObjTree_p *root, void* key, ComparisonFunctionType cmpfun)`
- `void* PTreeObjExtractRootObject(PObjTree_p *root, ComparisonFunctionType cmpfun)`
- `void* PTreeObjFindObj(PObjTree_p *root, void* key, ComparisonFunctionType cmpfun)`
- `void* PTreeObjStore(PObjTree_p *root, void* key, ComparisonFunctionType cmpfun)`

## Implementation Notes

### Internal Functions

- `splay_tree`

### Source-Level Behavior

- `splay_tree`: Perform the splay operation on tree at node with key.
- `PTreeObjInsert`: If an entry with cmpfun(*root->key, newnode->key) == 0 exists in the tree return a pointer to it. Otherwise insert *newnode in the tree and return NULL. Will splay the tree!
- `PTreeObjStore`: Store object in the tree. If an object that is equal to obj already exists in the tree, return it, otherwise return NULL. otherwise.
- `PTreeObjFind`: Find the entry with key key in the tree and return it. Return NULL if no such key exists.
- `PTreeObjFindObj`: Find and return object matching key (if any), return NULL if none.
- `PTreeObjFindBinary`: Find the entry with key key in the tree and return it. Return NULL if no such key exists. Does not reorganize the tree!
- `PTreeObjExtractEntry`: Find the entry with key key, remove it from the tree, rebalance the tree, and return the pointer to the removed element. Return NULL if no matching element exists.
- `PTreeObjExtractObject`: Extract the entry object, delete the PTree-Node and return the pointer to the object.
- `PTreeObjExtractRootObject`: Extract the root node of the tree, delete it and return the key. Return NULL if the tree is empty.
- `PTreeObjMerge`: Merge the two trees, i.e. destroy the second one and add its element to the first one.
- `PObjTreeFree`: Free a PObjTree, including the objects.
- `PObjTreeNodes`: Return the number of nodes in the tree.
- `DummyObjDelFun`: Do nothing, with a pointer ;-)

### Dependencies

- `"clb_objtrees.h"`
- `<clb_ptrees.h>`

### Compile-Time Conditions

- `CLB_OBJTREES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_objtrees.h`, `BASICS/clb_objtrees.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 602 lines, 14 scanned public declarations, 1 scanned internal function definitions, and 13 structured function-comment blocks.
- Data structures for the efficient management of objects represented by pointers. This inherits the ptree structure, but uses comparison on objects (by a user-provided comparison function) instead of pointer comparisons.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `PTreeObjMerge` asserts that inserted objects are not already present, but in release builds the returned duplicate node is only assigned to an unused local and is not freed. Rust enforces the disjoint-merge precondition with an assertion instead of reproducing the leak-shaped release path; a cleaned API should keep this as an explicit `Result` or prevalidated merge operation.
<!-- END MANUAL REVIEW: c_source_docs -->
