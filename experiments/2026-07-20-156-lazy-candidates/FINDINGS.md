# Lazy non-binding PD-tree candidates

## Question

Can the legacy non-binding demodulator-candidate API reuse the lazy PD-tree
substitution traversal without changing proof order, higher-order matching, or
the maintained 2-GiB resource boundary?

## Setup

- Parent source: commit `0af513b0` (`Specialize PD-tree tokens for first-order
  search`).
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile: 13,328,560,605 instructions with the exact proof and 4,873
  processed clauses.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at 5,254,361,329
  instructions.
- Proof corpus: GEO288, HEN011, LUSK6, and LUSK6ext with proof objects.
- Resource corpus: BOO020 and SWV851 at 60 process-CPU seconds and a 2-GiB C
  data allowance.

Profiles are retained under
`.artifacts/experiments/2026-07-20-156-lazy-candidates/`. Compatibility reports
are retained under `.artifacts/e-compare/`.

## Attribution

The parent profile called `ensure_search_query` 95,002 times from
`search_matching_occurrences`, retiring 152,457,535 instructions in query
flattening alone. Production unit simplification and rewriting used
`search_next_matching_occurrence`, which eagerly collected every matching
occurrence into a temporary vector before yielding the first candidate. The
binding-preserving API already had a lazy, C-shaped traversal cursor.

## Candidate sequence

The first adapter gave the non-binding API a private reusable `Substitution`,
called the binding-preserving cursor, and immediately backtracked before
returning each occurrence. Higher-order and uninitialized modes retained the
materialized collector because the substitution cursor is first-order. This
version produced the exact LUSK proof at 13,241,442,997 instructions, but the
resource report at `.artifacts/e-compare/20260720-220425-815998/` exposed a
BOO020 allocator abort at the 2-GiB boundary. A fixed 5,000-selection BOO020
run had exactly the same inference, rewrite, and termbank counts as the parent,
isolating storage rather than search semantics.

The second adapter made actual binding reconstruction optional inside the
existing cursor. Non-binding callers now reuse speculative cursor metadata
without allocating or cloning a redundant substitution stack; the public
binding-preserving API still reconstructs every live binding. That intermediate
version reached 13,130,960,445 instructions and restored the BOO020 boundary.
It initially ignored pre-existing bindings on tree-owned indexed variables,
however, causing HEN011 to reach its cutoff. The final implementation preserves
those live bindings while avoiding new candidate-only storage. A direct
first-order regression now pins both the binding-free result and the
pre-existing-binding case. Higher-order searches continue to use the unchanged
materialized collector.

## Accepted result

The final exact-proof profile retires 13,122,494,580 instructions, 206,066,025
below the parent (-1.5460%). The deterministic C/Rust ratio improves from
2.537 to 2.497. `ensure_search_query` disappears from the production hot list;
the shared lazy cursor accounts for 1,602,754,924 exclusive instructions while
serving both binding-preserving and non-binding callers.

The final proof report at `.artifacts/e-compare/20260720-224936-516877/` has
zero mismatches across all four proof cases. The final resource report at
`.artifacts/e-compare/20260720-225439-325226/` has zero mismatches across
BOO020 and SWV851. The complete Rust suite passes 4,373 library tests plus all
integration targets. Strict all-target/all-feature pedantic Clippy, formatting,
the all-feature release build, C-source documentation checks, and the vendored
C-tree cleanliness check all pass.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-final.out \
  target-wsl-156-lazy-candidates/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-156-lazy-candidates
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\native-156-lazy-candidates\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-156-lazy-candidates\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

## Decision

Accept the first-order lazy non-binding adapter, including the live indexed-
binding check, and retain the materialized compatibility collector for every
other problem mode. Keep the main parity issue open: the synthetic one-second
LUSK cutoff and the remaining 2.497 deterministic C/Rust instruction ratio
still fail the project-wide acceptance criteria.
