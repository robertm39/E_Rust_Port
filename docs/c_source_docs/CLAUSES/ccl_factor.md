<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_factor

## Source Files

- [CLAUSES/ccl_factor.h](../../../eprover/CLAUSES/ccl_factor.h)
- [CLAUSES/ccl_factor.c](../../../eprover/CLAUSES/ccl_factor.c)

## Purpose

Functions for ordered factorisation. the GNU Lesser General Public License. <1> Sun May 31 19:12:41 MET DST 1998 New

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_FACTOR`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p ComputeOrderedFactor(TB_p bank, OCB_p ocb, ClausePos_p pos1, ClausePos_p pos2, VarBank_p freshvars)`
- `Eqn_p ClausePosFirstEqualityFactorSides(Clause_p clause, ClausePos_p pos1, ClausePos_p pos2)`
- `Eqn_p ClausePosFirstOrderedFactorLiterals(Clause_p clause, ClausePos_p pos1, ClausePos_p pos2)`
- `Eqn_p ClausePosNextEqualityFactorSides(ClausePos_p pos1, ClausePos_p pos2)`
- `Eqn_p ClausePosNextOrderedFactorLiterals(ClausePos_p pos1, ClausePos_p pos2)`
- `void ComputeEqualityFactor(TB_p bank, OCB_p ocb, ClausePos_p pos1, ClausePos_p pos2, VarBank_p freshvars, bool* is_ho, PStack_p res)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `find_next_potential_eq_factor_partner`: Given two positions, set pos2->literal to the next positive literal (at or including pos2->literal) distinct from pos1->literal.
- `find_first_eq_factor_partner`: Given the maximal positive literal described in pos1, set pos2 to the first potential partner for an equality factoring inference. Return the selected literal, or NULL if no exists.
- `ComputeOrderedFactor`: Given two positions in a clause, try to compute the ordered factor. Return it, if it exists, otherwise return NULL.
- `ClausePosFirstOrderedFactorLiterals`: Given a clause, compute the first pair of literals were an ordered factor might be computed. See ClausePosNextFactorLiterals(). This works by setting an impossible initial state and searching for the next valid one...
- `ClausePosNextOrderedFactorLiterals`: Given a clause and two positions, set these position to the next valid combination for an ordered factor inference. Return the second literal, or NULL if no position pair exists. pos2->side is used to indicate wether the unification should take place as is or with one equation swapped.
- `ComputeEqualityFactor`: Given two positions in a clause, try to compute the equality factor. Return it, if it exists, otherwise return NULL.
- `ClausePosFirstEqualityFactorSides`: Given a clause and two uninialized positions, set the positions to the first potiental pair of sides for an equality factoring inference. Return the second literal, or NULL if no legal pair exists.
- `ClausePosNextEqualityFactorSides`: Given a pair of positions pos1, pos2, compute the next potential positions for a equality factoring inference.

### Dependencies

- `"ccl_factor.h"`
- `<ccl_clausesets.h>`
- `<cte_ho_csu.h>`

### Compile-Time Conditions

- `CCL_FACTOR`

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

Source files reviewed: `CLAUSES/ccl_factor.h`, `CLAUSES/ccl_factor.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 477 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 8 structured function-comment blocks.
- Functions for ordered factorisation. the GNU Lesser General Public License. <1> Sun May 31 19:12:41 MET DST 1998 New
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
