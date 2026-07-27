# Direct non-variable always dereferencing

## Question

Can the always-mode dereference loop handle its non-variable branch directly,
instead of calling the general dereference step and repeating the free-variable
test on every applied-variable candidate?

## Setup

- Parent source: commit `1119c317` (`Use direct links across evaluation tree`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-189-direct-eval-tree-boundary/rust-callgrind-direct-tree-boundary.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-190-direct-always-nonvar/rust-callgrind-direct-nonvar.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`deref_always_step` now handles the non-free-variable case directly. It tests
for an applied free variable whose head is bound and expands that application;
all other non-free variables terminate the loop. The free-variable binding
path is unchanged, as are the general `deref_step` callers.

This is the same branch behavior previously delegated to `deref_step`, without
its redundant `is_free_var` test and call boundary. It introduces no unsafe
code, allocation changes, cache changes, or semantic shortcuts.

## Performance result

The candidate preserves the exact LUSK6 proof and retires 11,588,500,898
instructions. This is 9,497,694 below the 11,597,998,592-instruction parent, a
0.081891% whole-prover reduction. The deterministic C/Rust ratio improves from
2.2073 to 2.205501.

The symbol-level attribution is layout-sensitive. In the parent, `norm_term`,
`deref_always_step`, and `deref_step` total 854,791,165 exclusive
instructions. In the candidate, `deref_step` is folded into the specialized
path and the comparable visible `norm_term` plus `deref_always_step` aggregate
is 860,451,604, an increase of 5,660,439. The whole binary nevertheless falls
by 9,497,694 instructions. Therefore the retained result is a reproducible
whole-program win for the pinned build, not a claim that the visible local
symbol aggregate explains the saving.

## Compatibility result

- Proof report `.artifacts/e-compare/20260721-191010-248606/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-191201-951728/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-191612-904933/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference.

## Validation

- The focused term dereference regressions pass.
- The complete serial suite passes: 4,384 library tests plus every integration
  target and feature. An initial parallel run passed 4,383 tests but its
  scheduler-sensitive one-second CPU-limit fixture completed before its limit;
  that exact test passed in isolation before the clean full serial rerun.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept the direct non-variable branch. It preserves the general dereference
behavior and the complete compatibility matrix while producing a small,
repeatable whole-program improvement. Keep the main issue open at 2.205501
times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-nonvar.out \
  target-wsl-190-direct-always-nonvar/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-190-direct-always-nonvar
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-190-direct-always-nonvar\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-190-direct-always-nonvar\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
