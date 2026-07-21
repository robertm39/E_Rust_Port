# Rejected compact PD-tree frame positions

## Question

Can the remaining PD-tree traversal frame pack its binding and terminal vector
positions into `u32`, reducing the 64-bit frame from 40 to 32 bytes without
making the cursor more expensive?

## Setup

- Parent source: commit `1704afa3` (`Specialize first-order PD-tree
  expansion`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-180-pdt-first-order-expansion/rust-callgrind-fo-expansion.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-181-compact-pdt-frame-positions/rust-callgrind-compact-positions.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

The accepted cursor frame retained full-width `binding_pos` and
`terminal_position` fields even though both are transient `Vec` lengths. The
candidate stored both as checked `u32` values and ordered them beside the
existing packed variable-child link and one-byte traversal step. Node indices
and effective term weights remained full-width. A 64-bit layout regression
confirmed that the complete frame fell from 40 to 32 bytes.

All 41 focused PD-tree tests passed, including branch order, live
substitutions, frame restoration, and the compact layout assertion. The
deterministic LUSK6 run produced the exact proof.

## Result

The candidate retires 11,949,202,024 instructions, 113,121,306 above the
11,836,080,718-instruction parent, a 0.9557% whole-prover regression. The cost
is localized to `search_next_matching_occurrence_impl`, which rises from
1,484,913,131 to 1,599,663,178 exclusive instructions: 114,750,047 or 7.7277%.
`pop_subst_cursor_frame` remains exactly 279,148,494 instructions, so smaller
frame movement does not offset packed-position handling in the main cursor.

Native proof/resource and full-matrix runs were intentionally skipped after
the deterministic performance gate failed. The source and 40-byte layout test
were restored exactly to commit `1704afa3` behavior, and all 41 focused tests
pass after restoration.

## Decision

Reject packed binding and terminal positions. Keep the accepted 40-byte frame
with full-width positions; its direct indexing is materially cheaper on this
hot state machine than the candidate's denser representation. Retain the raw
profile and this result to prevent another frame-size-only revisit unless the
cursor organization changes substantially.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-compact-positions.out \
  target-wsl-181-compact-pdt-frame-positions/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
