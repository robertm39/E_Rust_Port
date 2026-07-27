<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_simplification

## Source Files

- [CONTROL/cco_simplification.h](../../../eprover/CONTROL/cco_simplification.h)
- [CONTROL/cco_simplification.c](../../../eprover/CONTROL/cco_simplification.c)

## Purpose

Global control function used with simplification. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_SIMPLIFICATION`

### Globals

- None found in the source scan.

### Exported Functions

- `bool RemoveRewritableClauses(OCB_p ocb, ClauseSet_p from, ClauseSet_p into, ClauseSet_p archive, Clause_p new_demod, SysDate nf_date, GlobalIndices_p gindices, bool lambda_demod)`
- `bool RemoveRewritableClausesIndexed(OCB_p ocb, ClauseSet_p into, ClauseSet_p archive, Clause_p new_demod, SysDate nf_date, GlobalIndices_p gindices, bool lambda_demod)`
- `long ClauseSetUnitSimplify(ClauseSet_p set, Clause_p simplifier, ClauseSet_p tmp_set, ClauseSet_p archive, GlobalIndices_p gindices, bool lambda_demod)`
- `long RemoveContextualSRClauses(ClauseSet_p from, ClauseSet_p into, ClauseSet_p archive, Clause_p simplifier, GlobalIndices_p gindices, bool lambda_demod)`
- `void ClauseMoveSimplified(GlobalIndices_p gindices, Clause_p clause, ClauseSet_p tmp_set, ClauseSet_p archive, bool lambda_demod)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ClauseMoveSimplified`: Remove a processed simplifiable clause from its set, move it to the archive set, and move a fresh copy pointing to the original as its source into tmp_set.
- `RemoveRewritableClauses`: Remove all clauses which can be rewritten with new_demod.
- `RemoveRewritableClausesIndexed`: Remove all clauses in gindices->bw_rw_index which can be rewritten with new_demod.
- `ClauseSetUnitSimplify`: Try to simplify all clauses in set by performing matching unit resolution with simplifier. Move affected clauses from set into tmp_set. Return number of clauses moved.
- `RemoveContextualSRClauses`: Move clauses that simplifier can contextually simplify-reflect from from into into. Return number of clauses moved.

### Dependencies

- `"cco_simplification.h"`
- `<ccl_context_sr.h>`
- `<ccl_global_indices.h>`
- `<ccl_rewrite.h>`
- `<che_proofcontrol.h>`

### Compile-Time Conditions

- `CCO_SIMPLIFICATION`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_simplification.h`, `CONTROL/cco_simplification.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 308 lines, 5 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- Global control function used with simplification. the GNU Lesser General Public License.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `ClauseMoveSimplified` deletes the live processed clause from global indices before extracting it from its set, archives the original, flat-copies a fresh clause into the temporary set, and marks the archived original dead.
- `RemoveRewritableClauses` moves every clause collected by the plain backward-rewrite scan. The indexed variant treats `gindices->bw_rw_index` as authoritative across all processed sets, first clears `CPRWDetected` on each collected clause, then moves it through the same simplified-clause path.
- `ClauseSetUnitSimplify` scans the current set in successor order, advances the cursor before moving a matched clause, and returns only the number of moved clauses.
- `RemoveContextualSRClauses` can receive duplicate hits from `ClauseSetFindContextSRClauses`, so it rechecks that each candidate still belongs to the source set before moving it.

### Rust Port Status Notes

- `src/heuristics/proofcontrol.rs` ports the `cco_simplification` behavior through the public `proof_state_backward_simplify*` family: backward rewriting via four plain processed-set scans when no backward index exists or one authoritative explicitly supplied indexed lookup across all processed sets when it does, backward subsumption, unit back-simplification, contextual simplify-reflect, C-ordered global-index deletion before movement, archive/dead-original handling, tmp-store requeueing, statistics, and opt-in proof-documentation quotes. As in C, backward contextual simplify-reflect only discovers and requeues candidates; the later selected-clause forward-contraction owner performs and documents the actual `csr` mutation.
- `proof_state_simplify_watchlist*` ports the watchlist-oriented rewrite-and-requeue path over the same archive/dead-original shape, including explicitly supplied watchlist indices and represented proof-documentation output.

### Change Later

- The C `ClauseMoveSimplified` comment says it kills children and modifies the clause counter, but the checked body only archives/requeues the clause and sets `CPIsDead`. Rust mirrors the body; revisit child-link cleanup only when full clause-child ownership is represented.
- The C helpers rely on live clause pointer identity while clauses remain inside intrusive sets. Rust currently uses owned clauses and stable identifiers around set extraction, archive, and tmp-store movement; switch to stable clause handles only if duplicate identifiers or archive/requeue aliases make pointer identity observable.
- `RemoveRewritableClausesIndexed` couples duplicate suppression (`CPRWDetected`), source-set ownership (`handle->set`), and mutation through raw live clause pointers. Rust preserves the authoritative-index and reverse-stack movement behavior with index snapshots plus stable identifiers; a later proof-session owner should replace both the C flag protocol and Rust cross-set identifier lookup with explicit stable clause handles, and can add debug consistency checks without changing release search behavior.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
