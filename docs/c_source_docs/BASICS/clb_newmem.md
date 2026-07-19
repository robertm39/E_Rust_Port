<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_newmem

## Source Files

- [BASICS/clb_newmem.h](../../../eprover/BASICS/clb_newmem.h)
- [BASICS/clb_newmem.c](../../../eprover/BASICS/clb_newmem.c)

## Purpose

This module implements a simple general purpose memory management stystem that is efficient for problems with a very regular memory access pattern (like most theorem provers). In addition to the groundwork it also implements secure versions of standard functions

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `MemCell`
- `Mem_p`

### Macros And Constants

- `CLB_NEWMEM`
- `DataCellAlloc()`
- `DataCellFree(junk)`
- `ENSURE_NULL(junk)`
- `FREE(junk)`
- `IntArrayFree(array, size)`
- `MEMSIZE(type)`
- `MEM_ALIGN`
- `MEM_ARR_SIZE`
- `MEM_CHUNKLIMIT`
- `MEM_FREE_PATTERN`
- `MEM_MULTIPLIER`
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
- `void MemAddNewChunk(int mem_index)`
- `void MemDebugPrintStats(FILE* out)`
- `void MemFlushFreeList(void)`
- `void SizeFreeReal(void* junk, int size)`
- `void* SecureMalloc(int size)`
- `void* SecureRealloc(void *ptr, int size)`
- `void* SizeMallocReal(int size)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `MemFlushFreeList`: Returns all memory kept in free_mem_list[] to the operation system. This is useful if a very different memory access pattern is expected (SizeFree() never reorganizes the memory automatically).
- `SecureMalloc`: Returns a pointer to an unused memory block sized size. If possible, a fresh block is allocated, if not, the reorganization of free_mem_list is triggered, if still no memory is available, an error will be produced.
- `SecureRealloc`: Imitates realloc, but reorganizes free_mem_list to get new memory if the block has to be moved and no memory is available. Will terminate with OUT_OF_MEMORY if no memory is found.
- `SizeMallocReal`: Returns a block of memory sized size using the internal free-list. This block is not freeable with free(), but otherwise should behave like a normal malloc'ed block.
- `SizeFreeReal`: Returns a block sized size. Note: size has to be exact - you should only give blocks to SizeFree() that have been allocated with malloc(size) or SizeMalloc(size). Giving blocks that are to big wastes memory, blocks that are to small will result in more serious trouble (segmentation faults).
- `MemAddNewChunk`: Allocate a block of size MEM_MULTIPLIER*mem_index*MEM_ALIGN and split it into MEM_MULTIPLIER blocks linked into free_mem_list[index]. In other words, fill up the list of fresh blocks of a given size.
- `SecureStrdup`: Implements the functionality of strdup, but uses SecureMalloc() / for the memory handling.
- `SecureStrndup`: Implements the functionality of GNU strndup, but uses SecureMalloc() for the memory handling (creates a NULL-terminated copy of the string or the first n bytes of it).
- `IntArrayAlloc`: Return a pointer to a freshly allocated, 0-initialized block of longs.
- `MemDebugPrintStats`: Print information about allocated and deallocated memory.

### Dependencies

- `"clb_newmem.h"`
- `"clb_os_wrapper.h"`
- `"clb_verbose.h"`
- `<string.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
- `CLB_MEMORY_DEBUG2`
- `CLB_NEWMEM`
- `CONSTANT_MEM_ESTIMATE`
- `NDEBUG`
- `PRINT_SOMEERRORS_STDOUT`

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

Source files reviewed: `BASICS/clb_newmem.h`, `BASICS/clb_newmem.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 593 lines, 24 scanned public declarations, 0 scanned internal function definitions, and 10 structured function-comment blocks.
- Alternative allocator path selected by `USE_NEWMEM`; keep its chunk/block accounting distinct from the older freelist allocator.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `SizeMallocReal` compares the effective byte size, not `mem_index`, against `MEM_CHUNKLIMIT` (`4096 / 16 == 256`). Consequently requests below 256 bytes receive 1,024-block chunks, while a 256-byte request does not, even though both 255 and 256 round to bucket 16. Rust preserves this exact threshold. The pinned old/new policy probe and safe-owner audit are retained in [`experiment 123`](../../../experiments/2026-07-18-123-memory-policy-boundary/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `MemFlushFreeList` is intentionally a dummy in `clb_newmem.c`, unlike the old allocator in `clb_memory.c`. Rust's `newmem` facade preserves the no-op while the shared memory state still supports real flushing for the old policy.
- `SizeMallocReal` rounds requests up to `MEM_ALIGN` buckets and fills empty buckets with `MEM_MULTIPLIER` blocks only when the effective byte request is below 256. The header's `4096 / MEM_ALIGN` spelling can suggest a 4,096-byte object threshold, but the implementation compares bytes directly. This is a hot-path performance policy and documentation hazard; replace or clarify it only with benchmark evidence.
- Large newmem blocks are stored in the same `free_mem_list[mem_index]` shape as chunked blocks even when created one at a time. Rust models the bucketed reuse with safe `MemoryBlock`s, but not allocator-address identity or debug poison words.
- `MemAddNewChunk` computes `MEM_MULTIPLIER * mem_index * MEM_ALIGN` with C `int` arithmetic. Rust checks overflow and exposes the fatal wrapper plus `try_*` helper split; cleaned allocator APIs should avoid accepting raw bucket indices from ordinary callers.
- The header notes that `SizeMallocReal` blocks are not freeable with plain `free()`, while `SecureMalloc` blocks are. Rust's owned block type prevents mixing those deallocation paths; future typed arenas should keep that distinction private.
- As with `clb_memory`, allocation exhaustion ultimately terminates through the C error path. Rust keeps C-shaped panic wrappers and separate `try_*` helpers until executable-wide fatal-error routing owns allocation failures.
<!-- END MANUAL REVIEW: c_source_docs -->
