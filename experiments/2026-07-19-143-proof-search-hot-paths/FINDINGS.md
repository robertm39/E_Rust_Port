# Proof-search hot-path ownership

## Question

How much of the remaining proof-search performance gap comes from Rust-owned
temporary terms and release-mode checks that have no equivalent in the
optimized C prover, and can those costs be removed without changing proof
search, output, or resource-boundary behavior?

## Setup

- Parent source: commit `e8996c11` (`Reduce proof-state memory at resource
  boundary`).
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Native proof corpus: `GEO288+1.p`, `HEN011-2.p`, `LUSK6.lop`, and
  `LUSK6ext.lop` with proof objects enabled.
- Native resource corpus: `BOO020-1.p` and `SWV851-1.p` at the maintained
  60-second CPU and 2 GiB memory limits.

All Callgrind candidates produced the same LUSK proof and processed exactly
4,873 clauses. Raw profiles are retained under
`.artifacts/experiments/2026-07-19-143-proof-performance/`.

## Comparative profile

The direct C profile retires 5,254,361,329 instructions. The parent Rust
profile retires 17,441,814,419 instructions, or 3.32 times C. The gap is
concentrated rather than spread across the prover:

- Rust new-clause insertion costs 10.48 billion instructions. Forward
  modification alone costs 8.22 billion, of which clause rewrite
  normalization costs 7.15 billion.
- Rust new-clause generation costs 5.95 billion instructions. Indexed
  paramodulant construction costs 4.83 billion, including 2.55 billion in
  `EqnList::copy_repl`.
- The corresponding C generation and insertion subtrees cost approximately
  1.40 and 3.51 billion instructions.

The C and Rust call counts agree closely. The excess is therefore per-node
ownership, allocation, and validation cost, not divergent proof search.

## Accepted changes

The rewrite traversal now defers construction of a replacement top cell and
argument array until the first child actually changes. It borrows source
arguments during recursion and bulk-fills a replacement only on the changed
path. Failed top-level rewrite scans return `None` internally instead of
cloning the unchanged reference-counted term; public functions retain their
original owned return type.

Term-bank insertion similarly borrows argument slices and fills temporary
terms through one mutable argument borrow. Dereferencing has an internal
changed-only result so the two hottest insertion paths do not acquire and
immediately release an `Rc` for unchanged roots.

Several checks were direct ports of C `assert()` preconditions but remained
active in optimized Rust builds. The variable-binding precondition in
`term_deref`, replacement-presence checks in `insert_repl`, and
ground/bound-term presence checks in `insert_instantiated_fo` are now
`debug_assert!` checks. Debug and test builds retain the diagnostics, while
release behavior now matches the `NDEBUG` C reference. Type and initialized
argument invariants that protect required execution remain unconditional.

No unsafe code or representation weakening is introduced.

## Deterministic results

Each row adds one accepted change to the preceding candidate:

| Candidate | Instructions | Change from preceding |
| --- | ---: | ---: |
| Parent | 17,441,814,419 | - |
| Deferred rewrite copy | 16,691,609,348 | -4.30% |
| Borrowed rewrite arguments | 16,679,335,256 | -0.07% |
| Changed-only rewrite result | 16,570,012,260 | -0.66% |
| Release/debug dereference assertion | 16,337,901,932 | -1.40% |
| Term-bank assertions and argument borrows | 16,006,871,502 | -2.03% |
| Changed-only dereference result | 15,996,207,368 | -0.07% |

The final candidate removes 1,445,607,051 instructions, or 8.29%, from the
parent and reduces the deterministic C/Rust ratio from 3.32 to 3.04. The
largest accepted individual change is avoiding unconditional temporary term
allocation during leftmost-innermost rewrite traversal.

## Compatibility and resource results

The final resource comparison is
`.artifacts/e-compare/20260720-010835-470314/`: BOO020 and SWV851 both have
zero mismatches. This preserves the prior slice's exact `ResourceOut`
classification and confirms that the faster traversal does not reopen the
Windows allocation failure.

The final four-case proof run is
`.artifacts/e-compare/20260720-010524-146055/`. GEO288, LUSK6, and LUSK6ext
are exact. HEN011 reaches `ResourceOut` after 61.41 seconds while C proves it
in 27.02 seconds. Earlier runs of the same search complete HEN in roughly
52--55 seconds, including the exact four-case report
`.artifacts/e-compare/20260720-001708-900885/` after the first accepted
rewrite change. HEN therefore remains a performance-margin failure, not an
output or search-order difference.

The intermediate 50-case report
`.artifacts/e-compare/20260720-002022-082507/` retains the expected
sledgehammer formatting difference and unexpected BOO, HEN, and synthetic
one-second LUSK boundary results. The final resource run closes the BOO
observation. The one-second LUSK and intermittent HEN gaps remain open; this
slice improves throughput but does not claim overall performance parity.

## Falsification checks

- Focused rewrite, term-bank, and dereference tests preserve rewrite links,
  normal forms, dereference-limit updates, and shared-term identity.
- A new regression verifies that changed-only dereferencing returns no owned
  handle for an unchanged term and returns the bound term when dereferencing
  changes the root.
- Every Callgrind ablation preserves the exact proof and 4,873 processed
  clauses.
- The final BOO/SWV comparison has zero mismatches under unchanged limits.
- The full all-target/all-feature test, clippy, formatting, release-build, and
  documentation gates are run before acceptance.
- The vendored C checkout remains unchanged.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --target-dir target\proof-hot-paths
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\proof `
  -RustExe .\target\proof-hot-paths\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
& .\e-interop.ps1 compare `
  -Corpus .\.artifacts\e-corpus\diversity-scratch-139\resource `
  -RustExe .\target\proof-hot-paths\release\eprover.exe `
  -TimeoutSeconds 60 -MemoryLimitMb 2048
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-proof-hot-paths.out \
  target/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

## Decision

Accept the deferred rewrite allocation, borrowed argument traversal,
changed-only internal results, and C-compatible release assertion behavior.
They remove 8.29% of deterministic proof-search instructions while preserving
the proof and resource semantics. Keep the main parity issue open for the
remaining 3.04-times instruction gap, the synthetic one-second LUSK cutoff,
and HEN011's intermittent CPU-limit margin.
