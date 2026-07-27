# Forced inline term-tree splay

## Question

Does forcing the private term-tree splay routine inline remove enough call and
return overhead to improve the whole prover after its comparator is already
inlined?

## Setup

- Parent source: commit `aa37ed9f` (`Force-inline hot term-top ordering`),
  accepted Experiment 196.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-196-force-inline-term-top-order/rust-callgrind-force-inline-term-top-order.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-197-force-inline-term-splay/rust-callgrind-force-inline-term-splay.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`splay_term_tree` now uses `#[inline(always)]`. A narrow Clippy expectation
documents that the normally discouraged attribute is retained because the
pinned whole-prover measurement improves. Comparison semantics, rotations,
child ownership, and root assembly are unchanged; this is only a code-generation
directive.

## Performance result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 11,401,457,666 instructions. This is 52,029,674 below the
11,453,487,340-instruction parent, a 0.454269% whole-prover reduction. The
deterministic C/Rust ratio improves from 2.179806 to 2.169904.

The standalone splay symbol disappears into `TermTree::insert`. The comparable
splay-plus-insert aggregate falls from 706,538,649 to 653,988,753 instructions,
saving 52,549,896 or 7.437653%. The surrounding PD-tree cursor, dereference,
and normalization hotspots reproduce exactly, localizing the improvement.

An alternating eight-pair native Windows LUSK6 run also favors the candidate.
Its median is 1.979282 seconds versus 2.005448 for the parent, and its mean is
1.984274 versus 2.049904 seconds. All 16 runs prove with exit zero.

## Compatibility evidence

- Focused proof report `.artifacts/e-compare/20260721-220852-998198/` has exact
  GEO288, LUSK6, and LUSK6ext rows. HEN011 alone reaches the standard
  60-second Rust cutoff under the same documented host power/thermal state that
  also made the accepted Experiment 190 control miss this boundary.
- Resource report `.artifacts/e-compare/20260721-221202-361356/` has zero
  mismatches across BOO020 and SWV851.
- With diagnostic headroom, HEN report
  `.artifacts/e-compare/20260721-221727-352439/` is exact at a 90-second limit.
- Experiment 196's loaded 50-case report
  `.artifacts/e-compare/20260721-210721-396920/` matches its declarations on
  the other 48 rows. The full standard matrix was not repeated while the known
  host state continued to make accepted controls fail boundary cutoffs; the
  current source changes no proof-search semantics.

## Validation

- All four focused term-tree tests pass.
- The complete serial suite passes: 4,384 library tests plus every integration
  target and feature.
- Strict all-target, all-feature pedantic Clippy passes with the measured
  forced-inline justification.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept forced splay inlining. Deterministic Linux instructions fall 0.454269%,
paired native Windows runs improve by 1.30% at the median and 3.20% at the
mean, resource behavior is exact, and HEN is exact with diagnostic headroom.
Preserve the same controlled host-cutoff exception documented by Experiment
196. Keep the main issue open at 2.169904 times C and repeat the standard
matrix when the host returns to its earlier power state.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-term-splay.out \
  target-wsl-197-force-inline-term-splay/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
