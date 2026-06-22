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

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
