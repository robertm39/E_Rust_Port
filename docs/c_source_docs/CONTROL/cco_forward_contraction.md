<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_forward_contraction

## Source Files

- [CONTROL/cco_forward_contraction.h](../../../eprover/CONTROL/cco_forward_contraction.h)
- [CONTROL/cco_forward_contraction.c](../../../eprover/CONTROL/cco_forward_contraction.c)

## Purpose

Functions that apply the processed clause sets to simplify or eliminate a potential new clause. Extracted from cco_proofproc.[ch]. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_FORWARD_CONTRACTION`
- `DEFAULT_FILTER_DESCRIPTOR`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p ForwardContractSet(ProofState_p state, ProofControl_p control, ClauseSet_p set, bool non_unit_subsumption, RewriteLevel level, unsigned long* count_eliminated, bool terminate_on_empty)`
- `Clause_p ForwardContractSetReweight(ProofState_p state, ProofControl_p control, ClauseSet_p set, bool non_unit_subsumption, RewriteLevel level, unsigned long* count_eliminated)`
- `Clause_p ProofStateFilterUnprocessed(ProofState_p state, ProofControl_p control, char* desc)`
- `FVPackedClause_p ForwardContractClause(ProofState_p state, ProofControl_p control, Clause_p clause, bool non_unit_subsumption, bool context_sr, bool condense, RewriteLevel level)`
- `FVPackedClause_p ForwardSubsumption(ProofState_p state, Clause_p clause, unsigned long* subsumed_count, bool non_unit_subsumption)`
- `bool ForwardModifyClause(ProofState_p state, ProofControl_p control, Clause_p clause, bool context_sr, bool condense, RewriteLevel level)`
- `void ClauseSetFilterReweigth(ProofControl_p control, ClauseSet_p set, unsigned long* count_eliminated)`
- `void ClauseSetReweight(HCB_p heuristic, ClauseSet_p set)`

## Implementation Notes

### Internal Functions

- `forward_contract_keep`

### Source-Level Behavior

- `forward_contract_keep`: Apply all forward-contracting inferences to clause. Return NULL if it becomes trivialredundant, a FVPackedClause containing it otherwise. Does not delete clause. Subsumed and trivial clauses are counted in the cells pointed to by the 4th and 5th argument. Provide dummies to avoid this.
- `ForwadSubsumption`: Try to subsume clause with clauses in state->processed*. Return NULL if this succeeds, a FVPackedClause containing clause otherwise. Note that clause is _not_ deleted in either case!
- `ForwardModifyClause`: Apply all modifying forward-inferences to clause (unless it becomes trivial). Return true if it does become trivial.
- `ForwardContractClause`: Apply all forward-contracting inferences to clause. Return NULL and delete the clause if it becomes trivial, return FVPackedClause otherwise.
- `ForwardContractSet`: Apply the forward-contracting inferences to all clauses in set. Delete redundant clauses. If terminate_on_empty is true, return empty clause (if found), NULL otherwise. The empty clause will be extracted from set, which may not be fully contracted in this case.
- `ClauseSetReweight`: Re-Evaluate all clauses in set.
- `ForwardContractSetReweight`: Apply contracting inferences to all claues in set, then reevaluate them. Return empty clause (if found), NULL otherwise. The empty clause will be extracted from set, which may not be fully contracted in this case.
- `ClauseSetFilterReweigth`: Remove all trivial clauses from set and reweigth it.
- `ProofStateFilterUnprocessed`: Apply various filter operations (guided by *desc) to the set of unprocessed clauses in state. Return the empty clause (and stop filtering) if it was found, otherwise return NULL.

### Dependencies

- `"cco_forward_contraction.h"`
- `"cco_ho_inferences.h"`
- `<ccl_context_sr.h>`
- `<ccl_tautologies.h>`
- `<cco_eqnresolving.h>`
- `<cco_factoring.h>`
- `<cco_paramodulation.h>`
- `<cco_simplification.h>`
- `<cio_output.h>`

### Compile-Time Conditions

- `CCO_FORWARD_CONTRACTION`

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

Source files reviewed: `CONTROL/cco_forward_contraction.h`, `CONTROL/cco_forward_contraction.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 723 lines, 8 scanned public declarations, 1 scanned internal function definitions, and 9 structured function-comment blocks.
- Functions that apply the processed clause sets to simplify or eliminate a potential new clause. Extracted from cco_proofproc.[ch]. the GNU Lesser General Public License.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
