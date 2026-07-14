# Iterative PDTree Query Builder

## Question

Can PDTree prefix-query construction avoid recursive calls and one dynamic
argument borrow per child by using reusable typed traversal frames, while
preserving every query cell and improving real proof-search throughput?

## Setup

- Baseline commit: `d6c2b502` (`Reuse PDTree query storage across searches`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-004-pdt-query-builder/baseline-eprover`.
- Baseline SHA-256:
  `0a2ba9fd18c8c1ad81e02a0bde686281acc20d1ac94e546e7ac534da1bf944f2`.
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
bash experiments/2026-07-14-004-pdt-query-builder/benchmark.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-004-pdt-query-builder/callgrind-candidate.out \
  target/release/eprover --auto --silent --cpu-limit=600 \
  --memory-limit=2048 --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
```

The repository-wide gates used the freshly rebuilt Windows binary and native
WSL benchmark:

```powershell
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

## Implementation

The production builder uses a tree-owned `Vec<PrefixQueryBuildFrame>` with
typed `Enter(Term)` and `Exit(cell_index)` frames. Each parent borrows its
argument slice once, pushes children in reverse so traversal remains
left-to-right, moves the term into its query cell, and fills the subtree span
when the matching exit frame is reached. The vector is empty but retains
capacity between searches.

The previous recursive function remains compiled only for tests. Its output is
compared directly with the iterative builder, covering token, term identity,
type UID, weight, order, and subtree span.

## Results

Seven alternating LUSK6 pairs measured a baseline median of `2.81` and a
candidate median of `2.70` user seconds, a 3.9% improvement. Raw timings are
retained at
`.artifacts/experiments/2026-07-14-004-pdt-query-builder/alternating-times.txt`.

Callgrind moves in the opposite direction: the candidate executes
`22,458,102,696` instructions versus `22,337,531,886`, an increase of
`120,570,810` or 0.54%. The main
`search_next_matching_occurrence_with_subst` routine remains exactly
`1,337,404,051` instructions. Attribution places the increase in
`record_search_init`, where typed frame bookkeeping replaces repeated dynamic
`RefCell` argument borrowing. Consistent CPU-time improvements on all measured
workloads make elapsed CPU the acceptance metric for this tradeoff.

The paired long-search checks are:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 42.65 s | 41.83 s | Unsatisfiable |
| GEO288+1 | 52.12 s | 51.98 s | Theorem |

HEN011 retains exact `265,284` processed, `1,062,557` generated, `1,022,255`
rewrite-step, and inspected subsumption counters. GEO288 retains `10,215`
processed, `128,583` generated, `127,990` paramodulation, and `34,170` rewrite
steps. Small final non-redundant and unprocessed counts remain consistent with
the documented allocation-sensitive leaf ordering.

The 50-case differential report is
`.artifacts/e-compare/20260714-125028-176486/`. It has seven mismatches because
both known near-limit GEO288 and SWV851 cases miss in the same loaded run. The
other residuals are BOO020, HEN011, the synthetic one-second LUSK6 case, and
normalized output for LUSK6ext and sledgehammer. Direct 600-second-budget GEO
runs prove the theorem for both exact baseline and candidate, with the
candidate slightly faster; no new output or proof-path mismatch appears.

The five-run native report is
`.artifacts/e-compare/20260714-130227-242304-benchmark/`. Its aggregate Rust/C
wall-time ratio is `3.440`, improving from `3.479`; LUSK6 improves from `3.003`
to `2.935`. LUSK6ext's Rust median improves from `6.216` to `6.056` seconds,
although its ratio is `2.982` because the C samples were also faster.

## Falsification Checks

- All 32 PDTree tests pass, including exact iterative-versus-recursive query
  cell and span parity.
- Strict all-target, all-feature Clippy passes with pedantic warnings denied.
- The full suite passes 4,057 library tests and 3 integration tests, with all
  binary targets clean.
- LUSK6, HEN011, and GEO288 retain proof status and principal counters.
- The main matching-search Callgrind count is unchanged.
- The full native benchmark improves both aggregate ratio and absolute Rust
  medians for the two long LUSK cases.
- The benchmark script resolves paths from its own experiment directory.

## Conclusion And Limits

The reusable typed builder is accepted because it improves CPU time on all
three measured proof workloads and the standard benchmark while preserving
exact query cells and proof behavior. Its extra enter/exit bookkeeping raises
the instruction count slightly, so it is not a universal reduction in work.

C's direct raw argument-array traversal remains instruction-efficient, but its
reversible shared pointer stack relies on a precedence-sensitive loop
condition and assertion-only stack-shape checks. A later C cleanup should keep
direct argument access and allocation reuse while moving typed traversal state
into an explicit search object. Overall Rust/C performance parity and the
higher-order materialized matching fallback remain open.
