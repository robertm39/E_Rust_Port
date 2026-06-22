<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_filevars

## Source Files

- [INOUT/cio_filevars.h](../../../eprover/INOUT/cio_filevars.h)
- [INOUT/cio_filevars.c](../../../eprover/INOUT/cio_filevars.c)

## Purpose

Functions for managing file-stored "variable = value;" pairs. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FileVarsCell`
- `FileVars_p`

### Macros And Constants

- `CIO_FILEVARS`
- `FileVarsCellAlloc()`
- `FileVarsCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `FileVars_p FileVarsAlloc(void)`
- `bool FileVarsGetBool(FileVars_p vars, char* name, bool *value)`
- `bool FileVarsGetIdentifier(FileVars_p vars, char* name, char **value)`
- `bool FileVarsGetInt(FileVars_p vars, char* name, long *value)`
- `bool FileVarsGetStr(FileVars_p vars, char* name, char **value)`
- `long FileVarsParse(Scanner_p in, FileVars_p vars)`
- `long FileVarsReadFromFile(char* file, FileVars_p vars)`
- `void FileVarsFree(FileVars_p handle)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `FileVarsAlloc`: Allocate an empty, initialized filevars cell.
- `FileVarsFree`: Free a file vars cell.
- `FileVarsParse`: Parse a set of file var definitions. Return number of variables read. New definitions overwrite old ones!
- `FileVarsReadFromFile`: Read a set of file vars from a file (as opposed to an arbitrary scanner as above).
- `FileVarsGetBool`: Try to get a boolean value associated with a name. If it exist, set *var to the result and return true, otherwise leave *var untouched and return false. If value is not boolean, exit with error.
- `FileVarsGetInt`: Try to get an integer value associated with a name. If it exist, set *var to the result and return true, otherwise leave *var untouched and return false. If value is not integer, exit with error.
- `FileVarsGetStr`: Try to get any value associated with a name. If it exist, set *var to the result and return true, otherwise leave *var untouched and return false. *var will only live as long as vars!
- `FileVarsGetIdentifier`: Try to get an Identifier value associated with a name. If it exist, set *var to the result and return true, otherwise leave *var untouched and return false. If value is not integer, exit with error. *var will only live as long as vars!

### Dependencies

- `"cio_filevars.h"`
- `<cio_basicparser.h>`
- `<clb_pstacks.h>`
- `<clb_stringtrees.h>`

### Compile-Time Conditions

- `CIO_FILEVARS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_filevars.h`, `INOUT/cio_filevars.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 409 lines, 10 scanned public declarations, 0 scanned internal function definitions, and 8 structured function-comment blocks.
- Functions for managing file-stored "variable = value;" pairs. the GNU Lesser General Public License.
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
