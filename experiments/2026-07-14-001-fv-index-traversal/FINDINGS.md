# FV-Index Traversal And Matching-Stack Capacity

## Question

Where does bounded HEN011 forward-subsumption time go after the stable-parent,
indexed-clause-set, and picked-buffer changes, and can the hot path be reduced
without changing feature-vector traversal or candidate order?

## Setup

The WSL release build used deterministic rewrite/new-clause sorting, a 2 GiB
memory limit, and either 5,000 or 150,000 processed clauses:

```powershell
wsl -d Ubuntu-24.04 -- bash -lc 'cd /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port && CARGO_TARGET_DIR=.artifacts/target-wsl cargo build --locked --release --bin eprover'
wsl -d Ubuntu-24.04 -- bash -lc 'cd /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port && /usr/bin/time -p ./.artifacts/target-wsl/release/eprover --auto --output-level=0 --print-statistics --cpu-limit=600 --memory-limit=2048 --processed-clauses-limit=150000 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p'
wsl -d Ubuntu-24.04 -- bash -lc 'cd /mnt/c/Users/rober/OneDrive/Desktop/E_Rust_Port && valgrind --tool=callgrind --callgrind-out-file=callgrind.out ./.artifacts/target-wsl/release/eprover --auto --output-level=0 --print-statistics --cpu-limit=600 --memory-limit=2048 --processed-clauses-limit=5000 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p'
```

A temporary `instrument-perf-ctr` build counted FV-query node, successor, leaf,
and clause-candidate visits at a 5,000-clause bound. Production candidates were
then measured with Callgrind and three direct 150,000-clause runs. The large raw
outputs remain under `.artifacts/experiments/2026-07-14-001-fv-index-traversal/`.

## Results

1. The 5,000-clause query visited 504,333 FV nodes, 497,090 successors,
   496,503 nonempty successors, 51,452 leaves, and 350,137 clause candidates.
   Only 587 successor visits were empty (about 0.12%), and traversal averaged
   about 1.44 nodes per candidate. Maintaining a second active-child map or
   pruning tombstones would therefore add mutation complexity for negligible
   query reduction.
2. Loading the problem type once and routing first-order clauses through the
   infallible bank-free subsumption wrapper did not improve the three-run CPU
   median. Converting hot C-style substitution or equation-shape assertions to
   Rust debug assertions also did not improve the median or Callgrind total;
   all three candidates were discarded.
3. The existing first-order matching stack reserved 32 inline term pairs on
   every call. Capacity sweeps produced these deterministic 5,000-clause
   instruction counts:

| Inline pairs | Instructions |
| ---: | ---: |
| 32 | 3,666,631,426 |
| 8 | 3,492,746,123 |
| 4 | 3,470,378,668 |
| 2 | 3,497,381,128 |

4. Four inline pairs reduce the Callgrind total by about 5.4% relative to 32.
   Two pairs spill too often, while 8 and 32 pay unnecessary fixed
   initialization cost. The vector spill path preserves exact LIFO order and
   remains covered by a 40-pair regression test.
5. At 150,000 clauses, the clean parent median was 20.62 user seconds. Eight
   inline pairs measured 18.78 seconds and four measured 18.40 seconds, a
   10.8% reduction for the accepted capacity. Wall time varied with host
   scheduling, so user time and deterministic instruction counts are the
   primary metrics.
6. Every measured candidate reached 150,026 processed clauses with exactly
   59,209,580 non-unit clause-subsumption calls and 42,636,223 recursive calls.
   The accepted change therefore affects throughput only, not feature-vector
   traversal, candidate order, or proof search.
7. The full WSL HEN011 proof completed in 45.52 wall seconds and 44.60 user
   seconds, down from the preceding accepted build's 49.43 wall seconds. Its
   principal search counters remain exact at 265,284 processed and 1,062,557
   generated clauses. Five additional non-unit subsumption calls and one
   recursive call are consistent with allocator-dependent pointer ordering.
8. The 50-case differential run completed with seven mismatches. LUSK6 now
   matches normalized output; the remaining differences are resource outcomes
   for BOO020, GEO288, HEN011, SWV851, and the synthetic one-second case, plus
   normalized output for LUSK6ext and sledgehammer.
9. The five-run benchmark reports a load-sensitive 3.608 aggregate Rust/C
   median wall-time ratio, with BOO020 excluded for behavior mismatch. LUSK6
   and LUSK6ext ratios are 3.140 and 3.122, while their absolute Rust medians
   improve to 2.703 and 6.408 seconds. The 1.10 parity target remains unmet.

## Raw Artifacts

- Temporary FV counters: `.artifacts/experiments/2026-07-14-001-fv-index-traversal/instrumented-5000.txt`
- Parent 150,000-clause runs: `.artifacts/experiments/2026-07-14-001-fv-index-traversal/baseline-150000*.{txt,time}`
- Capacity-8 runs: `.artifacts/experiments/2026-07-14-001-fv-index-traversal/match-stack-8-150000*.{txt,time}`
- Capacity-4 runs: `.artifacts/experiments/2026-07-14-001-fv-index-traversal/match-stack-4-150000*.{txt,time}`
- Capacity profiles: `.artifacts/experiments/2026-07-14-001-fv-index-traversal/callgrind-match-stack-*-5000.out`
- Rejected candidates: `.artifacts/experiments/2026-07-14-001-fv-index-traversal/{first-order-dispatch,subst-debug-asserts,eqn-debug-asserts}-150000*.{txt,time}`
- Full WSL proof: `.artifacts/experiments/2026-07-14-001-fv-index-traversal/match-stack-4-full.{txt,time}`
- Differential report: `.artifacts/e-compare/20260714-070551-760424/`
- Five-run benchmark: `.artifacts/e-compare/20260714-072014-774942-benchmark/`

## Falsification Checks

- FV traversal and empty-successor counts test the tombstone-pruning hypothesis.
- Exact clause and subsumption counters detect search or candidate-order drift.
- The 2/4/8/32 capacity sweep distinguishes fixed initialization cost from
  vector-spill cost.
- The existing 40-pair unit test exercises overflow and binding order.
- Rejected wrapper and assertion candidates prevent attributing the improvement
  to unrelated source-layout changes.

## Conclusion And Limits

FV-tree tombstones are not a meaningful HEN011 cost. The accepted improvement is
a smaller four-pair inline first-order matching stack, which preserves C's LIFO
algorithm and Rust's safe overflow path while materially reducing fixed work per
candidate. This does not remove the broader Rust/C term-representation and
allocation gap. The differential suite still has seven mismatches and the
benchmark remains well above the 1.10 parity target, so overall performance
work is still required.
