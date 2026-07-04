<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_defines

## Source Files

- [BASICS/clb_defines.h](../../../eprover/BASICS/clb_defines.h)

## Purpose

Basic definition useful (very nearly) everywhere. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ComparisonFunctionType`
- `GenericExitFun`
- `IntOrP`

### Macros And Constants

- `ABS(x)`
- `BOOL2STR(val)`
- `CLB_DEFINES`
- `CMP(x,y)`
- `COMCHAR`
- `COMCHARRAW`
- `EQUIV(x,y)`
- `GCC_DIAGNOSTIC_POP`
- `GCC_DIAGNOSTIC_PUSH`
- `INTORP_MEM`
- `KILO`
- `LFHO(x)`
- `LIKELY(x)`
- `LONG_MEM`
- `MAX(x,y)`
- `MEGA`
- `MIN(x,y)`
- `SWAP(x,y)`
- `TSTPOUT(file,msg)`
- `TSTPOUTFD(fd,msg)`
- `UNLIKELY(x)`
- `UNUSED(x)`
- `XOR(x,y)`

### Globals

- None found in the source scan.

### Exported Functions

- `static inline size_t WriteStr(int fd, const char* msg)`

## Implementation Notes

### Internal Functions

- `WriteStr`

### Source-Level Behavior

- `WriteStr`: Computes the length of msg and writes msg to the file descriptor. WriteStr is used for output instead of the print functions in low memory situations since the later may try to allocate memory which is likely to fail. WriteStr is defined as a function instead of a macro to silence warnings in case the return value of write is unused. The function write may...

### Dependencies

- `<assert.h>`
- `<errno.h>`
- `<inttypes.h>`
- `<math.h>`
- `<stdbool.h>`
- `<stddef.h>`
- `<stdio.h>`
- `<stdlib.h>`
- `<string.h>`
- `<sys/param.h>`
- `<unistd.h>`

### Compile-Time Conditions

- `CLB_DEFINES`
- `CONSTANT_MEM_ESTIMATE`
- `ENABLE_LFHO`
- `PRINT_TSTP_STATUS`
- `UNIX_COMMENTS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_defines.h`.

### Review Notes

- Reviewed as a standalone header unit in `BASICS` covering 1 source file(s), about 181 lines, 4 scanned public declarations, 1 scanned internal function definitions, and 1 structured function-comment blocks.
- Basic definition useful (very nearly) everywhere. the GNU Lesser General Public License.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `COMCHAR` is compile-time selected: the reference WSL/Linux build used by `e-interop` does not define `UNIX_COMMENTS`, so C prints `%`-prefixed status, proof, resource, and statistics comments. Rust now uses that reference default for supported executable comment output; if a `UNIX_COMMENTS` C build becomes a supported target, expose the comment prefix as a deliberate compatibility mode rather than scattering literals through output call sites.
- `ABS(x)` uses the plain expression `((x)>0?(x):-(x))`, so ordinary signed values map to their positive magnitude but `LONG_MIN` would overflow in C. Rust keeps the helper for ordinary `long` values and panics on the minimum signed value instead of importing undefined behavior.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- In the default build `COMCHAR` is the printf-escaped string `%%`, while `COMCHARRAW` is `%`. Most call sites pass `COMCHAR` through `fprintf` and produce one percent sign, but direct `WriteStr` call sites such as hard CPU-timeout reporting emit the doubled `%%` literally. Rust preserves both spellings for compatibility; after drop-in behavior is secured, consider replacing this with separate explicit formatted-output and direct-output comment-prefix constants.
- `IntOrP` is a raw C union of `long` and `void*`, so generic containers can silently reinterpret the same word as either an integer tag or a pointer. Rust intentionally keeps a tagged enum at compatibility boundaries; after drop-in behavior is secured, prefer typed payload enums for each queue/tree/list role instead of carrying a generic union-shaped abstraction.
- `MAX`, `MIN`, `CMP`, and `SWAP` depend on GNU statement expressions and `__typeof__` to evaluate arguments once. They avoid some macro side effects but still make control flow and type conversions invisible at call sites. Rust should keep explicit helper functions where useful and eventually replace broad macro-style use with local `Ord`/`max`/`min` code in non-compatibility layers.
- `ABS` is not protected by the GNU statement-expression pattern, so an argument with side effects can be evaluated twice, and `LONG_MIN` negation overflows. Prefer a normal function for Rust call sites and keep the panic-on-minimum compatibility helper isolated where C macro behavior is being modeled.

<!-- END MANUAL REVIEW: c_source_docs -->
