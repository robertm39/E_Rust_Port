<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_clausefunc

## Source Files

- [CLAUSES/ccl_clausefunc.h](../../../eprover/CLAUSES/ccl_clausefunc.h)
- [CLAUSES/ccl_clausefunc.c](../../../eprover/CLAUSES/ccl_clausefunc.c)

## Purpose

Clause and formula functions that need to know about sets (and similar stuff (ccl_clauses.c is to big anyways). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_CLAUSEFUNC`

### Globals

- None found in the source scan.

### Exported Functions

- `Clause_p ClauseArchive(ClauseSet_p archive, Clause_p clause)`
- `Clause_p ClauseArchiveCopy(ClauseSet_p archive, Clause_p clause)`
- `Clause_p ClauseRecognizeInjectivity(TB_p terms, Clause_p clause)`
- `bool ClauseEliminateNakedBooleanVariables(Clause_p clause)`
- `bool ClauseIsOrphaned(Clause_p clause)`
- `bool ClauseUnitSimplifyTest(Clause_p clause, Clause_p simplifier)`
- `int ClauseCanonCompareRef(const void *clause1ref, const void* clause2ref)`
- `int ClauseRemoveACResolved(Clause_p clause)`
- `int ClauseRemoveSuperfluousLiterals(Clause_p clause)`
- `long ClauseSetDeleteOrphans(ClauseSet_p set)`
- `long ClauseSetRemoveSuperfluousLiterals(ClauseSet_p set)`
- `long ClauseSetReplaceInjectivityDefs(ClauseSet_p set, ClauseSet_p archive, TB_p terms)`
- `void ClauseFlipLiteralSign(Clause_p clause, Eqn_p lit)`
- `void ClauseRemoveLiteral(Clause_p clause, Eqn_p lit)`
- `void ClauseRemoveLiteralRef(Clause_p clause, Eqn_p *lit)`
- `void ClauseSetArchiveCopy(ClauseSet_p archive, ClauseSet_p set)`
- `void ClauseSetCanonize(ClauseSet_p set)`
- `void PStackClausePrint(FILE* out, PStack_p stack, char* extra)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `unif_all_pairs`: Assuming that stack contains [s1, t1, s2, t2, ..., sn, tn] computes simultaneous unifier of s1 =?= t1, ..., sn =?= tn and stores it in subst.
- `collect_free_vars`: Returns free variables of term t, except in subterm t|idx_to_skip.
- `ClauseCanonCompareRef`: / Compare two indirectly pointed to clauses with ClauseStructWeightLexCompare().
- `ClauseRemoveLiteralRef`: Remove *lit from clause, adjusting counters as necessary.
- `ClauseRemoveLiteral`: Remove lit from clause, adjusting counters as necessary. This is a lot less efficient then ClauseRemoveLiteralRef(), as we have to search for the literal.
- `ClauseFlipLiteralSign`: Change the sign of lit, adjusting counters as necessary.
- `ClauseRemoveSuperfluousLiterals`: Remove duplicate and trivial negative literals from the clause. Return number of removed literals.
- `ClauseSetRemoveSuperflousLiterals`: For all clauses in set remove the trivial and duplicated literals. Return number of literals removed.
- `ClauseSetCanonize`: Canonize a clause set by canonizing all clauses, and sorting them in the order defined by ClauseStructWeightLexCompare().
- `ClauseRemoveACResolved`: Remove AC-resolved literals.
- `ClauseUnitSimplifyTest`: Return true if clause can be simplified by a top-simplify-reflect step with the (non-orientable) unit clause simplifier.
- `ClauseArchive`: Move clause into the archive. Create a fresh copy pointing to the old clause in its derivation and return it. Also set the
- `ClauseArchiveCopy`: Create an archive copy of clause in archive. The archive copy inherits info and derivation. The original loses info, and gets a new derivation that points to the archive copy. Returns pointer to the archived copy.
- `ClauseSetArchiveCopy`: Create an archive copy of each clause in set in archive. The archive copy inherits info and derivation. The original loses info, and gets a new derivation that points to the archive copy.
- `ClauseIsOrphaned`: Return true if the clause is orphaned, i.e. if one of the direct premises of the original generating inferences that generated it has been back-simplified.
- `ClauseSetDeleteOrphans`: Remove all orphaned clauses, returning the number of clauses eliminated.
- `PStackClausePrint`: Print the clauses on the stack.
- `ClauseEliminateNakedBooleanVariables`: If the clause containts boolean variables X and ~X, convert the clause to {$true}. If the clause C contains only X replace the clause with C[X |-> $false]. If the clause C contains only ~X replace C with C[X |-> $true]. Returns true if a clause becomes a tautology.
- `ClauseRecognizeInjectivity`: Create a clause that postulates existence of an inverse function for a given expression. In other words: f X_1 ... X_n != f Y1 ... Y_n \/ X_i = Y_i inv_f_i(f sigma(X_1) ... X_i ... sigma(X_n)) = X_i where for some subset I of indexes from 1 to n, X_i = Y_i for each i in I, and for complementary set of indexes J there all X_j and Y_j (j \in J) and different...
- `ClauseSetInjectivityIsDefined`: Finds definitions of skolem symbols standing for the renaming of the injectivity axiom.
- `ReplaceInjectivityDefs`: Replaces defintions of injectivity by clauses that define inverse operators.

### Dependencies

- `"ccl_clausefunc.h"`
- `<ccl_clausesets.h>`
- `<ccl_formula_wrapper.h>`

### Compile-Time Conditions

- `CCL_CLAUSEFUNC`

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

Source files reviewed: `CLAUSES/ccl_clausefunc.h`, `CLAUSES/ccl_clausefunc.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 1131 lines, 18 scanned public declarations, 0 scanned internal function definitions, and 21 structured function-comment blocks.
- Clause and formula functions that need to know about sets (and similar stuff (ccl_clauses.c is to big anyways). the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
