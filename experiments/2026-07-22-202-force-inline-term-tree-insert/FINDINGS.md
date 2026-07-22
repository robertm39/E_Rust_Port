# Rejected forced inline term-tree insertion

## Question

Does forcing `TermTree::insert` into its single production caller improve the
whole prover after its comparator and splay routine are already inlined?

## Setup

- Parent source: commit `58bea608` (`Force-inline hot dereference dispatcher`),
  accepted Experiment 201.
- Candidate: add only `#[inline(always)]` and its narrow Clippy expectation to
  `TermTree::insert`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-201-force-inline-deref-dispatch/rust-callgrind-force-inline-deref-dispatch.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-202-force-inline-term-tree-insert/rust-callgrind-force-inline-term-tree-insert.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,946,368,419 instructions. This is 38,029,309 below the
10,984,397,728-instruction parent, a 0.346212% reduction. The hypothetical
C/Rust ratio is 2.083292.

The standalone 670,177,484-instruction insertion symbol disappears. The
comparable `TermTree::insert` plus `TermCellStore::insert` aggregate falls from
798,168,224 to 758,951,358 instructions, saving 39,216,866 or 4.913358%.
Unrelated major hotspots are otherwise stable, so the instruction saving is
well localized.

## Native result

Both binaries were warmed before 16 alternating native Windows pairs. The
candidate mean is 1.881704 seconds versus 1.847089 for the parent, a 1.874037%
regression. Candidate median is 1.883275 versus 1.840923, a 2.300607%
regression. The parent wins 9 pairs and the candidate wins 7; mean paired
regression is 1.958792%. All 32 runs prove with exit zero. The candidate binary
is 512 bytes larger.

## Validation

- Eight focused term-tree library tests pass.
- Formatting passes.
- The pinned proof is exact.
- The first broad focused Cargo command hit Windows paging error 1455 while
  mmaping unrelated binary-test metadata after repeated 2 GiB resource runs;
  the intended library-only tests then pass. This environmental compiler error
  is unrelated to the measured runtime decision.

## Decision

Reject and restore the parent source. The deterministic instruction saving is
real, local, and larger than the 512-byte footprint suggests, but production
wall time regresses by 1.87--2.30%. As in Experiment 198, instruction count
alone does not model the relevant native cache/layout effect. Preserve the
result to avoid retrying this inline boundary.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-force-inline-term-tree-insert.out \
  target-wsl-202-force-inline-term-tree-insert/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
