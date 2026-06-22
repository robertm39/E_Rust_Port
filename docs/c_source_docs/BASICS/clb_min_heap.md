<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_min_heap

## Source Files

- [BASICS/clb_min_heap.h](../../../eprover/BASICS/clb_min_heap.h)
- [BASICS/clb_min_heap.c](../../../eprover/BASICS/clb_min_heap.c)

## Purpose

Simple minimum heap implementation. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `CmpFun`
- `MinHeapCell`
- `MinHeap_p`
- `SetIndexFun`

### Macros And Constants

- `CALL_SETTER(h, el, idx)`
- `CLB_MIN_HEAPS`
- `IS_LEAF(idx, size)`
- `IS_ROOT(idx)`
- `LEFT(idx)`
- `MinHeapAlloc(f)`
- `MinHeapPopMinInt(m)`
- `MinHeapPopMinP(m)`
- `PARENT(idx)`
- `RIGHT(idx)`

### Globals

- None found in the source scan.

### Exported Functions

- `IntOrP MinHeapPopMin(MinHeap_p)`
- `MinHeap_p MinHeapAllocWithIndex(CmpFun, SetIndexFun)`
- `long MinHeapSize(MinHeap_p)`
- `void DBGPrintHeap(FILE* out, MinHeap_p h, bool as_ptr)`
- `void MinHeapAddInt(MinHeap_p, long)`
- `void MinHeapAddP(MinHeap_p, void*)`
- `void MinHeapDecrKey(MinHeap_p, long)`
- `void MinHeapFree(MinHeap_p)`
- `void MinHeapIncrKey(MinHeap_p, long)`
- `void MinHeapRemoveElement(MinHeap_p h, long idx)`
- `void MinHeapUpdateElement(MinHeap_p h, long idx)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `bubble_up`: If an element at child_idx was just inserted or its value has been decreased then bring the element up as necessary.
- `add`: Internal function for inserting a key.
- `MinHeapAllocWithIndex`: Allocate and initialize a min heap. Setter function is used to notify calling code that the index of stored element is changed. Setter is only necessary if we want heap to be able to increase/ decrease a key.
- `MinHeapSize`: Number of stored elements in the heap.
- `MinHeapAddP`: Add a pointer to heap. O(log n)
- `MinHeapAddInt`: Add an integer to heap. O(log n)
- `MinHeapPopMin`: Pop the maximum element and restore heap property. O(log n)
- `MinHeapDecrKey`: Notify that the key assigned to the idx has (possibly) been decreased. O(log n)
- `MinHeapIncrKey`: Notify that the key assigned to the idx has (possibly) been increased. O(log n)
- `MinHeapFree`: Deallocate the space allocated for junk.
- `DBGPrintHeap`: Print the contents of the heap. If as_ptr is true, then the heap is interpreted as heap of pointers. O(n)
- `MinHeapUpdateElement`: When the value of an element has been updated, then fix its position inside the heap by reruning comparison function on corresponding nodes. * If the element becomes smaller than parent -- than bubble the node up * Else -- try sifting the node down (this will succeed only if the node increased in value) O(log n)

### Dependencies

- `"clb_min_heap.h"`
- `<clb_pstacks.h>`

### Compile-Time Conditions

- `CLB_MIN_HEAPS`

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

Source files reviewed: `BASICS/clb_min_heap.h`, `BASICS/clb_min_heap.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 508 lines, 15 scanned public declarations, 0 scanned internal function definitions, and 14 structured function-comment blocks.
- Simple minimum heap implementation. the GNU Lesser General Public License.
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
