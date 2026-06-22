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

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
