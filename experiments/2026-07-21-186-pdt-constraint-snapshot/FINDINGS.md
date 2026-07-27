# Rejected PD-tree constraint-setting snapshot

## Question

Can the first-order PD-tree cursor load the two global size/age constraint
flags once per `search_next_matching_occurrence_impl` call instead of through
`node_satisfies_constraints` at each visited node?

## Setup

- Parent source: commit `a93785f7` (`Specialize always-mode term
  dereferencing`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-185-specialized-always-deref/rust-callgrind-specialized-always.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-186-pdt-constraint-snapshot/rust-callgrind-constraint-snapshot.out`.

## Candidate

The candidate factored constraint checks into an inline helper taking a copied
`PdtConstraintSettings` value. The first-order cursor loaded the two relaxed
atomic flags once on entry and passed the snapshot to the root, symbol-edge,
and variable-edge checks. The existing public/general helper continued to
load current settings for all other callers.

This preserved the run-level configuration contract and all matching,
constraint, traversal, and substitution behavior. All 41 focused PD-tree
tests passed, and deterministic LUSK6 produced the exact 4,873-clause proof.

## Result

The candidate retires 11,747,991,449 instructions, 11,522,856 above the
11,736,468,593-instruction parent, a 0.0982% whole-prover regression. The
entire material change is in
`PdTree::search_next_matching_occurrence_impl`, which rises from
1,484,913,131 to 1,496,435,383 exclusive instructions: 11,522,252 or 0.7760%.
All other dominant compact-profile entries reproduce exactly.

The relaxed atomic loads are therefore cheaper in the generated hot loop than
carrying the copied setting value through the larger helper call shape. The
candidate was restored exactly before any native compatibility matrix was
run.

## Decision

Reject the per-call constraint snapshot. Keep Experiment 185 source unchanged
at 11,736,468,593 instructions and a 2.2337 C/Rust ratio. Future PD-tree work
should target traversal state or variable matching rather than these already
cheap flag loads.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-constraint-snapshot.out \
  target-wsl-186-pdt-constraint-snapshot/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
