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

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `EPCtrlAlloc` initializes most fields but not `fileno`; callers must not add the control cell to an `EPCtrlSet` before `ECtrlCreateGeneric` assigns the pipe descriptor. Rust should model this as an optional descriptor at the safe boundary rather than exposing uninitialized state.
- `ECtrlCreateGeneric` constructs a shell command by concatenating the prover path, fixed options, caller-provided options, CPU limit, and input file without quoting or escaping. Preserve exact construction only where byte-compatible command text matters; actual process creation should prefer structured arguments to avoid shell/path surprises.
- `EPCtrlGetResult` publishes `PRResultTable` entries for `PRFailure` and `PRGaveUp`, but the result scanner does not recognize `% Failure:` or `% SZS status GaveUp`; it only assigns `PRFailure` on EOF when no recognized proof/saturation status was seen. Rust should keep this compatibility quirk until scheduler reference tests decide whether failure/gave-up output needs direct recognition.
- `EPCtrlSetGetResult` ignores `select` errors, scans every integer descriptor from zero through `maxfd`, deletes no-proof subprocesses during that scan, and treats `PRGaveUp` as an impossible default case. A cleaned event-loop API should separate readiness polling, result parsing, process deletion, and diagnostic policy after compatibility is covered.
- The C module mixes subprocess ownership, temporary-file deletion, result parsing, and `GlobalOut` printing. Rust should keep those as explicit responsibilities so server and scheduler integrations can choose compatibility routing without hardwiring global output into the reusable process-control core.
<!-- END MANUAL REVIEW: c_source_docs -->
