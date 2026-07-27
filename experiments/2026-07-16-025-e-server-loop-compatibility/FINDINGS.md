# Legacy `e_server` loop compatibility audit

## Status

Completed for Bead `E_Rust_Port-j76.1.16` as a source-backed compatibility
decision. This host has no installed WSL distribution, so a fresh live C/Rust
socket transcript was unavailable; permanent Rust regressions pin every stable
line and event-order distinction found in the C source.

## Question

Does the Rust nonblocking implementation preserve the C executable's logical
`select`-loop marker cadence, connection-close/read-error text, failed-`accept`
quirks, and platform-dependent descriptor rendering?

## C source evidence

`PROVER/e_server.c` prints `Main loop` at the start of every iteration,
immediately before rebuilding the descriptor sets and blocking in `select`.
After any selected event is processed, control returns to the top and prints
the next marker. Repeated nonblocking readiness probes in Rust are an
implementation detail and must not create extra markers while logically
waiting for the same event.

For the active socket, a failed TCP-string read prints exactly `Read error` if
the network error flag is set, or `Connection closed` on ordinary EOF. Neither
line includes `errno` or a host-specific suffix. The connection is then closed
and the one active-client slot is released.

The listener branch does not check `accept` before using its result. With no
active client, an `accept` failure therefore prints `Accepted -1` and retains
the empty slot. With an active client, it silently calls `close(-1)`. These are
observable bugs in the drop-in executable contract and are distinct from the
reusable control implementation.

`CONTROL/cco_eserver.c::EServerAccept` does check failure. It saves `errno`,
calls `SysWarning("Failure to accept connection")`, and returns false. The
stable output is two lines:

```text
e_server: Warning: Failure to accept connection
e_server: <host-specific error>
```

The successful `Accepted %d` value is a live descriptor and is inherently
process- and platform-dependent.

## Rust decision

- A small marker state prints and flushes `Main loop` once per logical blocking
  wait. It is reset only when an active-socket, listener, or failed-accept event
  makes progress.
- Active EOF and read failure retain the exact stable C lines and release the
  connection slot.
- Injectable listener polling pins `Accepted -1` when the slot is empty and
  silent rejection when it is occupied.
- `EServer::accept` accepts a diagnostic writer, emits the control API's exact
  stable two-line warning plus the host error, and returns `Ok(false)`.
- The interop normalizer rewrites only `Accepted` followed by decimal digits to
  `Accepted <DESCRIPTOR>`. It deliberately does not match the `-1` sentinel.

## Regression coverage

Focused tests cover:

- one marker across repeated idle polls and a new marker after progress;
- exact `Read error` output without a platform suffix;
- exact `Connection closed` output plus active-slot release;
- exact successful descriptor output and closure of a rejected second client;
- failed accept with and without an active client;
- exact stable control-API warning lines with a synthetic host suffix; and
- positive-descriptor-only comparison normalization.

## Performance decision

The marker remains one Boolean of state, and the private generic accept helper
is monomorphized in production. Poll ordering, socket operations, and the
10-millisecond idle backoff are unchanged. The new work occurs only on rare
accept failures or in the comparison harness, so a performance benchmark is
not warranted.

## Validation

- focused `prover::e_server::tests` passed: 19 passed
- focused `control::eserver::tests` passed: 3 passed
- interop harness tests passed: 26 passed
- Python bytecode compilation for the interop harness and tests
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,106 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 3 passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
