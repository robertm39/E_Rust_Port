# Revisited compact PD-tree traversal frame

## Question

After the evaluated-clause allocator boundary was made fallible in Experiment
165, can the previously rejected two-state PD-tree cursor-frame compaction now
retain its deterministic speedup without reopening BOO020's allocator abort?

## Setup

- Parent source: commit `88ea0177` (`Record rejected PD-tree variable scan`),
  whose executable behavior is the accepted Experiment 168 source.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 12,557,467,650 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Native resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a
  2-GiB C data allowance.

The retained candidate profile is
`.artifacts/experiments/2026-07-21-170-compact-pdt-frame-revisit/rust-callgrind-compact-frame.out`.
Compatibility reports are retained under `.artifacts/e-compare/`.

## History and candidate

Experiment 157 established that `PdtTraversalFrame::next_step` only represents
the symbols step, the variables step, and the exhausted sentinel. Narrowing it
from `usize` to `u8` reduces a 64-bit frame from 48 to 40 bytes while leaving
node indices, binding positions, terminal positions, weights, and packed
variable-child links unchanged. That experiment preserved proofs and improved
LUSK6, but BOO020 advanced far enough to hit an infallible Windows allocation
before its CPU cutoff, so the source was restored.

Experiment 165 subsequently made the complete evaluated-clause admission
boundary reserve fallibly before mutation and normalize a rejected allocation
with an active deadline to C-compatible `ResourceOut`. This revisit applies
only the same field narrowing and adds a 64-bit layout regression. All 40
focused PD-tree tests pass.

## Performance result

The candidate preserves the exact 4,873-clause proof at 12,525,374,625
instructions, 32,093,025 below the parent (-0.2556%). The deterministic C/Rust
ratio improves from 2.3899 to 2.3838.

The reduction remains structurally attributable to frame movement. Exclusive
`search_next_matching_occurrence_impl` work falls from 1,582,253,858 to
1,556,297,359 instructions, saving 25,956,499, while
`pop_subst_cursor_frame` falls from 284,175,947 to 279,148,494, saving
5,027,453. These two deltas exactly reproduce the cursor-local savings measured
in Experiment 157 against a substantially faster parent.

## Compatibility and resource result

- Proof report `.artifacts/e-compare/20260721-085845-362679/` has zero
  mismatches across GEO288, HEN011, LUSK6, and LUSK6ext.
- Resource report `.artifacts/e-compare/20260721-085430-449664/` has zero
  mismatches across BOO020 and SWV851. In particular, BOO020 now preserves
  normalized `ResourceOut` rather than reproducing Experiment 157's allocator
  abort.
- Full report `.artifacts/e-compare/20260721-090037-389794/` has 50 cases,
  zero unexpected mismatches, and the one declared `sledgehammer.p`
  difference.

## Validation

- `cargo fmt --all -- --check`
- 4,380 library tests plus every integration target and feature
- strict all-target, all-feature pedantic Clippy
- all-feature release `eprover` build
- all four C-source documentation gates
- clean vendored C worktree

## Decision

Accept the compact traversal frame. The allocator behavior that rejected the
same local optimization in Experiment 157 is now explicitly controlled at the
admission boundary, the complete constrained-resource matrix is exact, and the
change removes 0.2556% of whole-prover instructions without altering cursor
state or order. Keep the main performance issue open: the remaining
deterministic C/Rust instruction ratio is 2.3838.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-compact-frame.out \
  target-wsl-170-compact-pdt-frame/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-170-compact-pdt-frame
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-170-compact-pdt-frame\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-170-compact-pdt-frame\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```
