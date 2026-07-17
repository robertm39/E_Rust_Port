# TCP channel ownership reconciliation

## Status

Completed for Beads `E_Rust_Port-j76.2.95` and
`E_Rust_Port-j76.4.862`. The vendored C source remained unchanged. Rust's
owned stream is the completed safe representation of the C socket lifetime;
the reusable session boundary retains the real descriptor wherever C observes
it.

## C source contract

`INOUT/cio_multiplexer.c` gives each `TCPChannel` one raw socket descriptor and
two owning message queues. `TCPChannelClose` asserts that the descriptor is
nonnegative, conditionally writes `Closing channel %d\n` to stderr when the
process-global `Verbose` value is nonzero, calls `close`, ignores its return,
and changes the stored descriptor to `-1`. `TCPChannelFree` drains and frees
both queues and calls `close` only when the stored descriptor is still
nonnegative, again ignoring the result.

`CONTROL/cco_esession.c` is the only caller of `TCPChannelClose`. Both call
sites handle a channel read/write error or connection closure, close the
channel, and immediately set the session state to `ESStale`. Stale sessions
return before socket assertions and do not register the descriptor in the
next `fd_set` construction. The separate `PROVER/e_server.c` placeholder
executable uses its own direct socket loop and never calls the reusable
channel/session API.

## Rust ownership decision

`TcpChannel<S>` retains the stream in `Option<S>`. This maps the live C
descriptor to `Some(stream)` and the `-1` sentinel to `None` without allowing
a closed descriptor to remain usable:

- explicit close takes and immediately drops the only stream owner;
- dropping an open channel releases the stream once, like `TCPChannelFree`;
- dropping an explicitly closed channel cannot close it again; and
- `into_inner` explicitly transfers ownership without premature release.

There is no missing `close(2)` failure diagnostic to reproduce: C discards the
return from both close calls. Rust's safe direct double-close request returns a
diagnostic rather than reproducing C's internal debug assertion. Normal
`ESession` control flow cannot make that request because stale sessions leave
the I/O and readiness paths before reaching close again.

`ESession` stores the raw descriptor captured from the live `TcpStream` for the
same two uses present in C: readiness registration and the verbosity-gated
close notice. On a stale transition Rust now writes the exact stable
`Closing channel <descriptor>` line to stderr when global verbosity is
nonzero, ignores any output failure just as C ignores `fprintf`, releases the
owned stream, and marks the session stale. The retained numeric descriptor is
then inert because stale sessions register no read or write interest.

## Regression evidence

Three synthetic drop-count tests prove exact-once close, free-time close, and
ownership transfer independently. The existing close regression still proves
closed reads/writes return the `NWError`-equivalent status and a safe direct
double-close is rejected.

The session tests additionally pin the exact verbose line with a known
descriptor. A production `TcpListener`/`TcpStream` loopback test builds an
`ESession` from the accepted stream, closes it through the stale transition,
proves that the old descriptor is absent from both interest sets, and observes
EOF from the peer. This exercises the current host's real socket lifetime
rather than only a synthetic `Read + Write` owner.

A standalone C transcript is not needed for the exact close line because the
reusable C control modules have no executable caller in this source snapshot;
the source contains the complete literal output and state transition. The
separate legacy `e_server` executable loop remains covered by its existing
socket tests and compatibility audit.

## Performance decision

The ownership representation is unchanged. The only production addition is a
verbosity check on rare error/closure transitions; normal read/write and
readiness paths are unaffected. A performance benchmark is not warranted.

## Validation

- 10 focused `inout::multiplexer` tests;
- 8 focused `control::esession` tests, including a real loopback close;
- serialized all-target/all-feature Rust suite;
- strict all-target/all-feature Clippy with pedantic warnings denied;
- Rust formatting and documentation quality gates; and
- unchanged vendored C worktree.
