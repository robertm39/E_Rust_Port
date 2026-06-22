<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_subsumption

## Source Files

- [CLAUSES/ccl_subsumption.h](../../../eprover/CLAUSES/ccl_subsumption.h)
- [CLAUSES/ccl_subsumption.c](../../../eprover/CLAUSES/ccl_subsumption.c)

## Purpose

Functions for subsumption testing -> test a clause against a (unit) clauseset, test a clause set against a (unit) clause. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_SUBSUPTION`

### Globals

- `extern bool StrongUnitForwardSubsumption`
- `extern long ClauseClauseSubsumptionCalls`
- `extern long ClauseClauseSubsumptionCallsRec`
- `extern long ClauseClauseSubsumptionSuccesses`
- `extern long UnitClauseClauseSubsumptionCalls`

### Exported Functions

- `Clause_p ClauseSetFindFVVariantClause(ClauseSet_p set, FVPackedClause_p clause)`
- `Clause_p ClauseSetFindFirstFVSubsumedClause(ClauseSet_p set, FVPackedClause_p subsumer)`
- `Clause_p ClauseSetFindFirstSubsumedClause(ClauseSet_p set, Clause_p subsumer)`
- `Clause_p ClauseSetFindSubsumedClause(ClauseSet_p set, Clause_p set_position, Clause_p subsumer)`
- `Clause_p ClauseSetFindUnitSubsumedClause(ClauseSet_p set, Clause_p set_position, Clause_p subsumer)`
- `Clause_p ClauseSetFindVariantClause(ClauseSet_p set, Clause_p clause)`
- `Clause_p ClauseSetSubsumesClause(ClauseSet_p set, Clause_p sub_candidate)`
- `Clause_p ClauseSetSubsumesFVPackedClause(ClauseSet_p set, FVPackedClause_p sub_candidate)`
- `Clause_p UnitClauseSetSubsumesClause(ClauseSet_p set, Clause_p clause)`
- `PERF_CTR_DECL(SetSubsumeTimer)`
- `PERF_CTR_DECL(SubsumeTimer)`
- `bool ClauseNegativeSimplifyReflect(ClauseSet_p set, Clause_p clause)`
- `bool ClausePositiveSimplifyReflect(ClauseSet_p set, Clause_p clause)`
- `bool ClauseSubsumesClause(Clause_p subsumer, Clause_p sub_candidate)`
- `bool LiteralSubsumesClause(Eqn_p literal, Clause_p clause)`
- `bool UnitClauseSubsumesClause(Clause_p unit, Clause_p clause)`
- `long ClauseSetFindFVSubsumedClauses(ClauseSet_p set, FVPackedClause_p subsumer, PStack_p res)`
- `long ClauseSetFindSubsumedClauses(ClauseSet_p set, Clause_p subsumer, PStack_p res)`

## Implementation Notes

### Internal Functions

- `check_subsumption_possibility`
- `clause_set_subsumes_clause`
- `clause_set_subsumes_clause_indexed`
- `clause_subsumes_clause`
- `clause_tree_find_first_subsumed_clause`
- `clause_tree_find_subsumed_clauses`
- `clause_tree_find_subsuming_clause`
- `clause_tree_find_variant_clause`
- `clauseset_find_first_subsumed_clause`
- `clauseset_find_first_subsumed_clause_indexed`
- `clauseset_find_subsumed_clauses`
- `clauseset_find_subsumed_clauses_indexed`
- `clauseset_find_variant_clause_indexed`
- `eqn_list_rec_subsume`
- `eqn_subsumes_termpair`
- `eqn_topsubsumes_termpair`
- `find_spec_literal`
- `unit_clause_set_strongsubsumes_termpair`
- `unit_clause_set_subsumes_clause`

### Source-Level Behavior

- `unit_clause_set_strongsubsumes_termpair`: Return a unit clause with sign positive from set if there is a subset with sign positive that shows t1=t2 in one step. Return NULL otherwise.
- `unit_clause_set_subsumes_clause`: Return a clause from set that subsumes clause.
- `eqn_topsubsumes_termpair`: Return true if eqn subsumes t1=t2 at top level.
- `eqn_subsumes_termpair`: Return true if the equation subsumes t1=t2.
- `find_spec_literal`: Find a literal in list that is more special than lit. Return it or NULL if none exists.
- `check_subsumption_possibility`: Return true if each literal in subsumer is more general than a literal in sub_candidate.
- `eqn_list_rec_subsume`: Try to find a subset of sub_cand_list such that subst(subsum_list) = subset. Return true if this is possible, false otherwise.
- `clause_subsumes_clause`: Return true if subsumer subsumes sub_candidate. Assumes that weights are precomputed.
- `clause_set_subsumes_clause`: Return subsuming clause if the set subsumes sub_candidate, NULL otherwise. All clauses need correct weights!
- `clause_tree_find_subsuming_clause`: Given a PTree of clauses and a clause, return a subsuming clause or NULL
- `clause_set_subsumes_clause_indexed`: Return clause if the indexed set subsumes sub_candidate. All clauses need correct weights!
- `clause_tree_find_subsumed_clauses`: Given a PTree of clauses and a clause, push all subsumed clauses onto res.
- `clause_tree_find_first_subsumed_clause`: Given a PTree of clauses and a clause, return the first clause in the tree subsumed by the clause, or NULL.
- `clauseset_find_subsumed_clauses`: ; Find all clauses subsumed by subsumer and push them onto stack. Also write PCL statements to that effect (if required by output level).
- `clauseset_find_first_subsumed_clause`: ; Find first subsumed clause in set and return it (or NULL, if no such clause exists).
- `clauseset_find_subsumed_clauses_indexed`: Find all clauses subsumed by vec->clause in index and push them onto res.
- `clauseset_find_first_subsumed_clause_indexed`: Find and return the first clause in the indexed set that is subsumed by vec.
- `clause_tree_find_variant_clause`: Given a PTree of clauses and a clause, return a variant clause or NULL
- `clauseset_find_variant_clause_indexed`: Find and return a variant of the clause represented by vec in set (if any such clause exists), return NULL otherwise.
- `LiteralSubsumesClause`: Return true if literal subsumes one of the literals in clause (otherwise return false).
- `UnitClauseSubsumesClause`: Return true if unit subsumes clause.
- `UnitClauseSetSubsumesClause`: If a clause in set subsumes clause, return a pointer to it. Otherwise return NULL.
- `ClauseSetFindUnitSubsumedClause`: Return a pointer to the first clause in the list at or after set_position that is subsumed by the unit-clause subsumer. Return NULL, if no such clause exists.
- `ClausePositiveSimplifyReflect`: Remove all negative literals subsumed by the positive unit clauses in set from clause. Return true if clause is empty, false otherwise. Set has to be indexed and should contain only positive units!
- `ClauseNegativeSimplifyReflect`: Remove all positive literals subsumed by negative unit clauses in set from clause. Return true if clause is empty, false otherwise. Set has to be indexed and contain negative units only.
- `ClauseSubsumesClause`: Return true if subsumer subsumes sub_candidate. Requires that both clauses have correct weight information.
- `ClauseSetSubsumesFVPackedClause`: Return true if the set subsumes sub_candidate->clause. All clauses need correct weights!
- `ClauseSetSubsumesClause`: Return true if the set subsumes sub_candidate. All clauses need correct weights!
- `ClauseSetFindSubsumedClause`: Return a pointer to the first clause in the list at or after set_position that is subsumed by the (non-unit)clause subsumer. Return NULL, if no such clause exists.
- `ClauseSetFindFVSubsumedClauses`: Find all clauses in set that are subsumed by subsumer, and push them onto stack. Return number of clauses found.
- `ClauseSetFindFirstFVSubsumedClause`: Find and return first clause in set that is subsumed by subsumer (or NULL).
- `ClauseSetFindSubsumedClauses`: Find all clauses in set that are subsumed by subsumer, and push them onto stack. Return number of clauses found.
- `ClauseSetFindFirstSubsumedClause`: Find and return first clause in set subsumed by subsumer.
- `ClauseSetFindFVVariantClause`: Given a (FVPacked) clause and a clause set, find variant of the clause and return it if successful. Otherwise return NULL.
- `ClauseSetFindVariantClause`: Find and return a variant of clause in set (or 0 if none exists).

### Dependencies

- `"ccl_subsumption.h"`
- `<ccl_unit_simplify.h>`
- `<clb_os_wrapper.h>`

### Compile-Time Conditions

- `CCL_SUBSUPTION`
- `ENABLE_LFHO`
- `NDEBUG`
- `NEVER_DEFINED`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_subsumption.h`, `CLAUSES/ccl_subsumption.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1803 lines, 25 scanned public declarations, 19 scanned internal function definitions, and 35 structured function-comment blocks.
- Subsumption is performance-sensitive and depends on variable matching/indexing details; preserve pruning semantics exactly.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
