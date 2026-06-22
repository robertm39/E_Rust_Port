<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_clauseweight

## Source Files

- [HEURISTICS/che_clauseweight.h](../../../eprover/HEURISTICS/che_clauseweight.h)
- [HEURISTICS/che_clauseweight.c](../../../eprover/HEURISTICS/che_clauseweight.c)

## Purpose

Evaluation of a clause by clause weight, also an example for setting up an evaluation function. Contains some additional evaluation functions as well. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `WeightParamCell`
- `WeightParam_p`

### Macros And Constants

- `CHE_CLAUSEWEIGHT`
- `DEFAULT_POS_MULT`
- `WeightParamCellAlloc()`
- `WeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(WeightParamCell)) SizeFree(junk, sizeof(WeightParamCell)) WFCB_p ClauseWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, double pos_multiplier, double app_var_mult)`
- `WFCB_p CMaxWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, double pos_multiplier, double app_var_mult)`
- `WFCB_p CMaxWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p ClauseWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p DefaultWeightInit(ClausePrioFun prio_fun)`
- `WFCB_p DefaultWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p LMaxWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, double pos_multiplier, double app_var_mult)`
- `WFCB_p LMaxWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p UniqWeightInit(ClausePrioFun prio_fun)`
- `WFCB_p UniqWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double CMaxWeightCompute(void* data, Clause_p clause)`
- `double ClauseWeightCompute(void* data, Clause_p clause)`
- `double DefaultWeightCompute(void* data, Clause_p clause)`
- `double LMaxWeightCompute(void* data, Clause_p clause)`
- `double UniqWeightCompute(void* data, Clause_p clause)`
- `void ClauseWeightExit(void* data)`
- `void TrivialWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `uniq_term_weight`: Return the uniqweight of a term.
- `uniq_eqn_weight`: Return the uniqweight of a equation.
- `ClauseWeightInit`: Return an initialized WFCB for ClauseWeight evaluation.
- `ClauseWeightParse`: Parse a clauseweight-definition.
- `ClauseWeightCompute`: Compute an evaluation for a clause.
- `ClauseWeightExit`: Free the data entry in a clauseweight WFCB.
- `LMaxWeightInit`: Return an initialized WFCB for LMaxWeight evaluation.
- `LMaxWeightParse`: Parse a LMaxweight-definition.
- `LMaxWeightCompute`: Compute an LMax evaluation for a clause. Each literal is weigthed with the weight of its heaviest term.
- `CMaxWeightInit`: Return an initialized WFCB for CMaxWeight evaluation.
- `CMaxWeightParse`: Parse a CMaxweight-definition.
- `CMaxWeightCompute`: Compute an evaluation for a clause, multiplying the weight of the largest term by the number of literals.
- `UniqWeightInit`: Return an initialized WFCB for UniqWeight evaluation. UniqWeight is designed to return a "maximally unique" weight that is invariant with respect to function symbol renaming, reordering and so on.
- `UniqWeightParse`: Parse a uniqweight-definition.
- `UniqWeightCompute`: Compute a hopefully uniq weight for each clause (see above)
- `DefaultWeightInit`: Return an initialized WFCB for DefaultWeight evaluation. This uses the precomputed default clause weight for evaluation.
- `DefaultWeightParse`: Parse a default weight-definition.
- `DefaultWeightCompute`: Compute return the default weight.
- `TrivialWeightExit`: Do nothing with the correct argument (for evaluation functions that do not need to store any data).

### Dependencies

- `"che_clauseweight.h"`
- `<che_wfcb.h>`

### Compile-Time Conditions

- `CHE_CLAUSEWEIGHT`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_clauseweight.h`, `HEURISTICS/che_clauseweight.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 642 lines, 19 scanned public declarations, 0 scanned internal function definitions, and 19 structured function-comment blocks.
- Evaluation of a clause by clause weight, also an example for setting up an evaluation function. Contains some additional evaluation functions as well. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
