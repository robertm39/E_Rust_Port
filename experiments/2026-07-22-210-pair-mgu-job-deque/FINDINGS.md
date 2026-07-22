# Accepted pair-based first-order MGU job deque

## Question

Can first-order MGU preserve the C `PQueueStore`/`PQueueBury` work order while
avoiding a freshly initialized 128-slot `Option<Term>` queue on every
operation?

## Setup

- Parent source: commit `5fbefe8d` (`Fuse term-bank dereference root check`),
  accepted Experiment 207. Experiments 208 and 209 record rejected candidates
  and restore this same source.
- Candidate: represent each unification equation as `(Term, Term)` in an
  operation-local `VecDeque`. Immediate jobs are pushed and popped at the back;
  variable-containing jobs are pushed at the front, so they remain delayed in
  exactly the C order.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-207-fuse-deref-root-limit-check/rust-callgrind-fuse-deref-root-limit-check.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-210-pair-mgu-job-deque/rust-callgrind-pair-mgu-job-deque.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The queue stores whole equations rather than alternating term handles. A back
push is equivalent to two C stores followed by two last-element pops. A front
push is equivalent to the two C buries and preserves FIFO order among delayed
equations. The general C-compatible `PQueue` remains unchanged except that its
now-unused crate-private owned-pop extension and isolated test are removed.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,658,676,694 instructions. This is 146,618,509 below the
10,805,295,203-instruction parent, a 1.356913% reduction. The C/Rust ratio
improves from 2.056443 to 2.028539.

The comparable parent MGU plus queue `store`/`bury` aggregate is 261,887,237
exclusive instructions. Candidate MGU plus `VecDeque::grow` is 89,977,360,
saving 171,909,877 or 65.642709%. Calls to `__rustc::__rust_alloc` remain
exactly 6,457,944 in both binaries, and `malloc` is also exact at 290,004,395
instructions. The gain comes from smaller initialization, paired bookkeeping,
and owned pair removal rather than fewer allocator calls.

## Native result

Both binaries were warmed before 48 alternating native Windows pairs. The
candidate mean is 2.060667 seconds versus 2.067202 for the parent, a 0.316122%
improvement. Candidate median is 2.024423 versus 2.029105, a 0.230752%
improvement. Mean paired improvement is 0.172798%, paired median improvement
is 1.544470%, and the candidate wins 27 pairs to 21. All 96 timed runs prove
and exit zero.

The final 32 pairs also record process CPU time. Candidate mean CPU time is
1.971191 seconds versus 1.982422, improving 0.566502%; median improves
0.396825%, paired mean improves 0.442495%, and candidate wins 20 of those 32
CPU pairs. The release executable shrinks 7,680 bytes, from 8,641,024 to
8,633,344.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-050548-377987` has four
  cases and zero mismatches.
- Focused BOO020/SWV851 resource report
  `.artifacts/e-compare/20260722-050747-915554` has two cases and zero
  mismatches at the standard 60-second/2 GiB limits.
- Maintained report `.artifacts/e-compare/20260722-051232-054260` has all 50
  cases, zero unexpected mismatches, and only the declared sledgehammer output
  difference. HEN011 and the synthetic one-second LUSK6 case match C.
- The delayed-binding failure regression verifies that an earlier binding is
  rolled back when a later queued occurs-check job fails.
- All 21 remaining `PQueue` tests and all 20 focused MGU tests pass.
- The full serial 4,384-test suite plus integration and binary targets passes.
- Strict all-target pedantic Clippy, formatting, the all-feature release build,
  all four documentation gates, and vendored-C cleanliness pass.

## Decision

Accept. Pairing the equations preserves the C scheduling optimization while
removing Rust-only `Option` storage and 128-slot initialization overhead. The
improvement is large and local under deterministic measurement, positive
across robust native statistics, shrinks the binary, and preserves the full
compatibility matrix. The accepted baseline becomes 10,658,676,694
instructions, or 2.028539 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-pair-mgu-job-deque.out \
  target-wsl-210-pair-mgu-job-deque/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
