<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROPOSITIONAL / cpr_varset

## Source Files

- [PROPOSITIONAL/cpr_varset.h](../../../eprover/PROPOSITIONAL/cpr_varset.h)
- [PROPOSITIONAL/cpr_varset.c](../../../eprover/PROPOSITIONAL/cpr_varset.c)

## Purpose

Data type for (multi-)sets of propositional variables, currently organized as doubly linked lists. the GNU Lesser General Public License. <1> Tue May 13 21:37:34 CEST 2003

Within the source tree, this unit belongs to `PROPOSITIONAL`. Propositional abstraction and DPLL support: propositional signatures, clauses, formulas, variable sets, and solver routines.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `AtomSetCell`
- `AtomSet_p`

### Macros And Constants

- `AtomSetCellAlloc()`
- `AtomSetCellFree(junk)`
- `AtomSetEmpty(set)`
- `CPR_VARSET`

### Globals

- None found in the source scan.

### Exported Functions

- `AtomSet_p AtomSetAlloc(void)`
- `PLiteralCode AtomSetExtract(AtomSet_p var)`
- `void AtomSetFree(AtomSet_p set)`
- `void AtomSetInsert(AtomSet_p set, PLiteralCode atom)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `AtomSetAlloc`: Allocate an empty atom set.
- `AtomSetFree`: Free an atom set. Not extremely efficient (but I doubt it has to be).
- `AtomSetExtract`: Extract the atom of the cell pointed to, and return it.
- `AtomSetInsert`: Insert an atom into the atom set.

### Dependencies

- `"cpr_varset.h"`
- `<cpr_propsig.h>`

### Compile-Time Conditions

- `CPR_VARSET`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROPOSITIONAL/cpr_varset.h`, `PROPOSITIONAL/cpr_varset.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PROPOSITIONAL` covering 2 source file(s), about 215 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Data type for (multi-)sets of propositional variables, currently organized as doubly linked lists. the GNU Lesser General Public License. <1> Tue May 13 21:37:34 CEST 2003
- Propositional reasoning code. Keep DPLL state transitions, propositional signatures, and clause/formula conversions compatible with callers.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/propositional/varset.rs` ports `AtomSetAlloc`, `AtomSetEmpty`, `AtomSetInsert`, `AtomSetExtract`, and the drain behavior used by `AtomSetFree`.
- The Rust port represents cells with stable index handles owned by `AtomSet` instead of exposing the sentinel/list nodes as raw pointers. It preserves insertion after the sentinel, LIFO extraction from `set->succ`, arbitrary live-cell extraction, duplicate atoms, and the C assertion that `PLiteralNoLit` must not be extracted.
- The shared propositional literal vocabulary starts in `src/propositional/mod.rs` with `PLiteralCode`, `PLiteralNoLit`, and `PAtomP` equivalents so later `cpr_propsig`, `cpr_propclauses`, `cpr_dpllformula`, and `cpr_dpll` ports can use the same encoding.

### Change Later

- C uses `AtomSet_p` for both the set sentinel and ordinary cells, so callers can accidentally pass the sentinel, a stale cell, or a `PLiteralNoLit` payload to `AtomSetExtract`; the Rust API separates the owner from cell handles but keeps assertion-shaped failures for invalid extraction. Once all DPLL callers are ported and compatibility-tested, consider validating `PLiteralNoLit` at insertion or using a stronger typed literal wrapper.
- `AtomSetFree` drains the circular list one cell at a time even though atom cells have no owned payload beyond the code. Rust mirrors the observable extraction order; after performance baselines are available, a contiguous worklist representation may be simpler if no caller relies on arbitrary live-cell extraction.
<!-- END MANUAL REVIEW: c_source_docs -->
