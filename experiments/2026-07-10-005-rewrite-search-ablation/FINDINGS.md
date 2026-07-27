# Rewrite Search Ablation

Date: 2026-07-10

## Question

Can Rust's remaining `LUSK6.lop` forward-rewrite cost be reduced by making PDT candidate traversal incremental or by avoiding unchanged-term shell allocation during recursive normalization?

## Setup

- Parent baseline: commit `70ca322e` on Windows release Rust.
- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Shared arguments:

```powershell
--auto --silent --print-statistics --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new
```

- Normal build: `cargo build --release --bin eprover`.
- Instrumented build: `cargo build --release --features instrument-perf-ctr --bin eprover`.
- Focused verification: `cargo test --all-features --lib clauses::pdtrees::tests` and `cargo test --all-features --lib clauses::rewrite::tests`.
- Each rejected source candidate was removed with an explicit reverse patch; the worktree was checked against the exact parent before the next candidate.

## Incremental PDT Candidate

The retained Rust PDT recursively collects compact clause-id/side candidates and later runs `subst_match_complete` on each candidate. Three resumable cursor designs were tested:

| Cursor binding state | Forward rewrite | Forward modify |
| --- | ---: | ---: |
| Retained collector baseline | 2.232 s | 2.460 s |
| Per-frame `BTreeMap` clone | 2.986 s | 3.233 s |
| Per-frame small-`Vec` clone | 3.086 s | 3.336 s |
| Cursor-owned bind/unbind stack | 2.910 s | 3.162 s |

All variants preserved candidate order, constraints, repeated-variable checks, early-yield node counts, and the principal proof counters. The mutable-stack variant took 6.456 wall seconds in its final instrumented run, with 4,897 processed clauses, 122,867 generated clauses, 92,847 non-redundant clauses, and 518,287 rewrite steps.

The C cursor is not merely incremental: `PDTreeFindNextIndexedLeaf` extends the caller's live substitution and leaves those bindings active for the returned demodulator. The Rust cursor still repeated the complete match after returning a compact candidate, so its state-machine overhead had no compensating substitution reuse.

## Deferred Term Shell

The second candidate changed `term_subterm_rewrite_plain` to allocate and populate a top-cell copy only after the first rewritten child. The retained implementation, like C, eagerly allocates the shell and discards it when no child changes.

| Build | Trial 1 | Trial 2 | Trial 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Parent baseline | 7.353 | 5.804 | 5.702 | 5.804 |
| Deferred shell | 7.360 | 6.131 | 5.960 | 6.131 |

The candidate regressed the matched median by 5.6%. It also selected the known 92,833 non-redundant/518,389 rewrite-step allocation-layout variant, while the baseline trials selected the 92,847/518,287 variant. The source candidate was removed.

## Falsification Checks

- All 28 focused PDT tests passed for every incremental cursor, including candidate order, type checks, repeated variables, age/size constraints, and early-yield visit accounting.
- All 28 focused rewrite tests passed for the deferred-allocation candidate.
- Every measured run reported `SZS status Unsatisfiable`, 4,897 processed clauses, and 122,867 generated clauses.
- `MguTimer` was only about 0.026-0.027 seconds, so duplicate complete matching alone cannot explain the measured 2.2-second retained forward-rewrite cost.

## Conclusion

Neither candidate is retained. Incremental PDT traversal should be revisited only as part of a C-shaped search result that reuses the index-produced substitution. Eager term-shell allocation should remain until a different construction strategy demonstrates a matched end-to-end gain; fewer temporary allocations did not improve this allocator-sensitive workload.

## Limits

- Timings are Windows wall-clock samples on one workload and remain sensitive to post-build warmup and allocation layout.
- The PDT baseline timer came from the retained instrumented run in the preceding stable-clause-slot experiment rather than a same-build interleaving of every cursor prototype.
- No sampling profiler was used, so the experiment rejects these implementations but does not assign the remaining rewrite cost to a new subcomponent.
