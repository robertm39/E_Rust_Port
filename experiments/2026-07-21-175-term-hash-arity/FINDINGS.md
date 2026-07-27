# Rejected combined term hash and arity

## Question

Can term-cell-store insertion, extraction, and deletion reuse the argument
length already observed during hashing instead of borrowing `TermArgs` again
for accounting?

## Setup

- Parent source: commit `b76dcae0` (`Borrow term arguments while hashing
  cells`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,407,202,652 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-175-term-hash-arity/rust-callgrind-hash-arity.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

The candidate factored the accepted borrowed hash into a private helper that
returned both bucket and arity. Insert, extract, and delete consumed that arity
for `arg_count`, while the public hash wrapper returned only the bucket. Hash
bits, accounting, tree operations, and panic behavior were unchanged. All six
focused term-cell-store tests passed and the deterministic proof remained
exact.

## Result

The candidate retires 12,409,617,540 instructions, 2,414,888 above the parent
(+0.0195%). The intended insertion boundary itself rises from 107,487,162 to
110,923,956 exclusive instructions, an increase of 3,436,794 (+3.20%). Returning
and unpacking the tuple inhibits the compact code generation established by
Experiment 174; saving one later arity borrow does not repay that cost.

Native compatibility/resource matrices were intentionally skipped after the
deterministic performance gate failed. The source was restored exactly to
`b76dcae0`.

## Decision

Reject the combined helper. Retain Experiment 174's single-purpose borrowed
hash and the separate accounting arity read; in optimized Rust this produces a
cheaper insertion boundary than sharing the observation through a tuple.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-hash-arity.out \
  target-wsl-175-term-hash-arity/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
