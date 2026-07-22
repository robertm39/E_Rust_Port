# Accepted direct deferred rewrite-term construction

## Question

Can recursive subterm normalization construct the replacement top cell as
soon as the first changed child is known, instead of first collecting every
normalized child in a temporary `Vec<Term>` and then copying that vector into
a separately allocated term?

## Setup

- Parent source: commit `7334d07c` (`Record rejected PD-tree terminal
  substitution reuse`). Experiments 215 through 222 changed only evidence
  after restoring their rejected candidates, so its executable source remains
  accepted Experiment 214.
- Candidate: defer construction while all visited children remain unchanged.
  At the first change, allocate `Term::top_copy_without_args`, copy the
  unchanged prefix directly into its argument storage, and write that and all
  later normalized children directly into the replacement term.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-214-move-termtree-insert-links/rust-callgrind-move-termtree-insert-links.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-223-direct-rewrite-term/rust-callgrind-direct-rewrite-term.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

The replacement retains `TermArgs`' inline storage for arity one and two.
Only the temporary normalization vector is removed; the shared top cell still
uses the standard term-bank insertion path and preserves all rewrite links,
properties, and normal-form dates.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,497,884,296 instructions. This is 134,757,089 below the
10,632,641,385-instruction parent, a 1.267391% whole-prover reduction. The
C/Rust ratio improves from 2.023584 to 1.997937.

The recursive normalizer's exclusive instructions fall from 315,343,713 to
276,321,216, a reduction of 39,022,497 or 12.374592%. Its two optimized
recursive instances fall inclusively by 108,169,495 and 88,134,261
instructions respectively; those inclusive totals overlap through recursion
and are not summed.

The old reconstruction path called the temporary `Vec<Term>` allocator and
`IntoIter<Term>::drop` 319,552 times, costing 17,498,227 and 34,831,168
instructions respectively. Both sites disappear. Across the whole executable,
`__rust_alloc` calls fall from 6,312,342 to 5,845,869, a reduction of 466,473
or 7.389856%. `top_copy_without_args` rises by 920,683 instructions and
`term_top_insert` rises by 179,091, but those small costs are dominated by the
removed staging vector, ownership traffic, and loop work.

## Native result

The production-feature parent and candidate binaries completed 64 alternating
native Windows pairs, and all 128 processes exited zero. The first candidate
observation was a 3.620557-second cold/scheduler outlier. With that first pair
excluded, candidate and parent wall means are effectively tied at 1.834230 and
1.834890 seconds, a 0.035981% candidate improvement; CPU mean differs by a
noisy 0.123933% in the other direction because Windows reports CPU in coarse
15.625 ms quanta.

The stable last 32 pairs favor the candidate in both mean measures. Wall mean
falls from 1.829352 to 1.823059 seconds (0.343997%) and wall median falls
0.558896%. CPU mean falls from 1.792969 to 1.789063 seconds (0.217865%). Mean
paired wall and CPU changes improve 0.258135% and 0.115718%, respectively.
Wall wins split 15 of 32; CPU wins split 16 with one tie. Median paired wall is
0.924022% slower while median paired CPU is 0.423729% faster, so native timing
is best characterized as neutral-to-slightly-positive rather than as the
acceptance gate. The deterministic 1.267391% instruction reduction is the
decisive performance evidence. The Windows executable grows 2,560 bytes, from
8,645,120 to 8,647,680.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-111639-268358` has four
  cases and zero mismatches.
- Focused BOO020/SWV851 resource report
  `.artifacts/e-compare/20260722-111834-549152` has two cases and zero
  mismatches at the standard 60-second/2 GiB limits.
- Isolated BOO020 and SWV851 reports
  `.artifacts/e-compare/20260722-112240-210801` and
  `.artifacts/e-compare/20260722-112449-283145` each have one case and zero
  mismatches.
- Focused HEN011 and LCL365 reports
  `.artifacts/e-compare/20260722-112658-517637` and
  `.artifacts/e-compare/20260722-112829-573823` each have one case and zero
  mismatches.
- Full report `.artifacts/e-compare/20260722-112851-460818` completes all 50
  cases with 49 exact comparisons and the declared `sledgehammer` output
  difference. BOO020 is the sole unexpected row: after the C process consumed
  its full 2 GiB/60-second budget, retained WSL memory made Rust abort on a
  196,608-byte allocation with exit 9. Both the combined resource report and a
  clean isolated BOO020 run reproduce exact parity, identifying this as the
  established sequential-harness memory-pressure artifact rather than a
  candidate behavior change.
- All 33 focused rewrite tests pass, including a new binary-parent regression
  that changes the first or second child and verifies the other child is
  preserved.
- The full serial 4,386-test suite plus integration and binary targets passes.
- Strict all-target pedantic Clippy, formatting, the all-feature release build,
  all four documentation gates, and vendored-C cleanliness pass.

## Decision

Accept. The change removes a per-rebuilt-node staging allocation while
preserving inline term storage and exact proofs. Deterministic whole-program
instructions improve by 1.267391%, total allocator calls improve by 7.389856%,
native warmed timing is neutral-to-slightly-positive, and correctness gates
pass. The accepted baseline becomes 10,497,884,296 instructions, or 1.997937
times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-rewrite-term.out \
  target-wsl-223-direct-rewrite-term/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-214-move-termtree-insert-links\release\eprover.exe `
  -CandidateExe .\target\native-223-direct-rewrite-term\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-22-223-direct-rewrite-term\native-lusk.csv
```
