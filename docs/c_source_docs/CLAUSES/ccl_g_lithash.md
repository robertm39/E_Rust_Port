<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_g_lithash

## Source Files

- [CLAUSES/ccl_g_lithash.h](../../../eprover/CLAUSES/ccl_g_lithash.h)
- [CLAUSES/ccl_g_lithash.c](../../../eprover/CLAUSES/ccl_g_lithash.c)

## Purpose

Algorithms and data structures implementing a simple literal indexing structure for implementing local unification constraints for the grounding procedure. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `LitDescCell`
- `LitDesc_p`
- `LitHashCell`
- `LitHash_p`

### Macros And Constants

- `CCL_G_LITHASH`
- `LitDescCellAlloc()`
- `LitDescCellFree(junk)`
- `LitHashCellAlloc()`
- `LitHashCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `LitHash_p LitHashAlloc(Sig_p sig)`
- `int LitDescCompare(const void* lit1, const void* lit2)`
- `void LitHashFree(LitHash_p junk)`
- `void LitHashInsertClause(LitHash_p hash, Clause_p clause)`
- `void LitHashInsertClauseSet(LitHash_p hash, ClauseSet_p set)`
- `void LitHashInsertEqn(LitHash_p hash, Eqn_p eqn, Clause_p clause)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `lit_tree_free`: Free the memory taken up by a PObjTree containing LitDescCells.
- `LitDescCompare`: Compare two literal occurrence description cells. They are equal if the literal terms are equal, the clause is not used!
- `LitHashAlloc`: Allocate a literal hash suitable for the given signature.
- `LitHashFree`: Free the memory occupied by a lithashtable.
- `LitHashInsertEqn`: Insert a literal (the left hand side of the equation) into the literal hash.
- `LitHashInsertClause`: Insert all literals in clause into the loteral hash.
- `LitHashInsertClauseSet`: Insert all literals in all clauses in the set into the hash.

### Dependencies

- `"ccl_g_lithash.h"`
- `<ccl_clausesets.h>`

### Compile-Time Conditions

- `CCL_G_LITHASH`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_g_lithash.h`, `CLAUSES/ccl_g_lithash.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 370 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 7 structured function-comment blocks.
- Algorithms and data structures implementing a simple literal indexing structure for implementing local unification constraints for the grounding procedure. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
