<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termpos

## Source Files

- [TERMS/cte_termpos.h](../../../eprover/TERMS/cte_termpos.h)
- [TERMS/cte_termpos.c](../../../eprover/TERMS/cte_termpos.c)

## Purpose

Positions in terms. the GNU Lesser General Public License. <1> Sun May 10 17:37:08 MET DST 1998 Lifted from cte_rewrite.h (now moved to cte_replace.h)

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TermPosCell`
- `TermPos_p`

### Macros And Constants

- `CTE_TERMPOS`
- `TERM_POS_ELEMENT_SIZE`
- `TermPosAlloc()`
- `TermPosFree(junk)`
- `TermPosIsTopPos(pos)`

### Globals

- None found in the source scan.

### Exported Functions

- `Term_p TermPosNextLIPosition(TermPos_p pos)`
- `static inline Term_p TermPosFirstLIPosition(Term_p term, TermPos_p pos)`
- `static inline Term_p TermPosGetSubterm(Term_p term, TermPos_p pos)`
- `void TermPosDebugPrint(FILE* out, Sig_p sig, TermPos_p pos)`
- `void TermPosPrint(FILE* out, TermPos_p pos)`

## Implementation Notes

### Internal Functions

- `TermPosFirstLIPosition`
- `TermPosGetSubterm`

### Source-Level Behavior

- `TermPosGetSubterm`: Given a term and a position, return the denoted subterm.
- `TermPosFirstLIPosition`: Return the first subterm of term in leftmost-innermost order and make pos the corresponding position.
- `TermPosNextLIPosition`: Given an (implicit) term and a position, compute the next position (in leftmost-innermost order) and return the corresponding term.
- `TermPosPrint`: Print the position as a doted list.
- `TermPosDebugPrint`: Print a position in a term. If sig!=NULL, print terms, otherwise print adddresses

### Dependencies

- `"cte_termpos.h"`
- `<cte_termbanks.h>`

### Compile-Time Conditions

- `CTE_TERMPOS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_termpos.h`, `TERMS/cte_termpos.c`.

### Compatibility Notes

- `TermPosDebugPrint` treats `sig == NULL` as address-debug mode and `sig != NULL` as term-debug mode. The term mode prints each stored superterm twice, first with `DEREF_NEVER`, then after a literal `...` with `DEREF_ALWAYS`, followed by `Subterm <index>`.

### C Behaviors To Revisit After Compatibility

- The nullable `Sig_p` mode switch combines raw address diagnostics and dereferenced term rendering in one API. Rust preserves both modes through explicit helpers; a later cleaned diagnostic API could separate these concerns once drop-in compatibility is established.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 315 lines, 7 scanned public declarations, 2 scanned internal function definitions, and 5 structured function-comment blocks.
- Positions in terms. the GNU Lesser General Public License. <1> Sun May 10 17:37:08 MET DST 1998 Lifted from cte_rewrite.h (now moved to cte_replace.h)
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
