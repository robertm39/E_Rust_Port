<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_dagweight

## Source Files

- [HEURISTICS/che_dagweight.h](../../../eprover/HEURISTICS/che_dagweight.h)
- [HEURISTICS/che_dagweight.c](../../../eprover/HEURISTICS/che_dagweight.c)

## Purpose

Evaluation of a clause by DAG weight (i.e. counting multiple occurrences of subterms only once). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DAGWeightParamCell`
- `DAGWeightParam_p`
- `RDAGWeightParamCell`
- `RDAGWeightParam_p`

### Macros And Constants

- `CHE_DAGWEIGHT`
- `DAGWeightParamCellAlloc()`
- `DAGWeightParamCellFree(junk)`
- `DEFAULT_DAG_DUP_WEIGHT`
- `RDAGWeightParamCellAlloc()`
- `RDAGWeightParamCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SizeMalloc(sizeof(DAGWeightParamCell)) SizeFree(junk, sizeof(DAGWeightParamCell)) SizeMalloc(sizeof(RDAGWeightParamCell)) SizeFree(junk, sizeof(RDAGWeightParamCell)) WFCB_p DAGWeightInit(ClausePrioFun prio_fun, int fweight, int vweight, double pos_multiplier, long dup_weight, bool pos_use_dag, bool pos_term_reset, bool pos_eqn_reset, bool neg_use_dag, bool neg_term_reset, bool neg_eqn_reset, bool pos_neg_reset)`
- `WFCB_p DAGWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p RDAGWeight2Init(ClausePrioFun prio_fun, OCB_p ocb, long fweight, long vweight, long dup_weight, double max_term_multiplier, double pos_multiplier)`
- `WFCB_p RDAGWeight2Parse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p RDAGWeight3Init(ClausePrioFun prio_fun, OCB_p ocb, long fweight, long vweight, long nfweight, long nvweight, long dup_weight, double max_term_multiplier, double pos_multiplier, double pneq_multiplier, double nneq_multiplier)`
- `WFCB_p RDAGWeight3Parse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `WFCB_p RDAGWeightInit(ClausePrioFun prio_fun, OCB_p ocb, long fweight, long vweight, long dup_weight, double uniqmax_term_multiplier, double max_term_multiplier, double max_literal_multiplier, double pos_multiplier)`
- `WFCB_p RDAGWeightParse(Scanner_p in, OCB_p ocb, ProofState_p state)`
- `double DAGWeightCompute(void* data, Clause_p clause)`
- `double RDAGWeight2Compute(void* data, Clause_p clause)`
- `double RDAGWeight3Compute(void* data, Clause_p clause)`
- `double RDAGWeightCompute(void* data, Clause_p clause)`
- `void DAGWeightExit(void* data)`
- `void RDAGWeightExit(void* data)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `DAGWeightInit`: Return an initialized WFCB for DAGWeight evaluation.
- `DAGWeightParse`: Parse a DAGWeight-definition.
- `DAGWeightCompute`: Compute a dag-evaluation for a clause.
- `DAGWeightExit`: Free the data entry in a DAGWeight WFCB.
- `RDAGWeightInit`: Return an initialized WFCB for RDAGWeightCompute().
- `RDAGWeightParse`: Parse a refined DAG-clauseweight-definition.
- `RDAGWeightCompute`: Compute a refined-dag-evaluation for a clause.
- `RDAGWeightExit`: Free the data entry in a RDAGWeight WFCB.
- `RDAGWeight2Init`: Return an initialized WFCB for RDAGWeight2Compute().
- `RDAGWeight2Parse`: Parse a refined Twee-style DAG2-clauseweight-definition.
- `RDAGWeight2Compute`: Compute a Twee-style dag-evaluation for a clause. The "larger" (by wighted symbol count) is given higher weigth. Term orderings are ignored.
- `RDAGWeight3Init`: Return an initialized WFCB for RDAGWeight3Compute().
- `RDAGWeight3Parse`: Parse a refined Twee-style DAG2-clauseweight-definition.
- `RDAGWeight3Compute`: Compute a mixed dag weight. For positive literals, both terms are independendly computed, either with normal weights, or with dag weights. For negative literals, we use one dag.

### Dependencies

- `"che_dagweight.h"`
- `<che_clauseweight.h>`

### Compile-Time Conditions

- `CHE_DAGWEIGHT`

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

Source files reviewed: `HEURISTICS/che_dagweight.h`, `HEURISTICS/che_dagweight.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 826 lines, 18 scanned public declarations, 0 scanned internal function definitions, and 14 structured function-comment blocks.
- Evaluation of a clause by DAG weight (i.e. counting multiple occurrences of subterms only once). the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- `RDAGWeightCompute` calls `ClauseCondMarkMaximalTerms(local->ocb, clause)` before clearing `TPOpFlag` across the literal list and then scoring with `EqnDAGWeight`; the Rust initializer installs a banked callback that preserves this order with the active proof-control OCB, mutable owner bank, and clause. `DAGWeightCompute`, `RDAGWeight2Compute`, and `RDAGWeight3Compute` do not conditionally mark in C and deliberately keep immutable callbacks in Rust. The four-family owner audit, proof-control regression, and exact executable comparison are recorded in [`experiments/2026-07-17-068-dagweight-owner-context/FINDINGS.md`](../../../experiments/2026-07-17-068-dagweight-owner-context/FINDINGS.md).
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- The immutable RDAG scoring callback remains a low-level/test adapter for clauses whose orientation flags are already current. Removing that public compatibility surface after an API review is optional cleanup; production HCB evaluation already uses the banked owner path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
