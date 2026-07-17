# Network socket reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.96`. The vendored C source remained
unchanged. Rust now preserves the stable two-line `SysError` shape for socket
setup and connection failures, and a real loopback regression exercises the
safe server/client constructors used by the network executables.

## C source contract

`INOUT/cio_network.c:64-151` establishes the low-level setup behavior:

- server creation opens an IPv4 TCP socket, sets `SO_REUSEADDR`, binds to
  `INADDR_ANY`, and returns the raw descriptor;
- client creation resolves every address, attempts every result even after a
  successful connection, and returns only the outcome for the final address;
  and
- failed client attempts are closed, but earlier successful descriptors are
  overwritten without being closed.

The exported wrappers at lines 532-598 add the observable diagnostics.
`CreateServerSock` and connect failure use `SysError`, producing the stable
message line followed by the host error line and status 7. Resolver failure
uses `gai_strerror` inside the stable one-line `Could not resolve address
(...)` diagnostic. `Listen` applies backlog 10 after binding.

The only production server call sites are `EServerListen`, `e_server`, and
`e_deduction_server`; all call `Listen` immediately after creation, without
exposing the bound-only descriptor to another operation. `e_client` is the
only `CreateClientSock` caller.

## Safe Rust boundary

Rust returns owning `TcpListener` and `TcpStream` values. On Linux and Windows,
the scoped platform modules create the raw server socket, apply
`SO_REUSEADDR`, bind, and call `listen` with backlog 10 before transferring
ownership into `TcpListener`. The exported `listen` compatibility function is
therefore an idempotent no-op. Folding the two adjacent C calls avoids
representing a bound-but-not-listening socket as a safe `TcpListener`.

Platform creation now returns `io::Result<TcpListener>` internally instead of
discarding the socket error. The fatal wrapper renders C's stable first line,
the initialized process program name, the host error text, and status 7. The
Windows boundary reads `WSAGetLastError` before closing a failed socket and
retains the direct `WSAStartup` status; the Linux boundary captures `errno`
before cleanup.

Rust's Unix standard-library resolver obtains its detail directly from
`gai_strerror` and wraps it in `failed to lookup address information: ...`.
The compatibility formatter removes only that standard-library wrapper, so
the C `Could not resolve address (<gai_strerror detail>)` shape is recovered.
Other resolver errors retain their host-native text.

## Ownership and descriptor decisions

- Descriptor numbers are runtime OS allocation results, not cross-process
  identities. Rust keeps the real listener/session descriptor wherever C uses
  it for readiness or prints successful `Accepted <descriptor>` output; the
  comparison harness normalizes only successful platform-dependent values.
- C emits no close diagnostic in this path. It ignores the return from closing
  a failed client attempt and fails to close a server descriptor after
  `setsockopt` or `bind` failure. Rust closes all still-owned sockets and is
  therefore equally silent without reproducing the leaks.
- The final-address client outcome is preserved exactly. Rust intentionally
  drops an earlier successful stream when a later result replaces it, rather
  than leaking it. That resource-lifetime difference remains in
  `E_Rust_Port-j76.4.865` for the post-compatibility review.
- Exact non-Linux/non-Windows reuse setup and the folded listen boundary remain
  part of the existing `E_Rust_Port-j76.4.864` platform review. The reference
  Linux target and the project's native Windows target use the explicit C
  backlog/reuse path; other targets retain the standard-library fallback.

## Executable evidence

The new network regression creates the real server wrapper on an ephemeral
port, passes it through the compatibility `listen` call, connects with the real
client wrapper over IPv4 loopback, accepts the stream, and exchanges `ping` and
`pong`. This validates Winsock startup, reuse/bind/listen, safe ownership,
resolution, final-address connect, accept, and bidirectional I/O together on
the current Windows host.

Existing executable evidence covers the consumers:

- the deduction-server loopback regression keeps simultaneous isolated FO and
  THF clients open and validates framed protocol responses;
- the legacy `e_server` regressions pin successful real descriptor output,
  failed-accept behavior, connection closure, and the one-client rule; and
- the client protocol regression pins the `hello`/`add`/problem/`prove`
  transaction over an injectable bidirectional stream.

A fresh live C socket transcript is unavailable because this host has no
registered WSL distribution. The server/client setup decision is source-backed
and the permanent loopback regressions exercise the production Rust boundary.

## Validation

- 20 focused `inout::network` tests;
- real loopback server/client constructor and byte-exchange regression;
- deterministic two-line system-error and resolver-detail regression;
- serialized all-target/all-feature suite: 4,242 library tests plus all binary
  and integration targets;
- strict all-target/all-feature Clippy with pedantic warnings denied;
- Rust formatting and documentation quality gates; and
- unchanged vendored C worktree.
