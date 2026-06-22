<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_groundconstr

## Source Files

- [CLAUSES/ccl_groundconstr.h](../../../eprover/CLAUSES/ccl_groundconstr.h)
- [CLAUSES/ccl_groundconstr.c](../../../eprover/CLAUSES/ccl_groundconstr.c)

## Purpose

Computing constraints on the possible instances of groundable clauses. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `LitConstrCell`
- `LitOccTableCell`
- `LitOccTable_p`

### Macros And Constants

- `CCL_GROUNDCONSTR`
- `LIT_OCC_TABLE_ENTRY(table, pred, arity)`
- `LIT_OCC_TABLE_REF(table, pred, arity)`
- `LitOccTableCellAlloc()`
- `LitOccTableCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `(&((table)->matrix[(((table)->sig_size)*(arity)+(pred))])) LitOccTable_p LitOccTableAlloc(Sig_p sig)`
- `PTree_p LitPosGetConstraints(LitOccTable_p table, FunCode pred, int pos)`
- `bool LitPosAddConstraint(LitOccTable_p table, FunCode pred, int pos, Term_p term)`
- `bool LitPosGetConstrState(LitOccTable_p table, FunCode pred, int pos)`
- `long SigCollectConstantTerms(TB_p bank, PStack_p stack, FunCode uniq)`
- `void ClauseCollectVarConstr(LitOccTable_p p_table, LitOccTable_p n_table, Clause_p clause, PTree_p ground_terms, PDArray_p var_constr)`
- `void EqnCollectVarConstr(LitOccTable_p p_table, LitOccTable_p n_table, PDArray_p var_constr, Eqn_p eqn)`
- `void LitOccAddClauseAdd(LitOccTable_p p_table, LitOccTable_p n_table, Clause_p clause)`
- `void LitOccAddClauseSetAlt(LitOccTable_p p_table, LitOccTable_p n_table, ClauseSet_p set)`
- `void LitOccAddLitAdd(LitOccTable_p p_table, LitOccTable_p n_table, Eqn_p eqn)`
- `void LitOccTableFree(LitOccTable_p junk)`
- `void LitPosSetConstrState(LitOccTable_p table, FunCode pred, int pos, bool value)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `LitOccTableAlloc`: Allocate a LitOccTable suitable for the signature. This wastes some memory, but except for pathological cases, this should be insignificant, and the time efficiency of the operations should be good.
- `LitOccTableFree`: Free a LitOccTable.
- `LitPosGetConstrState`: Return true if the position described carries any constraints, false otherwise.
- `LitPosSetConstrState`: Return true if the position described carries any constraints, false otherwise.
- `LitPosGetConstraints`: Return the constraints carried at the position described. This function can only be called on positions that carry constraints!
- `LitPosAddConstraint`: Add the term to the set of disjunctive constraints at the described position. Return true if this makes the position unconstrained.
- `LitOccAddLitAlt`: Add the constraints induced by literal into the corresponding table.
- `LitOccAddClauseAlt`: Add the constraints induced by clause to the constraint tables.
- `LitOccAddClauseSetAlt`: Add the constraints induced by the clause set to the constraint tables.
- `SigCollectConstantTerms`: Push terms corresponding to all constants in sig onto the stack. If sig contains no constant, insert a new skolem constant. If uniq is set, just push the one term corresponding to uniq.
- `EqnCollectVarConstr`: For all variables occuring in eqn, remove the alternatives not compatible with the constraints in the tables.
- `ClauseCollectVarConstr`: Apply all variable constraints for clause to the initialized var_constr array return them.

### Dependencies

- `"ccl_groundconstr.h"`
- `<ccl_clausesets.h>`

### Compile-Time Conditions

- `CCL_GROUNDCONSTR`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_groundconstr.h`, `CLAUSES/ccl_groundconstr.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 552 lines, 15 scanned public declarations, 0 scanned internal function definitions, and 12 structured function-comment blocks.
- Computing constraints on the possible instances of groundable clauses. the GNU Lesser General Public License.
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
