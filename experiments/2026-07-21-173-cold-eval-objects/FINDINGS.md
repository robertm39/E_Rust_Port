# Rejected cold evaluation-object arena

## Question

Can the evaluation splay index keep only C's priority/heuristic/age key in its
hot node and move Rust's cloned-cell identity tie-break to a parallel cold
arena, reducing each node from 48 to 40 bytes?

## Setup

- Parent source: commit `72bc5e3b` (`Record rejected PD-tree query
  truncation`), whose executable source is accepted Experiment 170 commit
  `b4a8eed6`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,525,374,625 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-173-cold-eval-objects/rust-callgrind-cold-eval-objects.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

The candidate split `EvalIndexEntry` into a 24-byte comparison key stored in
each splay node and a parallel `EvalObjectHandle` vector indexed by the same
arena slot. Priority, heuristic, and evaluation-age comparisons stayed in the
hot node; only an equal C key loaded the object handle for Rust's required
cloned-cell distinction. The live splay node fell from 48 to 40 bytes.

Insertion, freed-slot reuse, clearing, leftmost lookup, and in-order iteration
maintained both vectors together. The non-Linux admission boundary fallibly
reserved both vectors before mutation. Five focused evaluation-index tests
passed, covering exact ordering, duplicate suppression, removal/reuse,
cloned-cell identity, 40-byte layout, and Windows pre-reservation. The
deterministic proof remained exact.

## Result

The candidate retires 12,565,960,720 instructions, 40,586,095 above the parent
(+0.3240%). The intended hotspot regresses: `EvalIndexTree::splay` rises from
368,087,119 to 378,459,393 exclusive instructions, an increase of 10,372,274
(+2.82%). Reconstructing entries and separately bounds-checking the cold arena
cost more than the smaller stride saves.

The second vector also worsens allocation and movement costs. `_int_malloc`
rises by about 3.31 million instructions and `memcpy` by about 2.52 million,
with related allocator shifts accounting for more of the whole-program loss.
Native compatibility/resource matrices were intentionally skipped after the
deterministic performance gate failed. The source was restored exactly to
accepted Experiment 170 behavior.

## Decision

Reject the parallel object arena. The existing 48-byte node is the better safe
layout because its complete key is contiguous and it requires only one arena
growth. Further evaluation-index work must reduce comparisons or safe arena
accesses without adding a synchronized allocation.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-cold-eval-objects.out \
  target-wsl-173-cold-eval-objects/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
