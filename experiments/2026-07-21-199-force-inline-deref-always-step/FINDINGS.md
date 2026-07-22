# Forced inline always-dereference step

## Question

Does forcing the single-loop `deref_always_step` helper inline remove enough
hot call overhead to improve both deterministic instructions and native wall
time without changing dereference behavior?

## Setup

- Parent source: commit `c197fca5` (`Record rejected evaluation-index splay
  inline`), whose executable source is accepted Experiment 197.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-197-force-inline-term-splay/rust-callgrind-force-inline-term-splay.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-199-force-inline-deref-always-step/rust-callgrind-force-inline-deref-always-step.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`deref_always_step` now uses `#[inline(always)]`. A narrow Clippy expectation
documents that the normally discouraged attribute is retained because pinned
whole-prover and native measurements improve. Free-variable detection, binding
borrows, one-hop free-variable compression, applied-variable dereferencing, and
the surrounding loop are unchanged.

## Performance result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 11,265,328,652 instructions. This is 136,129,014 below the
11,401,457,666-instruction parent, a 1.193961% whole-prover reduction. The
deterministic C/Rust ratio improves from 2.169904 to 2.143996.

The standalone 508,195,300-instruction helper disappears. The comparable
aggregate of always-dereference work, `term_deref_if_changed`, and
`Substitution::norm_term` falls from 1,115,248,621 to 980,338,780 instructions,
saving 134,909,841 or 12.096840%. PD-tree cursor and evaluation-index work
reproduce exactly, making the attribution local.

Thirty-two alternating native Windows pairs were recorded. The candidate's
first invocation is a 3.508-second cold-start outlier; the parent executable
had already been invoked in the preceding experiment. Across the 31 post-cold
pairs, the candidate wins 18, has a 1.945692-second median versus 1.973265 for
the parent, and has a 1.980118-second mean versus 2.003089. These are 1.397334%
and 1.146737% improvements. Mean paired improvement is 1.004683%. A separate
fully warm 16-pair block is split 8--8 and improves paired mean by 0.43%, so it
does not reproduce Experiment 198's wall-time regression. All 64 runs prove
with exit zero. The executable grows by only 512 bytes.

## Compatibility evidence

- Proof report `.artifacts/e-compare/20260721-232049-932982/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext at the standard
  60-second limit.
- Resource report `.artifacts/e-compare/20260721-232238-966464/` has zero
  mismatches across BOO020 and SWV851.
- The first loaded report `.artifacts/e-compare/20260721-232656-724918/` has
  one intermittent BOO020 allocation failure after consecutive 2 GiB reference
  runs; its other 49 rows match, including the declared sledgehammer output
  difference.
- A direct identical-argument control then reaches the expected CPU-limit exit
  for both parent and candidate; candidate wall time is 51.35 seconds versus
  55.03 for the parent.
- After WSL exited naturally and released retained memory, clean loaded report
  `.artifacts/e-compare/20260721-234057-582244/` has 50 cases, zero unexpected
  mismatches, and the one declared sledgehammer difference. HEN011 and the
  synthetic one-second LUSK6 case both match at their standard limits.

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

Accept forced inlining. Deterministic instructions fall 1.193961%, post-cold
native mean and median improve, focused proof/resource reports are exact, and
the clean full matrix has zero unexpected mismatches. The first full report's
BOO020 row is preserved rather than hidden: direct parent/candidate control and
the clean rerun show that it was environmental memory pressure after repeated
2 GiB WSL runs, not a stable candidate regression. Keep the main parity issue
open at 2.143996 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-deref-always-step.out \
  target-wsl-199-force-inline-deref-always-step/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
