<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_intmap

## Source Files

- [BASICS/clb_intmap.h](../../../eprover/BASICS/clb_intmap.h)
- [BASICS/clb_intmap.c](../../../eprover/BASICS/clb_intmap.c)

## Purpose

Definitions and functions for a data type that maps natural numbers (including 0) to void* pointers, supporting assignments, retrieval, deletion, and iteration. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `IntMapCell`
- `IntMapFreeFunc`
- `IntMapIterCell`
- `IntMapIter_p`
- `IntMapType`
- `IntMap_p`
- `admin_data`
- `values`

### Macros And Constants

- `CLB_INTMAP`
- `IM_ARRAY_SIZE`
- `INTMAPCELL_MEM`
- `IntMapCellAlloc()`
- `IntMapCellFree(junk)`
- `IntMapDStorage(map)`
- `IntMapIterCellAlloc()`
- `IntMapIterCellFree(junk)`
- `IntMapStorage(map)`
- `MAX_TREE_DENSITY`
- `MIN_TREE_DENSITY`

### Globals

- None found in the source scan.

### Exported Functions

- `IntMap_p IntMapAlloc(void)`
- `static inline void* IntMapIterNext(IntMapIter_p iter, long *key)`
- `void IntMapAssign(IntMap_p map, long key, void* value)`
- `void IntMapDebugPrint(FILE* out, IntMap_p map)`
- `void IntMapFree(IntMap_p map)`
- `void IntMapIterFree(IntMapIter_p junk)`
- `void* IntMapDelKey(IntMap_p map, long key)`
- `void* IntMapGetVal(IntMap_p map, long key)`
- `void** IntMapGetRef(IntMap_p map, long key)`

## Implementation Notes

### Internal Functions

- `IntMapIterNext`
- `add_new_tree_node`
- `array_to_tree`
- `switch_to_array`
- `switch_to_tree`
- `tree_to_array`

### Source-Level Behavior

- `IntMapIterNext`: Return the next value/key pair in the map (or NULL/ndef) if the iterator is exhausted.
- `switch_to_array`: Return true if representation should switch to array (because of high density)
- `switch_to_tree`: Return true if representation should switch to tree (because of low density)
- `add_new_tree_node`: Add a *new* key node to a IntMap in tree form and return its address. Assertion fail, if key is not new. Increases element count!
- `array_to_tree`: Convert a IntMap in array form to an equivalent one in tree form.
- `tree_to_array`: Convert a IntMap in tree form to an equivalent one in array form.
- `IntMapAlloc`: Allocate an empty int mapper.
- `IntMapFree`: Free an int mapper (does _not_ free pointed-to elements).
- `IntMapGetVal`: Given a key, return the associated value or NULL, if no suitable key/value pair exists.
- `IntMapGetRef`: Get a reference to the address of the value of a key/value pair. Note that this always creates the key value pair (with empty value) if it does not exist yet.
- `IntMapAssign`: Add key/value pair to map, overriding any previous association.
- `IntMapDelKey`: Delete a key/value association. If there was one, return the value, otherwise return NULL. **Currently, arrays never shrink. This might be worth **changing (unlikely, though).
- `IntMapIterAlloc`: Allocate an iterator object iterating over key range lower_key to upper_key (both inclusive) in map. This is only valid as long as no new key is introduced or old key is deleted.
- `IntMapIterFree`: Free an IntMapIterator.
- `IntMapDebugPrint`: Print an intmap datatype as a list of key:value pairs.

### Dependencies

- `"clb_intmap.h"`
- `<clb_numtrees.h>`
- `<clb_pdrangearrays.h>`
- `<limits.h>`

### Compile-Time Conditions

- `CLB_INTMAP`
- `CONSTANT_MEM_ESTIMATE`

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

Source files reviewed: `BASICS/clb_intmap.h`, `BASICS/clb_intmap.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 915 lines, 17 scanned public declarations, 6 scanned internal function definitions, and 15 structured function-comment blocks.
- Definitions and functions for a data type that maps natural numbers (including 0) to void* pointers, supporting assignments, retrieval, deletion, and iteration. the GNU Lesser General Public License.
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
