<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_propclauses

## Source Files

- [CLAUSES/ccl_propclauses.h](../../../eprover/CLAUSES/ccl_propclauses.h)
- [CLAUSES/ccl_propclauses.c](../../../eprover/CLAUSES/ccl_propclauses.c)

## Purpose

Definitions for propositional clauses (for eground) which can be stored much more compactly than ordinary clauses - at the price of less functionality and flexibility. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PropClauseCell`
- `PropClauseSetCell`
- `PropClauseSet_p`
- `PropClause_p`
- `PropLitCell`
- `PropLit_p`

### Macros And Constants

- `CCL_PROPCLAUSES`
- `PropClauseCellAlloc()`
- `PropClauseCellFree(junk)`
- `PropClauseSetCellAlloc()`
- `PropClauseSetCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p PropClauseToClause(TB_p bank, PropClause_p clause)`
- `PropClauseSet_p PropClauseSetAlloc(void)`
- `PropClause_p PropClauseAlloc(Clause_p clause)`
- `long PropClauseMaxVar(PropClause_p clause)`
- `long PropClauseSetInsertClause(PropClauseSet_p set, Clause_p clause)`
- `long PropClauseSetInsertPropClause(PropClauseSet_p set, PropClause_p clause)`
- `long PropClauseSetMaxVar(PropClauseSet_p set)`
- `void PropClauseFree(PropClause_p clause)`
- `void PropClausePrint(FILE* out, TB_p bank, PropClause_p clause)`
- `void PropClauseSetFree(PropClauseSet_p set)`
- `void PropClauseSetPrint(FILE* out, TB_p bank, PropClauseSet_p set)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PropClauseAlloc`: Allocate a propositional clause representing the same clause as the normal one. Does some sanity checking, but only in assertions.
- `PropClauseFree`: Free the memory taken up by a correctly build propositional clause. Does not touch the terms/atoms!
- `PropClauseToClause`: Generate a conventional clause from a propositional clause.
- `PropClausePrint`: Print a propositional clause (by temporarily converting it to a normal one (which will have an unpredictable identifier).
- `PropClauseMaxVar`: Return the largest variable index in clause.
- `PropClauseSetAlloc`: Allocate an empty propositional clause set.
- `PropClauseSetFree`: Free a PropClauseSet and all its clauses.
- `PropClauseSetInsertPropClause`: Insert a propositional clause into the set. Return new number of elements.
- `PropClauseSetInsertClause`: Insert the (normal) clause into set as a propositional clause.
- `PropClauseSetPrint`: Print a propositional clause set.
- `PropClauseSetMaxVar`: Return the largest used variable number in set.

### Dependencies

- `"ccl_propclauses.h"`
- `<ccl_clauses.h>`

### Compile-Time Conditions

- `CCL_PROPCLAUSES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_propclauses.h`, `CLAUSES/ccl_propclauses.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 473 lines, 17 scanned public declarations, 0 scanned internal function definitions, and 11 structured function-comment blocks.
- Definitions for propositional clauses (for eground) which can be stored much more compactly than ordinary clauses - at the price of less functionality and flexibility. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- `PropClausePrint` temporarily rebuilds an ordinary `Clause` and then calls global `ClausePrint`; the temporary clause has an unpredictable identifier. Rust now exposes explicit `ClausePrint`-style LOP/TPTP/TSTP rendering for the rebuilt clause and should keep identifier-sensitive global formats at the outer output boundary.
- `PropClausePrint` is documented with no global variables, but `ClausePrint` observes the process-global `OutputFormat`. Rust passes the output format explicitly; retain that boundary unless executable reference tests require a hidden global renderer.
- `PropClauseSetPrint` adds the newline after each `PropClausePrint` call, not inside the single-clause printer. Rust preserves this split in the LOP string helpers.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
