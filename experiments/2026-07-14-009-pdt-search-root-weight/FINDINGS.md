# PDTree Root Query Weight Reuse

## Question

Can `record_search_init` reuse the standard weight already stored in the root
query cell instead of evaluating `term_standard_weight` again, while preserving
PDTree query metadata, pruning, proof behavior, and performance?

## Setup

- Baseline commit: `9c7df69e` (`Borrow term arguments during tree comparison`).
- Exact saved Linux baseline:
  `.artifacts/experiments/2026-07-14-009-pdt-search-root-weight/baseline-eprover`.
- Baseline SHA-256:
  `b59290f051c2869062f0e195a22a8601bdcdfced237854bb1449bddee89b159c`.
- Candidate SHA-256:
  `7587e0362ee04d9e5fe7356dbdb621bbe69bfcf4392800e023730bf640de5850`.
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
bash experiments/2026-07-14-009-pdt-search-root-weight/benchmark.sh
bash experiments/2026-07-14-009-pdt-search-root-weight/proof-checks.sh
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-14-009-pdt-search-root-weight/callgrind-candidate.out \
  target/release/eprover --auto --silent --cpu-limit=600 \
  --memory-limit=2048 --detsort-rw --detsort-new \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop
```

## Implementation

`build_search_query` always emits the root first, and each query cell snapshots
the exact standard weight used by PDTree pruning. Search initialization now
reads `query[0].weight` through a checked `first()` access and stores that same
value in both the public search bookkeeping and `PdtSearchState`. A focused
assertion ties the state weight to the root query-cell weight.

This preserves the C-shaped standard-weight value and nonempty-query invariant.
It only removes the immediately repeated Rust evaluation after query
construction.

## Results

Seven alternating LUSK6 pairs measured a baseline median of `3.00` and a
candidate median of `2.98` user seconds, a 0.7% improvement. Median wall time
was `2.92` versus `2.99` seconds, with one candidate externally delayed to
`5.81` seconds; wall timing is therefore treated as inconclusive. Raw timings
are retained at `.artifacts/experiments/2026-07-14-009-pdt-search-root-weight/alternating-times.txt`.

Matched Callgrind runs execute `20,023,941,923` baseline instructions and
`20,021,308,767` candidate instructions. The reduction is `2,633,156`
instructions, or 0.013%. `record_search_init` itself falls from
`1,959,453,682` to `1,956,821,665`, an exact `2,632,017` instruction reduction
that isolates effectively the complete change.

The paired long-search checks are:

| Problem | Baseline user | Candidate user | Result |
| --- | ---: | ---: | --- |
| HEN011-2 | 54.40 s | 45.85 s | Unsatisfiable |
| GEO288+1 | 50.19 s | 50.62 s | Theorem |

HEN011 retains exact `265,284` processed, `1,062,557` generated,
`1,062,557` paramodulation, and `1,022,255` rewrite-step counters. GEO288
retains exact `10,215` processed, `128,583` generated, `127,990`
paramodulation, and `34,170` rewrite-step counters. GEO288's
allocation-sensitive non-redundant subcounter differs by six, within the
documented raw-address ordering behavior. These single timing pairs are
semantic checks, not timing evidence.

The final 50-case differential report is
`.artifacts/e-compare/20260714-192111-191604/`. It reports the six established
BOO020/SWV851 resource behaviors, resource-sensitive GEO288, normalized
LUSK6ext/sledgehammer output, and synthetic one-second LUSK6 limit classes,
plus HEN011 crossing the default resource boundary during that loaded run. A
focused 90-second rerun at
`.artifacts/e-compare/20260714-193735-494363/` completes HEN011 with zero
mismatches, so no new compatibility class remains.

The final five-run native report is
`.artifacts/e-compare/20260714-193830-797589-benchmark/`. Its aggregate Rust/C
wall ratio is `3.144`, versus `3.330` in the preceding report. LUSK6 records a
`2.491` ratio with Rust wall/CPU medians of `3.256`/`3.53` seconds; LUSK6ext
records `2.530` with Rust wall/CPU medians of `7.448`/`8.11`. Both C and Rust
absolute times shifted materially from the preceding report under the loaded
host, so the improved ratio is supporting context rather than evidence for
this small change. Exact alternating binaries and Callgrind remain the
acceptance measurements.

## Rejected Variant

A broader variant also replaced the query builder's separate arity borrow,
borrowed higher-order heads instead of cloning them, and added a non-owning
`Term::type_uid` accessor. It increased matched Callgrind instructions from
`20,023,941,923` to `20,145,397,975`, a 0.61% regression, and raised
`record_search_init` by `26,162,318` instructions. Those changes were removed;
they should not be retried as a group without line-level attribution.

## Falsification Checks

- All 33 focused `clauses::pdtrees` tests pass, including root query-weight
  identity and higher-order query metadata cases.
- LUSK6 preserves the proof result while reducing deterministic instruction
  count exactly where expected.
- HEN011 and GEO288 preserve status and principal saturation counters.
- The full all-feature suite passes 4,054 library tests and 3 integration
  tests; strict all-target, all-feature Clippy passes with pedantic warnings
  denied.
- The full differential plus focused HEN011 rerun leaves no new compatibility
  mismatch class.
- The broader borrowed-metadata experiment was rejected on deterministic
  instruction regression, despite its source-level appeal.
- Both experiment scripts pass `bash -n` and resolve paths from their own
  experiment directory.

## Conclusion And Limits

Root query-weight reuse is accepted because the query already owns the exact
pruning value, the implementation removes a redundant evaluation, and matched
Callgrind isolates the reduction to search initialization. The gain is small;
paired wall timing cannot resolve it reliably.

C `PDTreeSearchInit` evaluates `TermStandardWeight` once in an assertion and
again for assignment, while `PDTreeInsertTerm` repeats invariant root-weight
work across inserted nodes. A later C cleanup should snapshot and validate the
root weight once, but must preserve eta normalization, debug assertion
coverage, and the distinct per-node size-constraint value.
