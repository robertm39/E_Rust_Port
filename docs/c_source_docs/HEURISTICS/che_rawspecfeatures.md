<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_rawspecfeatures

## Source Files

- [HEURISTICS/che_rawspecfeatures.h](../../../eprover/HEURISTICS/che_rawspecfeatures.h)
- [HEURISTICS/che_rawspecfeatures.c](../../../eprover/HEURISTICS/che_rawspecfeatures.c)

## Purpose

Code and datatypes for handling rough classification of raw problem specs. <1> Tue May 22 01:10:30 CEST 2012 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RawSpecFeatureCell`
- `RawSpecFeature_p`

### Macros And Constants

- `ADJUST_FOR_HO(limit, scale)`
- `NUM_RAW_FEATURES`
- `RAWSPECFEATURES`
- `RAW_CLASSIFY(index, value, some, many, ho_scale_some, ho_scale_many)`
- `RAW_CLASS_SIZE`

### Globals

- None found in the source scan.

### Exported Functions

- `void RawSpecFeaturesClassify(RawSpecFeature_p features, SpecLimits_p limits, char* pattern)`
- `void RawSpecFeaturesCompute(RawSpecFeature_p features, ProofState_p state)`
- `void RawSpecFeaturesParse(Scanner_p in, RawSpecFeature_p features)`
- `void RawSpecFeaturesPrint(FILE* out, RawSpecFeature_p features)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `RawSpecFeaturesCompute`: Compute the raw features of state.
- `RawSpecFeaturesClassify`: Add a classifiction based on limits to the (initialized) features.
- `RawSpecFeaturesParse`: Parse a rawspecfeatures line.
- `RawSpecFeaturesPrint`: Print the features.

### Dependencies

- `"che_rawspecfeatures.h"`
- `<che_clausesetfeatures.h>`

### Compile-Time Conditions

- `RAWSPECFEATURES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_rawspecfeatures.h`, `HEURISTICS/che_rawspecfeatures.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 364 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Code and datatypes for handling rough classification of raw problem specs. <1> Tue May 22 01:10:30 CEST 2012 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `RawSpecFeaturesCompute` combines clause-set cardinality, clause standard weight, clause conjecture/hypothesis counts, and signature symbol counts with formula-set-only order and definition statistics. With no formula owners, C leaves `order` and `conj_order` at `1` and sets definition/lambda/app-var fields to empty defaults even if clause terms are higher-order; Rust now preserves that clause-only surface for represented proof states. Once formula sets are owned, revisit whether a cleaned classifier should expose clause-level order separately from the C raw-spec compatibility vector.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
