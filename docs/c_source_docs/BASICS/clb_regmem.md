<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_regmem

## Source Files

- [BASICS/clb_regmem.h](../../../eprover/BASICS/clb_regmem.h)
- [BASICS/clb_regmem.c](../../../eprover/BASICS/clb_regmem.c)

## Purpose

A module supporting dynamic memory for local static variables that is still freed quasi-automatically (via a call to a cleanup function) when the program terminates. This is useful if there is a need for dynamically growing (or shrinking) persistent memory "owned" by a function. I still want this cleaned up at the end to keep the usefulness of the memory counters in detecting (and hence avoiding)

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLB_REGMEM`

### Globals

- None found in the source scan.

### Exported Functions

- `void RegMemCleanUp(void)`
- `void RegMemFree(void* mem)`
- `void* RegMemAlloc(size_t size)`
- `void* RegMemProvide(void* mem, size_t *oldsize, size_t newsize)`
- `void* RegMemRealloc(void* mem, size_t size)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `RegMemAlloc`: Allocate a registered memory area.
- `RegMemRealloc`: Realloc a registererd memory area.
- `RegMemFree`: Free a registered memory area.
- `RegMemProvide`: Return pointer to a memory section that is large enough to store newsize bytes, with the first *oldsize bytes being initialized to the value at *mem, and the rest being initialized to '0'. If newsize <= oldsize, this is a NOP. If new memory is allocated, oldsize will be updated to reflect the new size (which most likely is larger than newsize).
- `RegMemCleanUp`: Free all registered memory areas.

### Dependencies

- `"clb_regmem.h"`
- `<clb_ptrees.h>`

### Compile-Time Conditions

- `CLB_REGMEM`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_regmem.h`, `BASICS/clb_regmem.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 250 lines, 5 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- Registered-memory cleanup support; preserve shutdown cleanup behavior for long-running tools and error exits.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `RegMemAlloc` stores raw allocation pointers in a file-static `PTree` and `RegMemCleanUp` later frees every registered pointer. Rust preserves process-global cleanup semantics through opaque handles, but future typed scratch-memory owners should prefer scoped ownership where possible.
- `RegMemRealloc` and `RegMemFree` ignore the `PTreeDeleteEntry` return value before reallocating or freeing the pointer. That leaves invalid or double-free calls to fail through allocator behavior; Rust's C-shaped `regmem_*` APIs panic for unregistered handles, while `try_regmem_*` APIs report the handle error explicitly.
- `RegMemProvide` computes `mem+*oldsize` on a `void*`, which relies on GNU-style byte arithmetic. A cleaned C version should cast to `char*` or use a typed byte buffer.
- `RegMemAlloc` uses `SecureMalloc`, so newly allocated bytes are not guaranteed to be zeroed, while `RegMemProvide` zero-fills only the newly grown tail. Rust's safe byte-buffer representation is initialized to zero; revisit this only if profiling shows the extra initialization matters for hot static scratch storage.
- Rust keeps stale handles invalid across cleanup by not resetting the handle counter. C stale raw pointers after cleanup already have undefined ownership, and a later allocation may or may not reuse the same address.
<!-- END MANUAL REVIEW: c_source_docs -->
