<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_global_indices

## Source Files

- [CLAUSES/ccl_global_indices.h](../../../eprover/CLAUSES/ccl_global_indices.h)
- [CLAUSES/ccl_global_indices.c](../../../eprover/CLAUSES/ccl_global_indices.c)

## Purpose

Code abstracting several (optional) indices into one structure. the GNU Lesser General Public License. <1> Fri May 7 21:13:39 CEST 2010 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `GlobalIndices`
- `GlobalIndices_p`

### Macros And Constants

- `CCL_GLOBAL_INDICES`
- `GetExtFromIdx(g)`
- `GetExtIntoIdx(g)`
- `GetExtMaxDepth(g)`
- `SetExtFromIdx(g, v)`
- `SetExtIntoIdx(g, v)`
- `SetExtMaxDepth(g, v)`

### Globals

- None found in the source scan.

### Exported Functions

- `PERF_CTR_DECL(BWRWIndexTimer)`
- `PERF_CTR_DECL(PMIndexTimer)`
- `void GlobalIndicesDeleteClause(GlobalIndices_p indices, Clause_p clause, bool lambda_demod)`
- `void GlobalIndicesFreeIndices(GlobalIndices_p indices)`
- `void GlobalIndicesInit(GlobalIndices_p indices, Sig_p sig, char* rw_bw_index_type, char* pm_from_index_type, char* pm_into_index_type, int ext_rules_max_depth)`
- `void GlobalIndicesInsertClause(GlobalIndices_p indices, Clause_p clause, bool lambda_demod)`
- `void GlobalIndicesInsertClauseSet(GlobalIndices_p indices, ClauseSet_p set, bool lambda_demod)`
- `void GlobalIndicesNull(GlobalIndices_p indices)`
- `void GlobalIndicesReset(GlobalIndices_p indices)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `GlobalIndicesNull`: Set the global indices to NULL.
- `GlobalIndicesInit`: Initialize the global indices as required by the parameters.
- `GlobalIndicesFreeIndices`: Free the existing indices.
- `GlobalIndicesReset`: Reset all exisiting indices.
- `GlobalIndicesInsertClause`: Add a clause to all exisiting global indices.
- `GlobalIndicesDeleteClause`: Remove a clause from all exisiting global indices.
- `GlobalIndicesInsertClauseSet`: Insert all clause in set into the indices.

### Dependencies

- `"ccl_global_indices.h"`
- `<ccl_clausesets.h>`
- `<ccl_ext_index.h>`
- `<ccl_overlap_index.h>`
- `<ccl_subterm_index.h>`

### Compile-Time Conditions

- `CCL_GLOBAL_INDICES`
- `ENABLE_LFHO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_global_indices.h`, `CLAUSES/ccl_global_indices.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 453 lines, 13 scanned public declarations, 0 scanned internal function definitions, and 7 structured function-comment blocks.
- Code abstracting several (optional) indices into one structure. the GNU Lesser General Public License. <1> Fri May 7 21:13:39 CEST 2010 New
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
