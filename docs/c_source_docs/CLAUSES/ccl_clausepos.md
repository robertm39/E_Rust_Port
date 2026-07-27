<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_clausepos

## Source Files

- [CLAUSES/ccl_clausepos.h](../../../eprover/CLAUSES/ccl_clausepos.h)
- [CLAUSES/ccl_clausepos.c](../../../eprover/CLAUSES/ccl_clausepos.c)

## Purpose

Positions of subterms in clauses (and in equations). the GNU Lesser General Public License. <1> Wed May 20 03:34:54 MET DST 1998 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ClausePosCell`
- `ClausePos_p`
- `Deleter`

### Macros And Constants

- `CLAUSEPOS`
- `CLAUSEPOSCELL_MEM`
- `ClausePosCellAlloc()`
- `ClausePosCellFree(junk)`
- `ClausePosFree(junk)`
- `ClausePosIsTop(position)`

### Globals

- None found in the source scan.

### Exported Functions

- `Eqn_p ClausePosFindMaxLiteral(ClausePos_p pos, bool positive)`
- `Eqn_p ClausePosFindPosLiteral(ClausePos_p pos, bool maximal)`
- `Term_p ClausePosFindFirstMaximalSide(ClausePos_p pos, bool positive)`
- `Term_p ClausePosFindFirstMaximalSubterm(ClausePos_p pos)`
- `Term_p ClausePosFindNextMaximalSide(ClausePos_p pos, bool positive)`
- `Term_p ClausePosFindNextMaximalSubterm(ClausePos_p pos)`
- `bool TermComputeRWSequence(PStack_p stack, Term_p from, Term_p to, int inject_op)`
- `static inline ClausePos_p ClausePosAlloc(void)`
- `static inline Term_p ClausePosGetOtherSide(ClausePos_p pos)`
- `static inline Term_p ClausePosGetSide(ClausePos_p pos)`
- `static inline Term_p ClausePosGetSubterm(ClausePos_p pos)`
- `static inline void ClausePosCellFreeWDeleter(ClausePos_p junk, Deleter del)`
- `static inline void ClausePosFreeWDeleter(ClausePos_p junk, Deleter deleter)`
- `void ClausePosPrint(FILE* out, ClausePos_p pos)`

## Implementation Notes

### Internal Functions

- `ClausePosAlloc`
- `ClausePosCellFreeWDeleter`
- `ClausePosFreeWDeleter`
- `ClausePosGetOtherSide`
- `ClausePosGetSide`
- `ClausePosGetSubterm`

### Source-Level Behavior

- `ClausePosAlloc`: Allocate an empty, semi-initialized ClausePosCell.
- `ClausePosCellFreeWDeleter`: Free a clause pos cell and use deleter on junk->data
- `ClausePosFree`: Free a clausepos.
- `ClausePosGetSide`: Given a clause position, return the designated side of the literal.
- `ClausePosGetOtherSide`: Given a clause position, return the _not_ designated side of the literal - don't ask, this has its use!
- `ClausePosGetSubterm`: Given a clause position, return the designated subterm of the literal.
- `ClausePosPrint`: Print a clause position.
- `ClausePosFindPosLiteral`: Find the first positive literal (if maximal = true, find the first positive and maximal literal).
- `ClausePosFindMaxLiteral`: Find the first maximal literal in the list at pos->literal. If positive, find positive literals only.
- `ClausePosFindFirstMaximalSide`: Find the first maximal side in the list at pos->literal, if positive is set, use positive equations only.
- `ClausePosFindNextMaximalSide`: Given a position, find the next maximal side in the eqnlist at pos->literal. If positive is set, use positive equations only.
- `ClausePosFindFirstMaximalSubterm`: Given a clause, find the first subterm in a maximal side in a maximal literal.
- `ClausePosFindNextMaximalSubterm`: Given a position in a clause, find the next maximal subterm in it.
- `TermComputeRWSequence`: Given two terms from and two, connected by a rewrite chain, push a sequence of clause idents onto the stack such that they represent a rewrite chain transforming to into from. Returns true if the chain has length 0.

### Dependencies

- `"ccl_clausepos.h"`
- `<ccl_clauses.h>`
- `<cte_termpos.h>`

### Compile-Time Conditions

- `CLAUSEPOS`
- `CONSTANT_MEM_ESTIMATE`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_clausepos.h`, `CLAUSES/ccl_clausepos.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 575 lines, 17 scanned public declarations, 6 scanned internal function definitions, and 14 structured function-comment blocks.
- Positions of subterms in clauses (and in equations). the GNU Lesser General Public License. <1> Wed May 20 03:34:54 MET DST 1998 New
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
