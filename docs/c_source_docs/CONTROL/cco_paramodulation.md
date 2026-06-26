<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_paramodulation

## Source Files

- [CONTROL/cco_paramodulation.h](../../../eprover/CONTROL/cco_paramodulation.h)
- [CONTROL/cco_paramodulation.c](../../../eprover/CONTROL/cco_paramodulation.c)

## Purpose

Functions for controling paramodulation inferences. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_PARAMODULATION`

### Globals

- None found in the source scan.

### Exported Functions

- `long ComputeAllParamodulants(TB_p bank, OCB_p ocb, Clause_p clause, Clause_p parent_alias, ClauseSet_p with_set, ClauseSet_p store, VarBank_p freshvars, ParamodulationType pm_type)`
- `long ComputeAllParamodulantsIndexed(TB_p bank, OCB_p ocb, VarBank_p freshvars, Clause_p clause, Clause_p parent_alias, OverlapIndex_p into_index, OverlapIndex_p negp_index, OverlapIndex_p from_index, ClauseSet_p store, ParamodulationType pm_type)`
- `long ComputeClauseClauseParamodulants(TB_p bank, OCB_p ocb, Clause_p clause, Clause_p parent_alias, Clause_p with, ClauseSet_p store, VarBank_p freshvars, ParamodulationType pm_type)`
- `long ComputeFromParamodulants(ParamodInfo_p pminfo, ParamodulationType type, Clause_p clause, OverlapIndex_p from_index, ClauseSet_p store)`
- `long ComputeFromSimParamodulants(ParamodInfo_p pminfo, ParamodulationType type, Clause_p clause, OverlapIndex_p from_index, ClauseSet_p store)`
- `long ComputeIntoParamodulants(ParamodInfo_p pminfo, ParamodulationType type, Clause_p clause, OverlapIndex_p into_index, OverlapIndex_p negp_index, ClauseSet_p store)`

## Implementation Notes

### Internal Functions

- `compute_from_pm_pos_clause`
- `compute_into_pm_pos_clause`
- `compute_pos_from_pm`
- `compute_pos_from_pm_termtree`
- `compute_pos_into_pm`
- `compute_pos_into_pm_termtree`
- `sim_paramod_q`
- `update_clause_info`
- `variable_paramod`

### Source-Level Behavior

- `sim_paramod_q`: Given frompos (instantiated) and pm_type, determine wether to use normal, simultaneous or super-simultaneois paramodulation.
- `variable_paramod`: Perform paramodulation or simulated paramodulation as requested. Return result (if any)
- `update_clause_info`: Given a (newly generated) paramodulant and the two "real" parents, update meta-information.
- `compute_into_pm_pos_clause`: Compute all paramodulations from pminfo->from* into the clause and positions described by into_clause_pos. Return number of such clauses.
- `compute_into_pm_pos_term`: Compute all paramodulations from clause with clause|pos = term, term is the LHS for the overlap, into clauses in into_clauses.
- `compute_pos_into_termtree`: Compute all paramodulations from clause with clause|pos = term, term is the LHS for the overlap, into clauses in into_tree.
- `compute_pos_into_pm`: Compute all paramodulations from clause with clause|pos = term, term is the LHS for the overlap, into clauses in into_index.
- `compute_from_pm_pos_clause`: Compute all paramodulations into pminfo->into* from the clause and positions described by from_clause_pos. Return number of such clauses.
- `compute_from_pm_pos_term`: Compute all paramodulations into clause with clause|pos = term, term is the LHS for the overlap, from clauses in from_clauses.
- `compute_pos_from_termtree`: Compute all paramodulations into clause with pminfo->into|pos = term, term is the LHS for the overlap, from clauses in from_tree.
- `compute_pos_from_pm`: Compute all paramodulations into clause with pminfo->into|pos = term, term is the LHS for the overlap, from clauses in from_index.
- `ComputeClauseClauseParamodulants`: Compute all (simultaneous) paramodulants between clause and with, with terms from bank, and put them into store. Returns number of paramodulants.
- `ComputeAllParamodulants`: Compute all paramodulants between clause and with_set, put them into store.
- `ComputeIntoParamodulants`: Compute all paramodulants from clause into clauses in into_index.
- `ComputeFromParamodulants`: Compute all paramodulants from clauses in from_index into clause.
- `ComputeFromSimParamodulants`: Compute all simultaneouss paramodulants from clauses in from_index into clause.
- `ComputeAllParamodulantsIndexed`: Compute all paramodulants (of the right pm_type) between clause and clauses in the indices. Put them into store. Return number of clauses generated.

### Dependencies

- `"cco_paramodulation.h"`
- `<ccl_paramod.h>`
- `<che_proofcontrol.h>`
- `<cte_ho_csu.h>`
- `<cte_idx_fp.h>`

### Compile-Time Conditions

- `CCO_PARAMODULATION`
- `NEVER_DEFINED`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for the first Rust indexed simultaneous/super-simultaneous wrapper slice on 2026-06-26.

Source files reviewed: `CONTROL/cco_paramodulation.h`, `CONTROL/cco_paramodulation.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 1110 lines, 6 scanned public declarations, 9 scanned internal function definitions, and 17 structured function-comment blocks.
- Functions for controling paramodulation inferences. the GNU Lesser General Public License.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `ComputeClauseClauseParamodulants` first paramodulates from `clause` into `with` with top target positions allowed, then, when the parents are distinct, paramodulates from `with` into `clause` with positive top target positions suppressed. This direction/no-top order is part of the generated-clause stream.
- `parent_alias` is metadata, not necessarily the same object used for source positions. The Rust wrapper preserves this split so temporary source views can still document the original parent.
- `update_clause_info` combines proof size, proof depth, TPTP type, and SOS flags from the two real parents before insertion into the caller store.
- Rust now ports the plain, simultaneous, and super-simultaneous first-order unindexed wrapper paths, the reusable indexed wrapper path with plain/simultaneous/super-simultaneous mode dispatch, and an explicit caller-owned global-index selected-clause generation helper with `DCParamod`/`DCSimParamod` derivation entries on generated child clauses as appropriate. State-owned process-clause indexed wiring, higher-order substitutions, and proof-documentation output remain pending.

### Change-Later Observations

- In the unindexed C wrapper, the two `ClausePushDerivation` calls pass `clause` rather than the freshly created `paramod` child. That looks inconsistent with the surrounding metadata update and other inference wrappers. Rust records the derivation on the generated child; keep this as a C/Rust reference-test target before changing C or compatibility expectations.
- `variable_paramod` selects plain, simultaneous, or super-simultaneous construction through `sim_paramod_q`, while the indexed path may decide a different simultaneous mode for each source position. Rust now supports simultaneous and super-simultaneous construction in the unindexed and indexed wrappers, including oriented-source, order-decreasing, and size-decreasing mode selection; process-clause proof-control still needs state-owned global-index integration before it can enable indexed generation automatically.
- The unindexed simultaneous C path relies on `TPPotentialParamod` mutable term flags to mark candidate targets and suppress duplicate all-occurrence rewrites. Rust mirrors the flag semantics for parity; a later design could carry this marking in per-inference side state instead of shared term cells.
- The C wrapper takes a reusable `VarBank_p freshvars` from outside this unit and resets/uses it through lower constructors. Rust low-level constructors seed a local fresh-variable bank; revisit reusable-bank performance once wrapper ownership and selected-clause integration are stable.
- `ComputeAllParamodulantsIndexed` omits the unindexed wrapper's explicit `CPNoGeneration` parent/candidate gate. Rust mirrors that shape for compatibility; verify whether globally indexed no-generation clauses can occur before deciding whether to clean this up after parity is secured.
- Rust can exercise indexed paramodulation with overlap indexes tied to a cloned signature, including through an explicit proof-control generation helper, but process-clause proof-control needs a proof-session owner that can hold global indexes tied to the term-bank signature while still mutating the term bank during generation.
- The indexed C path calls `sim_paramod_q` with source-position bindings from the current CSU active, whereas the unindexed path chooses before the constructor performs its MGU. Rust mirrors the indexed order-decreasing decision with a trial first-order MGU; benchmark whether this extra pass matters once indexed proof-control generation is wired.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
