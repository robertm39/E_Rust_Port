<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_varsets

## Source Files

- [TERMS/cte_varsets.h](../../../eprover/TERMS/cte_varsets.h)
- [TERMS/cte_varsets.c](../../../eprover/TERMS/cte_varsets.c)

## Purpose

Data structures for representing sets of variables. This is similar in concept to cte_varhash.c, but for a different application and hence with different access characteristics (extremely fast lookup).

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz, Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `VarSetCell`
- `VarSetStore_p`
- `VarSet_p`

### Macros And Constants

- `CTE_VARSETS`
- `VarSetCellAlloc()`
- `VarSetCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `VarSet_p VarSetAlloc(Term_p term)`
- `VarSet_p VarSetStoreFindVarSet(VarSetStore_p *store, Term_p key)`
- `VarSet_p VarSetStoreGetVarSet(VarSetStore_p *store, Term_p key)`
- `bool VarSetContains(VarSet_p set, Term_p var)`
- `bool VarSetDeleteVar(VarSet_p set, Term_p var)`
- `bool VarSetInsert(VarSet_p set, Term_p var)`
- `void VarSetCollectVars(VarSet_p set)`
- `void VarSetFree(VarSet_p set)`
- `void VarSetInsertVarSet(VarSet_p set, VarSet_p vars)`
- `void VarSetMerge(VarSet_p set, VarSet_p set1)`
- `void VarSetReset(VarSet_p set)`
- `void VarSetStoreFree(VarSetStore_p store)`
- `void VarSetUnion(VarSet_p set, VarSet_p set1, VarSet_p set2)`

## Implementation Notes

### Internal Functions

- `varset_free_fun`

### Source-Level Behavior

- `varset_free_fun`: Wrapper of type ObjDelFun
- `varset_cmp_fun`: Compare variable sets (by value of set->t).
- `VarSetAlloc`: Allocate a variable set.
- `VarSetReset`: Remove all variables from set (and mark it invalid).
- `VarSetFree`: Free a variable set.
- `VarSetInsert`: Insert a variable into a set. Nominally a NOP if variable is already in the set, but may reorganise the underlying tree. Returns false if variable is already in the set, true otherwise.
- `VarSetInsertVarSet`: Insert all vaiables in the second set into the first set.
- `VarSetDeleteVar`: Delete variable from set. A NOP if var is not present. Returns true if the key was present.
- `VarSetContains`: Return true iff var is in set.
- `VarSetCollectVars`: Make sure that set contains all variables in set->t.
- `VarSetUnion`: Make set the union of set1 and set2.
- `VarSetMerge`: Merge the second varset into the first, destroying the former.
- `VarSetStoreFree`: Free a VarSetStore.
- `VarSetStoreFindVarSet`: Find the varset associated with key and return it. Return NULL if it does not exist.
- `VarSetStoreGetVarSet`: Find varset for key in the store. If none, create an insert new empty varset.

### Dependencies

- `"cte_varsets.h"`
- `<clb_objtrees.h>`
- `<cte_termfunc.h>`
- `<cte_termvars.h>`

### Compile-Time Conditions

- `CTE_VARSETS`

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

Source files reviewed: `TERMS/cte_varsets.h`, `TERMS/cte_varsets.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 444 lines, 16 scanned public declarations, 1 scanned internal function definitions, and 15 structured function-comment blocks.
- Data structures for representing sets of variables. This is similar in concept to cte_varhash.c, but for a different application and hence with different access characteristics (extremely fast lookup).
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
