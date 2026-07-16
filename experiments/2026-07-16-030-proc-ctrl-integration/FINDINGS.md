# E-specific process-control integration

## Status

Completed for Bead `E_Rust_Port-j76.1.21` as a source-backed call-site,
termination, and output-routing audit with targeted implementation changes and
permanent Rust regressions. This host has neither a C compiler nor an installed
WSL distribution, and only the Windows Rust target is installed. A fresh C
executable and an executing POSIX signal regression were therefore unavailable
in this session; the narrow Unix FFI follows the same documented boundary and
safety-comment pattern as the existing signal module.

## Complete C call-site inventory

The E-specific `EPCtrl` family has two executing C consumers:

- `PROVER/e_stratpar.c` launches eight E strategy processes, reports no-proof
  exits to `GlobalOut`, and writes the winning process's accumulated output to
  `GlobalOut`.
- `CONTROL/cco_batch_spec.c` launches filtered E runners, reports polling
  failures and status/progress to `GlobalOut`, routes successful proof output
  to a socket or destination stream when distinct from `GlobalOut`, and also
  copies it to `GlobalOut` in interactive mode.

Main `eprover` scheduling uses the separate `cco_gproc_ctrl` fork/controller
family. `cco_esession` owns an optional E-specific process set and registers its
descriptors, but `ESessionDoIO` has an intentionally empty subprocess branch.
There is therefore no missing main-scheduler or server-consumption call site to
add. Rust already mirrors these ownership boundaries.

## Termination policy

C `EPCtrlCleanup` sends `SIGTERM` to the PID printed by E, clears that field,
then calls `pclose` and waits. Its parent signal handler cleans temporary files
and reraises termination; it does not keep a global registry that walks local
`EPCtrlSet` values.

Rust previously called `Child::kill`, which maps to immediate forced
termination rather than POSIX `SIGTERM`. Normal Unix cleanup now calls libc
`kill(owned_child_id, SIGTERM)` through a narrow safe wrapper and then waits.
If signaling fails, or on Windows where there is no POSIX `SIGTERM`, it falls
back to `Child::kill` and waits. Reader-thread joining and optional registered
temporary-file removal retain their existing order after the child wait.

The Rust constructor deliberately signals the `Child` it owns rather than
trusting arbitrary PID text from subprocess stdout. This is equivalent for the
production structured-spawn path, where E is the direct child, and avoids an
unsafe cross-process kill if a wrapper or test prints a forged/reused PID. The
opt-in shell compatibility constructor consequently terminates its owned shell;
that safety boundary is preferred to reproducing C's unchecked PID trust.

Like C, Rust does not add a process-global child registry to its parent signal
handler. Cleanup is deterministic on ordinary set deletion/drop; abrupt parent
termination follows the already ported signal and temporary-file policy.

## Output-routing decision

The reusable poller receives its diagnostic writer from the caller rather than
hardwiring a global stream. That writer is the selected executable global
output in both real consumers. Existing exact regressions cover:

- `% No proof found by <name>` on the polling writer;
- `e_stratpar` proof versus final `GaveUp` output;
- batch global status/progress output;
- destination status plus complete proof output;
- socket-only proof output without destination duplication;
- interactive duplication of proof output to global output; and
- global/external/socket `GaveUp` routing.

The socket regression now also asserts that a noninteractive socket proof is
absent from global output, pinning the negative side of C's route selection.

## Performance decision

Output routing and call-site ownership are unchanged. Unix cleanup replaces a
forced signal with C's graceful signal and both paths wait for the child; this
is lifecycle behavior, not a steady-state hot path, so a benchmark is not
warranted.

## Validation

- focused `inout::signals::tests`: 18 passed
- focused `control::proc_ctrl::tests`: 17 passed
- focused `prover::e_stratpar::tests`: 9 passed
- focused `control::batch_spec::tests`: 48 passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,115 passed
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
