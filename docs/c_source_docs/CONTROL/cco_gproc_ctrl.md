<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_gproc_ctrl

## Source Files

- [CONTROL/cco_gproc_ctrl.h](../../../eprover/CONTROL/cco_gproc_ctrl.h)
- [CONTROL/cco_gproc_ctrl.c](../../../eprover/CONTROL/cco_gproc_ctrl.c)

## Purpose

Code for handling forked processes and IPC. This is derived from cco_proc_ctrl.h, but not suitable for external processess started via popen(), but for fork()ed subprocesses. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `EGPCtrlCell`
- `EGPCtrlSetCell`
- `EGPCtrlSet_p`
- `EGPCtrl_p`

### Macros And Constants

- `CCO_GPROC_CTRL`
- `EGPCTRL_BUFSIZE`
- `EGPCtrlCellAlloc()`
- `EGPCtrlCellFree(junk)`
- `EGPCtrlSetCardinality(set)`
- `EGPCtrlSetCellAlloc()`
- `EGPCtrlSetCellFree(junk)`
- `EGPCtrlSetCoresReserved(set)`
- `EGPCtrlSetEmpty(set)`

### Globals

- None found in the source scan.

### Exported Functions

- `EGPCtrlSet_p EGPCtrlSetAlloc(void)`
- `EGPCtrl_p EGPCtrlAlloc(int cores)`
- `EGPCtrl_p EGPCtrlCreate(char* name, int cores, rlim_t cpu_limit)`
- `EGPCtrl_p EGPCtrlSetFindProc(EGPCtrlSet_p set, int fd)`
- `EGPCtrl_p EGPCtrlSetGetResult(EGPCtrlSet_p set)`
- `bool EGPCtrlGetResult(EGPCtrl_p ctrl, char* buffer, long buf_size)`
- `int EGPCtrlSetFDSet(EGPCtrlSet_p set, fd_set *rd_fds)`
- `void EGPCtrlCleanup(EGPCtrl_p ctrl)`
- `void EGPCtrlFree(EGPCtrl_p junk)`
- `void EGPCtrlSetAddProc(EGPCtrlSet_p set, EGPCtrl_p proc)`
- `void EGPCtrlSetDeleteProc(EGPCtrlSet_p set, EGPCtrl_p proc, bool kill_proc)`
- `void EGPCtrlSetFree(EGPCtrlSet_p junk, bool kill_proc)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `EPGCtrlAlloc`: Allocate an initialized EPGCtrlCell.
- `EGPCtrlFree`: Free a EPCtrlCell.
- `EGPCtrlCleanup`: Clean up: Kill process, close pipe,
- `EGCtrlCreate`: Fork the process and establish a pipe from child to parent. Returns NULL in the the child, a pointer to a new EGPclCtrl-Block wrapping that pipe in the parent.
- `EGPCtrlGetResult`: Read data from the connected subprocess. If that has terminated, determine the status and record in in the block. Return true. Otherwise return false.
- `EGPCtrlSetAlloc`: Allocate an empty EGPCtrlCell.
- `EGPCtrlSetFree`: Free an EPCtrlSet(), including the payload. Will clean up the processes.
- `EGPCtrlSetAddProc`: Add a process to the process set.
- `EGPCtrlSetFindProc`: Find the process associated with fd.
- `EGPCtrlSetDeleteProc`: Delete a process from the set.
- `EGPCtrlSetFDSet`: Set all file descriptor bits of the set in the fd_set data structure. Return the largest one.

### Dependencies

- `"cco_gproc_ctrl.h"`
- `<cco_proc_ctrl.h>`
- `<cio_signals.h>`
- `<sys/wait.h>`
- `<unistd.h>`

### Compile-Time Conditions

- `CCO_GPROC_CTRL`

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

Source files reviewed: `CONTROL/cco_gproc_ctrl.h`, `CONTROL/cco_gproc_ctrl.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 612 lines, 16 scanned public declarations, 0 scanned internal function definitions, and 11 structured function-comment blocks.
- Code for handling forked processes and IPC. This is derived from cco_proc_ctrl.h, but not suitable for external processess started via popen(), but for fork()ed subprocesses. the GNU Lesser General Public License.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `EGPCtrlCreate` is represented by an explicit executable worker boundary rather than a direct Rust `fork()` API. The fresh child process captures stdout through `Command::stdout(Stdio::piped())`, uses stdout as its logical `GlobalOut`, starts with exec-reset caught-signal state, and applies the requested child CPU limit on Linux. This preserves the caller-visible parent/child behavior without exposing C's child-side `NULL` return through unsafe post-fork Rust control flow. Generic cleanup sends Unix `SIGTERM` before waiting and retains a hard-termination fallback. The source comparison and safety decision are recorded in [`experiments/2026-07-16-031-gproc-worker-boundary/FINDINGS.md`](../../../experiments/2026-07-16-031-gproc-worker-boundary/FINDINGS.md).

### Change Later

- `EGPCtrlGetResult` reads raw chunks and derives the SZS result only after EOF by scanning the accumulated output. That differs from the line-oriented `cco_proc_ctrl` path; keep it until multicore scheduler traces prove whether early status detection would be observable.
- `EGPCtrlSetFree` delegates to `EGPCtrlSetDeleteProc`, and when `kill_proc` is false the C path frees the control cell without closing the pipe or terminating a live child. Rust ownership should prefer cleanup-on-drop unless a reference scenario relies on the leak-like lifetime.
- `cores_reserved` is maintained by unchecked integer addition/subtraction in C. Rust tracks it with safe `usize` accounting; revisit only if exact overflow behavior becomes part of a compatibility test.
<!-- END MANUAL REVIEW: c_source_docs -->
