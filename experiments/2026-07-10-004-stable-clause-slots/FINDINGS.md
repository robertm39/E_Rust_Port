# Stable Clause-Slot Re-evaluation

Date: 2026-07-10

## Question

After shared-term weight caching reduced the rest of proof search, does replacing linear evaluation-object lookup and `VecDeque` removal with private clause slots now produce a repeatable end-to-end improvement on `LUSK6.lop`?

## Setup

- Parent baseline: commit `1fd37225` on Windows release Rust.
- C reference: cached upstream build under WSL Ubuntu 24.04, 1.08 seconds.
- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Shared arguments:

```powershell
--auto --silent --print-statistics --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new
```

- Release build: `cargo build --release --bin eprover`.
- Instrumented build: `cargo build --release --features instrument-perf-ctr --bin eprover`.
- Matched A/B procedure: save the source patch, reverse it to the exact parent, rebuild and run three baseline trials, restore it, rebuild, and run three candidate trials.
- Raw outputs, elapsed-second files, and the tested source patch: `.artifacts/experiments/2026-07-10-004-stable-clause-slots/`.

## Candidate

- Store clauses in an order-preserving private `Vec<Option<Clause>>` so removal leaves a direct internal slot instead of shifting the container; bounded compaction is the only operation that relocates live slots.
- Map each evaluation object directly to its current clause slot and use the existing slot map for demodulator clause ids.
- Track the first occupied slot so repeated front extraction remains constant-time.
- Compact only when at least 64 holes outnumber live clauses, and only at insertion or completed batch-deletion boundaries where all internal maps can be rebuilt atomically.
- Keep slots private. They are not yet the typed, generational clause handles needed by long-lived proof and global-index owners.

## Results

| Build | Trial 1 | Trial 2 | Trial 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Parent baseline | 7.52 | 6.52 | 6.54 | 6.54 |
| Stable clause slots | 6.94 | 5.79 | 5.39 | 5.79 |
| C reference | - | - | - | 1.08 |

The matched median improves by 11.4%. The first post-build trial improves by 7.7%, so the result is not explained only by later-run cache warming. The retained Rust median is still about 5.4 times the cross-OS C reference and does not satisfy final performance parity.

The final instrumented candidate run attributes 0.085 seconds to selection, 2.232 seconds to forward rewriting, and 1.939 seconds to generation. The three pre-change instrumented runs attributed 1.219-1.299 seconds to selection.

## Falsification Checks

- All 43 focused clause-set tests pass, including extraction through middle holes, insertion, sorting, direct property mutation, first-live tracking, and evaluation-map rebuild after bounded compaction.
- The full all-target/all-feature suite passes 4,000 library tests, every binary target, and all three schedule integration tests; pedantic Clippy, formatting, C-source coverage, Change Later wording, Markdown links, and manual-section regeneration checks also pass.
- The final instrumented run retains `SZS status Unsatisfiable`, 4,897 processed clauses, 122,867 generated clauses, 259 backward rewrites, and 122,867 paramodulations.
- A final normal-release `--proof-object=1` run emits a complete CNF refutation with the same principal counters.
- The run remains in the documented 92,833/92,847 non-redundant-clause and 518,389/518,287 rewrite-step allocation-layout variants.
- Duplicate marking remains O(n^2); the initial sparse prototype's position-based adaptation would have made it O(n^3) and was replaced before retention.
- Batch deletions defer compaction until their saved slot list is no longer in use, preventing slot invalidation during traversal.

## Conclusion

The earlier rejection was valid for its pre-cache configuration, but no longer applies after other hot paths became cheaper. Private sparse clause slots now provide a measured end-to-end gain while moving set ownership closer to C's constant-time intrusive extraction without introducing unsafe Rust.

## Limits

- Sparse slots are internal relocation-prone implementation details, not stable public clause handles.
- Iteration still visits holes until bounded compaction runs, and the safe sorted evaluation roots still differ from C's intrusive splay-tree locality.
- Windows Rust and WSL C timings are not a same-OS final performance certification.
- Remaining measured costs are dominated by forward rewriting, generation, and generated-clause insertion.
