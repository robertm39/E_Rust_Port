<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_properties

## Source Files

- [BASICS/clb_properties.h](../../../eprover/BASICS/clb_properties.h)

## Purpose

Macros for dealing with 1 bit properties of objects (well, structs). It requires the object to be dealt with to have a field named "properties" that is of some integer or enumeration type. This is pretty ugly, but I did not want to spend to much time on it.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `AssignProp(obj, sel, prop)`
- `CLB_PROPERTIES`
- `DelProp(obj, prop)`
- `FlipProp(obj, prop)`
- `GiveProps(obj,prop)`
- `IsAnyPropSet(obj, prop)`
- `PropsAreEquiv(obj1, obj2, props)`
- `QueryProp(obj, prop)`
- `SetProp(obj, prop)`

### Globals

- None found in the source scan.

### Exported Functions

- None found in the source scan.

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- None found in the source scan.

### Compile-Time Conditions

- `CLB_PROPERTIES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_properties.h`.

### Review Notes

- Reviewed as a standalone header unit in `BASICS` covering 1 source file(s), about 75 lines, 0 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- Macros for dealing with 1 bit properties of objects (well, structs). It requires the object to be dealt with to have a field named "properties" that is of some integer or enumeration type. This is pretty ugly, but I did not want to spend to much time on it.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
