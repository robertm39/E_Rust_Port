# Rejected term-top comparator inline hint

## Question

Can an explicit Rust inline hint remove dispatch and expose more first-order
specialization around the hot term-tree comparator without changing its key or
the splay-tree algorithm?

## Setup

- Parent source: commit `b4a8eed6` (`Compact PD-tree traversal frames`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,525,374,625 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-171-inline-term-compare/rust-callgrind-inline-compare.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

The candidate added only `#[inline]` to
`term_top_compare_for_problem`. Function-code, problem-mode, type, arity, and
argument-identity ordering were unchanged. All four focused term-tree tests
passed and the deterministic proof remained exact.

## Result

The candidate retires 12,526,334,535 instructions, 959,910 above the parent
(+0.0077%). Its out-of-line comparator remains in the profile and falls by
only 33,363 instructions, from 529,064,108 to 529,030,745. The adjacent
`splay_term_tree` rises by 72,999 instructions and smaller compiler-layout
shifts account for the remaining regression; dominant unrelated functions are
unchanged.

This hint therefore does not create useful first-order specialization or
remove the comparator boundary. Native compatibility/resource matrices were
intentionally skipped after the deterministic performance gate failed. The
source was restored exactly to `b4a8eed6`.

## Decision

Reject the inline hint. Comparator work should be reduced by changing the
safe representation or the actual comparison operations, not by asking LLVM
to duplicate the existing function. Preserve the accepted 12,525,374,625
instruction parent and its 2.3838 C/Rust ratio.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-inline-compare.out \
  target-wsl-171-inline-term-compare/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
