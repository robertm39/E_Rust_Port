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

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
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

- Rust now ports the generated split-literal subset of `GenDefLit`, including arity-zero and split-variable-parameterized predicates, generated predicate typing, `FPClSplitDef`, `EPIsSplitLit`, and term-bank sharing.
- The arity-zero `GetDefinitions(fresh=false)` path is ported for controlled splitting: Rust canonicalizes the definition body, searches the proof-state definition store for variants, reuses the associated split predicate and formula parent when found, and inserts a canonical reusable definition body plus predicate/formula associations when none exists.
- Rust now builds and archives the reusable non-fresh `GetFormulaDefinition` shape (`~def <=> closed(body)`) and records represented clause derivations: new definition clauses get `DCSplitEquiv` formula parents, and residual split clauses get `DCApplyDef` formula parents.
- Fresh and reusable non-fresh arity-zero split definitions now archive represented formula parents in the proof state's dedicated definition formula archive for controlled splitting, and the archived `GetFormulaDefinition` wrappers carry formula-owned `DCIntroDef` derivations. Proof-control split branches now use those archived parents for opt-in split formula introduction, split-equivalence clause, and residual `apply_def` proof-documentation output.

### Change Later

- The current Rust proof state represents `DefStoreCell` as a `ClauseSet`, a dedicated definition `FormulaSet`, and predicate/formula association maps rather than as a single owner that also contains the term-bank pointer. Consolidate this into a fuller `DefStore`-shaped owner once all splitting paths and executable-wide split-definition proof output are represented.
- C `GetDefinitions(fresh=true)` deliberately does not insert reusable variant associations, but it still archives the introduced formula definition. Rust now mirrors this for proof-state arity-zero controlled splitting; keep the absence of reusable associations visible when later consolidating the split-definition owner.
- C expects `def_clauses` to be FV-indexed before reuse lookup. Rust falls back to a linear variant scan when the standalone helper is used before proof-state FV initialization; tighten this only if all real callers can guarantee the C initialization order.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
