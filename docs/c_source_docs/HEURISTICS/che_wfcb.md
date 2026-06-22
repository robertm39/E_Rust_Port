<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_wfcb

## Source Files

- [HEURISTICS/che_wfcb.h](../../../eprover/HEURISTICS/che_wfcb.h)
- [HEURISTICS/che_wfcb.c](../../../eprover/HEURISTICS/che_wfcb.c)

## Purpose

Weigth-function-Control blocks, functions computing weights for clauses. The interface to an evaluation function requires 3 or 4 functions: WFCB_p <eval>Init(PrioFun prio, &rest) This function takes a prority function and optional arguments, and return a WFCB. In particular, it is responsible for creating

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ClauseEvalFun`
- `WFCBCell`
- `WFCB_p`
- `WeightFunParseFun`

### Macros And Constants

- `CHE_WFCB`
- `WFCBCellAlloc()`
- `WFCBCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `WFCB_p WFCBAlloc(ClauseEvalFun wfcb_eval, ClausePrioFun prio_fun, GenericExitFun wfcb_exit, void* data)`
- `void ClauseAddEvaluation(WFCB_p wfcb, Clause_p clause, int pos, bool empty)`
- `void WFCBFree(WFCB_p junk)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `WFCBAlloc`: Create and return an initialized WFCB-block.
- `WFCBFree`: Free a WFCB.
- `ClauseAddEvaluation`: Given a clause and a wfcb, add an evaluation to the clause.

### Dependencies

- `"che_wfcb.h"`
- `<ccl_proofstate.h>`
- `<che_prio_funs.h>`
- `<cio_output.h>`
- `<clb_dstacks.h>`

### Compile-Time Conditions

- `CHE_WFCB`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_wfcb.h`, `HEURISTICS/che_wfcb.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 227 lines, 7 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Weight-function control blocks; preserve parameter parsing and evaluation dispatch.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
