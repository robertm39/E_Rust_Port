<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_formulasets

## Source Files

- [CLAUSES/ccl_formulasets.h](../../../eprover/CLAUSES/ccl_formulasets.h)
- [CLAUSES/ccl_formulasets.c](../../../eprover/CLAUSES/ccl_formulasets.c)

## Purpose

Data type for (wrapped) formula sets. the GNU Lesser General Public License. <1> Thu Jun 11 16:24:27 CEST 2009 New (factored out from ccl_wrapped_formulas.h)

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FormulaSetCell`
- `FormulaSet_p`

### Macros And Constants

- `CCL_FORMULASETS`
- `FormulaSetCardinality(set)`
- `FormulaSetCellAlloc()`
- `FormulaSetCellFree(junk)`
- `FormulaSetEmpty(set)`
- `FormulaSetMoveFormula(set, form)`

### Globals

- None found in the source scan.

### Exported Functions

- `FormulaSetExtractEntry(form);FormulaSetInsert((set), (form)) int FormulaConjectureOrder(FormulaSet_p set)`
- `FormulaSet_p FormulaSetAlloc(void)`
- `WFormula_p FormulaSetExtractEntry(WFormula_p form)`
- `WFormula_p FormulaSetExtractFirst(FormulaSet_p set)`
- `bool FormulaSetHasInterpretedSymbol(FormulaSet_p set)`
- `bool FormulaSetIsUntyped(FormulaSet_p set)`
- `long FormulaSetCollectFCode(FormulaSet_p set, FunCode f_code, PStack_p result)`
- `long FormulaSetCountConjectures(FormulaSet_p set, long* hypos)`
- `long FormulaSetInsertSet(FormulaSet_p set, FormulaSet_p from)`
- `long FormulaSetSplitConjectures(FormulaSet_p set, PList_p conjectures, PList_p rest)`
- `long long FormulaSetStandardWeight(FormulaSet_p set)`
- `void FormulaSetAppEncode(FILE* out, FormulaSet_p set)`
- `void FormulaSetDefinitionStatistics(FormulaSet_p orig, FormulaSet_p arch, TB_p bank, int* num_defs, double* percentage_form_defs, int* num_lams, bool* app_var_lits)`
- `void FormulaSetDeleteEntry(WFormula_p form)`
- `void FormulaSetFree(FormulaSet_p set)`
- `void FormulaSetFreeFormulas(FormulaSet_p set)`
- `void FormulaSetGCMarkCells(FormulaSet_p set)`
- `void FormulaSetInsert(FormulaSet_p set, WFormula_p newform)`
- `void FormulaSetMarkPolarity(FormulaSet_p set)`
- `void FormulaSetPrettyPrintTSTP(FILE* out, FormulaSet_p set, bool fullterms)`
- `void FormulaSetPrint(FILE* out, FormulaSet_p set, bool fullterms)`
- `void FormulaStackCondSetType(PStack_p stack, FormulaProperties type)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `FormulaSetAlloc`: Allocate and initialize a formula set.
- `FormulaSetFreeFormulas`: Free all formulas in set.
- `FormulaSetFree`: Free a formula set (and all its formulas).
- `FormulaSetStackCardinality`: Assume stack is a stack of formulasets. Return the number of formulas in all the sets.
- `FormulaSetGCMarkCells`: For all tformulas in set, mark their cells as being in use (for garbage collection).
- `FormulaSetMarkPolarity`: Mark the polarity of all subformulas in set.
- `FormulaSetInsert`: Insert newnode into set.
- `FormulaSetInsertSet`: Move all formulas from from into set (leaving from empty, but not deleted).
- `FormulaSetExtractEntry`: Extract a given formula from a formula set and return it.
- `FormulaSetExtractFirst`: Extract and return the first formula from set, if any, otherwise return NULL.
- `FormulaSetDeleteEntry`: Delete an element of a formulaset.
- `FormulaSetIsUntyped`: Return true if the formulaset is untyped, false otherwise.
- `FormulaSetPrint`: Print a set of formulae.
- `FormulaSetPrintPrettyPrintTSTP`: Print a set of formulae.
- `FormulaSetAppEncode`: App encodes the set of formulas and prints them to out. Initial set is not changed.
- `FormulaSetHasInterpretedSymbol`: Return true if any formula from set has a symbol from an interpreted sort.
- `FormulaSetSplitConjectures`: Find all (real or negated) conjectures in set and sort them into conjectures. Collect the rest in rest. Return number of conjectures found.
- `FormulaSetStandardWeight`: Return the sum of the standardweight of all clauses in set.
- `FormulaSetCountConjectures`: Count and return number of conjectures (and negated_conjectures) in set. Also find number of hypotheses, and add it to *hypos.
- `FormulaStackCondSetType`: Set the type of all formulas on stack to type if that does not change the semantics of the formula.
- `FormulaSetCollectFCode`: Push all formulas that contain f_code onto result. Return number of formulas found.
- `FormulaSetDefinitionStatistics`: Store information about the number of definitions and the percentage of definitions that define Boolean symbols in the arguments.

### Dependencies

- `"ccl_formulafunc.h"`
- `"ccl_formulasets.h"`
- `<ccl_formula_wrapper.h>`
- `<clb_plist.h>`

### Compile-Time Conditions

- `CCL_FORMULASETS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_formulasets.h`, `CLAUSES/ccl_formulasets.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 859 lines, 24 scanned public declarations, 0 scanned internal function definitions, and 23 structured function-comment blocks.
- Data type for (wrapped) formula sets. the GNU Lesser General Public License. <1> Thu Jun 11 16:24:27 CEST 2009 New (factored out from ccl_wrapped_formulas.h)
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `FormulaSetAppEncode` first walks every wrapped formula with `PreloadTypes`, then prints app-encoded type declarations, app-encoded symbol declarations, and finally each non-`$true` formula. It takes the term bank from the first set entry and assumes formula-shaped wrappers when calling `WFormulaAppEncode`; because `eprover` calls it after parsing both formula and clause owners, declarations can still include symbols introduced only by skipped clauses in mixed `--app-encode` input. The staged Rust helper keeps the preload/declaration ordering explicit and takes the term bank directly; the remaining compatibility question is whether full executable formula ownership must preserve the first-list-cell bank lookup and mixed clause/formula declaration leakage.

### Rust Port Status

- `src/clauses/formulasets.rs` now stages the basic `DefaultWFormulaAlloc`/`WTFormulaAlloc`/`WFormulaFlatCopy` identity, generated id rendering, input-name preservation policy, formula property/type helpers, and append-order `FormulaSet` owner operations for allocation, insert, insert-set, extract-first, extract-entry, delete-entry, move-formula, cardinality, and emptiness.
- The staged owner also covers the set-level untyped/interpreted-symbol scans, lambda-definition-excluding standard weight, conjecture split/count/order helpers, set-level f-code collection, formula-set stack cardinality, conditional stack type propagation, `WFormulaReturnFCodes`/`WFormulaSymbolDiversity` wrapper symbol collection, `WFormulaGetLambdaDefinedSym` lambda-definition symbol extraction, `WFormClauseToClause`/`WFormulaOfClause` conversion helpers, formula-backed and clause-backed `WFormulaTPTPPrint`/`WFormulaTSTPPrintFlex`/`WFormulaPrint` rendering, formula-backed and clause-backed append-order `FormulaSetPrint`/`FormulaSetPrettyPrintTSTP` rendering, staged `WFormulaSimplify`/`FormulaSetSimplify` with optional thresholded term GC, staged `WFormulaAnnotateQuestion`/`WFormulaConjectureNegate` and `FormulaSetPreprocConjectures`, staged `WFormulaReplaceEqnWithEquiv`/`TFormulaUnrollFOOL` wrapper behavior and `WFormulaSetUnrollFOOL`, staged `TFormulaApplyDefs` and `TFormulaSetIntroduceDefs`, staged `TFormulaSetDelTermpProp`, staged `TFormulaSetNamedToDBLambdas`, `TFormulaSetLiftItes`, `TFormulaSetLiftLets`, `TFormulaSetLiftLambdas`, `TFormulaSetUnfoldDefSymbols`, and `TFormulaSetLambdaNormalize`, staged `FormulaSetArchive`, staged `FormulaSetDocInital`, staged `WFormulaCNF2`, staged post-CNF `ClauseSetLiftLambdas`, and `FormulaSetCNF2` supported phase pipeline with higher-order named-to-DB/ITE/LET/definition-symbol/lambda-to-forall preprocessing, post-CNF clause lambda lifting, and thresholded CNF-path term GC, `WFormulaAppEncode`/`FormulaSetAppEncode` rendering, `FormulaSetDefinitionStatistics` aggregation with C's non-top-lambda and applied-variable-literal scans, `FormulaSetGCMarkCells`-style term-bank GC marking, and `FormulaSetMarkPolarity`-style polarity marking over term-encoded formula payloads.
- Formula parsing, lambda-lift generalization reuse, proof-state formula archives, and exact pointer-stable formula handles remain pending.

### Change Later

- C represents each `FormulaSet` as an intrusive doubly linked list by mutating `set`, `pred`, and `succ` fields inside every `WFormula`. Rust currently uses by-value ownership with stable wrapper entry ids for the staged owner; once proof state owns formulas, revisit arena or stable-handle ownership so extraction and derivation paths can keep pointer-like identity without exposing list links as public API.
- C `WFormulaGetId` combines a process-global generated-id counter with the mutable `FormulasKeepInputNames` global. The Rust helper takes input-name preservation explicitly; decide later whether executable compatibility needs a global policy shim or whether explicit call-site policy is preferable.
- C `FormulaSetPrint` delegates to `WFormulaPrint`, so it inherits the process-global `OutputFormat` and LOP-warning fallthrough behavior. Rust uses an explicit staged print-format argument; executable integration must decide where to keep the global compatibility shim.
- C `WFormClauseToClause` allocates a fresh clause from a wrapped formula and copies wrapper properties and source info, but it does not preserve the wrapper id as the printed clause id. Rust mirrors that staged behavior; proof-state integration should keep wrapper identity separate from transient conversion ids.
- C `WFormulaOfClause` closes a clause into a fresh formula wrapper without copying clause properties or source info. Rust mirrors that metadata drop; proof-document integration should decide whether parent/source metadata belongs beside the conversion rather than on the fresh wrapper itself.
- C `FormulaSetSimplify` can trigger term-bank GC while walking the set and records `DCFofSimplify` on changed formulas. Rust stages the mutation/count, optional thresholded GC checks, and simplification opcode/recovered-term metadata, but the formula-owned derivation stack remains proof-state owner work.
- C `FormulaSetPreprocConjectures` walks formulas in set order, annotates questions before negating conjectures, records formula derivations/proof output, and can populate `question_assoc` for answer literals. Rust now preserves the mutation order and exposes `DCAnnoQuestion`/`DCNegateConjecture` opcodes, but the association map, proof-document side effects, and formula-owned derivation stack remain owner-integration work.
- C `WFormulaSetUnrollFOOL` walks formulas in set order, applies Boolean-equality replacement before FOOL unrolling, records `DCEqToEq` separately, and returns only the number of formulas changed by `do_fool_unroll`. Rust preserves the order and exposes separate counters, but formula-owned derivation/proof-document side effects remain proof-state owner work.
- C `TFormulaSetIntroduceDefs` first clears shared term flags, marks polarity, finds definitions only on non-clause wrappers, archives polarity-neutral definitions, inserts active definitions, then applies definitions over the expanded set. Rust preserves that order and exposes `DCIntroDef`/`DCFofQuote`/`DCSplitEquiv`/`DCApplyDef` opcode metadata, but proof-document output and stable formula-parent handles remain proof-state owner work.
- C `TFormulaSetDelTermpProp` skips wrappers whose `tformula` pointer is null while clearing the requested term flags recursively with `DEREF_NEVER` on represented formula payloads. Rust keeps staged default wrappers as a no-op; revisit only if executable formula-set insertion policy must reject formula-less wrappers.
- C `TFormulaSetNamedToDBLambdas` is a higher-order-only set pass even though the lower-level `NamedToDB` conversion assumes lambda cells it converts are named-lambda cells. Rust keeps DB-lambda-only formulas as no-ops before the later DB lambda normalization phase; revisit only if full formula parsing can produce mixed named/DB lambda payloads that need a deeper conversion policy.
- C `TFormulaSetLiftItes` is a higher-order-only set pass using `map_formula` and `do_ite_unroll` to rewrite formula-position `$ite` nodes and the first term-position `$ite` found through the enclosing literal position. Rust now preserves higher-order gating, insertion order, left-before-right literal scanning, and `DCLiftIte` metadata; the C app-flattening helper path, lambda-bound subterm cases, and proof-document side effects should be revisited with reference traces when the proof-state formula owner lands.
- C `TFormulaSetLiftLets` renames quantified variables, closes local definitions over captured plus formal variables, replaces old local heads by fresh global heads through an f-code-keyed map, inserts generated definitions in stack-pop order, and records `DCIntroDef`/`DCApplyDef` provenance. Rust now preserves those staged owner effects; captured-variable ordering, nested LETs, app flattening, and proof-document side effects should still be revisited with reference traces when the proof-state formula owner lands.
- C `TFormulaSetLiftLambdas` lifts lambdas from active formula payloads, records `DCIntroDef` for generated lambda definitions, and inserts unique generated definitions into the active set only after the original traversal. Rust now preserves those staged owner effects while seeding `VarBank` counts before fresh variable generation; C omits that seed in this standalone pass despite seeding in `ClauseSetLiftLambdas`, so keep exact variable-numbering parity as a later trace question.
- C `TFormulaSetUnfoldDefSymbols` builds an f-code-keyed map from `CPIsLambdaDef` wrappers, recognizes equation and predicate-equivalence definition shapes after stripping leading universal quantifiers, freshens lambda binders, validates distinct free-variable definition arguments through temporary `TPIsSpecialVar` flags, rejects direct self-recursion and remaining free variables, intersimplifies generated definitions, recursively rewrites formulas under `MAX_RW_STEPS`, archives generated flat-copy definitions, and then archives the recognized originals by extracting them from the active set. Rust now preserves those staged owner effects; the C post-recursion step decrement, temporary term-flag mutation, duplicate-symbol overwrite behavior, `unfold_only_forms` predicate gate, and proof-document side effects should be revisited with reference traces when formula/archive ownership lands.
- C `ClauseSetLiftLambdas` runs after formula-set CNF conversion, lifts lambdas from clause terms, pushes `DCLiftLambdas` using generated definition formulas, archives the generated definition, simplifies/clausifies a flat copy of it back into the same clause set, and keeps those definition clauses out of `FormulaSetCNF2`'s returned generated-clause count. Rust now preserves those staged post-CNF side effects; the C `PDTree` generalization reuse in `LiftLambdas` remains a compatibility/performance item to revisit with reference traces.
- C `FormulaSetArchive` replaces the source set with flat copies after moving originals into the archive, and the replacement copies quote the archived originals. Rust preserves that movement order and exposes quote-source metadata, but formula-owned derivation stacks remain proof-state owner work.
- C `FormulaSetDocInital` documents formulas only at level two or higher and assigns fresh proof-document ids to the formulas it prints. Rust preserves that gate and id mutation through `ProofDocSession` and represented formula views, but executable formula-owner call-site integration remains proof-state owner work.
- C `WFormulaCNF2` mutates the wrapper formula and pushes formula-level derivation entries during each changed CNF phase. Rust now exposes those phase opcodes in the staged result but does not yet store a formula-owned derivation stack; add that stack when formula archives/proof documents become real owners.
- C `FormulaSetCNF2` runs higher-order named-to-DB conversion, ITE lifting, LET lifting, definition-symbol unfolding, and optional lambda-to-forall normalization before the ordinary FOOL/simplify/definition/CNF drain, then optionally runs post-CNF clause lambda lifting. Rust now stages named-to-DB, ITE lifting, LET lifting, definition-symbol unfolding, lambda-to-forall, and post-CNF clause lambda lifting in that order, preserves archive/GC order, and exposes quoted formula sources plus recovered-term counts; the standalone formula-set lambda-lift owner pass is available separately, while the real formula derivation stack/proof-document output still belongs with proof-state/archive ownership.
- C `FormulaSetPrettyPrintTSTP` discovers the signature through the first formula's term bank and prints all known type declarations when the set is not fully untyped. Rust takes the term bank explicitly while preserving declaration-before-formula ordering; decide during executable formula-owner integration whether first-entry signature coupling and declaration leakage from unrelated symbols must be emulated.
- C `FormulaSetAppEncode` discovers the term bank through the first set entry and mixes declaration preloading with formula emission. The staged Rust helper takes the term bank explicitly while preserving preload/declaration/formula ordering; decide during executable formula-owner integration whether first-entry bank coupling or mixed clause/formula declaration leakage must be emulated.
- C `WFormulaGCMarkCells` has a comment suggesting non-term formulas are a no-op, but it calls `TFormulaGCMarkCells`/`TBGCMarkTerm` directly and therefore depends on non-null bank/formula inputs. Rust currently treats a default allocated wrapper without a formula term as a no-op during GC marking so staged construction can be traversed safely; revisit this if exact assertion behavior becomes part of compatibility testing.
- C `WFormulaMarkPolarity` also assumes a non-null term formula. Rust currently treats a default allocated wrapper without a formula term as a no-op during set-level polarity marking; revisit this with the wrapper insertion policy if exact assertion behavior becomes observable.
- C `WFormulaGetLambdaDefinedSym` accepts an `equiv(eqn(lhs, rhs), ...)` lambda-definition shape without checking that `rhs` is `$true`, but `FormulaSetDefinitionStatistics` only counts that predicate-definition shape when `rhs` is literally the shared `$true` term. Rust mirrors both shapes in their respective helper paths; keep this mismatch visible when connecting formula statistics to proof-state feature extraction.
- C `FormulaSetDefinitionStatistics` writes `int` counters through caller pointers. Rust returns a typed statistics struct and saturates the staged `i32` counters; decide during proof-state feature integration whether executable compatibility needs exact C-width behavior or wider Rust feature counters.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
