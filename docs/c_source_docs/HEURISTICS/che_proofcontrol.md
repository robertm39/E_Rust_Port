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

### Compatibility Notes

- `ProofControlAlloc` initializes `ocb` and `hcb` to `NULL`, allocates the WFCB and HCB admins, sets `ac_handling_active=false`, initializes `heuristic_parms`, and eagerly creates a PicoSAT solver with trace generation enabled. It does not initialize `fvi_parms` or `problem_specs`; those are filled by `ProofControlInit` in `CONTROL/cco_proofproc.c`.
- `ProofControlFree` owns and frees `ocb`, `wfcbs`, and `hcbs`, but intentionally does not free `hcb` separately because the selected HCB is owned by the HCB admin. Rust should keep the selected HCB as a borrowed/admin handle unless later proof-control ownership changes are proven compatible.
- `ProofControlResetSATSolver` resets the existing PicoSAT instance and immediately initializes a replacement. Rust currently models this as solver lifecycle state and advances that state after each completed internal SATCheck until PicoSAT integration is ported.
- `DoLiteralSelection` first clears all selected-literal bits and the clause-oriented property, then tries inherited paramodulation literal selection before applying the configured selector. Rust now ports that wrapper behavior, including inherited selection, literal-count/weight gates, no-op/simple non-orienting selectors, and a banked wrapper for ordering-dependent `che_litselection` bodies whose Rust maximality marking needs an explicit term bank.
- `ForwardModifyClause` emits rewrite proof-documentation from the demodulation normalizer when C's `OutputLevel >= 4` gate is open, emits minimization documentation after superfluous-literal cleanup, emits condensation documentation after successful condensation, and emits simplify-reflect documentation after each removed literal. Rust exposes these through an explicit `ProofDocSession` wrapper for the represented rewrite/minimization/condensation/simplify-reflect path; broader proof-control output routing remains separate from the plain mutation helper.
- `ForwardModifyClause` does not have a C-side blanket higher-order/non-empty-OCB guard: higher-order mode runs extra normalization/pruning hooks but still orients through `control->ocb`. Rust now preserves this for first-order-shaped higher-order clauses and demodulators, plus visible LFHO DB/lambda/phony surfaces, when the OCB uses KBO6 with `LFHO_ORDER`. KBO6 `LAMBDA_ORDER` still needs owner-bank beta/eta normalization before it can be exposed here; non-KBO6 higher-order OCBs remain behind an explicit diagnostic.

### Change-Later Observations

- Eager PicoSAT allocation in `ProofControlAlloc` and eager reallocation in `ProofControlResetSATSolver` may be unnecessary for runs that never perform SAT checks. Rust preserves the lifecycle signal around the current internal solver; consider lazy external solver ownership only once reference tests cover SAT-check timing and resource use.
- The proof-control cell is split across allocation here and `ProofControlInit` in `CONTROL/cco_proofproc.c`; C leaves some fields unset between those calls. Rust should prefer initialized values at public boundaries, while keeping tests around the C initialization phases so missing initialization is not accidentally treated as a usable state.
- C `DoLiteralSelection` can call ordering-dependent selector functions with only `control->ocb` because the surrounding C clause and term representation carries the remaining context implicitly. Rust currently exposes an explicit banked wrapper for these selectors; collapse the bankless/banked split once the full proof-state owner can pass the active term bank at every selection call site.
- C couples `ForwardModifyClause` rewrite/minimization/condensation/simplify-reflect documentation to global output-level and clause-id state while mutating the clause. Rust keeps that compatibility behavior behind the documented wrapper; revisit whether the eventual proof-output owner should route all proof-control documentation through one session rather than exposing per-helper wrappers.
- C uses process-global `problemType` to decide whether `ForwardModifyClause` runs higher-order hooks, while the actual ordering risk depends on term surfaces, demodulator contents, and the selected higher-order ordering backend. Rust uses an explicit problem type plus an ordering-capability guard; revisit this boundary once KBO6 `LAMBDA_ORDER` and owner-bank normalization are implemented so the cleaned API can express the needed capability directly.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
