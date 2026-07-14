# PDTree Query Storage Reuse

## Question

Can the first-order PDTree search reuse its flattened prefix-query allocation
between searches, while preserving exact traversal behavior and improving the
query-construction and allocator costs visible in the accepted LUSK6 profile?

## Setup

- Baseline commit: `940c2523` (`Reuse live PDTree substitutions for rewriting`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-003-pdt-query-reuse/baseline-eprover`.
- Baseline SHA-256:
  `eac26256a047aad9000d94ec201336ef66229bf53eced3e8bed8c37f3e1e19b8`.
- Candidate: WSL release `target/release/eprover`.
- Primary problem:
  `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Falsification problems:
  `eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p` and
  `eprover/EXAMPLE_PROBLEMS/TPTP/GEO288+1.p`.
- Common proof options:
  `--auto --cpu-limit=600 --memory-limit=2048 --detsort-rw --detsort-new`.

Rebuild and rerun the focused measurements from the repository root:

```bash
cargo build --locked --release --bin eprover
bash experiments/2026-07-14-003-pdt-query-reuse/benchmark.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-003-pdt-query-reuse/callgrind-candidate.out \
  target/release/eprover --auto --silent --cpu-limit=600 \
  --memory-limit=2048 --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
```

The corrected repository-wide gates used the freshly rebuilt Windows binary
and the native WSL benchmark:

```powershell
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

## Results

The candidate retains query cells in a tree-owned scratch vector after search
exit, clears their term handles, and moves the allocation into the next search
state. Prefix construction also moves child `Term` handles returned by
`argument` directly into their cells rather than cloning each handle again.

The accepted profile executes `22,337,531,886` LUSK6 instructions versus
`23,498,629,423` for the exact baseline: `1,161,097,537` fewer instructions,
a 4.94% reduction. The main
`search_next_matching_occurrence_with_subst` count remains exactly
`1,337,404,051`, showing that the accepted search path did not change.

Seven alternating LUSK6 pairs measured a baseline median of `2.85` and a
candidate median of `2.81` user seconds. Raw timings are retained at
`.artifacts/experiments/2026-07-14-003-pdt-query-reuse/alternating-times.txt`.

The paired long-search checks were mixed but behaviorally stable:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 44.37 s | 43.54 s | Unsatisfiable |
| GEO288+1 | 51.78 s | 53.43 s | Theorem |

HEN011 retained `265,284` processed clauses, `1,062,557` generated clauses,
and `1,022,255` rewrite steps. GEO288 retained `10,215` processed clauses,
`128,583` generated clauses, `127,990` paramodulations, and `34,170` rewrite
steps. Small final subsumption and unprocessed-count differences remain
consistent with the documented allocation-sensitive leaf ordering.

The corrected 50-case differential report is
`.artifacts/e-compare/20260714-113704-388175/`. It has six mismatches, the same
count as the accepted baseline. GEO288 reached the 60-second resource limit in
this loaded run while SWV851 did not; an earlier report using the unchanged
baseline binary produced the opposite membership, and both direct 600-second
budget runs prove GEO288, so the pair remains cutoff and host-load sensitive.
The stable residuals are BOO020, HEN011, the synthetic one-second LUSK6 case,
and normalized output for LUSK6ext and sledgehammer.

The five-run native report is
`.artifacts/e-compare/20260714-115002-662310-benchmark/`. Its aggregate Rust/C
wall-time ratio is `3.479`, improving from `3.486`; LUSK6 is `3.003` and
LUSK6ext is `2.819`.

## Falsification Checks

- All 32 PDTree tests pass, including a test that proves capacity is recycled
  after exit and reused by a smaller next query.
- Query token, span, weight, type, and term-order tests continue to pass after
  changing construction to move child handles.
- Strict all-target, all-feature Clippy passes with pedantic warnings denied.
- The deterministic LUSK6 search routine count is unchanged while total
  instruction count falls.
- HEN011 and GEO288 retain proof status and principal proof-search counters.
- The complete compatibility corpus adds no mismatch by count, and the native
  benchmark improves both the aggregate and the two long LUSK ratios.
- The benchmark script resolves paths from its own experiment directory.

## Conclusion And Limits

Reusing the flattened query allocation removes meaningful allocator and vector
growth work without changing first-order PDTree traversal. The optimization is
accepted because deterministic work, paired LUSK timing, HEN011 timing, and the
standard native benchmark all improve.

The single paired GEO288 run is about 3.2% slower, and near-limit compatibility
cases remain sensitive to host load and C's allocation-dependent leaf order.
This change does not resolve higher-order materialized matching or the overall
Rust/C performance gap. C's allocation reuse is worth retaining in a future
cleanup, but its reusable query stack, callback, tree-global branch order, and
node-global continuation state should move into an explicit search object.
