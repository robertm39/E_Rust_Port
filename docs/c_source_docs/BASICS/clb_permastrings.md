<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_permastrings

## Source Files

- [BASICS/clb_permastrings.h](../../../eprover/BASICS/clb_permastrings.h)
- [BASICS/clb_permastrings.c](../../../eprover/BASICS/clb_permastrings.c)

## Purpose

Simple registry for (potentially shared) permanent strings to simplify memory mangement.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLB_PERMASTRINGS`

### Globals

- None found in the source scan.

### Exported Functions

- `char* PermaString(char *str)`
- `char* PermaStringStore(char *str)`
- `void PermaStringsFree(void)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PermaString`: Register a string. Will return a pointer to a permanent (possibly shared) copy of the string that is valid until PermaStringsFree() is called.
- `PermaStringStore`: As PermaString, but will FREE the original.
- `PermaStringsFree`: Free all permastrings (and their admin data structure).

### Dependencies

- `"clb_permastrings.h"`
- `<clb_stringtrees.h>`

### Compile-Time Conditions

- `CLB_PERMASTRINGS`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_permastrings.h`, `BASICS/clb_permastrings.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 190 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Simple registry for (potentially shared) permanent strings to simplify memory mangement.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- C uses `PermaString`/`PermaStringStore` in only five parameter-parser fields: `sine`, `heuristic_name`, `heuristic_def`, `to_pre_prec`, and `to_pre_weights`. Those pointers survive scanner-token destruction and shallow `HeuristicParmsCell`/`OrderParmsCell` copies, but downstream code compares, parses, or prints their contents; no production caller compares their pointer identities.
- Rust gives those five production fields direct `String`/`Option<String>` ownership. Parameter cloning therefore preserves C's required lifetime without coupling proof-control and scheduling state to a process-global registry. The separate `PermaStringRegistry` remains the C-shaped exported helper: equal live entries share one `Arc<str>` allocation, owned insertion consumes its `String`, optional helpers preserve the null branches, and clearing starts a fresh identity epoch.
- C calls `PermaStringsFree` only during final `eprover` teardown after freeing parameter and definition owners. Rust relies on ordinary owner drop for production parameter strings; the explicit registry-clear helper applies only to callers that chose the compatibility registry.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `PermaString` always duplicates the input before insertion and asserts that the returned pointer is not the caller's pointer, even when the string is new. Rust returns owned `Arc<str>` handles, so pointer identity is stable for Rust holders but not allocator-address-compatible with C.
- `PermaStringStore` frees the caller-owned input even when the registry already contains the same string, and `PermaStringStore(NULL)` returns `NULL`. Rust models the owned optional shape with `maybe_perma_string_store`, but ordinary Rust callers should prefer owned strings or explicit interned handles rather than raw ownership transfer.
- `PermaStringsFree` invalidates every previously returned C pointer. Rust `Arc<str>` handles remain valid after registry clearing, which is safer for in-process tests but not exact dangling-pointer compatibility.
- The registry is a file-static `StrTree`, so lookup/insertion can reorganize by string key and cleanup is process-global. Rust uses a mutex-protected `BTreeSet`; exact C splay locality and allocator-address reuse are not modeled.
<!-- END MANUAL REVIEW: c_source_docs -->
