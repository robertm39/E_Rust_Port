# Rejected owned substitution-normalization dereference

## Question

Can `Substitution::norm_term` consume each owned traversal-stack term through
the existing owned dereference helper, avoiding one reference-count clone on
the unchanged path while preserving C's pointer-shaped traversal?

## Setup

- Parent source: commit `62afa4a7` (`Use IntMap for PD-tree function edges`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,625,510,206 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-167-owned-subst-normalization/rust-callgrind-owned-norm.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

`norm_term` already pops an owned `Term` from its reusable traversal vector.
The candidate passed that value to `term_deref_owned`, which returns the same
handle when dereferencing makes no change, instead of borrowing it through
`term_deref`, cloning it, and then dropping the original. Traversal order,
fresh-variable allocation, binding order, scratch reuse, and dereference mode
were otherwise unchanged. The existing nine substitution tests passed.

## Result

The candidate preserves the exact 4,873-clause proof but retires
12,659,122,408 instructions, 33,612,202 above the parent (+0.2662%). The local
normalizer improves from 342,923,509 to 332,805,579 exclusive instructions,
saving 10,117,930 (-2.950%). That local ownership gain does not survive whole-
program optimization: the changed inlining layout moves `PdNode::child_index`
out of the PD-tree cursor. Cursor plus child lookup rises from 1,650,291,596 to
1,691,421,889 instructions (+41,130,293), while
`term_top_compare_for_problem` rises another 14,228,910 instructions. Smaller
unrelated decreases do not offset those shifts.

This is an end-to-end performance rejection, not a semantic failure. Because
the deterministic gate failed before any platform-specific change, native
proof/resource and full-matrix runs were intentionally skipped.

## Decision

Reject owned unchanged-term dereferencing in `Substitution::norm_term` and
restore the source exactly to `62afa4a7`. Keep the owned helper only in the
first-order MGU queue path accepted by Experiment 148, where its localized
whole-program result was positive. Future normalization work must beat the
accepted end-to-end profile rather than relying on the normalizer's local
exclusive reduction.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-owned-norm.out \
  target-wsl-167-owned-norm/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
