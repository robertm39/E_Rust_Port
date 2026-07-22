# Rejected two-pair inline MGU capacity

## Question

Can the accepted four-pair inline first-order MGU job deque shrink to two
pairs, reducing stack initialization and bookkeeping while retaining enough
capacity for common binary terms?

## Setup

- Parent source: commit `023e4063` (`Inline common first-order MGU jobs`),
  accepted Experiment 211.
- Candidate: change only `INLINE_MGU_JOB_PAIRS` from four to two.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-211-inline-mgu-job-deque/rust-callgrind-inline-mgu-job-deque.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-212-two-inline-mgu-jobs/rust-callgrind-two-inline-mgu-jobs.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Deterministic result

The candidate preserves the expected 4,873-processed-clause LUSK6 proof but
retires 10,660,225,939 instructions. This is 15,578,735 above the
10,644,647,204-instruction parent, a 0.146353% regression. The hypothetical
C/Rust ratio worsens from 2.025869 to 2.028834.

Reducing the inline capacity adds 48,175 Rust allocations. `malloc` rises by
2,167,875 instructions and `free` by 1,351,336. The comparable MGU plus deque
growth aggregate rises from 97,184,034 to 100,890,552 instructions, an increase
of 3,706,518 or 3.813917%. Binary-term intuition does not capture delayed jobs
that coexist with immediate work; two pairs spill often enough to lose both
locally and globally.

## Validation

- The inline front/back ordering and spill regression passes at capacity two.
- The candidate reaches the expected LUSK6 proof and exits zero under
  Callgrind.
- Native and compatibility matrices were intentionally skipped after the
  deterministic rejection.
- Source is restored exactly to the accepted four-pair capacity.

## Decision

Reject and retain four inline pairs. The smaller stack footprint does not
offset 48,175 additional overflow allocations or extra MGU bookkeeping.
Preserve the result to pin the measured capacity choice. The accepted baseline
remains Experiment 211 at 10,644,647,204 instructions, or 2.025869 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-two-inline-mgu-jobs.out \
  target-wsl-212-two-inline-mgu-jobs/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
