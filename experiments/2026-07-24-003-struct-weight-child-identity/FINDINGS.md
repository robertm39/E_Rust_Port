# Experiment 276: Structural-weight child-identity fast path

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can structural term comparison skip a recursive child call when the two
corresponding argument slots already contain the same shared term handle,
preserving the deterministic savings of Experiment 275 without adding an
identity branch to every top-level comparison?

## Setup

- Parent source: commit `c57776ac` (`perf: reject structural term identity
  guard`); executable source remains accepted Experiment 270.
- Parent WSL Callgrind profile:
  `.artifacts/experiments/2026-07-23-032-borrow-active-pdt-frame/rust-callgrind-borrow-active-pdt-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: at the existing nullable child-slot boundary, continue when the
  two borrowed child handles are identical; otherwise call the unchanged
  recursive comparator.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

Experiment 275's rejected entry guard removed 829,834 dominant recursive
comparator calls. This candidate moves identity recognition to their callers,
so a hit avoids both call and return while top-level comparisons pay no new
branch.

## Results

### Deterministic profile

The candidate proves the expected unsatisfiable result and falls from
8,992,812,925 to 8,832,782,309 instructions, a reduction of 160,030,616 or
1.779539%. The hypothetical Rust/C ratio improves from 1.711495 to 1.681038.

General term-type comparisons fall from 1,138,621 to 267,827 calls, a
reduction of 870,794 or 76.477950%. The two visible recursive-comparator call
edges also fall substantially because identical child handles never enter the
callee. Proof-search results and dominant unrelated owners remain unchanged.

### Native timing

After four alternating warmup pairs, 64 alternating measured pairs decisively
reverse the deterministic result:

- wall mean and median regress 1.523529% and 0.809244%;
- CPU mean and median regress 1.031488% and 1.162791%;
- mean and median paired wall changes regress 1.576179% and 0.606236%;
- mean and median paired CPU changes regress 1.088591% and 1.162791%;
- the candidate wins only 24 wall and 16 CPU pairs, with ten CPU ties.

The stable tail is worse. The last 32 pairs regress 2.455409% wall and
1.348397% CPU by aggregate means; the last 16 regress 3.017158% wall and
1.519537% CPU. All 128 measured processes and eight warmup processes exit
zero. Direct parent and candidate output is byte-identical, including the
expected proof and SZS status.

The candidate executable is 8,936,448 bytes, 15,872 bytes smaller than the
8,952,320-byte parent.

## Validation

- All 46 candidate term-function tests pass in default and all-feature
  configurations.
- A focused regression compares distinct parent cells whose corresponding
  children are identical handles.
- Strict all-feature library pedantic Clippy passes.
- Exact WSL Callgrind and direct native runs prove LUSK6.
- After rejection, the child-edge guard and its regression are removed and
  accepted `termfunc.rs` is restored byte-for-byte.
- Compatibility matrices are skipped because the native production gate is a
  decisive and tail-strengthening rejection.

## Decision

Reject the child-edge identity guard. It is semantically exact and removes
1.779539% of instrumented work, but it slows production materially and more
strongly as the timing block stabilizes. Keep Experiment 270 as the accepted
executable baseline at 8,992,812,925 instructions, or 1.711495 times C.

Both broad identity placements are now exhausted. A distinct narrower
follow-up may check pointer identity only after the existing free-variable
branch is taken. That would target the 870,794 avoidable general type
comparisons without changing top-level or recursive child control flow.

## Reproduction

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-276-struct-weight-child-identity\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-003-struct-weight-child-identity\native-lusk.csv
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-003-struct-weight-child-identity/rust-callgrind-struct-weight-child-identity.out \
  target-wsl-276-struct-weight-child-identity/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
