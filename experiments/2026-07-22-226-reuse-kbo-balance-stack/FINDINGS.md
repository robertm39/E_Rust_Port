# Accepted reusable KBO balance traversal stack

## Question

Can `OrderControlBlock` retain the traversal capacity used while accumulating
KBO variable balance and term weight, instead of allocating a new
`Vec<(Term, DerefType)>` for every completed walk?

## Setup

- Parent source: commit `f12debbf` (`Record rejected redundant output flag
  deletion`), whose executable source remains accepted Experiment 223.
- Candidate: add one initially empty traversal vector to the OCB and reuse it
  in the first-order, LFHO, and Lambda-order balance walkers. Every walker
  asserts that the scratch is empty on entry and leaves it empty on return.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-223-direct-rewrite-term/rust-callgrind-direct-rewrite-term.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-226-reuse-kbo-balance-stack/rust-callgrind-reuse-kbo-balance-stack.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The original C balance traversal uses operation-local stack macros. Rust's
heap-backed vector paid for a fresh allocation on each first-order walk. The
candidate preserves the same LIFO child order and completed-operation
lifecycle while amortizing capacity across calls on the same ordering object.
It also iterates LFHO arguments through their borrowed slice instead of
materializing an additional argument-clone vector.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,296,792,836 instructions. This is 201,091,460 below the
10,497,884,296-instruction parent, a 1.915543% whole-prover reduction. The
C/Rust ratio improves from 1.997937 to 1.959666.

The parent profile performs 340,049 allocations directly from the KBO balance
walker. Retained capacity reduces that path to one growth across the workload.
Whole-program Rust allocation calls fall from 5,845,869 to 5,505,821, an exact
reduction of 340,048 or 5.816894%. Dereference and variable-balance call counts
remain unchanged, confirming that traversal semantics and work coverage are
preserved.

## Native result

Both binaries completed four alternating warmup pairs followed by 64
alternating production-feature Windows pairs. All 136 processes prove and exit
zero.

Across the 64 measured pairs, wall mean falls from 1.835698 to 1.814541
seconds, an improvement of 1.152537%; wall median improves 1.226946% and the
mean paired change improves 1.079523%. The candidate wins 40 of 64 wall pairs.
Process-CPU mean falls from 1.787354 to 1.770264 seconds, an improvement of
0.956154%; CPU median improves 1.310044% and the mean paired change improves
0.881520%. The candidate wins 36 CPU pairs, ties seven, and loses 21.

The stable last 32 pairs remain positive: wall mean improves 1.549187% and CPU
mean improves 0.770501%. The candidate executable grows 3,072 bytes, from
8,647,680 to 8,650,752, a small cost relative to the consistent production
timing and allocation gains.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-131048-300203` has four
  cases and zero mismatches, covering GEO288, HEN011, LUSK6, and LUSK6ext.
- Combined BOO020/SWV851 resource report
  `.artifacts/e-compare/20260722-131238-080759` has two cases and zero
  mismatches at the standard 60-second/2 GiB limits.
- Isolated BOO020 and SWV851 reports
  `.artifacts/e-compare/20260722-131637-696091` and
  `.artifacts/e-compare/20260722-131841-345015` each have one case and zero
  mismatches. Retained traversal capacity does not reopen either resource
  boundary.
- Maintained report `.artifacts/e-compare/20260722-132050-034811` completes all
  50 cases with zero unexpected mismatches and only the declared
  `sledgehammer` output difference. HEN011, one-second LUSK6, BOO020, and
  SWV851 all match C.
- Both focused KBO/OCB modules pass 20 tests. A new lifecycle test verifies
  that a complete LHS/RHS pair empties the scratch, retains its capacity, and
  restores weight and variable balances.
- The full serial suite passes 4,387 library tests plus every integration and
  binary target. Its first attempt immediately after the 2 GiB resource matrix
  was blocked by Windows paging error 1455 while mapping an existing rlib; the
  same suite passed with one Cargo build job after host pressure subsided.
- Strict all-target/all-feature pedantic Clippy, formatting, the all-feature
  release build, all four documentation gates, and vendored-C cleanliness
  pass.

## Decision

Accept. The reusable OCB-owned scratch preserves KBO traversal order and
completed-call state while removing 340,048 whole-program allocations.
Deterministic work improves 1.915543%, native wall and CPU samples both improve,
and proof, resource, maintained-matrix, and repository-wide quality gates pass.
The accepted baseline becomes 10,296,792,836 instructions, or 1.959666 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-reuse-kbo-balance-stack.out \
  target-wsl-226-reuse-kbo-balance-stack/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-223-direct-rewrite-term\release\eprover.exe `
  -CandidateExe .\target\native-226-reuse-kbo-balance-stack\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\native-lusk.csv
```
