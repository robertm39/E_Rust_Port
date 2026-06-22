<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_context_sr

## Source Files

- [CLAUSES/ccl_context_sr.h](../../../eprover/CLAUSES/ccl_context_sr.h)
- [CLAUSES/ccl_context_sr.c](../../../eprover/CLAUSES/ccl_context_sr.c)

## Purpose

Declarations for functions implementing contextual simplify-reflect (or subsumption resolution in Vampire's terminology). C v L C' v -L v R --------------------- if s(C v L) = C' v L for some subst. s C' v R

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_CONTEXT_SR`

### Globals

- None found in the source scan.

### Exported Functions

- `int ClauseContextualSimplifyReflect(ClauseSet_p set, Clause_p clause)`
- `long ClauseSetFindContextSRClauses(ClauseSet_p set, Clause_p clause, PStack_p res)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ClauseContextualSimplifyReflect`: Perform contextial-simplify-reflect with all clauses in set on clause. Return number of literals deleted.
- `ClauseSetFindContextSRClauses`: Find all clauses in set that can be contextually simplify-reflected ;-) with clause and push them onto res. ATTENTION! A clause that can be simplified in more than one way will be pushed more than once onto the stack! Returns number of clauses pushed.

### Dependencies

- `"ccl_context_sr.h"`
- `<ccl_subsumption.h>`

### Compile-Time Conditions

- `CCL_CONTEXT_SR`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_context_sr.h`, `CLAUSES/ccl_context_sr.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 206 lines, 2 scanned public declarations, 0 scanned internal function definitions, and 2 structured function-comment blocks.
- Declarations for functions implementing contextual simplify-reflect (or subsumption resolution in Vampire's terminology). C v L C' v -L v R --------------------- if s(C v L) = C' v L for some subst. s C' v R
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
