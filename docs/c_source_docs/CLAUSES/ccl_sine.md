<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_sine

## Source Files

- [CLAUSES/ccl_sine.h](../../../eprover/CLAUSES/ccl_sine.h)
- [CLAUSES/ccl_sine.c](../../../eprover/CLAUSES/ccl_sine.c)

## Purpose

Code for a (generalized) version of the SinE formula selection algorithm. See http://www.cs.man.ac.uk/~hoderk/sine/. the GNU Lesser General Public License. <1> Fri Jul 2 00:55:03 CEST 2010

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `AxiomType`
- `DRelCell`
- `DRel_p`
- `DRelationCell`
- `DRelation_p`

### Macros And Constants

- `CCL_SINE`
- `DRelCellAlloc()`
- `DRelCellFree(junk)`
- `DRelationCellAlloc()`
- `DRelationCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `DRel_p DRelAlloc(FunCode f_code)`
- `DRel_p DRelationGetFEntry(DRelation_p rel, FunCode f_code)`
- `DRelation_p DRelationAlloc(void)`
- `long ClauseSetFindAxSelectionSeeds(ClauseSet_p set, PQueue_p res, bool inc_hypos)`
- `long DRelationTotalEntries(DRelation_p rel)`
- `long FormulaSetFindAxSelectionSeeds(FormulaSet_p set, PQueue_p res, bool inc_hypos)`
- `long SelectAxioms(GenDistrib_p f_distrib, PStack_p clause_sets, PStack_p formula_sets, PStackPointer hyp_start, AxFilter_p ax_filter, PStack_p res_clauses, PStack_p res_formulas)`
- `long SelectDefiningAxioms(DRelation_p drel, Sig_p sig, int max_recursion_depth, long max_set_size, bool trim, PQueue_p axioms, PStack_p res_clauses, PStack_p res_formulas)`
- `long SelectDefinitions(PStack_p clause_sets, PStack_p formula_sets, PStack_p res_clauses, PStack_p res_formulas)`
- `long SelectThreshold(PStack_p clause_sets, PStack_p formula_sets, AxFilter_p ax_filter, PStack_p res_clauses, PStack_p res_formulas)`
- `void DRelFree(DRel_p rel)`
- `void DRelPrintDebug(FILE* out, DRel_p rel, Sig_p sig)`
- `void DRelationAddClause(DRelation_p drel, GenDistrib_p generality, GeneralityMeasure gentype, double benevolence, long generosity, Clause_p clause)`
- `void DRelationAddClauseSet(DRelation_p drel, GenDistrib_p generality, GeneralityMeasure gentype, double benevolence, long generosity, ClauseSet_p set)`
- `void DRelationAddClauseSets(DRelation_p drel, GenDistrib_p generality, GeneralityMeasure gentype, double benevolence, long generosity, PStack_p sets)`
- `void DRelationAddFormula(DRelation_p drel, GenDistrib_p generality, GeneralityMeasure gentype, double benevolence, long generosity, bool trim, bool force_def, WFormula_p form)`
- `void DRelationAddFormulaSet(DRelation_p drel, GenDistrib_p generality, GeneralityMeasure gentype, double benevolence, long generosity, bool trim, bool force_def, FormulaSet_p set)`
- `void DRelationAddFormulaSets(DRelation_p drel, GenDistrib_p generality, GeneralityMeasure gentype, double benevolence, long generosity, bool trim, bool force_def, PStack_p sets)`
- `void DRelationFree(DRelation_p rel)`
- `void DRelationPrintDebug(FILE* out, DRelation_p rel, Sig_p sig)`
- `void PQueueStoreClause(PQueue_p axioms, Clause_p clause)`
- `void PQueueStoreFormula(PQueue_p axioms, WFormula_p form)`
- `void PStackClauseDelProp(PStack_p stack, FormulaProperties prop)`
- `void PStackClausePrintTSTP(FILE* out, PStack_p stack)`
- `void PStackClausesMove(PStack_p stack, ClauseSet_p set)`
- `void PStackFormulaDelProp(PStack_p stack, FormulaProperties prop)`
- `void PStackFormulaPrintTSTP(FILE* out, PStack_p stack)`
- `void PStackFormulasMove(PStack_p stack, FormulaSet_p set)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `DRelAlloc`: Allocate an initialized DRelCell for f_code.
- `DRelFree`: Free a DRel-Cell. Clauses and Formulas are external!
- `DRelationAlloc`: Allocate a complete DRelation.
- `DRelationFree`: Free a DRelation.
- `DRelationGetFEntry`: Return the entry for the DRel for f_code. Create one if it does not exist.
- `DRelationAddClause`: Add a clause to the D-Relation.
- `DRelationAddFormula`: Add a forrmula to the D-Relation
- `DRelationAddClauseSet`: Add all clauses in set to the D-Relation.
- `DRelationAddFormulaSet`: Add all formulas in set to the D-Relation.
- `DRelationAddClauseSets`: Add all clauses in sets on stack into the D-Relation.
- `DRelationAddFormulaSets`: Add all formulas in sets on stack into the D-Relation.
- `PQueueStoreClause`: Store the tuple (type, clause) in axioms.
- `PQueueStoreFormula`: Store the tuple (type, form) in axioms.
- `ClauseSetFindAxSelectionSeeds`: Find all conjectures and optionally hypotheses in set and store them in res. Returns number of seeds found.
- `FormulaSetFindAxSelectionSeeds`: Find all axiom selection seeds (conjecures and optionally hypotheses) in set and store them in res. Returns number of seeds found.
- `SelectDefiningAxioms`: Perform SinE-like axiom selection. All initially selected "axioms" (typically the conjectures/hypotheses) have to be in axioms, in the form of (type, pointer) values. Returns the number of axioms selected.
- `SelectAxioms`: Given a function symbol distribution, input sets (clauses and formulas) which contain the hypotheses (in a restricted part indicated by hyp_start), select axioms according to the D-Relation described by gen_measure and benevolence. Selected axioms are pushed onto res_clauses and res_formulas, the total number of selected axioms is returned.
- `SelectThreshold`: Dummy selector: If there are up to ax_filter->threshold clauses and formulas, pass them all. Otherwise pass none.
- `SelectDefinitions`: Select lambda definitions only
- `DRelPrintDebug`: Print a hint about clauses and formulas in D-Drelation with a given f_code.
- `DRelationPrintDebug`: Print a hint of the D-Relation to see what's going on.
- `DRelationTotalEntries`: Return the total number of clause/formula references in the D-Relation.
- `PStackClauseDelProp`: Delete prop in all clauses on stack.
- `PStackFormulaeDelProp`: Delete prop in all formulas on stack.
- `PStackClausePrintTSTP`: Print the clauses on the stack in TSTP format.
- `PStackFormulaPrintTSTP`: Print all the formulas on the stack.
- `PStackClausesMove`: Move all clauses on stack from their old set to set.
- `PStackFormulasMove`: Move all formulas on stack from their old set to set.

### Dependencies

- `"ccl_sine.h"`
- `<ccl_f_generality.h>`

### Compile-Time Conditions

- `CCL_SINE`
- `_symbols_in_drel`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_sine.h`, `CLAUSES/ccl_sine.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1322 lines, 33 scanned public declarations, 0 scanned internal function definitions, and 28 structured function-comment blocks.
- Code for a (generalized) version of the SinE formula selection algorithm. See http://www.cs.man.ac.uk/~hoderk/sine/. the GNU Lesser General Public License. <1> Fri Jul 2 00:55:03 CEST 2010
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `DRelAlloc` initializes `activated` to false and creates separate clause/formula stacks. `DRelationAlloc` starts with a 10-slot pointer array, `DRelationGetFEntry` grows by function-code index and creates missing entries, and `DRelationTotalEntries` counts clause plus formula entries from index 1 upward. Rust preserves the D-relation stack shape and the index-0 skip for clauses and staged formula refs.
- `DRelationAddClause` and `DRelationAddFormula` ask the matching `*ComputeDRel` helper for selected symbols, store no-symbol objects under function code 0, otherwise pop the selected symbol stack and push the object pointer into each symbol entry. `DRelationAddFormula` also appends a lambda-defined symbol when `force_def` requests it and the symbol is not already selected. Rust preserves the clause insertion shape, staged formula insertion shape, and C `TermTrimImplications` behavior for trimmed conjecture formula D-relations.
- `PStackClausePrintTSTP` and `PStackFormulaPrintTSTP` print entries in increasing stack-index order and append one newline after each object. Rust preserves the clause-stack text shape with explicit `ProblemType`, including typed first-order formula closure, and propagates diagnostics for the currently deferred higher-order formula-closure branch. Formula-stack TSTP printing now reuses the `WrappedFormula` TSTP renderer with full terms, complete output, explicit problem type, and caller-provided input-name preservation.
- `PStackClauseDelProp` and `PStackFormulaDelProp` mutate every object referenced by the stack without consuming or reordering the stack. Rust models these as stacks of mutable `Clause` and `WrappedFormula` references.
- `PStackClausesMove` and `PStackFormulasMove` unlink each stacked object from its current owner set and append it to the destination set; duplicate stack entries therefore relink an already-moved object to the destination tail. Rust now provides staged identifier/entry-id helpers over explicit old-owner and destination sets, including the duplicate relink behavior for objects already in the destination. Exact pointer-owner discovery remains deferred until stable clause/formula owner handles are available.
- `PQueueStoreClause` and `PQueueStoreFormula` write two adjacent queue entries, the raw axiom-type integer tag followed by the borrowed object pointer. `ClauseSetFindAxSelectionSeeds` and `FormulaSetFindAxSelectionSeeds` scan in set order and store conjectures plus hypotheses only when requested. Rust preserves the tuple layout with separate `IntOrP<&Clause>` and `IntOrP<&WrappedFormula>` queues for staged callers, and uses a typed borrowed `AxiomRef` payload for the staged mixed clause/formula selector while keeping C's adjacent tag/payload queue shape.
- `DRelPrintDebug` writes the relation summary and `formulas:` label to its `out` stream, but sends the terminating newline to `stderr`. Rust preserves that split-stream behavior for compatibility by requiring a separate stderr writer and now renders staged formula counts and `WFormulaGetId`-style ids. Future cleaned diagnostics should avoid this stream mismatch.
- `SelectDefiningAxioms` uses `ATNoType` queue markers as recursion-level delimiters, marks selected axioms with `CPIsRelevant`, activates each D-relation symbol only once, queues both clause and formula stacks for every newly activated symbol, and checks max-size/max-depth limits only at the start of each queue loop. Rust now ports the clause-side traversal and staged mixed clause/formula traversal with local selected-object identity tracking instead of transient property mutation.
- `SelectAxioms` constructs the D-relation from all clause/formula sets, seeds only sets from `seed_start` onward, inserts no-symbol axioms before defining-axiom traversal when requested, and can duplicate a no-symbol seed because that pre-insertion does not set `CPIsRelevant`. Rust preserves this behavior in the clause-side and staged mixed selectors, including the no-symbol duplicate quirk and C-style max-result-size truncation.
- `SelectThreshold` gates selection on combined clause/formula cardinality, pushes all clauses first and then formulas only when the total is within the threshold, and returns the final size of both result stacks rather than the number newly added. Rust preserves the staged clause/formula-set helper over borrowed `WrappedFormula` refs and the proof-state SInE call site now applies the combined cardinality gate to represented `axioms` plus `f_axioms`.
- `SelectDefinitions` ignores clause sets because C only marks formulas as lambda definitions, then keeps lambda-definition, conjecture, and hypothesis formulas. Rust now preserves that exact staged formula-set helper, including `question` through the shared conjecture predicate and the C return shape over `res_formulas` only. The proof-state SInE call site applies the same keep/drop policy to represented `f_axioms` and still clears pure clause-owner inputs when aggregate raw-formula metadata says no formula input contributed.
- Rust's executable SInE path relies on the all-or-nothing nature of `SelectThreshold` for threshold filters, uses selected clause identifiers and formula entry ids to move represented GSinE results into fresh axiom/formula-axiom sets without cloning, and applies the represented `LambdaDef` behavior above. Proof-search coverage now includes represented FOF formula owners pruned by the threshold filter before CNF emits clauses. Parser population of executable formula owners and pointer-owner-discovering mixed clause/formula movement remain deferred until stable clause/formula handles are wired through parsing and preprocessing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- C selected-axiom movement is raw-pointer based, so the same selected pointer can be moved more than once without creating a second object; this matters for the no-symbol seed duplicate quirk in `SelectAxioms` and for the stack move helpers' unlink/reinsert behavior. Rust's represented proof-state SInE move uses clause ids and formula entry ids with explicit source/destination sets; revisit this once stable handles allow owner-discovered relinking for duplicate selected objects across all parser/preprocessing owners.
- Mixed CNF-plus-formula inputs cannot currently attach a represented axiom back to its original C owner set after formula lowering. Rust can now move represented `f_axioms` during SInE, but parser-lowered formula clauses still act as clause owners; replace that approximation with exact owner handles when `WFormula`/`FormulaSet` are populated by the executable parser.

<!-- END MANUAL REVIEW: c_source_docs -->
