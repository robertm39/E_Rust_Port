# LTB variant worker boundary

## Status

Accepted for Bead `E_Rust_Port-j76.1.45`. The migrated claim that exact C
`fork()`-return child mode remained pending was stale: the safe executable
worker boundary already represents the complete caller-visible child contract.
This follow-up adds a native public-path regression and reconciles the status
documentation. The vendored C source remained unchanged.

## C caller contract

`EGPCtrlCreate("E-LTB wrapper", 1, 1000000)` creates a stdout pipe and forks.
The child restores default `SIGTERM`, routes stdout and `GlobalOut` through the
pipe, applies the fixed soft CPU limit, and returns `NULL`. That `NULL` has one
meaning in `BatchProcessVariants`: process the current concrete variant problem
with the already initialized control state and immediately call `exit(success)`.

The parent drains the child's stdout to EOF, derives its result from the
complete SZS output, counts only Theorem or Unsatisfiable as solved, replays the
captured output, frees the child controller, and prints the Ended marker. No C
caller uses the child-side pointer identity, returns the child to ordinary
parent control flow, or mutates shared state after the branch.

## Rust compatibility decision

Rust retains the existing hidden executable worker. `Command` supplies the
captured stdout pipe and parent-owned process lifecycle. The fixed argument
payload names the batch spec/index, variant, prover, concrete source,
destination, and proportional wall-clock limit. The fresh executable resets
process globals and signal state, applies the same fixed CPU limit on Linux,
reparses the selected batch into isolated owned state, processes exactly one
concrete problem, and exits. The parent uses the same complete-output SZS
classification and theorem/unsatisfiable success set as C.

A direct post-`fork()` Rust branch would require unsafe assumptions about the
runtime, allocator, locks, and inherited library state, while exposing a null
owner return that no safe Rust caller needs. The executable boundary is the
specific compelling reason not to reproduce that internal mechanism. It
preserves the observable isolation, I/O, resource-limit, result, and lifecycle
contract and is portable to Windows, where C's POSIX fork path is unavailable.

## Native public-path regression

`tests/e_ltb_variant_worker.rs` invokes the built `e_ltb_runner` with public
`--variants28`, supplies a real copied `eprover` at the hard-coded `./eprover`
location, and gives the first concrete variant a `$false` clause. The test
proves the full process chain rather than calling the hidden mode directly:

- the parent reports one abstract problem, two variants, and two concrete
  problems;
- round zero launches `E-LTB wrapper` with one core and the 1,000,000-second
  limit;
- the generic controller reports a child PID before replaying its captured
  Unsatisfiable result;
- the concrete destination file contains the Unsatisfiable status and prover
  output;
- the Ended marker follows captured output;
- round one reports the abstract problem already solved; and
- the public command exits successfully with the variant-batch completion
  marker.

When the winning prover causes sibling strategy cleanup, those losing
subprocesses may inherit stderr and emit the existing eprover `OutClose`
broken-pipe diagnostic. C's LTB wrapper captures only child stdout as
`GlobalOut`; the regression therefore permits zero or more exact instances of
that known stderr line and rejects any other diagnostic.

## Performance decision

The public native run takes about five seconds in a debug test build because
the concrete problem enters the normal multi-strategy prover backend. The
extra hidden-worker exec and small batch reparse happen once per attempted
variant problem and are outside proof-search hot paths. Replacing them with an
unsafe inherited Rust runtime is not justified by this startup-only cost; the
existing process boundary also predates this reconciliation, so this slice
does not change runtime performance.

## Validation

- native public variant integration: 1 passed in each of three consecutive
  runs;
- focused batch-variant unit regressions: 4 passed;
- focused LTB-runner unit regressions: 22 passed;
- full library suite: 4,165 passed;
- all binary targets passed;
- integration targets `eprover_schedule`, `e_stratpar`,
  `executable_inventory`, and `e_ltb_variant_worker`: 4, 1, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- bundled-Python `tools/e-interop` discovery: 32 passed;
- `git diff --check`: passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later wording and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
