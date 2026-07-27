<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_pdarrays

## Source Files

- [BASICS/clb_pdarrays.h](../../../eprover/BASICS/clb_pdarrays.h)
- [BASICS/clb_pdarrays.c](../../../eprover/BASICS/clb_pdarrays.c)

## Purpose

Dynamic arrays of pointers and long integers. You can define the growth behaviour by specifying a value. If it is GROW_EXPONENTIAL, arrays will always grow by a factor that is the lowest power of two that will make the array big enough. Otherwise it will grow by the smallest multiple of the value specified that creates the requested position.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PDArrayCell`
- `PDArray_p`

### Macros And Constants

- `CLB_PDARRAYS`
- `GROW_EXPONENTIAL`
- `PDARRAYCELL_MEM`
- `PDArrayAssign(array, idx, value)`
- `PDArrayAssignInt(array, idx, value)`
- `PDArrayAssignP(array, idx, value)`
- `PDArrayCellAlloc()`
- `PDArrayCellFree(junk)`
- `PDArrayElement(array, idx)`
- `PDArrayElementClear(arr, idx)`
- `PDArrayElementInt(array, idx)`
- `PDArrayElementP(array, idx)`
- `PDArraySize(array)`
- `PDArrayStorage(arr)`

### Globals

- None found in the source scan.

### Exported Functions

- `PDArray_p PDArrayAlloc(long init_size, long grow)`
- `PDArray_p PDArrayCopy(PDArray_p array)`
- `PDArray_p PDIntArrayAlloc(long init_size, long grow)`
- `long PDArrayElementIncInt(PDArray_p array, long idx, long value)`
- `long PDArrayFirstUnused(PDArray_p array)`
- `long PDArrayStore(PDArray_p array, IntOrP value)`
- `long PDArrayStoreInt(PDArray_p array, long value)`
- `long PDArrayStoreP(PDArray_p array, void* value)`
- `static inline IntOrP* PDArrayElementRef(PDArray_p array, long idx)`
- `void PDArrayAdd(PDArray_p collect, PDArray_p data, long limit)`
- `void PDArrayElementDeleteInt(PDArray_p array, long idx)`
- `void PDArrayElementDeleteP(PDArray_p array, long idx)`
- `void PDArrayEnlarge(PDArray_p array, long idx)`
- `void PDArrayFree(PDArray_p junk)`

## Implementation Notes

### Internal Functions

- `PDArrayElementRef`

### Source-Level Behavior

- `PDArrayElementRef`: Return a reference to an element in a dynamic array. This reference is only good until the next call to this function! User programs are expected to use this function only extremely rarely and with special care. Use PDArrayElement()/PDArrayAssign() instead.
- `PDArrayAlloc`: Return an initialized dynamic array of size init_size where all elements are interpreted as pointers and initialized to NULL.
- `PDIntArrayAlloc`: Return an initialized dynamic array of size init_size where all elements are interpreted as (long) integers and initialized to 0.
- `PDArrayFree`: Free a PDArray. Leaves elements untouched.
- `PDArrayEnlarge`: Enlarge array enough to accomodate index.
- `PDArrayCopy`: Copy a PDArray with contents. Use with care, as some data structures may not be copyable very well (e.g. pointers to the same array, registered references, ...)
- `PDArrayElementDeleteP`: If idx is within the currently allocated array, set the value to NULL. Otherwise do nothing.
- `PDArrayElementDeleteInt`: If idx is within the currently allocated array, set the value to 0. Otherwise do nothing.
- `PDArrayMembers`: Return number of non-NULL elements in the array.
- `PDArrayFirstUnused`: Return 1 + the index of the largest element != NULL in array (0 if the array is empty).
- `PDArrayStore`: Store the given value after the end of the used part of the array. This is similar to PStackPush() for stacks, but a LOT less efficient. Return value is the index assigned.
- `PDArrayStoreP`: Store the given pointer value after the end of the used part of / the array. See PDArrayStore().
- `PDArrayStoreInt`: Store the given long int value after the end of the used part of / the array. See PDArrayStore().
- `PDArrayAdd`: Add the first limit elements from new to the corresponding entries in collect. All entries are interpreted as numerical.
- `PDArrayElementIncInt`: Increment entry indexed in array by value. Return new value.

### Dependencies

- `"clb_pdarrays.h"`
- `<clb_memory.h>`

### Compile-Time Conditions

- `CLB_PDARRAYS`
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

Source files reviewed: `BASICS/clb_pdarrays.h`, `BASICS/clb_pdarrays.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 570 lines, 16 scanned public declarations, 1 scanned internal function definitions, and 15 structured function-comment blocks.
- Dynamic arrays of pointers and long integers. You can define the growth behaviour by specifying a value. If it is GROW_EXPONENTIAL, arrays will always grow by a factor that is the lowest power of two that will make the array big enough. Otherwise it will grow by the smallest multiple of the value specified that creates the requested posi...
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `PDArrayElementRef` asserts that indices are nonnegative before growing the backing array, and the element/assignment/integer-increment macros inherit that assertion while always succeeding for nonnegative indices.
- `PDArrayEnlarge` is exported but is normally reached through `PDArrayElementRef` only for uncovered indices. A direct call on an already covered fixed-growth array can compute a smaller target size before copying the old allocation. Rust exposes an explicit raw compatibility helper for the target-size calculation, but reports that under-allocation case as a panic instead of reproducing the C buffer overrun.
- `PDArrayElementClear` writes directly to `array[idx].p_val` without calling `PDArrayElementRef`, so it neither grows the array nor checks bounds. Rust exposes this as a raw pointer-slot clear helper for already allocated indices and panics on invalid indices instead of reproducing the C out-of-bounds write.
- `PDArrayAdd` takes a signed `long limit` and uses a plain `for(i=0; i<limit; i++)` loop, so zero and negative limits perform no work. Rust keeps that exact loop surface in a signed compatibility helper while leaving the existing `usize` helper for ordinary prefix addition.

### Change Later

- Negative `PDArray` access is assertion failure behavior in C. The compatibility-shaped Rust methods should keep panicking, while future Rust-only checked accessors should be separate wrappers instead of weakening the C-shaped array API.
- Direct `PDArrayEnlarge` calls rely on an implicit uncovered-index precondition that the function itself does not assert. A cleaned API should hide the helper, assert the precondition, or route all growth through the element-ref/accessor path instead of preserving the hazardous direct-call shape.
- `PDArrayElementClear` bypasses the otherwise central grow-on-access path and can write outside the allocation if a caller passes an uncovered index. Prefer the checked delete helper for ordinary Rust call paths and keep the raw clear helper quarantined for compatibility-only code.
- `PDArrayAdd` silently treats nonpositive limits as empty prefixes because of its signed loop condition. A cleaned Rust-facing API should keep using an unsigned prefix length or return a validation error for negative caller input.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
