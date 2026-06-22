<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termweightext

## Source Files

- [TERMS/cte_termweightext.h](../../../eprover/TERMS/cte_termweightext.h)
- [TERMS/cte_termweightext.c](../../../eprover/TERMS/cte_termweightext.c)

## Purpose

Generic extensions of term weight functions to clause weight functions the GNU Lesser General Public License.

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz, yan

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `TermWeightExtensionCell`
- `TermWeightExtension_p`
- `TermWeightExtenstionStyle`
- `TermWeightFun`

### Macros And Constants

- `CTE_TERMWEIGHTEXT`
- `TermWeightExtensionCellAlloc()`
- `TermWeightExtensionCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `TermWeightExtension_p TermWeightExtensionAlloc( double max_term_multiplier, double max_literal_multiplier, double pos_eq_multiplier, TermWeightExtenstionStyle ext_style, TermWeightFun term_weight_fun, void* data)`
- `double TermExtWeight(Term_p term, TermWeightExtension_p twe)`
- `void TermWeightExtensionFree(TermWeightExtension_p junk)`

## Implementation Notes

### Internal Functions

- `term_ext_weight_max`
- `term_ext_weight_sum`

### Source-Level Behavior

- `TermWeightExtensionAlloc`: Allocate and initialize a new extension cell.
- `TermWeightExtensionFree`: Free an extension cell.

### Dependencies

- `"cte_termtypes.h"`
- `"cte_termweightext.h"`
- `<float.h>`

### Compile-Time Conditions

- `CTE_TERMWEIGHTEXT`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `TERMS/cte_termweightext.h`, `TERMS/cte_termweightext.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 259 lines, 7 scanned public declarations, 2 scanned internal function definitions, and 2 structured function-comment blocks.
- Generic extensions of term weight functions to clause weight functions the GNU Lesser General Public License.
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
