# Rejected three-link dereference window

## Question

Can `DerefType::Always` extend the accepted fixed two-link free-variable borrow
window to three links, moving closer to C's raw binding-pointer loop without
reintroducing the rejected recursive eight-link shape?

## Setup

- Parent source: commit `4086b16e` (`Borrow term metadata and propagate
  admission failures`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 11,993,700,044 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-178-three-link-deref/rust-callgrind-three-link-deref.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

The accepted `deref_always_step` borrows a free variable's binding and, when
that binding is another free variable, borrows the second binding before
cloning the term that leaves the window. The candidate added one more fixed
nested immutable borrow. It retained iterative outer traversal, preserved the
unbound-variable stop, and did not change `DerefType::Once` or applied-variable
handling.

All 35 focused dereference tests passed, including the 20-variable chain, and
the deterministic run produced the exact 4,873-clause proof.

## Result

The candidate retires 12,112,284,087 instructions, 118,584,043 above the
parent (+0.9887%). `deref_always_step` rises from 328,211,680 to 436,856,988
exclusive instructions, an increase of 108,645,308 (+33.10%). Combined
`term_deref_if_changed`, `deref_always_step`, and `deref_step` work rises by
108,430,343 instructions (+11.43%).

The larger window reduces `deref_always_step` calls only from 20,963,152 to
20,939,267: 23,885 calls, or 0.114%. The extra borrow and branch sequence is
therefore paid on millions of short chains while almost never eliminating an
outer iteration. This directly falsifies the hypothesis that the common chain
is long enough to amortize a third link.

Native proof, resource, and full-matrix runs were intentionally skipped after
the deterministic performance gate failed. The source was restored exactly to
`4086b16e`.

## Decision

Reject the three-link window and retain the accepted fixed two-link
implementation. Further dereference work should reduce dispatch or
representation cost for short chains rather than widening the nested borrow
window.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-three-link-deref.out \
  target-wsl-178-three-link-deref/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
