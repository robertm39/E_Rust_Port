<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# ORDERINGS / cto_orderings

## Source Files

- [ORDERINGS/cto_orderings.h](../../../eprover/ORDERINGS/cto_orderings.h)
- [ORDERINGS/cto_orderings.c](../../../eprover/ORDERINGS/cto_orderings.c)

## Purpose

Generic Interface to the term comparison routines. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `ORDERINGS`. Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTO_ORDERINGS`

### Globals

- None found in the source scan.

### Exported Functions

- `CompareResult TOCompare(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `CompareResult TOCompareSymbolParse(Scanner_p in)`
- `PStackPointer TOPrecedenceParse(Scanner_p in, OCB_p ocb)`
- `PStackPointer TOSymbolComparisonChainParse(Scanner_p in, OCB_p ocb)`
- `bool TOGreater(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `long TOWeightsParse(Scanner_p in, OCB_p ocb)`
- `void TOSymbolWeightParse(Scanner_p in, OCB_p ocb)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `TOGreater`: Test wether t1 is greater that t2 in the ordering described by the ocb.
- `TOCompare`: Compare t1 and t2 in the ordering described by the ocb.
- `TOCompareSymbolParse`: Parse a symbol (>, <, =) and return the corresponding comparion result code.
- `TOSymbolComparisonChainParse`: Parse a chain of precedence constraints (e.g. f > g = h < a) and insert the constraints into ocb. Return new OCB status pointer.
- `TOPrecedenceParse`: Parse a precedence (list of precedence chains).
- `TOSymbolWeightParse`: Parse a f:w declaration.
- `TOWeightsParse`: Parse a list of weight assignments. Return number of assignments parsed.

### Dependencies

- `"cto_orderings.h"`
- `<cto_kbo.h>`
- `<cto_kbolin.h>`
- `<cto_lpo.h>`
- `<cto_lpo_debug.h>`

### Compile-Time Conditions

- `CTO_ORDERINGS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `ORDERINGS/cto_orderings.h`, `ORDERINGS/cto_orderings.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `ORDERINGS` covering 2 source file(s), about 426 lines, 7 scanned public declarations, 0 scanned internal function definitions, and 7 structured function-comment blocks.
- Generic Interface to the term comparison routines. the GNU Lesser General Public License.
- Ordering code. Comparison outcomes, caching, precedence, and weight handling must match the C implementation because they drive simplification and inference eligibility.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
