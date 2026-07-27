# Formula-set capacity release

## Question

Do `VecDeque` backing buffers remain live after C-shaped formula archive and CNF
drains, and can exact reservation plus releasing empty-set storage reduce the
remaining formula-owner memory gap?

C formula sets retain only an intrusive-list anchor after their last wrapper is
extracted. The experiment tested whether Rust's deque capacity was a comparable
long-lived allocation.

## Setup and commands

The baseline is commit `6a02b3aa`. Its native Linux release was copied to the
ignored artifact directory before the candidate build. The temporary candidate:

- reserved exact archive and temporary-set capacity for bulk archive/CNF work;
- shrank stable archive buffers after the bulk operations;
- replaced emptied deques with new empty deques after clear, last extraction,
  and set-to-set transfer;
- added focused tests requiring empty formula sets to have zero backing
  capacity.

The candidate was built by WSL Cargo at the repository-local Linux path
`target/release/eprover`. The existing five-run interleaved scaling harness was
then reused:

```bash
cargo build --locked --release --bin eprover

bash experiments/2026-07-16-011-clause-info-owner-layout/benchmark.sh \
  "$c_binary" \
  .artifacts/experiments/2026-07-16-012-formula-set-capacity-release/baseline/eprover \
  target/release/eprover \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus \
  .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus \
  .artifacts/experiments/2026-07-16-012-formula-set-capacity-release/raw/scaling-corrected.csv

python3 experiments/2026-07-16-011-clause-info-owner-layout/analyze.py \
  .artifacts/experiments/2026-07-16-012-formula-set-capacity-release/raw/scaling-corrected.csv
```

Paired Massif runs used the repeated-owner 1,000- and 20,000-owner corpora:

```bash
valgrind --tool=massif --time-unit=B \
  --massif-out-file="$massif_output" \
  "$binary" --cnf --silent --output-file=/dev/null "$problem"
```

## Results

The exact-capacity and eager-release candidate reduced RSS on both owner shapes
at the discriminating 20,000-owner scale:

| Shape, owners | Implementation | Wall median (s) | CPU median (s) | RSS median (KiB) |
| --- | --- | ---: | ---: | ---: |
| Repeated, 20,000 | C | 0.210 | 0.070 | 34,240 |
| Repeated, 20,000 | Baseline | 0.240 | 0.180 | 69,360 |
| Repeated, 20,000 | Candidate | 0.240 | 0.180 | 67,828 |
| Unique, 20,000 | C | 0.550 | 0.140 | 50,704 |
| Unique, 20,000 | Baseline | 0.560 | 0.510 | 102,184 |
| Unique, 20,000 | Candidate | 0.600 | 0.520 | 100,568 |

Repeated-owner RSS fell by 1,532 KiB (2.21%) with unchanged CPU and wall
medians. Unique-owner RSS fell by 1,616 KiB (1.58%); its CPU median increased
from 0.51 to 0.52 seconds and its wall median from 0.56 to 0.60 seconds in this
load window. The Rust/C RSS ratio fell from 2.026 to 1.981 for repeated owners
and from 2.015 to 1.983 for unique owners.

Massif independently confirmed a smaller live allocation peak:

| Owners | Implementation | Useful heap (B) | Extra heap (B) | Total (B) |
| ---: | --- | ---: | ---: | ---: |
| 1,000 | Baseline | 3,622,666 | 143,238 | 3,765,904 |
| 1,000 | Candidate | 3,426,319 | 143,249 | 3,569,568 |
| 20,000 | Baseline | 67,024,005 | 2,714,363 | 69,738,368 |
| 20,000 | Candidate | 65,589,865 | 2,710,519 | 68,300,384 |

At 20,000 owners, useful live heap fell by 1,434,140 bytes (2.14%) and total
live heap fell by 1,437,984 bytes (2.06%). At 1,000 owners, total live heap fell
by 196,336 bytes (5.21%).

Corrected raw profiles and scaling samples are under
`.artifacts/experiments/2026-07-16-012-formula-set-capacity-release/raw/`.

## Falsification checks

- The first candidate run accidentally invoked the cached e-interop Linux
  executable rather than the repository-local WSL Cargo output. Byte-identical
  baseline/candidate Massif peaks exposed that no-change control. Its
  `scaling.csv` and unqualified `candidate-repeated-*.massif` files are invalid
  and retained only for auditability.
- The corrected run used `target/release/eprover`; its hash differed from the
  copied baseline. The valid files are `scaling-corrected.csv` and
  `candidate-corrected-repeated-*.massif`.
- The focused append-order, extraction, transfer, front-drain, and new
  zero-capacity clear tests passed with the candidate present.
- Both repeated-term and unique-symbol corpora showed lower candidate RSS.
- Massif measured useful allocations independently of allocator RSS high-water
  behavior and confirmed the process-RSS direction.
- The five-run analyzer accepted all 150 corrected samples, with five successful
  runs per implementation/shape/count group and no negative wall samples.

## Repository-wide acceptance

The production candidate passed formatting, `git diff --check`, all-target
checking, pedantic clippy with warnings denied, 4,089 library tests, all binary
targets, three schedule integration tests, and the locked optimized Windows
build.

The standard 50-case C-vs-Rust comparison report is
`.artifacts/e-compare/20260716-033329-923642/comparison.json`. It reported seven
mismatches: the six already present in the preceding baseline run
(`BOO020-1.p`, `GEO288+1.p`, `HEN011-2.p`, `SWV851-1.p`, `sledgehammer.p`, and
the synthetic CPU-limit case), plus a normalized-proof-output difference for
`LCL365-1.p`. Two isolated reruns of `LCL365-1.p` both matched C exactly:

- `.artifacts/e-compare/20260716-034609-076884/comparison.json`;
- `.artifacts/e-compare/20260716-034631-857601/comparison.json`.

The added full-run difference is therefore intermittent proof-output variation,
not a stable candidate behavior change. All other 43 cases matched C in the
standard run.

The standard five-run native benchmark report is
`.artifacts/e-compare/20260716-034721-434361-benchmark/benchmark.json`. Its
aggregate Rust/C wall-time ratio was 3.314x, compared with 3.368x in the
preceding run. `BOO020-1.p` retained its known differing resource outcome and
was excluded from the timing ratio. The sustained cases remained behaviorally
matched:

| Case | C wall median (s) | Rust wall median (s) | Ratio | C max RSS (KiB) | Rust max RSS (KiB) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `LUSK6.lop` | 1.038 | 2.789 | 2.687x | 119,680 | 257,744 |
| `LUSK6ext.lop` | 2.438 | 6.140 | 2.518x | 231,840 | 503,872 |

These standard-case RSS values are effectively unchanged from the preceding
run (12 KiB lower on each sustained Rust case), as expected for a change that
targets formula-owner bulk drains rather than saturation storage. The standard
benchmark remains well above the project's 1.10x performance target.

## Conclusion and limits

Retain exact reservation for the known bulk moves and release the backing deque
when a formula set becomes empty. This restores the C representation's empty
anchor behavior and gives a repeatable, independently confirmed memory
improvement without changing formula order or ownership semantics.

This is a partial improvement, not closure of the memory or performance gaps:
focused RSS is still about 1.98 times C at 20,000 owners, while the standard
benchmark remains 3.314 times C. The standard benchmark did not reproduce a
material performance regression from eager capacity release. Live wrappers,
the clause archive, term-bank, evaluation-index, and clause-store allocations
remain candidates for the residual peak.
