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

- `src/inout/network.rs` ports the `MsgStatus` discriminants, `TCPMsgCell` allocation shape, four-byte network-order total-length header, C-string prefix truncation for pack/unpack, partial single-read/write status behavior, blocking send/receive loops, string send/receive wrappers, and safe `TcpListener`/`TcpStream` socket constructors.
- Tests cover message status values, new-message shape, packed header bytes, NUL truncation, partial writes, send loops, partial header/payload reads, closed-connection reporting, empty-payload status, receive loops, string wrappers, and ephemeral server binding.

### Change-Later Observations

- `TCPMsgRead` prints header and payload read progress directly to stdout, treats an empty-payload message as a closed connection after reading the header, and appends payload data through C-string APIs after partial reads. Rust keeps the wire format and empty-payload status quirk, but appends only initialized bytes and omits the debug prints; revisit this only if byte-for-byte debug output compatibility becomes required.
- `create_server_sock_nofail` sets `SO_REUSEADDR` before binding. The current Rust wrapper uses only `std::net::TcpListener::bind`, so exact reuse semantics are not represented without adding a platform socket option layer.
- `create_client_sock_nofail` continues iterating after a successful connection, which can leak or replace an earlier successful socket depending on later address records. Rust returns the first successful `TcpStream`; preserve that divergence unless a compatibility test shows a caller relies on the C loop's final-address behavior.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
