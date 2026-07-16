# Clause-info owner layout

## Question

Does matching C's nullable `ClauseInfo_p` ownership reduce the memory cost of
large clause and wrapped-formula populations without changing source-info
semantics or introducing a compatibility regression?

C stores source metadata behind pointers in both
`eprover/CLAUSES/ccl_clauses.h` and
`eprover/CLAUSES/ccl_formula_wrapper.h`. Rust instead embedded
`Option<ClauseInfo>` in every `Clause` and `WrappedFormula`, although copied and
derived objects normally have no source metadata.

## Setup and commands

The baseline is commit `00b36aed` and the candidate changes both owners to
`Option<Box<ClauseInfo>>`. Their public setter/taker APIs continue to accept and
return owned `ClauseInfo` values. Existing repeated-owner and unique-symbol CNF
corpora from experiments 009 and 010 cover 100, 1,000, 5,000, 10,000, and
20,000 formula owners.

The native Linux measurements used the cached upstream C binary, a baseline
Rust release copied to the ignored artifact directory, and the WSL release
candidate:

```bash
bash experiments/2026-07-16-011-clause-info-owner-layout/benchmark.sh \
  "$c_binary" \
  .artifacts/experiments/2026-07-16-011-clause-info-owner-layout/baseline/eprover \
  "$candidate_binary" \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus \
  .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus \
  .artifacts/experiments/2026-07-16-011-clause-info-owner-layout/raw/scaling-final.csv

python3 experiments/2026-07-16-011-clause-info-owner-layout/analyze.py \
  .artifacts/experiments/2026-07-16-011-clause-info-owner-layout/raw/scaling-final.csv

valgrind --tool=massif --time-unit=B \
  --massif-out-file=.artifacts/experiments/2026-07-16-011-clause-info-owner-layout/raw/baseline-repeated-01000.massif \
  .artifacts/experiments/2026-07-16-011-clause-info-owner-layout/baseline/eprover \
  --cnf --silent --output-file=/dev/null \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus/repeated-01000.p

valgrind --tool=massif --time-unit=B \
  --massif-out-file=.artifacts/experiments/2026-07-16-011-clause-info-owner-layout/raw/candidate-repeated-01000.massif \
  "$candidate_binary" --cnf --silent --output-file=/dev/null \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus/repeated-01000.p
```

The timeout-sensitive `SWV851-1.p` result was checked against a native Windows
release built from an archive of the exact baseline commit and against the
combined candidate:

```powershell
& .\experiments\2026-07-16-011-clause-info-owner-layout\benchmark-swv.ps1 `
  -Binary .artifacts\experiments\2026-07-16-011-clause-info-owner-layout\baseline-src\target\release\eprover.exe `
  -Problem .artifacts\experiments\2026-07-16-011-clause-info-owner-layout\corpus\SWV851-1.p `
  -OutputDirectory .artifacts\experiments\2026-07-16-011-clause-info-owner-layout\raw\swv-baseline

& .\experiments\2026-07-16-011-clause-info-owner-layout\benchmark-swv.ps1 `
  -Binary .artifacts\experiments\2026-07-16-011-clause-info-owner-layout\ablations\both\eprover.exe `
  -Problem .artifacts\experiments\2026-07-16-011-clause-info-owner-layout\corpus\SWV851-1.p `
  -OutputDirectory .artifacts\experiments\2026-07-16-011-clause-info-owner-layout\raw\swv-both
```

Final project checks used:

```powershell
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --lib clauseinfo --all-features
cargo test --lib wrapped_formula --all-features
cargo test --all-targets --all-features
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

## Results

Massif attributes the former owner allocations to 136-byte
`WrappedFormula` values and 248-byte `Clause` values. Pointer-backed source
metadata reduces those allocations to 80 and 192 bytes respectively: 56 bytes
per owner in both cases. On 1,000 repeated owners, useful peak heap fell from
4,072,114 to 3,622,698 bytes, a reduction of 449,416 bytes (11.0%). Heap
bookkeeping rose slightly because populated source metadata now has its own box,
but the useful-heap reduction dominates.

Five-run native medians at the largest corpus size were:

| Shape | Implementation | Wall (s) | CPU (s) | RSS (KiB) |
| --- | --- | ---: | ---: | ---: |
| Repeated, 20,000 | C | 0.240 | 0.080 | 34,208 |
| Repeated, 20,000 | Baseline Rust | 0.240 | 0.180 | 77,088 |
| Repeated, 20,000 | Candidate Rust | 0.170 | 0.160 | 69,840 |
| Unique, 20,000 | C | 0.450 | 0.140 | 50,704 |
| Unique, 20,000 | Baseline Rust | 0.560 | 0.490 | 110,148 |
| Unique, 20,000 | Candidate Rust | 0.500 | 0.490 | 102,504 |

Thus candidate RSS falls by 7,248 KiB (9.40%) on repeated owners and 7,644
KiB (6.94%) on unique owners. Rust/C RSS improves from 2.253x to 2.042x and
from 2.173x to 2.022x respectively. At 100 and 1,000 owners, allocator startup
and the additional boxes can add 160-400 KiB; the candidate crosses below the
baseline at 5,000 owners and retains the benefit as populations grow.

The standard 50-case compatibility report is
`.artifacts/e-compare/20260716-021940-989598/comparison.json`. Its six
mismatches are `BOO020-1.p`, `GEO288+1.p`, `HEN011-2.p`, `SWV851-1.p`,
`sledgehammer.p`, and `synthetic/cpu-limit-LUSK6.lop`. `SWV851-1.p` replaced
recent intermittent `LUSK6ext.lop`/`lists.p` results in this run, but it is not
a new port behavior: earlier reports also crossed this resource boundary. More
importantly, the exact pre-change Windows Rust binary also aborted for an
allocation failure under the same options after 55.689 seconds; the candidate
did the same after 60.845 seconds. Their raw records are under `raw/swv-baseline`
and `raw/swv-both`. The layout change therefore did not introduce that
candidate/C outcome mismatch.

The standard five-run benchmark is
`.artifacts/e-compare/20260716-024356-798447-benchmark/benchmark.json`. Its
aggregate Rust/C wall ratio is 3.368x, worse than the preceding 2.995x report
and still far above the required 1.10x. The aggregate is volatile because seven
of its nine included cases complete in under 25 ms and several C startup
medians roughly halved in this run. The sustained `LUSK6` ratio is essentially
unchanged at 2.721x versus 2.714x, while Rust maximum RSS falls from 264,356 to
257,756 KiB. `LUSK6ext` Rust RSS falls from 514,464 to 503,884 KiB; its ratio is
2.816x versus 2.660x because C improved more than Rust in this sample. The
port-wide performance requirement remains open.

## Falsification checks

- Every focused sample exited zero; the analyzer requires exactly five samples
  per implementation/shape/count group and rejects negative wall times.
- The first raw scaling run, `raw/scaling.csv`, recorded one negative and one
  compensating positive C wall time after a WSL clock correction. It was
  rejected rather than summarized. The harness was hardened and
  `raw/scaling-final.csv` contains the valid interleaved run.
- Both repeated-term and unique-symbol corpora were measured so the result is
  not explained only by term sharing.
- Existing source-info round-trip, copy, parsing, printing, and clause/formula
  conversion tests passed with the boxed representation.
- The exact baseline/candidate `SWV851-1.p` control distinguishes a pre-existing
  resource-boundary abort from a patch-induced search regression.
- The full 4,088-test library suite, all binary targets, and all three schedule
  integration tests passed. Formatting, checking, pedantic Clippy, and the
  locked release build also passed.

## Conclusion and limits

Retain pointer-backed `ClauseInfo` in both `Clause` and `WrappedFormula`. It
matches C's ownership model, preserves the owned public API and source-info
semantics, and removes 56 bytes from every empty-info owner. The measured memory
benefit grows with owner population and is visible in both focused corpora and
the standard sustained benchmark cases.

This is a partial memory-parity improvement, not completion of the formula-set
work or the port. Large focused corpora still use about 2.0x C's RSS, and the
standard performance ratio remains about 3.4x in this run. Further ownership,
container, and proof-search optimization work remains necessary.
