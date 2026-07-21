# Rejected PD-tree problem-mode specialization

## Question

Can the substitution cursor be const-specialized for first-order and
higher-order searches so the hot first-order symbol path does not repeatedly
branch on the captured problem mode?

## Setup

- Parent source: commit `3c4c8bba` (`Specialize non-variable always
  dereferencing`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-190-direct-always-nonvar/rust-callgrind-direct-nonvar.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-191-pdt-mode-specialization/rust-callgrind-pdt-mode-specialization.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

The candidate made `search_next_matching_occurrence_impl` const-generic over
the first-order mode. The ordinary cursor selected the first-order instance
after its existing HO diversion, while the substitution API dispatched once
per call so lambda-lifting searches retained the higher-order instance. Only
the two symbol-path mode conditionals changed; traversal order, frame layout,
bindings, constraints, and rollback were untouched.

## Result

The candidate passed all 41 focused PD-tree tests and preserved the exact
4,873-processed-clause LUSK6 proof. It nevertheless retired 11,737,912,191
instructions, 149,411,293 above the 11,588,500,898-instruction parent. That is
a 1.289306% whole-prover regression and raises the deterministic C/Rust ratio
from 2.205501 to 2.233937.

The intended local effect was favorable:
`search_next_matching_occurrence_impl` fell from 1,488,399,423 to
1,401,801,730 exclusive instructions, saving 86,597,693 or 5.818176%.
`pop_subst_cursor_frame` reproduced exactly at 279,148,494 instructions.

The extra monomorph changed code generation outside the cursor. A comparable
visible aggregate of `norm_term`, `deref_always_step`, and the visible
changed-only dereference symbols rises from 1,115,248,621 to 1,216,451,717
instructions, adding 101,203,096. Other layout changes account for the
remainder. The local branch saving therefore does not justify the pinned
whole-binary regression.

## Decision

Reject the const specialization and restore `src/clauses/pdtrees.rs` exactly
to `3c4c8bba`. Native compatibility matrices and full repository gates were
skipped because the deterministic acceptance benchmark rejected the candidate
after focused correctness coverage passed. Keep the accepted baseline at
11,588,500,898 instructions and 2.205501 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pdt-mode-specialization.out \
  target-wsl-191-pdt-mode-specialization/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
