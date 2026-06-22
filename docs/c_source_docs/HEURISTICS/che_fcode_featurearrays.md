<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_fcode_featurearrays

## Source Files

- [HEURISTICS/che_fcode_featurearrays.h](../../../eprover/HEURISTICS/che_fcode_featurearrays.h)
- [HEURISTICS/che_fcode_featurearrays.c](../../../eprover/HEURISTICS/che_fcode_featurearrays.c)

## Purpose

Sortable arrays associating a function symbol with a number of integer feature values (that define the order). Used by precedence generating functions, now also for weights. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FCodeFeatureArrayCell`
- `FCodeFeatureArray_p`
- `FCodeFeatureSortCell`
- `FCodeFeatureSort_p`

### Macros And Constants

- `CHE_F_CODE_FEATUREARRAYS`
- `FCodeFeatureArrayCellAlloc()`
- `FCodeFeatureArrayCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `(FCodeFeatureArrayCell*)SizeMalloc(sizeof(FCodeFeatureArrayCell)) SizeFree(junk, sizeof(FCodeFeatureArrayCell)) FCodeFeatureArray_p FCodeFeatureArrayAlloc(Sig_p sig, ClauseSet_p axioms)`
- `void FCodeFeatureArrayFree(FCodeFeatureArray_p junk)`
- `void FCodeFeatureArraySort(FCodeFeatureArray_p array)`
- `void FCodeFeatureArrayUpdateOccKey(FCodeFeatureArray_p array, OrderParms_p oparms)`
- `void FCodeFeatureArrayUpdateSymbKey(FCodeFeatureArray_p array, Sig_p sig, OrderParms_p oparms)`

## Implementation Notes

### Internal Functions

- `feature_compare_function`

### Source-Level Behavior

- `feature_compare_function`: Compare two featuresortcells and return <0, =0, >0 as for strcmp().
- `FCodeFeatureArrayAlloc`: Allocate an initialized FCodeFeature array.
- `FCodeFeatureArrayUpdateOccKey`: Update key0 based on the occurrence of the symbols in axioms, conjectures, or both.
- `FCodeFeatureArrayUpdateSymbKey`: Update key0 based on the occurrence of the symbols in axioms, conjectures, or both.
- `FCodeFeatureArrayFree`: Free an FCodeFeatureArray.
- `FCodeFeatureArraySort`: Sort an array according to feature_compare_function()

### Dependencies

- `"che_fcode_featurearrays.h"`
- `<che_clausesetfeatures.h>`
- `<che_to_params.h>`
- `<clb_simple_stuff.h>`
- `<stdlib.h>`

### Compile-Time Conditions

- `CHE_F_CODE_FEATUREARRAYS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_fcode_featurearrays.h`, `HEURISTICS/che_fcode_featurearrays.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 343 lines, 9 scanned public declarations, 1 scanned internal function definitions, and 6 structured function-comment blocks.
- Sortable arrays associating a function symbol with a number of integer feature values (that define the order). Used by precedence generating functions, now also for weights. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
