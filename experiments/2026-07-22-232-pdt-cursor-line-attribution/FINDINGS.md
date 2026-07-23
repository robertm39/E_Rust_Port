# PD-tree cursor line attribution

## Question

Which operations dominate the const-specialized first-order PD-tree cursor,
and can optimized line tables identify a bounded next candidate without
materially changing the deterministic workload?

## Setup

- Source: commit `298e0d64` (`Specialize PD-tree search cursor mode`), accepted
  Experiment 231.
- Build: ordinary release optimization plus `CARGO_PROFILE_RELEASE_DEBUG=1`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Accepted profile:
  `.artifacts/experiments/2026-07-22-231-specialize-pdt-cursor/rust-callgrind-specialize-pdt-cursor.out`.
- Line-table profile:
  `.artifacts/experiments/2026-07-22-232-pdt-cursor-line-attribution/rust-callgrind-pdt-cursor-lines.out`.

## Representativeness

The line-table binary reaches the exact 4,873-processed-clause LUSK6 proof at
9,923,797,474 instructions. This is 232,702 instructions or 0.002345% above
the accepted 9,923,564,772-instruction baseline, so its attribution is
representative.

## Attribution

The first-order cursor remains the largest isolated hot path. Its single-caller
`advance_variable_query` helper is invoked 2,303,361 times and accounts for
69,101,252 instructions at the call edge. Each call moves the pending query
term from `query_stack` into `query_steps` and returns the new step index. No
allocation, search-order, constraint, or binding policy is selected there.

The next bounded candidate is therefore forcing this helper into the cursor,
as already justified separately for the accepted frame-pop helper. The
candidate must still pass whole-program Callgrind and native timing because
prior local inline improvements have sometimes caused global layout
regressions.

## Decision

Keep executable source unchanged in this diagnostic experiment. Test forced
`advance_variable_query` inlining separately as Experiment 233.

## Reproduction

```bash
CARGO_PROFILE_RELEASE_DEBUG=1 cargo build --locked --release --bin eprover \
  --target-dir target-wsl-232-pdt-cursor-lines
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pdt-cursor-lines.out \
  target-wsl-232-pdt-cursor-lines/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
