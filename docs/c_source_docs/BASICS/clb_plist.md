<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_plist

## Source Files

- [BASICS/clb_plist.h](../../../eprover/BASICS/clb_plist.h)
- [BASICS/clb_plist.c](../../../eprover/BASICS/clb_plist.c)

## Purpose

Doubly linked lists of pointers and integers. the GNU Lesser General Public License. <1> Mon Jul 20 02:26:17 MET DST 1998 New

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PListCell`
- `PList_p`

### Macros And Constants

- `CLB_PLIST`
- `PListCellAlloc()`
- `PListCellFree(junk)`
- `PListEmpty(anchor)`

### Globals

- None found in the source scan.

### Exported Functions

- `PList_p PListAlloc(void)`
- `PList_p PListExtract(PList_p element)`
- `void PListDelete(PList_p element)`
- `void PListFree(PList_p junk)`
- `void PListInsert(PList_p where, PList_p cell)`
- `void PListStore(PList_p where, IntOrP val)`
- `void PListStoreInt(PList_p where, long val)`
- `void PListStoreP(PList_p where, void* val)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PListAlloc`: Allocate an empty PList.
- `PListFree`: Free a PList.
- `PListInsert`: Insert a PListCell into a list after where.
- `PListStore`: Store a given value in a PList.
- `PListStoreP`: Store a pointer in a PList
- `PListStoreInt`: Store an integer in a PList
- `PListExtract`: Extract a PListCell from a list
- `PListDelete`: Delete an entry from a PList.

### Dependencies

- `"clb_plist.h"`
- `<clb_memory.h>`

### Compile-Time Conditions

- `CLB_PLIST`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_plist.h`, `BASICS/clb_plist.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 301 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 8 structured function-comment blocks.
- Doubly linked lists of pointers and integers. the GNU Lesser General Public License. <1> Mon Jul 20 02:26:17 MET DST 1998 New
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change-Later Candidates

- `PListExtract` asserts that the element is a linked, non-anchor cell, while `PListInsert` trusts callers not to pass an already linked cell or an uninitialized allocation. Rust preserves the asserting extraction precondition and guards insertion, but a cleaned API should make detached-cell ownership explicit or fold insertion through checked store/move operations.
<!-- END MANUAL REVIEW: c_source_docs -->
