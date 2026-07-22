# Rejected removal of PD-tree initialization occurrence clear

## Question

After Experiment 205 made substitution-cursor cleanup exit-owned, can
`PdTree::record_search_init` likewise omit its apparently redundant
`search_cursor = None` assignment?

## Setup

- Parent source: commit `de626f34` (`Remove redundant PD-tree init reset`),
  accepted Experiment 205.
- Candidate: replace the initialization-time occurrence-cursor clear with a
  debug lifecycle assertion and extend the materialized-cursor test across
  exit plus reinitialization.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-205-remove-pdt-init-cursor-reset/rust-callgrind-remove-pdt-init-cursor-reset.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-206-remove-pdt-init-occurrence-clear/rust-callgrind-remove-pdt-init-occurrence-clear.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Deterministic result

The candidate preserves the expected LUSK6 proof and retires 10,867,147,423
instructions. This is 8,050,905 below the 10,875,198,328-instruction parent,
a 0.074030% reduction. The hypothetical C/Rust ratio is 2.068215.

The `record_search_init` path falls from 74,573,815 to 66,677,764
instructions, saving 7,896,051 or 10.588235%. The local saving accounts for
nearly all of the whole-program reduction.

## Native result

Both binaries were warmed before 16 alternating native Windows pairs. The
candidate mean is 2.012258 seconds versus 1.999959 for the parent, a 0.614965%
regression. Candidate median is 2.014632 versus 1.991912, a 1.140575%
regression. The parent wins 11 pairs and the candidate five; mean paired
regression is 0.710844%. All 32 runs prove with exit zero. Both executables are
8,632,320 bytes.

## Validation

- All 41 focused PD-tree tests pass with the candidate and after restoration.
- Formatting passes.
- The candidate reaches the expected LUSK6 proof and exit zero under
  Callgrind and in every native run.
- Compatibility matrices were intentionally skipped after native timing
  rejected the candidate.

## Decision

Reject and restore the parent source. The initialization assignment is
logically redundant and saves 0.074% deterministic instructions when removed,
but the production wall-time regression is larger and consistent across mean,
median, paired mean, and win count. Preserve the result to avoid retrying this
lifecycle boundary.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-remove-pdt-init-occurrence-clear.out \
  target-wsl-206-remove-pdt-init-occurrence-clear/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
