<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_partial_orderings

## Source Files

- [BASICS/clb_partial_orderings.h](../../../eprover/BASICS/clb_partial_orderings.h)
- [BASICS/clb_partial_orderings.c](../../../eprover/BASICS/clb_partial_orderings.c)

## Purpose

Functions and datatypes useful in dealing with partial orderings. the GNU Lesser General Public License. <1> Wed Jun 16 22:37:09 MET DST 1999 New

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `CompareResult`
- `HoOrderKind`

### Macros And Constants

- `CLB_PARTIAL_ORDERINGS`
- `Q_TO_PART(res)`

### Globals

- `extern char* POCompareSymbol[]`

### Exported Functions

- `(((res)>0) ? to_greater:to_equal)) static inline CompareResult POInverseRelation(CompareResult relation)`

## Implementation Notes

### Internal Functions

- `POInverseRelation`

### Source-Level Behavior

- `POInverseRelation`: Given a comparison relation, return the inverse relation.

### Dependencies

- `"clb_partial_orderings.h"`
- `<clb_defines.h>`

### Compile-Time Conditions

- `CLB_PARTIAL_ORDERINGS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_partial_orderings.h`, `BASICS/clb_partial_orderings.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 194 lines, 4 scanned public declarations, 1 scanned internal function definitions, and 1 structured function-comment blocks.
- Functions and datatypes useful in dealing with partial orderings. the GNU Lesser General Public License. <1> Wed Jun 16 22:37:09 MET DST 1999 New
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `POInverseRelation` handles every comparison result except `to_unknown`; that default branch is an assertion failure in C. Rust now keeps an assertion-shaped inverse helper for compatibility and a separate option-returning helper for checked callers.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- Ported in `src/basics/partial_orderings.rs`, including exact `CompareResult` and `HoOrderKind` discriminants, quasi-order conversion, C-shaped asserting inverse relation handling plus checked optional inversion, and comparison-symbol rendering.

### Change Later

- `POCompareSymbol` is an exported `char*` table whose order is coupled to the `CompareResult` discriminants. Rust preserves table-shaped rendering; a cleaned API should prefer an enum method while keeping the table only as a compatibility adapter.
- `POInverseRelation(to_unknown)` is an assertion failure even though `to_unknown` has a printable symbol. Later ordering APIs should decide whether unknown is a valid cached relation or only an uninitialized sentinel.
- `Q_TO_PART(res)` collapses arbitrary signed comparison integers into partial-ordering results. Rust preserves the sign-based conversion, but new callers should use typed comparison results directly once ordering backends no longer exchange raw `long` comparison values.
<!-- END MANUAL REVIEW: c_source_docs -->
