# Experiment 275: Structural-weight term-identity fast path

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can structural term comparison return immediately for two identical shared
term handles, avoiding cached-weight, type, arity, argument-borrow, and
recursive comparison work while preserving every comparison result?

## Setup

- Parent source: commit `edcf97bc` (`perf: reject term type identity fast
  path`); executable source remains accepted Experiment 270.
- Parent WSL Callgrind profile:
  `.artifacts/experiments/2026-07-23-032-borrow-active-pdt-frame/rust-callgrind-borrow-active-pdt-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: add one `Term` pointer-identity guard at the entry to
  `term_struct_weight_compare`. All nonidentical terms retain the exact
  existing C-shaped comparison path.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

## Results

### Deterministic profile

The candidate proves the expected unsatisfiable result and falls from
8,992,812,925 to 8,824,108,030 instructions, a reduction of 168,704,895 or
1.875997%. The hypothetical Rust/C ratio improves from 1.711495 to 1.679387.

The guard recognizes substantial shared structure. Calls on the dominant
recursive comparator clone fall from 2,037,807 to 1,207,973, a reduction of
829,834 or 40.721913%. General term-type comparisons fall from 1,138,621 to
267,690 calls, a reduction of 870,931 or 76.489982%. The proof-search result,
processed-clause count, and dominant unrelated owners remain unchanged.

### Native timing

Two independent native blocks each use four alternating warmup pairs and 64
alternating measured pairs. Both reverse the deterministic result:

| Sample | Wall mean | CPU mean | Wall wins | CPU wins | CPU ties |
| --- | ---: | ---: | ---: | ---: | ---: |
| Block 1 | +0.360040% | +0.361533% | 25 | 22 | 15 |
| Block 2 | +0.144893% | +0.415537% | 28 | 25 | 12 |
| Combined | +0.252311% | +0.388543% | 53 | 47 | 27 |

Positive changes are regressions. Combined mean paired wall and CPU changes
regress 0.324733% and 0.462250%; paired medians regress 0.395387% wall and tie
on the quantized CPU clock. The combined last 32 pairs from each block also
regress 0.261702% wall and 0.543774% CPU by aggregate means.

All 256 measured processes and all 16 warmup processes exit zero. Direct
parent and candidate output is byte-identical, including the expected proof
and SZS status. The candidate executable is 8,936,448 bytes, 15,872 bytes
smaller than the 8,952,320-byte parent.

## Validation

- All 46 candidate term-function tests pass in default and all-feature
  configurations.
- A focused regression covers the exact identical-handle result.
- Strict all-feature library pedantic Clippy passes.
- Exact WSL Callgrind and direct native runs prove LUSK6.
- After rejection, the entry guard and its regression are removed and the
  accepted `termfunc.rs` is restored byte-for-byte.
- Compatibility matrices are skipped because both native production blocks
  reject this performance-only code shape.

## Decision

Reject the comparator-entry guard. Pointer identity is semantically exact and
removes 1.875997% of instrumented work, but both independent production blocks
are slower and their stable halves remain negative. Keep Experiment 270 as the
accepted executable baseline at 8,992,812,925 instructions, or 1.711495 times
C.

The profile does motivate a distinct follow-up: roughly 95% of the eliminated
recursive-clone calls are child comparisons. A child-edge identity check can
avoid both the recursive call and its return on a hit without adding a branch
to every top-level comparison. That formulation is not covered by this
rejection.

## Reproduction

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-275-struct-weight-term-identity\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-002-struct-weight-term-identity\native-lusk.csv
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-002-struct-weight-term-identity/rust-callgrind-struct-weight-term-identity.out \
  target-wsl-275-struct-weight-term-identity/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
