<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_proc_ctrl

## Source Files

- [CONTROL/cco_proc_ctrl.h](../../../eprover/CONTROL/cco_proc_ctrl.h)
- [CONTROL/cco_proc_ctrl.c](../../../eprover/CONTROL/cco_proc_ctrl.c)

## Purpose

Code for running E as a separate process within other programs. This is only a first draft - there probably will be a much better version eventually ;-). the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `EPCtrlCell`
- `EPCtrlSetCell`
- `EPCtrlSet_p`
- `EPCtrl_p`

### Macros And Constants

- `CCO_PROC_CTRL`
- `EPCTRL_BUFSIZE`
- `EPCtrlCellAlloc()`
- `EPCtrlCellFree(junk)`
- `EPCtrlSetCardinality(set)`
- `EPCtrlSetCellAlloc()`
- `EPCtrlSetCellFree(junk)`
- `EPCtrlSetEmpty(set)`
- `E_OPTIONS`
- `E_OPTIONS_BASE`
- `MAX_CORES`
- `SZS_CONTRAAX_STR`
- `SZS_COUNTERSAT_STR`
- `SZS_FAILURE_STR`
- `SZS_GAVEUP_STR`
- `SZS_SATSTR_STR`
- `SZS_THEOREM_STR`
- `SZS_UNSAT_STR`

### Globals

- `extern char* PRResultTable[]`

### Exported Functions

- `EPCtrlSet_p EPCtrlSetAlloc(void)`
- `EPCtrl_p ECtrlCreate(char* prover, char* name, char* extra_options, long cpu_limit, char* file)`
- `EPCtrl_p ECtrlCreateGeneric(char* prover, char* name, char* options, char* extra_options, long cpu_limit, char* file)`
- `EPCtrl_p EPCtrlAlloc(char* name)`
- `EPCtrl_p EPCtrlSetFindProc(EPCtrlSet_p set, int fd)`
- `EPCtrl_p EPCtrlSetGetResult(EPCtrlSet_p set, bool delete_files)`
- `bool EPCtrlGetResult(EPCtrl_p ctrl, char* buffer, long buf_size)`
- `int EPCtrlSetFDSet(EPCtrlSet_p set, fd_set *rd_fds)`
- `void EPCtrlCleanup(EPCtrl_p ctrl, bool delete_file1)`
- `void EPCtrlFree(EPCtrl_p junk)`
- `void EPCtrlSetAddProc(EPCtrlSet_p set, EPCtrl_p proc)`
- `void EPCtrlSetDeleteProc(EPCtrlSet_p set, EPCtrl_p proc, bool delete_file)`
- `void EPCtrlSetFree(EPCtrlSet_p junk, bool delete_files)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `EPCtrlAlloc`: Allocate an initialized EPCtrlCell.
- `EPCtrlFree`: Free a EPCtrlCell.
- `EPCtrlCleanup`: Clean up: Kill process, close pipe,
- `ECtrlCreate`: Create a pipe running prover with time limit cpu_limit on file. "prover" must conform to the calling conventions of E and provide similar output. This takes over responsibility for the string pointed to by file.
- `ECtrlCreateGeneric`: Create a pipe running prover with time limit cpu_limit on file. "prover" must conform to the calling conventions of E and provide similar output. This takes over responsibility for the string pointed to by file.
- `EPCtrlGetResult`: Try to read a line from the E process. If successful, try to extract a result state. Return true if the E process terminated (i.e. the read returns 0), false otherwise.
- `EPCtrlSetAlloc`: Allocate an empty EPCtrlCell.
- `EPCtrlSetFree`: Free an EPCtrlSet(), including the payload.Will clean up the processes.
- `EPCtrlSetAddProc`: Add a process to the process set.
- `EPCtrlSetFindProc`: Find the process associated with fd.
- `EPCtrlSetDeleteProc`: Delete a process from the set.
- `EPCtrlSetFDSet`: Set all file descriptor bits of the set in the fd_set data structure. Return the largest one.

### Dependencies

- `"cco_proc_ctrl.h"`
- `<cio_tempfile.h>`
- `<clb_numtrees.h>`
- `<clb_simple_stuff.h>`
- `<signal.h>`
- `<sys/select.h>`

### Compile-Time Conditions

- `CCO_PROC_CTRL`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_proc_ctrl.h`, `CONTROL/cco_proc_ctrl.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 661 lines, 18 scanned public declarations, 0 scanned internal function definitions, and 12 structured function-comment blocks.
- Code for running E as a separate process within other programs. This is only a first draft - there probably will be a much better version eventually ;-). the GNU Lesser General Public License.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- The opt-in `ECtrlCreate`/`ECtrlCreateGeneric` compatibility constructors preserve the complete C-concatenated command as one shell command operand. They use `/bin/sh -c` on POSIX targets and `cmd.exe /C` on Windows, matching the respective `popen` command-processor contracts rather than consulting an arbitrary `COMSPEC` override.
- The compatibility path deliberately adds no quoting or escaping around prover, option, or input-file text. Regression tests pin spaces, quotes, redirections, and command separators byte-for-byte before shell execution, plus successful execution through the native Windows command processor.
- If the shell starts but the requested prover does not, both C and Rust reach the empty-first-line `Cannot read eprover PID line` diagnostic. Rust now returns C's `OTHER_ERROR` exit code 11 for that case and for a non-PID first line; a failure to start the shell itself remains `SYS_ERROR` code 7 with a host-specific system suffix.
- `EPCtrlSet::get_result` exposes C's fixed 500 ms `EPCtrlSetGetResult` wait and consumes at most one queued line from each process in ascending descriptor order. The ready-descriptor core retains C's later-descriptor-wins proof result, continues scanning after a proof, and removes no-proof EOF processes during the same scan.
- Rust keeps the reader-thread/channel wait backend because Windows Winsock `select` accepts sockets but rejects child-pipe handles. This preserves one portable owner shape; the channel loop has at most 10 ms readiness latency, preserves the requested timeout even for an empty set, and has no kernel-selector error to expose. C ignores `select` errors, so that difference has no result or diagnostic surface. A pipe read error now follows C `fgets` behavior by becoming EOF/failure rather than a Rust-only fatal diagnostic.
- The complete C call-site inventory contains `e_stratpar` and `BatchProcessProblem`; main `eprover` scheduling uses `cco_gproc_ctrl` instead. Server sessions only add running-process descriptors to their read set because the corresponding `ESessionDoIO` subprocess block is empty. Rust mirrors those boundaries rather than introducing a new E-specific scheduler/server path.
- Normal Unix cleanup now sends `SIGTERM` to the directly owned structured-spawn child and waits, matching `EPCtrlCleanup`'s `kill(..., SIGTERM)` then `pclose` order. Windows and a failed POSIX signal fall back to `Child::kill` before waiting. Rust deliberately targets the owned child rather than trusting arbitrary prover-reported PID text; the safe production constructor owns E directly, while the opt-in shell constructor owns its compatibility shell.
- Process-set failure messages are written through the poll caller's selected global-output writer. Batch success/give-up helpers preserve C's distinct global, destination, socket, and interactive proof-output routes; `e_stratpar` writes the winning accumulated child output through its executable output writer. Neither C nor Rust keeps a process-global registry that walks these local controls from the parent signal handler.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `EPCtrlAlloc` initializes most fields but not `fileno`; callers must not add the control cell to an `EPCtrlSet` before `ECtrlCreateGeneric` assigns the pipe descriptor. Rust should model this as an optional descriptor at the safe boundary rather than exposing uninitialized state.
- `ECtrlCreateGeneric` trusts the first line containing `% Pid: ` but then parses the PID from byte offset 7 rather than from the actual match location. Rust mirrors the effective line-start parser for normal C output and should only widen it if nonstandard prover wrappers are intended to work.
- `EPCtrlGetResult` publishes `PRResultTable` entries for `PRFailure` and `PRGaveUp`, but the result scanner does not recognize `% Failure:` or `% SZS status GaveUp`; it only assigns `PRFailure` on EOF when no recognized proof/saturation status was seen. Rust should keep this compatibility quirk until scheduler reference tests decide whether failure/gave-up output needs direct recognition.
- C `ESessionInitFDSet` delegates running controls to `EPCtrlSetFDSet`, but `ESessionDoIO` never calls `EPCtrlGetResult` or `EPCtrlSetGetResult`; its running-process branch is empty. Rust therefore exposes process-control polling to real batch/scheduler consumers while keeping legacy session readiness consumption as a pinned no-op.
<!-- END MANUAL REVIEW: c_source_docs -->
