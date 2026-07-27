# Rejected forced inline evaluation-index splay

## Question

Does forcing `EvalIndexTree::splay` inline improve the whole prover after the
two accepted term-tree inline changes?

## Setup

- Parent source: commit `81f5135e` (`Force-inline hot term-tree splay`),
  accepted Experiment 197.
- Candidate: add only `#[inline(always)]` and its narrow Clippy expectation to
  the private evaluation-index splay routine.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-197-force-inline-term-splay/rust-callgrind-force-inline-term-splay.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-198-force-inline-eval-splay/rust-callgrind-force-inline-eval-splay.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 11,361,031,849 instructions. This is 40,425,817 below the
11,401,457,666-instruction parent, a 0.354567% reduction. The deterministic
C/Rust ratio would improve from 2.169904 to 2.162210.

The standalone 306,825,308-instruction splay symbol disappears. The comparable
aggregate of splay, `index_clause_evaluations`, and `ClauseSet::extract_at_slot`
falls from 458,795,343 to 418,372,403 instructions, saving 40,422,940 or
8.810669%. Unrelated major hotspots reproduce exactly, so the instruction
change is local.

## Native result

The native Windows result disagrees with the instruction count. Across 16
alternating pairs, the parent mean is 1.865747 seconds and median is 1.864637;
the candidate mean is 1.893263 and median is 1.900025. The candidate therefore
regresses by 1.474769% at the mean and 1.897844% at the median. All 32 runs
prove with exit zero. The candidate executable is also 2,560 bytes larger
(8,642,560 versus 8,640,000 bytes), consistent with a code-size or instruction-
cache cost that Callgrind instruction counts do not model.

The first eight pairs alone had favored the candidate median, so the sample was
extended rather than allowing one noisy run to decide the experiment. The full
16-pair result reverses both central measures and is the retained comparison.

## Compatibility evidence

- Focused proof report `.artifacts/e-compare/20260721-224019-455667/` has exact
  GEO288, LUSK6, and LUSK6ext rows. HEN011 alone reaches the known host-throttled
  60-second Rust cutoff.
- Resource report `.artifacts/e-compare/20260721-224411-041503/` has zero
  mismatches across BOO020 and SWV851. Its report completed before the outer
  wrapper reached its 240-second deadline.
- HEN report `.artifacts/e-compare/20260721-224947-908125/` is exact with a
  90-second limit.
- The focused evaluation-index tree test and formatting pass.

## Decision

Reject and restore the parent source. The deterministic instruction saving is
real and well localized, but the maintained goal is production performance,
not instruction count alone. A repeatable 1.47--1.90% native wall-time
regression and larger executable outweigh the Callgrind improvement. Preserve
this result to avoid retrying the same code-size tradeoff.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-eval-splay.out \
  target-wsl-198-force-inline-eval-splay/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
