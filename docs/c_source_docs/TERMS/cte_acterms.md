<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_acterms

## Source Files

- [TERMS/cte_acterms.h](../../../eprover/TERMS/cte_acterms.h)
- [TERMS/cte_acterms.c](../../../eprover/TERMS/cte_acterms.c)

## Purpose

Functions and data types for terms in AC normal form (flattened, subterms sorted alphabetically). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ACTermCell`
- `ACTerm_p`

### Macros And Constants

- `ACTermCellAlloc()`
- `ACTermCellFree(junk)`
- `CTE_ACTERMS`

### Globals

- None found in the source scan.

### Exported Functions

- `ACTerm_p ACTermAlloc(FunCode f)`
- `ACTerm_p ACTermNormalize(Sig_p sig, Term_p term)`
- `bool TermACEqual(Sig_p sig, Term_p t1, Term_p t2)`
- `int ACTermCompare(ACTerm_p t1, ACTerm_p t2)`
- `void ACTermFree(ACTerm_p term)`
- `void ACTermPrint(FILE* out, ACTerm_p term, Sig_p sig)`

## Implementation Notes

### Internal Functions

- `ac_collect_args`
- `acterm_uniq_compare`

### Source-Level Behavior

- `acterm_uniq_compare`: Compare two AC-Terms first lexicographically and then by top-level-pointer. Two copies of the same term compare as different here.
- `ac_collect_args`: Collect all subterms of the AC-Symbol f in the orderd tree anchored at *root.
- `ACTermAlloc`: Allocate an initialized AC-Term cell
- `ACTermFree`: Free an AC-Term.
- `ACTermCompare`: Compare two AC terms lexicograpically.
- `ACTermNormalize`: Transform a CLIB term into an AC term in AC-normalform.
- `ACTermPrint`: Print an AC-Normalized term in flat form.
- `TermACEqual`: Return true if the two terms are equal modulo AC as described in the signature.

### Dependencies

- `"cte_acterms.h"`
- `<clb_objtrees.h>`
- `<cte_termfunc.h>`

### Compile-Time Conditions

- `CTE_ACTERMS`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_acterms.h`, `TERMS/cte_acterms.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 450 lines, 8 scanned public declarations, 2 scanned internal function definitions, and 8 structured function-comment blocks.
- Functions and data types for terms in AC normal form (flattened, subterms sorted alphabetically). the GNU Lesser General Public License.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `acterm_uniq_compare()` uses raw pointer order to keep structurally equal temporary AC subterms as distinct tree entries. Rust keeps duplicate normalized arguments without depending on allocator address order; if future diagnostics or heuristics expose duplicate ordering, add reference traces before replacing the C tie break with deterministic metadata.
- `ACTermCompare()` returns `-1` whenever either top symbol is `SIG_DB_LAMBDA_CODE`, even if both sides are DB lambdas. Rust preserves the inherited comparison quirk; a cleaned higher-order AC comparison should define lambda ordering explicitly once lambda-normalization and AC callers are fully covered.
- `TermACEqual()` first rejects terms with different standard weights or either phony-application marker before building AC-normal forms. Rust mirrors that fast path for parity; if phony applications become normal first-class terms in a later owner model, revisit whether AC equality should normalize them instead of rejecting them early.
<!-- END MANUAL REVIEW: c_source_docs -->
