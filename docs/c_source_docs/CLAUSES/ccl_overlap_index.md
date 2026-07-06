<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_overlap_index

## Source Files

- [CLAUSES/ccl_overlap_index.h](../../../eprover/CLAUSES/ccl_overlap_index.h)
- [CLAUSES/ccl_overlap_index.c](../../../eprover/CLAUSES/ccl_overlap_index.c)

## Purpose

A simple (hashed) index from terms to clause position sets (organized as trees of clauses with a tree of positions at which the term occurs. Positions are encoded in a two-level tree itself: Position sets are indexed by clauses.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OverlapIndex_p`

### Macros And Constants

- `CCL_OVERLAP_INDEX`

### Globals

- None found in the source scan.

### Exported Functions

- `long ClauseCollectFromTerms(Clause_p clause, PTree_p *terms)`
- `long ClauseCollectFromTermsPos(Clause_p clause, PStack_p terms)`
- `long ClauseCollectIntoTerms(Clause_p clause, PTree_p *terms)`
- `long ClauseCollectIntoTerms2(Clause_p clause, PTree_p *terms, PTree_p *natoms)`
- `long ClauseCollectIntoTermsPos(Clause_p clause, PStack_p terms)`
- `long ClauseCollectIntoTermsPos2(Clause_p clause, PStack_p terms, PStack_p natoms)`
- `void OverlapIndexDeleteClauseOcc(OverlapIndex_p index, Clause_p clause, Term_p term)`
- `void OverlapIndexDeleteFromClause(OverlapIndex_p index, Clause_p clause)`
- `void OverlapIndexDeleteIntoClause(OverlapIndex_p index, Clause_p clause)`
- `void OverlapIndexDeleteIntoClause2(OverlapIndex_p tindex, OverlapIndex_p naindex, Clause_p clause)`
- `void OverlapIndexDeletePos(OverlapIndex_p index, Clause_p clause, CompactPos pos, Term_p iterm)`
- `void OverlapIndexFPLeafPrint(FILE* out, PStack_p stack, FPTree_p leaf)`
- `void OverlapIndexInsertFromClause(OverlapIndex_p index, Clause_p clause)`
- `void OverlapIndexInsertIntoClause(OverlapIndex_p index, Clause_p clause)`
- `void OverlapIndexInsertIntoClause2(OverlapIndex_p tindex, OverlapIndex_p naindex, Clause_p clause)`
- `void OverlapIndexInsertPos(OverlapIndex_p index, Clause_p clause, CompactPos pos, Term_p iterm)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `term_collect_into_terms`: Collect all potential into-subterms into terms.
- `term_collect_into_terms2`: Collect all potential into-subterms into terms/natoms.
- `eqn_collect_into_terms`: Collect all paramod-into terms in lit into terms.
- `eqn_collect_into_terms2`: Collect all paramod-into terms in lit into terms/natoms.
- `term_collect_into_terms_pos`: Collect all potential into-subterms/pos position onto terms.
- `term_collect_into_terms_pos2`: Collect all potential into-subterms/pos positions of the LHS of a negative non-equational literal onto terms/natoms.
- `eqn_collect_into_terms_pos`: Collect all paramod-into terms with position in lit into terms.
- `eqn_collect_into_terms_pos2`: Collect all paramod-into terms with position in lit into terms/natoms.
- `OverlapIndexInsertPos`: Insert a position with clause|pos = iterm into the index. If iterm is NULL, it will be computed from clause.
- `OverlapIndexDeletePos`: Delete a term->clause/position association from the index.
- `OverlapIndexDeleteClauseOcc`: Delete all associations clause->pos via term from the index. This is an optimization - we usually index and unindex full clauses.
- `ClauseCollectIntoTerms`: Collect all term for paramodulation _into_ into tree. These are non-variable terms in maximal sides of maximal literals. Return number of term positions affected.
- `ClauseCollectIntoTermsPos`: Collect tuples cpos, t on stack, so that c|cpos = t and t is a paramod-into position.
- `ClauseCollectFromTerms`: Collect all "from" terms (i.e. potential left hand sides of the rule of a clause seen as a conditional rewrite rule) into terms.
- `ClauseCollectFromTermsPos`: Collect all t|p tuples such that c|p=t and this is a paramod-from position.
- `OverlapIndexInsertIntoClause`: Insert a clause into an overlap-into index
- `OverlapIndexDeleteIntoClause`: Delete a clause from the overlap-into index.
- `OverlapIndexInsertFromClause`: Insert a clause into an overlap-from index
- `OverlapIndexDeleteFromClause`: Delete a clause from an overlap-from index
- `OverlapIndexClauseTreePrint`: Print an overlapIndex.
- `OverlapIndexSubtermTreePrint`: Print a suberm tree (only for debugging)
- `OverlapIndexFPLeafPrint`: Print a leaf as the path leading to it and the number of direct entries in the subterm.
- `ClauseCollectIntoTerms2`: Collect all term for paramodulation _into_ into two trees. These are non-variable terms in maximal sides of maximal literals. Negative atom-terms go into the second tree, all others into the first. Return number of term positions affected.
- `ClauseCollectIntoTermsPos2`: Collect tuples cpos, t on stack(s), so that c|cpos = t and t is a paramod-into position. Negative non-equational predicate terms go onto natoms, the rest onto terms.
- `OverlapIndexInsertIntoClause2`: Insert a clause into two overlap-into indices.
- `OverlapIndexDeleteIntoClause2`: Delete a clause from the two overlap-into indeces.

### Dependencies

- `"ccl_overlap_index.h"`
- `<ccl_subterm_tree.h>`
- `<cte_fp_index.h>`

### Compile-Time Conditions

- `CCL_OVERLAP_INDEX`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_overlap_index.h`, `CLAUSES/ccl_overlap_index.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1032 lines, 17 scanned public declarations, 0 scanned internal function definitions, and 26 structured function-comment blocks.
- A simple (hashed) index from terms to clause position sets (organized as trees of clauses with a tree of positions at which the term occurs. Positions are encoded in a two-level tree itself: Position sets are indexed by clauses.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Compatibility Notes

- Rust currently ports the fingerprint-index wrapper shape, direct compact-position insert/delete, clause occurrence deletion, into/from term collection, stable clause-identifier keys for cloned clause snapshots in occurrence payloads, and the split normal-term versus negative-atom into-index helpers.
- Into-term collectors count every non-variable occurrence visited but store terms in a pointer-identity set, so the returned count can exceed the number of stored unique terms. Rust preserves that count-versus-storage split.
- Into-position collection skips descent under lambda terms, while the non-position into-term collector recurses through all arguments. Rust mirrors that asymmetry.
- `term_collect_into_terms2` and `term_collect_into_terms_pos2` send only the top negative non-equational atom term to the `natoms` collection; its subterms are collected into the normal term collection. Rust preserves that top-only split.
- `OverlapIndexInsertIntoClause` and `OverlapIndexInsertFromClause` collect positions onto a stack and then pop them for insertion, reversing insertion traversal order. Rust iterates the collected positions in reverse when inserting.
- Indexed paramodulation collects unifiable fingerprint leaves onto a C `PStack` and then pops them before traversing the subterm trees. Rust now reverses the leaf collection before flattening occurrence payloads so generated indexed-paramodulation candidates follow the same stack-pop order.
- Rust exposes an `OverlapIndexFPLeafPrint`-style renderer for fingerprint leaf paths, direct term counts, subterm entries, and compact clause-position payloads using an explicit term bank and problem type.

### Change Later

- `ClauseCollectIntoTerms2` depends on `EqnIsEquLit(lit)` through the literal's owning term bank in C. Rust has no equation back-pointer yet, so the split collectors take an explicit `&TermBank`; replace this with a typed owner handle once clause/literal ownership can provide the bank safely.
- C overlap indexes group occurrence positions by raw clause pointer. Rust cannot use borrowed wrapper addresses for cloned clause values, so it uses stable clause identifiers for current cloned snapshots; replace cloned payloads with stable clause handles before long-lived proof-state indexes depend on deletion order or duplicate identifiers.
- The C debug printers expose pointer addresses and splay-tree shape. Rust now renders the leaf path, term count, term text, and compact-position payload content, but does not recreate byte-identical raw tree-node layout; add that only if diagnostics or reference-output tests require it.
- `OverlapIndexFPLeafPrint` allocates and frees a local `PStack` named `iter` without using it. Rust intentionally omits that no-op allocation; if allocation side effects ever become visible in C debug builds, treat that as compatibility-only behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
