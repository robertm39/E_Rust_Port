# Experiment 266: Reject an allocator empty-list hint

## Status

Rejected in Experiment 266 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

The accepted exact-size allocator takes the process-wide free-list lock before
discovering that a selected size class is empty. A relaxed atomic head load can
treat an observed empty list as a cache miss and avoid the compare-exchange
lock acquisition plus release store. A nonempty observation still takes the
accepted global lock and rechecks the head before reading the intrusive link,
so concurrency and ABA safety remain unchanged.

The accepted profile indicates roughly three cache hits for each System miss.
The candidate therefore trades one relaxed load on every cacheable allocation
for fewer lock operations on the miss fraction.

## Baseline

Accepted Experiment 261:

- Rust instructions: 9,106,424,013
- C instructions: 5,254,361,329
- Rust/C ratio: 1.733117
- allocator `GlobalAlloc::alloc`: 173,101,164 exclusive instructions

## Candidate and validation

Before calling `lock_free_lists`, allocation loaded the exact-size head with
relaxed ordering. A null value went directly to the existing normalized
`System` miss path. A nonnull value entered the accepted critical section,
loaded the head again, and popped it only while locked.

The change does not dereference a pointer outside the lock. A concurrent pop
may turn the second load null, producing a safe cache miss; a concurrent push
after an observed null may be missed by this allocation and reused later.

- All four focused allocator tests pass with all features, including parallel
  reuse.
- Strict all-feature library pedantic Clippy passes.
- Formatting and diff checks pass.
- The exact LUSK6 workload reaches the same 4,873-processed-clause proof,
  reports `Unsatisfiable`, and exits zero.

## Measurement

The candidate retires 9,108,914,649 instructions:

- global delta: +2,490,636;
- global regression: +0.027350%;
- Rust/C ratio: 1.733591.

The intended allocator owner rises from 173,101,164 to 174,722,699
instructions, an increase of 1,621,535 or 0.936756%. The extra relaxed load on
the more common hit path costs more than the skipped miss locks.
`TermTree::insert` also rises by 720,860 instructions, from 658,797,132 to
659,517,992, accounting with the allocator for most of the global regression.

The deterministic and intended-owner gates both fail. Native timing and
compatibility/resource matrices are skipped because allocator micro-layout
candidates with better deterministic results have already regressed more
strongly in native production measurements.

## Result

Reject the empty-list hint and restore the accepted unconditional locked
probe byte-for-byte. Experiment 261 remains the production baseline at
9,106,424,013 instructions, or 1.733117 times C.

The raw profile is retained at:

```text
.artifacts/experiments/2026-07-23-028-allocator-empty-list-hint/rust-callgrind-allocator-empty-list-hint.out
```

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-allocator-empty-list-hint.out \
  target-wsl-266-empty-list-hint/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
