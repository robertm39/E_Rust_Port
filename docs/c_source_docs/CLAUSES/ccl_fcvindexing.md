<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_fcvindexing

## Source Files

- [CLAUSES/ccl_fcvindexing.h](../../../eprover/CLAUSES/ccl_fcvindexing.h)
- [CLAUSES/ccl_fcvindexing.c](../../../eprover/CLAUSES/ccl_fcvindexing.c)

## Purpose

Functions for handling frequency count vector indexing for clause subsumption. the GNU Lesser General Public License. <1> Tue Jul 1 13:05:36 CEST 2003

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FVIAnchorCell`
- `FVIAnchor_p`
- `FVIndexCell`
- `FVIndexParmsCell`
- `FVIndexParms_p`
- `FVIndex_p`
- `u1`

### Macros And Constants

- `CCL_FCVINDEXING`
- `FVIAnchorCellAlloc()`
- `FVIAnchorCellFree(junk)`
- `FVINDEX_MEM`
- `FVIndexCellAlloc()`
- `FVIndexCellFree(junk)`
- `FVIndexParmsCellAlloc()`
- `FVIndexParmsCellFree(junk)`
- `FVIndexParmsFree(junk)`
- `FVIndexStorage(index)`

### Globals

- None found in the source scan.

### Exported Functions

- `FVIAnchor_p FVIAnchorAlloc(FVCollect_p cspec, PermVector_p perm)`
- `FVIndexParms_p FVIndexParmsAlloc(void)`
- `FVIndex_p FVIndexAlloc(void)`
- `FVIndex_p FVIndexGetNextNonEmptyNode(FVIndex_p node, long key)`
- `FVPackedClause_p FVIndexPackClause(Clause_p clause, FVIAnchor_p anchor)`
- `PERF_CTR_DECL(FVIndexTimer)`
- `bool FVIndexDelete(FVIAnchor_p index, Clause_p clause)`
- `long FVIndexCountNodes(FVIndex_p index, bool leaves, bool empty)`
- `void FVIAnchorFree(FVIAnchor_p junk)`
- `void FVIndexFree(FVIndex_p junk)`
- `void FVIndexInsert(FVIAnchor_p index, FreqVector_p vec_clause)`
- `void FVIndexParmsInit(FVIndexParms_p parms)`
- `void FVIndexPrint(FILE* out, FVIndex_p index, bool fullterms)`

## Implementation Notes

### Internal Functions

- `insert_empty_node`

### Source-Level Behavior

- `print_lvl`: Prints enough dashes to indent a tree level.
- `print_clauses`: Prints clauses stored in the leaf indented with level.
- `fv_index_print`: Driver function for printing fv index. To be initially called with root for index and 0 for level.
- `insert_empty_node`: Insert an empty node into FVIndex at node node and key key.
- `FVIndexParmsInit`: Initialize a FVIndexParmCell with rational values.
- `FVIndexParmsAlloc`: Allocate an FVIndexParmsCell with rational values.
- `FVIndexAlloc`: Allocate an empty and initialize FVIndexCell.
- `FVIndexFree`: Free a FVIndex - recursively and slightly complex because of the weird structure...
- `FVIAnchorAlloc`: Allocate an (empty) FV index.
- `FVIAnchorFree`: Free a FV incex.
- `FVIndexGetNextNonEmptyNode`: Get the next node if it is not empty. Otherwise return NULL.
- `FVIndexInsert`: Insert a FreqVector (with associated clause) into the index.
- `FVIndexDelete`: Delete a clause from a FVIndex. At the moment, just removes the clause from the final cell (I expect removals to be rare enough that it pays to leave the structure intact. Returns true if the clause was in the index, false otherwise.
- `FVIndexCountNodes`: Count the number of nodes. If empty is true, count empty leaves only. If leaves it true, count leaves only.
- `FVIndexPackClause`: Pack a clause into an apropriate FVPackedClauseStructure for the index.
- `FVIndexPrint`: Pretty prints FVIndex.

### Dependencies

- `"ccl_fcvindexing.h"`
- `<ccl_freqvectors.h>`
- `<clb_intmap.h>`

### Compile-Time Conditions

- `CCL_FCVINDEXING`
- `CONSTANT_MEM_ESTIMATE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-07-17.

Source files reviewed: `CLAUSES/ccl_fcvindexing.h`, `CLAUSES/ccl_fcvindexing.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 699 lines, 21 scanned public declarations, 1 scanned internal function definitions, and 16 structured function-comment blocks.
- Functions for handling frequency count vector indexing for clause subsumption. the GNU Lesser General Public License. <1> Tue Jul 1 13:05:36 CEST 2003
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- `FVIndexPrint` accepts an `out` stream, but `FVIndexPrint`, `fv_index_print`, and `print_clauses` write the root marker, alternatives, and leaf newlines to `stderr` while indentation and clause text are written to `out`. Rust preserves both observable cases: its ordinary pure renderer returns the interleaved human-readable tree produced when `out == stderr`, while explicit split-stream LOP/TPTP/TSTP renderers return the exact independent byte strings for distinct streams. After compatibility, C should route the whole diagnostic through `out` and the Rust compatibility surface can be reconsidered.
- Final-leaf clause lines are indented one level deeper than the final node level because `print_clauses` receives `level+1`. Rust preserves this visible indentation in the default LOP and format-aware tree renderers.
- `print_clauses` is documented with no global variables, but its `ClausePrint` call observes the process-global `OutputFormat` and TSTP printing observes the process-global problem type. Rust keeps these dependencies explicit through output-format and problem-type parameters.
- `FVIndexStorage` is an insertion-side compatibility counter, not live memory usage: it charges new successor `IntMap` storage and `FVIndexCell` allocations, can decrease when an `IntMap` switches from tree to array, and does not decrement for `FVIndexDelete` or count final-leaf `PTree` clause storage. Rust preserves the signed representation-transition deltas and cumulative counter with C constant-memory estimates; later profiling APIs should expose actual Rust allocation or logical index statistics separately rather than changing this compatibility value.
- `FVIndexInsert` stores raw `Clause_p` pointers in the final-node `PTree`, and `FVIndexDelete` deletes by that same pointer after recomputing the vector. Rust FV indexes store owned/cloned clause snapshots, so the port keys leaves by the clause identifier to survive safe value moves through `ClauseSet`; replace this with stable typed clause handles once long-lived shared clause ownership is available.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
