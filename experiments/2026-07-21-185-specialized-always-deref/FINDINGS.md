# Specialized always-dereference normalization

## Question

Can `Substitution::norm_term`, whose dereference mode is statically always,
avoid constructing and routing a mutable `DerefType` through the general
mode dispatcher on every visited term?

## Setup

- Parent source: commit `2a966bc7` (`Use substitution normalization scratch
  in place`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-184-in-place-norm-scratch/rust-callgrind-in-place-scratch.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-185-specialized-always-deref/rust-callgrind-specialized-always.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

The retained implementation adds a crate-private `term_deref_always` entry
point and factors the existing always-mode loop into one shared private core.
The public mutable-mode API still dispatches to that core for
`DerefType::Always`, so all existing callers retain their exact contract.
`Substitution::norm_term` calls the specialized entry directly because its
mode never changes.

The two-link borrowed binding walk, applied-variable expansion, clone-on-no-
change behavior, traversal order, binding order, and general once/never
semantics are unchanged. No unsafe code, allocation, or new term state is
introduced. The focused long-chain test now checks the specialized and
general always-mode entries against the same terminal term.

## Performance result

The candidate preserves the exact LUSK6 proof and retires 11,736,468,593
instructions. This is 56,694,114 below the 11,793,162,707-instruction parent,
a 0.4807% whole-prover reduction. `Substitution::norm_term` falls from
337,700,237 to 302,975,103 exclusive instructions, saving 34,725,134 or
10.2828%. The deterministic C/Rust ratio improves from 2.2445 to 2.2337.

The dominant PD-tree cursor and `deref_always_step` reproduce exactly at
1,484,913,131 and 328,211,680 exclusive instructions. The candidate's split
dereference entry points make individual dispatcher symbols less directly
comparable, but the reduction lands in the callers that previously carried
the constant mutable-mode path: `subst_compute_mgu` falls by 11,475,901 and
`mfy_vwb` by 11,850,699 instructions in addition to the normalization saving.

## Compatibility result

- Proof report `.artifacts/e-compare/20260721-163240-862459/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-163431-929323/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-163842-807487/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference.

## Validation

- All nine focused substitution tests and all 18 focused term-type tests pass.
- 4,384 library tests plus every integration target and feature pass.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept the specialized always-dereference entry. It preserves the shared
general implementation and exact C-facing behavior while removing constant
mutable-mode traffic from a hot, statically known caller. Keep the main
performance issue open at 2.2337 times C; the remaining gap is still too large
to claim performance parity.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-specialized-always.out \
  target-wsl-185-specialized-always-deref/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-185-specialized-always-deref
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-185-specialized-always-deref\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-185-specialized-always-deref\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
