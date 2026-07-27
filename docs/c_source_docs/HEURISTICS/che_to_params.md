<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_to_params

## Source Files

- [HEURISTICS/che_to_params.h](../../../eprover/HEURISTICS/che_to_params.h)
- [HEURISTICS/che_to_params.c](../../../eprover/HEURISTICS/che_to_params.c)

## Purpose

Data types and auxilliary functions for describing orderig parameters. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz, Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OrderParmsCell`
- `OrderParms_p`
- `TOPrecGenMethod`
- `TOWeightGenMethod`
- `WCombFrequencyRank`
- `WInvCombFrequencyCount`
- `WInvCombFrequencyRank`
- `WInvTypeFrequencyCount`
- `WInvTypeFrequencyRank`
- `however`

### Macros And Constants

- `CHE_TO_PARAMS`
- `DEFAULT_DB_WEIGHT`
- `DEFAULT_LAMBDA_WEIGHT`
- `HOK2STR(x)`
- `OrderParmsCellAlloc()`
- `OrderParmsCellFree(junk)`
- `PARSE_BOOL(name)`
- `PARSE_IDENTIFIER(name)`
- `PARSE_IDENT_INTO(name, maxlen)`
- `PARSE_IDENT_NO(name, ids)`
- `PARSE_INT(name)`
- `PARSE_INTMAX(name)`
- `PARSE_INT_LIMITED(name, low, high)`
- `PARSE_STRING(name)`
- `PARSE_STRING_AND_CONVERT(name, converter)`
- `STR2HOK(val)`
- `TOGetPrecGenName(method)`
- `TOGetWeightGenName(method)`
- `WConstNoSpecialWeight`
- `WConstNoWeight`

### Globals

- `extern char* TOPrecGenNames[]`
- `extern char* TOWeightGenNames[]`

### Exported Functions

- `(OrderParmsCell*)SizeMalloc(sizeof(OrderParmsCell)) SizeFree(junk, sizeof(OrderParmsCell)) void OrderParmsInitialize(OrderParms_p handle)`
- `(TOWeightGenNames[(method)]) TOWeightGenMethod TOTranslateWeightGenMethod(char* name)`
- `TOPrecGenMethod TOTranslatePrecGenMethod(char* name)`
- `bool OrderParmsParseInto(Scanner_p in, OrderParms_p handle, bool warn_missing)`
- `void OrderParmsPrint(FILE* out, OrderParms_p handle)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `OrderParmsInitialize`: Initialize an ordering parameter cell with rational default values.
- `OrderParmsPrint`: Print the ordering parameters in Human/Machine-readable form.
- `OrderParmsParseInto`: Parse the OrderParram-Cell into/over the existing cell. Parameters are expected in-order, but may be missing. Returns true if all parameters have been found, false otherwise.

### Dependencies

- `"che_to_params.h"`
- `<clb_permastrings.h>`
- `<cto_ocb.h>`

### Compile-Time Conditions

- `CHE_TO_PARAMS`
- `ENABLE_LFHO`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for executable missing-field warnings on 2026-07-17.

Source files reviewed: `HEURISTICS/che_to_params.h`, `HEURISTICS/che_to_params.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 667 lines, 17 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Data types and auxilliary functions for describing orderig parameters. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `OrderParmsParseInto(..., true)` warns once for every absent field while preserving the existing value. Its `Warning("Config misses %s\n", name)` call embeds a newline before the common warning routine appends another, so the executable surface contains a blank line after each warning. Rust retains those warnings in its parse report and the strategy-file executable path now writes them before any later selected-strategy lookup. Normal search and later lookup failure are byte-exact against isolated C in [`experiments/2026-07-17-074-strategy-warning-output/FINDINGS.md`](../../../experiments/2026-07-17-074-strategy-warning-output/FINDINGS.md).

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
