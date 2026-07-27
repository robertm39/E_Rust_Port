# Rejected single-borrow PD-tree symbol expansion

## Question

Can the first-order PD-tree symbol-expansion helper derive arity from its
borrowed argument slice, eliminating the immediately preceding `Term::arity`
borrow without changing traversal or ownership?

## Setup

- Parent source: commit `b22a35d1` (`Move TermTree insertion links without
  cloning`), accepted Experiment 214.
- Candidate: in `advance_first_order_symbol_query`, borrow `term.arguments()`
  first and use `arguments.len()` instead of calling `term.arity()` before the
  existing slice borrow. Child cloning, reverse push order, query-step state,
  and backtracking are unchanged.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-215-single-borrow-pdt-expansion/rust-callgrind-single-borrow-pdt-expansion.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

This isolates one portion of the broader metadata bundle rejected in
Experiment 009. It does not change higher-order query building or head/type
access.

## Result

The candidate reaches the expected LUSK6 proof and retires 10,637,694,553
instructions. This is 5,053,168 above the 10,632,641,385-instruction parent, a
0.047525% regression. The hypothetical C/Rust ratio worsens from 2.023584 to
2.024546.

The regression is localized to the intended cursor.
`search_next_matching_occurrence_impl` rises from 1,697,827,541 to
1,702,881,771 exclusive instructions, adding 5,054,230. `TermTree::insert` and
`Substitution::norm_term` reproduce exactly. Keeping the mapped slice alive
while deriving its length therefore generates slightly more expensive code
than the accepted short `arity()` borrow followed by the argument borrow.

## Validation

- All 41 focused PD-tree tests pass.
- Strict all-feature library pedantic Clippy and formatting pass.
- The release candidate reaches the expected unsatisfiable result and exits
  zero under Callgrind.
- Native and compatibility matrices were skipped after the deterministic gate
  rejected the candidate.

## Decision

Reject and restore the accepted two-borrow helper exactly. A source-level
borrow reduction is not a machine-level reduction here, and the direct cursor
attribution leaves no compensating effect to investigate. Keep the accepted
baseline at 10,632,641,385 instructions, or 2.023584 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-single-borrow-pdt-expansion.out \
  target-wsl-215-single-borrow-pdt-expansion/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
