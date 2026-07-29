<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / ekb_create

## Source Files

- [PROVER/ekb_create.c](../../../eprover/PROVER/ekb_create.c)

## Purpose

Create a new, empty knowledge base for E. the GNU Lesser General Public License.

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
- `<cle_kbdesc.h>`
- `<e_version.h>`
- `<sys/stat.h>`
- `<sys/types.h>`

### Compile-Time Conditions

- `CLB_MEMORY_DEBUG`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit compile-time feature gates and debug-only behavior; map supported variants to explicit Rust configuration or document why Umlaut intentionally chooses one path.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/ekb_create.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `PROVER` covering 1 source file(s), about 262 lines, 3 scanned public declarations, 0 scanned internal function definitions, and 1 structured function-comment blocks.
- Create a new, empty knowledge base for E. the GNU Lesser General Public License.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Notes

- `src/prover/kb_create.rs` and `src/bin/umlaut-kb-create.rs` port the standalone KB creation executable over the Rust `learn::kbdesc` helper.
- The Rust wrapper preserves the default basename `E_KNOWLEDGE`, short/long `-V`/`--version`, optional `--verbose`, negative-example count/proportion options, the one-argument limit, and the C typo in the negative-proportion diagnostic.
- File creation is intentionally C-shaped: create the base directory, write `description`, `signature`, `problems`, and `clausepatterns`, then create the `FILES` subdirectory. Later failures do not roll back earlier filesystem changes.

### Change Later

- Consider making KB creation transactional after compatibility mode exists. The C executable leaves partial KB directories if a later seed file or the `FILES` subdirectory cannot be created.
- The first `mkdir()` failure is reported with `SYNTAX_ERROR`, while the `FILES` subdirectory failure uses `FILE_ERROR` and passes the base directory name rather than the failing subdirectory path. A cleaned interface should report path-specific filesystem errors consistently.
- `--negative-example-number` accepts negative values even though the generated `FailExamples` field is later parsed as a positive integer by `KBDescParse`. Tighten this only after compatibility tests cover legacy KB files.
- `OPT_SELECT_EVAL` is present in the local option enum but has no option table entry or switch arm; remove that dead enum slot in any future C cleanup.
<!-- END MANUAL REVIEW: c_source_docs -->
