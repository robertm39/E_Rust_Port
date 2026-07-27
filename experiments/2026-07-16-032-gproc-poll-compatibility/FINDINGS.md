# Generic process-control poll compatibility

## Status

Completed for Bead `E_Rust_Port-j76.1.23` as a source- and platform-backed
compatibility decision with permanent Rust regressions. No fresh C executable
was available because this host has neither a C compiler nor an installed WSL
distribution.

## C behavior

`EGPCtrlSetGetResult` builds a read `fd_set`, calls `select` with a 500-ms
timeout, and silently returns no result when `select` fails. Otherwise it scans
integer descriptors from zero upward and calls `EGPCtrlGetResult` once for each
ready owned descriptor until a proof-producing process completes. A completed
failure is deleted and scanning continues; the first completed theorem,
unsatisfiable, satisfiable, or counter-satisfiable process stops the scan and is
returned. An empty set still calls `select(1, ...)` with empty sets and consumes
the timeout.

`EGPCtrlGetResult` performs one raw `read` of at most `EGPCTRL_BUFSIZE - 1`
bytes. A read error is a fatal `SYS_ERROR`. Nonempty bytes are accumulated and
do not trigger result classification. EOF scans the complete accumulated
output, waits for the child, prints the completion line, and yields either a
proof result or failure.

## Portable backend decision

Rust retains one blocking reader thread per child and polls the resulting
channels. Windows Winsock
[`select`](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-select)
accepts socket handles rather than anonymous child-pipe handles, so it cannot
provide one shared implementation for the project's native Windows target.
Replacing the established channel boundary with separate platform ownership
models would add lifecycle risk without exposing a missing C diagnostic: the C
caller discards selector errors.

The channel implementation now preserves the observable contract:

- `EGPCTRL_SET_WAIT_TIMEOUT` and `EGPCtrlSet::get_result` expose the fixed
  500-ms poll used by multicore scheduling;
- each call consumes at most one queued raw chunk or EOF/error message from
  each process;
- `BTreeMap` order matches ascending descriptor scanning;
- scanning stops at the first completed proof-producing process;
- completed failures are deleted and scanning continues;
- an empty process set consumes the requested timeout instead of returning
  immediately; and
- reader-thread I/O failures use `SYSTEM_ERROR`, matching C's fatal
  `SYS_ERROR` class.

The portable loop checks channels at most every 10 ms. A disconnected internal
channel without its normal EOF/error message remains an interface diagnostic;
it represents a broken Rust reader-thread invariant rather than a C selector or
pipe-read outcome.

## Performance decision

The production reader-channel backend and 10-ms maximum polling interval are
unchanged. The fixed wrapper removes a duplicated scheduler constant, the
empty-set path now performs C's intended wait, and the error change only affects
exceptional I/O. A benchmark is not warranted.

## Validation

- focused `control::gproc_ctrl::tests`: 12 passed
- focused `control::scheduling::tests`: 6 passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,119 passed
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
