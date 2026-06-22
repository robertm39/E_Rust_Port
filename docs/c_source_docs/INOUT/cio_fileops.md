<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_fileops

## Source Files

- [INOUT/cio_fileops.h](../../../eprover/INOUT/cio_fileops.h)
- [INOUT/cio_fileops.c](../../../eprover/INOUT/cio_fileops.c)

## Purpose

Simple operations on files. the GNU Lesser General Public License. <1> Wed Jul 28 12:43:28 MET DST 1999 New

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CIO_FILEOPS`
- `FileNameIsAbsolute(name)`

### Globals

- None found in the source scan.

### Exported Functions

- `FILE* InputOpen(char *name, bool fail)`
- `bool FileExists(char* name)`
- `char* FileFindBaseName(char *file)`
- `char* FileNameBaseName(char* name)`
- `char* FileNameDirName(char* name)`
- `char* FileNameStrip(char* name)`
- `long ConcatFiles(char* target, char** sources)`
- `long CopyFile(char* target, char* source)`
- `long FileLoad(char* name, DStr_p dest)`
- `void FilePrint(FILE* out, char* name)`
- `void FileRemove(char* name)`
- `void InputClose(FILE* file)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `InputOpen`: Open an input file for reading. NULL and "-" are stdin. If fail is true, terminate with error, otherwise pass error down.
- `InputClose`: Close an input file.
- `FileLoad`: Load the content of the named file and append it to dest. Returns number of characters read.
- `ConcatFiles`: Concatenate all file in (NULL-terminated) array sources into target. "-" is stdin, as always. Return number of files concated. This could be much optimized. Let me know if it ever shows up in a profile...
- `CopyFile`: Copy source to target (the lazy way ;-). Notice argument order (compatible with = and strcpy(), not with cp!)
- `FileRemove`: Remove a arbitrary file.
- `FilePrint`: Print the contents of the named file to out.
- `FileNameDirName`: Given a path name, return the directory portion (i.e. the part from the first character to the last / character (including it). Return "" if no directory part exists. It is the users responsibility to FREE the memory returned.
- `FileFindBaseName`: Return a pointer to the first character of the last file name component of name.
- `FileNameBaseName`: Given a path, return a copy of the base name part of it, i.e. the string starting at the last / (if any). In contrast to the UNIX command 'basename', it will return the empty string for a string ending in "/".
- `FileNameStrip`: Given a path, return a copy of the core name - i.e. the basename without a possible suffix.
- `FileExists`: Return true if file exists and can be opened for reading, false otherwise. This is not race-safe!

### Dependencies

- `"cio_fileops.h"`
- `<cio_output.h>`
- `<sys/stat.h>`

### Compile-Time Conditions

- `CIO_FILEOPS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_fileops.h`, `INOUT/cio_fileops.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 501 lines, 12 scanned public declarations, 0 scanned internal function definitions, and 12 structured function-comment blocks.
- Simple operations on files. the GNU Lesser General Public License. <1> Wed Jul 28 12:43:28 MET DST 1999 New
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
