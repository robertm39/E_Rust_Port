<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_diseq_decomp

## Source Files

- [CLAUSES/ccl_diseq_decomp.h](../../../eprover/CLAUSES/ccl_diseq_decomp.h)
- [CLAUSES/ccl_diseq_decomp.c](../../../eprover/CLAUSES/ccl_diseq_decomp.c)

## Purpose

Implement the disequality decomposition inference. f(s1,...,sn)!=f(t1,...,tn) | R s1!=t1 | ... | sn_tn | R This wraps the actual inference into one small function.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_DISEQ_DECOMP`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p ClauseDisEqDecomposition(TB_p terms, Clause_p clause, CompactPos litpos)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ClauseDisEqDecomposition`: Perform the disequality decomposition of clause as litpos.

### Dependencies

- `"ccl_diseq_decomp.h"`
- `<ccl_clausecpos.h>`
- `<ccl_clauses.h>`
- `<ccl_derivation.h>`

### Compile-Time Conditions

- `CCL_DISEQ_DECOMP`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_diseq_decomp.h`, `CLAUSES/ccl_diseq_decomp.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 157 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Implement the disequality decomposition inference. f(s1,...,sn)!=f(t1,...,tn) | R s1!=t1 | ... | sn_tn | R This wraps the actual inference into one small function.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/clauses/diseq_decomp.rs` ports `ClauseDisEqDecomposition`, including compact-position literal selection, copying all residual literals except the selected disequality, prepending generated argument-pair disequalities, appending that temporary list, and removing resolved/duplicate literals before clause allocation.
- Derivation-stack side effects (`ClausePushDerivation(..., DCDisEqDecompose, ...)`) are ported with a compact source-clause reference.

### Change Later

- C builds the generated argument-pair disequalities by pushing each new literal onto a temporary list head, so argument-pair order is reversed before append. Rust preserves this order; change it only after proof-output and search-order comparisons show it is unobservable.
- `ClauseDisEqDecomposition` assumes the selected compact position is a top-level literal and asserts equal top symbols and arities. Rust keeps those as panicking internal preconditions; a later public API could expose fallible validation if external callers need it.
- C attaches derivation metadata inside the low-level clause builder rather than in the control wrapper. Rust now keeps that ownership boundary, but the parent is a compact reference until stable clause handles exist.
<!-- END MANUAL REVIEW: c_source_docs -->
