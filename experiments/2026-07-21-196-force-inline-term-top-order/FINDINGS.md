# Forced inline term-top ordering

## Question

Does forcing the private term-tree comparator inline remove enough of its
roughly six million call boundaries to improve the whole prover, despite the
usual code-size risk?

## Setup

- Parent source: commit `af2b408a` (`Record neutral term comparator inline
  hint`), whose executable source is accepted Experiment 190.
- Diagnostic motivation: Experiment 193 line attribution records 35,558,175
  instructions at comparator entry and 49,781,445 at exit; Experiment 195
  showed that an ordinary `#[inline]` hint is ignored.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-190-direct-always-nonvar/rust-callgrind-direct-nonvar.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-196-force-inline-term-top-order/rust-callgrind-force-inline-term-top-order.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`term_top_order_for_problem` now uses `#[inline(always)]`. A narrow Clippy
expectation documents that this normally discouraged attribute is retained
because the pinned whole-prover measurement improves. Comparison keys,
first-order type preconditions, higher-order type identity, arity, argument
identity, splay order, and tree ownership are unchanged.

## Performance result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 11,453,487,340 instructions. This is 135,013,558 below the
11,588,500,898-instruction parent, a 1.165065% whole-prover reduction. The
deterministic C/Rust ratio improves from 2.205501 to 2.179806.

The standalone 510,401,663-instruction comparator disappears. The comparable
aggregate of comparator, `splay_term_tree`, and `TermTree::insert` falls from
841,164,968 to 706,538,649 instructions, saving 134,626,319 or 16.004746%.
The surrounding PD-tree cursor, dereference, and normalization hotspots
reproduce exactly, making the attribution local rather than a broad layout
accident.

An alternating eight-pair native Windows LUSK6 run also favors the candidate.
Its median is 2.778366 seconds versus 2.818998 for the accepted binary, and its
mean is 2.782401 versus 2.818217 seconds. All 16 runs prove with exit zero.

## Compatibility evidence

- Proof report `.artifacts/e-compare/20260721-210034-493607/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-210244-740174/` has zero
  mismatches across BOO020 and SWV851.
- Loaded full report `.artifacts/e-compare/20260721-210721-396920/` has 50
  cases, 2 unexpected cutoff rows, and the one declared `sledgehammer.p`
  difference. HEN011 reaches the 60-second Rust limit and synthetic one-second
  LUSK6 reaches `ResourceOut`; the other 48 rows match their declarations.
- Isolated HEN report `.artifacts/e-compare/20260721-212308-141656/` reaches the
  same cutoff at 61.03 seconds. The accepted Experiment 190 control also
  reaches the cutoff at 61.71 seconds in
  `.artifacts/e-compare/20260721-212546-437307/`, showing current host
  throttling is not candidate-specific.
- With a diagnostic 90-second window, HEN report
  `.artifacts/e-compare/20260721-214348-460708/` is exact.

The host showed sustained 14--28% background CPU load during the cutoff runs,
including an interactive user process that was deliberately left untouched.
Later probes at about 10% load still missed the one-second boundary, indicating
a sustained host power/thermal state rather than transient harness contention.
The standard 60-second matrix should be rerun when that state no longer makes
the previously accepted binary fail the same boundary.

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

Accept forced inlining. The source change is a code-generation directive with
no semantic branch, deterministic Linux instructions fall 1.165065%, paired
native Windows runs improve about 1.27%, all 48 non-cutoff matrix rows match,
and the focused proof and resource reports are exact. The only failed rows are
the two throughput cutoffs during a host state in which the previously
accepted binary also fails its control. Preserve that exception and the
90-second exact HEN report rather than misclassifying environmental throttling
as a proof-search regression. Keep the main issue open at 2.179806 times C and
repeat the standard matrix when the host returns to its earlier power state.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-term-top-order.out \
  target-wsl-196-force-inline-term-top-order/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
