# Default-parallel test isolation

## Status

Completed for Bead `E_Rust_Port-9yb`. The required all-target/all-feature gate
is deterministic under Cargo's default parallel runner across ten consecutive
post-change runs. No production behavior changed, and the vendored C source
remained unchanged.

## Question

Which mutable process or executable-worker state causes the default-parallel
all-target/all-feature suite to intermittently fail the represented `$let`
formula-owner regression or the real hidden LTB variant-worker integration
test, despite both tests passing in isolation?

## Reproduction

[`run_stress.ps1`](run_stress.ps1) repeats the required quality-gate command
with Cargo's default parallel test runner. It retains hashes and timing for
successful runs, preserves complete streams only for failures, and writes the
compact [`results-summary.json`](results-summary.json).

```powershell
& experiments\2026-07-17-049-parallel-test-isolation\run_stress.ps1 `
  -Iterations 10
```

## Findings

### Formula parser state

The represented `$let` regression already takes the repository-wide
`global_state_lock`, whose guard resets the current test thread's problem type
at acquisition and release. The underlying problem type is also thread-local,
not process-global, because server and test threads must parse independent
dialects. Ten unchanged-tree baseline runs did not reproduce the historical
type failure.

A new deterministic regression removes timing from that audit: one thread
holds `ProblemType::HigherOrder` across barriers while another parses a FOF
Boolean `$let` through the represented formula-owner path. The FOF parse
remains first-order, the other thread remains higher-order, and both succeed.
This would fail with C-shaped process-global dialect state even if an ordinary
parallel suite happened to schedule the tests favorably.

### LTB worker timeout

The original integration fixture gave each problem a 12-second wall-clock
limit. Each hidden variant child can fan out up to eight real prover runners.
Running four complete parent/hidden-child/prover pipelines concurrently made
the historical failure reproducible: after the first `+1` run exhausted that
test-only budget, production correctly tried the `_1` concrete path, which the
fixture intentionally had not created. Two reproductions took about 21.6
seconds and reported `Cannot open file Problems/prob__1.p`; this was a
load-sensitive fixture deadline, not shared path state.

The permanent integration now launches four complete pipelines concurrently,
uses an atomic serial in addition to PID/time for unique test directories, and
gives the test batch a 60-second per-problem / 180-second overall budget. The
production scheduler, limits, fallback behavior, and worker protocol are
unchanged. After the fixture correction, 20 direct iterations and the 10
iterations retained by [`run_ltb_stress.ps1`](run_ltb_stress.ps1) all passed,
for 120 successful real hidden-worker pipelines.

## Validation

- post-change default-parallel all-target/all-feature suite: 10/10 exact exits
  passed, with 4,257 library tests plus every binary/integration target per run;
- focused concurrent formula-owner dialect regression: passed;
- four-pipeline LTB hidden-worker stress: 30/30 iterations passed after the
  test-only deadline correction;
- strict all-target/all-feature pedantic Clippy: passed; and
- formatting and all four C-source documentation integrity gates: passed.
