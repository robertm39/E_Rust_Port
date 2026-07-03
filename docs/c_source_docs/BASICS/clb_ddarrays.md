<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_ddarrays

## Source Files

- [BASICS/clb_ddarrays.h](../../../eprover/BASICS/clb_ddarrays.h)
- [BASICS/clb_ddarrays.c](../../../eprover/BASICS/clb_ddarrays.c)

## Purpose

Dynamic arrays of large data types - at the moment doubles only. the GNU Lesser General Public License. <1> Sun Aug 8 22:45:29 GMT 1999 Copied from clb_pdarrays.h

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DDArrayCell`
- `DDArray_p`

### Macros And Constants

- `CLB_DDARRAYS`
- `DDArrayAssign(array, idx, value)`
- `DDArrayCellAlloc()`
- `DDArrayCellFree(junk)`
- `DDArrayElement(array, idx)`

### Globals

- None found in the source scan.

### Exported Functions

- `DDArray_p DDArrayAlloc(long init_size, long grow)`
- `double DDArraySelectPart(DDArray_p array, double part, long size)`
- `static inline double* DDArrayElementRef(DDArray_p array, long idx)`
- `void DDArayEnlarge(DDArray_p array, long idx)`
- `void DDArrayFree(DDArray_p junk)`

## Implementation Notes

### Internal Functions

- `DDArrayElementRef`

### Source-Level Behavior

- `DDArrayElementRef`: Return a reference to an element in a dynamic array. This reference is only good until the next call to this function! User programs are expected to use this function only extremely rarely and with special care. Use DDArrayElement()/DDArrayAssign() instead.
- `DDArrayAlloc`: Return an initialized dynamic array of size init_size where all elements are interpreted as pointers and initialized to NULL.
- `DDArrayFree`: Free a DDArray. Leaves elements untouched.
- `DDArayEnlarge`: Enlarge array enough to accomodate idx.
- `DDArrayDebugPrint`: Print the array, only for debugging.
- `DDArrayAdd`: Add the first limit elements from new to the corresponding entries in collect. All entries are interpreted as numerical.
- `DDArraySelectPart`: Find a value d with at least part*(last+1) values >= d and (1-part)*(last+1) values <= d in array.

### Dependencies

- `"clb_ddarrays.h"`
- `<clb_memory.h>`

### Compile-Time Conditions

- `CLB_DDARRAYS`

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

Source files reviewed: `BASICS/clb_ddarrays.h`, `BASICS/clb_ddarrays.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 397 lines, 7 scanned public declarations, 1 scanned internal function definitions, and 7 structured function-comment blocks.
- Dynamic arrays of large data types - at the moment doubles only. the GNU Lesser General Public License. <1> Sun Aug 8 22:45:29 GMT 1999 Copied from clb_pdarrays.h
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `DDArrayDebugPrint` calls `DDArrayElement` for every printed position, so asking it to print beyond the current allocation enlarges and zero-fills the array as a side effect. Rust preserves this in the explicit debug-string helper.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
