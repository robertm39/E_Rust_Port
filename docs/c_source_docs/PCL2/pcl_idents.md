<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_idents

## Source Files

- [PCL2/pcl_idents.h](../../../eprover/PCL2/pcl_idents.h)
- [PCL2/pcl_idents.c](../../../eprover/PCL2/pcl_idents.c)

## Purpose

Identifiers for PCL2 - lists of posititive numbers. the GNU Lesser General Public License. <1> Wed Mar 22 19:32:20 MET 2000 New

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCLIdCell`
- `PCLId_p`

### Macros And Constants

- `NO_PCL_ID_ELEMENT`
- `PCLIdAlloc()`
- `PCLIdCellAlloc()`
- `PCLIdCellFree(junk)`
- `PCLIdFree(junk)`
- `PCLIdPrint(out, id)`
- `PCL_IDENTS`

### Globals

- None found in the source scan.

### Exported Functions

- `PCLId_p PCLIdParse(Scanner_p in)`
- `int PCLIdCompare(PCLId_p id1, PCLId_p id2)`
- `void PCLIdPrintFormatted(FILE* out, PCLId_p id, bool formatted)`
- `void PCLIdPrintTSTP(FILE* out, PCLId_p id)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PCLIdParse`: Parse a PCL-Identifier, i.e. a usually short list of pos-ints separated by spaces.
- `PCLIdPrintFormatted`: Print a PCL identifier.
- `PCLIdPrintTSTP`: Print a PCL identifier in a format suitable for TSTP. If a single number, print it, otherwise convert it to pclid<no1>_<no2>...
- `PCLIdCompare`: Compare two PCL identifiers lexicographically.

### Dependencies

- `"pcl_idents.h"`
- `<cio_scanner.h>`
- `<clb_pdarrays.h>`

### Compile-Time Conditions

- `PCL_IDENTS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_idents.h`, `PCL2/pcl_idents.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 256 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Identifiers for PCL2 - lists of posititive numbers. the GNU Lesser General Public License. <1> Wed Mar 22 19:32:20 MET 2000 New
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
