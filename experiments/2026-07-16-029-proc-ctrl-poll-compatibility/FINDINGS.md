# Process-control poll compatibility

## Status

Completed for Bead `E_Rust_Port-j76.1.20` as a source- and platform-backed
compatibility decision with permanent Rust regressions. No fresh C executable
was available because this host has neither a C compiler nor an installed WSL
distribution.

## C behavior

`EPCtrlSetGetResult` builds a read `fd_set`, waits in `select` for 500 ms, and
silently returns no result when `select` returns `-1`. Otherwise it scans every
integer descriptor from zero through the maximum descriptor. For each ready
owned descriptor it calls `fgets` exactly once, then:

- retains theorem/unsatisfiable processes and remembers the latest descriptor;
- prints `% No proof found by <name>` and deletes satisfiable,
  counter-satisfiable, or failed processes; and
- continues scanning after either outcome.

The later ready proof therefore wins. `EPCtrlGetResult` does not distinguish a
pipe read error from EOF: a null `fgets` result becomes failure when no earlier
result was recognized.

## Portable backend decision

Rust keeps one reader thread per child and polls the resulting channels. On
Windows, the official Winsock [`select` documentation](https://learn.microsoft.com/en-us/windows/win32/api/winsock2/nf-winsock2-select)
requires socket handles and reports `WSAENOTSOCK` for a non-socket entry, so it
cannot wait on `ChildStdout` pipe handles. Splitting `EPCtrl` into unrelated
Unix and Windows ownership models would add lifecycle and cleanup risk without
adding an observable error: the C code discards selector errors.

The retained channel implementation preserves the observable nonempty-set
contract:

- `EPCTRL_SET_WAIT_TIMEOUT` and `EPCtrlSet::get_result` expose the fixed 500 ms
  C wait used by `e_stratpar` and the default batch runner;
- each poll consumes at most one queued message from each process;
- `BTreeMap` iteration gives the same ascending descriptor scan;
- later proofs replace earlier proofs while the scan continues;
- no-proof EOF processes are reported and deleted in the same poll; and
- a reader-thread I/O error now becomes EOF/failure like null `fgets`; and
- an empty process set still consumes the requested timeout, preventing the
  exhausted-filter batch loop from spinning until its wall-clock limit.

The portable loop checks channels at most every 10 ms. A disconnected internal
channel without its normal EOF/error message remains a Rust infrastructure
diagnostic rather than being misclassified as prover failure.

## Performance decision

The production backend and its 10 ms maximum polling interval are unchanged.
The new public wrapper removes duplicated 500 ms constants and the read-error
branch now performs the cheaper existing EOF path. A benchmark is not
warranted.

## Validation

- focused `control::proc_ctrl::tests`: 17 passed
- focused `prover::e_stratpar::tests`: 9 passed
- focused `control::batch_spec::tests`: 48 passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,114 passed
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
