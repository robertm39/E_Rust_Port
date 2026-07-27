# Rejected PD-tree query-stack truncation

## Question

Can PD-tree backtracking restore the pending-query prefix with one safe
`Vec::truncate` call instead of one checked `Vec::pop` per expanded child?

## Setup

- Parent source: commit `36140599` (`Record rejected term comparator inline
  hint`), whose executable source is accepted Experiment 170 commit
  `b4a8eed6`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,525,374,625 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-172-pdt-query-truncate/rust-callgrind-query-truncate.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

`pop_subst_cursor_frame` currently pops each expanded child with an explicit
checked loop, drops those temporary `Term` handles, and pushes the retained
parent term. The candidate computed the same retained prefix with
`checked_sub`, called `truncate` once, and restored the same parent. Frame,
binding, query-step, child order, and ownership semantics were unchanged. All
40 focused PD-tree tests passed and the deterministic proof remained exact.

## Result

The candidate retires 12,550,598,055 instructions, 25,223,430 above the parent
(+0.2014%). The regression is localized: `pop_subst_cursor_frame` rises from
279,148,494 to 303,408,905 exclusive instructions, an increase of 24,260,411
(+8.69%). The generic truncation/drop path is more expensive for the common
zero-, unary-, and binary-expansion cases than the short explicit pop loop.

Native compatibility/resource matrices were intentionally skipped after the
deterministic performance gate failed. The source was restored exactly to
`b4a8eed6` behavior.

## Decision

Reject query-stack truncation and retain the checked per-child pop loop. Future
backtracking work must reduce `Term` handle ownership or avoid restoration
entirely; substituting a generalized vector operation for this small bounded
loop is not an optimization.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-query-truncate.out \
  target-wsl-172-pdt-query-truncate/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
