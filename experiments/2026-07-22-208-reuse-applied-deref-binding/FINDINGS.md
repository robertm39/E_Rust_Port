# Rejected applied-dereference binding reuse

## Question

After Experiment 207 fused the dereference-root condition, can the applied
free-variable expansion reuse the binding already loaded by its caller instead
of cloning the head and binding handles again?

## Setup

- Parent source: commit `5fbefe8d` (`Fuse term-bank dereference root check`),
  accepted Experiment 207.
- Candidate: classify the applied head with one argument/binding retrieval,
  pass the owned binding to a private expansion helper, and use the same helper
  in higher-order instantiated insertion.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-207-fuse-deref-root-limit-check/rust-callgrind-fuse-deref-root-limit-check.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-208-reuse-applied-deref-binding/rust-callgrind-reuse-applied-deref-binding.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

`Term::argument` and `Term::binding` clone reference-counted handles. The
candidate removes repeated clones while retaining the public checked expansion
entry point and the all-build arity precondition.

## Deterministic result

The candidate preserves the expected LUSK6 proof and retires 10,803,270,649
instructions. This is 2,024,554 below the 10,805,295,203-instruction parent, a
0.018737% reduction. The hypothetical C/Rust ratio is 2.056058.

The comparable dereference-root inclusive aggregate falls from 197,441,979 to
195,510,044 instructions, saving 1,931,935 or 0.978482%. The remaining small
whole-program difference includes the second known-binding caller in
higher-order instantiated insertion.

## Native result

Both binaries were warmed before 16 alternating native Windows pairs. The
candidate mean is 2.034677 seconds versus 2.005247 for the parent, a 1.467673%
regression. Candidate median is 2.031245 versus 2.010045, a 1.054708%
regression. Mean paired regression is 1.655325%, paired median regression is
1.270066%, and the parent wins nine pairs to seven. All 32 runs prove with exit
zero. The candidate executable is 1,024 bytes smaller.

## Validation

- All 122 focused term-bank tests pass with the candidate and after
  restoration.
- Formatting passes.
- The candidate reaches the expected LUSK6 proof and exit zero under
  Callgrind and in every native run.
- Compatibility matrices were intentionally skipped after native timing
  rejected the candidate.

## Decision

Reject and restore the parent source. Removing the extra reference-counted
handle traffic saves a small deterministic amount and shrinks the executable,
but every native distribution statistic regresses by more than 1%. Preserve
the result to avoid retrying this code-shape transformation. The accepted
baseline remains Experiment 207.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-reuse-applied-deref-binding.out \
  target-wsl-208-reuse-applied-deref-binding/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
