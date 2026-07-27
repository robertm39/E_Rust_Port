<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_axiomsorter

## Source Files

- [CLAUSES/ccl_axiomsorter.h](../../../eprover/CLAUSES/ccl_axiomsorter.h)
- [CLAUSES/ccl_axiomsorter.c](../../../eprover/CLAUSES/ccl_axiomsorter.c)

## Purpose

Datatypes an code for implementing generic evaluation and sorting of axiomsets (clauses and formulas). the GNU Lesser General Public License. <1> Sun Jun 14 00:31:54 CEST 2009

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `AxiomType`
- `WAxiomCell`
- `WAxiom_p`
- `ax`

### Macros And Constants

- `CCL_AXIOMSORTER`
- `WAxiomCellAlloc()`
- `WAxiomCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `WAxiom_p WAxiomAlloc(void* axiom, AxiomType type)`
- `int WAxiomCmp(WAxiom_p s1, WAxiom_p s2)`
- `int WAxiomCmpWrapper(const void* s1, const void* s2)`
- `void WAxiomAddRelEval(WAxiom_p ax, Sig_p sig, PDArray_p rel_vec)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `WAxiomAlloc`: Allocate and initialze a weighted axiom cell.
- `WAxiomAddRelEval`: Given a vector of relevance levels for the symbols, assign a clause or formula (in a WAxiom data structure) the average relevancy of its symbols.
- `WAxiomCmp`: Compare two WAxioms by weight, with some extra work to make the ordering total.
- `WAxiomCmpWrapper`: Compare two IntOrP's pointing to WAxioms by WAxiom weight.

### Dependencies

- `"ccl_axiomsorter.h"`
- `<ccl_proofstate.h>`

### Compile-Time Conditions

- `CCL_AXIOMSORTER`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_axiomsorter.h`, `CLAUSES/ccl_axiomsorter.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 280 lines, 8 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Datatypes an code for implementing generic evaluation and sorting of axiomsets (clauses and formulas). the GNU Lesser General Public License. <1> Sun Jun 14 00:31:54 CEST 2009
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `WAxiomAlloc()` accepts an `AxiomType` and initializes the union payload, but never writes `ax->type` before `WAxiomAddRelEval()` and `WAxiomCmp()` switch or compare that field. Rust initializes the type so the helper is usable; after reference call-path coverage is broader, decide whether this C uninitialized-read surface needs any compatibility shim or should remain documented as a C bug.
- `WAxiomCmp()` makes equal-weight/equal-type ordering total with raw pointer comparison. Rust uses stable object identity for the same tie break, but a cleaned relevance sorter should expose deterministic insertion/order metadata if user-visible axiom order ever needs to be independent of allocator addresses.
<!-- END MANUAL REVIEW: c_source_docs -->
