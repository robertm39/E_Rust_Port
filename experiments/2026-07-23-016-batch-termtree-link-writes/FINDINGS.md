# Experiment 254: Batch paired term-tree link writes

## Status

Rejected in Experiment 254 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

The accepted profile assigns 659,343,823 exclusive instructions to
`TermTree::insert`, versus about 333.6 million for C `TermTreeInsert` plus
`splay_term_tree`. After each distinct-key splay, Rust writes the new root's
left and right child links through two separate `TermLinks` mutable borrows;
C assigns both fields directly.

Add one crate-private safe helper that writes both intrusive tree links under a
single `RefCell` borrow. Use it only for the two paired insertion branches and
the existing paired clear. Preserve the individual link APIs, ownership
transfers, comparator, splay algorithm, pointer-derived key, and topology.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851

## Candidate

Add `TermCell::set_tree_links(left, right)`, which updates both intrusive
`TermLinks` fields under one mutable borrow. Use it after each distinct-key
root insertion and from `clear_tree_links`; retain `set_left_son` and
`set_right_son` for all one-sided splay rotations and other callers.

The candidate changes neither link ownership nor topology. The existing
link-boundary test exercises the paired helper, while all public behavior and
the representation remain unchanged.

## Validation

- All four focused term-tree tests pass.
- The focused link-boundary test passes.
- Strict library pedantic Clippy passes.
- Formatting and `git diff --check` pass.
- The exact LUSK6 profile proves `Unsatisfiable` and exits zero.
- Every warmup and measured native process proves `Unsatisfiable` and exits
  zero.

## Deterministic measurement

The candidate retires 9,897,089,273 instructions, 1,345,493 below the
9,898,434,766-instruction parent. This is a 0.013593% whole-prover improvement,
and the hypothetical Rust/C ratio changes from 1.883851 to 1.883595.

Because the paired helper is outlined in the optimized build, the direct
`TermTree::insert` owner alone rises while the aggregate intended boundary
improves:

| Exclusive owner | Parent | Candidate |
| --- | ---: | ---: |
| `TermTree::insert` | 659,343,823 | 662,509,552 |
| `set_left_son` | 41,885,260 | 31,921,320 |
| `set_right_son` | 41,699,840 | 31,735,660 |
| `set_tree_links` | 0 | 16,255,191 |
| `clear_tree_links` | 753,848 | included in paired helper |
| Aggregate | 743,682,771 | 742,421,723 |

The aggregate intended boundary improves by 1,261,048 instructions. That
accounts for nearly the complete whole-program reduction, but its magnitude is
too small to accept without the native production gate.

## Native production measurement

After four alternating warmup pairs, one independent block ran 64 alternating
pairs with a fresh process for each parent and candidate execution. All 128
measured processes prove the theorem and exit zero.

Across all 64 measured pairs, the candidate regresses mean wall time by
0.657143% and mean CPU time by 0.505370%. Its medians regress by 0.574975% and
1.020408%, while paired aggregate wall and CPU time regress by 0.721807% and
0.563490%. The candidate wins only 24 wall pairs and 23 CPU pairs, with 10 CPU
ties.

The stable last 32 pairs still regress:

- mean wall time: +0.401146%;
- mean CPU time: +0.318167%;
- median wall time: +0.260312%;
- median CPU time: tie;
- paired aggregate wall time: +0.478058%;
- paired aggregate CPU time: +0.365506%;
- wins: 15 wall and 13 CPU, with 7 CPU ties.

The stable native reversal is much larger than the 0.013593% deterministic
gain. A second block is not proportionate because this first block already
decisively fails the production gate.

## Result

Reject. Restore the two individual link writes and remove the candidate helper
and focused candidate use. Accepted Experiment 245 remains the baseline at
9,898,434,766 instructions, or 1.883851 times C. Compatibility and resource
matrices are skipped after native production rejects the candidate.

The measured native samples are in `native-lusk.csv`. Raw ignored artifacts
are preserved at:

```text
.artifacts/experiments/2026-07-23-016-batch-termtree-link-writes/rust-callgrind-batch-termtree-link-writes.out
.artifacts/experiments/2026-07-23-016-batch-termtree-link-writes/native-warmup.csv
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-batch-termtree-link-writes.out \
  target-wsl-254-batch-termtree-link-writes/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
