<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_pstacks

## Source Files

- [BASICS/clb_pstacks.h](../../../eprover/BASICS/clb_pstacks.h)
- [BASICS/clb_pstacks.c](../../../eprover/BASICS/clb_pstacks.c)

## Purpose

Soemwhat efficient unlimited growth stacks for pointers/long ints. the GNU Lesser General Public License. <1> Wed Dec 3 16:22:48 MET 1997 New

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PStackCell`
- `PStackPointer`
- `PStack_p`

### Macros And Constants

- `CLB_PSTACKS`
- `PSTACK_AVG_MEM`
- `PSTACK_DEFAULT_SIZE`
- `PStackAssignInt(stack, pos, value)`
- `PStackAssignP(stack, pos, value)`
- `PStackBaseAddress(stackarg)`
- `PStackBelowTopInt(stack)`
- `PStackBelowTopP(stack)`
- `PStackCellAlloc()`
- `PStackCellFree(junk)`
- `PStackElementInt(stack,pos)`
- `PStackElementP(stack,pos)`
- `PStackEmpty(stack)`
- `PStackGetSP(stack)`
- `PStackGetTopSP(stack)`
- `PStackPopInt(stack)`
- `PStackPopP(stack)`
- `PStackTopInt(stack)`
- `PStackTopP(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `IntOrP* PStackTopAddr(PStack_p stack)`
- `PStackPointer PStackBinSearch(PStack_p stack, void* key, PStackPointer lower, PStackPointer upper, ComparisonFunctionType cmpfun)`
- `double PStackComputeAverage(PStack_p stack, double *deviation)`
- `static inline IntOrP *PStackElementRef(PStack_p stack, PStackPointer pos)`
- `static inline IntOrP PStackBelowTop(PStack_p stack)`
- `static inline IntOrP PStackElement(PStack_p stack, PStackPointer pos)`
- `static inline IntOrP PStackPop(PStack_p stack)`
- `static inline IntOrP PStackTop(PStack_p stack)`
- `static inline PStack_p PStackAlloc(void)`
- `static inline PStack_p PStackCopy(PStack_p stack)`
- `static inline PStack_p PStackVarAlloc(long size)`
- `static inline void PStackDiscardTop(PStack_p stack)`
- `static inline void PStackFree(PStack_p junk)`
- `static inline void PStackPushInt(PStack_p stack, long val)`
- `static inline void PStackPushP(PStack_p stack, void* val)`
- `static inline void PStackReset(PStack_p stack)`
- `void PStackGrow(PStack_p stack)`
- `void PStackMerge(PStack_p st1, PStack_p st2, PStack_p res, ComparisonFunctionType cmpfun)`
- `void PStackPrintInt(FILE* out, char* format, PStack_p stack)`
- `void PStackPrintP(FILE* out, char* format, PStack_p stack)`
- `void PStackPushStack(PStack_p target, PStack_p source)`
- `void PStackSort(PStack_p stack, ComparisonFunctionType cmpfun)`

## Implementation Notes

### Internal Functions

- `PStackAlloc`
- `PStackBelowTop`
- `PStackCopy`
- `PStackDiscardTop`
- `PStackElement`
- `PStackFindInt`
- `PStackFindP`
- `PStackFree`
- `PStackPop`
- `PStackPushInt`
- `PStackPushP`
- `PStackReset`
- `PStackTop`
- `PStackVarAlloc`
- `push`

### Source-Level Behavior

- `push`: Implements push operation for pstacks and checks and ensures there is enought space on the steck.
- `PStackAlloc`: Allocate an empty stack.
- `PStackVarAlloc`: Allocate an empty stack with selectable initial size.
- `PStackFree`: Free a stack.
- `PStackCopy`: Copy a PStack with contents. Use with care, as some data structures may not be copyable very well (e.g. pointers to the same array, registered references, ...)
- `PStackReset`: Reset a PStack to empty state.
- `PStackPushInt`: Push a (long) int onto the stack
- `PStackPushP`: Push a pointer onto the stack
- `PStackPop`: Implement pop operation for non-empty pstacks.
- `PStackDiscardTop`: Do a PStackPop without returning result, to avoid warnings.
- `PStackTop`: Implement Top operation for non-empty pstacks.
- `PStackBelowTop`: Return second item on the stack (asserts that stack has >=2 elements).
- `PStackElement`: Return element at position pos.
- `PStackElementRef`: Return reference to element at position pos.
- `PStackFindP`: Find a pointer in the stack
- `PStackFindInt`: Find an int in the stack
- `PStackGrow`: Grow the stack area. Realloc is emulated in terms of SizeMalloc()/SizeFree(). This is because stacks are allocated and deallocated a lot, and usually in the same sizes, so it pays off to optimize this behaviour.
- `PStackDiscardElement`: Remove element number i from the stack. If it is not the top element, the top element gets swapped in. 0
- `PStackTopAddr`: Return address of top element on the stack.
- `PStackComputeAverage`: Given a stack of integers, compute the arithmetic mean (returned) and the standard deviation (stored in *deviation) of the integers.
- `PStackSort`: Sort the elements of the PStack using qsort.
- `PStackBinSearch`: Perform a binar search on the (ordered) stack between indices lower (inclusive) and upper (exclusive). Return index of key, when found, or index of the next bigger element if not.
- `PStackMerge`: Merge two sorted stacks onto a third. Discards duplicates.
- `PStackPushStack`: Push all elements from source onto target.
- `PStackPrintInt`: Print a stack (interpreted as (long) integers) using the format given.
- `PStackPrintP`: Print a stack (interpreted as pointer stack) using the format given.

### Dependencies

- `"clb_simple_stuff.h"`
- `<clb_memory.h>`
- `<clb_pstacks.h>`

### Compile-Time Conditions

- `CLB_PSTACKS`
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

Source files reviewed: `BASICS/clb_pstacks.h`, `BASICS/clb_pstacks.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 846 lines, 25 scanned public declarations, 15 scanned internal function definitions, and 26 structured function-comment blocks.
- Soemwhat efficient unlimited growth stacks for pointers/long ints. the GNU Lesser General Public License. <1> Wed Dec 3 16:22:48 MET 1997 New
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `PStackFindP` performs raw pointer identity comparison, while `PStackFindInt` compares integer payload values. Rust keeps these as distinct helpers so borrowed-object searches do not accidentally become structural equality checks.
- `PStackTop`, `PStackTopAddr`, `PStackBelowTop`, `PStackPop`, `PStackElement`, `PStackElementRef`, `PStackDiscardTop`, and `PStackDiscardElement` assert their non-empty/range preconditions before reading or mutating the backing stack. Rust compatibility-shaped methods keep those as panics rather than optional access, while retaining a separate option-returning drain helper for Rust-owned control flow.
- `PStackGetTopSP` is a signed macro returning `current - 1`, so an empty stack reports `-1`; it is not an optional value in C.
- `PStackPrintInt` and `PStackPrintP` are thin loops over the live stack pointer that pass each payload to caller-supplied `fprintf` formats. The current C tree only calls `PStackPrintInt` with `"%4ld."`; Rust ports that concrete integer rendering, provides a `%p`-shaped pointer-address renderer for pointer payloads, and exposes a safe element-writer hook for other typed renderers.
- `PStackAlloc` reserves 128 `IntOrP` entries, so the C allocation is 128 pointer-sized words regardless of the logical payload. Rust `PStack<T>` preserves the logical 128-entry growth boundary but normally limits its initial physical capacity to the number of wide typed entries that fit in the same byte count. Clause derivation stacks start at the six-entry occupancy assumed by C's `PSTACK_AVG_MEM`; formula derivation stacks follow `WFormulaPushDerivation`'s explicit `PStackVarAlloc(3)` element count and three-to-six growth point. Typed `Vec` storage avoids hundreds of megabytes of eager wide derivation metadata on large generated-clause queues.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `PStack` mixes assertion-backed operations (`Top`, `Pop`, indexed access, and discard) with callers that often conceptually want checked draining. Rust keeps both shapes represented; future Rust-only helpers should use explicit `try_` names so optional control flow is not confused with the original stack contract.
- The C print helpers accept arbitrary `fprintf` format strings for integer and pointer payloads. Rust intentionally avoids a generic printf parser for now because checked call sites only require the `"%4ld."` integer format and the pointer helper only covers `%p`-style address rendering; add a small audited compatibility parser later if more C-format call sites become reachable.
- `PSTACK_DEFAULT_SIZE` eagerly allocates 128 pointer words even though common clause derivation stacks contain only two or three entries, while `PSTACK_AVG_MEM` budgets only six entries in `CLAUSECELL_MEM`; Rust uses that estimate for clause derivations while retaining the C logical growth boundary. Formula derivations instead use the source's explicit three-entry allocation. Both implementations should eventually benchmark a small inline buffer or demand-grown clause derivation store, and C's aggregate memory accounting should be reconciled with its actual eager allocation.
<!-- END MANUAL REVIEW: c_source_docs -->
