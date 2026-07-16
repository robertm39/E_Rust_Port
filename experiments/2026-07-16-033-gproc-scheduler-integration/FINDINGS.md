# Generic process-control scheduler integration

## Status

Completed for Bead `E_Rust_Port-j76.1.24` as a complete C/Rust call-site and
ownership audit with a permanent hidden-worker command regression. The vendored
C source remained unchanged. This host has no C compiler or installed WSL
distribution, so executable evidence comes from the native Rust binary and the
checked-in C source.

## Complete C call-site inventory

The generic `EGPCtrl` family has exactly two executing consumers:

- `CONTROL/cco_scheduling.c` owns an `EGPCtrlSet` while it executes multicore
  preprocessing and search schedules. It launches every next strategy that
  fits the remaining core capacity, polls until the first proof-producing child
  completes, replays that child's output, terminates remaining children, and
  reports schedule exhaustion. The child-side `NULL` branch selects the
  strategy/ordering and returns its schedule index to the surrounding prover.
- `CONTROL/cco_batch_spec.c` creates one `E-LTB wrapper` per concrete variant
  problem, drains raw output until EOF, classifies theorem/unsatisfiable as
  solved, copies the accumulated output, and frees the controller.

No other C source file calls the generic controller. E-specific strategy and
batch runner subprocesses use the separate `EPCtrl` family.

## Rust integration mapping

Both consumers execute through production Rust paths:

- `control::scheduling::execute_schedule_multi_core` initializes absolute
  schedule times, reserves cores through `EGPCtrlSet`, launches eligible
  workers in schedule order, polls through the fixed generic-controller entry
  point, removes completed failures, maps the winning descriptor back to its
  schedule index, writes `% Result found by ...` and the captured output,
  cleans all remaining workers, and returns the winning exit status. Exhausted
  search schedules can enter the filtered default-schedule retry wrapper.
- `prover::eprover` connects that coordinator to explicit preprocessing and
  nested search worker modes. The worker payload carries preprocessing and
  search indices, strategy names, C ordering values, absolute CPU budgets, and
  the original executable invocation. A new pure-command regression pins the
  complete payload for both layers.
- `control::batch_spec` drains one `EGPCtrl` to completion for each variant and
  `prover::e_ltb_runner` supplies the concrete executable worker.

Existing coordinator tests cover first success, capacity sequencing,
exhaustion, filtered default retry, the retry threshold, and the unschedulable
edge. Existing batch tests cover captured generic output and C's restricted
variant-success result set. Native integration tests execute the real eprover
binary through preprocessing and nested search workers, replay the winning
proof, and verify multi-layer resource output.

## Scope boundary

This audit proves that every generic-controller consumer is connected and that
the parent-side scheduler lifecycle is exercised. Exact differences caused by
replacing C's inherited post-`fork()` proof state with a fresh executable—such
as parsed-state reuse, parent-request signaling, resource-output ordering, and
parent/child CPU accounting—remain the separately tracked multicore-scheduling
compatibility work. They are not treated as evidence that a call site is
missing.

## Performance decision

This slice factors existing command construction into pure helpers without
changing the spawned arguments, process count, polling backend, or proof-search
path. The additional code runs only while assembling worker arguments, so a
benchmark is not warranted.

## Validation

- focused hidden-worker command regression: 1 passed
- focused `control::scheduling::tests`: 6 passed
- native `eprover_schedule` integration: 3 passed
- `cargo fmt --all -- --check`
- `cargo test --locked --lib --quiet`: 4,120 passed
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
