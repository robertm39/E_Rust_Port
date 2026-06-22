<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_axfilter

## Source Files

- [HEURISTICS/che_axfilter.h](../../../eprover/HEURISTICS/che_axfilter.h)
- [HEURISTICS/che_axfilter.c](../../../eprover/HEURISTICS/che_axfilter.c)

## Purpose

Definitions dealing with the description of axiom set filters based on relevancy/SinE principles. This only deals with their parameters and specifications. The real code is (for now) in CONTROL and knows nothing about this ;-).

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `AxFilterCell`
- `AxFilterSetCell`
- `AxFilterSet_p`
- `AxFilterType`
- `AxFilter_p`
- `GeneralityMeasure`

### Macros And Constants

- `AxFilterCellAlloc()`
- `AxFilterCellFree(junk)`
- `AxFilterSetAddFilter(s, f)`
- `AxFilterSetCellAlloc()`
- `AxFilterSetCellFree(junk)`
- `AxFilterSetElements(s)`
- `AxFilterSetGetFilter(s, i)`
- `CHE_AXFILTER`

### Globals

- `extern char* AxFilterDefaultSet`

### Exported Functions

- `AxFilterSet_p AxFilterSetAlloc(void)`
- `AxFilterSet_p AxFilterSetCreateInternal(char* str)`
- `AxFilter_p AxFilterAlloc(void)`
- `AxFilter_p AxFilterDefParse(Scanner_p in)`
- `AxFilter_p AxFilterParse(Scanner_p in)`
- `AxFilter_p AxFilterSetFindFilter(AxFilterSet_p set, char* name)`
- `bool AxFilterPrintBuf(char* buf, int buflen, AxFilter_p filter)`
- `long AxFilterSetParse(Scanner_p in, AxFilterSet_p set)`
- `void AxFilterDefPrint(FILE* out, AxFilter_p filter)`
- `void AxFilterFree(AxFilter_p junk)`
- `void AxFilterPrint(FILE* out, AxFilter_p filter)`
- `void AxFilterSetAddNames(DStr_p res, AxFilterSet_p filters)`
- `void AxFilterSetFree(AxFilterSet_p junk)`
- `void AxFilterSetPrint(FILE* out, AxFilterSet_p set)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `get_gen_measure`: Given a string, return the corresponding GenMeasure, or 0 on failure.
- `AxFilterAlloc`: Allocate an empty, initialized axiom filter description.
- `AxFilterFree`: Free an axiom filter description.
- `GSinEParse`: Parse an Axiom Filter description into a newly allocated cell. The preliminary syntax is: GSinE(<g-measure:type>, <[no]hypos>,<benvolvence:double>, <generosity:int>, <rec-depth:int>, <set-size:int>, <set-fraction:double>) where "GSinE" represents AFGSineE, "Generalized SinE", currently the only support filter type. Other filter types can support different f...
- `ThresholdParse`: Parse an Threshold filter The preliminary syntax is: Threshold(<threshold:int>)
- `LambdaDefParse`: Parse an LambdaDef filter: has no arguments
- `AxFilterParse`: Parse an AxFilter and return it.
- `AxFilterDefParse`: Parse an AxFilterDefinition of the form [name=]<def>, where "name" is an Identifier, and <def> is an axiom filter definition. If the optional part is missing, an automatically generated name of the form "axfilter_auto%4udd" is generated. This name is unique among auto-generated names (up to the period of unsigned long, but not checked against manually given...
- `AxFilterPrintBuf`: Print an axiom filter specification into a buffer. Return true on success, false if the buffer is too small.
- `AxFilterPrint`: Print an axiom filter specification.
- `AxFilterDefPrint`: Print an axiom filter defintion
- `AxFilterSetAlloc`: Allocate an empy AxFilterSet.
- `AxFilterSetFree`: Free an axion filter set (including the filters).
- `AxFilterSetParse`: Parse a set of axfilter definitions. Returns number of filters parsed.
- `AxFilterSetCreateInternal`: Create and return an AxFilterSet from a provided string description.
- `AxFilterSetPrint`: Print a set of axfilter definitions.
- `AxFilterSetFindFilter`: Given a name, return the filter (or NULL).
- `AxFilterSetAddNames`: Add the names of all filters in the set to the provided DStr.

### Dependencies

- `"che_axfilter.h"`
- `<cio_basicparser.h>`
- `<clb_simple_stuff.h>`

### Compile-Time Conditions

- `CHE_AXFILTER`
- `_symbols_in_drel`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_axfilter.h`, `HEURISTICS/che_axfilter.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 823 lines, 21 scanned public declarations, 0 scanned internal function definitions, and 18 structured function-comment blocks.
- Definitions dealing with the description of axiom set filters based on relevancy/SinE principles. This only deals with their parameters and specifications. The real code is (for now) in CONTROL and knows nothing about this ;-).
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
