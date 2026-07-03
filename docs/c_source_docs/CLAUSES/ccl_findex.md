<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_findex

## Source Files

- [CLAUSES/ccl_findex.h](../../../eprover/CLAUSES/ccl_findex.h)
- [CLAUSES/ccl_findex.c](../../../eprover/CLAUSES/ccl_findex.c)

## Purpose

Implementation of function symbol indexing into clauses/formulas. the GNU Lesser General Public License. <1> Sun May 31 11:20:27 CEST 2009 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FIndexCell`
- `FIndex_p`

### Macros And Constants

- `CCL_FINDEX`
- `FIndexCellAlloc()`
- `FIndexCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `FIndex_p FIndexAlloc(void)`
- `void FIndexAddClause(FIndex_p index, Clause_p clause)`
- `void FIndexAddClauseSet(FIndex_p index, ClauseSet_p set)`
- `void FIndexAddPLClause(FIndex_p index, PList_p lclause)`
- `void FIndexAddPLClauseSet(FIndex_p index, PList_p set)`
- `void FIndexAddPLFormula(FIndex_p index, PList_p lform)`
- `void FIndexAddPLFormulaSet(FIndex_p index, PList_p set)`
- `void FIndexFree(FIndex_p junk)`
- `void FIndexRemovePLClause(FIndex_p index, PList_p lclause)`
- `void FIndexRemovePLFormula(FIndex_p index, PList_p lform)`
- `void FindexRemoveClause(FIndex_p index, Clause_p clause)`

## Implementation Notes

### Internal Functions

- `findex_add_instance`
- `findex_remove_instance`

### Source-Level Behavior

- `findex_add_instance`: Add an instance (of clause or formula) into index with function symbol i.
- `findex_remove_instance`: Add an instance (of clause or formula) from index with function symbol i.
- `FIndexAlloc`: Allocate an empty FIndex.
- `FIndexFree`: Free an FIndex.
- `FIndexAddClause`: Add a clause to the FIndex.
- `FIndexRemoveClause`: Remove a clause from the FIndex.
- `FIndexAddClauseSet`: Build a FIndex from clauses in set.
- `FIndexAddPLClause`: Add PListCell containing a clause as payload to the index.
- `FIndexRemovePLClause`: Remove a PListCell conaining a clause from the FIndex.
- `FIndexAddPLClauseSet`: Add all the clauses in a PList to the index.
- `FIndexAddPLFormula`: Add PListCell containing a formula as payload to the index.
- `FIndexRemovePLFormula`: Remove a PListCell conaining a formula from the FIndex.
- `FIndexAddPLFormulaSet`: Add all the formulas in a PList to the index.

### Dependencies

- `"ccl_findex.h"`
- `<ccl_clausesets.h>`
- `<clb_plist.h>`

### Compile-Time Conditions

- `CCL_FINDEX`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_findex.h`, `CLAUSES/ccl_findex.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 482 lines, 13 scanned public declarations, 2 scanned internal function definitions, and 13 structured function-comment blocks.
- Implementation of function symbol indexing into clauses/formulas. the GNU Lesser General Public License. <1> Sun May 31 11:20:27 CEST 2009 New
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- The index maps each function code to a `PTree` of raw clause/formula pointers or raw `PList` cell pointers. Duplicate suppression is pointer-identity based rather than structural.
- PList-backed add/remove helpers recompute all function codes from the current clause/formula payload when removing a cell, so callers must remove from the index before mutating the payload's symbols.
- Rust now ports the PList-backed formula indexing path over represented `WrappedFormula` cells for `ccl_relevance` formula-owner pruning; plain clause indexing and PList clause indexing remain available for existing clause callers.

### Change Later

- `extract_new_core` in `ccl_relevance` observes the `PTree` root for a function-code bucket. Because the bucket is keyed by raw addresses and the root is affected by splay operations, exact extraction order is allocator- and history-sensitive. Rust should keep a deterministic handle order unless reference compatibility requires modeling this incidental root choice.
- Formula and clause index helpers are duplicated in C; Rust can further share the indexing logic once stable owners for both payload kinds replace the remaining temporary identifier bridges.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
