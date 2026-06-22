<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# ORDERINGS / cto_kbolin

## Source Files

- [ORDERINGS/cto_kbolin.h](../../../eprover/ORDERINGS/cto_kbolin.h)
- [ORDERINGS/cto_kbolin.c](../../../eprover/ORDERINGS/cto_kbolin.c)

## Purpose

Definitions for implementing a linear time implementation of the Knuth-Bendix ordering. The implementation is based in the ideas presented in [Loechner:JAR-2006] (Bernd Loechner, "Things to Know when Implementing KBO", JAR 36(4):289-310, 2006.

Within the source tree, this unit belongs to `ORDERINGS`. Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTO_KBOLIN`

### Globals

- None found in the source scan.

### Exported Functions

- `CompareResult KBO6Compare(OCB_p ocb, Term_p t1, Term_p t2, DerefType deref_t1, DerefType deref_t2)`
- `bool KBO6Greater(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`

## Implementation Notes

### Internal Functions

- `__attribute__`
- `classify_head`
- `cmp_arities`
- `cmp_heads`
- `dec_vb`
- `dec_vb_ho`
- `heads_same`
- `inc_vb`
- `inc_vb_ho`
- `is_fluid`
- `kbo6cmp`
- `kbo6cmplex`
- `kbo6reset`
- `kbolincmp`
- `kbolincmp_ho`
- `kbolincmp_lambda`
- `kbolincmp_lambda_driver`
- `local_vb_update`
- `mfyvwb`
- `mfyvwb_ho`
- `mfyvwbc`
- `mfyvwblhs`
- `mfyvwbrhs`

### Source-Level Behavior

- `resize_vb`: Enlarge ocb->vb array enough to accomodate index.
- `is_fluid`: Approximation the fluidity test -- see https://arxiv.org/abs/2102.00453 for definition
- `inc_vb`: Update all values in ocb when processing var on the LHS of a comparison.
- `inc_vb_ho`: Like inc_vb, but maps fluid terms to fresh variables.
- `dec_vb`: Update all values in ocb when processing var on the RHS of a comparison.
- `dec_vb_ho`: Like dec_vb, but maps fluid terms to fresh variables.
- `local_vb_update`: Perform a local update of ocb according to t (which is not derefed).
- `mfyvwbc`: Update ocb according to t and lhs while checking if var occurs in t.
- `mfyvwb`: Update ocb according to t and lhs.
- `kbo6cmplex`: Perform a lexicographical comparison of the argument lists of s and t, updating the variable/weight balances accordingly. NB: function called only for FO terms
- `kbo6cmp`: Perform a KBO comparison between s and t.
- `mfyvwblhs`: Update ocb according to term on the LHS of a comparison.
- `mfyvwbrhs`: Update ocb according to term on the RHS of a comparison.
- `mfyvwblhs_ho`: Update ocb according to term on the LHS of a comparison.
- `heads_same`: Update ocb according to term on the LHS of a comparison.
- `classify_head`: Assigns a number to the term head that can be used for comparison.
- `cmp_heads`: Compares head terms.
- `kbolincmp`: Perform a KBO comparison between s and t.
- `kbolincmp_lambda_driver`: Does the actual comparison.
- `kbolincmp_lambda`: Perform a KBO comparison between s and t that takes lambdas into account. Amounts to Boolean free derived lambda KBO.
- `cmp_arities`: Support length-lexicographic comparsion.
- `kbolincmp_ho`: Perform a KBO comparison between s and t, which are LFHOL terms.
- `kbo6reset`: Reset data in ocb changed when determining KBO6 comparison of terms.
- `KBO6Compare`: Compare two terms s,t in the Knuth-Bendix Ordering, return the result to_greater if s >KBO t to_equal if s =KBO t to_lesser if t >KBO s to_uncomparable otherwise Its a variant of KBOCompare where the variable condition is tested in the end.
- `KBO6Greater`: Checks whether the term s is greater than the term t in the Knuth-Bendix Ordering (KBO), i.e. returns true if s >KBO t, false otherwise. For a description of the KBO see the header of this file. Its a variant of KBOGreater where the variable condition is tested in the end.

### Dependencies

- `"clb_plocalstacks.h"`
- `"cto_kbolin.h"`
- `<cte_lambda.h>`
- `<cto_ocb.h>`

### Compile-Time Conditions

- `CTO_KBOLIN`
- `ENABLE_LFHO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `ORDERINGS/cto_kbolin.h`, `ORDERINGS/cto_kbolin.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `ORDERINGS` covering 2 source file(s), about 1405 lines, 7 scanned public declarations, 23 scanned internal function definitions, and 25 structured function-comment blocks.
- Definitions for implementing a linear time implementation of the Knuth-Bendix ordering. The implementation is based in the ideas presented in [Loechner:JAR-2006] (Bernd Loechner, "Things to Know when Implementing KBO", JAR 36(4):289-310, 2006.
- Ordering code. Comparison outcomes, caching, precedence, and weight handling must match the C implementation because they drive simplification and inference eligibility.
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
<!-- END MANUAL REVIEW: c_source_docs -->
