# Direct evaluation-index link sentinel

## Question

Can the private evaluation-index arena replace `Option<NonZeroUsize>` child
links with a direct `usize::MAX` null sentinel, preserving the compact 48-byte
node while removing `+1/-1` link encoding from the hot splay loop?

## Setup

- Parent source: commit `07254517` (`Record rejected three-link dereference
  window`), whose executable source is the accepted Experiment 177 baseline.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 11,993,700,044 instructions with the exact proof and 4,873
  processed clauses.
- Candidate profile:
  `.artifacts/experiments/2026-07-21-179-eval-link-sentinel/rust-callgrind-eval-link-sentinel.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.

## Candidate

Experiment 151 replaced the generic evaluation `BTreeSet` with C's top-down
splay shape and a compact indexed arena. Its child links encoded arena index
`n` as `NonZeroUsize(n + 1)` so `Option` could use zero as its niche. Every
hot link read therefore mapped the option and subtracted one, and every write
performed the inverse encoding.

The arena now reserves `usize::MAX` as its private null link. Live links store
their zero-based `Vec` index directly; debug assertions reject the sentinel as
a live link. A `Vec` cannot allocate a slot at that index, so the sentinel does
not narrow any realizable arena. `EvalIndexNode` remains exactly one 32-byte
entry plus two pointer-width links, or 48 bytes on the maintained 64-bit
target. Existing regressions cover sorted order, best lookup, duplicate
suppression, removal, freed-slot reuse including index zero, logical equality,
clearing, and the node-size invariant.

## Performance result

The candidate preserves the exact 4,873-clause proof at 11,963,095,182
instructions, 30,604,862 below the parent (-0.2552%). The deterministic C/Rust
ratio improves from 2.2826 to 2.2768.

`EvalIndexTree::splay` falls from 368,087,119 to 339,117,969 exclusive
instructions, a reduction of 28,969,150 (-7.87%) that accounts for nearly the
entire program improvement. Libc `malloc` is unchanged at 290,004,395
instructions, and `_int_free` changes by only 17,112 instructions, confirming
that the gain comes from direct safe link access rather than a changed
allocation or proof-search shape.

## Compatibility and resource result

- Proof report `.artifacts/e-compare/20260721-133537-101934/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-133748-122459/` has zero
  mismatches across BOO020 and SWV851; both preserve normalized `ResourceOut`.
- Full report `.artifacts/e-compare/20260721-134204-125488/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference. BOO020, SWB008, SWV851, HEN011, and the synthetic one-second
  LUSK case all retain the C outcome.

## Validation

- `cargo fmt --all -- --check`
- 4,383 library tests plus every integration target and feature
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four C-source documentation gates
- clean vendored C worktree

## Decision

Accept direct sentinel evaluation-tree links. They keep the resource-critical
node size unchanged, preserve the same safe indexed ownership model and splay
order, and remove 7.87% of the hot splay's exclusive instructions. Keep the
main performance issue open: the remaining deterministic C/Rust instruction
ratio is 2.2768.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-eval-link-sentinel.out \
  target-wsl-179-eval-link-sentinel/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-179-eval-link-sentinel
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-179-eval-link-sentinel\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-179-eval-link-sentinel\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -RustExe .\target\native-179-eval-link-sentinel\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
