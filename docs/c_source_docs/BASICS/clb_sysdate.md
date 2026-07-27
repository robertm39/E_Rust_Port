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

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
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

### Compatibility Notes

- `SysDate` is a signed C `long` used as a monotone event date, with `0` as creation time and `-1` as invalid time.
- `SysDateInc(sd)` increments the pointed-to date first and then asserts the result is nonzero, so incrementing `SysDateInvalidTime()` mutates it to the creation-time sentinel before failing.
- `SysDatePrint` uses `%5lu`, so a negative date is rendered through unsigned-C-long formatting rather than signed decimal formatting.

### Rust Port Status Notes

- `src/basics/sysdate.rs` ports the sentinel constructors, comparisons, maximum operation, raw conversion boundary, C-shaped increment assertion helper, reportable increment helper, unsigned-C-long print shape, and tests for sentinel, assertion, overflow, and print behavior.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `SysDateInc` uses signed `long` increment without overflow handling. Rust treats overflow as a panic/reportable state instead of importing C undefined behavior; future date APIs should make resource-limit/overflow policy explicit.
<!-- END MANUAL REVIEW: c_source_docs -->
