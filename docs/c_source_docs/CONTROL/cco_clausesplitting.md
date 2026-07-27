<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_clausesplitting

## Source Files

- [CONTROL/cco_clausesplitting.h](../../../eprover/CONTROL/cco_clausesplitting.h)
- [CONTROL/cco_clausesplitting.c](../../../eprover/CONTROL/cco_clausesplitting.c)

## Purpose

The interface functions for controlled clause splitting. the GNU Lesser General Public License. <1> Fri Apr 27 20:13:53 MET DST 2001 New

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_CLAUSESPLITTING`

### Globals

- None found in the source scan.

### Exported Functions

- `int ControlledClauseSplit(DefStore_p store, Clause_p clause, ClauseSet_p set, SplitClassType which, SplitType how, bool fresh_defs)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ControlledClauseSplit`: Check if the clause meets one of the criteria for splitting, if yes try to split it. Return number of new clauses if splitting occurs, 0 otherwise.

### Dependencies

- `"cco_clausesplitting.h"`
- `<ccl_splitting.h>`

### Compile-Time Conditions

- `CCO_CLAUSESPLITTING`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_clausesplitting.h`, `CONTROL/cco_clausesplitting.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 149 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- The interface functions for controlled clause splitting. the GNU Lesser General Public License. <1> Fri Apr 27 20:13:53 MET DST 2001 New
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- Rust now ports `ControlledClauseSplit` for represented first-order clauses: the split-class mask checks for Horn, non-Horn, negative, positive, and mixed clauses are preserved, and matching clauses call the `ClauseSplit` port before requeueing results through `tmp_store`.
- Fresh definitions and non-fresh clause-level definition reuse are both supported. Arity-zero split-definition formula archives are represented for proof-state splitting, and the proof-control split branches now emit opt-in `DocIntroSplitDef`, `DocIntroSplitDefRest`, and `DocClauseApplyDefs` output before requeueing split children.

### Change Later

- `SplitAll` is still the C value `7`, so the wrapper's Horn/non-Horn checks make it effectively broad even though the mask does not include the later positive/mixed bits. Rust preserves this rather than normalizing the mask.
- The C wrapper receives a full `DefStore_p`; Rust currently threads the reusable clause store, represented arity-zero formula archive parents, and predicate association through proof state. Reintroduce a fuller owner at this boundary if later set-level callers need parameterized split-variable formula archives or helper-local proof-output ownership.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
