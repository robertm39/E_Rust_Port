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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for ClauseSet-owned FV-index integration on 2026-07-17.

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

### Rust Port Status Notes

- `src/clauses/subsumption.rs` ports direct subsumption helpers, unit subsumption helpers, positive and negative simplify-reflect over plain or demodulator-indexed unit sets, optional strong positive unit simplify-reflect, plain and FV-indexed subsumed-clause discovery, variant lookup, and the process-global subsumption counters now read by executable statistics. `UnitClauseSetSubsumesClause` now follows C through the unit demodulator index rather than a linear set scan, so both indexed sides of an unorientable equality remain candidates. Recursive whole-clause matching preserves C's direct-then-swapped orientation choice point across failures in later literals. Mutable-bank variants route proof-control contraction, contextual simplify-reflect, condensation, split-definition variants, and CSSCPA through complete higher-order matching while retaining the unbanked first-order compatibility APIs.
- `ClauseSet` now owns its optional FV anchor and maintains it through indexed insertion and extraction. Set-owned lookup wrappers select indexed or plain search from that anchor, and production proof-control, contextual simplify-reflect, watchlist, and split-definition variant callers no longer pass the set's anchor back as a redundant independent argument. Explicit-anchor functions remain lower-level test and interop surfaces.
- Contextual simplify-reflect reaches the same feature-vector subsumer and subsumed-clause queries as C whenever the clause set owns an FV anchor. The indexed subsumer wrapper deliberately accepts unit queries produced after an earlier contextual deletion; only the no-index linear fallback retains C's non-unit assertion.
- Positive and negative simplify-reflect record `DCSR` derivation entries with compact references to the simplifying unit clause when a literal is removed. Opt-in documenting helpers emit represented `DocClauseModificationDefault(..., inf_simplify_reflect, ...)` output before pushing the matching `DCSR` entry. This preserves the currently represented documentation side effects without importing C's unsafe raw-parent-pointer lifetime into Rust.
- Proof-object reconstruction from stable parent handles remains separate future work. Rust stores C's process-global `StrongUnitForwardSubsumption` as proof-control session configuration for forward subsumption and positive simplify-reflect callers.
- Release HEN011 profiling confirmed that recursive clause subsumption is reached more than 100 million times on a full proof. Rust now maps C's assertion-only ordering/weight preconditions to debug assertions and reuses the picked-candidate bitmap through reentrant thread-local scratch while keeping one fresh substitution per top-level call; this preserves direct LUSK6ext proof order better than reusing both allocations.

### Change Later

- C's simplify-reflect functions receive indexed unit-clause sets, emit proof documentation from global output/id state, and push the raw simplifying `Clause_p` in `DCSR`. Rust routes lookup through the set-owned represented indexes and uses compact clause references plus explicit proof-doc sessions; retain this safe boundary until proof-object reconstruction has stable clause handles and genuinely depends on parent identity.
- Positive simplify-reflect's strong mode is controlled by the process-global `StrongUnitForwardSubsumption` in C. Rust now exposes the lower-level helper parameter while routing configured proof search through a `ProofControl` session flag; revisit only if later strategy scheduling needs C-global sharing semantics.
- The simplify-reflect helpers mutate the target clause while iterating literal links in C. Rust uses index-based removal over owned literal vectors; keep tests around repeated removals and empty-clause return behavior before refactoring the loop.
- `eqn_topsubsumes_termpair` tries the swapped equation direction only when matching the left pattern against the first target fails. If that first match succeeds but the corresponding right match fails, C returns false even when the swapped pair would match. Rust preserves this branch guard; a cleaned commutative top-subsumption API should either try both complete directions or name the asymmetric behavior explicitly after compatibility traces are fixed.
- `UnitClauseSetSubsumesClause` delegates to the indexed simplifying-unit lookup. Because an unorientable equality is indexed under both sides, this can find an opposite-side candidate that a linear scan plus the asymmetric `eqn_topsubsumes_termpair` retry misses. Rust preserves the combination because it changes forward contraction and clause selection; after compatibility is secured, a cleaned API should make commutative direction handling independent of index representation.
- C subsumption APIs recover the mutable owner bank indirectly through terms when `SubstMatchComplete` enters higher-order mode. This keeps signatures deceptively read-only even though eta normalization and applied-variable binding construction can allocate shared terms; prefer an explicit mutable proof-session or bank parameter in a cleaned API.
- `eqn_list_rec_subsume` embeds each unoriented literal's direct/swapped alternatives inside recursive whole-clause search. The backtracking is semantically required, but the literal API does not expose an iterator or continuation over successful orientations, which made a locally successful direction easy to commit too early in the Rust port. After compatibility is secured, make orientation alternatives explicit while preserving substitution rollback and candidate-reservation semantics.
- `ClauseSetSubsumesClause` has an index-dependent precondition: the plain scan asserts a non-unit target, while the FV-index branch accepts unit targets. Contextual simplify-reflect reaches both shapes during one mutation loop. Rust preserves the split and tests it, but a cleaned API should state one contract independent of storage representation.
- FV-index leaves in C are pointer-keyed splay trees, so first-hit subsumer order can depend on allocator addresses and tree rotations even when feature vectors and candidate sets are identical. Repeated bounded GEO288 runs changed C's non-unit subsumption and retained-clause counts slightly under ASLR. Rust uses stable clause identifiers for deterministic leaf order. Do not emulate C allocator layout merely to reproduce identifier permutations; if exact first-hit order proves externally visible, define and test a semantic tie-break in both implementations.
- C allocates and frees a `SubstCell` and zeroed `long` pick array for every recursive non-unit clause-subsumption attempt. The allocator's free lists make this cheaper than general allocation but also let unrelated allocation chronology affect pointer-keyed proof order. A later C implementation should accept reusable caller/session scratch with explicit reset and reentry semantics instead of relying on process-global allocator reuse.
- The four subsumption counters are mutable process globals and assume one active proof search. Rust keeps compatibility-shaped atomics; a cleaned C/Rust interface should store counters in proof-session statistics so parallel or nested searches do not share them.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
