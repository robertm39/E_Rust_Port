<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_garbage_coll

## Source Files

- [CLAUSES/ccl_garbage_coll.h](../../../eprover/CLAUSES/ccl_garbage_coll.h)
- [CLAUSES/ccl_garbage_coll.c](../../../eprover/CLAUSES/ccl_garbage_coll.c)

## Purpose

High-level garbage collection (which needs clause - and formulasets). This is complemented by cte_garbage_coll.[ch] for the lower-level functions. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_GARBAGE_COLL`

### Globals

- None found in the source scan.

### Exported Functions

- `long TBGCCollect(TB_p bank)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `TBGCCollect`: Perform garbage collection on bank.

### Dependencies

- `"ccl_garbage_coll.h"`
- `<ccl_clausesets.h>`
- `<ccl_formulasets.h>`

### Compile-Time Conditions

- `CCL_GARBAGE_COLL`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for formula-CNF registered-root behavior on 2026-07-16.

Source files reviewed: `CLAUSES/ccl_garbage_coll.h`, `CLAUSES/ccl_garbage_coll.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 141 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- High-level garbage collection (which needs clause - and formulasets). This is complemented by cte_garbage_coll.[ch] for the lower-level functions. the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `TBGCCollect` marks every clause and formula set registered in `bank->gc`, even when called from a helper such as `FormulaSetSimplify` that receives only one active set. GEO288 tracing showed this global root coverage is compatibility-visible: omitting the pre-CNF archive reclaimed 897 additional nodes, shifted later collections, and left Rust with 39 additional unique term entries after CNF.
- Rust proof-state CNF now supplies a typed owner context that resolves all 12 registered clause owners and all four registered formula owners. Standalone tools use a separate local context for their registered active/archive/clause trio. Both contexts preserve clause-before-formula marking and check live registration before sweeping; explicit helper root slices remain only in low-level compatibility tests.

### Change Later

- `TBGCCollect` dispatches through untyped pointers stored in the term bank's GC admin and assumes every registered pointer still names a live `ClauseSet` or `FormulaSet`. Rust now avoids that hazard for represented proof-state and standalone owners through typed contexts and stable handles; a future C cleanup should likewise bind registration to typed owner lifetimes.
- GC retention depends on all sets currently registered with the bank, not on roots named by the caller. This makes local transformations sensitive to unrelated owner-registration timing. Rust now makes the effective proof-state and standalone root domains explicit; retain that distinction if the C registry is later redesigned.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
