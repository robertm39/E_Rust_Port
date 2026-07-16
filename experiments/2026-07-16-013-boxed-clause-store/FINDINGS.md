# Boxed sparse clause store

## Question

Can `ClauseSet` match C's individually allocated, pointer-stable clause cells by
boxing Rust sparse-store entries, while reducing the live memory cost of
power-of-two `Vec<Option<Clause>>` buffers and whole-clause sort temporaries?

## Setup and commands

The baseline is commit `08c41b04`, whose production code is unchanged from the
clause-info layout commit `6a02b3aa`. The temporary candidate changed
`SparseClauseStore` from `Vec<Option<Clause>>` to
`Vec<Option<Box<Clause>>>`, adapted all owned/shared/mutable iterators, and
sorted pointers rather than 192-byte clause values. A focused regression
confirmed that the first clause retained the same address after 4,096 further
insertions, matching C's intrusive-list address stability.

The correct native candidate was `target/release/eprover`, produced by WSL
Cargo from the working tree. Five-run measurements used:

```bash
cargo build --locked --release --bin eprover

bash experiments/2026-07-16-011-clause-info-owner-layout/benchmark.sh \
  "$c_binary" \
  .artifacts/experiments/2026-07-16-013-boxed-clause-store/baseline/eprover \
  target/release/eprover \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus \
  .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus \
  .artifacts/experiments/2026-07-16-013-boxed-clause-store/raw/scaling-corrected.csv

python3 experiments/2026-07-16-011-clause-info-owner-layout/analyze.py \
  .artifacts/experiments/2026-07-16-013-boxed-clause-store/raw/scaling-corrected.csv
```

Paired Massif runs used the repeated-owner 1,000- and 20,000-owner corpora:

```bash
valgrind --tool=massif --time-unit=B \
  --massif-out-file="$massif_output" \
  "$binary" --cnf --silent --output-file=/dev/null "$problem"
```

## Results

The boxed store reduces useful live bytes but worsens the actual process peak:

| Shape, owners | Implementation | Wall median (s) | CPU median (s) | RSS median (KiB) |
| --- | --- | ---: | ---: | ---: |
| Repeated, 20,000 | Baseline | 0.220 | 0.170 | 69,364 |
| Repeated, 20,000 | Boxed | 0.210 | 0.170 | 70,428 |
| Unique, 20,000 | Baseline | 0.530 | 0.500 | 102,184 |
| Unique, 20,000 | Boxed | 0.500 | 0.490 | 103,416 |

RSS increases by 1,064 KiB (1.53%) on repeated owners and 1,232 KiB (1.21%)
on unique owners. CPU medians are unchanged or within 0.01 seconds.

Massif exposes the allocator tradeoff:

| Owners | Implementation | Useful heap (B) | Extra heap (B) | Total (B) |
| ---: | --- | ---: | ---: | ---: |
| 1,000 | Baseline | 3,622,666 | 143,238 | 3,765,904 |
| 1,000 | Boxed | 3,613,175 | 175,345 | 3,788,520 |
| 20,000 | Baseline | 67,024,005 | 2,714,363 | 69,738,368 |
| 20,000 | Boxed | 61,225,233 | 3,342,487 | 64,567,720 |

At 20,000 owners, useful heap falls 5,798,772 bytes (8.65%) and total Massif
heap falls 5,170,648 bytes (7.41%), but per-clause allocation bookkeeping rises
628,124 bytes (23.1%). System-allocator fragmentation/high-water behavior more
than offsets that useful-byte saving in process RSS.

## Falsification checks

- Focused storage, insertion/extraction, and 4,096-growth pointer-stability
  tests passed while the candidate was present.
- Both repeated-term and unique-symbol corpora showed higher large-case RSS,
  ruling out term-sharing shape as the explanation.
- Massif independently confirmed that dense-vector capacity was reduced, while
  also quantifying the additional allocation overhead.
- The first attempted run accidentally invoked the unchanged cached standard
  benchmark binary rather than WSL Cargo's working-tree ELF. Identical Massif
  byte counts exposed the error. `scaling.csv` and unlabelled
  `candidate-repeated-*.massif` are invalid no-change controls; the decision uses
  only `scaling-corrected.csv` and `candidate-corrected-repeated-*.massif`.
  File hashes also confirmed the corrected candidate differed from the
  baseline.
- All production and test changes were reverted with `apply_patch`; formatting
  restored an empty source diff afterward.

## Conclusion and limits

Reject per-clause `Box` storage. Stable clause addresses and a 7.4% Massif
reduction are desirable, but no current safe caller retains a clause reference
across store mutation, and the operational RSS metric regresses on both large
corpora. The port should not trade lower useful bytes for higher resident
memory without a demonstrated compatibility need.

A chunked or arena-backed store could retain stable addresses and dense
allocation without one system allocation per clause, but it would also require
an explicit order vector so C-style sorting moves links rather than clauses.
That is a separate, larger design experiment.
