# Accepted detached indexed target position

## Question

When indexed paramodulation already passes its target clause separately, can
the compact target position retain only its selected literal and literal index
instead of cloning the complete `Clause` into `ClausePos`?

## Setup

- Parent source: commit `8aa84a39` (`Skip duplicate trees for short literal
  lists`), accepted Experiment 227.
- Candidate: add a compact-position unpacker that clones only the selected
  `Eqn` handles, retains its literal index, side, and term path, and leaves the
  cursor without an owned clause. Use it only for indexed target positions;
  the separate target `&Clause` remains the owner for maximality checks and
  inference construction.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-227-skip-short-literal-dedup/rust-callgrind-skip-short-literal-dedup.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-228-detached-indexed-target-position/rust-callgrind-detached-indexed-target-position.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

Original C clause positions hold pointers into their owning clause; unpacking
does not deep-copy the clause. Rust still uses owned positions for public and
mutable traversal APIs, but the indexed target helper already has a stable
separate clause owner and needs only the selected literal shape.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,145,909,203 instructions. This is 75,027,002 below the
10,220,936,205-instruction parent, a 0.734052% whole-prover reduction. The
C/Rust ratio improves from 1.945229 to 1.930950.

Full `Clause::clone` calls fall from 136,318 to 78,856, an exact reduction of
57,462 or 42.152907% matching the indexed target call site. Whole-program Rust
allocation calls fall from 5,258,856 to 5,086,470, an exact reduction of
172,386 or 3.278013%: each avoided target clause clone removed three
allocations. The source-side indexed position clone remains unchanged for a
separate experiment.

## Native result

Both binaries completed four alternating warmup pairs followed by 64
alternating production-feature Windows pairs. All 136 processes prove and exit
zero.

Across the 64 measured pairs, wall mean falls from 1.756904 to 1.749802
seconds, an improvement of 0.404250%; wall median improves 0.638659% and mean
paired wall change improves 0.268120%. Process-CPU mean falls from 1.725098 to
1.717041 seconds, an improvement of 0.467025%; CPU median improves 0.909091%
and mean paired CPU change improves 0.333765%. The candidate wins 33 wall pairs
and 31 CPU pairs with three CPU ties.

The last 32 pairs remain positive, improving wall mean 0.380171% and CPU mean
0.423370%. The candidate executable shrinks 5,632 bytes, from 8,650,752 to
8,645,120.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-150110-450639` has four
  cases and zero mismatches, covering GEO288, HEN011, LUSK6, and LUSK6ext.
- Combined BOO020/SWV851 resource report
  `.artifacts/e-compare/20260722-150306-516174` has two cases and zero
  mismatches at the standard 60-second/2 GiB limits.
- Maintained report `.artifacts/e-compare/20260722-150714-797279` completes all
  50 cases with zero unexpected mismatches and only the declared
  `sledgehammer` output difference. HEN011, one-second LUSK6, BOO020, and
  SWV851 all match C.
- All four compact-position tests and all 53 paramodulation tests pass. The
  round-trip regression verifies the detached cursor has no clause owner while
  preserving literal identity, literal index, side, term path, and selected
  subterm.
- The full serial suite passes 4,388 library tests plus every integration and
  binary target.
- Strict all-target/all-feature pedantic Clippy, formatting, the all-feature
  release build, all four documentation gates, and vendored-C cleanliness
  pass.

## Decision

Accept. The detached target cursor moves indexed paramodulation closer to the
C pointer-owned position model while preserving the separate Rust clause owner
used by inference checks. It removes 57,462 full clause clones and 172,386
allocations, improves deterministic and native performance, shrinks the
binary, and passes all compatibility and quality gates. The accepted baseline
becomes 10,145,909,203 instructions, or 1.930950 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-detached-indexed-target-position.out \
  target-wsl-228-detached-indexed-target-position/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-227-skip-short-literal-dedup\release\eprover.exe `
  -CandidateExe .\target\native-228-detached-indexed-target-position\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\native-lusk.csv
```
