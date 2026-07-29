<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / ekb_delete

## Source Files

- [PROVER/ekb_delete.c](../../../eprover/PROVER/ekb_delete.c)

## Purpose

Delete a training example from the knowledge base. the GNU Lesser General Public License. <1> Wed Jul 28 16:21:33 MET DST 1999 New

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
- `<cio_output.h>`
- `<cle_kbinsert.h>`
- `<e_version.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`

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

Source files reviewed: `PROVER/ekb_delete.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 267 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Delete a training example from the knowledge base. the GNU Lesser General Public License. <1> Wed Jul 28 16:21:33 MET DST 1999 New
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- The user-visible options are `-h`/`--help`, `--version`, `-v`/`--verbose[=level]`, and `-k`/`--knowledge-base=<path>`. `OPT_NAME` appears in the enum and switch but has no option-table entry, so it is not reachable from the command line.
- The executable reads and rewrites only `problems` and `clausepatterns`; existing `description`, `signature`, and stored example files other than `FILES/<name>` are not inspected or rewritten during deletion.
- C removes annotations and the proof-example entry in memory before calling `FileRemove` on `KBFileName(kb_name, "FILES/") + ex_name`, then writes `clausepatterns` before `problems`. If file removal fails, the fatal error happens before either metadata file is rewritten.
- `FileRemove` prints its verbose progress through the generic file helper and reports unlink failure with two diagnostics: one for `Cannot remove file <path>` and then a generic temporary-file message.

### Rust Port Notes

- Rust ports this executable as `src/prover/kb_delete.rs` with a thin `src/bin/umlaut-kb-delete.rs` wrapper. It preserves the default `E_KNOWLEDGE` basename, one-argument validation, unreachable `OPT_NAME` omission, read/remove/write order, and C-shaped KB output formatting.
- Filesystem tests cover deletion of the selected `FILES/<name>` payload, retention of unrelated stored examples, problem-list rewriting, annotation removal by source id, missing-example rejection before file removal, help/version text, and verbose progress output.

### Change Later

- Consider making KB deletion transactional once compatibility is established. The C flow mutates in-memory metadata first, removes the example file next, then rewrites metadata; a crash or fatal I/O error can leave stale metadata or partially updated files depending on where it occurs.
- Replace the generic `FileRemove` temporary-file wording for non-temporary KB files in a modernized mode. The current second diagnostic says "temporary file" even when deleting a stored problem file.
- Remove or wire up the dead `OPT_NAME` path if the command-line surface is ever cleaned; for compatibility it should stay invisible.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
