<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termcpos

## Source Files

- [TERMS/cte_termcpos.h](../../../eprover/TERMS/cte_termcpos.h)
- [TERMS/cte_termcpos.c](../../../eprover/TERMS/cte_termcpos.c)

## Purpose

Functions dealing with compact term positions represented by simple integers. Subterms are numbered in standard left-right pre-order, with the root position at 0. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TermCPos`

### Macros And Constants

- `CTE_TERMCPOS`
- `TermCPosIsTopPos(pos)`

### Globals

- None found in the source scan.

### Exported Functions

- `TermCPos TermCPosFromTermPos(TermPos_p termpos)`
- `Term_p TermCPosGetSubterm(Term_p term, TermCPos pos)`
- `bool TermPosFromTermCPos(Term_p term, TermCPos pos)`
- `void TermPrintAllCPos(FILE* out, TB_p bank, Term_p term)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `TermPrintAllCPos`: Print all compact positions in a term, with the associated subterm. Probably only for testing and debugging.

### Dependencies

- `"cte_termcpos.h"`
- `<cte_termpos.h>`

### Compile-Time Conditions

- `CTE_TERMCPOS`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Determine which term-sharing and term-bank properties are semantic constraints and which are replaceable memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_termcpos.h`, `TERMS/cte_termcpos.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 184 lines, 5 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Functions dealing with compact term positions represented by simple integers. Subterms are numbered in standard left-right pre-order, with the root position at 0. the GNU Lesser General Public License.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
