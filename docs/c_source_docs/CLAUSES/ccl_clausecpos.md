<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_clausecpos

## Source Files

- [CLAUSES/ccl_clausecpos.h](../../../eprover/CLAUSES/ccl_clausecpos.h)
- [CLAUSES/ccl_clausecpos.c](../../../eprover/CLAUSES/ccl_clausecpos.c)

## Purpose

Positions of subterms in clauses (and in equations) using compact (i.e. single integer) positions. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz, Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `CompactPos`

### Macros And Constants

- `CCL_CLAUSECPOS`

### Globals

- None found in the source scan.

### Exported Functions

- `ClausePos_p UnpackClausePos(CompactPos cpos, Clause_p clause)`
- `CompactPos PackClausePos(ClausePos_p pos)`
- `CompactPos PackTermPos(TermPos_p pos)`
- `Eqn_p ClauseCPosFirstLit(Clause_p clause, CompactPos *cpos)`
- `Eqn_p ClauseCPosNextLit(Eqn_p lit, CompactPos *cpos)`
- `Eqn_p ClauseCPosSplit(Clause_p clause, CompactPos *cpos)`
- `Term_p ClauseCPosGetSubterm(Clause_p clause, CompactPos cpos)`
- `void UnpackClausePosInto(CompactPos cpos, Clause_p clause, ClausePos_p pos)`
- `void UnpackTermPos(TermPos_p pos, Term_p t, CompactPos cpos)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PackTermPos`: Pack a term position.
- `PackPos`: Convert a full position into an integer-encoded position.
- `UnpackTermPos`: Given a compact term position in t, encode it into the given full postion.
- `UnpackClausePosInto`: Unpack the compact position cpos in clause into the existing ClausePos pos.
- `UnpackClausePos`: Unpack a clause compact position, returning a newly allocated clause position.
- `ClauseCPosGetSubterm`: Given a clause and a compact position, return the indicated term. This is a very simple but obviously correct version.
- `ClauseCPosFirstLit`: Return the first literal of a clause and the correcponding compact position. Returns NULL/0 for the empty clause.
- `ClauseCPosNextLit`: Given clause literal lit at compact position *cpos, return the next literal and update *cpos to the corresponding compact position. Return NULL/0 if there is no empty clause.
- `ClauseCPosSplit`: Given a clause and a compact position *cpos, determine the literal of the position, and return it. Also update *cpos to denote the position relative to that literal.

### Dependencies

- `"ccl_clausecpos.h"`
- `<ccl_clausepos.h>`

### Compile-Time Conditions

- `CCL_CLAUSECPOS`

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

Source files reviewed: `CLAUSES/ccl_clausecpos.h`, `CLAUSES/ccl_clausecpos.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 398 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 9 structured function-comment blocks.
- Positions of subterms in clauses (and in equations) using compact (i.e. single integer) positions. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
