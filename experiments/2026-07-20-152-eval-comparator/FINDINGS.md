# Rejected evaluation-comparator rewrites

## Question

Can the compact evaluation-index splay path become cheaper by flattening its
ordering branches or by consuming a C-shaped signed three-way result directly,
without changing priority, heuristic, age, or distinct-object ordering?

## Setup

- Parent source: commit `7d0127c0` (`Replace evaluation B-tree with compact
  splay index`).
- Deterministic parent: 14,023,295,072 instructions with the exact `LUSK6.lop`
  proof and 4,873 processed clauses.
- Workload: upstream `LUSK6.lop` under WSL Callgrind with `--auto --silent
  --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.
- Target hotspot: `EvalIndexTree::splay`, 368,087,119 exclusive instructions
  in the parent.

Profiles are retained at
`.artifacts/experiments/2026-07-20-152-flat-eval-comparator/`.

## Flat `Ordering` branches

The first candidate replaced the nested priority/count match and final
`then_with` object tie-break with early returns. It preserved the exact proof
and retired 14,020,840,584 instructions, a nominal reduction of 2,454,488
(-0.01750%).

Attribution did not support accepting that number. The target splay function
remained exactly 368,087,119 instructions; the small total shift appeared in
unrelated allocator and term-tree functions because the source layout changed.
The result is compiler-layout noise rather than a measured comparator
improvement. The profile is retained as `rust-callgrind-flat-eval-cmp.out`.

## Signed three-way comparator

The second candidate factored priority, count, heuristic, and object comparison
into one overflow-safe signed result. Splay consumed the sign directly, while
the public `Ord` implementation converted it to `Ordering`. This is closer to
C's integer comparator interface but substantially worse for LLVM's generated
Rust control flow.

It preserved the exact proof but retired 14,095,000,532 instructions,
71,705,460 above the parent (+0.51133%). The profile is retained as
`rust-callgrind-signed-eval-cmp.out`.

## Falsification checks

- The direct evaluation-index regression passed for both candidates, covering
  sorted order, duplicate suppression, removal, slot reuse, and distinct
  object tie-breaking.
- Both profiles produced the exact proof, ruling out clause-order differences.
- Function-level attribution rejected the superficially lower flat total
  because the intended hotspot did not move at all.
- The signed candidate was rejected on a large deterministic regression before
  spending time on native proof or resource matrices.
- All source changes were reverted; the worktree returned exactly to the
  pushed parent before this findings note was added.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-flat-eval-cmp.out \
  target-wsl-152-flat-eval-cmp/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-signed-eval-cmp.out \
  target-wsl-152-signed-eval-cmp/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

## Decision

Reject both comparator rewrites and retain the nested `Ordering` implementation
from Experiment 151. Further evaluation-index work should change comparison
frequency or data access, not merely restate the same ordering branches. The
main parity issue remains open for the synthetic one-second LUSK cutoff and
the remaining overall performance gap.
