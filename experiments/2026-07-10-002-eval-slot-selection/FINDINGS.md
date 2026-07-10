# Evaluation-Slot Selection Experiment

Date: 2026-07-10

## Question

Does replacing `ClauseSet`'s linear evaluation-object lookup with stable internal clause slots materially reduce `LUSK6.lop` proof-search time?

## Setup

- Baseline: commit `d436cc24` on Windows release Rust, median 11.11 seconds from the preceding LUSK6 experiment.
- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Shared arguments:

```powershell
--auto --silent --print-statistics --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new
```

- Candidate design: a private sparse clause store with stable internal slots, direct evaluation-object-to-slot lookup, hole-preserving extraction, and map rebuilds after sorting.
- Phase attribution build:

```powershell
cargo build --release --features instrument-perf-ctr --bin eprover
```

- Raw outputs and the removed candidate patch: `.artifacts/experiments/2026-07-10-002-eval-slot-selection/`.

## Results

| Trial | Seconds |
| --- | ---: |
| Candidate 1 | 12.36 |
| Candidate 2 | 10.76 |
| Candidate 3 | 11.30 |

The candidate median was 11.30 seconds, compared with the retained 11.11-second baseline. This does not establish an end-to-end improvement.

The instrumented candidate run took 12.38 seconds and attributed only 0.114 seconds to the complete selection wrapper across 4,897 selected clauses. The same run attributed 4.650 seconds to forward rewriting and 2.596 seconds to clause generation.

## Falsification Checks

- The candidate compiled with all features and passed all 42 focused clause-set tests, including a new middle-hole, insertion, sorting, property-update, and extraction regression.
- Every measured run retained `SZS status Unsatisfiable`, 4,897 processed clauses, 122,867 generated clauses, 259 backward rewrites, and 122,867 paramodulations.
- The observed 92,833/92,847 non-redundant-clause allocation-layout split remained within the already documented variants.
- The candidate source and its regression test were removed after the timing result; the retained branch does not carry the unvalidated sparse-store complexity.

## Conclusion

Linear evaluation-object lookup is a real ownership gap, but it is not a material bottleneck on this parity workload after the parent-liveness fixes. A stable clause arena should be introduced only as part of the broader handle-ownership port, not as an isolated LUSK6 optimization.

Follow-up: after shared-term weight caching reduced the surrounding workload, a matched rebuild A/B made the selection cost material. The hardened, bounded-compaction version is retained and measured in `experiments/2026-07-10-004-stable-clause-slots/`; this note remains the record of the earlier falsified configuration.

## Limits

- The result applies to this proof-search workload; delete-bad-heavy schedules may spend more time resolving evaluation handles.
- Windows scheduling noise is visible across the three trials, but the 0.114-second phase total independently bounds the likely gain well below the remaining roughly tenfold C/Rust gap.
