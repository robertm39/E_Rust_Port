# One-Pass PDTree Query Metadata

## Question

Can query construction classify each term once and reuse its token, type,
weight, traversal lower bound, and free-variable decision without changing
first-order or higher-order PDTree behavior?

## Setup

- Baseline commit: `f31a4611` (`Build PDTree queries with reusable frames`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-005-pdt-query-metadata/baseline-eprover`.
- Baseline SHA-256:
  `8b31e29b6a23921db93806b06da711f335b36fc37439012238c386d2ec66755c`.
- Candidate SHA-256:
  `23546b845227e8169866c396c8b801daa5c757b1e044da2171f520bcd562d042`.
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
bash experiments/2026-07-14-005-pdt-query-metadata/benchmark.sh
bash experiments/2026-07-14-005-pdt-query-metadata/proof-checks.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-005-pdt-query-metadata/callgrind-candidate.out \
  target/release/eprover --auto --silent --cpu-limit=600 \
  --memory-limit=2048 --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
```

The repository-wide gates used the freshly rebuilt Windows binary and native
WSL comparison:

```powershell
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

## Implementation

`PrefixQueryMetadata` computes a term's function code, DB-variable property,
phony-application/lambda class, optional head, type UID, and cached standard
weight once. It derives the prefix token, first visible argument, and whether
the traversal descends from that snapshot. Free-variable token fields reuse
the already-computed type UID and weight.

Lambda head validation runs only in the lambda branch, while a malformed
zero-arity phony application retains the previous function-token fallback.
The generic `prefix_token` helper and recursive query builder remain
independent references. Debug builds recompute every legacy predicate and
assert exact agreement. A dedicated test covers constants, free variables, DB
variables, applied free/DB variables, DB lambdas, and the malformed fallback.

## Results

Seven alternating LUSK6 pairs measured a baseline median of `2.72` and a
candidate median of `2.66` user seconds, a 2.2% improvement. Median wall time
improved from `2.65` to `2.59` seconds, or 2.3%. Raw timings are retained at
`.artifacts/experiments/2026-07-14-005-pdt-query-metadata/alternating-times.txt`.

Matched Callgrind runs execute `22,460,989,749` baseline instructions and
`22,351,336,038` candidate instructions. The reduction is `109,653,711`
instructions, or 0.49%. Raw profiles are retained as `callgrind-baseline.out`
and `callgrind-candidate.out` in the ignored experiment artifact directory.

The paired long-search checks are:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 42.80 s | 43.35 s | Unsatisfiable |
| GEO288+1 | 51.35 s | 51.68 s | Theorem |

HEN011 retains exact `265,284` processed, `1,062,557` generated, and
`1,022,255` rewrite-step counters. GEO288 retains exact `10,215` processed,
`128,583` generated, `127,990` paramodulation, and `34,170` rewrite-step
counters. Recursive subsumption-call counters differ by one for HEN011 and two
for GEO288, consistent with the documented allocation-sensitive leaf order.
Earlier pairs reversed the long-case timing direction, so these single samples
are semantic falsification checks rather than performance evidence.

The 50-case differential report is
`.artifacts/e-compare/20260714-140516-380418/`. This pre-review candidate
report's six mismatches are the established BOO020, resource-sensitive
GEO288/HEN011, synthetic one-second
LUSK6 limit, and normalized LUSK6ext/sledgehammer output cases. SWV851 agrees
in this loaded run. No new status, proof, or output mismatch class appears.

The five-run native report is
`.artifacts/e-compare/20260714-141805-781443-benchmark/`. This pre-review
candidate report's aggregate Rust/C wall ratio is `3.351`, down from `3.440`,
but the C medians also move
substantially. Rust LUSK6 wall/CPU medians are `2.680`/`2.92` seconds versus
`2.525`/`2.74` previously; LUSK6ext is `6.081`/`6.62` versus
`6.056`/`6.59`. Treat this loaded aggregate as noisy rather than independent
evidence of a speedup.

## Falsification Checks

- All 33 focused PDTree tests pass, including explicit higher-order metadata
  parity and exact iterative-versus-recursive query cells and spans.
- Strict all-target, all-feature Clippy passes with pedantic warnings denied.
- The full suite passes 4,058 library tests and 3 integration tests, with all
  binary targets clean.
- LUSK6 improves in alternating exact-binary pairs and deterministic
  instruction count.
- HEN011 and GEO288 preserve status and principal saturation counters; their
  single-sample timing direction is inconclusive.
- The 50-case report contains no new mismatch class.
- Both experiment scripts pass `bash -n` and resolve paths from their own
  experiment directory.

## Conclusion And Limits

The one-pass metadata snapshot is accepted because it reduces deterministic
instructions and paired LUSK6 CPU time while preserving exact
query metadata and proof behavior. The standard native benchmark was noisy
and does not independently confirm the gain.

C's term predicates are direct-field macros and therefore cheaper than Rust's
current method/borrow boundary, but the C query path also repeats overlapping
classification across traversal and edge selection. A later C cleanup could
snapshot class, head, and first visible argument once without giving up direct
argument-array access. C also redundantly reevaluates invariant root weight in
`PDTreeInsertTerm` and `PDTreeSearchInit`; assertion builds can turn those
checks into repeated recursive weight walks. Overall Rust/C performance parity
and the higher-order materialized matching fallback remain open.
