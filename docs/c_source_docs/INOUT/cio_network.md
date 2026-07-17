<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_network

## Source Files

- [INOUT/cio_network.h](../../../eprover/INOUT/cio_network.h)
- [INOUT/cio_network.c](../../../eprover/INOUT/cio_network.c)

## Purpose

Helper code for TCP connections and "message" based communication over TCP (each message corresponds to a transaction request and is packages as a message to allow parsing in whole). <1> Wed Mar 9 22:24:40 CET 2011

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `MsgStatus`
- `TCPMsgCell`
- `TCPMsg_p`

### Macros And Constants

- `CIO_NETWORK`
- `TCPMsgCellAlloc()`
- `TCPMsgCellFree(junk)`
- `TCP_BACKLOG`
- `TCP_BUF_SIZE`
- `TCP_MSG_COMPLETE(msg)`

### Globals

- None found in the source scan.

### Exported Functions

- `MsgStatus TCPMsgRead(int sock, TCPMsg_p msg)`
- `MsgStatus TCPMsgSend(int sock, TCPMsg_p msg)`
- `MsgStatus TCPMsgWrite(int sock, TCPMsg_p msg)`
- `MsgStatus TCPStringSend(int sock, char* str, bool err)`
- `TCPMsg_p TCPMsgAlloc(void)`
- `TCPMsg_p TCPMsgPack(char* str)`
- `TCPMsg_p TCPMsgRecv(int sock, MsgStatus *res)`
- `char* TCPMsgUnpack(TCPMsg_p msg)`
- `char* TCPStringRecv(int sock, MsgStatus* res, bool err)`
- `char* TCPStringRecvX(int sock)`
- `int CreateClientSock(char* host, int port)`
- `int CreateServerSock(int port)`
- `void Listen(int sock)`
- `void TCPMsgFree(TCPMsg_p junk)`
- `void TCPStringSendX(int sock, char* str)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `create_server_sock_nofail`: Try to create a bound server socket. Return -1 on failure, the socket identifier on success.
- `create_client_sock_nofail`: Try to create a client socket connected to the provided host. Return negative value on failure, the socket identifier on success. The error return is -1 for errno-errors, -2-error for gai_error-errors.
- `TCPMsgAlloc`: Allocate an initialized TCP message cell.
- `TCPMsgFree`: Free a TCP message cell.
- `TCPMsgPack`: Take a string and convert it into a newly allocated TCP Msg.
- `TCPMsgUnpack`: Given a TCP message, return the string and destroy the container. If this ever becomes measurable, we can make this faster by avoiding the copy...
- `TCPMsgWrite`: Send the message over the socket. Return NWError, NWIncomplete, or NWSuccess depending on wether the transmission was partial, complete or a failure.
- `TCPMsgRead`: Receive a (partial) TCP message. Return NWError, NWIncomplete, or NWSuccess depending on wether the transmission was partial, complete or a failure. Return NWConnClosed if the connection was closed. This assumes that the message itself is plain ASCII string (i.e. no '\0' in the message), although it probably works otherwise.
- `TCPMsgSend`: Send the message over the connection represented by socket. This will block until transmission is complete. Return status.
- `TCPMsgRecv`: Receive and return a message and return it. This will block until transmission terminates.
- `TCPStringSend`: Send the string as a TCP message over the connection represented by socket. This will block until transmission is complete. Return status.
- `TCPStringRecv`: Receive a message string, unpack it, and return the message content (or NULL on failure).
- `TCPStringSendX`: Send a string, fail on error.
- `TCPStringRecvX`: Read and return a string.
- `CreateServerSock`: Create a server socket bound to the given port and return it. Fail with error message if the port cannot be creates.
- `Listen`: Thin wrapper around listen() terminating with an error message if it fails.
- `CreateClientSock`: Create a socket connected to the given host and port. Return sock or terminate with error on fail.

### Dependencies

- `"cio_network.h"`
- `<clb_dstrings.h>`
- `<clb_pqueue.h>`
- `<netdb.h>`
- `<netinet/in.h>`
- `<stdint.h>`
- `<sys/socket.h>`

### Compile-Time Conditions

- `CIO_NETWORK`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_network.h`, `INOUT/cio_network.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 718 lines, 18 scanned public declarations, 0 scanned internal function definitions, and 17 structured function-comment blocks.
- Helper code for TCP connections and "message" based communication over TCP (each message corresponds to a transaction request and is packages as a message to allow parsing in whole). <1> Wed Mar 9 22:24:40 CET 2011
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Rust Port Status Notes

- `src/inout/network.rs` ports the `MsgStatus` discriminants, `TCPMsgCell` allocation shape, four-byte network-order total-length header, C-string prefix truncation for pack/unpack and payload read accumulation, partial single-read/write status behavior, C `TCPMsgRead` progress tracing as an explicit read/receive path, blocking send/receive loops, string send/receive wrappers, safe `TcpListener`/`TcpStream` socket constructors including C's final-address client-connect outcome, Linux/Windows server-socket creation with `SO_REUSEADDR` plus C backlog setup before wrapping the listening socket, C-shaped two-line server/connect system diagnostics, and Unix `gai_strerror` detail recovery from Rust's resolver wrapper.
- Tests cover message status values, new-message shape, packed header bytes, NUL truncation during pack/unpack/read accumulation, partial writes, send loops, partial header/payload reads, C progress trace text, closed-connection reporting, empty-payload status, receive loops, string wrappers, ephemeral server binding, a real loopback server/client byte exchange, and deterministic socket diagnostic shapes.

### Compatibility Evidence

- `EServerListen`, `e_server`, and `e_deduction_server` are the only server-creation callers, and each calls `Listen` immediately after creation. Rust safely folds bind plus backlog-10 listen into the owning `TcpListener` constructor and retains `listen` as an idempotent compatibility call, avoiding a safe type that temporarily owns a bound-but-not-listening socket.
- C uses raw descriptors for readiness and successful `Accepted %d` output, but the numeric values are process-local OS allocation results. Rust retains the actual raw listener/session descriptors at those boundaries; only cross-process comparison output normalizes successful descriptor numbers.
- C has no close-error diagnostic here: it ignores failed-client close results and leaks the server descriptor on option/bind failure. Rust preserves silence while closing all still-owned resources. The final-address return outcome remains C-shaped, while the earlier-success lifetime difference stays in the post-compatibility item below.
- Rust's Unix standard library builds resolver failures from the platform `gai_strerror` detail with a fixed `failed to lookup address information: ` prefix. Removing that prefix recovers C's one-line `Could not resolve address (<detail>)` diagnostic; other host errors retain native text.

### Change Later

- `TCPMsgRead` prints header and payload read progress directly to stdout, treats an empty-payload message as a closed connection after reading the header, and appends payload data through C-string APIs after partial reads. Rust keeps the wire format, embedded-NUL truncation, empty-payload status quirk, and exact progress-line text through explicit traced helpers, but truncates only within initialized read bytes; C writes the terminator at the requested read length rather than the actual short-read length, which can expose uninitialized buffer contents. Keep avoiding that unsafe tail unless byte-for-byte uninitialized-tail compatibility becomes required.
- `create_server_sock_nofail` sets `SO_REUSEADDR` before binding. Rust now preserves that setup on Linux and Windows through scoped socket-library boundaries, but other platform reuse semantics and raw descriptor-number behavior remain deferred until server/client programs require byte-identical platform behavior.
- `create_client_sock_nofail` continues iterating after a successful connection, which can leak or replace an earlier successful socket depending on later address records. Rust now preserves the returned final-address success/failure outcome, but ownership closes earlier successful `TcpStream`s instead of preserving C's descriptor leak.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
