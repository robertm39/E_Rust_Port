<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_dstacks

## Source Files

- [BASICS/clb_dstacks.h](../../../eprover/BASICS/clb_dstacks.h)
- [BASICS/clb_dstacks.c](../../../eprover/BASICS/clb_dstacks.c)

## Purpose

Soemwhat efficient unlimited growth stacks for doubles. the GNU Lesser General Public License. <1> Sun Jun 7 12:21:16 MET DST 1998 New

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DStackCell`
- `DStackPointer`
- `DStack_p`

### Macros And Constants

- `CLB_DSTACKS`
- `DSTACK_DEFAULT_SIZE`
- `DStackCellAlloc()`
- `DStackCellFree(junk)`
- `DStackEmpty(stack)`
- `DStackGetSP(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `DStack_p DStackAlloc(void)`
- `double DStackBelowTop(DStack_p stack)`
- `double DStackElement(DStack_p stack, DStackPointer pos)`
- `double DStackPop(DStack_p stack)`
- `double DStackTop(DStack_p stack)`
- `void DStackFree(DStack_p junk)`
- `void DStackPush(DStack_p stack, double val)`
- `void DStackReset(DStack_p stack)`

## Implementation Notes

### Internal Functions

- `push`

### Source-Level Behavior

- `push`: Implement push operation for DStacks. If the stack area needs to grow, Realloc is emulated in terms of SizeMalloc()/SizeFree(). This is because stacks are allocated and deallocated a lot, and usually in the same sizes, so it pays of to optimize this behaviour.
- `DStackAlloc`: Allocate an empty stack.
- `DStackFree`: Free a stack.
- `DStackReset`: Reset a DStack to empty state.
- `DStackPush`: Push a double onto the stack
- `DStackPop`: Implement pop operation for non-empty DStacks.
- `DStackTop`: Implement Top operation for non-empty DStacks.
- `DStackBelowTop`: Return second item on the stack (asserts that stack has >=2 elements).
- `DStackElement`: Return element at position pos.

### Dependencies

- `<clb_dstacks.h>`
- `<clb_memory.h>`

### Compile-Time Conditions

- `CLB_DSTACKS`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_dstacks.h`, `BASICS/clb_dstacks.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 336 lines, 11 scanned public declarations, 1 scanned internal function definitions, and 9 structured function-comment blocks.
- Soemwhat efficient unlimited growth stacks for doubles. the GNU Lesser General Public License. <1> Sun Jun 7 12:21:16 MET DST 1998 New
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `DStackPop`, `DStackTop`, `DStackBelowTop`, and `DStackElement` assert their non-empty, two-element, and in-range preconditions instead of returning sentinel values.

### Change Later

- Empty or out-of-range stack access is assertion failure behavior in C. The compatibility-shaped Rust methods should keep panicking, while any future Rust-only optional/checked accessors should be separate APIs.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
