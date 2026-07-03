<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_plocalstacks

## Source Files

- [BASICS/clb_plocalstacks.h](../../../eprover/BASICS/clb_plocalstacks.h)
- [BASICS/clb_plocalstacks.c](../../../eprover/BASICS/clb_plocalstacks.c)

## Purpose

Stack implementation with macros that use local (automatic) variables. The responsibility to ensurce space is delegeted to the user for the simple push operation. There are macro-functions to push all arguments of a term.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Martin Möhrmann

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLB_PLOCALSTACKS`
- `PLOCALSTACK_DEFAULT_SIZE`
- `PLOCALSTACK_TAG_BITS`
- `PLOCALSTACK_TAG_MASK`
- `PLOCALSTACK_VAL_MASK`
- `PLocalStackEmpty(stack)`
- `PLocalStackEnsureSpace(stack, space)`
- `PLocalStackFree(junk)`
- `PLocalStackInit(stack)`
- `PLocalStackInitWithSize(stack, num)`
- `PLocalStackPop(stack)`
- `PLocalStackPush(stack, val)`
- `PLocalStackPushTermArgs(stack, term)`
- `PLocalStackPushTermArgsReversed(stack, term)`
- `PLocalStackTop(stack)`
- `PLocalTaggedStackEmpty(stack)`
- `PLocalTaggedStackEnsureSpace(stack, space)`
- `PLocalTaggedStackFree(junk)`
- `PLocalTaggedStackInit(stack)`
- `PLocalTaggedStackPop(stack, val, tag)`
- `PLocalTaggedStackPush(stack, val, tag)`
- `PLocalTaggedStackPushTermArgs(stack, term, tag)`
- `PLocalTaggedStackPushTermArgsReversed(stack, term, tag)`

### Globals

- None found in the source scan.

### Exported Functions

- `PLocalStackGrow`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PLocalStackGrow`: Grow stack to have room for at least space new items.

### Dependencies

- `<clb_memory.h>`

### Compile-Time Conditions

- `CLB_PLOCALSTACKS`
- `TAGGED_POINTERS`

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

Source files reviewed: `BASICS/clb_plocalstacks.h`, `BASICS/clb_plocalstacks.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 191 lines, 0 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Stack implementation with macros that use local (automatic) variables. The responsibility to ensurce space is delegeted to the user for the simple push operation. There are macro-functions to push all arguments of a term.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `PLocalStackPush`, `PLocalStackPop`, and `PLocalTaggedStackPop` are raw macros without recoverable error returns; callers are expected to ensure capacity/non-emptiness before use. Rust compatibility methods keep the same non-optional surface and panic when safe Rust detects a missed precondition.
- `PLocalTaggedStackPush` has two C variants. The raw low-bit `TAGGED_POINTERS` branch asserts pointer alignment and tag width before packing the tag into the value pointer; the portable branch stores value and tag in two pointer slots. Rust currently models the portable two-slot accounting instead of raw pointer tagging.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- Raw push/pop macros can write past capacity or underflow if a caller skips `EnsureSpace` or emptiness checks. After compatibility is secured, consider scoped traversal helpers that make the required checks structural rather than caller discipline.
- `PLocalStackTop` returns the current stack pointer/count, not the top value. A later cleaned API should use clearer naming while preserving the C macro spelling for compatibility wrappers.
- `PLocalStackGrow` copies `old_size` pointer slots, not only initialized/current entries, so C may copy unused slots during growth. Rust avoids uninitialized data, but benchmark-sensitive ports should confirm whether copying full capacity ever mattered for locality.
- The safe Rust tagged stack models the portable two-slot branch, not low-bit pointer tagging. Revisit only if profiling proves the packed representation matters and either a safe representation is available or the project-level unsafe policy is explicitly changed for this non-DLL use case.
<!-- END MANUAL REVIEW: c_source_docs -->
