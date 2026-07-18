<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_hcb

## Source Files

- [HEURISTICS/che_hcb.h](../../../eprover/HEURISTICS/che_hcb.h)
- [HEURISTICS/che_hcb.c](../../../eprover/HEURISTICS/che_hcb.c)

## Purpose

Heuristic control blocks, describing heuristics for clause selection. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ACHandlingType`
- `ClauseSelectFun`
- `ExtInferenceType`
- `HCBCell`
- `HCB_p`
- `HeuristicParmsCell`
- `HeuristicParms_p`
- `PrimEnumMode`
- `UnifMode`

### Macros And Constants

- `CHE_HCB`
- `DEFAULT_DELETE_BAD_LIMIT`
- `DEFAULT_FILTER_ORPHANS_LIMIT`
- `DEFAULT_FORWARD_CONTRACT_LIMIT`
- `DEFAULT_MINISCOPE_LIMIT`
- `DEFAULT_PM_FROM_INDEX_NAME`
- `DEFAULT_PM_INTO_INDEX_NAME`
- `DEFAULT_RW_BW_INDEX_NAME`
- `DEFAULT_SYM_OCCS`
- `EIT2STR(x)`
- `HCBCellAlloc()`
- `HCBCellFree(junk)`
- `HCB_DEFAULT_HEURISTIC`
- `HeuristicParmsCellAlloc()`
- `HeuristicParmsCellFree(junk)`
- `NO_ELIM_LEIBNIZ`
- `NO_EXT_SUP`
- `PEM2STR(x)`
- `STR2PEM(val)`
- `STR2UM(val)`
- `UM2STR(x)`

### Globals

- None found in the source scan.

### Exported Functions

- `(HeuristicParmsCell*)SizeMalloc(sizeof(HeuristicParmsCell)) SizeFree(junk, sizeof(HeuristicParmsCell)) void HeuristicParmsInitialize(HeuristicParms_p handle)`
- `Clause_p HCBSingleWeightClauseSelect(HCB_p hcb, ClauseSet_p set)`
- `Clause_p HCBStandardClauseSelect(HCB_p hcb, ClauseSet_p set)`
- `HCB_p HCBAlloc(void)`
- `HeuristicParms_p HeuristicParmsAlloc(void)`
- `HeuristicParms_p HeuristicParmsParse(Scanner_p in, bool warn_missing)`
- `PERF_CTR_DECL(ClauseEvalTimer)`
- `bool HeuristicParmsParseInto(Scanner_p in, HeuristicParms_p handle, bool warn_missing)`
- `long HCBAddWFCB(HCB_p hcb, WFCB_p wfcb, long steps)`
- `long HCBClauseSetDelProp(HCB_p hcb, ClauseSet_p set, long number, FormulaProperties prop)`
- `long HCBClauseSetDeleteBadClauses(HCB_p hcb, ClauseSet_p set, long number)`
- `void HCBClauseEvaluate(HCB_p hcb, Clause_p clause)`
- `void HCBFree(HCB_p junk)`
- `void HeuristicParmsFree(HeuristicParms_p junk)`
- `void HeuristicParmsPrint(FILE* out, HeuristicParms_p handle)`

## Implementation Notes

### Internal Functions

- `get_next_clause`

### Source-Level Behavior

- `str2eit`: Parse the value of ExtInferenceType parameter.
- `get_next_clause`: Return the next clause from the selected EvalTreeTraverse-Stack, or NULL if the stack is empty.
- `HeuristicParmsInitialize`: Initialize a heuristic parameters cell.
- `HeuristicParmsAlloc`: Allocate a cell for parameters, with initialized empty stacks.
- `HeuristicParmsFree`: Free a parameter cell.
- `HeuristicParmsPrint`: Print a HeuristicParmsCell in human/computer-readable form.
- `HeuristicParmsParseInto`: Parse the HeuristicParmsCell into/over the existing cell. Parameters are expected in-order, but may be missing. Returns true if all parameters have been found, false otherwise. The PARSE_-macros are in che_to_params.h (because they are also used to parse the ordering parameters).
- `HeuristicParmsParse`: Parse a (newly allocated) HeuristicParmsCell and return it.
- `HCBAlloc`: Return an empty, initialized HCB.
- `HCBFree`: Free a heuristics control block.
- `HCBAddWFCB`: Add a WFCB with to the HCB, adjust selection function. Return number of weight functions in HCB.
- `HCBClauseEvaluate`: Giben a HCB-Block, add evaluations to the given clause.
- `HCBStandardClauseSelect`: Select a clause from set, based on the evaluations and the data in hcb.
- `HCBSingleWeightClauseSelect`: Select a clause from the set based on the first weight.
- `HCBClauseSetDelProp`: Delete the property prop from the first number clauses in set that would be picked according to hcb. Note that this is _not_ reliable, as in real processing, clauses that would have been picked may vanish due to missing parents. It should be a fairly good approximation, though.
- `HCBClauseSetDeleteBadClauses`: Delete all but the best number clauses from the set.

### Dependencies

- `"che_hcb.h"`
- `<ccl_clausefunc.h>`
- `<ccl_condensation.h>`
- `<ccl_paramod.h>`
- `<ccl_satinterface.h>`
- `<ccl_splitting.h>`
- `<ccl_unfold_defs.h>`
- `<che_litselection.h>`
- `<che_to_params.h>`
- `<che_to_precgen.h>`
- `<che_to_weightgen.h>`
- `<che_wfcbadmin.h>`
- `<clb_dstacks.h>`
- `<clb_permastrings.h>`

### Compile-Time Conditions

- `CHE_HCB`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for selection-time parent liveness on 2026-07-13 and executable missing-field warnings on 2026-07-17.

Source files reviewed: `HEURISTICS/che_hcb.h`, `HEURISTICS/che_hcb.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 1398 lines, 25 scanned public declarations, 1 scanned internal function definitions, and 18 structured function-comment blocks.
- Heuristic control block execution; priority queues and evaluation order directly shape search.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.

### Compatibility Notes

- `HeuristicParmsInitialize` sets the BCE, predicate-elimination, and `lambda_demod` fields, but the current `HeuristicParmsPrint`/`HeuristicParmsParseInto` code deliberately omits those fields. Preserve the stored defaults, but treat strategy-file round-tripping for those settings as a later compatibility decision rather than an available C feature.
- `HeuristicParmsParseInto` expects the same fixed field order emitted by `HeuristicParmsPrint`, but it treats each field as optional: if the next token does not match the expected field name, the existing value is left unchanged and parsing continues with the following field. Preserve this sparse in-order override behavior before considering a map-based parser.
- Parsing stores `sine`, `heuristic_name`, and `heuristic_def` through `PermaStringStore`, while `HeuristicParmsFree` releases only the cell. The Rust port can own these strings directly, but parser integration should decide whether the permanent-string allocation pattern has observable lifetime behavior before replacing it everywhere.
- `HeuristicParmsPrint` renders the default missing `sine` as `"None"` and the missing `heuristic_def` as an empty string. `HeuristicParmsParseInto` parses those printed strings back into real stored strings, so a default print/parse cycle does not reconstruct the exact initialized cell.
- `mem_limit` is parsed with `ParseIntMax`, whose sign handling negates even positive decimal literals before the value is assigned to unsigned `rlim_t`; this makes printed positive values wrap when parsed as configuration input. Keep this compatibility quirk isolated so it can be removed later if strategy-file compatibility permits it.
- `selection_strategy` is a function pointer initialized to `SelectNoLiterals`; the public strategy-file spelling for that function is `NoSelection`. Rust code that stores the name should convert through the literal-selection table before runtime selection is wired in.
- `HCBFree` releases only the HCB's pointer arrays and optional HCB-local data; it intentionally does not free stored WFCBs because those come from `WFCBAdmin`. Rust should continue to store handles or borrows to admin-owned WFCBs rather than transferring ownership into HCB.
- `HCBAddWFCB` converts each added `steps` value into a cumulative switch-count boundary and changes the selector from `HCBSingleWeightClauseSelect` to `HCBStandardClauseSelect` after the second WFCB. Preserve that cumulative representation before considering a clearer schedule data type.
- `HCBClauseEvaluate` assumes `clause->evaluations == NULL`, allocates a fresh `EvalCell` sized to `wfcb_no`, and writes WFCB evaluations in list order. Rust clause-owned storage now preserves this shape, keeps the explicit evaluation adapter for callers not yet integrated with clause-owned evaluations, and exposes a banked HCB evaluation path for WFCBs whose C callbacks mutate maximality/orientation while scoring. Proof-control now uses that banked path for state-owned axiom init, processed reset, forward-contract reweight, cleanup reweight, and eval-store scoring.
- The completed production audit finds zero immutable HCB evaluation/reweight calls outside the explicit adapter modules and eight banked proof-control calls across the named lifecycle boundaries. Immutable HCB functions remain low-level/test adapters, not an alternate proof-search path; exact call records are in [`experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md`](../../../experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md).
- `HCBStandardClauseSelect` updates `select_count` and `current_eval` after `ClauseSetFindBest`/orphan removal, compares the incremented count against cumulative switch boundaries with equality, and resets only when `current_eval == wfcb_no`. Rust preserves this order through clause-set eval-index selection; proof control detaches the unprocessed owner before checking generation-qualified compact parent references directly in stable source, processed, and archive owners. Processed sets use maintained identifier positions followed by exact generation comparison, while periodic cleanup builds a hash snapshot of the same stable references across every owner it can mutate.
- `HCBClauseSetDelProp` appears to use `PDArrayElementInt(hcb->select_switch, j)` as the inner-loop bound while `j` is also the loop variable. Rust preserves that compiled loop-bound shape for delete-bad/filtering compatibility; it may be an accidental loop-index bug worth cleaning up later.
- `SplitClassType` is used as a bitmask despite being declared as an enum. Values such as `SplitHorn | SplitNonHorn` are accepted by the executable option handler and by `HeuristicParmsParseInto`, so Rust preserves raw C-width bit patterns rather than rejecting combinations as invalid enum discriminants.
- `SplitAll` is the numeric mask `7`, so it does not include `SplitPositive` (`8`) or `SplitMixed` (`16`) despite the name. Keep that value until clause-splitting callers prove whether this is intentional legacy behavior or a cleanup candidate.
- `HeuristicParmsParseInto(..., true)` emits its own missing-field warnings and forwards `OrderParmsParseInto` warnings in parse order. Rust's report-backed parser now feeds the executable warning owner before selected-strategy lookup; exact normal-search and later-error output, including C's doubled newline, is retained in [`experiments/2026-07-17-074-strategy-warning-output/FINDINGS.md`](../../../experiments/2026-07-17-074-strategy-warning-output/FINDINGS.md).

### Change Later

- `HCBStandardClauseSelect` interleaves evaluation-queue extraction with `ClauseIsOrphaned`, whose C parent checks depend on raw derivation pointers into long-lived clause owners. Rust has completed the safe-identity replacement with `ClauseDerivationRef`: nonzero generations remain stable across moves/renumbering and distinguish reused visible IDs, while generation-zero references preserve legacy identifier/source behavior. Selection performs indexed stable-owner lookup only for candidates it examines; periodic bulk cleanup materializes a compact stable-reference hash snapshot. A maintained proof-wide liveness registry should replace those two lookup strategies only if profiling justifies its per-clause mutation and memory cost.
- HCB evaluation cells and clause-set evaluation indexes store multiple views of the same scheduling state and rely on mutation order for consistency. Rust preserves this representation; after compatibility is secured, an indexed owner that updates both views through one API would make stale-entry invariants easier to enforce and benchmark.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
