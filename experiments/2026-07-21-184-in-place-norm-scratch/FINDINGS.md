# In-place substitution-normalization scratch

## Question

Can `Substitution::norm_term` operate directly on its reusable traversal
vector instead of moving the three-word `Vec` owner out of the substitution
and back on every call?

## Setup

- Parent source: commit `a6b52d7e` (`Return term-tree ordering directly`);
  commit `513d8cdf` adds only rejected-experiment evidence.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-182-term-tree-ordering/rust-callgrind-ordering.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-184-in-place-norm-scratch/rust-callgrind-in-place-scratch.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

Experiment 138 made the normalization stack reusable across calls, but
`norm_term` still used `mem::take` to replace the field with an empty vector,
walk through a local owner, and assign the empty allocation back afterward.
That shape predated the current field-scoped traversal: each stack push/pop is
complete before `add_binding` mutates the other substitution field.

The accepted path now checks the same empty-stack invariant and pushes, pops,
and restores arguments through `self.norm_stack` directly. Dereference mode,
right-to-left argument pushes, left-to-right variable binding order, reusable
capacity, fresh-variable allocation, marking, and backtracking are unchanged.
No new allocation, unsafe code, or shared borrow is introduced.

## Performance result

The candidate preserves the exact LUSK6 proof and retires 11,793,162,707
instructions. This is 5,228,544 below the 11,798,391,251-instruction parent, a
0.0443% whole-prover reduction. `Substitution::norm_term` falls from
342,923,509 to 337,700,237 exclusive instructions, saving 5,223,272 or
1.5232% and accounting for effectively the complete global change. The
deterministic C/Rust ratio improves from 2.2454 to 2.2445.

All other dominant cursor, comparator, dereference, rewrite, evaluation,
term-bank, and allocator entries remain unchanged in the compact exclusive
profile.

## Compatibility result

- Proof report `.artifacts/e-compare/20260721-155507-677743/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-155712-100075/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-160125-656560/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference.

## Validation

- All nine focused substitution tests pass.
- 4,384 library tests plus every integration target and feature pass.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept in-place reusable normalization scratch access. It removes redundant
per-call vector-owner moves, preserves the established C-shaped traversal and
binding order, localizes a small deterministic reduction to `norm_term`, and
passes the complete proof/resource matrix. Keep the main performance issue
open at 2.2445 times C. The separate higher-order `WHNF_deref` ownership gap
also remains unchanged.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-in-place-scratch.out \
  target-wsl-184-in-place-norm-scratch/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-184-in-place-norm-scratch
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-184-in-place-norm-scratch\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-184-in-place-norm-scratch\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
