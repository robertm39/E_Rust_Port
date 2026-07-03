<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_refinedweight

## Source Files

- [HEURISTICS/che_refinedweight.h](../../../eprover/HEURISTICS/che_refinedweight.h)
- [HEURISTICS/che_refinedweight.c](../../../eprover/HEURISTICS/che_refinedweight.c)

## Purpose

Evaluation of a clause by refined clause weight, using weight penalty factors for maximal terms and literals. the GNU Lesser General Public License. <1> Wed Jun 17 00:11:03 MET DST 1998

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RefinedWeightParamCell`
- `RefinedWeightParam_p`

### Macros And Constants

- `CHE_REFINEDWEIGHT`
- `DEFAULT_MAX_MULT`
- `RefinedWeightParamCellAlloc()`
- `RefinedWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(RefinedWeightParamCell)) SizeFree(junk, sizeof(RefinedWeightParamCell)) WFCB_p ClauseRefinedWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, OCB_p ocb, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier, double app_var_mult)`
- `WFCB_p ClauseRefinedWeight2Parse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p ClauseRefinedWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double ClauseRefinedWeight2Compute(void* data, Clause_p clause)`
- `double ClauseRefinedWeightCompute(void* data, Clause_p clause)`
- `void ClauseRefinedWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ClauseRefinedWeightInit`: Return an initialized WFCB for ClauseRefinedWeight evaluation.
- `ClauseRefinedWeightParse`: Parse a refined clauseweight-definition.
- `ClauseRefinedWeightCompute`: Compute an evaluation for a clause.
- `ClauseRefinedWeight2Parse`: Refined weight counting EQ-Encoding
- `ClauseRefinedWeight2Compute`: Compute an evaluation for a clause.
- `ClauseRefinedWeightExit`: Free the data entry in a clauseweight WFCB.

### Dependencies

- `"che_refinedweight.h"`
- `<che_clauseweight.h>`

### Compile-Time Conditions

- `CHE_REFINEDWEIGHT`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_refinedweight.h`, `HEURISTICS/che_refinedweight.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 316 lines, 8 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Evaluation of a clause by refined clause weight, using weight penalty factors for maximal terms and literals. the GNU Lesser General Public License. <1> Wed Jun 17 00:11:03 MET DST 1998
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- `ClauseRefinedWeightCompute` and `ClauseRefinedWeight2Compute` call `ClauseCondMarkMaximalTerms(local->ocb, clause)` before `ClauseWeight`; the Rust port preserves that ordering with explicit OCB-backed helpers and banked WFCB callbacks for mutable-clause callers that can pass the owner bank.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
- Change later candidate: once all proof-control evaluation sites can pass both the active `OCB` and mutable owner bank, route ordinary HCB evaluation through the banked WFCB path and collapse any remaining immutable refined-weight scoring fallbacks without changing the mark-then-`ClauseWeight` sequence.
<!-- END MANUAL REVIEW: c_source_docs -->
