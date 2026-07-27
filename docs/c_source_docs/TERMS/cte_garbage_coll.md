<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_garbage_coll

## Source Files

- [TERMS/cte_garbage_coll.h](../../../eprover/TERMS/cte_garbage_coll.h)
- [TERMS/cte_garbage_coll.c](../../../eprover/TERMS/cte_garbage_coll.c)

## Purpose

Support for the termcell garbage collection. This allows the association of all clause- and formulasets with a term bank. the GNU Lesser General Public License. New

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `GCAdminCell`
- `GCAdmin_p`

### Macros And Constants

- `CTE_GARBAGE_COLL`
- `GCAdminCellAlloc()`
- `GCAdminCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `GCAdmin_p GCAdminAlloc()`
- `void GCAdminFree(GCAdmin_p junk)`
- `void GCDeregisterClauseSet(GCAdmin_p gc, void* set)`
- `void GCDeregisterFormulaSet(GCAdmin_p gc, void *set)`
- `void GCRegisterClauseSet(GCAdmin_p gc, void* set)`
- `void GCRegisterFormulaSet(GCAdmin_p gc, void* set)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `GCAdminAlloc`: Allocate an initialized GCAdminCell.
- `GCAdminFree`: Free a GCAdmin Cell.
- `GCRegisterFormulaSet`: Register a formula set as containing relevant terms.
- `GCRegisterClauseSet`: Register a clause set as containing relevant terms.
- `GCDeregisterFormulaSet`: Unregister a formula set as containing relevant terms.
- `GCDeregisterClauseSet`: Unregister a clause set as containing relevant terms.

### Dependencies

- `"cte_garbage_coll.h"`
- `<clb_ptrees.h>`

### Compile-Time Conditions

- `CTE_GARBAGE_COLL`

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

Source files reviewed: `TERMS/cte_garbage_coll.h`, `TERMS/cte_garbage_coll.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 246 lines, 8 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Support for the termcell garbage collection. This allows the association of all clause- and formulasets with a term bank. the GNU Lesser General Public License. New
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- `GCAdmin` stores borrowed clause/formula set addresses in generic pointer trees and does not own, type-check, or lifetime-check those registrations. Rust should preserve the observable registration/deregistration behavior for compatibility, but future proof-state and helper APIs should use typed stable owner handles instead of raw-address identity.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
