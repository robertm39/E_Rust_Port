# Rejected substitution-owned MGU queue reuse

## Question

Can first-order MGU avoid constructing and destroying its C-shaped 128-slot
`PQueue<Term>` on every call by leasing a boxed queue from `Substitution` and
returning it after success or failure?

## Setup

- Parent source: commit `5fbefe8d` (`Fuse term-bank dereference root check`),
  accepted Experiment 207. The intervening Experiment 208 commit records a
  rejected change and restores this same source.
- Candidate: add semantically invisible boxed MGU scratch to `Substitution`,
  exclude it from clone/equality state, drain failed jobs on recycle, and lease
  it from `subst_compute_mgu`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-207-fuse-deref-root-limit-check/rust-callgrind-fuse-deref-root-limit-check.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-209-reuse-mgu-job-queue/rust-callgrind-reuse-mgu-job-queue.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The scratch queue is boxed so every `Substitution` grows by only one pointer.
Its custom clone starts with empty scratch and its equality ignores scratch,
preserving the existing value semantics.

## Deterministic result

The candidate preserves the expected LUSK6 proof and retires 11,038,333,778
instructions. This is 233,038,575 above the 10,805,295,203-instruction parent,
a 2.156707% regression. The hypothetical C/Rust ratio rises from 2.056443 to
2.100795.

Callgrind splits candidate scratch destruction from `subst_compute_mgu`.
Their combined exclusive cost is 196,432,793 instructions versus 226,167,230
for the parent MGU, a local saving of 29,734,437 or 13.147102%. The queue
`store` and `bury` costs are identical between binaries.

That local saving does not amortize the scratch allocation. Calls to
`__rustc::__rust_alloc` increase from 6,457,944 to 6,605,733: exactly 147,789
additional calls, matching the previously measured number of first-order MGU
invocations. This workload therefore does not reuse a `Substitution` across
those MGU invocations. The candidate instead adds a box allocation to every
operation, while scratch drop glue costs 105,263,968 exclusive instructions.
Allocator and global compiler-layout changes account for the remaining
whole-program regression.

## Validation

- The new scratch growth, draining, reuse, clone, and equality unit test
  passes.
- All 20 focused MGU tests pass with all features enabled.
- Strict pedantic library Clippy and formatting pass.
- The candidate reaches the expected LUSK6 proof and exits zero under
  Callgrind.
- Native timing and compatibility matrices were intentionally skipped after
  the decisive deterministic rejection.

## Decision

Reject and restore the parent source. Substitution-owned storage cannot help
when the hot callers create a distinct substitution per MGU operation, and a
boxed lease adds one allocation per call. Preserve the local-cost result as
evidence that future queue work should change the operation-local
representation rather than attach scratch to `Substitution`. The accepted
baseline remains Experiment 207.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-reuse-mgu-job-queue.out \
  target-wsl-209-reuse-mgu-job-queue/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
