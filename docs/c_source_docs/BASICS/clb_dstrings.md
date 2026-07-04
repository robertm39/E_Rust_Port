<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_dstrings

## Source Files

- [BASICS/clb_dstrings.h](../../../eprover/BASICS/clb_dstrings.h)
- [BASICS/clb_dstrings.c](../../../eprover/BASICS/clb_dstrings.c)

## Purpose

Declarations for dynamic, arbitrary length strings (i.e. 0-terminated arrays of characters). The conversion between DStrs and C-strings is as simple and efficient as possible. This implementation is optimized for strings with a certain behaviour,

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `DStrCell`
- `DStr_p`

### Macros And Constants

- `CLB_DSTRINGS`
- `DSTRGETS_CHUNK`
- `DSTRGROW`
- `DStrAppendDStr(strdes, str)`
- `DStrCellAlloc()`
- `DStrCellFree(junk)`
- `DStrGetRef(strdes)`
- `DStrLastChar(strdes)`
- `DStrReleaseRef(strdes)`

### Globals

- `extern char NullStr[]`

### Exported Functions

- `DStrAppendStr((strdes), DStrView(str)) char DStrDeleteLastChar(DStr_p strdes)`
- `DStr_p DStrAlloc(void)`
- `char* DStrAddress(DStr_p strdes, int index)`
- `char* DStrAppendBuffer(DStr_p strdes, char* buf, int len)`
- `char* DStrAppendChar(DStr_p strdes, char newch)`
- `char* DStrAppendInt(DStr_p strdes, long newpart)`
- `char* DStrAppendStr(DStr_p strdes, const char* newpart)`
- `char* DStrAppendStrArray(DStr_p strdes, char* array[], char* separator)`
- `char* DStrCopy(DStr_p strdes)`
- `char* DStrCopyCore(DStr_p strdes)`
- `char* DStrFGetS(DStr_p strdes, FILE* fp)`
- `char* DStrSet(DStr_p strdes, char* string)`
- `char* DStrView(DStr_p strdes)`
- `long DStrLen(DStr_p strdes)`
- `void DStrFree(DStr_p junk)`
- `void DStrMinimize(DStr_p strdes)`
- `void DStrReset(DStr_p strdes)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `DStrAlloc`: Return a pointer to an initialized DStrCell.
- `DStrFree`: Decrease the reference counter. If it is equal to 0, free both the DStr-Cell and the contained string.
- `DStrAppendStr`: Append a C-String to a DStr efficiently
- `DStrAppendChar`: Append a single character to a DStr. This is the operation that will probably be called with the highest frequency, so I try to make it efficient.
- `DStrAppendBuffer`: Append a (not necessarily 0-terminated) buffer to the end of a DStr.
- `DStrAppendInt`: Append the string representation of a long number to a DStr.
- `DStrAppendStrArray`: Append the elements of the NULL terminated array to str as a separator separated list.
- `DStrDeleteLastChar`: If String is not empty, delete last character and return is. Otherwise return '\0'.
- `DStrView`: Return a pointer to the stored C-string. This is guaranteed to stay fresh as long as no other DStr-Operation is performed on the string. The user is responsible for the use of this pointer - in particular, write-operations on the string should not change the lenght of the string, or it will become corrupted!
- `DStrAddress`: Return the address of the given character in the DStr, or 0 if the string has less than index+1 chars.
- `DStrCopy`: Return a pointer to a copy of the stored string. The user is responsible for freeing the memory (via free()/FREE()).
- `DStrCopyCore`: Return a pointer to a copy of the stored string without the first and last character (this is useful for stripping quotes off string literals). The user is responsible for freeing the memory (via free()/FREE()). Fails if string has less than two characters.
- `DStrSet`: Set a dstring to a given C-String.
- `DStrLen`: Return the length of a stored string (efficiency hack...)
- `DStrReset`: Set the string to "" efficiently (does _not_ change internal memory - call this to e.g. reinitialize a string in a loop)
- `DStrMinimize`: Minimize the space used to store the string in strdes.
- `DStrFGetS`: fgets() analog for arbitray lenght lines. strdes is reset first. Returns char* pointer to result or NULL if EOF is encountered before any characters are read.

### Dependencies

- `"clb_dstrings.h"`
- `<clb_memory.h>`

### Compile-Time Conditions

- `CLB_DSTRINGS`

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

Source files reviewed: `BASICS/clb_dstrings.h`, `BASICS/clb_dstrings.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 636 lines, 20 scanned public declarations, 0 scanned internal function definitions, and 17 structured function-comment blocks.
- Declarations for dynamic, arbitrary length strings (i.e. 0-terminated arrays of characters). The conversion between DStrs and C-strings is as simple and efficient as possible. This implementation is optimized for strings with a certain behaviour,
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `DStrAlloc` initializes `refs` to 1. `DStrGetRef(NULL)` and `DStrReleaseRef(NULL)` are no-ops, while `DStrFree` asserts a non-NULL descriptor with `refs >= 1`, decrements the count, and frees the descriptor and backing buffer at zero. Rust keeps the counter surface explicit and returns whether C would have freed the descriptor on release.
- `DStrAppendStr` measures `newpart` with `strlen` and appends with `strcat`, so embedded NUL bytes truncate the appended content. Rust preserves this C-string boundary for `append_str` and exposes a byte-oriented C-string helper for non-UTF-8 external data.
- `DStrAppendDStr` is a macro over `DStrView(str)` and `DStrAppendStr`, so it appends the source descriptor's current C-string prefix rather than its full stored byte length if embedded NUL bytes are present. Rust keeps this as an explicit DStr-to-DStr compatibility helper.
- `DStrSet` resets the descriptor without shrinking and then delegates to `DStrAppendStr`, so it accepts raw C-string bytes, reuses existing allocation where possible, and truncates at the first embedded NUL. Rust preserves this through string and byte-oriented set helpers.
- `DStrAppendBuffer` takes a signed `int len` and uses a plain `for(i=0; i<len; i++)` loop, so zero and negative lengths append nothing. Rust keeps that loop surface in a signed compatibility helper while checking that the requested prefix fits the provided slice.
- `DStrAppendStrArray` consumes a NULL-terminated `char*[]`, appends separators only between entries before the first NULL, and leaves the descriptor untouched when the first entry is NULL. Rust keeps this sentinel-array behavior in an explicit compatibility helper.
- `DStrDeleteLastChar` returns `'\0'` for an empty descriptor, but asserts that a removed byte from a non-empty descriptor is non-NUL. Rust preserves that assertion when binary append paths place a NUL at the logical end.
- `DStrAddress` checks only `index > len`, so `index == len` returns the address of the trailing NUL for allocated strings. Rust preserves this as `Some(0)` for allocated buffers while keeping a never-allocated empty string as no address.
- `DStrCopy` allocates from the descriptor's logical `len`, then copies with `strcpy`, so embedded NUL bytes truncate the returned C string. `DStrCopyCore` uses the logical interior length but still returns a NUL-terminated string whose visible content stops at the first interior NUL. Rust copy helpers preserve the C-string-visible prefix.
- `DStrCopyCore` asserts that the descriptor has allocated string data and that `len >= 2` before copying the interior bytes. Rust preserves this as a panic on strings shorter than two bytes rather than returning an optional value.
- `DStrMinimize` only acts when the descriptor has allocated string storage. A never-allocated empty descriptor stays unallocated, but an allocated empty descriptor, such as one reset after appending, is reallocated to a one-byte trailing-NUL buffer. Rust preserves this allocation-state distinction.
- `DStrFGetS` reads with `fgets(buffer, 256, fp)`, so each iteration consumes at most 255 bytes, appends only the C-string prefix before any embedded NUL, and decides whether to continue from the descriptor's last appended character. Rust preserves this chunked C-string behavior for the dynamic-string line reader.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `DStrGetRef` and `DStrFree` implement pointer-alias lifetime management inside the string descriptor. Rust compatibility helpers expose the counter and final-release event, but a future cleaned API should use `Rc`/arena handles or owned string values instead of asking callers to remember that a final release means the C descriptor would be invalid.
- `DStrAddress` accepts a signed `int` and checks only `index > len`; negative indices can produce a pointer before the string buffer in C. Rust intentionally does not expose negative indexes. If a byte-for-byte compatibility test ever exercises this, decide whether to reject it at the parser/caller boundary or add a narrowly named compatibility helper.
- `DStrAppendDStr` can be invoked with aliased descriptors in C because it is only a macro, but self-append through `DStrAppendStr(strdes, DStrView(strdes))` can interact badly with reallocating the backing buffer. Rust's explicit helper requires distinct borrows; add a deliberately named self-append operation only if reference traces prove the C alias case is observable and required.
- `DStrAppendStr`, `DStrSet`, `DStrAppendDStr`, and separator handling in `DStrAppendStrArray` all inherit C-string truncation at the first embedded NUL. Rust preserves this for drop-in compatibility, but cleaned high-level APIs should reject embedded NULs explicitly or route binary payloads through length-based buffer helpers.
- `DStrFGetS` can merge later input into the returned line when an earlier `fgets` chunk contains an embedded NUL before its newline, because the loop checks the last appended byte rather than the consumed buffer. A cleaned line API should either reject NUL-bearing text input or make binary line handling length-based.
- `DStrAppendBuffer` trusts its raw pointer/length pair and can read past the supplied buffer when `len` is too large. Rust reports that as an invariant failure at the slice boundary; a cleaned API should keep accepting ordinary slices instead of raw pointer/length pairs.
- `DStrCopy` and `DStrCopyCore` mix logical DStr lengths with C-string copy operations. A cleaned API should separate full byte-buffer copies from text/string copies that reject or truncate at embedded NULs by design.
- `DStrMinimize` preserves whether an empty descriptor has ever allocated storage, because that affects whether `DStrAddress(str, 0)` can return the trailing NUL slot. A cleaned API should make this allocation-state observability explicit or remove it from higher-level callers.
- `DStrAppendStrArray` inherits C's sentinel-array convention. Rust callers that already own a slice or iterator should prefer the non-sentinel helper unless they are preserving an original C call boundary.
- `DStrView` exposes a mutable C buffer pointer and relies on callers not changing its length. Rust keeps immutable byte/string views; later APIs that need writable buffer access should make length preservation explicit.
- `DStrCopyCore` is specialized for quoted strings but only checks length, not matching quote delimiters. A later high-level string-literal API should validate delimiters before stripping them.
<!-- END MANUAL REVIEW: c_source_docs -->
