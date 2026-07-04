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
- `DDArrayDebugPrint` takes a signed `long size`, loops while `i < size`, and always prints a final newline. Rust keeps a signed compatibility helper where zero or negative sizes produce only that trailing newline.
- `DDArrayElementRef` asserts that indices are nonnegative before growing the backing array, and the element/assignment macros inherit that assertion while always succeeding for nonnegative indices.
- `DDArraySelectPart` asserts that `part` is in the inclusive `[0, 1]` range, `size` is positive, and the allocated array already covers the requested prefix before partitioning the backing array in place.
- Rust keeps a signed `DDArraySelectPart` wrapper so negative C `long size` inputs hit the same assertion-shaped failure as zero-size inputs instead of being unrepresentable at the call boundary.
- `DDArayEnlarge` is exported with the historical misspelling and is normally reached through `DDArrayElementRef` only for uncovered indices. A direct call on an already covered index can compute a smaller target size before copying the old allocation. Rust exposes an explicit raw compatibility helper for the target-size calculation, but reports that under-allocation case as a panic instead of reproducing the C buffer overrun.
- `DDArrayAdd` takes a signed `long limit` and uses a plain `for(i=0; i<limit; i++)` loop, so zero and negative limits perform no work. Rust keeps that exact loop surface in a signed compatibility helper while leaving the existing `usize` helper for ordinary prefix addition.

### Change Later

- Negative `DDArray` access is assertion failure behavior in C. The compatibility-shaped Rust methods should keep panicking, while future Rust-only checked accessors should be separate wrappers instead of weakening the C-shaped array API.
- `DDArraySelectPart` treats invalid percentile/range requests as assertion failures. The compatibility-shaped Rust method should keep panicking, while user-facing statistics APIs should validate inputs before calling it or expose a separate checked wrapper.
- `DDArrayDebugPrint` silently treats nonpositive sizes as empty output plus the final newline because of its signed loop condition. A cleaned diagnostic helper should prefer an unsigned length or explicit validation.
- Direct `DDArayEnlarge` calls rely on an implicit uncovered-index precondition that the function itself does not assert. A cleaned API should hide the misspelled helper, assert the precondition, or route all growth through the element-ref/accessor path instead of preserving the hazardous direct-call shape.
- `DDArrayAdd` silently treats nonpositive limits as empty prefixes because of its signed loop condition. A cleaned Rust-facing API should keep using an unsigned prefix length or return a validation error for negative caller input.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
