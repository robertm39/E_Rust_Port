# Accepted short-list duplicate-removal guard

## Question

Can `EqnList::remove_duplicates` restore the original C guard that skips its
temporary literal tree when a list has fewer than two literals?

## Setup

- Parent source: commit `473b4b23` (`Reuse KBO balance traversal storage`),
  accepted Experiment 226.
- Candidate: return zero before constructing the `BTreeSet` when the equation
  list is empty or contains one literal.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-226-reuse-kbo-balance-stack/rust-callgrind-reuse-kbo-balance-stack.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-227-skip-short-literal-dedup/rust-callgrind-skip-short-literal-dedup.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

Upstream `EqnListRemoveDuplicates` explicitly tests `list && list->next`
before allocating its `PObjTree`. The Rust port previously constructed a
`BTreeSet` and inserted every literal unconditionally. The guard is therefore
both a semantic no-op and a direct restoration of the C ownership boundary.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,220,936,205 instructions. This is 75,856,631 below the
10,296,792,836-instruction parent, a 0.736702% whole-prover reduction. The
C/Rust ratio improves from 1.959666 to 1.945229.

All 120,780 duplicate-removal calls in this profile take the restored
short-list path. The duplicate-removal call edge falls from 38,287,260 to
1,570,140 instructions, down 36,717,120 or 95.899054%. The four-integer-key
B-tree allocation owner disappears. Whole-program Rust allocation calls fall
from 5,505,821 to 5,258,856, an exact reduction of 246,965 or 4.485525%.

## Native result

Both binaries completed four alternating warmup pairs followed by 64
alternating production-feature Windows pairs. All 136 processes prove and exit
zero.

Across the 64 measured pairs, wall mean falls from 1.795948 to 1.749528
seconds, an improvement of 2.584756%; wall median improves 2.210493% and mean
paired wall change improves 2.434043%. The candidate wins 47 of 64 wall pairs.
Process-CPU mean falls from 1.764893 to 1.716553 seconds, an improvement of
2.738968%; CPU median improves 2.222222% and mean paired CPU change improves
2.576397%. The candidate wins 46 CPU pairs and ties three.

The last 32 pairs remain positive, improving wall mean 1.561805% and CPU mean
1.954216%. Both executables are exactly 8,650,752 bytes.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-141132-327518` has four
  cases and zero mismatches, covering GEO288, HEN011, LUSK6, and LUSK6ext.
- Combined BOO020/SWV851 resource report
  `.artifacts/e-compare/20260722-141326-408827` has two cases and zero
  mismatches at the standard 60-second/2 GiB limits.
- Maintained report `.artifacts/e-compare/20260722-141729-461384` completes all
  50 cases with zero unexpected mismatches and only the declared
  `sledgehammer` output difference. HEN011, one-second LUSK6, BOO020, and
  SWV851 all match C.
- All 21 focused equation-list tests pass. A new regression covers empty and
  singleton lists while the existing multi-literal test continues to cover
  commutative duplicate equality and polarity distinctions.
- The full serial suite passes 4,388 library tests plus every integration and
  binary target.
- Strict all-target/all-feature pedantic Clippy, formatting, the all-feature
  release build, all four documentation gates, and vendored-C cleanliness
  pass.

## Decision

Accept. The guard restores the exact C short-list ownership boundary, removes
246,965 allocations without changing literal order or equality semantics, and
improves both deterministic and native performance. All behavioral, resource,
and repository-wide quality gates pass. The accepted baseline becomes
10,220,936,205 instructions, or 1.945229 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-skip-short-literal-dedup.out \
  target-wsl-227-skip-short-literal-dedup/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-226-reuse-kbo-balance-stack\release\eprover.exe `
  -CandidateExe .\target\native-227-skip-short-literal-dedup\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\native-lusk.csv
```
