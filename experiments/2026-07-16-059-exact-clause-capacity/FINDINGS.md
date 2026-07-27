# Exact clause batch capacity

## Question

Can the formula-to-proof-state path avoid retaining Rust `Vec` geometric-growth
slack when the source clause count is already known, without changing sparse
clause handles, canonical clause order, or C-visible proof behavior?

The 20,000-owner Massif profile retained two 32,768-slot clause stores and one
32,768-element canonicalization scratch vector for only 20,001 live clauses.
Each allocation was 6,291,456 bytes because `Clause` is 192 bytes on the
profiled 64-bit build.

## Candidate

`ClauseSet::reserve_exact` forwards a known additional batch size to the owned
sparse store. Two count-known callers use it:

- `clause_set_archive_copy` reserves the source set's member count before
  copying into the archive;
- `ProofStateInit` reserves the evaluation-order handle count before copying
  axioms into `unprocessed`.

Sparse-store canonicalization still moves the same clauses into a dense vector
and invokes the same unstable comparator. It now allocates that vector with the
live clause count before extending it, instead of relying on `Flatten`'s
zero-lower-bound iterator and geometric growth. No clause ordering, identity,
indexing, or removal rule changed.

A regression reserves 257 clauses, fills the batch, and verifies that insertion
does not grow the sparse owner.

## Exact baseline construction

The current Windows and WSL release executables were copied to the ignored
experiment artifact tree before the source edit. Candidate binaries were built
from the edited tree with the same Windows and WSL Rust toolchains and copied
separately. The upstream C checkout was not modified.

Paired Massif commands used the preserved binaries and the existing unique-atom
20,000-owner corpus:

```powershell
wsl.exe -d Ubuntu-24.04 -- valgrind --tool=massif --time-unit=B `
  --massif-out-file=/mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-16-059-exact-clause-capacity/raw/baseline-unique-20000.massif `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-16-059-exact-clause-capacity/baseline/eprover `
  --cnf --silent --output-file=/dev/null `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus/unique-atom-20000.p

wsl.exe -d Ubuntu-24.04 -- valgrind --tool=massif --time-unit=B `
  --massif-out-file=/mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-16-059-exact-clause-capacity/raw/candidate-unique-20000.massif `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-16-059-exact-clause-capacity/candidate/eprover `
  --cnf --silent --output-file=/dev/null `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus/unique-atom-20000.p
```

## Live-heap result

| Implementation | Useful heap (B) | Extra heap (B) | Total (B) |
| --- | ---: | ---: | ---: |
| Exact baseline | 86,847,328 | 8,646,416 | 95,493,744 |
| Candidate | 78,667,080 | 8,634,368 | 87,301,448 |

Useful heap falls 8,180,248 bytes (9.42%), and total live heap falls 8,192,296
bytes (8.58%). The nearest detailed snapshots attribute the intended three
allocations exactly:

| Owner | Baseline (B) | Candidate (B) |
| --- | ---: | ---: |
| `ProofStateInit` unprocessed clause store | 6,291,456 | 3,840,192 |
| clause preprocessing archive | 6,291,456 | 3,840,192 |
| canonicalization dense scratch | 6,291,456 | 3,840,192 |

Each owner therefore removes 2,451,264 bytes of capacity slack. The exact three
allocation delta is 7,353,792 bytes; the remaining peak difference comes from
the lower candidate peak occurring at a later allocation snapshot.

## Process scaling

The existing interleaved C/baseline/candidate harness ran five trials at 100,
1,000, 5,000, 10,000, and 20,000 formula owners for both repeated-term and
unique-atom corpora. All 150 processes exited zero. At 20,000 owners:

| Shape | Implementation | Wall (s) | CPU (s) | RSS (KiB) |
| --- | --- | ---: | ---: | ---: |
| Repeated | C | 0.46 | 0.11 | 34,864 |
| Repeated | Exact baseline | 0.29 | 0.24 | 62,212 |
| Repeated | Candidate | 0.27 | 0.22 | 61,552 |
| Unique atom | C | 0.65 | 0.19 | 51,328 |
| Unique atom | Exact baseline | 0.62 | 0.59 | 93,552 |
| Unique atom | Candidate | 0.63 | 0.62 | 92,976 |

Median process RSS falls 660 KiB on repeated owners and 576 KiB on unique
owners. The OS-visible reduction is smaller than Massif's live-heap reduction
because the process allocator retains released pages. Timing changes are within
the resolution and load variance of these sub-second runs.

## Compatibility and proof-search controls

The standard 50-case C/Rust report is
`.artifacts/e-compare/20260716-192302-949681/comparison.json`. Its three
mismatches are established cases: `HEN011-2.p` resource behavior,
`sledgehammer.p` normalized proof text, and the synthetic one-second CPU-limit
fixture. The other 47 cases match; in particular, no allocation-layout proof
ordering difference was introduced.

The standard five-run native report is
`.artifacts/e-compare/20260716-193623-623098-benchmark/benchmark.json`. It
measures a 2.934x aggregate Rust/C wall ratio across the nine behavior-matching
cases. `LUSK6.lop` is 2.618x with 241,272 KiB Rust maximum RSS, and
`LUSK6ext.lop` is 2.490x with 467,912 KiB. Those RSS values are effectively
unchanged from the exact compact-term-link controls (241,236 and 467,728 KiB),
so exact initial batch capacity does not regress sustained proof-search memory.
The known `BOO020-1.p` outcome mismatch is excluded from the aggregate.

## Conclusion

Retain exact clause batch capacity and bounded canonicalization scratch. The
change mirrors C's one-cell-per-clause allocation more closely for two owners
whose final counts are known, removes 8.58% from the focused total live-heap
peak, modestly lowers process RSS, and preserves proof behavior and throughput.

This remains a partial improvement to `E_Rust_Port-j76.1.8`. The focused
unique-owner process still uses about 1.81x C RSS, and Formula Sets still have
remaining free/delete, derivation-owner, and GC-marker parity work.

## Validation

The retained candidate passed:

- all 4,179 library tests and every binary/integration target;
- the exact batch-capacity regression;
- `cargo fmt --all -- --check`;
- `cargo check --locked --all-targets --all-features`;
- all-target, all-feature Clippy with warnings and `clippy::pedantic` denied;
- locked Windows and WSL release builds of `eprover`;
- all 32 Python interop-tool tests;
- C-source documentation coverage, Change Later wording, local-link, and
  manual-section regeneration checks.
