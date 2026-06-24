<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# ORDERINGS / cto_lpo

## Source Files

- [ORDERINGS/cto_lpo.h](../../../eprover/ORDERINGS/cto_lpo.h)
- [ORDERINGS/cto_lpo.c](../../../eprover/ORDERINGS/cto_lpo.c)

## Purpose

Definitions for implementing a lexicographic path ordering. the GNU Lesser General Public License. <1> Thu May 28 12:14:31 MET DST 1998 New

Within the source tree, this unit belongs to `ORDERINGS`. Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching.

Authors noted in source headers: Stephan Schulz (original implementation and definitions by, Stephan Schulz and Joachim Steinbach

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTO_LPO`
- `DB`
- `LAMBDA`
- `QUANT`
- `SYM`

### Globals

- `extern long LPORecursionDepthLimit`

### Exported Functions

- `CompareResult LPO4Compare(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `CompareResult LPO4CompareCopy(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `CompareResult LPOCompare(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `CompareResult LPOCompareCopy(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `bool LPO4Greater(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `bool LPO4GreaterCopy(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `bool LPOGreater(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `bool LPOGreaterCopy(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`

## Implementation Notes

### Internal Functions

- `adjust_ho_deref`
- `classify_head`
- `is_ho_subterm`
- `lpo4_alpha`
- `lpo4_copy_alpha`
- `lpo4_copy_greater`
- `lpo4_copy_lex_ma`
- `lpo4_copy_majo`
- `lpo4_greater`
- `lpo4_lex_ma`
- `lpo4_majo`
- `lpo_greater`
- `lpo_lex_greater`
- `lpo_subterm_dominates_term`
- `lpo_term_dominates_args`

### Source-Level Behavior

- `classify_head`: Depending on what the head of the term is, assign it an integer such that LAMBDA > DB > QUANTIFIERS > SYMBOLS
- `adjust_ho_deref`: If the term has an applied variable and deref kind is deref_once, instantiate the term and set the deref to DEREF_NEVER. This is done so that the problems with DEREF_ONCE and applied variables (that part of the term are derefed and parts not) are avoided.
- `lpo_term_dominates_args`: Return true if s >LPO t_i for all subterms t_i of t, false otherwise.
- `lpo_subterm_dominates_term`: Return true if s_i >=LPO t for a direct subterm of s.
- `lpo_lex_greater`: Compare the arguments of s and t. Return to_greater if s1...sn >_LPO_lex t1...tm to_equal if s1...sn ~LPO_lex t1...tm to_uncomparable if s1...sn =!=LPO_lex t1...tm to_notgteq otherwise
- `lpo_greater`: Check if s >lpo t. Return to_equal if s = t to_greater if s >lpo t to_uncomparable if s and t are definitly uncomparable to_nogteq otherwise
- `lpo4_alpha`: Handle the LPO case alpha (s_i >=LPO t). s, pos represents the argument list of s starting at pos.
- `lpo4_majo`: Handle the majorisation check of LPO (s >=LPO t_i for all i). See above (this time its t, pos).
- `lpo4_lex_ma`: Implement the lex_ma_4 function, combining lexicographical comparison and alpha case.
- `lpo4_greater`: LPO comparison using the lpo_4_nc algorithm by Bernd Loechner.
- `lpo4_copy_alpha`: Handle the LPO case alpha (s_i >=LPO t). s, pos represents the argument list of s starting at pos.
- `lpo4_copy_majo`: Handle the majorisation check of LPO (s >=LPO t_i for all i). See above (this time its t, pos).
- `lpo4_copy_lex_ma`: Implement the lex_ma_4_nc function, combining lexicographical comparison and alpha case.
- `lpo4_copy_greater`: LPO comparison using the lpo_4_nc algorithm by Bernd Loechner.
- `LPOGreater`: Checks whether the term s is greater than the term t in the Lexicographic Path Ordering (LPO), i.e. returns true if s >LPO t, false otherwise. For a description of the LPO see the header of this file.
- `LPOCompare`: Compare two terms s,t in the Lexicographic Path Ordering, return the result to_greater if s >LPO t to_equal if s =LPO t to_lesser if t >LPO s to_uncomparable otherwise
- `LPO4Greater`: Checks whether the term s is greater than the term t in the Lexicographic Path Ordering (LPO), i.e. returns true if s >LPO t, false otherwise. For a description of the LPO see the header of this file.
- `LPO4Compare`: Determine relationship between s and t.
- `LPO4GreaterCopy`: Wrapper for comparing two terms using the LPO4 implementation.
- `LPO4CompareCopy`: Determine relationship between s and t.
- `LPOGreaterCopy`: Wrapper for comparing two terms using the standard LPO implementation with uninstantiated terms.
- `LPOCompareCopy`: Determine relationship between s and t.

### Dependencies

- `"cto_lpo.h"`
- `<cte_lambda.h>`
- `<cto_ocb.h>`

### Compile-Time Conditions

- `CTO_LPO`
- `ENABLE_LFHO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `ORDERINGS/cto_lpo.h`, `ORDERINGS/cto_lpo.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `ORDERINGS` covering 2 source file(s), about 1240 lines, 21 scanned public declarations, 15 scanned internal function definitions, and 23 structured function-comment blocks.
- LPO implementation. Recursive comparison semantics are correctness-critical for simplification.
- Ordering code. Comparison outcomes, caching, precedence, and weight handling must match the C implementation because they drive simplification and inference eligibility.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Standard `lpo_greater` returns the internal `to_notgteq` value for "not greater-or-equal"; public `LPOCompare` only then tries the reverse direction. Preserve that two-step result flow rather than collapsing it into ordinary incomparability too early.
- `lpo_greater` uses a file-static `recursion_depth` counter with global `LPORecursionDepthLimit`, which makes the C helper non-reentrant. A Rust comparison-local depth counter is a sensible cleanup as long as the observable limit result is preserved.
- Standard LPO and LPO4/copy variants are separate implementations. A standard-LPO port does not cover the polynomial LPO4 algorithm or copy wrappers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
