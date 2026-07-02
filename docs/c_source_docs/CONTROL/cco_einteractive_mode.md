<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_einteractive_mode

## Source Files

- [CONTROL/cco_einteractive_mode.h](../../../eprover/CONTROL/cco_einteractive_mode.h)
- [CONTROL/cco_einteractive_mode.c](../../../eprover/CONTROL/cco_einteractive_mode.c)

## Purpose

Code for parsing and handling the server's interactive mode. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org), Stephan Schulz (schulz@eprover.org), Mohamed Bassem Hasona

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `AxiomSetCell`
- `AxiomSet_p`
- `InteractiveSpecCell`
- `InteractiveSpec_p`

### Macros And Constants

- `ADD_COMMAND`
- `AXIOM_SET_NAME_TOKENS`
- `AxiomSetCellAlloc()`
- `AxiomSetCellFree(junk)`
- `CCO_EINTERACTIVE_MODE`
- `DOWNLOAD_COMMAND`
- `END_OF_BLOCK_TOKEN`
- `ERR_AXIOM_SET_IS_ALREADY_STAGED_MESSAGE`
- `ERR_AXIOM_SET_IS_ALREADY_UNSTAGED_MESSAGE`
- `ERR_AXIOM_SET_IS_STAGED_MESSAGE`
- `ERR_AXIOM_SET_NAME_TAKEN_MESSAGE`
- `ERR_CANNOT_READ_SERVER_LIBRARY_MESSAGE`
- `ERR_ERROR_MESSAGE`
- `ERR_NO_AXIOM_LIBRARY_ON_SERVER_MESSAGE`
- `ERR_SYNTAX_ERROR_MESSAGE`
- `ERR_UNKNOWN_AXIOM_SET_MESSAGE`
- `ERR_UNKNOWN_COMMAND_MESSAGE`
- `HELP_COMMAND`
- `InteractiveSpecCellAlloc()`
- `InteractiveSpecCellFree(junk)`
- `LIST_COMMAND`
- `LOAD_COMMAND`
- `OK_ADDED_MESSAGE`
- `OK_DOWNLOADED_MESSAGE`
- `OK_LOADED_MESSAGE`
- `OK_REMOVED_MESSAGE`
- `OK_STAGED_MESSAGE`
- `OK_SUCCESS_MESSAGE`
- `OK_UNSTAGED_MESSAGE`
- `QUIT_COMMAND`
- `REMOVE_COMMAND`
- `RUN_COMMAND`
- `STAGE_COMMAND`
- `UNSTAGE_COMMAND`

### Globals

- None found in the source scan.

### Exported Functions

- `AxiomSet_p AxiomSetAlloc(ClauseSet_p cset, FormulaSet_p fset, DStr_p raw_data, int staged)`
- `InteractiveSpec_p InteractiveSpecAlloc(BatchSpec_p spec, StructFOFSpec_p ctrl, FILE* fp, int sock_fd)`
- `void AxiomSetFree(AxiomSet_p axiomset)`
- `void InteractiveSpecFree(InteractiveSpec_p spec)`
- `void StartDeductionServer(BatchSpec_p spec, StructFOFSpec_p ctrl, char* server_lib, FILE* fp, int sock_fd)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `get_directory_listings`: Open a directory and return a newly created stack of freshly allocated DStrs containing the names of regular files in the directory.
- `InteractiveSpecAlloc`: Allocate an initialized interactive spec structure.
- `InteractiveSpecFree`: Free an interactive spec structure. The BatchSpec struct and StructFOFSpec are not freed.
- `AxiomSetAlloc`: Allocate an initialized axiom set structure.
- `AxiomSetFree`: Free an interactive spec structure. The BatchSpec struct and StructFOFSpec are not freed.
- `StartDeductionServer`: Run the deduction server on the specified socked. Read commands and react to them.

### Dependencies

- `"cco_einteractive_mode.h"`
- `<ccl_formulafunc.h>`
- `<cco_batch_spec.h>`
- `<cco_proc_ctrl.h>`
- `<cio_network.h>`
- `<cio_scanner.h>`
- `<cio_simplestuff.h>`
- `<clb_dstrings.h>`
- `<clb_pstacks.h>`
- `<dirent.h>`
- `<sys/wait.h>`

### Compile-Time Conditions

- `CCO_EINTERACTIVE_MODE`
- `__sun`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_einteractive_mode.h`, `CONTROL/cco_einteractive_mode.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 1171 lines, 21 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Code for parsing and handling the server's interactive mode. the GNU Lesser General Public License.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- `src/control/einteractive_mode.rs` starts the Rust port of `cco_einteractive_mode` with the deduction-server command names, block terminator, success/error response strings, help text, and the C `AXIOM_SET_NAME_TOKENS`/`AcceptAxiomSetName` scanner loop.
- Tests cover the command/response string surface, token-mask membership, whitespace-tolerant name concatenation, stopping before unaccepted tokens, and the C-compatible empty-name acceptance path.
- Full interactive-server ownership remains pending: axiom-set storage over clause/formula owners, directory-library listing, staged/unstaged problem mutation, socket/stdout transport, block command parsing, and forked `RUN` job execution are not wired yet.

### Change-Later Observations

- `AcceptAxiomSetName` accepts zero tokens and uses ordinary scanner token tests, so whitespace between name fragments is silently removed while slashes stop the name. Command words such as `GO` are still acceptable name tokens if they appear in the same token run; the protocol relies on block terminators being on their own line. Rust preserves this parser shape, but a cleaned server protocol should require a nonempty name and probably use a single filename/name grammar.
- `AxiomSetAlloc` takes a `staged` argument but ignores it and always initializes `handle->staged = 0`. Preserve that when porting allocation, but treat the parameter as an obsolete API artifact once compatibility is covered.
- `get_directory_listings` allocates the result stack before `opendir()` and returns `NULL` without freeing it on open failure; it also depends on `dirent.d_type == DT_REG` except on Solaris. A Rust port should preserve visible regular-file filtering and output ordering where needed while avoiding the leak and handling filesystems that report unknown `d_type`.
- `StartDeductionServer` has a stdout/file-pointer path in its signature, but the implementation prints "Server mode not implemented yet for stdout" and exits unless `sock_fd != -1`. Keep socket-only behavior for drop-in compatibility until reference server tests cover the stdout path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
