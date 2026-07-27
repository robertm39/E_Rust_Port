<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# BASICS / clb_error

## Source Files

- [BASICS/clb_error.h](../../../eprover/BASICS/clb_error.h)
- [BASICS/clb_error.c](../../../eprover/BASICS/clb_error.c)

## Purpose

Functions and datatypes for handling and reporting errors, warnings, and dealing with simple system stuff. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `BASICS`. Low-level infrastructure used by the rest of E: memory allocation, dynamic arrays, stacks, trees, strings, errors, dates, OS wrappers, and generic container support.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ErrorCodes`

### Macros And Constants

- `CLB_ERROR`
- `CPU_LIMIT_ERROR`
- `FILE_ERROR`
- `INCOMPLETE_PROOFSTATE`
- `INPUT_SEMANTIC_ERROR`
- `INTERFACE_ERROR`
- `LFHO_ASSERT(check)`
- `MAX_ERRMSG_ADD`
- `MAX_ERRMSG_LEN`
- `NO_ERROR`
- `OTHER_ERROR`
- `OUT_OF_MEMORY`
- `PARENT_REQUEST`
- `PROOF_FOUND`
- `RESOURCE_OUT`
- `SATISFIABLE`
- `SYNTAX_ERROR`
- `SYS_ERROR`
- `TYPE_ERROR`
- `USAGE_ERROR`
- `getrusage(a, b)`

### Globals

- `extern char ErrStr[]`
- `extern char* ProgName`
- `extern int TmpErrno`

### Exported Functions

- `bool TestLetterString(char* to_check, char* options)`
- `double GetTotalCPUTime(void)`
- `double GetTotalCPUTimeIncludingChildren(void)`
- `void CheckOptionLetterString(char* to_check, char* options, char *option)`
- `void ELog(char* message, ...)`
- `void Error(char* message, ErrorCodes ret, ...)`
- `void InitError(char* progname)`
- `void PrintRusage(FILE* out)`
- `void SysError(char* message, ErrorCodes ret, ...)`
- `void SysWarning(char* message, ...)`
- `void Warning(char* message, ...)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `InitError`: Initialize the error handling module. Currently only stores the name under which the program has been called. Copies only the pointer, not the pointed-to value!
- `Error`: Print an error message to stderr and exit the program with the given return code.
- `SysError`: Print a user error message and a system error message to stderr and exit the program with an appropriate return code. The value of errno is restored from TmpErrno.
- `Warning`: Print a warning to stderr
- `SysWarning`: Print a user error message and a system error message to stderr and exit the program with an appropriate return code. The value of errno is restored from TmpErrno.
- `ELog`: Write a message to a logfile.
- `GetTotalCPUTime`: Return the total CPU time use by the process s far, in floating point seconds - or -1.0 if this cannot be determined.
- `GetTotalCPUTimeIncludingChildren`: Return the total CPU time use by the process s far, in floating point seconds - or -1.0 if this cannot be determined. Compared to GetTotalCPUTime
- `PrintRusage`: Print resource usage to given stream.
- `TestLetterString`: Return true if all letters in to_check also appear in options, false otherwise.
- `CheckOptionLetterString`: Check if all the letters in to_check appear in options. If not, terminate with an error message.

### Dependencies

- `"clb_error.h"`
- `"clb_simple_stuff.h"`
- `<clb_defines.h>`
- `<stdarg.h>`
- `<string.h>`
- `<sys/resource.h>`
- `<sys/time.h>`
- `<sys/types.h>`
- `<sys/uio.h>`
- `<syscall.h>`

### Compile-Time Conditions

- `CLB_ERROR`
- `ENABLE_LFHO`
- `HP_UX`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `BASICS/clb_error.h`, `BASICS/clb_error.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `BASICS` covering 2 source file(s), about 505 lines, 15 scanned public declarations, 0 scanned internal function definitions, and 11 structured function-comment blocks.
- Central warning/error path. Many callers assume fatal diagnostics terminate rather than returning recoverable errors.
- Foundation code. Preserve allocation, container, assertion, and fatal-error conventions before trying to make the Rust version more idiomatic.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `InitError` only stores the passed program-name pointer in C, and `Error`/`Warning`/`SysError`/`SysWarning` all close over `ProgName`. Rust keeps the C-shaped global program name as owned text and exposes explicit render/write helpers so tests and lower-level owners can preserve the two-line `SysError`/`SysWarning` shape without terminating the process. All 26 executable entry points now initialize that owner before running and route returned fatal diagnostics through one shared reporter. The exact C split is retained: 22 use their canonical `NAME`, while `term2dag`, `ex_commandline`, `ekb_ginsert`, and `termprops` use the invocation name. The complete entry-point audit and exact unchanged-C fatal comparison are retained in [`experiment 125`](../../../experiments/2026-07-18-125-executable-diagnostic-owner/FINDINGS.md).
- `SysError` and `SysWarning` restore `errno` from `TmpErrno` immediately before calling `perror(ProgName)`. Rust keeps an explicit `TmpErrno` equivalent and renders the system line from `std::io::Error::from_raw_os_error`.
- `ELog` appends records to `elog<pid>.log` with a current-process CPU timestamp and sends only the trailing newline to `stderr`. Rust now exposes writer-injected helpers for the exact record/newline split plus a filesystem wrapper that appends to the C-shaped PID-named log file.

### Change Later

- `PrintRusage` sums `getrusage(RUSAGE_SELF)` and `getrusage(RUSAGE_CHILDREN)` user/system times, then prints the parent raw maximum resident set size under a "pages" label. Rust now has the C-shaped footer, native Windows process counters, and a Linux `getrusage` path that preserves the same child-time aggregation and parent-only `ru_maxrss` choice, with `/proc` fallback. Per-target resident-set units should remain visible in compatibility tests.
- `ELog` opens `elog<pid>.log` without checking for `fopen` failure, writes the log prefix and message to the file, then writes the trailing newline to `stderr` instead of the log file. Rust preserves the observable stream split but returns `io::Result` for file-open/write failures; after drop-in compatibility is secured, decide whether the newline-to-stderr behavior should remain compatibility-only or be replaced by an explicit logging policy.
- C `InitError` aliases caller-owned program-name storage rather than copying it. Rust stores owned text to avoid dangling references; if a compatibility test ever mutates the original C buffer after initialization, that aliasing quirk should be represented explicitly rather than leaking into ordinary Rust diagnostics.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
