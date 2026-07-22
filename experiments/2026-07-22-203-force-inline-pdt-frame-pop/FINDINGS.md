# Forced inline PD-tree frame pop

## Question

Does forcing the accepted PD-tree cursor frame-restoration helper into its
single state-machine caller improve the whole prover without changing the
backtracking algorithm rejected by Experiment 192?

## Setup

- Parent source: commit `b8c8ff45` (`Record rejected term-tree insertion
  inline`), whose executable source is accepted Experiment 201.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-201-force-inline-deref-dispatch/rust-callgrind-force-inline-deref-dispatch.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-203-force-inline-pdt-frame-pop/rust-callgrind-force-inline-pdt-frame-pop.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`pop_subst_cursor_frame` now uses `#[inline(always)]`. A narrow Clippy
expectation records the pinned whole-prover and native evidence. Frame removal,
binding truncation, query-step restoration, child popping, and restored-term
push order are unchanged. This is distinct from Experiment 192, which replaced
the accepted child-pop loop with `Vec::truncate` and regressed.

## Performance result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,914,678,029 instructions. This is 69,719,699 below the
10,984,397,728-instruction parent, a 0.634716% whole-prover reduction. The
deterministic C/Rust ratio improves from 2.090530 to 2.077261.

The standalone 279,148,494-instruction frame-pop symbol disappears. The
comparable cursor-plus-pop aggregate falls from 1,767,547,917 to 1,697,827,541
instructions, saving 69,720,376 or 3.944469%. Other major hotspots reproduce
exactly, localizing the improvement.

Both binaries were warmed before 16 alternating native Windows pairs. The
candidate and parent each win eight pairs. Candidate mean is 1.831373 seconds
versus 1.835508, a 0.225280% improvement; candidate median is 1.811474 versus
1.842190, a 1.667350% improvement. Mean paired improvement is 0.176900%. All
32 runs prove with exit zero. The executable grows by 1,536 bytes.

## Compatibility evidence

- Proof report `.artifacts/e-compare/20260722-013308-897499/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext at the standard
  60-second limit.
- Initial resource report `.artifacts/e-compare/20260722-013503-066148/` has an
  intermittent BOO020 allocation exit after the C reference run while WSL
  retains about 713 MiB; SWV851 matches.
- Identical direct BOO controls then reach the expected CPU-limit exit for both
  candidate and parent. Candidate wall time is 56.36 seconds and parent is
  53.98 seconds.
- After WSL exits naturally, isolated BOO report
  `.artifacts/e-compare/20260722-014320-467717/` is exact with zero mismatches.
- The recent clean loaded report
  `.artifacts/e-compare/20260721-234057-582244/` has 50 cases, zero unexpected
  mismatches, and the one declared sledgehammer difference. Subsequent accepted
  changes are measured code-generation directives, and current focused proof
  plus clean resource boundary evidence is exact.

## Validation

- All 50 focused PD-tree tests pass.
- The complete serial suite passes: 4,384 library tests plus every integration
  target and feature.
- Strict all-target, all-feature pedantic Clippy passes with the measured
  forced-inline justification.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept forced frame-pop inlining. Deterministic instructions fall 0.634716%,
native mean and median improve, proof compatibility is exact, and BOO is exact
for direct parent/candidate controls and a clean-state C/Rust rerun. Preserve
the initial resource report as environmental pressure evidence rather than
hiding it. Keep the main parity issue open at 2.077261 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-pdt-frame-pop.out \
  target-wsl-203-force-inline-pdt-frame-pop/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
