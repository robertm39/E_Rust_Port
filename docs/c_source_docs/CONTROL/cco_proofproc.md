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

- `ProofControlInit` assumes a freshly allocated proof-control cell with no OCB or selected HCB, selects the ordering, installs built-in and user weight-function/heuristic definitions, then copies the finalized `HeuristicParmsCell` and `FVIndexParmsCell` into the control. Rust now has a proof-state-aware helper for the OCB selection, definition-installation, active-HCB lookup, `heuristic_def` stack mutation, split-dependent FV-index slack normalization, higher-order CSU unification-limit snapshot portion, reusable `fvi_param_init` FV-index anchor construction, clause-set anchor installation helper, and the currently ported `ProofStateInit` path through indexing/watchlist setup, `Uniq` axiom reweighting, initial-clause watchlist-hit checks, axiom copying/evaluation into `unprocessed`, `DCCnfQuote` derivation metadata on initial copies, optional `DCCnfEvalGC` metadata when GC selection is recorded, initial-clause priority adjustment, SOS marking, AC-axiom scan/activation, and a caller-owned global-index free/init tail. Remaining `ProofStateInit` side effects are proof-documentation output; long-lived state-owned `gindices`/`wlindices` integration remains pending.
- After copying `FVIndexParmsCell`, `ProofControlInit` forces `fvi_parms.symbol_slack=0` when splitting is disabled. Keep that normalization at proof-control initialization time rather than in raw option parsing because it depends on the finalized heuristic split setting.
- `fvi_param_init` derives the actual collection spec from the proof state's original symbol count, `symbol_slack`, and `max_symbols`, computes one permutation vector from the active spec, copies that vector into the processed/watchlist anchors, and passes the same effective vector to the definition-store anchor even though the definition store uses a separate AC-fold collection spec. The raw `FVIndexParmsCell` is not the final index spec by itself. Rust now computes the active and definition-store `FVCollect` specs, builds the empty anchor bundle, installs it into `ClauseSet` owners, and calls it from the ported `ProofStateInit` indexing helper when the state is not already initialized.
- `ProofStateInit` orders source axioms by reweighting `state->axioms` with the built-in `Uniq` HCB, then copies clauses in that eval-tree order, checks the initialized watchlist before active-HCB evaluation, evaluates each copy with the active HCB, records a `DCCnfQuote` derivation to the source axiom, optionally shifts every evaluation priority by `-PrioLargestReasonable`, inserts the copy into `unprocessed`, marks SOS, and then scans the initialized `unprocessed` set for AC axioms when AC handling is enabled. It finishes by freeing and reinitializing `state->gindices` from the finalized index parameters and the process-global problem type. Rust preserves the dual-HCB ordering/evaluation split, keeps source axioms in place with their `Uniq` evaluations, ports the initial-copy watchlist mark/removal/archive side effects, and exposes the global-index tail over a caller-owned `GlobalIndices` plus explicit `ProblemType`.
- `check_ac_status` is the selected-clause dynamic AC activation hook. When AC handling is enabled, it scans the processed clause and flips `control->ac_handling_active` if a commutativity axiom is newly detected. Rust now ports the local signature mutation and activation flag update; status printing remains part of later proof-output integration.
- `check_watchlist` uses `CPSubsumesWatch` for both static and dynamic watchlists. Static watchlists only set the property when the new clause subsumes a watched clause. Dynamic watchlists remove every subsumed watched clause, mark each removed clause `CPIsDead`, and move it to `archive`. Rust mirrors those local set/archive mutations for initial clauses, including removal from the watchlist's owned FV index.
- `ForwardModifyClause` is the modifying forward-contraction prefix used by `insert_new_clauses` and `ForwardContractClause`: it rewrites with `state->demods`, removes duplicate/resolved literals, optionally removes AC-resolved literals and runs local rewriting, orients literals, optionally condenses, checks triviality, and then runs positive/negative simplify-reflect until `CPLimitedRW` stops changing. Rust now ports this first-order/local mutation loop over the processed positive-rule/equation demodulator sets and processed unit simplify-reflect sets.
- The clause-level `ForwardContractClause` wrapper adds the surrounding first-order/local checks: `ForwardSubsumption`, AC-redundancy handling, ground-completion tautology filtering, optional contextual simplify-reflect, final literal selection, maximal-term marking, and the processed subsumed/trivial statistics. Rust now ports that staged wrapper, and the set-level `ForwardContractSet`/`ForwardContractSetReweight`/`ProofStateFilterUnprocessed` path now preserves ordered deletion, early empty-clause extraction, tail restoration, and HCB reweighting. The generated-clause route uses the separate C `insert_new_clauses` sequence instead of this wrapper.
- `cleanup_unprocessed_clauses` runs three maintenance gates in order: delete orphaned unprocessed clauses after enough backward simplification, special forward-contract/reweight unprocessed clauses after enough processed clauses, and delete bad clauses when `ProofStateStorage` crosses `delete_bad_limit`. Rust now ports that gate order, statistic updates, early empty-clause return, HCB delete-bad behavior, incomplete-state marking, and term-GC call, while taking an explicit orphan predicate because compact derivation parent references are not exact C clause pointers yet.
- `ProofStateResetProcessedSet` drains a processed set, archives each original clause, flat-copies it back through the proof-state term bank, records a `DCCnfQuote` derivation from the requeued copy to the archived original, evaluates the copy with the active HCB, clears `CPIsOriented`, optionally applies the same `prefer_initial_clauses` priority offset as initialization, and inserts the copy into `unprocessed`. Rust now ports the local archive/copy/quote/evaluate/requeue behavior for all four processed sets through `ProofStateResetProcessed`.
- `ProofStateMoveSetToTmp` is intentionally lighter than reset: it drains a processed set into `tmp_store`, clears only `CPIsOriented`, and does not copy, archive, or reevaluate clauses. Rust now ports this behavior for all four processed sets through `ProofStateMoveToTmpStore`, including preservation of any existing evaluation cells on moved clauses.
- The generated-clause route inside `insert_new_clauses` snapshots tmp-store generated counters, runs the C-specific modifying-contraction path, checks the watchlist, returns an empty clause before eval-store evaluation/drain, optionally applies aggressive forward subsumption with its separate statistic, and then reaches `eval_store` only after the remaining replacement/splitting filters. Rust now ports the generated counters, `CPIsIRVictim` full-rewrite prefix, normal `ForwardModifyClause` call, trivial deletion, watchlist check, early empty return, aggressive forward-subsumption gate, first-order destructive equality-resolution requeueing/statistics, fresh-definition controlled-splitting requeueing/statistics, local stats/property/selection/date/eval-store queueing behavior, optional `DCCnfEvalGC` metadata when GC selection is recorded, eval-store HCB evaluation, and final move to `unprocessed`. Controlled splitting with definition reuse is still pending and is reported as an explicit diagnostic when enabled for pending generated clauses.
- The selected-clause `replacing_inferences` helper runs before backward simplification and uses the non-aggressive replacement gates: higher-order immediate clausification, destructive equality resolution controlled only by `er_varlit_destructive`, and controlled splitting keyed by `split_clauses`. Rust now ports the first-order destructive equality-resolution and fresh-definition controlled-splitting branches, returns an explicit survivor/replaced outcome, and routes replacement products through `insert_new_clauses` just like C. Higher-order immediate clausification and split definition reuse remain explicit diagnostics.
- The selected-clause processed insertion tail normalizes variables, stamps the selected clause's date, sets `CPLimitedRW`, marks oriented positive-rule left sides as rewritable, and inserts the clause into the positive-rule, positive-equation, negative-unit, or non-unit processed set through that set's FV index. Rust now ports these local effects as an owned-clause helper; global-index insertion, inference generation, and temporary-term-bank GC remain outside this slice.
- `simplify_watchlist` is a post-processed-insertion maintenance hook for watched clauses rewritable by the new processed demodulator. C removes rewritable watched originals through `wlindices` or a plain watchlist scan, archives them as dead, normalizes quoted copies with the processed demodulators, removes superfluous/AC-resolved literals, marks maximal terms, reinserts the copies into `state->watchlist`, and updates `wlindices`. Rust now ports the local owned-watchlist scan/archive/normalize/minimize/AC-clean/maximal-mark/FV-index reinsertion behavior; long-lived `wlindices` and proof-output side effects remain pending.
- `generate_new_clauses` runs selected-clause generators in C order: higher-order inferences, optional equality factoring, equality resolution, disequality decomposition, then paramodulation unless the selected clause is a negative unit and negative-unit paramodulation is disabled. Rust now ports the first-order equality factoring/equality resolution/disequality-decomposition calls plus plain and simultaneous/super-simultaneous unindexed paramodulation into `tmp_store` with matching statistics, and an explicit caller-owned global-index generation helper can drive the lower-level plain/simultaneous/super-simultaneous indexed wrapper. Higher-order and process-clause state-owned indexed paramodulation generation remain explicit diagnostics.
- `ProcessClause` selects the next HCB-best unprocessed clause, strips evaluations, marks it processed, optionally archive-copies it for proof-object GC selection, forward-contracts it, handles semantic-answer returns, checks dynamic AC status, documents/nontrivially counts it, runs replacement inferences, checks the watchlist, performs backward simplification, inserts the survivor into processed sets, optionally simplifies the watchlist, generates inferences, sorts the tmp store, and finally calls `insert_new_clauses`. Rust now ports a staged wrapper over the local selected-clause path: HCB selection, processed/stat marking, GC archive-copy cleanup on contraction, forward contraction, answer-literal evaluation on returned answers, dynamic AC status, replacement inference routing, watchlist checks, backward simplification by rewrite/subsumption/unit/context-SR, processed insertion, local watchlist simplification, staged first-order plain and simultaneous/super-simultaneous generation including unindexed paramodulation, C-position deterministic tmp-store sorting, and `insert_new_clauses` for generated/backward-simplified clauses. Process-clause state-owned indexed and higher-order generation remain explicit diagnostics.
- `Saturate` wraps `ProcessClause` in the top-level loop, checking `TimeIsUp`, unprocessed emptiness, step/processed/unprocessed/total/generated/term-bank-insertion limits, and watchlist completion before each iteration; after a non-returning processed clause it runs `cleanup_unprocessed_clauses` and then the SAT-check trigger gates. Rust now ports this staged loop over the available `ProcessClause` path, preserving limit-check order, signal-timeout observation, cleanup returns, C-shaped generated/term-bank unsigned comparisons, and active-empty-watchlist stopping. The SAT-check branch reports an explicit diagnostic when enabled and due because `SATCheck` itself remains pending.
- `eval_clause_set` evaluates every clause currently held in `state->eval_store` with the active HCB but does not drain or otherwise route those clauses; `insert_new_clauses` performs the later extraction and insertion into `unprocessed`. Rust now exposes this eval-store evaluation step as a standalone helper and preserves eval-store membership/order.
- The final `insert_new_clauses` tail drains evaluated clauses from `eval_store`, clears `CPIsOriented` again, emits the `"eval"` proof quote, and inserts the clauses into `unprocessed`. Rust now ports the local eval-store-to-unprocessed movement and reuses `ClauseSet` insertion to preserve evaluation indices.

### Change-Later Observations

- `ProofControlInit` mutates both the caller's heuristic-definition stack and the caller's heuristic parameter object while reconciling direct `heuristic_def` text with stacked `--define-heuristic` options. Rust preserves this in the current compatibility helper, but a later higher-level API could return an initialized control object and normalized parameter snapshot instead of exposing caller-side mutation.
- `ProofControlInit` calls `InitUnifLimits` after writing initialized params back to the caller. In C this stores a process-global pointer to `control->heuristic_parms` for higher-order CSU enumeration rather than deriving new values. Rust stores a safe snapshot of the fields read by the CSU and binding-generation helpers; revisit exact pointer aliasing only if a later port mutates `control->heuristic_parms` after initialization and expects CSU reads to observe those writes.
- In the `FVICollectFeatures` branch, `fvi_param_init` copies the overflow layout fields and assembly-vector length but allocates a fresh assembly vector rather than copying the original vector contents. Rust preserves that shape; revisit only if strategy files or generated specs make non-default assembly vectors observable at this boundary.
- `fvi_param_init` overwrites `cspec->max_symbols` for the active proof-state spec but not for `def_store_cspec`, which keeps the `FVCollectAlloc` default. Rust preserves this difference in the reusable spec helper.
- `fvi_param_init` computes and installs a permutation vector without consulting `control->fvi_parms.use_perm_vectors`; the CLI still records `Direct` versus `Perm` in that field. Rust preserves the initialization path and records the flag for compatibility, but this unused option bit should be revisited once indexed subsumption behavior is covered by end-to-end reference tests.
- The definition-store FV index uses the permutation vector computed from the active proof-state spec rather than from `def_store_cspec`. Rust preserves the effective packing input, but a future cleanup could make this dependency explicit or compute a definition-store-specific vector after compatibility is secured.
- `ProofStateInit` mixes FV-index setup, watchlist indexing, axiom reweighting, initial-clause copying, watchlist matching, derivation pushes, priority rewriting, SOS marking, AC scanning, and global-index reset in one function. Rust now exposes both a C-shaped wrapper for the ported prefix and narrower phase helpers around the same call order; once the remaining side effects are wired, decide whether the public API should keep the C monolith, retain explicit phase helpers, or separate compatibility initialization from reusable proof-state construction.
- The initial-clause path leaves `Uniq` evaluations attached to the source axiom set even though selection uses freshly copied clauses evaluated by the active HCB. Rust preserves this because it can affect later eval-order traversals of `state->axioms`, but a future cleanup could make the temporary ordering view explicit if reference tests show no caller depends on those source evaluations.
- `check_watchlist` builds the packed FV query before calling `ClauseSubsumeOrderSortLits` and recomputing the candidate's standard weight. Rust sorts and weights before using the indexed query helper because the reusable indexed helpers own packing internally. This should be equivalent for order-insensitive FV features, but keep it as a reference-test target before changing or generalizing watchlist lookup.
- Dynamic `check_watchlist` interleaves mutation with global output and proof-documentation quotes for both removed watched clauses and the matching new clause. Rust currently records the same local mutation and counts but leaves those output/derivation side effects for the proof-documentation integration.
- Dynamic `check_watchlist` deletes removed watched clauses from `state->wlindices` using `lambda_demod`; Rust removes them from the owned watchlist FV index and archive only. Add long-lived `wlindices` deletion when proof-state/global-index ownership is represented.
- `ForwardModifyClause` accepts a `context_sr` flag but does not read it; contextual simplify-reflect is performed by `forward_contract_keep` after packing instead. Rust preserves this split in the staged clause-level wrapper; revisit the public helper boundaries again when generated-clause routing and set-level contraction share the wrapper.
- `cleanup_unprocessed_clauses` bases delete-bad cleanup on `ProofStateStorage`, a C byte estimate over selected clause sets and `TBStorage`. Rust currently uses a domain-equivalent storage proxy from maintained clause/literal/evaluation counts plus non-variable term nodes; replace this with exact C-compatible memory accounting only after allocator and index storage models are settled.
- The cleanup orphan gate uses `ClauseSetDeleteOrphans`, whose exact C behavior depends on derivation entries holding live parent clause pointers. Rust keeps this as an explicit predicate for now; wire the default path only when proof-state clause handles can distinguish duplicate archived/requeued identifiers.
- `insert_new_clauses` adds `state->tmp_store->members` and `state->tmp_store->literals` to generated counters before any contraction, deletion, replacement inference, or splitting. Rust preserves the initial snapshot and separately adds destructive equality-resolution and fresh controlled-splitting replacement counts to `generated_count`; revisit literal accounting after definition reuse and proof-output reference tests cover split replacement clauses.
- `insert_new_clauses` handles `CPIsIRVictim` by calling `ForwardModifyClause` once with `FullRewrite`, ignoring that boolean return value, clearing `CPIsIRVictim`, and then running the normal `ForwardModifyClause` pass. Rust mirrors the double-modify shape; consider collapsing it only after inference-replacement tests prove the intermediate `CPLimitedRW` behavior is unobservable.
- An empty clause returned from `insert_new_clauses` stops before `eval_clause_set`, so earlier survivors can remain in `eval_store` without HCB evaluations and later clauses can remain in `tmp_store`. Rust preserves this partial-state return because it is visible to the surrounding proof-search loop.
- Aggressive forward subsumption inside `insert_new_clauses` uses `aggressive_forward_subsumed_count`, not the processed-clause `proc_forward_subsumed_count`. Rust preserves the separate statistic; any future stats cleanup should keep reference-output compatibility in mind.
- The higher-order-only `NormalizeEquations` and `ClausePruneArgs` hooks inside `ForwardModifyClause` are not wired in Rust yet, so the staged helper reports a diagnostic if it is called while the process problem type is higher-order. Port those hooks before enabling higher-order proof search through this path.
- `ForwardModifyClause` proof-documentation side effects such as `DocClauseModificationDefault(..., inf_minimize, ...)` are still pending. Rust currently preserves local clause mutation/statistics only.
- `ClausePositiveSimplifyReflect` can be affected by the C process-global `StrongUnitForwardSubsumption` flag; Rust currently uses the plain simplify-reflect path inside the proof-control helper until that global option is represented in proof-control state.
- `prefer_initial_clauses` is implemented as a blanket negative priority offset on every active-HCB evaluation cell after evaluation. Rust preserves the numeric `-PrioLargestReasonable` rewrite; a later strategy API could represent this as a named priority tier once byte-for-byte heuristic behavior is no longer the immediate constraint.
- `ClauseSetScanAC` reports activation only when a commutativity axiom is detected; associativity-only scans still mark the signature associative but leave `control->ac_handling_active` false. Rust preserves this return-value split, but a future API could separate "signature AC properties changed" from "commutativity activated AC handling" once callers are covered by reference tests.
- Dynamic `check_ac_status` prints the updated signature AC status and an activation message only when it turns AC handling on. Rust currently returns that transition as a boolean and leaves the output to the eventual proof-output layer.
- C stores `GlobalIndices` directly in `ProofStateCell` as raw pointers tied to `state->signature`. Rust currently initializes caller-owned indices because `ProofState` owns the `TermBank` and `GlobalIndices` borrows its signature; a later proof-session owner should hold both without requiring a self-referential struct.
- `ProofStateResetProcessedSet` archives the original processed clause and requeues a flat copy with the same identifier and copied properties, so the same clause identifier can appear in both `archive` and `unprocessed`, and the requeued copy can still carry `CPIsProcessed`. Rust preserves this because it is visible to clause-set inspection; a later compatibility-relaxed API could make requeue identity and processed-state clearing explicit.
- `ProofStateResetProcessedSet` conditionally records `DCCnfEvalGC` on the archived original when proof-object GC selection is enabled and always emits a `move_eval` quote through the proof-documentation layer. Rust records both the optional GC-selection metadata and the unconditional `DCCnfQuote` on the requeued copy; wire the remaining output side effects when `ccl_inferencedoc` ownership is integrated.
- Both reset and move-to-tmp paths delete `CPIsGlobalIndexed` clauses from `state->gindices` before moving them. Rust's current wrappers do not own long-lived proof-state global indices, so this deletion remains part of the future proof-session/global-index ownership work.
- The `eval_clause_set` comment claims no side effects even though the function adds evaluation cells to every clause in `eval_store`. Rust documents the mutation explicitly.
- C evaluates clauses after inserting them into `eval_store`, which means the set's evaluation trees do not receive entries until the clauses are later extracted and inserted into another set. Rust extracts and reinserts each evaluated clause inside `eval_store` to keep `ClauseSet` evaluation indices synchronized while preserving the visible clause order.
- `insert_new_clauses` clears `CPIsOriented` both before literal selection/eval-store insertion and again when draining `eval_store` into `unprocessed`. Rust preserves the tail clearing so the eventual full helper can remain C-shaped; a later cleanup could collapse duplicate clearing only after selection and proof-output tests prove it unobservable.
- The `insert_new_clauses` admission step stores `proc_non_trivial_count` in each clause's `create_date`. Rust currently has unsigned proof-state counters and a signed C-shaped clause date field, so the conversion saturates at `i64::MAX`; revisit the type split after end-to-end search/reference tests determine whether date overflow behavior needs exact C compatibility.
- `select_on_proc_only` bypasses the full `DoLiteralSelection` wrapper and clears only selected literal flags before eval-store insertion. Rust preserves this flag-level behavior; a later API could make the option name/phase split clearer without changing compatibility defaults.
- C treats `DoLiteralSelection` as infallible after option parsing, while the staged Rust port can still report a diagnostic for selector bodies that are not ported yet. Remove that staging-only error boundary once literal-selection coverage is complete.
- `insert_new_clauses` destructive equality resolution (`ClauseERNormalizeVar`) and controlled splitting (`ControlledClauseSplit`) both mutate `tmp_store` and generated counters when they replace a generated clause. Rust now ports the first-order destructive equality-resolution path and fresh-definition controlled splitting; split definition reuse still reports a diagnostic rather than silently admitting an unreused/unsplit clause.
- `replacing_inferences` sets `pclause->clause = NULL`, unpacks the `FVPackedClause_p`, and then delegates to `insert_new_clauses` whenever destructive equality resolution or splitting replaces the selected clause. Rust consumes the owned `FvPackedClause` and reports that state with `ReplacingInferenceOutcome`; replace this with stable clause handles once the selected-clause processing loop owns C-shaped packed-clause aliases.
- `replacing_inferences` performs destructive equality resolution when `er_varlit_destructive` is enabled even if `er_aggressive` is disabled. Rust preserves that separate gate; avoid collapsing it with generated-clause aggressive ER unless reference tests prove the phase distinction is unobservable.
- `ProcessClause` normalizes selected-clause variables with `state->freshvars`, while the current Rust proof state keeps only the term bank's `VarBank`. Rust clones that shared var-bank handle for the normalization call; split this back into an explicit proof-state fresh-variable owner if later reference tests show independent fresh-variable-bank state is observable.
- `ClauseSetIndexedInsert` receives the same packed-clause alias used by forward contraction and later unpacks it after processed-set insertion. Rust currently inserts an owned clause into the target set and rebuilds any FV index from that owned value; replace this with stable selected-clause handles when the full processed-clause loop is represented.
- `ProcessClause` deletes the exact `arch_copy` pointer if forward contraction consumes a selected clause after proof-object GC archive copying. Rust currently deletes the archived copy by identifier because archive entries are owned values; replace this with stable archive handles if duplicate archived identifiers become observable.
- The backward simplification calls inside `ProcessClause` delete processed clauses from `state->gindices`, kill children, emit proof-documentation quotes, and then either archive dead originals or move quoted flat copies into `tmp_store`. Rust ports the local set/archive/tmp-store movement and statistics but leaves global-index deletion, child-kill traversal, and proof-output quotes to the later proof-session/proof-documentation layer.
- `generate_new_clauses` begins with `VarBankSetVCountsToUsed(state->terms->vars)` before any generator runs. Rust's staged helper currently relies on each lower-level generator's fresh-variable handling; reintroduce an explicit proof-state fresh-variable owner if reference tests show this pre-seeding affects generated variable identities.
- `generate_new_clauses` calls higher-order generation unconditionally before the first-order generators. Rust currently reports a diagnostic for higher-order problem type before mutating `tmp_store`; port `ComputeHOInferences` before enabling higher-order selected-clause generation.
- `ProcessClause` always reaches `generate_new_clauses` unless the selected literal strategy is `NoGeneration`. Rust now allows generation-enabled first-order plain and simultaneous/super-simultaneous processing, including unindexed paramodulation, and has an explicit caller-owned indexed generation helper; remove the remaining staging diagnostics only when proof-control can use state-owned indexed paramodulation and higher-order generation.
- `simplify_watchlist` prefers `state->wlindices.bw_rw_index` when available and updates `wlindices` after reinserting simplified watched copies. Rust currently uses the owned watchlist set scan and local FV index only; add long-lived `wlindices` deletion/insertion when proof-state global-index ownership is represented.
- `simplify_watchlist` emits proof-documentation output for minimization and simplification moves. Rust preserves local mutation and `rw_count` but leaves proof-output quotes for the proof-documentation layer.
- `Saturate` compares signed `generated_limit` and `tb_insert_limit` against unsigned C counters, so negative limits become very large unsigned sentinels, and the generated counter expression can wrap if backward-rewritten clauses exceed generated clauses. Rust preserves this through explicit helper casts and wrapping subtraction; revisit only with reference tests if a higher-level limit API wants conventional signed sentinel semantics.
- C uses watchlist pointer presence to stop saturation once an active watchlist becomes empty. Rust proof states allocate an empty optional watchlist by default, so the port adds an explicit activation bit set by `ProofStateLoadWatchlist`; keep this compatibility shim until executable watchlist source ownership can represent "no watchlist" and "active empty watchlist" without ambiguity.
- `Saturate` increments SAT-check thresholds after each successful `SATCheck`, including repeated size-threshold catch-up and term-bank insertion threshold doubling. Rust currently stops with a staging diagnostic when a SAT check is enabled and due; port the threshold mutation together with the SAT solver bridge so satisfiable SAT-check returns preserve C scheduling.
- The `insert_new_clauses` eval-store drain emits `DocClauseQuoteDefault(..., "eval")` in C. Rust currently ports only the set movement and leaves the quote for proof-documentation/global-output integration.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
