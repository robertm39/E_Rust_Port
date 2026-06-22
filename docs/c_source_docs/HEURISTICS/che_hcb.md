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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

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

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
