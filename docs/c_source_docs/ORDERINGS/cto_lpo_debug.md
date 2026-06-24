<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# ORDERINGS / cto_lpo_debug

## Source Files

- [ORDERINGS/cto_lpo_debug.h](../../../eprover/ORDERINGS/cto_lpo_debug.h)
- [ORDERINGS/cto_lpo_debug.c](../../../eprover/ORDERINGS/cto_lpo_debug.c)

## Purpose

Definitions for implementing a lexicographic path ordering. the GNU Lesser General Public License. <1> Thu May 28 12:14:31 MET DST 1998 New

Within the source tree, this unit belongs to `ORDERINGS`. Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching.

Authors noted in source headers: Joachim Steinbach

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTO_LPO_DEBUG`

### Globals

- None found in the source scan.

### Exported Functions

- `CompareResult D_LPOCompare(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `CompareResult D_LPOCompareVars(Term_p, Term_p, DerefType, DerefType)`
- `bool D_LPOGreater(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`

## Implementation Notes

### Internal Functions

- `lpocheckarg`
- `lpofuneq`
- `lpofungtr`
- `lpogtr`
- `lpogtrcheckarg`
- `lpogtrcompvars`
- `lpogtrfuneq`

### Source-Level Behavior

- `lpofungtr`: Compares the term s with the arguments t_1,...,t_n of t and returns to_greater if s >LPO t_1, ..., s >LPO t_n to_lesser if there exists t_i with t_i >LPO s to_uncomparable otherwise
- `lpofuneq`: Compares the arguments s_1,...,s_m of the term s with the arguments t_1,...,t_n of the term t and returns to_equal if m=n & s_1 =LPO t_1, ..., s_m =LPO t_m to_greater if (s_1,...,s_m) >LPOlex (t_1,...,t_n) & forall t_i: s >LPO t_i to_lesser if (t_1,...,t_n) >LPOlex (s_1,...,s_n) & forall s_i: t >LPO s_i to_uncomparable otherwise
- `lpocheckarg`: Checks the third condition of the LPO, i.e. returns to_greater if there is an argument s_i of s with s_i >=LPO t to_uncomparable otherwise
- `lpogtr`: Returns to_greater if s >LPO t, to_equal if s =LPO t, to_uncomparable otherwise.
- `lpogtrcompvars`: Compares two terms s and t wrt. LPO if either s or t is a variable and returns to_greater if t is a subterm of s to_equal if s == t to_uncomparable otherwise If s is a variable, then varps is true.
- `lpogtrfuneq`: Compares the arguments s_1,...,s_m of the term s with the arguments t_1,...,t_n of the term t and returns to_equal if m=n & s_1 =LPO t_1, ..., s_m =LPO t_m to_greater if (s_1,...,s_m) >LPOlex (t_1,...,t_n) & forall t_i: s >LPO t_i to_uncomparable otherwise
- `lpogtrcheckarg`: Checks the third condition of the LPO, i.e. returns v// to_greater if there is an argument s_i of s with s_i >=LPO t to_uncomparable otherwise
- `D_LPOCompare`: Compare two terms s,t in the Lexicographic Path Ordering, return the result to_greater if s >LPO t to_equal if s =LPO t to_lesser if t >LPO s to_uncomparable otherwise
- `D_LPOGreater`: Checks whether the term s is greater than the term t in the Lexicographic Path Ordering (LPO), i.e. returns true if s >LPO t, false otherwise. For a description of the LPO see the header of this file.
- `D_LPOCompareVars`: Compares two terms s and t wrt. LPO if either s or t is a variable and returns to_greater if t is a subterm of s to_equal if s == t to_lesser if s is a subterm of t to_uncomparable otherwise

### Dependencies

- `"cto_lpo_debug.h"`
- `<cto_ocb.h>`

### Compile-Time Conditions

- `CTO_LPO_DEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `ORDERINGS/cto_lpo_debug.h`, `ORDERINGS/cto_lpo_debug.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `ORDERINGS` covering 2 source file(s), about 674 lines, 10 scanned public declarations, 7 scanned internal function definitions, and 10 structured function-comment blocks.
- Definitions for implementing a lexicographic path ordering. the GNU Lesser General Public License. <1> Thu May 28 12:14:31 MET DST 1998 New
- Ordering code. Comparison outcomes, caching, precedence, and weight handling must match the C implementation because they drive simplification and inference eligibility.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- This debug LPO implementation has no recursion-depth guard and its same-head tail checks use `MAX(s->arity,t->arity)` while indexing only the remaining arguments from one side. Rust should keep the intended LPO tail condition without out-of-bounds access; revisit only if this debug path becomes externally observable.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
