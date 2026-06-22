<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_subterm_tree

## Source Files

- [CLAUSES/ccl_subterm_tree.h](../../../eprover/CLAUSES/ccl_subterm_tree.h)
- [CLAUSES/ccl_subterm_tree.c](../../../eprover/CLAUSES/ccl_subterm_tree.c)

## Purpose

A simple mapping from terms to clauses in which this term appears as priviledged (rewriting rstricted) or unpriviledged term. the GNU Lesser General Public License. <1> Wed Aug 5 17:25:30 EDT 2009

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `BWRWPayload`
- `OverlapPayload`
- `SubtermOccCell`
- `SubtermOcc_p`
- `SubtermTree_p`
- `pl`

### Macros And Constants

- `CCL_SUBTERM_TREE`
- `SubtermOccCellAlloc()`
- `SubtermOccCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SubtermOcc_p SubtermOccAlloc(Term_p term)`
- `SubtermOcc_p SubtermTreeFindTerm(SubtermTree_p *root, Term_p term)`
- `SubtermOcc_p SubtermTreeInsertTerm(SubtermTree_p *root, Term_p term)`
- `bool SubtermTreeDeleteTermOcc(SubtermTree_p *root, Term_p term, Clause_p clause, bool restricted)`
- `bool SubtermTreeInsertTermOcc(SubtermTree_p *root, Term_p term, Clause_p clause, bool restricted)`
- `int CmpSubtermCells(const void *soc1, const void *soc2)`
- `void SubtermBWTreeFree(SubtermTree_p root)`
- `void SubtermBWTreeFreeWrapper(void *junk)`
- `void SubtermOLTreeFree(SubtermTree_p root)`
- `void SubtermOLTreeFreeWrapper(void *junk)`
- `void SubtermOccFree(SubtermOcc_p soc)`
- `void SubtermPosFree(SubtermOcc_p soc)`
- `void SubtermTreeDeleteTerm(SubtermTree_p *root, Term_p term)`
- `void SubtermTreePrint(FILE* out, SubtermTree_p root, Sig_p sig)`
- `void SubtermTreePrintDot(FILE* out, SubtermTree_p root, Sig_p sig)`
- `void SubtermTreePrintDummy(FILE* out, SubtermTree_p root, Sig_p sig)`

## Implementation Notes

### Internal Functions

- `subterm_occ_free_wrapper`
- `subterm_pos_free_wrapper`

### Source-Level Behavior

- `subterm_occ_free_wrapper`: Wrapper of type ObjFreeFun.
- `subterm_pos_free_wrapper`: Wrapper of type ObjFreeFun.
- `subterm_tree_print_dot`: Print a subterm tree in dot notation.
- `SubtermOccAlloc`: Allocate an initialized Subterm-Occurrence-Cell.
- `SubtermOccFree`: Free a Subterm-Occurrence-Cell
- `SubtermPosFree`: Free a Subterm-Occurrence-Cell with clause positions.
- `CmpSubtermCells`: Compare two SubtermOccurrence cells via their term pointers. This is a synthetic but machine-independent measure useful primarily for indexing.
- `SubtermBWTreeFree`: Free a subterm tree.
- `SubtermBWTreeFreeWrapper`: Free a subterm tree, with proper signature for FPIndexFree().
- `SubtermOLTreeFree`: Free a subterm tree.
- `SubtermOLTreeFreeWrapper`: Free a subterm tree, with proper signature for FPIndexFree().
- `SubtermTreeInsertTerm`: Return the SubtermOccNode corresponding to term, creating it if it does not exist.
- `SubtermTreeFind`: Find and return tree node with key term. Return it or NULL if no such node exists.
- `SubtermTreeDeleteTerm`: Delete the SubtermOccNode corresponding to term,
- `SubtermTreeInsertTermOcc`: Insert a term occurrence into the Subterm tree. Return false if an entry already exists, true otherwise.
- `SubtermTreeDeleteTermOcc`: Delete an indexing of clause via term.
- `SubtermTreePrint`: Print a suberm tree (only for debugging)
- `SubtermTreePrintDot`: Print a suberm tree as a subgraph in Dot notation.
- `SubtermTreePrintDummy`: Print subterm trees as "..."

### Dependencies

- `"ccl_subterm_tree.h"`
- `<ccl_clausepos_tree.h>`
- `<ccl_clauses.h>`

### Compile-Time Conditions

- `CCL_SUBTERM_TREE`
- `PRT_SUBTERM_SET_AS_TREE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_subterm_tree.h`, `CLAUSES/ccl_subterm_tree.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 625 lines, 22 scanned public declarations, 2 scanned internal function definitions, and 19 structured function-comment blocks.
- A simple mapping from terms to clauses in which this term appears as priviledged (rewriting rstricted) or unpriviledged term. the GNU Lesser General Public License. <1> Wed Aug 5 17:25:30 EDT 2009
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
