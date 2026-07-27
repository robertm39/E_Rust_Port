# Rejected PD-tree bulk frame restoration

## Question

Can PD-tree backtracking restore expanded query children with one checked
length calculation and `Vec::truncate`, instead of one checked `Vec::pop` per
child?

## Setup

- Parent source: commit `d640c472` (`Record rejected PD-tree mode
  specialization`), whose executable source is accepted Experiment 190.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-190-direct-always-nonvar/rust-callgrind-direct-nonvar.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-192-pdt-bulk-pop/rust-callgrind-pdt-bulk-pop.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

`pop_subst_cursor_frame` replaced its loop of `query_stack.pop()` operations
with a checked subtraction followed by `query_stack.truncate(restored_len)`.
Bindings, query-step ownership, the restored term push, traversal order, and
frame layout were unchanged.

## Result

The candidate passed all 41 focused PD-tree tests and preserved the exact
4,873-processed-clause LUSK6 proof. It retired 11,612,239,649 instructions,
23,738,751 above the 11,588,500,898-instruction parent. That is a 0.204847%
whole-prover regression and raises the deterministic C/Rust ratio from
2.205501 to 2.210019.

The intended hotspot regressed directly. `pop_subst_cursor_frame` rises from
279,148,494 to 303,408,905 exclusive instructions, adding 24,260,411 or
8.690862%. The surrounding cursor reproduces exactly at 1,488,399,423
instructions. For these reference-counted `Term` elements, `truncate` does not
avoid the required drops and generates a more expensive restoration loop than
the explicit repeated pop.

## Decision

Reject bulk truncation and restore `src/clauses/pdtrees.rs` exactly to the
accepted implementation. Native compatibility matrices and full repository
gates were skipped because the deterministic benchmark rejected the candidate
after focused correctness coverage passed. Keep the accepted baseline at
11,588,500,898 instructions and 2.205501 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pdt-bulk-pop.out \
  target-wsl-192-pdt-bulk-pop/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
