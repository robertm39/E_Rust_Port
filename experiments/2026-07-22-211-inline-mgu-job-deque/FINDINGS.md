# Accepted inline first-order MGU job deque

## Question

After Experiment 210 replaced the 128-slot C-shaped MGU queue with a pair
deque, can a four-pair inline ring eliminate its remaining one allocation per
operation without changing immediate/deferred work order?

## Setup

- Parent source: commit `f958b20b` (`Use paired deque for first-order MGU
  jobs`), accepted Experiment 210.
- Candidate: retain four equation pairs in an inline circular buffer and spill
  in logical front-to-back order to a `VecDeque` only on the fifth live job.
  Back pushes/pops remain immediate work and front pushes remain delayed work.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-210-pair-mgu-job-deque/rust-callgrind-pair-mgu-job-deque.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-211-inline-mgu-job-deque/rust-callgrind-inline-mgu-job-deque.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The parent profile records 147,789 MGU calls and 149,976 pair-deque growth
calls. Every MGU performs one initial growth, leaving only 2,187 additional
growths. Four inline pairs therefore cover at least 98.5% of operations without
heap storage. A focused regression exercises front/back ordering through an
inline spill and verifies reuse after the spill deque empties.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,644,647,204 instructions. This is 14,029,490 below the
10,658,676,694-instruction parent, a 0.131625% reduction. The C/Rust ratio
improves from 2.028539 to 2.025869.

Calls to `__rustc::__rust_alloc` fall from 6,457,944 to 6,312,342, eliminating
exactly 145,602 allocations. `malloc` falls by 6,552,090 instructions and
`free` by 4,079,292. Inline ring bookkeeping raises the comparable MGU plus
deque-growth exclusive aggregate from 89,977,360 to 97,184,034 instructions,
an increase of 7,206,674 or 8.009430%; allocator savings exceed that local
cost at whole-program scale.

## Native result

Both binaries were warmed before 64 alternating native Windows pairs, with
wall and process CPU time recorded. Wall time is neutral: candidate mean is
1.762053 seconds versus 1.762505 for the parent, a 0.025600% improvement;
candidate median is 1.740999 versus 1.736453, a 0.261784% regression. Mean
paired wall time regresses 0.054692%, paired median regresses 0.046640%, and
the candidate wins 31 pairs to 33. All 128 timed runs prove and exit zero.

Process CPU time favors the candidate despite Windows timer quantization.
Candidate mean is 1.726807 seconds versus 1.731689, improving 0.281968%; mean
paired CPU improves 0.230832%, medians tie at 1.718750 seconds, and candidate
wins 29 CPU pairs with six ties. The release executable grows 12,288 bytes,
from 8,633,344 to 8,645,632.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-055121-569306` has four
  cases and zero mismatches.
- Focused BOO020/SWV851 resource report
  `.artifacts/e-compare/20260722-055316-315009` has two cases and zero
  mismatches at the standard 60-second/2 GiB limits.
- The immediate parent maintained report
  `.artifacts/e-compare/20260722-051232-054260` has all 50 cases, zero
  unexpected mismatches, and only the declared sledgehammer output difference.
  The candidate changes only equivalent deque storage/order mechanics, so the
  full matrix was not duplicated after the candidate-focused proof/resource
  reports were exact.
- All 21 focused MGU tests, including the new inline spill/order regression,
  pass.
- The full serial 4,385-test suite plus integration and binary targets passes.
- Strict all-target pedantic Clippy, formatting, the all-feature release build,
  all four documentation gates, and vendored-C cleanliness pass.

## Decision

Accept. Native wall time is neutral rather than an acceptance claim, but exact
whole-program instructions and measured process CPU both improve, 145,602 hot
allocations disappear, and proof/resource behavior is exact. The tested
four-pair inline ring is bounded, preserves the simpler deque fallback for rare
wide jobs, and provides a concrete memory/CPU improvement despite its 12 KiB
code-size cost. The accepted baseline becomes 10,644,647,204 instructions, or
2.025869 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-inline-mgu-job-deque.out \
  target-wsl-211-inline-mgu-job-deque/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
