<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termcellstore

## Source Files

- [TERMS/cte_termcellstore.h](../../../eprover/TERMS/cte_termcellstore.h)
- [TERMS/cte_termcellstore.c](../../../eprover/TERMS/cte_termcellstore.c)

## Purpose

Abstract interface for storing term cells, implemented by a combination of a hashed array and term cell trees. Use (term->f_code^term->args[1])&TERM_STORE_HASH_MASK as hash if args != NULL, otherwise use term->f_code&TERM_STORE_HASH_MASK.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TermCellStoreCell`
- `TermCellStore_p`

### Macros And Constants

- `CTE_TERMCELLSTORE`
- `TERM_STORE_HASH_MASK`
- `TERM_STORE_HASH_SIZE`
- `TermCellHash(term)`
- `TermCellStoreNodes(store)`
- `tcs_arity0hash(term)`
- `tcs_arity1hash(term)`
- `tcs_aritynhash(term)`

### Globals

- None found in the source scan.

### Exported Functions

- `Term_p TermCellStoreExtract(TermCellStore_p store, Term_p term)`
- `Term_p TermCellStoreFind(TermCellStore_p store, Term_p term)`
- `Term_p TermCellStoreInsert(TermCellStore_p store, Term_p term)`
- `bool TermCellStoreDelete(TermCellStore_p store, Term_p term)`
- `long TermCellStoreCountNodes(TermCellStore_p store)`
- `long TermCellStoreGCSweep(TermCellStore_p store, TermProperties gc_state )`
- `void TermCellStoreDelProp(TermCellStore_p store, TermProperties props)`
- `void TermCellStoreExit(TermCellStore_p store)`
- `void TermCellStorePrintDistrib(FILE* out, TermCellStore_p store)`
- `void TermCellStoreSetProp(TermCellStore_p store, TermProperties props)`

## Implementation Notes

### Internal Functions

- `collect_unmarked_termcells`

### Source-Level Behavior

- `collect_unmarked_termcells`: Push the addresses of all unmarked term cells in the tree onto the stack.
- `TermCellStoreInit`: Initialize a term cell storage.
- `TermCellStoreExit`: Free the trees in a term cell storage.
- `TermCellStoreFind`: Find a term cell in the store.
- `TermCellStoreInsert`: Insert a term cell into the store.
- `TermCellStoreExtract`: Extract a term cell from the store, return it.
- `TermCellStoreDelete`: Delete a node from the store.
- `TermCellStoreSetProp`: Set the given properties in all term cells in store.
- `TermCellStoreDelProp`: Delete the given properties in all term cells in store.
- `TermCellStoreCountNodes`: Return the number of nodes in the term cell store.
- `TermCellStoreGCSweep`: Sweep the term cell store and free unmarked cells. Return number of cells recovered. Note that we separate the collection of unmarked terms from the actual deletion, since walking the trees while they may be reorganized is somewhere between messy and impossible.
- `TermCellStorePrintDistrib`: For each entry (hash value) in store, print the number of term cells in the corresponding tree.

### Dependencies

- `"cte_termcellstore.h"`
- `<cte_termtrees.h>`

### Compile-Time Conditions

- `CTE_TERMCELLSTORE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_termcellstore.h`, `TERMS/cte_termcellstore.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 460 lines, 12 scanned public declarations, 1 scanned internal function definitions, and 12 structured function-comment blocks.
- Term-cell storage allocator; preserve reuse patterns and sharing assumptions for term-heavy workloads.
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
