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

- `src/control/einteractive_mode.rs` ports the deduction-server command names, block terminator, success/error response strings, help text, the C `AXIOM_SET_NAME_TOKENS`/`AcceptAxiomSetName` scanner loop, `AxiomSet`/`InteractiveSpec` state for parsed axiom sets, `ADD`/`LOAD` over every TSTP form accepted by the C server parser, `STAGE`, `UNSTAGE`, `LIST`, `DOWNLOAD`, `REMOVE`, `QUIT`, single-message `StartDeductionServer` dispatch over injected block readers and `RUN` handlers, `ReadTextBlock`/`TCPReadTextBlock`-style adapters for `ADD`/`RUN` payloads, a TCP-string receive/send loop adapter that retains the C call-level frame boundaries, staged `RUN` job parsing plus `BatchProcessProblem` execution over injected runner hooks, and the `get_directory_listings` regular-file helper.
- Tests cover the command/response string surface, token-mask membership, whitespace-tolerant name concatenation, stopping before unaccepted tokens, the C-compatible empty-name acceptance path, regular-file-only directory listings, hidden regular-file inclusion, directory exclusion, open-failure handling, axiom-set allocation defaults, duplicate-name rejection, uploaded CNF/FOF/TFF/TCF/THF/watchlist/`$distinct`/include parsing, load status rewriting, stage/unstage control-stack mutation, list/download output, removal order, separate body/status TCP frames, injected and transport-backed `ADD`/`RUN` block reads, exact `RUN` start/proof/finish/success frame order, TCP socket-loop command receive/send behavior, `RUN` batch execution over injected runner hooks, the 30-second fallback/per-problem limit behavior, `QUIT` cleanup, the staged-remove stack side effect, and the unstage flag-clear-before-control-miss side effect.
- Each `print_to_outstream` call maps to one `TCPStringSendX`. A later temporary WSL build captured the complete intended four-frame `RUN` exchange and the default-build process-control PID-prefix failure; a real-loopback Rust regression now compares every message byte. The initial source audit is recorded in [`experiment 023`](../../../experiments/2026-07-16-023-deduction-server-concurrency/FINDINGS.md), and the live follow-up is recorded in [`experiment 044`](../../../experiments/2026-07-17-044-deduction-server-run-framing/FINDINGS.md).

### Change Later

- `AcceptAxiomSetName` accepts zero tokens and uses ordinary scanner token tests, so whitespace between name fragments is silently removed while slashes stop the name. Command words such as `GO` are still acceptable name tokens if they appear in the same token run; the protocol relies on block terminators being on their own line. Rust preserves this parser shape, but a cleaned server protocol should require a nonempty name and probably use a single filename/name grammar.
- `AxiomSetAlloc` takes a `staged` argument but ignores it and always initializes `handle->staged = 0`. Preserve that when porting allocation, but treat the parameter as an obsolete API artifact once compatibility is covered.
- `get_directory_listings` allocates the result stack before `opendir()` and returns `NULL` without freeing it on open failure; it also depends on `dirent.d_type == DT_REG` except on Solaris. Rust preserves the visible `NULL`/`None` open-failure result, regular-file filtering, unsorted directory iteration order, and stack-shaped caller behavior while avoiding the leak. A cleaned server-library API could use sorted output or richer I/O errors only after compatibility tests cover `LIST` and `LOAD` output.
- `remove_command` pops axiom sets from the stack while searching. If it finds the target staged, it returns `ERR_AXIOM_SET_IS_STAGED_MESSAGE` immediately without restoring the already-popped spare stack and without putting the staged target back. Rust preserves the visible stack loss for compatibility; a cleaned server state API should make failed removal non-mutating once reference tests no longer rely on this behavior.
- `list_command` prints in-memory staged and unstaged sets by increasing stack index, but prints on-disk files by popping the directory-listing stack, reversing the raw helper push order. Rust keeps this split ordering; sorted or stable display should be a later UI/protocol cleanup.
- `stage_command` inserts the same clause/formula-set pointers into `ctrl` that `AxiomSet` still retains, then sets only `shared_ax_sp`, not `shared_ax_f_count`. Rust currently uses cloned owned sets as a safe bridge and preserves the stack boundary update shape; replace this with stable shared handles once proof-control ownership can model the C aliasing directly.
- `unstage_command` clears `handle->staged` before it proves that the matching clause/formula set exists in `ctrl`, so a corrupted or desynchronized control stack yields `ERR_UNKNOWN_AXIOM_SET_MESSAGE` while leaving the axiom set marked unstaged. Rust preserves this visible ordering; a cleaned command API should make state updates transactional after compatibility tests cover the server protocol.
- `add_command` writes uploaded text to a temporary file, parses that file, then leaves duplicate-name detection until after parsing/allocation. `load_command` reads the selected server-library file into memory and delegates through the same temp-file parse path, so include resolution and read-error behavior are not simply "relative to the server library". Rust preserves parse-before-duplicate behavior and uses an in-memory scanner with no default include directory for the current bridge; audit temp-file side effects and include semantics again once full server protocol tests exist.
- `StartDeductionServer` validates the command word and the consumed argument tokens, but it does not reject trailing tokens after immediate commands or after the one-token `RUN` job name. Rust preserves that single-message dispatch shape; a cleaned protocol should reject extra tokens only after compatibility tests pin down existing clients.
- `RUN` captures only the current token literal and then accepts one `Identifier`, unlike the broader `AcceptAxiomSetName` parser used by `ADD`, `LOAD`, and most state commands. Rust keeps this one-token job-name behavior; later server cleanup should use one explicit name grammar across named commands.
- `ADD` and `RUN` parse the command line first and then read a separate transport block terminated by exact `GO\n`. Rust represents this with line-reader and TCP-message block adapters plus an injected block-reader dispatch hook; the eventual socket/stdin integration should keep the block boundary explicit and preserve exact terminator handling.
- `quit_command` gathers staged names in stack order, then pops that temporary stack and calls `unstage_command`, so shutdown cleanup runs in reverse staged-stack order and ignores per-set unstage statuses. Rust preserves that order; a later cleanup path should report inconsistent control state once compatibility tests cover connection close behavior.
- `run_command` forks before parsing the uploaded job, writes the job name directly to stdout, sends start/finish messages through `print_to_outstream` from the child, and returns `OK_SUCCESS_MESSAGE` from the parent after waiting. Rust preserves the logical parse, `BatchProcessProblem` call, C-shaped 30-second fallback limit, captured global/stdout side channel, and exact socket order synchronously inside the per-client worker. The live reference confirms wait-before-success on the intended path; any future asynchronous implementation must retain the captured start/proof/finish/success ordering.
- `StartDeductionServer` sends one TCP string for each `print_to_outstream` call, including a successful empty `DOWNLOAD` body, and sends no response for `QUIT`. Rust records byte offsets rather than flattening those calls, so HELP, LIST, DOWNLOAD, and RUN retain separate body/progress/status frames while text-mode helpers can still consume the aggregate string.
- `StartDeductionServer` has a stdout/file-pointer path in its signature, but the implementation prints `e_deduction_server: Server mode not implemented yet for stdout` and exits unless `sock_fd != -1`. Rust preserves that executable no-port message while keeping text-session helpers as reusable test/internal surfaces; a cleaned server can implement stdin/stdout mode only outside strict compatibility.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
