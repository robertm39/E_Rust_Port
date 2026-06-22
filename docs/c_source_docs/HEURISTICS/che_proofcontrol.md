<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_proofcontrol

## Source Files

- [HEURISTICS/che_proofcontrol.h](../../../eprover/HEURISTICS/che_proofcontrol.h)
- [HEURISTICS/che_proofcontrol.c](../../../eprover/HEURISTICS/che_proofcontrol.c)

## Purpose

Object storing all information about control of the search process: Ordering, heuristic, similar stuff. the GNU Lesser General Public License. <1> Fri Oct 16 14:52:53 MET DST 1998

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ProofControlCell`
- `ProofControl_p`

### Macros And Constants

- `CHE_PROOFCONTROL`
- `CHE_PROOFCONTROL_INTERNAL`
- `HCBARGUMENTS`
- `ProofControlCellAlloc()`
- `ProofControlCellFree(junk)`

### Globals

- `extern char* DefaultHeuristics`
- `extern char* DefaultWeightFunctions`

### Exported Functions

- `(ProofControlCell*)SizeMalloc(sizeof(ProofControlCell)) SizeFree(junk, sizeof(ProofControlCell)) ProofControl_p ProofControlAlloc(void)`
- `HeuristicParms_p parms typedef HCB_p (*HCBCreateFun)(HCBARGUMENTS)`
- `void DoLiteralSelection(ProofControl_p control, Clause_p clause)`
- `void ProofControlFree(ProofControl_p junk)`
- `void ProofControlResetSATSolver(ProofControl_p ctrl)`

## Implementation Notes

### Internal Functions

- `select_inherited_literal`

### Source-Level Behavior

- `select_inherited_literal`: If there is at least one negative literal with EPIsPMIntoLit, select all literals with this property, return true. Otherwise return false.
- `sat_solver_init`: Create and initialize the SAT solver in the ProofControl object.
- `ProofControlAlloc`: Allocate an empty, initialized ProofControlCell.
- `ProofControlFree`: Free a ProofControlCell.
- `ProofContrlResetSATSolver`: Resets SAT solver state to make it ready for the next attempt.
- `DoLiteralSelection`: Based on control, select a literal selection strategy and apply it to clause.

### Dependencies

- `"che_proofcontrol.h"`
- `<ccl_proofstate.h>`
- `<ccl_rewrite.h>`
- `<che_hcbadmin.h>`
- `<che_to_precgen.h>`
- `<che_to_weightgen.h>`

### Compile-Time Conditions

- `CHE_PROOFCONTROL`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_proofcontrol.h`, `HEURISTICS/che_proofcontrol.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 390 lines, 9 scanned public declarations, 1 scanned internal function definitions, and 6 structured function-comment blocks.
- Object storing all information about control of the search process: Ordering, heuristic, similar stuff. the GNU Lesser General Public License. <1> Fri Oct 16 14:52:53 MET DST 1998
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
