<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_eserver

## Source Files

- [CONTROL/cco_eserver.h](../../../eprover/CONTROL/cco_eserver.h)
- [CONTROL/cco_eserver.c](../../../eprover/CONTROL/cco_eserver.c)

## Purpose

Control code for realising the E server. <1> Thu Mar 17 01:08:00 CET 2011 New

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `EServerCell`
- `EServer_p`

### Macros And Constants

- `CCO_ESERVER`
- `EServerCellAlloc()`
- `EServerCellFree(junk)`
- `XXXCellAlloc()`
- `XXXCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `EServer_p EServerAlloc(void)`
- `bool EServerAccept(EServer_p server)`
- `bool EServerListen(EServer_p server, int port)`
- `int EServerInitFDSet(EServer_p server, fd_set *rd_fds, fd_set *wr_fds)`
- `void EServerFree(EServer_p junk)`
- `void EServerReset(EServer_p server)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `EServerAlloc`: Allocate an initialized EServer cell. This is not yet listening.
- `EServerFree`: Free an EServer.
- `EServerReset`: Close all communication channels and delete their queues.
- `EServerListen`: Switch the server to listening mode on the given port number. Return success (true) or failure.
- `EServerAccept`: Accept a new connection on the listening port and queue it in the connection queue. This assumes that there is a pending connection (e.g. indicated via select). Return success or failure.
- `EServerInitFDSet`: Set the fd bits for all file descriptors relevant to the server. Return largest fd.

### Dependencies

- `"cco_eserver.h"`
- `<cco_esession.h>`
- `<cio_multiplexer.h>`
- `<netinet/in.h>`

### Compile-Time Conditions

- `CCO_ESERVER`

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

Source files reviewed: `CONTROL/cco_eserver.h`, `CONTROL/cco_eserver.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 298 lines, 8 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Control code for realising the E server. <1> Thu Mar 17 01:08:00 CET 2011 New
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

- `EServerFree` and `EServerReset` are empty in this source snapshot despite comments promising to free/close server state. Rust should preserve the callable no-op surface where needed for compatibility, but ordinary owned server values should release listener/session resources on drop; revisit explicit close/drain APIs once server lifecycle call sites are ported.
- `EServerInitFDSet` unconditionally applies `FD_SET(server->listening, rd_fds)` after maxing with `server->listening`. If a caller invokes it before `EServerListen`, the C code can operate on descriptor `-1`; Rust should keep this as an internal precondition or compatibility-shim concern rather than exposing invalid descriptor registration in safe APIs.
- The C server queue stores raw `ESession_p` values in a generic `PQueue`. Rust can use an owned queue for now, but stale-session removal and active-session ordering should be reference-tested before replacing the queue policy with a higher-level event-loop abstraction.
<!-- END MANUAL REVIEW: c_source_docs -->
