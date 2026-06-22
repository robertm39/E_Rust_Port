<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_random

## Source Files

- [HEURISTICS/che_random.h](../../../eprover/HEURISTICS/che_random.h)
- [HEURISTICS/che_random.c](../../../eprover/HEURISTICS/che_random.c)

## Purpose

Clause "evaluations" incorporating random elements. Note that these are not, in general, fair if used with naive parameterization. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RandomWeightParamCell`
- `RandomWeightParam_p`

### Macros And Constants

- `CHE_RANDOM`
- `RandomWeightParamCellAlloc()`
- `RandomWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(RandomWeightParamCell)) SizeFree(junk, sizeof(RandomWeightParamCell)) WFCB_p RandWeightInit(ClausePrioFun prio_fun, long range, double fifo_w, double sc_w, unsigned int seed1, unsigned int seed2, unsigned int seed3)`
- `WFCB_p RandWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double RandWeightCompute(void* data, Clause_p clause)`
- `void RandWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `RandWeightInit`: Return an initialized WFCB for Random evaluation.
- `RandWeightParse`: Parse a Random declaration of the form (priofun, range, fifo_w, sc_w [, seed1 [, seed2 [,seed3]]])
- `RandWeightCompute`: Compute an evaluation for a clause. aGlobal Variables: -
- `RandWeightExit`: Free the data entry in a Random WFCB.

### Dependencies

- `"che_random.h"`
- `<che_wfcb.h>`

### Compile-Time Conditions

- `CHE_RANDOM`

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

Source files reviewed: `HEURISTICS/che_random.h`, `HEURISTICS/che_random.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 265 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Clause "evaluations" incorporating random elements. Note that these are not, in general, fair if used with naive parameterization. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
