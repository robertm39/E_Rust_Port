<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_memory

## Source Files

- [BASICS/clb_memory.h](../../../eprover/BASICS/clb_memory.h)
- [BASICS/clb_memory.c](../../../eprover/BASICS/clb_memory.c)

## Purpose

This module implements simple general purpose memory management routines that is efficient for problems with a very regular memory access pattern (like most theorem provers). In addition to the groundwork it also implements secure versions of standard functions

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `MemCell`
- `Mem_p`

### Macros And Constants

- `CLB_MEMORY`
- `DataCellAlloc()`
- `DataCellFree(junk)`
- `ENSURE_NULL(junk)`
- `FREE(junk)`
- `IntArrayFree(array, size)`
- `MEMSIZE(type)`
- `MEM_ARR_MIN_INDEX`
- `MEM_ARR_SIZE`
- `MEM_FREE_PATTERN`
- `MEM_RSET_PATTERN`
- `SizeFree(junk, size)`
- `SizeMalloc(size)`

### Globals

- `extern Mem_p free_mem_list[]`
- `extern bool MemIsLow`
- `extern long clb_free_count`
- `extern long secure_malloc_count`
- `extern long secure_malloc_mem`
- `extern long secure_realloc_count`
- `extern long secure_realloc_f_count`
- `extern long secure_realloc_m_count`
- `extern long size_free_count`
- `extern long size_free_mem`
- `extern long size_malloc_count`
- `extern long size_malloc_mem`

### Exported Functions

- `char* SecureStrdup(const char* source)`
- `char* SecureStrndup(const char* source, size_t n)`
- `long* IntArrayAlloc(int size)`
- `static inline void SizeFreeReal(void* junk, size_t size)`
- `static inline void* SizeMallocReal(size_t size)`
- `void MemDebugPrintStats(FILE* out)`
- `void MemFlushFreeList(void)`
- `void MemFreeListPrint(FILE* out)`
- `void* SecureMalloc(size_t size)`
- `void* SecureRealloc(void *ptr, size_t size)`

## Implementation Notes

### Internal Functions

- `SizeFreeReal`
- `SizeMallocReal`
- `free_list_size`

### Source-Level Behavior

- `SizeMallocReal`: Returns a block of memory sized size using the internal free-list. This block is freeable with free(), and in all respects behaves like a normal malloc'ed block.
- `SizeFreeReal`: Returns a block sized size. Note: size has to be exact - you should only give blocks to SizeFree() that have been allocated with malloc(size) or SizeMalloc(size). Giving blocks that are to big wastes memory, blocks that are to small will result in more serious trouble (segmentation faults).
- `free_list_size`: Return the length if the list of MemCells.
- `MemFlushFreeList`: Returns all memory kept in free_mem_list[] to the operation system. This is useful if a very different memory access pattern is expected (SizeFree() never reorganizes the memory automatically).
- `SecureMalloc`: Returns a pointer to an unused memory block sized size. If possible, a fresh block is allocated, if not, the reorganization of free_mem_list is triggered, if still no memory is available, an error will be produced. If the first malloc fails, MemIsLow will be set.
- `SecureRealloc`: Imitates realloc, but reorganizes free_mem_list to get new memory if the block has to be moved and no memory is available. Will terminate with OUT_OF_MEMORY if no memory is found.
- `SecureStrdup`: Implements the functionality of strdup, but uses SecureMalloc() / for the memory handling.
- `SecureStrndup`: Implements the functionality of GNU strndup, but uses SecureMalloc() for the memory handling (creates a NULL-terminated copy of the string or the first n bytes of it).
- `IntArrayAlloc`: Return a pointer to a freshly allocated, 0-initialized block of longs.
- `MemDebugPrintStats`: Print information about allocated and deallocated memory.
- `MemFreeListPrint`: Print the size of the free list for each size.

### Dependencies

- `"clb_memory.h"`
- `"clb_newmem.c"`
- `"clb_newmem.h"`
- `<clb_os_wrapper.h>`
- `<clb_verbose.h>`

### Compile-Time Conditions

- `CLB_MEMORY`
- `CLB_MEMORY_DEBUG`
- `CLB_MEMORY_DEBUG2`
- `CONSTANT_MEM_ESTIMATE`
- `NDEBUG`
- `PRINT_SOMEERRORS_STDOUT`
- `USE_NEWMEM`
- `USE_SYSTEM_MEM`

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

Source files reviewed: `BASICS/clb_memory.h`, `BASICS/clb_memory.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 665 lines, 24 scanned public declarations, 3 scanned internal function definitions, and 11 structured function-comment blocks.
- Core allocation facade: exact-size freelists, secure allocation retries, and debug poisoning/nulling behavior are performance and safety contracts for most other modules.
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
