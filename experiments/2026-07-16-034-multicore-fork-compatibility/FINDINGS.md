# Multicore fork-boundary compatibility

## Status

Completed for Bead `E_Rust_Port-j76.1.25`. The vendored C source remained
unchanged. This slice pins the observable scheduler contracts around C's
`fork()` boundary and records the one deliberate state-transfer difference in
the portable Rust implementation.

## C contracts

`CONTROL/cco_scheduling.c:245-322` establishes these contracts:

- schedule initialization subtracts `GetTotalCPUTime()`, which is the parent
  process's own CPU time;
- a forked child inherits the caller's parsed proof state, selects its schedule
  cell, resets SIGTERM handling, and returns the schedule index to the caller;
- the parent polls workers, replays the first successful child's output, prints
  its own resource footer, and terminates the remaining children;
- a caught scheduler SIGTERM cleans workers and exits with `PARENT_REQUEST`;
- schedule exhaustion prints the parent resource footer before returning
  `SCHEDULE_DONE`.

`PROVER/eprover.c:709-731` adds a second CPU-clock rule for nested search. The
default retry budget subtracts `GetTotalCPUTimeIncludingChildren()`, but the
second `ExecuteScheduleMultiCore` call again initializes its schedule by
subtracting the parent's own CPU time. `PrintRusage` includes self and waited
children, so a successful nested run naturally emits the winning search child,
preprocessing worker, and outer parent resource summaries in that order.

## Rust compatibility work

- The coordinator has an explicit `ParentRequest` outcome with C exit status
  14. The executable installs the scheduler SIGTERM handler, observes its
  caught-signal latch while polling, and cleans live workers before returning.
- Schedule initialization uses self CPU. Default-retry budget calculation uses
  self-plus-children CPU, while retry initialization again receives self CPU.
- Unix resource accounting continues to use `getrusage`. Windows records kernel
  process times for every waited child. Generic schedule workers also propagate
  the aggregate resource footer reported by a nested child, avoiding loss of
  grandchild CPU at the next parent boundary.
- The original real-binary resource regression required three nested resource
  summaries after the proof. A later direct C comparison showed that the search
  leaf must suppress its ordinary footer through `SilentTimeOut`, leaving the
  nested and outer coordinator summaries only. The corrected two-footer
  contract and nondecreasing totals are retained in
  [`experiments/2026-07-18-102-auto-schedule-duplicate-closure/FINDINGS.md`](../2026-07-18-102-auto-schedule-duplicate-closure/FINDINGS.md).
- An input containing `-` is read once by the scheduling parent into a
  registered temporary file in the native temporary directory. The snapshot
  path is part of the private worker protocol, and each exec worker reads it as
  its first standard-input occurrence. A second `-` still observes EOF. A
  real-binary regression proves that piped `$false` reaches both preprocessing
  and search workers and produces the proof instead of the pre-fix spurious
  `Satisfiable` result.

The Linux signal trampoline is compiled only on Linux. This Windows host can
pin the handler state machine and coordinator cleanup, but cannot deliver a
native Unix SIGTERM to that path.

## Safe state-transfer decision

The C child returns into the caller with a copy-on-write view of pointer-rich
parser, signature, proof-state, term-bank, and heuristic state. Reproducing that
with `std::process::Command` would require a versioned serialization format for
the complete internal object graph. Calling `fork()` from the Rust process and
continuing through allocation-heavy Rust code would also violate the project's
safe-Rust rules and is not available on Windows.

The portable implementation therefore keeps explicit exec workers. Standard
input is snapshotted because it is ephemeral; named files are assumed stable
for the duration of one run and are reparsed/reprocessed by workers. This
preserves supported proof results and process isolation, but it does not
preserve C's file-mutation race semantics or its copy-on-write preprocessing
cost. A future state-transfer format is warranted only if representative
scheduled workloads show that repeated preprocessing materially dominates
search; it should not be introduced merely to emulate pointer identity.

## Startup benchmark

The native release binary was measured for five invocations on the trivial
file `cnf(stdin_false, axiom, ($false)).`:

| Mode | Mean wall time |
| --- | ---: |
| `--auto` | 25.116 ms |
| `--auto-schedule=1` | 69.320 ms |

The scheduled/direct ratio is 2.760 on this startup-dominated input, or about
44 ms of additional process/scheduler overhead. This is deliberately a
worst-case boundary measurement, not a C comparison and not evidence about
long-running proof-search throughput. It makes the remaining reparse cost
visible rather than treating the safe exec boundary as free.

## Validation

- `cargo test --locked --lib --quiet`: 4,122 passed, including parent request,
  two-clock retry, and nested-resource parsing
- `cargo test --locked --bins --quiet`: all binary targets passed
- `cargo test --locked --test eprover_schedule --quiet`: 4 passed, including
  nested snapshot replay and three-layer resource ordering
- `cargo test --locked --test executable_inventory --quiet`: 1 passed
- release `eprover` build and five-run startup benchmark
- `cargo fmt --all -- --check`
- `cargo check --locked --all-targets`
- `cargo clippy --locked --all-targets -- -D warnings`
- C-source coverage: 492 source files covered by 266 unit docs
- Change Later wording: 269 Markdown files checked
- local Markdown links: 269 Markdown files checked
- regeneration preservation: manual sections preserved in 268 Markdown files
- `git diff --check`
