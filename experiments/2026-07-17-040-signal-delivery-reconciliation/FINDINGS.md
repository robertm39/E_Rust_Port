# Signal-delivery reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.97`. The vendored C source remained
unchanged. No Rust implementation change was needed: the initial-port signal
contract is source-aligned, covered by deterministic outcome tests, and backed
by retained Linux delivery and current C/Rust executable evidence.

## C and Rust contract map

`INOUT/cio_signals.c:73-180` defines three observable paths:

- `ESignalSetup` captures the process hard CPU limit and installs
  `ESignalHandler`;
- the first soft `SIGXCPU` latches `TimeIsUp`, resets the process limit to the
  captured system maximum, rearms the next hard limit, and reinstalls the
  handler, while a hard `SIGXCPU` writes the `ResourceOut` banner directly to
  `GlobalOutFD`, reports the fatal diagnostic, and exits with status 8; and
- `SIGTERM`/`SIGINT` cleans registered temporary files once, restores the
  default action, and re-raises the signal. The scheduler-specific handler
  increments `SigTermCaught` and restores the default `SIGTERM` action.

Rust preserves those transitions as `SignalOutcome` and
`SchedulerSignalOutcome` values in `src/inout/signals.rs`. Normal Linux builds
install libc trampolines for `SIGXCPU` and scheduler `SIGTERM`; the hard-limit
trampoline performs descriptor writes before exiting, and the termination
trampoline restores the default action and re-raises. Tests retain a
non-mutating boundary so the state transitions, cleanup-once rule, exact bytes,
and exit mapping can be checked deterministically.

On platforms without the Linux trampoline, executable proof search checks the
same process-CPU deadline cooperatively and feeds the hard stop through the
same finalizer. This is the compatibility abstraction, not a missing native
shutdown mechanism: a Windows Job Object quota would terminate the process
before E could emit its banner, SZS status, diagnostic, and status-8 mapping.

## Retained Linux delivery evidence

Commit `e11b51e91bb1831aebc8e3eb735d20e6e2288d80` installed the hard Linux
`SIGXCPU` exit path on 2026-07-06. The later WSL-native runs recorded under
`.artifacts/experiments/2026-07-13-005-hen011-divergence/` therefore exercise
that production trampoline rather than the cooperative Windows path.

Three independent full-limit logs contain the exact doubled-comment failure
banner, `SZS status ResourceOut`, fatal stderr diagnostic, status 8, and about
60 seconds of user CPU:

| Raw log | Exit | User CPU | Wall time |
| --- | ---: | ---: | ---: |
| `rust-debug-only-subsume-asserts-full.txt` | 8 | 59.47 s | 58.14 s |
| `rust-inline-match-full.txt` | 8 | 59.45 s | 58.34 s |
| `rust-reused-subsume-scratch-full.txt` | 8 | 59.36 s | 58.38 s |

The `rust-candidate-full.txt` and `rust-borrowed-comparators-full.txt` logs also
place the direct failure banner and stderr diagnostic before pending normal
stdout, preserving the C signal-time buffering boundary.

## Current reference comparison

The same-tree comparison
`.artifacts/e-compare/20260717-002556-450711/comparison.json` used E 3.3.5 at
upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, built with GCC 13.3
on Ubuntu 24.04.4 under WSL2. Its silent 60-second cases compare actual C
`SIGXCPU` expiry with native-Windows Rust cooperative expiry:

| Problem | C result | Rust result | Comparison |
| --- | --- | --- | --- |
| `SWB008+1.p` | `ResourceOut`, exit 8, 60.116 s | `ResourceOut`, exit 8, 55.907 s | no mismatches; normalized output equal |
| `SWV851-1.p` | `ResourceOut`, exit 8, 60.105 s | `ResourceOut`, exit 8, 56.877 s | no mismatches; normalized output equal |

The synthetic one-second `LUSK6` case is not a signal-semantics mismatch: C
finishes in 0.555 seconds, whereas Rust reaches its limit in 0.972 seconds.
That retained comparison correctly reports a proof-versus-timeout performance
difference.

## Evidence boundary and decision

This host no longer has a registered WSL distribution, so this reconciliation
does not claim a fresh Linux run. It relies on raw Linux artifacts created
after the trampoline implementation and on the current same-tree comparison.

There is no retained live-injection artifact for the normal Linux
`SIGTERM`/`SIGINT` trampoline. That path is source-aligned and deterministically
covered through cleanup-once, outcome, scheduler-latch, and default-reset state
tests, but live signal delivery is not overclaimed here. The broader question
of replacing high-level signal-handler work with a smaller async-signal-safe
boundary is already tracked by the `cio_signals` Change Later work; it is not
an incomplete initial-port surface.

## Validation

- focused `inout::signals` unit tests;
- hard and soft executable CPU-limit regressions;
- deterministic direct-output-before-buffered-output regression;
- scheduler parent-request cleanup regression;
- Rust formatting and documentation quality gates; and
- unchanged vendored C worktree.
