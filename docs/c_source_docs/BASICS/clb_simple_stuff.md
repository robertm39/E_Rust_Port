<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_simple_stuff

## Source Files

- [BASICS/clb_simple_stuff.h](../../../eprover/BASICS/clb_simple_stuff.h)
- [BASICS/clb_simple_stuff.c](../../../eprover/BASICS/clb_simple_stuff.c)

## Purpose

Useful routines, usually pretty trivial. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ProblemType`
- `ProverResult`
- `RandStateCell`
- `RandState_p`
- `WeightedObjectCell`
- `WeightedObject_p`

### Macros And Constants

- `CLB_SIMPLE_STUFF`
- `DBG_PRINT(out, prefix, main, suffix)`
- `DBG_TPRINT(out, prefix, term, suffix)`
- `MAXINDENTSPACES`
- `WeightedObjectArrayAlloc(number)`
- `WeightedObjectArrayFree(array)`
- `WeightedObjectArraySort(array, size)`

### Globals

- `extern ProblemType problemType`

### Exported Functions

- `SecureMalloc(number * sizeof(WeightedObjectCell)) int WeightedObjectCompareFun(WeightedObject_p o1, WeightedObject_p o2)`
- `bool StringStartsWith(const char* pattern, const char* prefix)`
- `char* IndentStr(int level)`
- `double JKISSRandDouble(RandState_p state)`
- `int StrDistance(const char* a, const char* b)`
- `int StringIndex(char* key, char* list[])`
- `long ComputeGCD(long a, long b)`
- `long StringArrayCardinality(char *array[])`
- `qsort(array, size, sizeof(WeightedObjectCell),\ (ComparisonFunctionType)WeightedObjectCompareFun) void JKISSSeed(RandState_p state, int seed1, int seed2, int seed3)`
- `unsigned JKISSRand(RandState_p state)`
- `void SetProblemType(ProblemType t)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `StrDistance`: Compute distance of two strings (number of different characters, plus difference in length.
- `WeightedObjectCompareFun`: Compare the weight of two weighted objects.
- `JKISSSeed`: Initialize the portable KISS random number generator.
- `JKISSRand`: Improved "Keep It Simple, Stupid" RNG generator, adapted from the public domain version by Davida Jones (d.jones@cs.ucl.ac.uk).
- `JKISSRandDouble`: Returns a pseudo-random number r with 0 <= r < 1.
- `IndentStr`: Return a pointer to a string of level spaces, or MAXINDENTSPACES if this is smaller. Not reentrant.
- `StringStartsWith`: Determines if string pattern starts with string prefix.
- `StringIndex`: Given a NULL-Terminated array of strings, return the index of key (or -1 if key does not occur in the array).
- `StringArrayCardinality`: Return the number of initial non-NULL entries in a NULL-terminated string array.
- `ComputeGCD`: Compute the Greatest Common Divisor of two (positive) longs. Returns 0 if both are 0 or one is negative.
- `SetProblemType`: If user tries to overried the problem type the error is reported.

### Dependencies

- `"clb_simple_stuff.h"`
- `<clb_error.h>`
- `<pthread.h>`
- `<semaphore.h>`
- `<string.h>`
- `<unistd.h>`

### Compile-Time Conditions

- `CLB_SIMPLE_STUFF`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_simple_stuff.h`, `BASICS/clb_simple_stuff.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 496 lines, 18 scanned public declarations, 0 scanned internal function definitions, and 11 structured function-comment blocks.
- Useful routines, usually pretty trivial. the GNU Lesser General Public License.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Compatibility Notes

- C stores `problemType` as process-global state and parser paths set it as first-order or higher-order syntax is observed. Rust now sets the same global for supported first-order executable parsing so lower-level ordering, indexing, and inference helpers see C-shaped state during a run.

### Change Later Candidates

- `problemType` is convenient C global state but awkward for repeated in-process Rust runs and future parallel solving. Rust resets it around executable `run()` calls for isolation; replace that shim with an explicit proof-session/parser context once the full first-order and higher-order parser owners are wired.
<!-- END MANUAL REVIEW: c_source_docs -->
