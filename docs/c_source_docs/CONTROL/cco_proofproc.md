<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_proofproc

## Source Files

- [CONTROL/cco_proofproc.h](../../../eprover/CONTROL/cco_proofproc.h)
- [CONTROL/cco_proofproc.c](../../../eprover/CONTROL/cco_proofproc.c)

## Purpose

Top level proof procedure the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_PROOFPROC`
- `TMPBANK_GC_LIMIT`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p ProcessClause(ProofState_p state, ProofControl_p control, long answer_limit)`
- `Clause_p SATCheck(ProofState_p state, ProofControl_p control)`
- `Clause_p Saturate(ProofState_p state, ProofControl_p control, long step_limit, long proc_limit, long unproc_limit, long total_limit, long generated_limit, long tb_insert_limit, long answer_limit)`
- `PERF_CTR_DECL(BWRWTimer)`
- `PERF_CTR_DECL(ParamodTimer)`
- `void ProofControlInit(ProofState_p state, ProofControl_p control, HeuristicParms_p params, FVIndexParms_p fvi_params, PStack_p wfcb_defs, PStack_p hcb_defs)`
- `void ProofStateInit(ProofState_p state, ProofControl_p control)`
- `void ProofStateMoveToTmpStore(ProofState_p state, ProofControl_p control)`
- `void ProofStateResetProcessed(ProofState_p state, ProofControl_p control)`
- `void ProofStateResetProcessedSet(ProofState_p state, ProofControl_p control, ClauseSet_p set)`

## Implementation Notes

### Internal Functions

- `check_ac_status`
- `cleanup_unprocessed_clauses`
- `eliminate_backward_rewritten_clauses`
- `eliminate_backward_subsumed_clauses`
- `eliminate_context_sr_clauses`
- `eliminate_unit_simplified_clauses`
- `generate_new_clauses`
- `insert_new_clauses`
- `print_sharing_factor`
- `remove_subsumed`

### Source-Level Behavior

- `document_processing`: Document processing of the new given clause (depending on the output level).
- `check_ac_status`: Check if the AC theory has been extended by the currently processed clause, and act accordingly.
- `remove_subsumed`: Remove all clauses subsumed by subsumer from set, kill their children. Return number of removed clauses.
- `eliminate_backward_rewritten_clauses`: Remove all processed clauses rewritable with clause and put them into state->tmp_store.
- `eliminate_backward_subsumed_clauses`: Eliminate subsumed processed clauses, return number of clauses deleted.
- `eliminate_unit_simplified_clauses`: Perform unit-back-simplification on the proof state.
- `eliminate_context_sr_clauses`: If required by control, remove all backward-contextual-simplify-reflectable clauses.
- `check_watchlist`: Check if a clause subsumes one or more watchlist clauses, if yes, set appropriate property in clause and remove subsumed clauses.
- `simplify_watchlist`: Simplify all clauses in state->watchlist with processed positive units from state. Assumes that all those clauses are in normal form with respect to all clauses but clause!
- `generate_new_clauses`: Apply the generating inferences to the proof state, putting new clauses into state->tmp_store.
- `eval_clause_set`: Add evaluations to all clauses in state->eval_set. Factored out so that batch-processing with e.g. neural networks can be easily integrated.
- `insert_new_clauses`: Rewrite clauses in state->tmp_store, remove superfluous literals, insert them into state->unprocessed. If an empty clause is detected, return it, otherwise return NULL.
- `replacing_inferences`: Perform the inferences that replace a clause by another: Destructive equality-resolution and/or splitting. Returns NULL if clause was replaced, the empty clause if this produced an empty clause, and the original clause otherwise
- `cleanup_unprocessed_clauses`: Perform maintenenance operations on state->unprocessed, depending on parameters in control: - Remove orphaned clauses - Simplify all unprocessed clauses - Reweigh all unprocessed clauses - Delete "bad" clauses to avoid running out of memories. Simplification can find the empty clause, which is then returned.
- `SATCheck`: Create ground (or pseudo-ground) instances of the clause set, hand them to a SAT solver, and check then for unsatisfiability.
- `print_sharing_factor`: Determine the sharing factor and print it. Potentially expensive, only useful for manual analysis.
- `print_rw_state`: Print the system (R,E,NEW), e.g. the two types of demodulators and the newly generated clauses.
- `ProofControlInit`: Initialize a proof control cell for a given proof state (with at least axioms and signature) and a set of parameters describing the ordering and heuristics.
- `ProofStateResetProcessedSet`: Move all clauses from set into state->unprocessed.
- `ProofStateMoveSetToTmp`: Lightweight version of ProofStateResetProcessedSet which simply moves all clauses from set to tmp_store without reevaluating clause evaluation features.
- `ProofStateResetProcessed`: Move all clauses from the processed clause sets to unprocessed.
- `ProofStateMoveToTmpStore`: Move all clauses from the processed clause sets to tmp store.
- `fvi_param_init`: Initialize the parameters for all feature vector indices in state.
- `ProofStateInit`: Given a proof state with axioms and a heuristic parameter description, initialize the ProofStateCell, i.e. generate the HCB, the ordering, and evaluate the axioms and put them in the unprocessed list.
- `ProcessClause`: Select an unprocessed clause, process it. Return pointer to empty clause if it can be derived, NULL otherwise. This is the core of the main proof procedure.
- `Saturate`: Process clauses until either the empty clause has been derived, a specified number of clauses has been processed, or the clause set is saturated. Return empty clause (if found) or NULL.

### Dependencies

- `"cco_proofproc.h"`
- `<ccl_fcvindexing.h>`
- `<ccl_satinterface.h>`
- `<cco_clausesplitting.h>`
- `<cco_diseq_decomp.h>`
- `<cco_forward_contraction.h>`
- `<cco_ho_inferences.h>`
- `<cco_interpreted.h>`
- `<che_axiomscan.h>`
- `<che_heuristics.h>`
- `<che_to_autoselect.h>`
- `<cio_signals.h>`
- `<clb_os_wrapper.h>`
- `<cte_ho_csu.h>`
- `<picosat.h>`

### Compile-Time Conditions

- `CCO_PROOFPROC`
- `PRINT_RW_STATE`
- `PRINT_SHARING`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_proofproc.h`, `CONTROL/cco_proofproc.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 1894 lines, 12 scanned public declarations, 10 scanned internal function definitions, and 26 structured function-comment blocks.
- Main proof-process orchestration. Saturation loop phases, generated/processed limits, and termination reasons are user-visible.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `ProofControlInit` assumes a freshly allocated proof-control cell with no OCB or selected HCB, selects the ordering, installs built-in and user weight-function/heuristic definitions, then copies the finalized `HeuristicParmsCell` and `FVIndexParmsCell` into the control. Rust now has a proof-state-aware helper for the OCB selection, definition-installation, active-HCB lookup, `heuristic_def` stack mutation, split-dependent FV-index slack normalization, higher-order CSU unification-limit snapshot portion, and reusable `fvi_param_init` FV-index anchor construction. Final attachment to full proof-state clause-set owners remains part of the later proof-state initialization slice.
- After copying `FVIndexParmsCell`, `ProofControlInit` forces `fvi_parms.symbol_slack=0` when splitting is disabled. Keep that normalization at proof-control initialization time rather than in raw option parsing because it depends on the finalized heuristic split setting.
- `fvi_param_init` derives the actual collection spec from the proof state's original symbol count, `symbol_slack`, and `max_symbols`, computes one permutation vector from the active spec, copies that vector into the processed/watchlist anchors, and passes the same effective vector to the definition-store anchor even though the definition store uses a separate AC-fold collection spec. The raw `FVIndexParmsCell` is not the final index spec by itself. Rust now computes the active and definition-store `FVCollect` specs and the empty anchor bundle; moving those anchors into processed/watchlist/definition-store clause sets remains with the future proof-state owner.

### Change-Later Observations

- `ProofControlInit` mutates both the caller's heuristic-definition stack and the caller's heuristic parameter object while reconciling direct `heuristic_def` text with stacked `--define-heuristic` options. Rust preserves this in the current compatibility helper, but a later higher-level API could return an initialized control object and normalized parameter snapshot instead of exposing caller-side mutation.
- `ProofControlInit` calls `InitUnifLimits` after writing initialized params back to the caller. In C this stores a process-global pointer to `control->heuristic_parms` for higher-order CSU enumeration rather than deriving new values. Rust stores a safe snapshot of the fields read by the CSU and binding-generation helpers; revisit exact pointer aliasing only if a later port mutates `control->heuristic_parms` after initialization and expects CSU reads to observe those writes.
- In the `FVICollectFeatures` branch, `fvi_param_init` copies the overflow layout fields and assembly-vector length but allocates a fresh assembly vector rather than copying the original vector contents. Rust preserves that shape; revisit only if strategy files or generated specs make non-default assembly vectors observable at this boundary.
- `fvi_param_init` overwrites `cspec->max_symbols` for the active proof-state spec but not for `def_store_cspec`, which keeps the `FVCollectAlloc` default. Rust preserves this difference in the reusable spec helper.
- `fvi_param_init` computes and installs a permutation vector without consulting `control->fvi_parms.use_perm_vectors`; the CLI still records `Direct` versus `Perm` in that field. Rust preserves the initialization path and records the flag for compatibility, but this unused option bit should be revisited once indexed subsumption behavior is covered by end-to-end reference tests.
- The definition-store FV index uses the permutation vector computed from the active proof-state spec rather than from `def_store_cspec`. Rust preserves the effective packing input, but a future cleanup could make this dependency explicit or compute a definition-store-specific vector after compatibility is secured.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
