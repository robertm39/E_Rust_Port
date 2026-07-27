# Clause-page growth at the resource boundary

## Question

Can the remaining one-second proof-search gap be reduced without reopening the
maintained 2 GiB resource failures, and why did `BOO020-1.p` sometimes abort in
the allocator even on the last accepted source?

## Setup

- Parent source: commit `9a23c6ac` (`Reduce proof-search term ownership
  overhead`).
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Native proof corpus: `GEO288+1.p`, `HEN011-2.p`, `LUSK6.lop`, and
  `LUSK6ext.lop` with proof objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at the maintained
  60-second CPU and 2 GiB memory limits.

Raw candidate profiles are retained under
`.artifacts/experiments/2026-07-19-144-ac-parent-batch-cache/`. Windows
boundary repetitions are retained in this directory as CSV files.

## Rejected throughput candidates

Resolving AC-axiom derivation parents once per generated-clause batch reduced
the LUSK profile from 15,996,207,368 to 15,918,182,572 instructions (0.49%).
The snapshot held derivation references across the complete batch and did not
survive the BOO resource falsification, so it was reverted.

A bounded term-replacement memo table reduced the profile to
15,664,432,484 instructions (2.07%). Even 32 retained entries repeatedly
ended in BOO allocator aborts, including with the original PD-tree prewalk.
The cache and its tests were removed completely.

Fat LTO plus one codegen unit reduced the cached candidate to
14,992,138,949 instructions, and deferring the Rust-only PD-tree path prewalk
reduced it further to 14,282,665,670. Thin LTO, codegen-unit variants, and a
direct-mapped replacement cache were also tested. These combinations did not
pass the resource boundary, and the cache confounded the PD-tree experiment,
so no build-profile or PD-tree change is accepted from those measurements.

Other isolated candidates either regressed or were neutral: lazy replacement
insertion (16,082,794,803), axiom-first proof-set lookup
(16,069,875,620), ground-only replacement caching (15,668,294,073),
changed-only substitution dereferencing (15,688,912,464), and ground-subtree
skipping (15,718,730,853). All were reverted.

## Resource-boundary diagnosis

Two repeats of the exact parent source failed BOO with 278,528- and
557,056-byte allocation requests. Those sizes are respectively 2,048 and
4,096 inline `Clause` headers at the current 136-byte layout. A lazily grown
4,096-slot overflow page therefore had to allocate its full buffer while its
half-full buffer was still live. Near the Job Object ceiling, the temporary
50% page-growth spike could lose a race with the CPU deadline even though the
steady-state page layout was already bounded.

`SparseClauseStore` now allocates each overflow page at its complete fixed
capacity before inserting its first clause. The first inline page and logical
page size are unchanged. This moves the same final allocation earlier, when
the proof state is smaller, and eliminates every half-to-full reallocation and
its simultaneous old/new buffers. Encoded slots, clause order, compaction,
and steady-state full-page memory are unchanged.

## Results

The accepted candidate returned normal `ResourceOut`/8 in all six direct
boundary repetitions:

| Fixture | Outcomes | Sampled peak range |
| --- | --- | ---: |
| BOO020 | 3/3 `ResourceOut`/8 | 2,105,744--2,277,716 KiB |
| SWV851 | 3/3 `ResourceOut`/8 | 2,144,952--2,145,760 KiB |

The exact two-case C/Rust resource report is
`.artifacts/e-compare/20260720-045153-775892/` and has zero mismatches. The
four-case proof report is `.artifacts/e-compare/20260720-045609-544176/` and
also has zero mismatches, including HEN011.

The final 50-case report is
`.artifacts/e-compare/20260720-050303-046516/`. Its only unexpected mismatch
is the already-open synthetic one-second LUSK cutoff. `sledgehammer.p` retains
its declared normalized-output difference; BOO, SWV, HEN, and GEO all match.

The final deterministic profile retires 15,985,039,196 instructions. This is
11,168,172 instructions, or 0.07%, below the parent. Avoiding page
reallocations therefore fixes the resource race without a throughput tradeoff.
The remaining C/Rust instruction ratio is approximately 3.04, so the overall
performance-parity issue stays open.

## Falsification checks

- A focused sparse-store regression pins full overflow-page capacity, stable
  encoded slots, bounded capacity, and cross-page ordering.
- Three direct runs each cover BOO and SWV under the unchanged limits.
- Exact resource and proof corpora cover normalized output and proof identity.
- The complete 50-case matrix adds no undeclared mismatch beyond the existing
  one-second synthetic LUSK cutoff.
- Callgrind retains the exact LUSK proof and 4,873 processed clauses.
- Full all-target/all-feature tests, strict pedantic clippy, formatting,
  release build, documentation checks, and the unchanged vendored C checkout
  are required before acceptance.

## Reproduction

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-144-prealloc-pages
& .\experiments\2026-07-19-134-compact-clause-owners\measure_windows.ps1 `
  -Binary .\target\native-144-prealloc-pages\release\eprover.exe `
  -Problem .\eprover\EXAMPLE_PROBLEMS\SMOKETEST\BOO020-1.p `
  -OutputCsv .\experiments\2026-07-19-144-clause-page-growth\preallocated-overflow-boo.csv `
  -Label preallocated-overflow-boo -Runs 3 -CpuLimit 60
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\native-144-prealloc-pages\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-preallocated-pages.out \
  target-wsl-144-pages/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

## Decision

Accept full-capacity allocation for fixed-size overflow clause pages. It
removes a transient allocation spike, makes both maintained resource fixtures
stable across repeated runs, and slightly improves deterministic throughput.
Reject all other experiment candidates. Keep the main parity issue open for
the approximately 3.04-times instruction gap and the one-second synthetic
LUSK cutoff.
