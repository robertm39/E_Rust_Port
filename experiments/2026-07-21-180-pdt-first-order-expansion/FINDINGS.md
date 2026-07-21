# Direct first-order PD-tree symbol expansion

## Question

After a first-order PD-tree symbol edge has already accepted a nonnegative
function code, can Rust expand that term's arguments directly instead of
retesting the higher-order and variable forms that the successful dispatch has
already excluded?

## Setup

- Parent source: commit `f9ab7eed` (`Use direct evaluation-index link
  sentinels`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-179-eval-link-sentinel/rust-callgrind-eval-link-sentinel.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-180-pdt-first-order-expansion/rust-callgrind-fo-expansion.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Structural attribution

The accepted first-order symbol dispatch from experiment 168 rejects negative
function codes and probes the function `IntMap` directly. Once that lookup
succeeds, the query term cannot be a top-level free variable, lambda, applied
DB variable, or another higher-order special form. The shared expansion helper
nevertheless repeated those classifications before choosing argument zero as
the first visible child.

C `pdtree_forward` and `TermLRTraverseNext` operate on the already selected
ordinary-function branch and push its argument-array entries directly. Rust
now follows the same boundary with a first-order-only expansion helper. It
borrows the argument slice once, pushes owned handles in reverse order so the
left child remains next, and records the unchanged arity for backtracking.
Higher-order and uninitialized modes retain the complete existing classifier.
A focused regression pins left-to-right stack order, the retained root, and
the expansion count.

## Performance result

The candidate preserves the exact LUSK6 proof and retires 11,836,080,718
instructions. This is 127,014,464 below the 11,963,095,182-instruction parent,
a 1.0617% whole-prover reduction. The deterministic C/Rust ratio improves from
2.2768 to 2.2526.

The reduction is concentrated on the intended boundary:

| Metric | Parent | Candidate | Change |
| --- | ---: | ---: | ---: |
| Whole prover | 11,963,095,182 | 11,836,080,718 | -127,014,464 (-1.0617%) |
| `search_next_matching_occurrence_impl` exclusive | 1,556,297,359 | 1,484,913,131 | -71,384,228 (-4.5868%) |
| `Term::is_applied_free_var` exclusive | 134,063,204 | 106,212,392 | -27,850,812 (-20.7744%) |
| C/Rust ratio | 2.2768 | 2.2526 | -0.0242 |

The remaining difference includes the eliminated lambda/DB-variable property
tests and compiler layout around the specialized path. Allocator, rewrite,
term-comparison, evaluation-index, and frame-pop counts remain unchanged in
the compact exclusive profile.

## Compatibility result

- Proof report `.artifacts/e-compare/20260721-141814-046865/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-142011-035881/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-142425-744017/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference. The higher-order cases and synthetic one-second LUSK case retain
  their accepted behavior.

## Validation

- 41 focused PD-tree tests pass.
- 4,384 library tests plus every integration target and feature pass.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept direct first-order symbol expansion. It removes classification work
that the preceding symbol branch has already resolved, preserves the complete
higher-order traversal path, cuts the dominant cursor by 4.59%, and passes the
complete compatibility and resource matrix. Keep the main performance issue
open: the remaining deterministic ratio is 2.2526 times C rather than
comparable performance.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-fo-expansion.out \
  target-wsl-180-fo-symbol-expansion/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-180-fo-symbol-expansion
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-180-fo-symbol-expansion\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-180-fo-symbol-expansion\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
