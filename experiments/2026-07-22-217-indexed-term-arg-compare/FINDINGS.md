# Rejected indexed term-argument comparison

## Question

Can the term-tree comparator replace its equal-length zipped argument walk
with the upstream C implementation's explicit index loop, eliminating the
52.6-million-instruction `zip` category exposed by Experiment 216?

## Setup

- Parent source: commit `c9cc681c` (`Record TermTree line attribution`), whose
  executable source is accepted Experiment 214.
- Candidate: after proving equal argument lengths, walk both borrowed slices
  with one `while` index. Function-code, type, arity, argument identity, early
  return order, splaying, and tree topology are unchanged.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-217-indexed-term-arg-compare/rust-callgrind-indexed-term-arg-compare.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Result

The candidate reaches the expected LUSK6 proof but retires 10,786,406,738
instructions. This is 153,765,353 above the 10,632,641,385-instruction parent,
a 1.446163% regression. The hypothetical C/Rust ratio worsens from 2.023584 to
2.052848.

The intended comparator does not improve: `TermTree::insert` rises from
658,651,917 to 659,099,457 exclusive instructions, adding 447,540. The larger
failure is a whole-binary code-generation effect. Parent
`Substitution::norm_term` includes the forced-inline always-dereference path
and costs 437,245,456 instructions. In the candidate, `norm_term` falls to
303,784,507 but a standalone `term_deref_always` symbol reappears at
276,328,019, making the comparable aggregate 580,112,526. That 142,867,070
increase explains 92.91% of the whole-program regression. The PD-tree cursor
also rises by 11,512,774 instructions.

Thus the source-line `zip` category was attribution, not removable overhead:
the iterator optimized more cheaply than the explicit indexed loop locally,
and the larger forced-inline comparator body perturbed another critical
inlining decision globally.

## Validation

- All four focused term-tree tests pass.
- Strict all-feature library pedantic Clippy and formatting pass.
- The release candidate reaches the expected unsatisfiable result and exits
  zero under Callgrind.
- Native and compatibility matrices were skipped after the deterministic gate
  rejected the candidate.

## Decision

Reject and restore the accepted zipped argument comparator exactly. Do not
replace this adapter merely because line attribution charges work to it; both
its local code and its whole-binary inlining interaction are better in the
accepted shape. Keep the baseline at 10,632,641,385 instructions, or 2.023584
times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-indexed-term-arg-compare.out \
  target-wsl-217-indexed-term-arg-compare/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
