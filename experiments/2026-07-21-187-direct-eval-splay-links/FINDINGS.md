# Direct evaluation-splay links

## Question

After storing evaluation-index child links as direct `usize` values with a
`usize::MAX` null sentinel, can the hot splay loop also keep its temporary
roots and tails in that representation instead of repeatedly constructing
two-word `Option<usize>` values?

## Setup

- Parent source: commit `95d35534` (`Record rejected PD-tree constraint
  snapshot`), whose executable source is accepted Experiment 185.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-21-185-specialized-always-deref/rust-callgrind-specialized-always.out`.
- Final candidate profile:
  `.artifacts/experiments/2026-07-21-187-direct-eval-splay-links/rust-callgrind-direct-splay-links-clippy.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Implementation

`EvalIndexNode` already reserves `NO_EVAL_INDEX_NODE` as its private null link.
The retained splay now uses that representation end to end: child reads,
rotations, temporary lower/upper roots and tails, reassembly, and node writes
all carry direct arena indices or the sentinel. A sentinel is always checked
before indexing the vector.

Comparator semantics, duplicate handling, root ownership, free-slot reuse,
allocation, and the top-down splay order are unchanged. Nodes remain exactly
one 32-byte evaluation entry plus two pointer-width links, or 48 bytes on the
maintained 64-bit target. The focused regression covers sorted order, best
lookup, removal, duplicate rejection, index-zero slot reuse, clearing, logical
equality, and the node-size invariant.

## Performance result

The final candidate preserves the exact LUSK6 proof and retires
11,704,174,869 instructions. This is 32,293,724 below the
11,736,468,593-instruction parent, a 0.2752% whole-prover reduction.
`EvalIndexTree::splay` falls from 339,117,969 to 306,825,308 exclusive
instructions, saving 32,292,661 or 9.5225% and accounting for effectively the
complete program change. The deterministic C/Rust ratio improves from 2.2337
to 2.2275.

Every other dominant compact-profile entry reproduces exactly apart from
negligible instruction-level process noise. The PD-tree cursor remains
1,484,913,131 instructions, and libc `malloc` remains 290,004,305.

## Compatibility result

- Final-source proof report `.artifacts/e-compare/20260721-175014-763684/`
  has zero mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Final-source resource report `.artifacts/e-compare/20260721-175203-071227/`
  has zero mismatches across BOO020 and SWV851; both preserve normalized
  `ResourceOut`.
- Final-source full report `.artifacts/e-compare/20260721-175614-397171/` has
  50 cases, zero unexpected mismatches, and the one declared
  `sledgehammer.p` difference.

## Validation

- The focused evaluation-index regression passes.
- 4,384 library tests plus every integration target and feature pass.
- Strict all-target, all-feature pedantic Clippy passes.
- Formatting and the all-feature release `eprover` build pass.
- All four C-source documentation gates pass.
- The vendored C worktree is clean.

## Decision

Accept direct sentinel use throughout the evaluation splay. It keeps the
resource-critical arena and comparator unchanged, makes the safe indexed loop
closer to C pointer-link operations, removes 9.52% of its exclusive work, and
passes the complete compatibility matrix. Keep the main performance issue
open at 2.2275 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-splay-links-clippy.out \
  target-wsl-187-direct-eval-splay-links/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-187-direct-eval-splay-links
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-187-direct-eval-splay-links\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-187-direct-eval-splay-links\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
