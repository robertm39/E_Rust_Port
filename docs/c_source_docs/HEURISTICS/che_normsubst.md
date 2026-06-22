<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_normsubst

## Source Files

- [HEURISTICS/che_normsubst.h](../../../eprover/HEURISTICS/che_normsubst.h)
- [HEURISTICS/che_normsubst.c](../../../eprover/HEURISTICS/che_normsubst.c)

## Purpose

Substitutions mapping function symbols and variables to norm values. the GNU Lesser General Public License. <1> Mon Feb 16 01:04:12 MET 1998

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `NormSubstCell`
- `NormSubst_p`

### Macros And Constants

- `CLE_NORMSUBST`
- `NormSubstCellAlloc()`
- `NormSubstCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `NormSubst_p NormSubstAlloc(void)`
- `void NormSubstFree(NormSubst_p junk)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `"che_normsubst.h"`
- `<clb_numtrees.h>`
- `<cte_signature.h>`

### Compile-Time Conditions

- `CLE_NORMSUBST`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_normsubst.h`, `HEURISTICS/che_normsubst.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 142 lines, 4 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- Substitutions mapping function symbols and variables to norm values. the GNU Lesser General Public License. <1> Mon Feb 16 01:04:12 MET 1998
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
