<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / ekb_insert

## Source Files

- [PROVER/ekb_insert.c](../../../eprover/PROVER/ekb_insert.c)

## Purpose

Insert an new training example file into a knowledge base. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OptionCodes`

### Macros And Constants

- `NAME`

### Globals

- None found in the source scan.

### Exported Functions

- `CLState_p process_options(int argc, char* argv[])`
- `void print_help(FILE* out)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `process_options`: Read and process the command line option, return (the pointer to) a CLState object containing the remaining arguments.

### Dependencies

- `<cio_commandline.h>`
- `<cio_fileops.h>`
- `<cio_output.h>`
- `<cle_kbinsert.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`
- `STACK_SIZE`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/ekb_insert.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 308 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Insert an new training example file into a knowledge base. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- User-visible options are `-h`/`--help`, `-V`/`--version`, `-v`/`--verbose[=level]`, `-n`/`--name=<name>`, and `-k`/`--knowledge-base=<path>`.
- If no input names remain after option parsing, the program inserts one synthetic `-` input after the old KB files have been parsed. A stdin example with no explicit name is named `__problem__<proof_examples->count+1>`.
- The `-n`/`--name` global is cleared after each loop iteration, so an explicit name applies only to the first inserted input. Later files infer their names from the input basename or from the default stdin pattern.
- File-derived names use `FileFindBaseName`, which recognizes only `/` as a path separator and keeps the filename extension.
- Each example file is copied into `FILES/<example-name>` before `KBParseExampleFile` parses that stored copy. `clausepatterns` and `problems` are written only after all inputs have been copied and parsed.
- The executable reads `signature` before parsing old `clausepatterns`, but it rewrites only `clausepatterns` and `problems`; the `signature` file is not updated after inserted examples are parsed.

### Rust Port Notes

- Rust ports this executable as `src/prover/ekb_insert.rs` with a thin `src/bin/ekb_insert.rs` wrapper. It preserves the default `E_KNOWLEDGE` basename, no-argument stdin insertion, first-input-only `--name` behavior, basename selection, duplicate-name rejection before copy, stored-file parse flow, and C-shaped KB output formatting.
- Filesystem tests cover stdin insertion, file insertion, first-use name override, duplicate rejection before file copy, stored example payloads, problem-list rewriting, annotation output, help/version text, and verbose progress output.

### Change Later

- Make insertion transactional after compatibility is secured. C copies each input into `FILES/` before parsing it and writes metadata only after the full loop, so a parse or I/O failure can leave stored files without matching `problems`/`clausepatterns` entries.
- Revisit the one-use `--name` behavior for multi-file insertion. It follows the C global reset, but it is surprising for users and should be made explicit or replaced in a modernized interface.
- Decide whether the `signature` file should be updated when inserted examples introduce symbols not present in the old KB signature. C mutates the in-memory signature during parsing but does not write that file back.
- Replace char-by-char `CopyFile`-style copying with a buffered or atomic copy path in any non-compatibility mode.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
