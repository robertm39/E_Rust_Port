<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_clausesetfeatures

## Source Files

- [HEURISTICS/che_clausesetfeatures.h](../../../eprover/HEURISTICS/che_clausesetfeatures.h)
- [HEURISTICS/che_clausesetfeatures.c](../../../eprover/HEURISTICS/che_clausesetfeatures.c)

## Purpose

Functions for determining various features of clause sets. the GNU Lesser General Public License. <1> Mon Sep 28 19:17:50 MET DST 1998 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `SpecFeatureCell`
- `SpecFeature_p`
- `SpecFeatures`
- `SpecLimitsCell`
- `SpecLimits_p`

### Macros And Constants

- `ADJUST_FOR_HO(limit, scale)`
- `AX_1_DEFAULT`
- `AX_4_DEFAULT`
- `AX_MANY_DEFAULT`
- `AX_SOME_DEFAULT`
- `CHE_CLAUSESETFEATURES`
- `ClauseSetAxiomsAreHorn(set)`
- `ClauseSetAxiomsAreUnit(set)`
- `ClauseSetCountAxioms(set)`
- `ClauseSetCountHornAxioms(set)`
- `ClauseSetCountNonGroundUnitAxioms(set)`
- `ClauseSetCountUnitAxioms(set)`
- `ClauseSetGoalsAreGround(set)`
- `ClauseSetGoalsAreHorn(set)`
- `ClauseSetGoalsAreUnit(set)`
- `ClauseSetIsEquational(set)`
- `ClauseSetIsEquationalSet(set)`
- `ClauseSetIsGround(set)`
- `ClauseSetIsHornSet(set)`
- `ClauseSetIsPureEquationalSet(set)`
- `ClauseSetIsUnitSet(set)`
- `DEFAULT_CLASS_MASK`
- `DEFAULT_OUTPUT_DESCRIPTOR`
- `DEFS_LARGE_DEFAULT`
- `DEFS_MEDIUM_DEFAULT`
- `DEFS_PERC_LARGE_DEFAULT`
- `DEFS_PERC_MEDIUM_DEFAULT`
- `DEPTH_DEEP_DEFAULT`
- `DEPTH_MEDIUM_DEFAULT`
- `FAR_SUM_LARGE_DEFAULT`
- `FAR_SUM_MED_DEFAULT`
- `FUNC_LARGE_DEFAULT`
- `FUNC_MEDIUM_DEFAULT`
- `FUN_LARGE_DEFAULT`
- `FUN_MEDIUM_DEFAULT`
- `GET_ENCODING(idx)`
- `GPC_ABSOLUTE`
- `GPC_FEW_ABSDEFAULT`
- `GPC_FEW_DEFAULT`
- `GPC_MANY_ABSDEFAULT`
- `GPC_MANY_DEFAULT`
- `IS_NON_FO_TERM(t)`
- `LIT_MANY_DEFAULT`
- `LIT_SOME_DEFAULT`
- `NGU_ABSOLUTE`
- `NGU_FEW_ABSDEFAULT`
- `NGU_FEW_DEFAULT`
- `NGU_MANY_ABSDEFAULT`
- `NGU_MANY_DEFAULT`
- `NUM_LAMS_LARGE_DEFAULT`
- `NUM_LAMS_MEDIUM_DEFAULT`
- `ORDER_LARGE_DEFAULT`
- `ORDER_MEDIUM_DEFAULT`
- `PERC_APPLIT_LARGE_DEFAULT`
- `PERC_APPLIT_MEDIUM_DEFAULT`
- `PREDC_LARGE_DEFAULT`
- `PREDC_MEDIUM_DEFAULT`
- `PRED_LARGE_DEFAULT`
- `PRED_MEDIUM_DEFAULT`
- `SPEC_STRING_MEM`
- `SYMBOLS_LARGE_DEFAULT`
- `SYMBOLS_MEDIUM_DEFAULT`
- `Spec(spec)`
- `SpecAvgFArity0(spec)`
- `SpecAvgFArity1(spec)`
- `SpecAvgFArity2(spec)`
- `SpecAvgFArity3Plus(spec)`
- `SpecAxiomsAreGeneral(spec)`
- `SpecAxiomsAreHorn(spec)`
- `SpecAxiomsAreNonUnitHorn(spec)`
- `SpecAxiomsAreUnit(spec)`
- `SpecDeepMaxDepth(spec)`
- `SpecFeatureCellAlloc()`
- `SpecFeatureCellFree(junk)`
- `SpecFewAxioms(spec)`
- `SpecFewGroundPos(spec)`
- `SpecFewLiterals(spec)`
- `SpecFewNGPosUnits(spec)`
- `SpecGoalsAreGround(spec)`
- `SpecGoalsAreHorn(spec)`
- ... 30 more

### Globals

- None found in the source scan.

### Exported Functions

- `((set)->members-ClauseSetCountGoals(set)) long ClauseSetCountUnit(ClauseSet_p set)`
- `(ClauseSetCountUnitAxioms(set)-ClauseSetCountGroundUnitAxioms(set)) long ClauseSetCountRangeRestricted(ClauseSet_p set)`
- `(SpecLimitsCell*)SizeMalloc(sizeof(SpecLimitsCell)) SizeFree(junk, sizeof(SpecLimitsCell)) SpecLimits_p SpecLimitsAlloc(void)`
- `SpecLimits_p CreateDefaultSpecLimits(void)`
- `bool ClauseSetHasHOFeatures(ClauseSet_p set)`
- `char* SpecTypeString(SpecFeature_p features, const char* mask)`
- `double ClauseSetNonGoundAxiomPart(ClauseSet_p set)`
- `int ClauseSetComputeMaxOrder(ClauseSet_p set, Sig_p sig)`
- `long ClauseSetCollectArityInformation(ClauseSet_p set, Sig_p sig, int *max_fun_arity, int *avg_fun_arity, int *sum_fun_arity, int *max_pred_arity, int *avg_pred_arity, int *sum_pred_arity, int *non_const_funs, int *non_const_preds)`
- `long ClauseSetCountEqnLiterals(ClauseSet_p set)`
- `long ClauseSetCountGroundUnitAxioms(ClauseSet_p set)`
- `long ClauseSetCountHornGoals(ClauseSet_p set)`
- `long ClauseSetCountMaximalLiterals(ClauseSet_p set)`
- `long ClauseSetCountMaximalTerms(ClauseSet_p set)`
- `long ClauseSetCountPositiveAxioms(ClauseSet_p set)`
- `long ClauseSetCountSingletons(ClauseSet_p set)`
- `long ClauseSetCountUnitGoals(ClauseSet_p set)`
- `long ClauseSetCountUnorientableLiterals(ClauseSet_p set)`
- `long ClauseSetCountVariables(ClauseSet_p set)`
- `long ClauseSetMaxLiteralNumber(ClauseSet_p set)`
- `long ClauseSetMaxStandardWeight(ClauseSet_p set)`
- `long ClauseSetTPTPDepthInfoAdd(ClauseSet_p set, long* depthmax, long* depthsum, long* count)`
- `long ClauseSetTermCells(ClauseSet_p set)`
- `void ClauseSetComputeHOFeatures(ClauseSet_p set, Sig_p sig, bool* has_ho_features, int* order, bool* quantifies_bools, bool* has_defined_choice, double* perc_appvar_lit)`
- `void ClauseSetPrintNegUnits(FILE* out, ClauseSet_p set, bool printinfo)`
- `void ClauseSetPrintNonUnits(FILE* out, ClauseSet_p set, bool printinfo)`
- `void ClauseSetPrintPosUnits(FILE* out, ClauseSet_p set, bool printinfo)`
- `void ClausifyAndClassifyWTimeout(ProofState_p state, int timeout, char* mask, char class[SPEC_STRING_MEM])`
- `void ProofStatePrintSelective(FILE* out, ProofState_p state, char* descriptor, bool printinfo)`
- `void SpecFeaturesAddEval(SpecFeature_p features, SpecLimits_p limits)`
- `void SpecFeaturesCompute(SpecFeature_p features, ClauseSet_p cset, FormulaSet_p fset, FormulaSet_p arch, TB_p bank)`
- `void SpecFeaturesParse(Scanner_p in, SpecFeature_p features)`
- `void SpecFeaturesPrint(FILE* out, SpecFeature_p features)`
- `void SpecLimitsPrint(FILE* out, SpecLimits_p limits)`
- `void SpecTypePrint(FILE* out, SpecFeature_p features, char* mask)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `SpecLimitsAlloc`: Allocate an initialized SpecLimitsCell.
- `ClauseSetCountGoals`: Count number of goal clauses.
- `ClauseSetCountUnit`: Count the unit clauses in set.
- `ClauseSetCountUnitGoals`: Count the unit goal clauses in set.
- `ClauseSetCountHorn`: Count the unit clauses in set.
- `ClauseSetCountHornGoals`: Count the unit clauses in set.
- `ClauseSetCountEquational`: Count number of clauses with at least one equational literal.
- `ClauseSetCountPureEquational`: Count number of clauses which have only equational literals.
- `ClauseSetCountPosUnits`: Count number of positive unit clauses.
- `ClauseSetCountGroundGoals`: Count number of ground goal clauses.
- `ClauseSetCountGround`: Count number of ground clauses.
- `ClauseSetCountGroundUnitAxioms`: Count number of positive ground unit clauses.
- `ClauseSetCountGroundPositiveAxioms`: Count number of positive ground clauses.
- `ClauseSetCountPositiveAxioms`: Count number of positive ground clauses.
- `ClauseSetCountRangeRestricted`: Count number of positive ground clauses.
- `ClauseSetNonGoundAxiomPart`: Return the percentage of non-ground clauses among the unit clauses (0 if no unit clauses exist).
- `ClauseSetCollectArityInformation`: Collect information about the arities of function and predicate symbol arities. Average and sum for function symbols does not include constants, it does for predicate symbols. Equality is not counted, Returns number of function symbol constants.
- `ClauseSetCountMaximalTerms`: Count the number of maximal terms in maximal literals in clauses in set.
- `ClauseSetCountMaximalLiterals`: Count the number of maximal literals in clauses in set.
- `ClauseSetCountVariables`: Count the number of variables in a clause set, where variables in different clauses are considered to be distinct.
- `ClauseSetCountSingletons`: Count the number of singletons in a clause set, where variables in different clauses are considered to be distinct.
- `ClauseSetTPTPDepthInfoAdd`: Add the depth information in TPTP interpretation to the variables. See che_clausefeatures.c for more.
- `ClauseSetCountUnorientableLiterals`: Count the number of Unorientable literals in clauses in set.
- `ClauseSetCountEqnLiterals`: Count the number of equational literals in clauses in set.
- `ClauseSetMaxStandardWeight`: Return the standard weight of the largest clause in set (or -1 if set is empty).
- `ClauseSetTermCells`: Return the number of term positions in the clause set.
- `ClauseSetMaxLiteralNumber`: Return the length of the longest clause.
- `SpecFeaturesCompute`: Compute all relevant features for a set of clauses.
- `SpecFeaturesAddEval`: Add the cheap, subjective things to a SpecFeatureCell.
- `SpecFeaturesPrint`: Print the feature vector.
- `SpecFeaturesParse`: Parse the relevant (i.e. currently used and printed) parts of a spec features cell from in into a caller-provided structure. Also parse the type and extract the invariant parts from it.
- `SpecTypeString`: Encode the type of the problem as a n-letter code. 1) Axioms are [U]nit, [H]orn, [General] 2) Goals are [U]nit, [H]orn, [General] 3) [N]o equality, [S]ome equality, [P]ure equality 4) [F]ew, [S]ome, [M]any non-ground facts 5) [G]round goals or [N]on-ground goals
- `SpecTypePrint`: Print the string created by SpecTypeString
- `ClauseSetPrintPosUnits`: Print the positive unit clauses from set.
- `ClauseSetPrintNegUnits`: Print the negative unit clauses from set.
- `ClauseSetPrintNonUnits`: Print the non-unit clauses from set.
- `ProofStatePrintSelective`: Print parts of the proof state to the given stream. Descriptor controls which parts.
- `CreateDefaultSpecLimits`: Return a SpecLimits cell initialized with the default limits for Auto-Mode problem classification.
- `ClauseSetComputeHOFeatures`: Fill in the HO statistics such as: are there non-FO features of the problem, what is the maximal term order in the problem, does the problem quantify booleans and does it have defined choice clauses.
- `SpecLimitsPrint`: Fill in the HO statistics such as: are there non-FO features of the problem, what is the maximal term order in the problem, does the problem quantify booleans and does it have defined choice clauses.
- `ClausifyAndClassifyWTimeout`: Run the defaultclausification and get the corresponding classification string. If last three arguments are non-NULL, the full classification string with computed features will be output to stdout.

### Dependencies

- `"che_clausesetfeatures.h"`
- `<ccl_proofstate.h>`
- `<che_clausefeatures.h>`
- `<sys/wait.h>`

### Compile-Time Conditions

- `CHE_CLAUSESETFEATURES`
- `NDEBUG`
- `_choice`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
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

Source files reviewed: `HEURISTICS/che_clausesetfeatures.h`, `HEURISTICS/che_clausesetfeatures.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 2508 lines, 40 scanned public declarations, 0 scanned internal function definitions, and 41 structured function-comment blocks.
- Functions for determining various features of clause sets. the GNU Lesser General Public License. <1> Mon Sep 28 19:17:50 MET DST 1998 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `SpecFeaturesPrint` prints the higher-order tail fields after `clause_avg_depth`, but `SpecFeaturesParse` still expects the older vector shape ending at `clause_avg_depth` before `): class`. Rust preserves these as separate print and legacy parse surfaces instead of making them round-trip.
- `SpecTypeString` builds 21 classification bytes in a 22-byte local buffer, accepts masks with length 13 through 22, and returns only 21 bytes via `SecureStrndup(result, 21)`. A 22nd mask byte can affect only the C buffer terminator and is not observable in the returned string.
- `SpecFeaturesParse` accepts `G`, `H`, or `U` for the axiom class but only `H` or `U` for the goal class, even though `SpecTypeString` can encode general goals as `G`.
- `ClauseSetPrintNegUnits` is named as if it prints all negative unit clauses, but it filters on `ClauseIsUnit && ClauseIsGoal`; with the current clause predicates this means unit goal clauses. Rust preserves that filter in both the caller-rendered helper and the default LOP wrapper.
- `ClauseSetComputeHOFeatures` computes `has_defined_choice` by calling `ClauseRecognizeChoice(NULL, clause)`, which beta-normalizes and eta-reduces the two predicate terms before recognizing `~P X | P (f P)`. Rust now ports the non-choice HO statistics and exposes the choice result as a caller-supplied predicate until lambda normalization and the choice-symbol map are ported.
- `SpecFeaturesCompute` computes the clause-level higher-order order through `ClauseSetComputeHOFeatures`, then overwrites both `features->order` and `features->goal_order` with `1` before scanning formula archives/current formulas. If both formula sets are `NULL`, the final strategy order ignores the clause-level HO order. Rust preserves this in the clause-set helper; a later cleaned API should expose the raw clause HO order separately from the classification order.
- `SpecFeaturesCompute` sets `num_of_definitions = -1` but does not write `perc_of_form_defs`; `SpecFeaturesAddEval` can still classify whatever ratio is already in the feature cell. Rust leaves `perc_of_form_defs` untouched in the clause-set helper and should fill it only when formula-definition statistics are ported.
- `ClauseSetHasHOFeatures` and `ClauseSetComputeMaxOrder` are declared in `che_clausesetfeatures.h` but have no implementation in this checkout; leave them documented as header-only surface until a C definition or real caller appears.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
