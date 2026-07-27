# Rejected direct term-identity ordering

## Question

After the term-tree comparator was changed to return `Ordering`, can its
argument tie-break compare raw term identity integers directly instead of
calling the existing signed `term_identity_cmp` helper and converting that
result to `Ordering`?

## Setup

- Parent source: commit `a6b52d7e` (`Return term-tree ordering directly`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-182-term-tree-ordering/rust-callgrind-ordering.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-183-direct-term-identity-order/rust-callgrind-direct-identity.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

The accepted comparator obtains a signed `-1/0/1` result from
`term_identity_cmp` and converts it to `Ordering`. The candidate instead read
both stable allocation identity integers through `term_identity_id` and used
their direct `usize::cmp` result. Key order, pointer identity, public APIs,
higher-order type ordering, and the splay algorithm remained unchanged.

All four focused term-tree tests passed, and the deterministic LUSK6 run
produced the exact proof.

## Result

The candidate retires 11,805,060,446 instructions, 6,669,195 above the
11,798,391,251-instruction parent, a 0.0565% whole-prover regression. The hot
private comparator rises from 512,581,477 to 519,615,632 exclusive
instructions: 7,034,155 or 1.3723%. `splay_term_tree` rises only 22,453
instructions and `TermTree::insert` rises 3,563, localizing the rejected code-
generation shape to identity comparison itself.

Native proof/resource and full-matrix runs were intentionally skipped after
the deterministic gate failed. The source was restored exactly to commit
`a6b52d7e`, and the four focused tests pass after restoration.

## Decision

Reject direct identity-integer ordering and retain
`term_identity_cmp(...).cmp(&0)`. The existing signed helper produces faster optimized code in this
hot comparator despite the apparent extra conversion. Revisit only if the
term identity representation or complete comparator changes.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-identity.out \
  target-wsl-183-direct-term-identity-order/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
