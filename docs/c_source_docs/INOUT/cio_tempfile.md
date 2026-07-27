<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_tempfile

## Source Files

- [INOUT/cio_tempfile.h](../../../eprover/INOUT/cio_tempfile.h)
- [INOUT/cio_tempfile.c](../../../eprover/INOUT/cio_tempfile.c)

## Purpose

Functions dealing with temporary files. the GNU Lesser General Public License. <1> Sat Jul 24 02:25:20 MET DST 1999 New

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CIO_TEMPFILE`

### Globals

- None found in the source scan.

### Exported Functions

- `char* TempFileCreate(FILE* source)`
- `char* TempFileName(void)`
- `void TempFileCleanup(void)`
- `void TempFileRegister(char *name)`
- `void TempFileRemove(char* name)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `TempFileCleanup`: Remove all temporary files.
- `TempFileRegister`: Register a file as temporary and to remove at exit.
- `TempFileName`: Allocate and register a new temporary file name. The caller has to free the name!
- `TempFileCreate`: Create a temporary file storing the data from source. Return name of the created file.
- `TempFileRemove`: Remove a temporary file.

### Dependencies

- `"cio_tempfile.h"`
- `<cio_commandline.h>`
- `<cio_fileops.h>`
- `<clb_memory.h>`
- `<clb_stringtrees.h>`
- `<stdlib.h>`

### Compile-Time Conditions

- `CIO_TEMPFILE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_tempfile.h`, `INOUT/cio_tempfile.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 271 lines, 5 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- Functions dealing with temporary files. the GNU Lesser General Public License. <1> Sat Jul 24 02:25:20 MET DST 1999 New
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- `src/inout/tempfile.rs` ports the process-global temporary-file registry, authoritative `TMPDIR` selection with `/tmp` as the Unix-like fallback and the native temporary directory as the Windows fallback, `epr_` prefix, safe atomic `create_new` retry creation, source-copy creation, explicit removal/unregistration including a C-shaped registered-file assertion wrapper, and cleanup warning collection.
- Rust uses a `Mutex<BTreeSet<PathBuf>>` instead of C's file-static `StrTree`, keeping safe registration and cleanup behavior for current callers.
- Tests cover TMPDIR placement, the six-character base-36 suffix shape, registration count, source-copy contents, duplicate registration, cleanup of existing and missing files, registered and unregistered removal behavior, removal diagnostics, and signal-triggered cleanup.

### Compatibility Notes

- C's registry retains caller-owned path pointers and orders them by string content; Rust stores owned `PathBuf` copies in a content-keyed set. Production callers observe registration only through removal or process-wide cleanup, so pointer identity and tree shape are not part of executable behavior.
- C `mkstemp` and Rust `OpenOptions::create_new` both atomically create and close an empty file before registration. Rust explicitly requests Unix mode `0o600`, matching `mkstemp`'s owner-only contract while avoiding an unnecessary unsafe native-API boundary.
- Explicit removal unlinks first and unregisters only after success in both ports. Registry-wide cleanup removes every registration even after an unlink warning, and the signal layer invokes it once for the first SIGTERM/SIGINT termination outcome.
- `TempFileCreate` preserves the registered file on copy/open failure so the termination cleanup owner can still attempt removal, matching the C lifetime boundary.

### Change Later

- C `TempFileName` delegates suffix selection and file mode to `mkstemp`. Rust uses a safe `create_new` retry loop with a generated six-character base-36 suffix on every target and owner-only `0o600` permissions on Unix. Exact libc suffix distribution, NUL-byte path diagnostics, and the intentional native-Windows fallback instead of C's literal `/tmp` remain platform-policy questions.
- C's global registry is cleared during cleanup even when unlinking a file fails. Rust mirrors that shape by clearing registrations and returning warnings; scoped run-state ownership would be cleaner after signal/atexit compatibility is designed.
- `TempFileRemove` asserts that the removed path was registered. Rust keeps the bool-returning helper for safer callers and now exposes an asserting compatibility wrapper for direct C-shaped paths.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
