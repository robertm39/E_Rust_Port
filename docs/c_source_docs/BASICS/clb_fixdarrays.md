<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_fixdarrays

## Source Files

- [BASICS/clb_fixdarrays.h](../../../eprover/BASICS/clb_fixdarrays.h)
- [BASICS/clb_fixdarrays.c](../../../eprover/BASICS/clb_fixdarrays.c)

## Purpose

Rather trivial datatype for arrays of long integers with a known, fixed and and queryable size. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FixedDArrayCell`
- `FixedDArray_p`

### Macros And Constants

- `CLB_FIXDARRAYS`
- `FixedDArraySize(array)`
- `FixedDArraySub(dest, s1, s2)`

### Globals

- None found in the source scan.

### Exported Functions

- `FixedDArray_p FixedDArrayAlloc(long size)`
- `FixedDArray_p FixedDArrayCopy(FixedDArray_p array)`
- `void FixedDArrayAdd(FixedDArray_p dest, FixedDArray_p s1, FixedDArray_p s2)`
- `void FixedDArrayFree(FixedDArray_p junk)`
- `void FixedDArrayInitialize(FixedDArray_p array, long value)`
- `void FixedDArrayMax(FixedDArray_p dest, FixedDArray_p s1, FixedDArray_p s2)`
- `void FixedDArrayMin(FixedDArray_p dest, FixedDArray_p s1, FixedDArray_p s2)`
- `void FixedDArrayMulAdd(FixedDArray_p dest, FixedDArray_p s1, long f1, FixedDArray_p s2, long f2)`
- `void FixedDArrayPrint(FILE* out, FixedDArray_p array)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `FixedDArrayAlloc`: Allocate an array of known size.
- `FixedDArrayFree`: Free an array. Handles NULL silently.
- `FixedDArrayInitialize`: Set all values in the array to a given value.
- `FixedDArrayAdd`: Component-wise addition of both sources. Guaranteed to work if dest is a source (but not maximally efficient - who cares). Yes, it's worth mentioning it ;-)
- `FixedDArrayMulAdd`: Component-wise addition of both weighted sources. Guaranteed to work if dest is a source (but not maximally efficient - who cares). Yes, it's worth mentioning it ;-)
- `FixedDArrayMax`: Compute componentwise max of vectors. See above.
- `FixedDArrayMin`: Compute componentwise min of vectors. See above.
- `FixedDArrayPrint`: Print an array (useful for debugging, I suspect).
- `FixedDArrayCopy`: Copy an array, return pointer to new copy.

### Dependencies

- `"clb_fixdarrays.h"`
- `<clb_memory.h>`

### Compile-Time Conditions

- `CLB_FIXDARRAYS`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_fixdarrays.h`, `BASICS/clb_fixdarrays.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 350 lines, 11 scanned public declarations, 0 scanned internal function definitions, and 9 structured function-comment blocks.
- Rather trivial datatype for arrays of long integers with a known, fixed and and queryable size. the GNU Lesser General Public License.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `FixedDArrayAlloc` allocates the flexible-array payload and stores the size but does not initialize element values; callers are expected to call `FixedDArrayInitialize` or fill every slot before reading.
- `FixedDArrayAdd`, `FixedDArrayMulAdd`, `FixedDArrayMax`, and `FixedDArrayMin` assert that all source arrays and the destination are non-null and have equal sizes before component-wise operations. Rust compatibility helpers should keep size mismatches as invariant failures rather than recoverable `false` results.
- Element reads and writes in C use direct `array[i]` payload access through callers rather than checked exported accessors, so Rust's compatibility-shaped indexed helpers should treat out-of-range indices as invariant violations.
- `FixedDArrayCopy(NULL)` returns `NULL` instead of asserting. Rust keeps that public branch in an optional compatibility helper while retaining the value-level `copy_array` method for known arrays.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `FixedDArrayAlloc` exposes uninitialized payload storage. Rust's safe constructor zero-fills instead; if performance-sensitive callers need allocation without initialization later, that should be a narrowly scoped internal builder with explicit fill-before-read invariants rather than the default public API.
- `FixedDArray` is used as an invariant-backed feature vector in C. Keep assertion-shaped size and index contracts for drop-in compatibility, but future Rust-only APIs fed by user-derived dimensions may want explicit checked constructors or `try_` component-wise operations.
- `FixedDArrayCopy` has a nullable pointer-shaped API even though most Rust callers own concrete arrays. Prefer the non-null value method in new Rust code and reserve the optional helper for preserving C call boundaries.
<!-- END MANUAL REVIEW: c_source_docs -->
