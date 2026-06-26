<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_def_handling

## Source Files

- [CLAUSES/ccl_def_handling.h](../../../eprover/CLAUSES/ccl_def_handling.h)
- [CLAUSES/ccl_def_handling.c](../../../eprover/CLAUSES/ccl_def_handling.c)

## Purpose

Datatypes for handling clausal definitions as used (up to now implicitly) in splitting, i.e. data structures associating a clause with a fresh constant predicate symbol or literal. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DefStoreCell`
- `DefStore_p`

### Macros And Constants

- `CCL_DEF_HANDLING`
- `DefStoreCellAlloc()`
- `DefStoreCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p GetClauseDefinition(Eqn_p litlist, FunCode def_pred, WFormula_p parent)`
- `DefStore_p DefStoreAlloc(TB_p terms)`
- `Eqn_p GenDefLit(TB_p bank, FunCode pred, bool positive, PStack_p split_vars)`
- `FunCode GetDefinitions(DefStore_p store, Eqn_p litlist, WFormula_p* res_form, Clause_p* res_clause, bool fresh)`
- `WFormula_p GetFormulaDefinition(Eqn_p litlist, FunCode def_pred)`
- `void DefStoreFree(DefStore_p junk)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `DefStoreAlloc`: Return an initialized definitions storage object. Note that the FVIndex in def_clauses still has to be set (this is an inherited uglyness I'll fix soon).
- `DefStoreFree`: Free a definition storage object and all data it is responsible for (includes the FVIndex of def_clauses, but not the term bank).
- `GenDefLit`: Generate a definition literal with terms from bank.
- `GetClauseDefinition`: Given a literal list and the definition predicate, generate one of the two clauses the equivalence definition splits into (namely the one we need to add for splitting). This recycles the literal list!
- `GetFormulaDefinition`: Given a literal list and the definition predicate, generate the equivalent defintion. This one leaves the literal list alone!
- `GetDefinitions`: Given a literal list, provide (optionally) the full definition and the clause equivalent to the non-applied direction of the definition. Return defined predicate. If fresh is true, always return a fresh definition and do not insert the clause/predicate association into the store. If fresh is false, check if it is a variant of a known definiton and return th...

### Dependencies

- `"ccl_def_handling.h"`
- `<ccl_formulafunc.h>`
- `<ccl_subsumption.h>`

### Compile-Time Conditions

- `CCL_DEF_HANDLING`

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

Source files reviewed: `CLAUSES/ccl_def_handling.h`, `CLAUSES/ccl_def_handling.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 397 lines, 8 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Datatypes for handling clausal definitions as used (up to now implicitly) in splitting, i.e. data structures associating a clause with a fresh constant predicate symbol or literal. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- Rust now ports the generated split-literal subset of `GenDefLit` for fresh split definitions, including arity-zero and split-variable-parameterized predicates, generated predicate typing, `FPClSplitDef`, `EPIsSplitLit`, and term-bank sharing.
- `DefStore`, `GetFormulaDefinition`, `GetClauseDefinition`, `GetDefinitions` variant lookup/reuse, `def_archive`, and the formula/derivation output side effects remain pending.

### Change-Later Observations

- The current Rust proof state represents `definition_store` as a `ClauseSet`, not the full C `DefStoreCell` with term-bank pointer, definition clause variants, numeric associations, and formula archive. Keep definition reuse disabled until that owner is represented.
- C `GetDefinitions(fresh=true)` deliberately does not insert reusable variant associations, but it still archives the introduced formula definition. Rust fresh splitting skips the formula archive for now; add it when formula sets and split-definition proof output are ported.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
