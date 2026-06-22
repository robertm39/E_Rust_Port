<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_functypes

## Source Files

- [TERMS/cte_functypes.h](../../../eprover/TERMS/cte_functypes.h)
- [TERMS/cte_functypes.c](../../../eprover/TERMS/cte_functypes.c)

## Purpose

Simple, widely used functions for dealing with function symbols and operators. the GNU Lesser General Public License. <1> Sun Nov 9 23:09:33 MET 1997

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz, Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FunCode`
- `FuncSymbType`

### Macros And Constants

- `ATOMIC_FUNC_SYM_TOK`
- `CTE_FUNCTYPES`

### Globals

- `extern TokenType FuncSymbStartToken`
- `extern TokenType FuncSymbToken`

### Exported Functions

- `FuncSymbType FuncSymbParse(Scanner_p in, DStr_p id)`

## Implementation Notes

### Internal Functions

- `normalize_float_rep`
- `normalize_int_rep`
- `normalize_rational_rep`

### Source-Level Behavior

- `normalize_int_rep`: Take a string representation of an integer and turn it into a normal form. This is done by dropping the optional leading + and all leading zeros (except for the case of plain '0', of course).
- `normalize_rational_rep`: Take a string representation of an integer and turn it into a normal form. This is done by dropping optional leading +es and all leading zeros (except for the case of plain '0', of course), and moving any remaining '-' to the very front. Return true on success and false if something weird happened.
- `normalize_float_rep`: Take a string representation of a floating point number and turn it into a normal form. The normal form is whatever sprintf() makes of it. Over- and underflow are accepted and ingnored (this is floating point math, after all - what do you expect?).
- `FuncSymbParse`: Parse a function or predicate symbol (or, currently, variable) and store the representation into id. Operators are now of the types - Identifier - SemIdent - String - SQString - Integers, potentially signed - Reals, potentially signed - Rationals (fractions)

### Dependencies

- `"cte_functypes.h"`
- `<cio_basicparser.h>`
- `<clb_simple_stuff.h>`

### Compile-Time Conditions

- `CTE_FUNCTYPES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_functypes.h`, `TERMS/cte_functypes.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 384 lines, 5 scanned public declarations, 3 scanned internal function definitions, and 4 structured function-comment blocks.
- Simple, widely used functions for dealing with function symbols and operators. the GNU Lesser General Public License. <1> Sun Nov 9 23:09:33 MET 1997
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
