<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_fixpoint_unif

## Source Files

- [TERMS/cte_fixpoint_unif.h](../../../eprover/TERMS/cte_fixpoint_unif.h)
- [TERMS/cte_fixpoint_unif.c](../../../eprover/TERMS/cte_fixpoint_unif.c)

## Purpose

Interface to fixpoint decider. the GNU Lesser General Public License. <1> ma 25 okt 2021 10:35:21 CEST New

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Petar Vukmirovic, Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTE_FIXPOINT_UNIF`

### Globals

- None found in the source scan.

### Exported Functions

- `OracleUnifResult SubstComputeFixpointMgu(Term_p t1, Term_p t2, Subst_p subst)`

## Implementation Notes

### Internal Functions

- `rigid_path_check_args`

### Source-Level Behavior

- `rigid_path_check`: Computes the fixpoint unifier of two terms.
- `rigid_path_check_args`: Does the same as rigid_path_check but for an array of terms.
- `SubstComputeFixpointMgu`: Computes the fixpoint unifier of two terms.

### Dependencies

- `"cte_pattern_match_mgu.h"`
- `<clb_plocalstacks.h>`
- `<cte_lambda.h>`
- `<cte_match_mgu_1-1.h>`
- `<cte_subst.h>`
- `<cte_termtypes.h>`

### Compile-Time Conditions

- `CTE_FIXPOINT_UNIF`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_fixpoint_unif.h`, `TERMS/cte_fixpoint_unif.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 244 lines, 2 scanned public declarations, 1 scanned internal function definitions, and 3 structured function-comment blocks.
- Interface to fixpoint decider. the GNU Lesser General Public License. <1> ma 25 okt 2021 10:35:21 CEST New
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status

- Initial Rust support is in `src/terms/fixpoint_unif.rs`, covering `SubstComputeFixpointMgu` over the existing Rust substitution stack, C-shaped weak-head dereference plus DB eta-reduction before the top-level decision, free-variable/free-variable binding, non-free/non-free `NOT_IN_FRAGMENT`, and rigid-path classification of direct occurrence, applied-variable-under-occurrence, lambda-prefix, arrow-variable, and loose-DB-variable cases.
- Rust takes the owning `TermBank` explicitly for WHNF and eta-reduction. This is the completed ownership boundary: it preserves allocation and sharing without a movable/stale per-term owner pointer, as audited in [experiment 336](../../../experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/FINDINGS.md).

### Change Later

- The C helper returns `NOT_IN_FRAGMENT` for any pair of non-free top-level terms, even if they are syntactically identical. Rust preserves that oracle boundary; revisit only if the full CSU dispatcher needs a separate exact-term fast path before calling the fixpoint oracle.
- The C file body still carries the copied `cte_pattern_match_mgu.c` header/include wording while exporting `cte_fixpoint_unif` behavior. Keep the Rust module named for the actual exported unit, and avoid using the stale banner as ownership evidence.
- The C routine only adds bindings after a successful oracle decision and does not perform its own speculative backtracking. Future multi-oracle callers should keep substitution-stack checkpointing outside this helper, matching the current C contract.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
