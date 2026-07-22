# Forced inline always-dereference loop

## Question

After Experiment 199 inlines the inner always-dereference step, does forcing
its single-caller loop inline remove the remaining boundary without incurring a
native code-size penalty?

## Setup

- Parent source: commit `1484d04b` (`Force-inline hot always-dereference
  step`), accepted Experiment 199.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-199-force-inline-deref-always-step/rust-callgrind-force-inline-deref-always-step.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-200-force-inline-deref-always-loop/rust-callgrind-force-inline-deref-always-loop.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`term_deref_always_if_changed` now uses `#[inline(always)]`. A narrow Clippy
expectation records the pinned whole-prover and native evidence. The function
has one call site in `term_deref_if_changed`, so this creates one copy. Loop
termination, free-variable binding traversal, applied-variable handling, and
the `DerefType` dispatch are unchanged.

## Performance result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 11,069,284,760 instructions. This is 196,043,892 below the
11,265,328,652-instruction parent, a 1.740241% whole-prover reduction. The
deterministic C/Rust ratio improves from 2.143996 to 2.106685.

The standalone 466,167,506-instruction loop disappears. The comparable
aggregate of always-dereference work, `term_deref_if_changed`, and
`Substitution::norm_term` falls from 980,338,780 to 784,287,784 instructions,
saving 196,050,996 or 19.998290%. PD-tree cursor, term-tree insertion, and
evaluation-index work reproduce exactly, making the attribution local.

Both binaries were warmed before 16 alternating native Windows pairs. The
candidate mean is 1.861093 seconds versus 1.867908 for the parent, a 0.364833%
improvement. Its median is 1.851898 versus 1.854500, a 0.140294% improvement.
The noisy pair split is 6 candidate wins to 10 parent wins, but the candidate's
larger wins improve both central measures; all 32 runs prove with exit zero.

## Compatibility evidence

- Proof report `.artifacts/e-compare/20260722-001008-676665/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext at the standard
  60-second limit.
- Resource report `.artifacts/e-compare/20260722-001203-622553/` has zero
  mismatches across BOO020 and SWV851.
- The immediate parent clean loaded report
  `.artifacts/e-compare/20260721-234057-582244/` has 50 cases, zero unexpected
  mismatches, and the one declared sledgehammer difference. The current change
  is a single-caller code-generation directive with no semantic branch; the
  standard boundary-sensitive proof and resource subsets are exact.

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

Accept forced inlining. Deterministic instructions fall 1.740241%, both native
mean and median improve after explicit warmup, and focused proof/resource
compatibility is exact at standard limits. Keep the main parity issue open at
2.106685 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-deref-always-loop.out \
  target-wsl-200-force-inline-deref-always-loop/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
