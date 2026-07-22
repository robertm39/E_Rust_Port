# Fuse term-bank dereference-root prefix-limit check

## Question

Can the production term-bank dereference root compute the higher-order applied
binding prefix limit without testing the same bound-applied-variable condition
twice on every call?

## Setup

- Parent source: commit `c1acb78d` (`Record rejected PD-tree cursor clear
  removal`), whose executable source is accepted Experiment 205.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-205-remove-pdt-init-cursor-reset/rust-callgrind-remove-pdt-init-cursor-reset.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-207-fuse-deref-root-limit-check/rust-callgrind-fuse-deref-root-limit-check.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`deref_root_no_whnf_if_changed` previously called `deref_limit`, which tested
whether `DerefType::Once` was acting on a bound applied free variable, then
immediately repeated the same condition to decide whether to expand that
variable. The production profile records 3,355,147 calls.

The fused path tests the condition once. On success it loads the already-known
binding, derives the identical ignored-argument prefix, and performs the same
one-step applied-variable expansion. All other cases call
`term_deref_if_changed` and return the same zero prefix limit. The general
`deref_limit` helper remains unchanged for debug-term printing, which has
different traversal needs.

No ownership, binding, dereference-mode, argument-prefix, or insertion behavior
changes. The 122 focused term-bank tests include recursive, cached,
optimized, replacement, and instantiated applied-variable cases, including
ignored bound-prefix arguments.

## Performance result

The candidate preserves the expected LUSK6 proof and retires 10,805,295,203
instructions. This is 69,903,125 below the 10,875,198,328-instruction parent,
a 0.642776% whole-prover reduction. The deterministic C/Rust ratio improves
from 2.069747 to 2.056443.

The comparable dereference-root inclusive aggregate falls from 267,900,072 to
197,441,979 instructions, saving 70,458,093 or 26.300140%. This differs from
the whole-program saving by 554,968 instructions. The 1,697,827,541-
instruction PD-tree cursor, 670,177,484-instruction term-tree insertion,
437,245,456-instruction substitution normalization, and allocator hotspots
reproduce exactly, localizing the change.

Native timing required more samples because the first block contained one
2.475-second parent outlier and conflicting mean/median signals. Across the
final 48 warmed alternating pairs, candidate mean is 1.951021 seconds versus
1.962832, a 0.601752% improvement. Candidate median is 1.933949 versus
1.941877, a 0.408247% improvement. Mean paired improvement is 0.371547%,
paired median improvement is 0.555681%, and the candidate wins 28 pairs to 20.
A symmetric 10% trimmed mean still improves 0.152725%. All 96 runs prove with
exit zero. The executable grows by 8,704 bytes, from 8,632,320 to 8,641,024.

## Compatibility evidence

- Proof report `.artifacts/e-compare/20260722-033127-064510/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext at the standard
  60-second limit.
- Resource report `.artifacts/e-compare/20260722-033344-099527/` has zero
  mismatches for BOO020 and SWV851 at the 60-second, 2-GiB boundary.
- The recent clean loaded report
  `.artifacts/e-compare/20260721-234057-582244/` has 50 cases, zero unexpected
  mismatches, and the one declared sledgehammer difference. Subsequent
  accepted changes have exact focused proof and resource reports.

## Validation

- All 122 focused term-bank tests pass.
- The complete serial suite passes 4,384 library tests plus every integration
  target and feature.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept the fused dereference-root check. It removes duplicated work from a
3.36-million-call path, deterministic and robust native measurements improve,
and focused proof/resource compatibility is exact. Keep the main parity issue
open at 2.056443 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-fuse-deref-root-limit-check.out \
  target-wsl-207-fuse-deref-root-limit-check/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
