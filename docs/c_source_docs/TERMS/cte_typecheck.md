<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_typecheck

## Source Files

- [TERMS/cte_typecheck.h](../../../eprover/TERMS/cte_typecheck.h)
- [TERMS/cte_typecheck.c](../../../eprover/TERMS/cte_typecheck.c)

## Purpose

Type checking and inference for Simple types

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Simon Cruanes, Petar Vucmirovic, Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTE_TYPECHECK`
- `TI_ERROR(msg)`

### Globals

- None found in the source scan.

### Exported Functions

- `Type_p TypeCheckArithBinop(Sig_p sig, Term_p t)`
- `Type_p TypeCheckArithConv(Sig_p sig, Term_p t)`
- `Type_p TypeCheckDistinct(Sig_p sig, Term_p t)`
- `Type_p TypeCheckEq(Sig_p sig, Term_p t)`
- `bool TypeCheckConsistent(Sig_p sig, Term_p term)`
- `void TypeDeclareIsNotPredicate(Sig_p sig, Term_p term, Scanner_p in)`
- `void TypeDeclareIsPredicate(Sig_p sig, Term_p term)`
- `void TypeInferSort(Sig_p sig, Term_p term, Scanner_p in)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `term_determine_type`: Given number of arguments and type, return the type of the term resulting from consuming the number of arguments. Returned type is shared.
- `TypeInferSort`: Infer the sort of this term. It can either use the type of the function symbol, if already known, or guess a type and add it to the signature otherwise. By default terms are supposed not to be atoms, unless the parser decides that they must be boolean.
- `TypeDeclareIsPredicate`: declare that the term has a role of predicate (occurs as a boolean atom)
- `TypeDeclareIsNotPredicate`: Declare that this term is not a boolean atom, because it ocurs in an equation or is a subterm of another term.

### Dependencies

- `"cte_termfunc.h"`
- `"cte_typecheck.h"`
- `<cte_signature.h>`
- `<cte_termtypes.h>`
- `<cte_typebanks.h>`

### Compile-Time Conditions

- `CTE_TYPECHECK`
- `ENABLE_LFHO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_typecheck.h`, `TERMS/cte_typecheck.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 508 lines, 9 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Type checking and inference for Simple types
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
