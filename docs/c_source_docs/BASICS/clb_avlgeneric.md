<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_avlgeneric

## Source Files

- [BASICS/clb_avlgeneric.h](../../../eprover/BASICS/clb_avlgeneric.h)

## Purpose

Macros for the creation of generic binary tree functions. Currently used for traversal functions only. Please note that the name is obsolete (used for convenience only). The functions currently implemented are generic for binary search trees, and binary search trees in E are splay trees by now.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `AVL_TRAVERSE_DECLARATION(name,type)`
- `AVL_TRAVERSE_DEFINITION(name,type)`
- `CLB_AVLGENERIC`

### Globals

- None found in the source scan.

### Exported Functions

- `PStack_p name##TraverseInit(type root);\ type name##TraverseNext(PStack_p state)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `<clb_pstacks.h>`

### Compile-Time Conditions

- `CLB_AVLGENERIC`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->







<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_avlgeneric.h`.

### Review Notes

- Reviewed as a standalone header unit in `BASICS` covering 1 source file(s), about 136 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- Macros for the creation of generic binary tree functions. Currently used for traversal functions only. Please note that the name is obsolete (used for convenience only). The functions currently implemented are generic for binary search trees, and binary search trees in E are splay trees by now.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
