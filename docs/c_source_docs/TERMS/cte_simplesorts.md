<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_simplesorts

## Source Files

- [TERMS/cte_simplesorts.h](../../../eprover/TERMS/cte_simplesorts.h)
- [TERMS/cte_simplesorts.c](../../../eprover/TERMS/cte_simplesorts.c)

## Purpose

Data structure and function interfaces for managing simple, disjoint sorts. the GNU Lesser General Public License. <1> Sat Sep 15 01:33:52 EDT 2007

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `SortTableCell`
- `SortTable_p`
- `SortType`

### Macros And Constants

- `CTE_SIMPLESORTS`
- `STBool`
- `STIndividuals`
- `STInteger`
- `STKind`
- `STNoSort`
- `STPredefined`
- `STRational`
- `STReal`
- `SortIsInterpreted(sort)`
- `SortIsUserDefined(sort)`
- `SortTableCellAlloc()`
- `SortTableCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `SortTable_p DefaultSortTableAlloc(void)`
- `SortTable_p SortTableAlloc(void)`
- `SortType SortParseTSTP(Scanner_p in, SortTable_p table)`
- `SortType SortTableInsert(SortTable_p table, char* sort_name)`
- `char* SortTableGetRep(SortTable_p table, SortType sort)`
- `void SortPrintTSTP(FILE *out, SortTable_p table, SortType sort)`
- `void SortTableFree(SortTable_p junk)`
- `void SortTablePrint(FILE* out, SortTable_p table)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `default_sort_table_init`: Add the default types in proper order.
- `SortTableAlloc`: Allocate an empty but initialized sort table.
- `SortTableFree`: Free a SortTable.
- `SortTableInsert`: Add an entry (i.e. sort name) to the table (if unknown) and retrieve its encoding.
- `DefaultSortTableAlloc`: Allocate a sort table and insert the system-defined sorts in the proper order for their reserved names to work.
- `SortTableGetRep`: Given a sort, return a pointer to its external representation.
- `SortPrintTSTP`: Print a sort in the TSTP format
- `SortTablePrint`: Print a sort table (mainly for debugging)

### Dependencies

- `"cte_functypes.c"`
- `"cte_simplesorts.h"`
- `<cio_scanner.h>`
- `<clb_stringtrees.h>`

### Compile-Time Conditions

- `CTE_SIMPLESORTS`
- `STReal`

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

Source files reviewed: `TERMS/cte_simplesorts.h`, `TERMS/cte_simplesorts.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 379 lines, 11 scanned public declarations, 0 scanned internal function definitions, and 8 structured function-comment blocks.
- Data structure and function interfaces for managing simple, disjoint sorts. the GNU Lesser General Public License. <1> Sat Sep 15 01:33:52 EDT 2007
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
