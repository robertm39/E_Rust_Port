<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_os_wrapper

## Source Files

- [BASICS/clb_os_wrapper.h](../../../eprover/BASICS/clb_os_wrapper.h)
- [BASICS/clb_os_wrapper.c](../../../eprover/BASICS/clb_os_wrapper.c)

## Purpose

Functions wrapping some OS functions in a convenient manner. the GNU Lesser General Public License. <1> New

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RLimResult`

### Macros And Constants

- `CLB_OS_WRAPPERS`
- `GETTIME`
- `GetMSecTime()`
- `GetSecTime()`
- `GetSecTimeMod()`
- `INCREASE_STACK_SIZE`
- `MEM_PHRASE`
- `PERF_CTR_DECL(name)`
- `PERF_CTR_DEFINE(name)`
- `PERF_CTR_ENTRY(name)`
- `PERF_CTR_EXIT(name)`
- `PERF_CTR_PRINT(out, name)`
- `PERF_CTR_RESET(name)`

### Globals

- None found in the source scan.

### Exported Functions

- `FILE* SecureFOpen(char* name, char* mode)`
- `RLimResult SetSoftRlimit(int resource, rlim_t limit)`
- `int GetCoreNumber(void)`
- `long GetSystemPageSize(void)`
- `long long GetSystemPhysMemory(void)`
- `long long GetUSecClock(void)`
- `long long GetUSecTime(void)`
- `rlim_t GetSoftRlimit(int resource)`
- `void SecureFClose(FILE* fp)`
- `void SetMemoryLimit(rlim_t mem_limit)`
- `void SetSoftRlimitErr(int resource, rlim_t limit, char* desc)`
- `void StrideMemory(char* mem, long size)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `SetSoftRlimit`: Set a soft limit to the given value (or as close as the hard limit allows). Return result. If get- or setrlimit() fail, errno will contain the corresponding cause.
- `SetSoftRlimitErr`: Try to set a soft limit to the given value. Print a warning if it has to be reduced, terminate with a proper system error if it fails. If desc is provided, use it for messages.
- `SetMemoryLimit`: Set memory limit to the given limit (if any), or the largest possible.
- `GetSoftRlimit`: Return the soft limit for the given resource, or 0 on failure.
- `IncreaseMaxStackSize`: Try to increase the maximum stack size, then reexec the process to work under the new limit. At least on some UNIXES, maximum stack size cannot increase after the process has started).
- `GetUSecTime`: Return the time in microseconds since the epoch.
- `GetUSecClock`: Return the process cpu time in microseconds.
- `GetCoreNumber`: Return the number of cores (via sysconf(_SC_NPROCESSORS_ONLN). Returns 1 on failure (and prints a warning), for safe continuation.
- `GetSystemPageSize`: Find and return the system page size (in bytes), if possible. Return -1 otherwise.
- `GetSystemPhysMemory`: Try to find the phyical memory installed in the machine. Return it (in MB) or -1 if no information can be obtained.
- `StrideMemory`: Write an arbitrary value into memory each E_PAGE_SIZE bytes. It's used for preallocated memory reserves. Normally, allocated pages need not really be available unless written to if overallocation is being used. This should ensure that allocated pages are backed by real memory in such (broken!) cases.
- `SecureFOpen`: As fopen(), but terminate with a useful error message on failure.
- `SecureFClose`: As fclose(), but print a warning on error.

### Dependencies

- `"clb_error.h"`
- `"clb_os_wrapper.h"`
- `<assert.h>`
- `<sys/resource.h>`
- `<sys/time.h>`
- `<sys/types.h>`
- `<time.h>`
- `<unistd.h>`

### Compile-Time Conditions

- `CLB_OS_WRAPPERS`
- `INSTRUMENT_PERF_CTR`
- `PROFILE_WALL_CLOCK`
- `RLIMIT_AS`
- `_SC_PHYS_PAGES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_os_wrapper.h`, `BASICS/clb_os_wrapper.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 646 lines, 13 scanned public declarations, 0 scanned internal function definitions, and 13 structured function-comment blocks.
- OS abstraction layer for resource limits, time, memory, and process interaction; Windows/Rust portability needs explicit compatibility decisions here.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- Rust may use narrowly scoped unsafe external DLL/shared-library interop for this unit when a native platform API is required; keep those calls behind safe wrappers and document pointer/initialization invariants at the boundary.
- The C performance-counter macros expand to process-global microsecond accumulators only when `INSTRUMENT_PERF_CTR` is defined. Rust maps that debug surface to the non-default `instrument-perf-ctr` Cargo feature in `src/basics/perf_counters.rs`, with same-shaped `% PC(...)` statistics lines for represented call sites.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `SetMemoryLimit` labels the second branch as `RLIMIT_AS` when that macro is present, but the C call still passes `RLIMIT_DATA`. Rust now mirrors that duplicated `RLIMIT_DATA` behavior on Linux; after reference tests cover memory-limit handling, decide whether this is a typo to fix or a platform-specific compatibility quirk to keep.
- `SetSoftRlimitErr` suppresses failed `RLIMIT_DATA` warnings, warns when limits are reduced, and includes `strerror(errno)` for unmasked failures. Rust now carries the errno code from the Linux `getrlimit`/`setrlimit` boundary to the executable warning layer, formats it through `strerror`, and preserves the failed-`RLIMIT_DATA` mask.
- Native Windows has no POSIX `setrlimit` equivalent. Rust maps `SetMemoryLimit`-style process memory limits to a retained Kernel32 Job Object with `JOB_OBJECT_LIMIT_PROCESS_MEMORY`; this can fail when the current process cannot be assigned to a new job or an outer job policy forbids the limit. Rust deliberately does not apply `JOB_OBJECT_LIMIT_PROCESS_TIME` for `--cpu-limit`, because Windows terminates the process with `STATUS_QUOTA_EXCEEDED` before the C-shaped hard-timeout banner, SZS status, stderr diagnostic, and exit code can be produced. Treat both behaviors as platform boundaries rather than C behavior, and revisit only after Windows reference tests cover expected diagnostics.
- Rust's Linux resource-usage path now uses a narrow `getrusage` boundary for the C-shaped resource footer, with `/proc/self/stat` and `/proc/self/status` retained only as fallback. Exact target units for maximum resident set size remain a compatibility detail to keep visible in output tests.
- Rust's Linux `GetUSecClock` path now uses C `clock()` semantics for process CPU time. Unsupported targets still fall back to a monotonic process-relative wall clock, which should remain documented as a portability fallback rather than exact C behavior.
- Rust's Linux `GetCoreNumber`, `GetSystemPageSize`, and `GetSystemPhysMemory` paths now prefer C-shaped `sysconf` queries, with safe Rust or `/proc` fallbacks if those calls fail. The hard-coded Linux/glibc selector values should be revisited if non-glibc Linux targets become supported.
- `PERF_CTR_ENTRY` stores one start timestamp slot per counter name and `PERF_CTR_EXIT` consumes that slot, so nested or overlapping entries for the same C counter can overwrite the earlier start time. Rust's `instrument-perf-ctr` guards accumulate elapsed spans with RAII and atomics for represented call sites instead; revisit only if byte-level compatibility with the single-slot overwrite behavior becomes observable, and keep adding missing guards as the remaining proof-search owners are ported.
<!-- END MANUAL REVIEW: c_source_docs -->
