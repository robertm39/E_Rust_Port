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

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
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

### Rust Port Status Notes

- `src/control/esession.rs` ports the `ESessionState` discriminants, session allocation around the ported `TcpChannel`, live OS-descriptor retention, descriptor-interest collection corresponding to C `fd_set` registration, no-state/stale readiness filtering, write readiness when outbound messages are queued, optional subprocess descriptor delegation through a trait, the intentional no-op when only a registered subprocess descriptor is ready, and stale transition with channel close on read/write error or closed input.
- The Rust I/O path preserves the C session skeleton: complete inbound messages enqueue a literal `"wait"` reply, queued outbound messages are written when the descriptor is write-ready, and `ESessionProcessCmds`-style command draining renders `Received: ...` through an explicit writer for testability.
- The close transition preserves C's verbosity-gated `Closing channel <descriptor>` stderr line and then releases the owned stream without manufacturing a close-error diagnostic; C ignores `close(2)`'s return. Stale sessions stop registering the retained descriptor, and safe ownership prevents a later free-time double close.
- Tests cover enum values, new-session readiness, active-session read/write interest registration, the asymmetric subprocess registration/no-I/O boundary, read-side command capture plus `"wait"` reply writing, exact close output, stale marking on closed input, descriptor-interest removal, and peer-observed closure of a real loopback socket.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- `ESessionDoIO` registers descriptors for `running` subprocess controls through `EPCtrlSetFDSet`, but its actual subprocess I/O branch is an empty placeholder. Rust preserves that observable asymmetry even though `cco_proc_ctrl` result polling is now ported for scheduler/batch callers. A future functional server protocol may call the process-control API deliberately, but doing so in the compatibility session loop would add behavior absent from C.
- After every successful channel read, the C code blindly queues the literal string `"wait"` before command processing. Preserve this handshake for compatibility, but a cleaned server protocol should make request parsing and reply policy explicit after reference tests cover the server mode.
- `ESessionProcessCmds` drains queued messages by printing `Received: %s\n` directly to stdout and freeing the unpacked string; it does not update session state or dispatch commands. Rust should route this through explicit writers in reusable APIs, then reproduce direct stdout only in the executable compatibility layer if server mode needs it.
<!-- END MANUAL REVIEW: c_source_docs -->
