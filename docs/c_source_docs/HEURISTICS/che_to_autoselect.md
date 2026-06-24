<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_to_autoselect

## Source Files

- [HEURISTICS/che_to_autoselect.h](../../../eprover/HEURISTICS/che_to_autoselect.h)
- [HEURISTICS/che_to_autoselect.c](../../../eprover/HEURISTICS/che_to_autoselect.c)

## Purpose

Functions dealing with the automatic selection of a (suitable?) term ordering. the GNU Lesser General Public License. <1> Thu Dec 31 17:39:46 MET 1998

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OrderEvaluationFun`

### Macros And Constants

- `CHE_HEURISTICS_AUTO`
- `CHE_HEURISTICS_AUTO_CASC`
- `CHE_HEURISTICS_AUTO_DEV`
- `CHE_HEURISTICS_AUTO_SCHED0`
- `CHE_HEURISTICS_AUTO_SCHED1`
- `CHE_HEURISTICS_AUTO_SCHED2`
- `CHE_HEURISTICS_AUTO_SCHED3`
- `CHE_HEURISTICS_AUTO_SCHED4`
- `CHE_HEURISTICS_AUTO_SCHED5`
- `CHE_HEURISTICS_AUTO_SCHED6`
- `CHE_HEURISTICS_AUTO_SCHED7`
- `CHE_HEURISTICS_AUTO_SCHED8`
- `CHE_HEURISTICS_AUTO_SCHED9`
- `CHE_TO_AUTOSELECT`
- `KBO_BONUS`
- `MAX_CONST_WEIGHT`
- `MAX_LITERAL_PENALTY`
- `MAX_TERM_PENALTY`
- `OrderParmsCellAlloc()`
- `OrderParmsCellFree(junk)`
- `TO_ORDERING_INTERNAL`
- `UNORIENT_LITERAL_PENALTY`

### Globals

- None found in the source scan.

### Exported Functions

- `(OrderParmsCell*)SizeMalloc(sizeof(OrderParmsCell)) SizeFree(junk, sizeof(OrderParmsCell)) double OrderEvaluate(OCB_p ocb, ProofState_p state, HeuristicParms_p params)`
- `OCB_p OrderFindOptimal(OrderParms_p mask, OrderEvaluationFun eval_fun, ProofState_p state, HeuristicParms_p params)`
- `OCB_p TOCreateOrdering(ProofState_p state, OrderParms_p params, char* pre_precedence, char* pre_weights)`
- `OCB_p TOSelectOrdering(ProofState_p state, HeuristicParms_p params, SpecFeature_p specs)`
- `bool OrderNextConstWeight(OrderParms_p ordering)`
- `bool OrderNextOrdering(OrderParms_p ordering, OrderParms_p mask)`
- `bool OrderNextPrecGen(OrderParms_p ordering)`
- `bool OrderNextType(OrderParms_p ordering)`
- `bool OrderNextWeightGen(OrderParms_p ordering)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `generate_auto_ordering`: Generate a term ordering suitable to the problem in state.
- `generate_autocasc_ordering`: Generate a term ordering suitable to the problem in state. This is the CASC-20 auto mode
- `generate_autodev_ordering`: Generate a term ordering suitable to the problem in state.
- `generate_autosched0_ordering`: Generate term orderings according to the selected auto-schedule mode.
- `OrderEvaluate`: Given an OCB, evaluate the resulting ordering on the axioms of state. Low is good.
- `OrderNextType`: In an implicit ordering on TermOrdering, set ordering->ordertype to the next value (if it exists) and return true. Set it to NoOrdering and return false otherwise.
- `OrderNextWeightGen`: Set ordering->to_weight_gen to the next value if it exists, to WNoMethod if not. Return true if next value existed, false otherwise.
- `OrderNextPrecGen`: Set ordering->to_prec_gen to the next value if it exists, to PNoMethod if not. Return true if next value existed, false otherwise.
- `OrderNextConstWeight`: Set ordering->to_const_weight to the next value <= MAX_CONST_WEIGHT or to WConstNoSpecialWeight if already at MAX_CONST_WEIGHT. Return true in this case. Otherwise, set to_const_weight to WConstNoWeight and return false.
- `OrderNextOrdering`: Set ordering to the next possible ordering by alternating those of the 4 parameters that are indeterminate in mask (NoOrdering, PNoMethod, WNoMethod, WConstNoWeight). Return true if successful, false otherwise (in which case ordering will have cycled to the first possible combination, but don't count on this, it is an artifact of this particular implementat...
- `OrderFindOptimal`: Iterate through all orderings matching mask (see previous function) and find the optimal one. Return a corresponding OCB.
- `TOSelectOrdering`: Given a proof state, select a (hopefully suitable) ordering for it and return the corresponding OCB.
- `TOCreateOrdering`: Given a proof state and a fully specified OrderParamCell, create the ordering.

### Dependencies

- `"che_new_autoschedule.h"`
- `"che_to_autoselect.h"`
- `<che_proofcontrol.h>`

### Compile-Time Conditions

- `CHE_TO_AUTOSELECT`
- `COMPILE_HEURISTICS_OPTIMIZED`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_to_autoselect.h`, `HEURISTICS/che_to_autoselect.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 1050 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 13 structured function-comment blocks.
- Functions dealing with the automatic selection of a (suitable?) term ordering. the GNU Lesser General Public License. <1> Thu Dec 31 17:39:46 MET 1998
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change-Later Observations

- `TOCreateOrdering` selects matrix-backed precedence solely from `pre_precedence != NULL`. With `PNoMethod`, a predefined precedence can stay partial, and KBO weight generation can still query it through `OCBFunCompare`; revisit this only after reference tests cover first-maximal and rank-style weight methods under partial user precedence.
- `TOCreateOrdering` assigns `params->lit_cmp` directly into `ocb->lit_cmp` as a raw enum value. A cleaned Rust boundary should validate it, but compatibility may require preserving arbitrary raw values if malformed strategy files are observable.
<!-- END MANUAL REVIEW: c_source_docs -->
