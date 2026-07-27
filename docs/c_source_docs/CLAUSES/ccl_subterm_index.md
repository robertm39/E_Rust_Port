<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_subterm_index

## Source Files

- [CLAUSES/ccl_subterm_index.h](../../../eprover/CLAUSES/ccl_subterm_index.h)
- [CLAUSES/ccl_subterm_index.c](../../../eprover/CLAUSES/ccl_subterm_index.c)

## Purpose

A simple (hashed) index from terms to clauses in which this term appears as priviledged (rewriting restricted) or unpriviledged term. the GNU Lesser General Public License. <1> Wed May 5 10:19:14 CEST 2010

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `SubtermIndex_p`

### Macros And Constants

- `CCL_SUBTERM_INDEX`

### Globals

- None found in the source scan.

### Exported Functions

- `bool SubtermIndexDeleteOcc(SubtermIndex_p index, Clause_p clause, Term_p term, bool restricted)`
- `bool SubtermIndexInsertOcc(SubtermIndex_p index, Clause_p clause, Term_p term, bool restricted)`
- `long ClauseCollectIdxSubterms(Clause_p clause, PTree_p *rest, PTree_p *full, bool lambda_demod)`
- `void SubtermIndexDeleteClause(SubtermIndex_p index, Clause_p clause, bool lambda_demod)`
- `void SubtermIndexInsertClause(SubtermIndex_p index, Clause_p clause, bool lambda_demod)`

## Implementation Notes

### Internal Functions

- `eqn_collect_idx_subterms`
- `subterm_index_delete_set`
- `subterm_index_insert_set`
- `term_collect_idx_subterms`

### Source-Level Behavior

- `term_collect_idx_subterms`: Collect all non-variable subterms in term either into rest or full (rest for "restricted rewriting" terms, full for the "full rewriting" terms).
- `eqn_collect_idx_subterms`: Collect all non-variable subterms in eqn either into rest or full (rest for "restricted rewriting" terms, full for the "full rewriting" terms).
- `subterm_index_insert_set`: Insert all the subterm/clause relationships in set (represented as a PTree) into the index.
- `subterm_index_delete_set`: Delete all the subterm/clause relationships in set (represented as a PTree) into the index.
- `SubtermIndexInsertOcc`: Insert a given occurance of a subterm into the index. Return true if it was new, false if it already existed.
- `SubtermIndexDeleteOcc`: Delete a given occurance of a subterm from the index. Return true if the clause existed, false otherwise.
- `ClauseCollectIdxSubterms`: Collect all non-variable subterms in clause either into rest or full (rest for "restricted rewriting" terms, full for the "full rewriting" terms).
- `SubtermIndexInsertClause`: Insert a clause into the subterm index.
- `SubtermIndexDeleteClause`: Delete a clause from the subterm index.

### Dependencies

- `"ccl_subterm_index.h"`
- `<ccl_subterm_tree.h>`
- `<cte_fp_index.h>`

### Compile-Time Conditions

- `CCL_SUBTERM_INDEX`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_subterm_index.h`, `CLAUSES/ccl_subterm_index.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 398 lines, 6 scanned public declarations, 4 scanned internal function definitions, and 9 structured function-comment blocks.
- A simple (hashed) index from terms to clauses in which this term appears as priviledged (rewriting restricted) or unpriviledged term. the GNU Lesser General Public License. <1> Wed May 5 10:19:14 CEST 2010
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Compatibility Notes

- Rust now preserves the C `FPIndexFindMatchable` candidate-stack consumption order for backward-rewrite occurrence queries: fingerprint leaves are collected in traversal order but flattened after reversing that leaf list, matching callers such as `find_rewritable_clauses_indexed()` that pop the C `PStack`. A symmetric unifiable occurrence wrapper uses the same stack-pop order for future direct `SubtermIndex_p`/`FPIndex_p` call sites.
<!-- END MANUAL REVIEW: c_source_docs -->
