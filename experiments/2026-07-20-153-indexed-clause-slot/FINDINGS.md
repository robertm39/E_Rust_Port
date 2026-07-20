# Direct sparse-clause occupied-slot iteration

## Question

Can the 255.8-million-instruction sparse-clause occupied-slot scan be reduced
without adding a per-clause index or disturbing stable slots, clause order, or
the maintained resource boundary?

## Setup

- Parent source: commit `2b349117` (`Document rejected evaluation comparator
  rewrites`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 14,023,295,072 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Native proof corpus: retained GEO/HEN/LUSK four-case corpus with proof
  objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at 60 process-CPU
  seconds and a 2-GiB C data allowance.

Profiles are retained at
`.artifacts/experiments/2026-07-20-153-indexed-clause-slot/`. Compatibility
reports are retained under `.artifacts/e-compare/`.

## Rejected indexed lookup

The first candidate used the existing position and derivation maps as fast
paths for `slot_by_id` and `slot_by_derivation_ref`, validating the mapped slot
and retaining the scan as a fallback for transiently stale indices. It passed
the focused clause-set tests but regressed the exact proof to 14,102,727,892
instructions, 79,432,820 above the parent (+0.5665%). The profiled occupied
scan was essentially unchanged at 255,786,185 instructions. This showed that
the hot instantiation belonged to plain, non-indexed clause sets rather than
the sets covered by those maps. The source and its test adjustment were
reverted. The rejected profile is `rust-callgrind-indexed-slot.out`.

## Direct iterator

The retained change replaces the nested `once`/`chain`/`enumerate`/`flat_map`/
`filter_map`/`skip_while` adapter type with a private `SparseOccupiedSlots`
iterator. Its state is only a store reference, chunk and offset cursors, and
the known number of live clauses. It starts directly at `first_occupied`,
walks chunk storage in slot order, and stops when that live count reaches zero
instead of scanning trailing holes.

The iterator implements `ExactSizeIterator`, preserving an exact collection
reservation without adding any persistent clause-set memory. A direct test
pins leading, interior, and trailing-hole handling, slot order, the decreasing
remaining length, first-slot removal, and the empty-store result. Existing
multi-page and in-place-compaction tests continue to cover chunk boundaries.

## Performance result

The exact-proof profile falls to 13,863,033,680 instructions, 160,261,392
below the parent (-1.1428%). The deterministic C/Rust ratio improves from
about 2.669 to 2.638. The old adapter instantiation was a 255,801,862-
instruction exclusive function; in the retained binary the stateful
iterator's non-inlined `next` body accounts for only 44 instructions and the
small loop is folded into its callers. The retained profile is
`rust-callgrind-direct-occupied.out`.

## Compatibility result

The final four-case proof report at
`.artifacts/e-compare/20260720-192820-534593/` has zero mismatches across
GEO288, HEN011, LUSK6, and LUSK6ext. The final resource report at
`.artifacts/e-compare/20260720-193025-763630/` has zero mismatches for BOO020
and SWV851. The latter is the important falsification check for an optimization
in the sparse clause store: both cases preserve the upstream resource outcome
at the maintained memory boundary, and the retained iterator adds no storage.

The complete Rust suite passes 4,371 library tests plus every integration
target. Strict all-target/all-feature pedantic Clippy, formatting, and the
all-feature release build pass.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-direct-occupied.out \
  target-wsl-153-direct-occupied/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-153-direct-occupied
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-153-direct-occupied\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-153-direct-occupied\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Accept the direct iterator. It removes generic iterator machinery from a
confirmed hot scan, improves the exact deterministic workload by more than one
percent, preserves stable slot order and exact-size collection behavior, and
passes the proof and tight-memory resource gates. Keep the main parity issue
open: the synthetic one-second LUSK cutoff and the remaining 2.638
deterministic C/Rust instruction ratio still fail the project-wide acceptance
criteria.
