<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_replace

## Source Files

- [TERMS/cte_replace.h](../../../eprover/TERMS/cte_replace.h)
- [TERMS/cte_replace.c](../../../eprover/TERMS/cte_replace.c)

## Purpose

Functions for replacing and rewriting of terms. the GNU Lesser General Public License. <1> Mon Jan 12 17:50:21 MET 1998 New

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RWResultType`

### Macros And Constants

- `CTE_REPLACE`

### Globals

- None found in the source scan.

### Exported Functions

- `Term_p TBTermPosReplace(TB_p bank, Term_p repl, TermPos_p pos, DerefType deref, int remains, Term_p orig)`
- `Term_p TermFollowRWChain(Term_p term)`
- `void TermAddRWLink(Term_p term, Term_p replace, struct clause_cell *demod, bool sos, RWResultType type)`
- `void TermDeleteRWLink(Term_p term)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `TermAddRWLink`: Add a rewrite link from term to replace, induced by demod. Note: If demod is REWRITE_AT_SUBTERM, actual rewriting happened at a subterm.
- `TermDeleteRWLink`: Delete rewrite link from term.
- `TermFollowRWChain`: Return the last term in an existing rewrite link chain.
- `TBTermPosReplace`: Create a new term by replacing the subterm designated by pos with repl and insert it into the term bank. Return pointer to the new term. The superterm is implicit in the position (or, in the case of the empty position, is unnecessary). Does not free any terms - if necessary, this is the responsibility of the calling functions. Note that this function may de...

### Dependencies

- `"cte_replace.h"`
- `<ccl_clauses.h>`
- `<cio_output.h>`
- `<clb_pqueue.h>`
- `<cte_termcpos.h>`

### Compile-Time Conditions

- `CTE_REPLACE`
- `ENABLE_LFHO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_replace.h`, `TERMS/cte_replace.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 316 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Functions for replacing and rewriting of terms. the GNU Lesser General Public License. <1> Mon Jan 12 17:50:21 MET 1998 New
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `TBTermPosReplace` rebuilds the enclosing term inside-out from the `TermPos` stack, using shallow `TermTopCopy` cells and inserting the final temporary term through `TBInsertNoProps`. Rust preserves this ordinary replacement path with safe temporary cells and term-bank sharing.
- The LFHO positive-`remains` branch calls `MakeRewrittenTerm`, appends the remaining original arguments, sets owner-bank state, and runs lambda normalization. Rust ports the top-level and nested retained-argument construction, shares through the explicit bank, and beta-normalizes both retained and zero-suffix replacements. Exact higher-order rewrite/inference projections cover the production boundary; C's hidden owner/cache writes are deliberately absent under [experiment 336](../../../experiments/2026-07-25-035-lfho-explicit-bank-cache-decision/FINDINGS.md).

### Change Later

- The C integer `remains` sentinel plus `old_into` side parameter may be worth replacing with an explicit prefix-rewrite descriptor, but only as an API cleanup after compatibility.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
