<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_varhash

## Source Files

- [TERMS/cte_varhash.h](../../../eprover/TERMS/cte_varhash.h)
- [TERMS/cte_varhash.c](../../../eprover/TERMS/cte_varhash.c)

## Purpose

Data structures for hashing and traversing variable occurences. the GNU Lesser General Public License. <1> Wed Jul 22 04:53:43 MET DST 1998 New

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `VarHashCell`
- `VarHashEntryCell`
- `VarHashEntry_p`
- `VarHash_p`

### Macros And Constants

- `CTE_VARHASH`
- `VAR_HASH_MASK`
- `VAR_HASH_SIZE`
- `VarHashCellAlloc()`
- `VarHashCellFree(junk)`
- `VarHashEntryCellAlloc()`
- `VarHashEntryCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `(VarHashCell*)SizeMalloc(sizeof(VarHashCell)) SizeFree(junk, sizeof(VarHashCell)) static inline VarHash_p VarHashAlloc(void)`
- `(VarHashEntryCell*)SizeMalloc(sizeof(VarHashEntryCell)) SizeFree(junk, sizeof(VarHashEntryCell)) static inline VarHashEntry_p VarHashEntryAlloc(Term_p var, long value)`
- `VarHashEntry_p VarHashListFind(VarHashEntry_p list, Term_p var)`
- `int VarHashFunction(Term_p var)`
- `long VarHashAddValue(VarHash_p hash, Term_p var, long value)`
- `static inline VarHashEntry_p VarHashFind(VarHash_p hash, Term_p var)`
- `void PDArrayAddVarDistrib(PDArray_p array, Term_p term, DerefType deref, long add)`
- `void VarHashAddVarDistrib(VarHash_p hash, Term_p term, DerefType deref, long add)`
- `void VarHashEntryListFree(VarHashEntry_p list)`
- `void VarHashFree(VarHash_p junk)`

## Implementation Notes

### Internal Functions

- `VarHashAlloc`
- `VarHashEntryAlloc`
- `VarHashFind`

### Source-Level Behavior

- `VarHashEntryAlloc`: Allocate an initialized hash entry cell.
- `VarHashAlloc`: Allocate an initialized variable hash.
- `VarHashFind`: Return the entry for var in hash (NULL if non-existant).
- `VarHashEntryListFree`: Free a linear list of var hash entries.
- `VarHashFree`: Free a variable hash.
- `VarHashFunction`: Hash function, map term cell with f_code onto an index.
- `VarHashListFind`: Find an entry in the linear list of hash entries. Return NULL on failure.
- `VarHashAddValue`: If var is stored in hash, add value to its entries value, otherwise create an entry and set its value to value. Return the stored value.
- `VarHashAddVarDistrib`: Scans a term and adds the variable occurences to the hash, with each occurence being counted with the "add" value. NB: Derefs not changed, function called only with FOL arguments.
- `PDArrayAddVarDistrib`: Scans a term and adds the variable occurences to the array, with each occurence being counted with the "add" value. NB: Derefs not changed, function called only with FOL arguments.

### Dependencies

- `"cte_varhash.h"`
- `<cte_termvars.h>`

### Compile-Time Conditions

- `CTE_VARHASH`

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

Source files reviewed: `TERMS/cte_varhash.h`, `TERMS/cte_varhash.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 460 lines, 14 scanned public declarations, 3 scanned internal function definitions, and 10 structured function-comment blocks.
- Data structures for hashing and traversing variable occurences. the GNU Lesser General Public License. <1> Wed Jul 22 04:53:43 MET DST 1998 New
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
