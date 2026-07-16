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

The candidate was compiled and the existing five-run interleaved scaling
harness was reused:

```bash
cargo build --locked --release --bin eprover

bash experiments/2026-07-16-011-clause-info-owner-layout/benchmark.sh \
  "$c_binary" \
  .artifacts/experiments/2026-07-16-012-formula-set-capacity-release/baseline/eprover \
  "$candidate_binary" \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus \
  .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus \
  .artifacts/experiments/2026-07-16-012-formula-set-capacity-release/raw/scaling.csv

python3 experiments/2026-07-16-011-clause-info-owner-layout/analyze.py \
  .artifacts/experiments/2026-07-16-012-formula-set-capacity-release/raw/scaling.csv
```

Paired Massif runs used the repeated-owner 1,000- and 20,000-owner corpora:

```bash
valgrind --tool=massif --time-unit=B \
  --massif-out-file="$massif_output" \
  "$binary" --cnf --silent --output-file=/dev/null "$problem"
```

## Results

The allocation-side-effect tests passed, but neither process RSS nor useful
live heap improved.

| Shape, owners | Implementation | Wall median (s) | CPU median (s) | RSS median (KiB) |
| --- | --- | ---: | ---: | ---: |
| Repeated, 20,000 | Baseline | 0.270 | 0.210 | 69,224 |
| Repeated, 20,000 | Candidate | 0.200 | 0.190 | 69,844 |
| Unique, 20,000 | Baseline | 0.640 | 0.570 | 102,184 |
| Unique, 20,000 | Candidate | 0.580 | 0.570 | 102,660 |

Candidate RSS is 620 KiB higher on repeated owners and 476 KiB higher on
unique owners. The wall medians were lower in this load window, but CPU was
essentially unchanged and the experiment targeted memory.

Massif's exact live peaks were decisive:

| Owners | Implementation | Useful heap (B) | Extra heap (B) | Total (B) |
| ---: | --- | ---: | ---: | ---: |
| 1,000 | Baseline | 3,622,666 | 143,238 | 3,765,904 |
| 1,000 | Candidate | 3,622,698 | 143,238 | 3,765,936 |
| 20,000 | Baseline | 67,024,005 | 2,714,363 | 69,738,368 |
| 20,000 | Candidate | 67,024,037 | 2,714,427 | 69,738,464 |

The candidate adds 32 useful bytes at both scales and adds 96 total bytes at
20,000 owners. It does not remove a live peak allocation.

Raw profiles and scaling samples are under
`.artifacts/experiments/2026-07-16-012-formula-set-capacity-release/raw/`.

## Falsification checks

- The focused append-order, extraction, transfer, and new zero-capacity tests
  all passed while the candidate was present.
- Both repeated-term and unique-symbol corpora showed higher candidate RSS.
- Massif measured useful allocations independently of allocator RSS high-water
  behavior and found the same peak within 32 bytes at both scales.
- The five-run analyzer accepted all 150 samples, with five successful runs per
  implementation/shape/count group and no negative wall samples.
- All production and test changes were reverted with `apply_patch`; formatting
  restored the exact baseline source, and `git diff` was empty afterward.

## Conclusion and limits

Reject exact reservation and eager empty-deque release. The hypothesized stale
capacity is not live at the measured peaks; existing append, local-drop, and
drain behavior already removes it before the discriminating phase. Eager
release only changes allocation chronology and slightly increases measured RSS
and heap overhead.

This result rules out formula-set backing capacity as the next meaningful
memory target. It does not rule out the live wrapper, clause archive, term-bank,
evaluation-index, or clause-store allocations identified in the same Massif
peak.
