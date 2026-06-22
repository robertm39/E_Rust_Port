<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_esession

## Source Files

- [CONTROL/cco_esession.h](../../../eprover/CONTROL/cco_esession.h)
- [CONTROL/cco_esession.c](../../../eprover/CONTROL/cco_esession.c)

## Purpose

Code and data structures representing a single session (i.e. connection to the user and all processes run on behalf of this user). <1> Fri Apr 22 15:08:31 CEST 2011

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ESessionCell`
- `ESessionState`
- `ESession_p`

### Macros And Constants

- `CCO_ESESSION`
- `ESessionCellAlloc()`
- `ESessionCellFree(junk)`
- `ESessionSetState(session, state)`
- `XXXCellAlloc()`
- `XXXCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `ESession_p ESessionAlloc(int sock)`
- `int ESessionInitFDSet(ESession_p session, fd_set *rd_fds, fd_set *wr_fds)`
- `void ESessionDoIO(ESession_p session, fd_set *rd_fds, fd_set *wr_fds)`
- `void ESessionFree(ESession_p junk)`
- `void ESessionProcessCmds(ESession_p session)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ESessionAlloc`: Allocate an initialized ESession cell. This is not yet listening.
- `ESessionFree`: Free an ESession.
- `ESessionInitFDSet`: Set the fd bits for all file descriptors relevant to the server. Return largest fd.
- `ESessionDoIO`: Perform I/O on all connections of the session.
- `ESessionProcessCmds`: Process the messages stored in the input queue of the channel.

### Dependencies

- `"cco_esession.h"`
- `<cco_proc_ctrl.h>`
- `<cio_multiplexer.h>`
- `<netinet/in.h>`

### Compile-Time Conditions

- `CCO_ESESSION`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_esession.h`, `CONTROL/cco_esession.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 326 lines, 8 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- Code and data structures representing a single session (i.e. connection to the user and all processes run on behalf of this user). <1> Fri Apr 22 15:08:31 CEST 2011
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
