# PD-tree cursor problem-mode specialization

## Question

Can the PD-tree matching cursor capture its invariant first-order versus
higher-order mode at dispatch and use separate optimized monomorphizations,
instead of testing the mode inside every symbol traversal?

## Setup

- Parent source: commit `50df1c97` (`Permit justified unsafe Rust`), whose
  executable source is accepted Experiment 230.
- Candidate: dispatch `search_next_matching_occurrence_impl` through a const
  generic problem-mode parameter. The first-order demodulator path and the
  higher-order lambda-lifting path retain the same traversal state machine,
  ordering, constraints, bindings, and output.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-230-lazy-ac-parent-snapshot/rust-callgrind-lazy-ac-parent-snapshot.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at
  5,254,361,329 instructions.

## Results

The candidate reaches the exact 4,873-processed-clause LUSK6 proof at
9,923,564,772 instructions. This is 86,536,024 below the
10,010,100,796-instruction parent, a 0.864487% whole-prover reduction. The
C/Rust ratio improves from 1.905103 to 1.888634.

Callgrind assigns 1,756,960,849 parent instructions to the two callers of the
shared cursor implementation, versus 1,709,361,574 candidate instructions to
the specialized cursor and its callees. The cursor aggregate therefore falls
47,599,275 instructions, or 2.709182%. Traversal counts, the exact proof, and
the binding/backtracking call counts remain unchanged. The remaining global
gain is optimized-code layout around the now separately compiled first-order
cursor.

Four alternating warmup pairs and 64 measured Windows pairs completed with
exit zero for both binaries. Across all pairs, candidate wall mean improves
0.688038% and process-CPU mean improves 0.197480%. Wall median improves
0.046230%, while the quantized CPU median regresses 0.219298%. Mean paired
wall and CPU changes improve 0.645435% and 0.103351%; the candidate wins 34
wall pairs and 31 CPU pairs, with five CPU ties.

The stable last 32 pairs remain positive in the means: wall improves
1.020277% and CPU improves 0.208131%. Mean paired wall and CPU changes improve
0.921406% and 0.164287%; paired medians improve 1.310301% and 0.444444%.
Candidate medians are effectively tied, at -0.017034% wall and -0.224719%
CPU. The candidate executable grows 14,336 bytes, from 8,640,000 to 8,654,336
bytes.

## Compatibility and validation

- All 41 focused PD-tree tests pass, including first-order live-substitution,
  traversal-order, binding, backtracking, constraint, and higher-order query
  cases. The lambda-lifting generalization regression separately exercises
  the higher-order cursor monomorphization and passes.
- Focused proof report `.artifacts/e-compare/20260722-201905-308509` has GEO,
  LUSK6, and LUSK6ext byte-exact. HEN reaches the 60-second CPU cutoff after
  the host slowed markedly during this session.
- Isolated HEN reports at 60 and 90 seconds,
  `.artifacts/e-compare/20260722-202411-128528` and
  `.artifacts/e-compare/20260722-202658-050358`, retain that cutoff. With a
  diagnostic 120-second allowance, report
  `.artifacts/e-compare/20260722-203443-397467` proves in 93.49 seconds with
  byte-exact normalized output. The same parent LUSK binary uses 3.585 seconds
  mean CPU in this session versus 1.932 seconds in Experiment 230, so the
  standard HEN cutoff is not reproducible for the unchanged baseline on this
  host. The failed reports are retained rather than reclassified.
- Strict resource report `.artifacts/e-compare/20260722-203022-287161` keeps
  the standard 60-second/2 GiB limits and has zero mismatches for BOO020 and
  SWV851, including exact `ResourceOut` output and exit status 8.
- The immediately preceding maintained 50-case report
  `.artifacts/e-compare/20260722-175442-598051` has zero unexpected
  mismatches. It was not duplicated on the throttled host because its
  maintained 90-second HEN allowance is below the measured 93.49-second
  candidate completion and below the slowdown-scaled parent requirement.
- The full serial all-target/all-feature suite passes 4,388 library tests plus
  every integration and binary target. The first parallel compile exhausted
  the Windows paging file; the first serial run then observed one transient
  scheduler-budget test failure under the same host pressure. That test passed
  alone, and the complete serial suite passed on immediate repeat.
- Strict all-target/all-feature pedantic Clippy, formatting, the locked
  all-feature release build, all four documentation gates, and vendored-C
  cleanliness pass.

## Decision

Accept. Capturing the problem mode once and monomorphizing the shared state
machine preserves both first-order demodulation and higher-order lambda
lifting while removing a runtime mode branch from the dominant cursor loop.
The exact deterministic workload improves 0.864487%, native means remain
positive across the full and stable-half samples, and strict resource behavior
is unchanged. The accepted baseline becomes 9,923,564,772 instructions, or
1.888634 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-specialize-pdt-cursor.out \
  target-wsl-231-specialize-pdt-cursor/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-230-lazy-ac-parent-snapshot\release\eprover.exe `
  -CandidateExe .\target\native-231-specialize-pdt-cursor\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-22-231-specialize-pdt-cursor\native-lusk.csv
```
