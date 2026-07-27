<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_tautologies

## Source Files

- [CLAUSES/ccl_tautologies.h](../../../eprover/CLAUSES/ccl_tautologies.h)
- [CLAUSES/ccl_tautologies.c](../../../eprover/CLAUSES/ccl_tautologies.c)

## Purpose

Functions for detecting tautologies using the algorithm suggested by Roberto Nieuwenhuis: Do ground completion on negative literals, see if they imply the positive ones the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCL_TAUTOLOGIES`
- `ClauseIsTautology(b,c)`
- `MAX_EQ_TAUTOLOGY_CHECK_LITNO`

### Globals

- None found in the source scan.

### Exported Functions

- `bool ClauseIsTautologyReal(TB_p work_bank, Clause_p clause, bool copy)`

## Implementation Notes

### Internal Functions

- `ground_backward_contract`
- `ground_complete_neg_eqns`
- `ground_normalize_eqn`
- `ground_orient_eqn`
- `term_compute_ground_NF`
- `term_compute_top_nf`

### Source-Level Behavior

- `TO_ground_compare`: Compare two terms with a very simple total ordering extendable to a reduction ordering.
- `ground_orient_eqn`: Orient an equation (by setting or deleting appropriate flag). Return true if terms are different, false otherwise.
- `term_compute_top_nf`: Checks if one of the eqns can reduce *ref, if yes does so and returns true. Otherwise returns false.
- `term_compute_ground_NF`: Compute a ground normal form of *ref with respect to eqns. *ref should be unshared, eqns should be interreduced. Return true if term changed. This is probably not an optimal implementation, but *ref and eqns should be pretty small and not worth any of the overhead of the more sophisticated algorithms.
- `ground_normalize_eqn`: Normalize eqn with respect to eqns (which should be interreduced). Return true if maximal side has been rewritten, false otherwise.
- `ground_backward_contract`: Normalize all eqations in from with respect to eqns. Put those whose maximal side has changed into to.
- `ground_complete_neg_eqns`: Complete the negative equations in *list. Return completed system.
- `ClauseIsTautologyReal`: Return true if clause certainly is a tautology, false if this cannot be shown at the accepted expense.

### Dependencies

- `"ccl_derivation.h"`
- `"ccl_tautologies.h"`
- `<ccl_clauses.h>`

### Compile-Time Conditions

- `CCL_TAUTOLOGIES`

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

Source files reviewed: `CLAUSES/ccl_tautologies.h`, `CLAUSES/ccl_tautologies.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 460 lines, 1 scanned public declarations, 6 scanned internal function definitions, and 8 structured function-comment blocks.
- Functions for detecting tautologies using the algorithm suggested by Roberto Nieuwenhuis: Do ground completion on negative literals, see if they imply the positive ones the GNU Lesser General Public License.
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/clauses/tautologies.rs` implements the ground-completion tautology path, including the public copy/no-copy flag shape. C can consume a caller-owned temporary clause on the no-copy path because scratch banks share canonical truth terms with the source bank. Rust banks own distinct canonical term handles, so both flag values create a bank-local work clause before the final pointer-identity comparison. A cross-bank complementary-predicate regression pins this ownership translation and the executable predicate-gate fixture covers the production caller.
<!-- END MANUAL REVIEW: c_source_docs -->
