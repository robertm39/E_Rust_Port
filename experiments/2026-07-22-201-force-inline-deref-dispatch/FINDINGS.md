# Forced inline dereference dispatcher

## Question

After inlining the always-dereference step and loop, does forcing the shared
`term_deref_if_changed` dispatcher into its hot callers improve production
performance despite the larger code footprint?

## Setup

- Parent source: commit `0877753f` (`Force-inline hot always-dereference
  loop`), accepted Experiment 200.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-200-force-inline-deref-always-loop/rust-callgrind-force-inline-deref-always-loop.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-201-force-inline-deref-dispatch/rust-callgrind-force-inline-deref-dispatch.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`term_deref_if_changed` now uses `#[inline(always)]`. A narrow Clippy
expectation records the pinned whole-prover and native evidence. Always, never,
and once dereference dispatch, applied-free-variable handling, binding
traversal, and mutable `DerefType` updates are unchanged.

## Performance result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,984,397,728 instructions. This is 84,887,032 below the
11,069,284,760-instruction parent, a 0.766870% whole-prover reduction. The
deterministic C/Rust ratio improves from 2.106685 to 2.090530.

The standalone 347,042,328-instruction dispatcher disappears, but work is
redistributed across several callers and unrelated hot functions shift under
the new code layout. This experiment therefore claims the pinned whole-binary
change rather than presenting a misleading local aggregate.

Both binaries were warmed before 16 alternating native Windows pairs. The
candidate wins 10 pairs and the parent wins 6. Candidate mean is 2.085015
seconds versus 2.113948, a 1.368680% improvement; candidate median is 2.017354
versus 2.085128, a 3.250349% improvement. Mean paired improvement is 1.108930%.
All 32 runs prove with exit zero. The executable grows by 5,632 bytes, but the
native result shows that this footprint does not reproduce Experiment 198's
cache regression.

## Compatibility evidence

- Proof report `.artifacts/e-compare/20260722-003712-747053/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext at the standard
  60-second limit.
- Resource report `.artifacts/e-compare/20260722-003906-218173/` has zero
  mismatches across BOO020 and SWV851.
- The recent clean loaded report
  `.artifacts/e-compare/20260721-234057-582244/` has 50 cases, zero unexpected
  mismatches, and the one declared sledgehammer difference. Experiments 200 and
  201 only add measured code-generation directives; both current
  boundary-sensitive proof and resource subsets are exact.

## Validation

- All four focused always-dereference tests pass.
- The complete serial suite passes: 4,384 library tests plus every integration
  target and feature.
- Strict all-target, all-feature pedantic Clippy passes with the measured
  forced-inline justification.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept forced dispatcher inlining. Deterministic instructions fall 0.766870%,
warmed native mean and median both improve despite the larger binary, and
focused proof/resource compatibility is exact. Keep the main parity issue open
at 2.090530 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-deref-dispatch.out \
  target-wsl-201-force-inline-deref-dispatch/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
