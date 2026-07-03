<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_pqueue

## Source Files

- [BASICS/clb_pqueue.h](../../../eprover/BASICS/clb_pqueue.h)
- [BASICS/clb_pqueue.c](../../../eprover/BASICS/clb_pqueue.c)

## Purpose

Functions for LIFO-lists. the GNU Lesser General Public License. <1> Tue Jun 30 17:14:42 MET DST 1998 New

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PQueueCell`
- `PQueue_p`

### Macros And Constants

- `CLB_PQUEUE`
- `PQUEUE_DEFAULT_SIZE`
- `PQueueCellAlloc()`
- `PQueueCellFree(junk)`
- `PQueueElementInt(Queue, index)`
- `PQueueElementP(Queue, index)`
- `PQueueEmpty(queue)`
- `PQueueGetLastInt(Queue)`
- `PQueueGetLastP(Queue)`
- `PQueueGetNextInt(Queue)`
- `PQueueGetNextP(Queue)`
- `PQueueLookInt(Queue)`
- `PQueueLookLastInt(Queue)`
- `PQueueLookLastP(Queue)`
- `PQueueLookP(Queue)`

### Globals

- None found in the source scan.

### Exported Functions

- `IntOrP PQueueElement(PQueue_p queue, long index)`
- `long PQueueCardinality(PQueue_p queue)`
- `long PQueueIncIndex(PQueue_p queue, long index)`
- `long PQueueTailIndex(PQueue_p queue)`
- `static inline IntOrP PQueueGetLast(PQueue_p queue)`
- `static inline IntOrP PQueueGetNext(PQueue_p queue)`
- `static inline IntOrP PQueueLook(PQueue_p queue)`
- `static inline IntOrP PQueueLookLast(PQueue_p queue)`
- `static inline PQueue_p PQueueAlloc(void)`
- `static inline void PQueueBuryInt(PQueue_p queue, long val)`
- `static inline void PQueueBuryP(PQueue_p queue, void* val)`
- `static inline void PQueueFree(PQueue_p junk)`
- `static inline void PQueueReset(PQueue_p queue)`
- `static inline void PQueueStoreInt(PQueue_p queue, long val)`
- `static inline void PQueueStoreP(PQueue_p queue, void* val)`
- `void PQueueGrow(PQueue_p queue)`

## Implementation Notes

### Internal Functions

- `PQueueAlloc`
- `PQueueBuryInt`
- `PQueueBuryP`
- `PQueueFree`
- `PQueueGetLast`
- `PQueueGetNext`
- `PQueueLook`
- `PQueueLookLast`
- `PQueueReset`
- `PQueueStoreInt`
- `PQueueStoreP`
- `pqueue_bury`
- `pqueue_store`

### Source-Level Behavior

- `pqueue_store`: Put an element in the queue.
- `pqueue_bury`: Put an element at the front of the queue (i.e. "bury" it under all the other elements in a stack-view of the queue).
- `PQueueAlloc`: Allocate an empty, initialized Queue.
- `PQueueFree`: Free a Queue.
- `PQueueReset`: Reset a queue to empty state.
- `PQueueStoreInt`: Store an integer in the queue.
- `PQueueStoreP`: Store a pointer in the queue.
- `PQueueBuryInt`: Store an integer at the front of the the queue.
- `PQueueBuryP`: Store a pointer at the front of the queue.
- `PQueueGetNext`: Extract the next value from the queue and return it.
- `PQueueGetLast`: Extract the last value from the queue (i.e. pop from the queue viewed as a stack) and return it.
- `PQueueLook`: Return the next element from the queue without changing the queue.
- `PQueueLookLast`: Return the last (youngest) value from the queue without modifyin the queue.
- `PQueueGrow`: Increase the size of queue.
- `PQueueCardinality`: Return the number of elements in the queue.
- `PQueueElement`: Retutn the entry at absolute index index.
- `PQueueTailIndex`: Return the index of the tail (oldest, last) element (or -1 if the queue is empty).
- `PQueueIncIndex`: Given an index to a (used) element in the queue, return a similar index to to next element (or -1 if there is no next element).

### Dependencies

- `"clb_pqueue.h"`
- `<clb_memory.h>`

### Compile-Time Conditions

- `CLB_PQUEUE`

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

Source files reviewed: `BASICS/clb_pqueue.h`, `BASICS/clb_pqueue.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 584 lines, 18 scanned public declarations, 13 scanned internal function definitions, and 18 structured function-comment blocks.
- Functions for LIFO-lists. the GNU Lesser General Public License. <1> Tue Jun 30 17:14:42 MET DST 1998 New
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `PQueueGetNext` and `PQueueGetLast` return an `IntOrP` by value and only move `tail`/`head`; they do not clear the backing ring slot. `PQueueReset` likewise only rewinds `head` and `tail`. Absolute-slot access through `PQueueElement` can still observe old payload words after extraction or reset, and Rust preserves this compatibility shape by cloning/copying extracted payloads.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
- Future non-compatibility queue APIs should avoid exposing stale absolute slots after extraction or reset and can drop/reset owned Rust payloads eagerly, but that must stay separate from the C-shaped `PQueue` surface.
<!-- END MANUAL REVIEW: c_source_docs -->
