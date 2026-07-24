# Experiment 277: Structural-weight free-variable identity fast path

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

After structural term comparison has already established that its left operand
is a free variable, can it return immediately when both operands are the same
variable handle instead of cloning their types and calling `TypesCmp`?

## Setup

- Parent source: commit `9fccba0c` (`perf: reject structural child identity
  guard`); executable source remains accepted Experiment 270.
- Parent WSL Callgrind profile:
  `.artifacts/experiments/2026-07-23-032-borrow-active-pdt-frame/rust-callgrind-borrow-active-pdt-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: add one pointer-identity check only inside the existing
  free-variable branch. Nonvariables, recursive child traversal, distinct
  variables, de-Bruijn variables, and every general type comparison retain the
  accepted path.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

Experiments 275 and 276 show that identical shared variables account for about
870,000 avoidable general type comparisons, but broad entry and child-edge
identity guards regress native production. This candidate targets only that
confirmed type-comparison boundary.

## Results

### Deterministic profile

The candidate proves the expected unsatisfiable result and falls from
8,992,812,925 to 8,935,104,637 instructions, a reduction of 57,708,288 or
0.641716%. The hypothetical Rust/C ratio improves from 1.711495 to 1.700512.

The effect is exactly localized. General term-type comparisons fall from
1,138,621 to 267,690 calls, a reduction of 870,931 or 76.489982%. Both visible
recursive comparator call counts remain bit-for-bit unchanged at 2,037,807
and 508,375, confirming that the candidate does not perturb child traversal.

### Native timing

After four alternating warmup pairs, 64 alternating measured pairs reverse
the deterministic result:

- wall mean and median regress 0.655635% and 0.659460%;
- CPU mean and median regress 1.005025% and 1.162791%;
- mean and median paired wall changes regress 0.720951% and 0.890520%;
- mean and median paired CPU changes regress 1.084179% and 1.156108%;
- the candidate wins only 19 wall and 23 CPU pairs, with four CPU ties.

The last 32 pairs remain negative at 0.790241% wall and 0.934915% CPU by
aggregate means. The last 16 aggregate means flatten to small 0.055752% wall
and 0.071788% CPU improvements, but paired means are effectively neutral and
the full stable half rejects the candidate. All 128 measured processes and
eight warmup processes exit zero.

Direct parent and candidate output is byte-identical, including the expected
proof and SZS status. The candidate executable is 8,936,448 bytes, 15,872
bytes smaller than the 8,952,320-byte parent.

## Validation

- All 46 candidate term-function tests pass in default and all-feature
  configurations.
- A focused regression covers the identical free-variable result.
- Strict all-feature library pedantic Clippy passes.
- Exact WSL Callgrind and direct native runs prove LUSK6.
- After rejection, the free-variable guard and its regression are removed and
  accepted `termfunc.rs` is restored byte-for-byte.
- Compatibility matrices are skipped because the native production gate
  rejects this performance-only change.

## Decision

Reject. The free-variable-only placement is semantically exact, preserves
recursive control flow, and removes 0.641716% of instrumented work, but native
production slows across the complete sample and stable last 32. Keep
Experiment 270 as the accepted executable baseline at 8,992,812,925
instructions, or 1.711495 times C.

Together with Experiments 219, 252, and 274 through 276, this exhausts the
safe structural-comparator ownership and identity variants: borrowed type
handles, reused argument borrows, optional-type identity, comparator-entry
identity, child-edge identity, and free-variable identity all fail replicated
native timing despite deterministic wins.

## Reproduction

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-277-struct-weight-free-var-identity\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-004-struct-weight-free-var-identity\native-lusk.csv
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-004-struct-weight-free-var-identity/rust-callgrind-struct-weight-free-var-identity.out \
  target-wsl-277-struct-weight-free-var-identity/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
