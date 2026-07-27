# Post-Cache PDTree Profile

## Question

After caching demodulator-index coverage, what remains expensive in the
`LUSK6.lop` rewrite path, and can the next PDTree change improve it without
regressing canonical behavior?

## Setup

- Exact baseline commit: `3e4fb0a6` (`Clean up cross-platform Rust checks`).
- Workload: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Shared arguments: `--auto --silent --cpu-limit=600 --detsort-rw
  --detsort-new`.
- Candidate benchmark script: `benchmark-candidate.sh` in this directory.
- Raw Callgrind output:
  `.artifacts/experiments/2026-07-11-006-post-cache-callgrind/callgrind.out`.
- Alternating timing outputs in the same artifact directory:
  `lazy-cursor-only-timings.txt`,
  `lazy-cursor-and-compact-query-timings.txt`, and
  `compact-query-timings.txt`.

The profile command was:

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-11-006-post-cache-callgrind/callgrind.out \
  ./target/release/eprover --auto --silent --cpu-limit=600 \
  --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop

callgrind_annotate --inclusive=no --threshold=95 --auto=no \
  .artifacts/experiments/2026-07-11-006-post-cache-callgrind/callgrind.out
```

The exact baseline was built in a detached worktree. Ten baseline and ten
candidate runs were alternated, reversing pair order each iteration:

```bash
bash experiments/2026-07-11-006-post-cache-callgrind/benchmark-candidate.sh
```

## Profile

The proof completed with 30,019,562,208 instruction references. The previous
whole-set coverage check fell from 7.69% to 0.18%, confirming the retained cache
removed its intended hot path. The largest remaining self costs included:

| Function or group | Instruction share |
| --- | ---: |
| allocator `_int_free`/`free` | 10.76% |
| allocator `_int_malloc`/`malloc` | 9.58% |
| `PdTree::collect_matching_occurrences` | 4.95% |
| `push_prefix_query_cell` | 3.92% |
| `term_top_compare_for_problem` | 3.55% |
| `Term::argument_clones` | 2.79% |
| `term_deref` plus `deref_step` | 4.98% |
| `PdTree::node_may_have_matchable_path` | 2.27% |

## Retained Change

`PdtSearchState` now keeps the already-built `Vec<PrefixQueryCell>` directly
instead of copying it into four parallel vectors for tokens, spans, type UIDs,
and subtree weights. Matching and conservative path checks read those fields
from the single query array. Candidate collection and repeated-variable
comparison retain their previous traversal and token/span semantics.

Final alternating measurements:

| Build | Median user CPU | Median wall |
| --- | ---: | ---: |
| Exact baseline | 3.765 s | 3.75 s |
| Compact query state | 3.650 s | 3.655 s |

The median user-CPU improvement is about 3.1%. An isolated Windows
`GEO288+1.p` run also proved the theorem in 58.1 seconds, compared with 59.2
seconds for the exact baseline.

## Rejected Cursor

An explicit DFS continuation stack made `search_next_matching_occurrence`
return candidates without first materializing the complete occurrence vector.
It preserved all 28 focused PDTree tests and improved the LUSK6 median by about
1.2% alone and 2.2% with compact query storage. It was nevertheless rejected:
the canonical 50-case run at `.artifacts/e-compare/20260711-230041-892973/`
changed `GEO288+1.p` from a theorem to `ResourceOut`. Isolated runs showed the
same proof search completing in about 65 seconds with the cursor versus 59.2
seconds at baseline. The prototype traversed incrementally but did not carry C's
live substitution into candidate validation, and its task machinery added cost
on searches that consumed many candidates.

## Falsification Checks

- All 28 focused PDTree tests pass with the retained representation.
- The final retained source passes full Windows and WSL tests, pedantic Clippy,
  and release builds.
- The cursor's LUSK6 gain was not accepted after the independent GEO regression.
- The compact-only GEO run restores the theorem inside the 60-second limit.
- The final 50-case report at
  `.artifacts/e-compare/20260712-000443-172305/` has seven known mismatches;
  both GEO and BOO020 match in that Windows-candidate run.
- The final five-run native report at
  `.artifacts/e-compare/20260712-002030-352475-benchmark/` improves the aggregate
  ratio from 3.8238x to 3.509x; performance parity remains unmet.
- The nested upstream checkout was not modified.

## Conclusion

The next accepted post-cache optimization is query-state compaction, not a
standalone Rust task-stack cursor. C's `PDTreeFindNextDemodulator` remains the
correct architectural target because it couples incremental tree traversal to
the substitution used by rewriting. A future cursor port must preserve that
coupling and beat both rewrite-heavy and candidate-heavy workloads.

## Limits

- The retained speedup is measured on one rewrite-heavy workload and remains
  small relative to the multi-fold C/Rust gap.
- Timing drift is visible across long WSL sessions; conclusions use alternating
  pairs and medians, plus the independent GEO falsification case.
- The current Rust candidate cursor still materializes all matching occurrences.
- Allocator, term comparison, dereference, and argument-cloning costs remain
  major profile targets.
