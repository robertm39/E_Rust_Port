<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_f_generality

## Source Files

- [CLAUSES/ccl_f_generality.h](../../../eprover/CLAUSES/ccl_f_generality.h)
- [CLAUSES/ccl_f_generality.c](../../../eprover/CLAUSES/ccl_f_generality.c)

## Purpose

Code for computing the generality of function/predicate symbols using a generalize SinE approach, counting occurences in terms, literals, clauses, and formulas. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FunGenCell`
- `FunGen_p`
- `GenDistribCell`
- `GenDistrib_p`

### Macros And Constants

- `CCL_F_GENERALITY`
- `GenDistribAddClauseSets(dist, stack)`
- `GenDistribAddFormulaSets(dist, stack, trim)`
- `GenDistribBacktrackClauseSets(dist, stack, sp)`
- `GenDistribBacktrackFormulaSets(dist, stack, sp)`
- `GenDistribCellAlloc()`
- `GenDistribCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `GenDistribAddClauseSetStack((dist), (stack), 0, 1) GenDistribAddFormulaSetStack((dist), (stack), 0, trim, 1) GenDistribAddClauseSetStack((dist), (stack), (sp), -1) GenDistribAddFormulaSetStack((dist), (stack), (sp), false, -1) void GenDistribPrint(FILE* out, GenDistrib_p dist, long limit)`
- `GenDistrib_p GenDistribAlloc(Sig_p sig)`
- `int FunGenCGCmp(const FunGen_p fg1, const FunGen_p fg2)`
- `int FunGenTGCmp(const FunGen_p fg1, const FunGen_p fg2)`
- `void ClauseComputeDRel(GenDistrib_p generality, GeneralityMeasure gentype, double benevolence, long generosity, Clause_p clause, PStack_p res)`
- `void FormulaComputeDRel(GenDistrib_p generality, GeneralityMeasure gentype, double benevolence, long generosity, WFormula_p form, PStack_p res, bool trim_impl)`
- `void GenDistribAddClause(GenDistrib_p dist, Clause_p clause, short factor)`
- `void GenDistribAddClauseSet(GenDistrib_p dist, ClauseSet_p set, short factor)`
- `void GenDistribAddClauseSetStack(GenDistrib_p dist, PStack_p stack, PStackPointer start, short factor)`
- `void GenDistribAddFormula(GenDistrib_p dist, WFormula_p form, bool trim, short factor)`
- `void GenDistribAddFormulaSet(GenDistrib_p dist, FormulaSet_p set, bool trim, short factor)`
- `void GenDistribAddFormulaSetStack(GenDistrib_p dist, PStack_p stack, PStackPointer start, bool trim, short factor)`
- `void GenDistribFree(GenDistrib_p junk)`
- `void GenDistribSizeAdjust(GenDistrib_p gd, Sig_p sig)`

## Implementation Notes

### Internal Functions

- `compute_d_rel`
- `extract_generality`
- `fun_gen_cg_cmp_wrapper`
- `fun_gen_tg_cmp_wrapper`
- `gd_merge_single_res`
- `init_fun_gen_cell`

### Source-Level Behavior

- `init_fun_gen_cell`: Initi a FunGenCell for keeping track of occurrences of f.
- `gd_merge_single_res`: Merge the new f-counts in dist_array into dist.
- `fun_gen_tg_cmp_wrapper`: Wrapper around FunGenTGCmp() to sort stacks of pointers.
- `fun_gen_cg_cmp_wrapper`: Wrapper around FunGenCGCmp() to sort stacks of pointers.
- `extract_generality`: Given a FunGen_p and a gentype, return the proper generality measure.
- `GenDistribAlloc`: Allocate an initialized GenDistribCell.
- `GenDistribFree`: Free a GenDistrib cell. The signature is external!
- `GenDistribSizeAdjust`: Ensure that GenDistrib is large enough to accomodate all symbols in sig.
- `GenDistribAddClause`: Add f_code occurrences to dist.
- `GenDistribAddClauseSet`: Add all clauses in set into the distribution.
- `GenDistribAddFormula`: Add a Formula to the distribution.
- `GenDistribAddFormulaSet`: Add all formulas in set into the distribution.
- `GenDistribAddClauseSetStack`: Add all clause sets on stack into dist.
- `GenDistribPrint`: Print the symbol distribution.
- `FunGenTGCmp`: Compare function for FunGen cell pointers, by term-frequency, tie-break by clause frequency, tie-break by f_code.
- `FunGenCGCmp`: Compare function for FunGen cell pointers, by clause/formula-frequency, tie-break by term frequency, tie-break by f_code.
- `ClauseComputeDRel`: Push the FCodes of functions in D-relation with clause onto res.
- `FormulaComputeDRel`: Push the FCodes of functions in D-relation with form onto res.

### Dependencies

- `"ccl_f_generality.h"`
- `<ccl_clausesets.h>`
- `<ccl_formulasets.h>`
- `<che_axfilter.h>`

### Compile-Time Conditions

- `CCL_F_GENERALITY`

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

Source files reviewed: `CLAUSES/ccl_f_generality.h`, `CLAUSES/ccl_f_generality.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 847 lines, 18 scanned public declarations, 6 scanned internal function definitions, and 18 structured function-comment blocks.
- Code for computing the generality of function/predicate symbols using a generalize SinE approach, counting occurences in terms, literals, clauses, and formulas. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
