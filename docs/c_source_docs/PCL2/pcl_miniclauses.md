<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_miniclauses

## Source Files

- [PCL2/pcl_miniclauses.h](../../../eprover/PCL2/pcl_miniclauses.h)
- [PCL2/pcl_miniclauses.c](../../../eprover/PCL2/pcl_miniclauses.c)

## Purpose

Maximal compact representation for clauses, to be used in compact pcl listings (and possibly wherever elese needed). Adaptded from can_clausestore.h. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `MiniClauseCell`
- `MiniClause_p`

### Macros And Constants

- `MiniClauseCellAlloc()`
- `MiniClauseCellFree(junk)`
- `PCL_MINICLAUSES`

### Globals

- None found in the source scan.

### Exported Functions

- `(MiniClauseCell*)SizeMalloc(sizeof(MiniClauseCell)) SizeFree(junk, sizeof(MiniClauseCell)) void MiniClauseFree(MiniClause_p clause)`
- `Clause_p MiniClauseToClause(MiniClause_p clause, TB_p bank)`
- `Clause_p UnMinifyClause(MiniClause_p clause, TB_p bank)`
- `MiniClause_p ClauseToMiniClause(Clause_p clause)`
- `MiniClause_p MinifyClause(Clause_p clause)`
- `void MiniClausePCLPrint(FILE* out, MiniClause_p compact, TB_p bank)`
- `void MiniClausePrint(FILE* out, MiniClause_p compact, TB_p bank, bool full_terms)`
- `void MiniClauseTSTPCorePrint(FILE* out, MiniClause_p compact, TB_p bank)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `MiniClauseFree`: Release the memory taken by a compact clause.
- `MiniClauseAddTerms`: Add the terms in term_clause to clause.
- `ClauseToMiniClause`: Generate a compact clause represention from the normal one.
- `MiniClauseToClause`: Given a compact clause, create the corresponding normal clause.
- `MinifyClause`: As ClauseToMiniClause(), but destroy original.
- `UnMinifyClause`: As MiniClauseToClause(), but destroy original.
- `MiniClausePrint`: Print a compact clause. Not the best way, but the easiest!
- `MiniClausePCLPrint`: Print the clause in PCL format, i.e. as a literal list.
- `MiniClauseTSTPCorePrint`: Print the core clause in TSTP format, i.e. as a literal list.

### Dependencies

- `"pcl_miniclauses.h"`
- `<ccl_clauses.h>`

### Compile-Time Conditions

- `PCL_MINICLAUSES`

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

Source files reviewed: `PCL2/pcl_miniclauses.h`, `PCL2/pcl_miniclauses.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 378 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 9 structured function-comment blocks.
- Maximal compact representation for clauses, to be used in compact pcl listings (and possibly wherever elese needed). Adaptded from can_clausestore.h. the GNU Lesser General Public License.
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
