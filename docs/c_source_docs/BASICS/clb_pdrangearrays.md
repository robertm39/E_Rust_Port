<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_pdrangearrays

## Source Files

- [BASICS/clb_pdrangearrays.h](../../../eprover/BASICS/clb_pdrangearrays.h)
- [BASICS/clb_pdrangearrays.c](../../../eprover/BASICS/clb_pdrangearrays.c)

## Purpose

Dynamic arrays of pointers and long integers with an index range defined by upper and lower bound. You can define the growth behaviour by specifying a value. If it is GROW_EXPONENTIAL, arrays will always grow by a factor that is the lowest power of two that will make the array big enough. Otherwise it will grow by the smallest multiple of the value specified that

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PDRangeArrCell`
- `PDRangeArr_p`

### Macros And Constants

- `CLB_PDRANGEARRAYS`
- `PDRANGEARRELL_MEM`
- `PDRANGEARR_MEM`
- `PDRangeArrAssign(array, idx, value)`
- `PDRangeArrAssignInt(array, idx, value)`
- `PDRangeArrAssignP(array, idx, value)`
- `PDRangeArrCellAlloc()`
- `PDRangeArrCellFree(junk)`
- `PDRangeArrElement(array, idx)`
- `PDRangeArrElementInt(array, idx)`
- `PDRangeArrElementP(array, idx)`
- `PDRangeArrIndexIsCovered(array, idx)`
- `PDRangeArrLimitKey(array)`
- `PDRangeArrLowKey(array)`
- `PDRangeArrStorage(arr)`

### Globals

- None found in the source scan.

### Exported Functions

- `PDRangeArr_p PDIntRangeArrAlloc(long idx, long grow)`
- `PDRangeArr_p PDRangeArrAlloc(long idx, long grow)`
- `PDRangeArr_p PDRangeArrCopy(PDRangeArr_p array)`
- `static inline IntOrP* PDRangeArrElementRef(PDRangeArr_p array, long idx)`
- `void PDRangeArrElementDeleteInt(PDRangeArr_p array, long idx)`
- `void PDRangeArrElementDeleteP(PDRangeArr_p array, long idx)`
- `void PDRangeArrEnlarge(PDRangeArr_p array, long idx)`
- `void PDRangeArrFree(PDRangeArr_p junk)`

## Implementation Notes

### Internal Functions

- `PDRangeArrElementRef`
- `range_arr_expand_down`
- `range_arr_expand_up`
- `range_arr_size`

### Source-Level Behavior

- `PDRangeArrElementRef`: Return a reference to an element in a dynamic array. This reference is only good until the next call to this function! User programs are expected to use this function only extremely rarely and with special care. Use PDRangeArrElement()/PDRangeArrAssign() instead.
- `range_arr_size`: Given the current size, growths model, and minimal new size, retunr the actual new size.
- `range_arr_expand_down`: Expand a range array down until it is big enough to accomodate idx.
- `range_arr_expand_up`: Expand a range array up until it is big enough to accomodate idx.
- `PDRangeArrAlloc`: Return an initialized dynamic array of size init_size where all elements are interpreted as pointers and initialized to NULL.
- `PDIntRangeArrAlloc`: Return an initialized dynamic array of size init_size where all elements are interpreted as (long) integers and initialized to 0.
- `PDRangeArrFree`: Free a PDRangeArr. Leaves elements untouched.
- `PDRangeArrEnlarge`: Enlarge array enough to accomodate index.
- `PDRangeArrCopy`: Copy a PDRangeArr with contents. Use with care, as some data structures may not be copyable very well (e.g. pointers to the same array, registered references, ...)
- `PDRangeArrElementDeleteP`: If idx is within the currently allocated array, set the value to NULL. Otherwise do nothing.
- `PDRangeArrElementDeleteInt`: If idx is within the currently allocated array, set the value to 0. Otherwise do nothing.
- `PDRangeArrMembers`: Return number of non-NULL elements in the array.
- `PDRangeArrElementIncInt`: Increment entry indexed in array by value. Return new value.

### Dependencies

- `"clb_pdrangearrays.h"`
- `<clb_pdarrays.h>`

### Compile-Time Conditions

- `CLB_PDRANGEARRAYS`
- `CONSTANT_MEM_ESTIMATE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_pdrangearrays.h`, `BASICS/clb_pdrangearrays.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 554 lines, 10 scanned public declarations, 4 scanned internal function definitions, and 13 structured function-comment blocks.
- Dynamic arrays of pointers and long integers with an index range defined by upper and lower bound. You can define the growth behaviour by specifying a value. If it is GROW_EXPONENTIAL, arrays will always grow by a factor that is the lowest power of two that will make the array big enough. Otherwise it will grow by the smallest multiple o...
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `PDRangeArrElementRef` grows the range when the index is outside the current `[low, limit)` window and then asserts that the resulting offset/size covers the requested index. The element, assignment, and integer-increment macros inherit that grow-then-return behavior and always expose a slot for representable indices.
- `PDRangeArrElementDeleteP` and `PDRangeArrElementDeleteInt` first test coverage and do nothing outside the current range, so deletion is checked while normal element access is mutating.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `PDRangeArrEnlarge` is exported but assertion-sensitive: the normal inline accessor calls it only for uncovered indices, while a direct call on an already covered index can take the wrong expansion branch. Rust keeps ordinary access C-compatible and safe to call, but a cleaned API should make raw expansion preconditions explicit or hide the helper.
- Reads through `PDRangeArrElement*` can allocate and shift the backing array even when the caller is only probing for a value. Preserve this for compatibility, especially for `IntMap`, but future Rust-only lookup APIs should use non-mutating checked accessors.
<!-- END MANUAL REVIEW: c_source_docs -->
