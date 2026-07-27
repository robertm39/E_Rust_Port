# Accepted detached indexed source position

## Question

Can indexed paramodulation use the detached selected-literal cursor for source
positions too, passing the separately owned source clause explicitly to
maximality checks instead of cloning the complete clause into `ClausePos`?

## Setup

- Parent source: commit `eee5507f` (`Avoid cloning indexed target clauses`),
  accepted Experiment 228.
- Candidate: use `unpack_clause_pos_literal` for the indexed source cursor and
  pass `source_entry.clause()` explicitly to the substituted maximality check.
  Existing clause-backed entry points capture their source clause once before
  repeated checks.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-228-detached-indexed-target-position/rust-callgrind-detached-indexed-target-position.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-229-detached-indexed-source-position/rust-callgrind-detached-indexed-source-position.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

Like the accepted target-position half, this restores the original C
pointer-owned position shape at a call site that already has a stable clause
owner. The cursor retains its literal index, selected equation handles, side,
and term path; the separate clause provides complete literal context for
ordering and inference construction.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 10,089,561,875 instructions. This is 56,347,328 below the
10,145,909,203-instruction parent, a 0.555370% whole-prover reduction. The
C/Rust ratio improves from 1.930950 to 1.920226.

Full `Clause::clone` calls fall from 78,856 to 12,174, an exact reduction of
66,682 or 84.561733% matching the indexed source call site. Whole-program Rust
allocation calls fall from 5,086,470 to 4,886,424, an exact reduction of
200,046 or 3.932904%: each avoided source clause clone removes three
allocations. The remaining 12,174 clause clones belong to other indexing and
proof-control paths and are outside this candidate.

## Native result

Both binaries completed four alternating warmup pairs followed by 64
alternating production-feature Windows pairs. All 136 processes prove and exit
zero.

Across the 64 measured pairs, wall mean falls from 1.667706 to 1.658525
seconds, an improvement of 0.550493%; mean paired wall change improves
0.449631%. Process-CPU mean falls from 1.638184 to 1.624023 seconds, an
improvement of 0.864382%; CPU median improves 0.961538% and mean paired CPU
change improves 0.764458%. The candidate wins 34 wall pairs and 38 CPU pairs
with six CPU ties.

The last 32 pairs are wall-neutral: mean regresses 0.069830% while median
improves 0.965126%. Their CPU mean and median improve 0.525665% and 1.000000%,
respectively. The candidate executable shrinks 6,144 bytes, from 8,645,120 to
8,638,976.

## Compatibility and validation

- Focused proof report `.artifacts/e-compare/20260722-154259-908560` has four
  cases and zero mismatches, covering GEO288, HEN011, LUSK6, and LUSK6ext.
- Combined BOO020/SWV851 resource report
  `.artifacts/e-compare/20260722-154450-131513` has two cases and zero
  mismatches at the standard 60-second/2 GiB limits.
- Maintained report `.artifacts/e-compare/20260722-154857-170811` completes all
  50 cases with zero unexpected mismatches and only the declared
  `sledgehammer` output difference. HEN011, one-second LUSK6, BOO020, and
  SWV851 all match C.
- All 53 focused paramodulation tests pass, including indexed source and target
  queries, reused substitutions, higher-order CSU paths, simultaneous modes,
  derivation metadata, and variable-normalization order.
- The full serial suite passes 4,388 library tests plus every integration and
  binary target.
- Strict all-target/all-feature pedantic Clippy, formatting, the all-feature
  release build, all four documentation gates, and vendored-C cleanliness
  pass.

## Decision

Accept. Explicit source-clause ownership lets indexed paramodulation preserve
the complete maximality context without deep-copying it into every compact
position. The change removes 66,682 clause clones and 200,046 allocations,
improves deterministic and native CPU performance, shrinks the binary, and
passes all compatibility and quality gates. The accepted baseline becomes
10,089,561,875 instructions, or 1.920226 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-detached-indexed-source-position.out \
  target-wsl-229-detached-indexed-source-position/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-228-detached-indexed-target-position\release\eprover.exe `
  -CandidateExe .\target\native-229-detached-indexed-source-position\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\native-lusk.csv
```
