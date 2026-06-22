<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_objmaps

## Source Files

- [BASICS/clb_objmaps.h](../../../eprover/BASICS/clb_objmaps.h)
- [BASICS/clb_objmaps.c](../../../eprover/BASICS/clb_objmaps.c)

## Purpose

Data structure for efficiently dealing with mapping a key to a value. You only need to provide a (total) comparison function on the keys and optionally a deleter function for keys. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `KeyValDelFun`
- `PObjMap_p`

### Macros And Constants

- `CLB_OBJMAPS`
- `PObjMapNodeAlloc()`
- `PObjMapNodeFree(junk)`
- `PObjMapTraverseExit(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `PStack_p PObjMapTraverseInit(PObjMap_p, PStack_p)`
- `size_t SizeOfPObjNode()`
- `void PObjMapFree(PObjMap_p root)`
- `void PObjMapFreeWDeleter(PObjMap_p root, KeyValDelFun del_fun)`
- `void* PObjMapExtract(PObjMap_p *root, void* key, ComparisonFunctionType cmpfun)`
- `void* PObjMapFind(PObjMap_p *root, void* key, ComparisonFunctionType cmpfun)`
- `void* PObjMapStore(PObjMap_p *root, void* key, void* value, ComparisonFunctionType cmpfun)`
- `void* PObjMapTraverseNext(PStack_p, void**)`
- `void** PObjMapGetRef(PObjMap_p *root, void* key, ComparisonFunctionType cmpfun, bool* updated)`

## Implementation Notes

### Internal Functions

- `splay_tree`

### Source-Level Behavior

- `splay_tree`: Perform the splay operation on tree at node with key.
- `do_extract_entry`: Find the entry with key key, remove it from the tree, rebalance the tree, and return the pointer to the removed element. Return NULL if no matching element exists. NB: Does not free the node.
- `PObjMapInsert`: If an entry with cmpfun(*root->key, newnode->key) == 0 exists in the tree return a pointer to it. Otherwise insert *newnode in the tree and return NULL. Will splay the tree!
- `PObjMapStore`: Stores a key value pair in the store. If a key already existed in the tree, the old value is returned. Else, NULL is returned. In either way, map is updated to store a mapping key -> value;
- `PObjMapGetRef`: Returns a reference to the value for the corresponding key. If the key was previously not stored, new node is created and reference to its "value" field is returned. updated_map is set to true if a new node was created [if you are not interested in this info, just pass NULL for updated_map].
- `PObjMapFind`: Finds a value associated to the key. If no such value exists, NULL is returned.
- `PObjMapExtract`: Finds a value associated to the key, deletes it and returns it. Returns NULL if no value is associated to the key.
- `PObjTreeFreeWDeleter`: Free the tree using the functions that frees keys and values.
- `PObjMapTraverseInit`: Initialize the interator. Unlike other iterator initializers, does not do memory managent on stacks.
- `PObjMapTraverseNext`: Traverses the nodes and returns value stored in each node. If you want to know the value of the key, use a non-NULL second argument,
- `SizeOfPObjNode`: Size of one PObjMap node object.

### Dependencies

- `"clb_objmaps.h"`
- `"clb_objtrees.h"`
- `<clb_avlgeneric.h>`
- `<clb_pstacks.h>`

### Compile-Time Conditions

- `CLB_OBJMAPS`

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

Source files reviewed: `BASICS/clb_objmaps.h`, `BASICS/clb_objmaps.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 558 lines, 12 scanned public declarations, 1 scanned internal function definitions, and 12 structured function-comment blocks.
- Data structure for efficiently dealing with mapping a key to a value. You only need to provide a (total) comparison function on the keys and optionally a deleter function for keys. the GNU Lesser General Public License.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
