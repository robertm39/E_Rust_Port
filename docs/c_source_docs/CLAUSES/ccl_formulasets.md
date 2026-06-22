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

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
