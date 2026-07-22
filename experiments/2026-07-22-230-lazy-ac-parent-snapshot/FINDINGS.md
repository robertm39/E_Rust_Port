# Accepted lazy AC proof-parent snapshots

## Question

Can ordinary forward simplification avoid rebuilding the signature's AC-axiom
proof-parent metadata when proof documentation is disabled?

## Setup

- Parent runtime source: commit `73398df3` (`Avoid cloning indexed source
  clauses`), accepted Experiment 229.
- Candidate: resolve AC-axiom parent identities only when AC handling and live
  proof documentation are both active. The ordinary branch continues to call
  `clause_remove_ac_resolved` directly and never materializes the unused
  parent vector. The same condition is applied to watchlist simplification.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-229-detached-indexed-source-position/rust-callgrind-detached-indexed-source-position.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-230-lazy-ac-parent-snapshot/rust-callgrind-lazy-ac-parent-snapshot.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

C keeps live AC-axiom clause pointers in the signature. Rust intentionally
stores stable derivation identities instead, but the forward path was
resolving those identities through the complete proof state on every call,
before borrowing the mutable term bank and processed sets. That snapshot is
needed only by the proof-documentation branch. The normal prover branch uses
the signature's AC-axiom count for derivation metadata and never reads the
resolved parents.

## Deterministic result

The candidate proves LUSK6 and retires 10,010,100,796 instructions. This is
79,461,079 below the 10,089,561,875-instruction parent, a 0.787557% whole-prover
reduction. The C/Rust ratio improves from 1.920226 to 1.905103.

Whole-program Rust allocation calls fall from 4,886,424 to 4,380,910, an exact
reduction of 505,514 or 10.345275%. The eliminated snapshots had caused both
their own vector allocations and temporary proof-set lookup allocations.
`ProofState::proof_clause_by_derivation_ref` calls fall from 379,349 to zero on
this production workload. The remaining 12,174 full clause clones are
unchanged, confirming that the gain is independent of Experiments 228 and 229.

## Native result

Four alternating warmup pairs and 64 measured production-feature Windows
pairs completed successfully for both binaries.

Across all 64 measured pairs, wall mean falls from 1.999357 to 1.944175
seconds, an improvement of 2.759977%; wall median improves 3.755152%, and mean
paired wall change improves 2.644974%. Process-CPU mean falls from 1.932129 to
1.871338 seconds, an improvement of 3.146323%; CPU median improves 3.688525%,
and mean paired CPU change improves 2.948831%. The candidate wins 41 wall
pairs and 43 CPU pairs, with five CPU ties.

The last 32 pairs remain stable: wall mean improves 2.751018% and CPU mean
improves 3.181818%; paired wall and CPU medians improve 2.865460% and
3.055556%, respectively. The candidate wins 23 of those wall pairs and 24 CPU
pairs, with three CPU ties. The candidate executable grows 1,024 bytes, from
8,638,976 to 8,640,000 bytes.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-164223-468657` has four
  cases and zero mismatches, covering GEO288, HEN011, LUSK6, and LUSK6ext at
  the standard 60-second/2 GiB limits.
- Focused resource report `.artifacts/e-compare/20260722-164421-492209` has
  BOO020 and SWV851 matching C exactly at the standard 60-second/2 GiB limits.
- Maintained report `.artifacts/e-compare/20260722-175442-598051` completes all
  50 cases with zero unexpected mismatches and the one declared
  `sledgehammer` normalized-output difference. The maintained matrix now runs
  BOO020 and SWV851 last, gives HEN011 a 90-second minimum on slow hosts, and
  retains the standard 60-second internal limits for both resource cases.
- The compatibility run exposed two independent harness reliability issues.
  Commit `1344759e` makes declared differences an allowed set, so a known
  difference disappearing cannot fail the gate. Commit `1b5aa2a0` isolates
  resource-stress cases and extends only the outer cleanup grace. All 38
  interop harness tests pass.
- All 19 focused forward-modification tests and five focused watchlist tests
  pass, including AC proof-documentation coverage that verifies current parent
  identifiers are still resolved when documentation is enabled.
- The full serial suite passes 4,388 library tests plus every integration and
  binary target under all features.
- Strict all-target/all-feature pedantic Clippy, formatting, the all-feature
  release build, all four documentation gates, and vendored-C cleanliness
  pass.

## Decision

Accept. The ordinary prover no longer pays to reconstruct proof-documentation
metadata it cannot consume. The change matches the ownership and laziness of
the C path, removes 379,349 proof-parent searches and 505,514 allocations,
improves deterministic and native performance, and preserves the documented
AC-resolution path and all compatibility behavior. The accepted baseline
becomes 10,010,100,796 instructions, or 1.905103 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-lazy-ac-parent-snapshot.out \
  target-wsl-230-lazy-ac-parent-snapshot/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-229-detached-indexed-source-position\release\eprover.exe `
  -CandidateExe .\target\native-230-lazy-ac-parent-snapshot\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\native-lusk.csv
```
