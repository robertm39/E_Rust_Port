<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_sysdate

## Source Files

- [BASICS/clb_sysdate.h](../../../eprover/BASICS/clb_sysdate.h)
- [BASICS/clb_sysdate.c](../../../eprover/BASICS/clb_sysdate.c)

## Purpose

Data types dealing with "dates" and "times". A "time" in this context is a data type with a defined starting point and a total ordering that monotonically increases during the run of the program and can be used to define an order of events. A "date" is a specific element from a "time".

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `SysDate`

### Macros And Constants

- `CLB_SYSDATE`
- `SysDateCreationTime()`
- `SysDateEqual(date1, date2)`
- `SysDateInc(sd)`
- `SysDateInvalidTime()`
- `SysDateIsCreationDate(date)`
- `SysDateIsEarlier(date1, date2)`
- `SysDateIsInvalid(date)`
- `SysDateMaximum(date1, date2)`

### Globals

- None found in the source scan.

### Exported Functions

- `void SysDatePrint(FILE* out, SysDate date)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `SysDatePrint`: Print representation of a system time to the given channel.

### Dependencies

- `"clb_simple_stuff.h"`
- `"clb_sysdate.h"`
- `<limits.h>`
- `<stdio.h>`

### Compile-Time Conditions

- `CLB_SYSDATE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_sysdate.h`, `BASICS/clb_sysdate.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 139 lines, 2 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Data types dealing with "dates" and "times". A "time" in this context is a data type with a defined starting point and a total ordering that monotonically increases during the run of the program and can be used to define an order of events. A "date" is a specific element from a "time".
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
