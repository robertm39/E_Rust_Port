# Direct evaluation-tree operation boundary

## Question

After Experiment 187 made the evaluation splay use direct sentinel links, can
the surrounding first/insert/remove operations use the arena representation
directly instead of converting every child read and write through
`Option<usize>`?

## Setup

- Parent source: commit `641eb8e6` (`Record rejected evaluation splay
  ordering`), whose executable source is accepted Experiment 187.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-187-direct-eval-splay-links/rust-callgrind-direct-splay-links-clippy.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-189-direct-eval-tree-boundary/rust-callgrind-direct-tree-boundary.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

The retained evaluation tree now carries `NO_EVAL_INDEX_NODE` directly while
walking to the first entry, attaching the old root during insertion, detaching
its former child, selecting the removal replacement, and joining the removed
root's subtrees. The remaining optional root converts the sentinel only once
when a removal leaves an empty or right-only tree. The option-writing node
helpers became unused and were removed.

All indices are checked against the sentinel before vector access. Comparator
semantics, top-down splay order, allocation, duplicate handling, free-slot
reuse, logical iteration, and the 48-byte node layout are unchanged. No unsafe
code or pointer arithmetic is introduced.

## Performance result

The candidate preserves the exact LUSK6 proof and retires 11,597,998,592
instructions. This is 106,176,277 below the 11,704,174,869-instruction parent,
a 0.9072% whole-prover reduction. The deterministic C/Rust ratio improves
from 2.2275 to 2.2073.

The direct evaluation boundary has a small attributable local gain:
`index_clause_evaluations` falls from 81,266,079 to 80,191,181 exclusive
instructions, saving 1,074,898 or 1.3227%. `extract_at_slot` and the already
direct splay reproduce exactly at 71,778,854 and 306,825,308 instructions.

Most of the whole-program reduction is a compiler-layout effect that must be
reported separately from the local source change. The candidate inlines
`term_deref_always` into substitution normalization, so its standalone
141,565,741-instruction symbol disappears. The comparable aggregate of
`norm_term`, `term_deref_always`, and `deref_always_step` falls from
772,752,524 to 653,052,545 instructions, a 119,699,979 or 15.4901% reduction.
Small cursor, comparator, and rewrite increases offset part of that gain.
Thus the exact measured improvement is real for the pinned toolchain and
binary, but larger and more compiler-layout-sensitive than the direct
evaluation-tree saving alone.

## Compatibility result

- Proof report `.artifacts/e-compare/20260721-183345-986284/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-183540-487289/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-183954-380166/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference.

## Validation

- The focused evaluation-index regression passes.
- 4,384 library tests plus every integration target and feature pass.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept direct sentinel links across the remaining evaluation-tree operation
boundary. The local representation is simpler and faster, preserves the
resource-critical node layout and complete compatibility matrix, and produces
a reproducible whole-program reduction with the pinned build. Track the
compiler-layout attribution explicitly rather than claiming the complete gain
comes from evaluation indexing. Keep the main issue open at 2.2073 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-tree-boundary.out \
  target-wsl-189-direct-eval-tree-boundary/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-189-direct-eval-tree-boundary
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-189-direct-eval-tree-boundary\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-189-direct-eval-tree-boundary\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
