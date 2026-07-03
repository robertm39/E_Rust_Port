<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_clausepos_tree

## Source Files

- [CLAUSES/ccl_clausepos_tree.h](../../../eprover/CLAUSES/ccl_clausepos_tree.h)
- [CLAUSES/ccl_clausepos_tree.c](../../../eprover/CLAUSES/ccl_clausepos_tree.c)

## Purpose

Associate clauses with a number of compact positions in clauses. the GNU Lesser General Public License. <1> Sun Jun 6 13:25:19 CEST 2010 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ClauseTPosCell`
- `ClauseTPosTree_p`
- `ClauseTPos_p`

### Macros And Constants

- `CCL_CLAUSEPOS_TREE`
- `ClauseTPosCellAlloc()`
- `ClauseTPosCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `ClauseTPos_p ClauseTPosAlloc(Clause_p clause)`
- `int CmpClauseTPosCells(const void *soc1, const void *soc2)`
- `void ClauseTPosFree(ClauseTPos_p soc)`
- `void ClauseTPosTreeDeleteClause(ClauseTPosTree_p *tree, Clause_p clause)`
- `void ClauseTPosTreeDeletePos(ClauseTPosTree_p *tree , Clause_p clause, CompactPos pos)`
- `void ClauseTPosTreeFree(ClauseTPosTree_p tree)`
- `void ClauseTPosTreeInsertPos(ClauseTPosTree_p *tree , Clause_p clause, CompactPos pos)`
- `void ClauseTPosTreePrint(FILE* out, ClauseTPos_p tree)`
- `void ClauseTPosTreeTreeFreeWrapper(void *junk)`

## Implementation Notes

### Internal Functions

- `clause_tpos_free_wrapper`

### Source-Level Behavior

- `clause_tpos_free_wrapper`: Wrapper of type ObjFreeFun.
- `ClauseTPosAlloc`: Allocate a ClauseTPosCell for clause clause.
- `ClauseTPosFree`: Free a ClauseTPosCell, including the position tree, but not the clause.
- `ClauseTPosTreeFree`: Free a ClauseTPOS-Tree.
- `CmpClauseTPosCells`: Compare two ClauseTPos cells via their clausepointers.
- `ClauseTPosTreeFreeWrapper`: Free a subterm tree, with proper signature for FPIndexFree().
- `ClauseTPosTreeInsertPos`: Add a clause->pos association to the tree.
- `ClauseTPosTreeDeletePos`: Delete a clause->pos association (and the clause, if there is no remaining position).
- `ClauseTPosTreeDeleteClause`: Delete all associations clause->pos for any pos from the tree.
- `ClauseTPosTreePrint`: Print a ClauseTposTree (mostly for debuging).

### Dependencies

- `"ccl_clausepos_tree.h"`
- `<ccl_clausecpos.h>`

### Compile-Time Conditions

- `CCL_CLAUSEPOS_TREE`

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

Source files reviewed: `CLAUSES/ccl_clausepos_tree.h`, `CLAUSES/ccl_clausepos_tree.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 368 lines, 12 scanned public declarations, 1 scanned internal function definitions, and 10 structured function-comment blocks.
- Associate clauses with a number of compact positions in clauses. the GNU Lesser General Public License. <1> Sun Jun 6 13:25:19 CEST 2010 New
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### C Behaviors To Revisit After Compatibility

- `ClauseTPosTreePrint` combines global `ClausePrint` output with `NumTreeDebugPrint`, which prints the actual numeric-tree shape and a `Tree size` line. Rust now provides default LOP and explicit LOP/TPTP/TSTP clause rendering over sorted compact positions; reproduce the exact numeric-tree debug output only if this path becomes compatibility-visible.
- `ClauseTPosTreePrint` is documented with no global variables, but its `ClausePrint` call observes the process-global `OutputFormat` and TSTP printing observes the process-global problem type. Rust keeps those dependencies explicit through output-format and problem-type parameters.
- The header declares `ClauseTPosTreeTreeFreeWrapper`, while the C implementation defines `ClauseTPosTreeFreeWrapper`. Keep the mismatch visible for compatibility audits before deciding whether Rust should expose only the implemented spelling or an alias for the header typo.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
