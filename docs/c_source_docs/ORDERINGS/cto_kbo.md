<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# ORDERINGS / cto_kbo

## Source Files

- [ORDERINGS/cto_kbo.h](../../../eprover/ORDERINGS/cto_kbo.h)
- [ORDERINGS/cto_kbo.c](../../../eprover/ORDERINGS/cto_kbo.c)

## Purpose

Definitions for implementing a Knuth-Bendix ordering. the GNU Lesser General Public License. <1> Thu May 28 12:14:31 MET DST 1998 New

Within the source tree, this unit belongs to `ORDERINGS`. Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching.

Authors noted in source headers: Stephan Schulz, Stephan Schulz (original Version and some comments by JS)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTO_KBO`

### Globals

- None found in the source scan.

### Exported Functions

- `CompareResult KBOCompare(OCB_p ocb, Term_p t1, Term_p t2, DerefType deref_t1, DerefType deref_t2)`
- `CompareResult KBOVarCompare(Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `bool KBOGreater(OCB_p ocb, Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`
- `bool KBOVarGreater(Term_p s, Term_p t, DerefType deref_s, DerefType deref_t)`

## Implementation Notes

### Internal Functions

- `gettermweight`
- `getweight`
- `kbocomparevars`
- `kbogtrnew`

### Source-Level Behavior

- `getweight`: Provides the weight of the operator specified by <op>.
- `gettermweight`: Returns the weight of a term t=f(t_1,...,t_n): w(t) = w(f) + w(t_1) + ... + w(t_n)
- `kbocomparevars`: Compares two terms s and t wrt. KBO if either s or t is a variable and returns to_greater if t is a subterm of s to_equal if s == t to_lesser if s is a subterm of t to_uncomparable otherwise
- `kbogtrnew`: Returns to_greater if s >KBO t, to_equal if s =KBO t, to_uncomparable otherwise. Its a variant of kbogtr where the variable condition is tested in the end.
- `KBOCompare`: Compare two terms s,t in the Knuth-Bendix Ordering, return the result to_greater if s >KBO t to_equal if s =KBO t to_lesser if t >KBO s to_uncomparable otherwise Its a variant of KBOCompare where the variable condition is tested in the end. NOTE: derefs have not been updated here because it is used only on FOL terms.
- `KBOVarCompare`: Compare the variable occurences in two terms, return the strongest KBO result compatible with the variable condition.
- `KBOGreater`: Checks whether the term s is greater than the term t in the Knuth-Bendix Ordering (KBO), i.e. returns true if s >KBO t, false otherwise. For a description of the KBO see the header of this file. Its a variant of KBOGreater where the variable condition is tested in the end.
- `KBOVarGreater`: Return true if vars(s) multisetsubseteq vars(t), false otherwise.

### Dependencies

- `"cto_kbo.h"`
- `<cte_varhash.h>`
- `<cto_ocb.h>`

### Compile-Time Conditions

- `CTO_KBO`
- `NEVER_DEFINED`

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

Source files reviewed: `ORDERINGS/cto_kbo.h`, `ORDERINGS/cto_kbo.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `ORDERINGS` covering 2 source file(s), about 800 lines, 8 scanned public declarations, 4 scanned internal function definitions, and 8 structured function-comment blocks.
- KBO implementation. Weight, precedence, variable-condition, and cache interactions must match C comparisons.
- Ordering code. Comparison outcomes, caching, precedence, and weight handling must match the C implementation because they drive simplification and inference eligibility.
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
