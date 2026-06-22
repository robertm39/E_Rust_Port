<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROPOSITIONAL / cpr_propsig

## Source Files

- [PROPOSITIONAL/cpr_propsig.h](../../../eprover/PROPOSITIONAL/cpr_propsig.h)
- [PROPOSITIONAL/cpr_propsig.c](../../../eprover/PROPOSITIONAL/cpr_propsig.c)

## Purpose

Definitions for dealing with signatures for propositional variables - essentially juat associating a name with an internal number and vice versa. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PROPOSITIONAL`. Propositional abstraction and DPLL support: propositional signatures, clauses, formulas, variable sets, and solver routines.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PLiteralCode`
- `PropSigCell`
- `PropSig_p`

### Macros And Constants

- `CPR_PROPSIG`
- `PAtomP(code)`
- `PLiteralNoLit`
- `PropSigAtomNumber(psig)`
- `PropSigCellAlloc()`
- `PropSigCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `PLiteralCode PropSigGetAtomEnc(PropSig_p psig, char* name)`
- `PLiteralCode PropSigInsertAtom(PropSig_p psig, char* name)`
- `PropSig_p PropSigAlloc(void)`
- `char* PropSigGetAtomName(PropSig_p psig, PLiteralCode atom)`
- `void PropSigFree(PropSig_p junk)`
- `void PropSigPrint(FILE* out, PropSig_p sig)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PropSigAlloc`: Allocate an empty, initialized propositional signature.
- `PropSigFree`: Free a propositional signature and all associated memory.
- `PropSigGetAtomEnc`: Given a name, return the encoding. Return PAtomNoAtom if name is unknown.
- `PropSigInsertAtom`: Insert a new atom into a psig. Return encoding. If atom already exists, do nothing but returning the encoding.
- `PropSigGetAtomName`: Return a pointer name of an atom. Fail on assertion if no valid atom is passed.
- `PSigPrint`: Print a PSig (mainly for debugging)

### Dependencies

- `"cpr_propsig.h"`
- `<clb_pdarrays.h>`
- `<clb_stringtrees.h>`

### Compile-Time Conditions

- `CPR_PROPSIG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROPOSITIONAL/cpr_propsig.h`, `PROPOSITIONAL/cpr_propsig.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PROPOSITIONAL` covering 2 source file(s), about 284 lines, 9 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Definitions for dealing with signatures for propositional variables - essentially juat associating a name with an internal number and vice versa. the GNU Lesser General Public License.
- Propositional reasoning code. Keep DPLL state transitions, propositional signatures, and clause/formula conversions compatible with callers.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
