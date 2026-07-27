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

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for the accepted KBO6 non-owning traversal on 2026-07-25.

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
- `PLocalStackTop` returns the current stack pointer/count rather than a top element; Rust exposes this as an explicit C-shaped alias beside the clearer `current` count helper.
- `PLocalTaggedStackPush` has two C variants. The raw low-bit `TAGGED_POINTERS` branch asserts pointer alignment and tag width before packing the tag into the value pointer; the portable branch stores value and tag in two pointer slots. Rust currently models the portable two-slot accounting instead of raw pointer tagging.
- The ordinary upstream build enables `-DTAGGED_POINTERS`, and `cto_kbolin.c` is the sole tagged-stack C owner. The generic Rust `PLocalTaggedStack` has no production owner and continues to model the portable two-slot accounting. First-order KBO6 now uses its own typed `(BorrowedTermCell, DerefType)` stack: it adopts C's non-owning pointer semantics without packing tags into pointer bits, while higher-order KBO traversal remains owning. Complete generic-stack evidence is retained in [`experiment 122`](../../../experiments/2026-07-18-122-plocal-tagged-stack-boundary/FINDINGS.md), and the focused raw-cursor safety and performance evidence is retained in [`experiment 316`](../../../experiments/2026-07-25-015-borrowed-kbo-balance/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- Raw push/pop macros can write past capacity or underflow if a caller skips `EnsureSpace` or emptiness checks. After compatibility is secured, consider scoped traversal helpers that make the required checks structural rather than caller discipline.
- `PLocalStackTop` returns the current stack pointer/count, not the top value. A later cleaned API should use clearer naming while preserving the C macro spelling for compatibility wrappers.
- `PLocalStackGrow` copies `old_size` pointer slots, not only initialized/current entries, so C may copy unused slots during growth. Rust avoids uninitialized data, but benchmark-sensitive ports should confirm whether copying full capacity ever mattered for locality.
- `PLocalStackInit` heap-allocates 64 pointer slots for every stack, including the extremely hot two-term-pair matching stack. HEN011 profiling found that four inline term pairs plus a spill vector reduced Rust's 5,000-clause instruction count by about 5.4% relative to 32 inline pairs; a later C cleanup should benchmark smaller per-call storage or caller-owned reusable buffers instead of applying the generic default to this path.
- The safe Rust tagged stack models the portable two-slot branch, not low-bit pointer tagging, and still has no production owner. KBO6's dedicated borrowed cursor has now removed the measured `Rc` frame-ownership cost without pointer-bit packing; its narrowly contained unsafe boundary documents provenance, ownership, lifetime, initialization, aliasing, and mutation invariants. Do not generalize that cursor or introduce low-bit tagging without a separate owner-specific profile and equally explicit contracts.
<!-- END MANUAL REVIEW: c_source_docs -->
