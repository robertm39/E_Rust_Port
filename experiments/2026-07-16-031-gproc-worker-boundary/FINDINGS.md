# Generic process-control worker boundary

## Status

Completed for Bead `E_Rust_Port-j76.1.22` as a source-backed compatibility
decision with targeted implementation changes and permanent Rust regressions.
The vendored C source was treated as read-only. This host has no C compiler or
installed WSL distribution, and only the Windows Rust target is installed, so
the Linux-only resource-limit and signal paths were reviewed against their C
sources and narrow FFI wrappers but could not be executed locally.

## C behavior

`EGPCtrlCreate` creates a pipe and calls `fork()`. In the child it restores the
default `SIGTERM` disposition, duplicates the pipe onto stdout, closes the
original pipe descriptors, assigns `GlobalOut = stdout`, applies a soft
`RLIMIT_CPU` when requested, and returns `NULL`. The parent closes its write
end and returns an owned controller containing the child PID and read
descriptor. Cleanup sends `SIGTERM`, waits, and closes the descriptor.

The two executing consumers give the child-side `NULL` return its meaning:

- multicore scheduling enters a strategy-worker branch and returns the
  strategy index to the scheduling coordinator; and
- LTB variant processing enters a wrapper branch, processes one concrete
  problem, and exits.

No caller depends on sharing a Rust object identity across the branch. The
observable contract is a separately executing worker with captured stdout,
fresh signal/global state, the requested CPU limit, and parent-owned lifecycle.

## Rust compatibility mapping

The Rust port deliberately retains its explicit executable-worker design. A
direct post-`fork()` Rust API would require unsafe process-state assumptions and
would violate the repository rule that ordinary porting work use safe Rust.
It would also expose a C-shaped `NULL` return that has no useful ownership-safe
Rust meaning. The existing boundary maps the C behavior as follows:

- `Command::stdout(Stdio::piped())` supplies the child-to-parent stdout pipe;
- hidden schedule and LTB child modes replace the child-side `NULL` branch;
- a fresh executable image isolates mutable globals and resets caught POSIX
  signal dispositions, after which each worker installs only its normal
  handlers;
- schedule-worker argument filtering removes every output-file spelling so
  the worker's logical global output remains the captured stdout;
- schedule workers set their selected hard/schedule CPU limit before search;
- LTB variant children now apply C's fixed 1,000,000-second soft CPU limit on
  Linux and render the same failure/reduction warnings as
  `SetSoftRlimitErr`; and
- generic cleanup now sends `SIGTERM` to the directly owned Unix child before
  waiting, with `Child::kill` retained for failed signaling and non-POSIX
  platforms.

On platforms without POSIX `RLIMIT_CPU`, the LTB child-limit setup is a no-op.
That is an explicit portability boundary rather than emulated wall-clock
termination; the C implementation itself is fork/POSIX-specific.

## Regression evidence

Existing tests already pin exact startup/completion text, captured chunk
assembly, complete-output SZS scanning, worker argument filtering including
all output-file forms, per-strategy CPU configuration, real-binary schedule
worker replay, and direct LTB child behavior. This slice adds a regression for
the fixed LTB child CPU limit's success, reduction, and OS-error warning
mapping. Generic-controller tests continue to exercise cleanup and process-set
ownership on the native Windows hard-termination fallback.

The later reconciliation of migrated Bead `E_Rust_Port-j76.1.45` adds a native
public `--variants28` regression across the real LTB parent, hidden child, and
prover process. That end-to-end evidence is recorded in
[`experiments/2026-07-16-054-ltb-variant-worker-boundary/FINDINGS.md`](../2026-07-16-054-ltb-variant-worker-boundary/FINDINGS.md).

## Performance decision

The worker executable boundary predates this slice and is used only at process
startup. The changes add one Linux `setrlimit` call in the LTB child and replace
immediate Unix termination with C's `SIGTERM`-then-wait lifecycle. Neither
changes proof-search or output-polling hot paths, so a microbenchmark would not
provide actionable compatibility evidence.

## Validation

- focused LTB CPU-limit warning regression: 1 passed
- focused generic process-control tests: 9 passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,116 passed
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
