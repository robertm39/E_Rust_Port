# Compact lazy TSM target features

## Question

Can the default lazy `TSMRWeight` retain the target problem with C-shaped
lifetime cost without introducing a self-reference or eagerly loading the TSM
knowledge base?

Massif attributed a large part of the remaining parse/CNF peak to a deep
`ClauseSet::clone`. C retains a non-owning `ProofState_p` for lazy TSM
initialization, whereas Rust cloned every target clause so that the evaluator
could outlive the weight-parser call. The default strategy does not select TSM
on these inputs, so the clone remained live solely as unused deferred state.

## Setup and implementation

The saved baseline is the native Linux release from commit `ff23334a`, copied
before editing to:

```text
.artifacts/experiments/2026-07-16-014-tsm-target-features/baseline/eprover
```

The candidate captures the target's small numerical `Features` value while the
proof-state axioms and signature are available. Production weight-parse
contexts pass that snapshot to lazy `TSMWeight` and `TSMRWeight` evaluators.
The first evaluation loads the knowledge base, selects the examples using the
snapshot, and then drops the deferred target. Existing standalone parser APIs
retain the owned-clause fallback for compatibility, and also drop it after
initialization. TSM knowledge-base I/O remains lazy, including the unused
default strategy and its `E_KNOWLEDGE` lookup.

This is the safe owned equivalent of C's borrowed proof-state pointer. It does
not add an unsafe self-reference and does not weaken the evaluator lifetime.

The final WSL release candidate was built from the working tree with:

```bash
cargo build --locked --release --bin eprover
```

Five-run interleaved scaling used the existing harness on both repeated-term
and unique-symbol corpora:

```bash
bash experiments/2026-07-16-011-clause-info-owner-layout/benchmark.sh \
  "$c_binary" \
  .artifacts/experiments/2026-07-16-014-tsm-target-features/baseline/eprover \
  target/release/eprover \
  .artifacts/experiments/2026-07-15-009-formula-owner-memory-scaling/corpus \
  .artifacts/experiments/2026-07-16-010-unique-symbol-parser-scaling/corpus \
  .artifacts/experiments/2026-07-16-014-tsm-target-features/raw/scaling-final.csv

python3 experiments/2026-07-16-011-clause-info-owner-layout/analyze.py \
  .artifacts/experiments/2026-07-16-014-tsm-target-features/raw/scaling-final.csv
```

Paired Massif profiles used the repeated-owner 1,000- and 20,000-owner inputs:

```bash
valgrind --tool=massif --time-unit=B \
  --massif-out-file="$massif_output" \
  "$binary" --cnf --silent --output-file=/dev/null "$problem"
```

## Results

The final candidate reduced median RSS at the discriminating 20,000-owner
scale on both symbol shapes:

| Shape, owners | Implementation | Wall median (s) | CPU median (s) | RSS median (KiB) |
| --- | --- | ---: | ---: | ---: |
| Repeated, 20,000 | C | 0.320 | 0.090 | 34,864 |
| Repeated, 20,000 | Baseline | 0.240 | 0.180 | 68,376 |
| Repeated, 20,000 | Candidate | 0.260 | 0.190 | 62,292 |
| Unique, 20,000 | C | 0.520 | 0.140 | 51,272 |
| Unique, 20,000 | Baseline | 0.570 | 0.520 | 101,352 |
| Unique, 20,000 | Candidate | 0.590 | 0.510 | 95,276 |

Repeated-owner RSS fell 6,084 KiB (8.90%), and unique-owner RSS fell 6,076
KiB (5.99%). The Rust/C RSS ratios fell from 1.961 to 1.787 and from 1.977 to
1.858, respectively. CPU medians moved by +0.01 seconds on repeated owners and
-0.01 seconds on unique owners, within this short-run measurement resolution.

Massif independently confirmed a smaller live allocation peak:

| Owners | Implementation | Useful heap (B) | Extra heap (B) | Total (B) |
| ---: | --- | ---: | ---: | ---: |
| 1,000 | Baseline | 3,491,380 | 144,564 | 3,635,944 |
| 1,000 | Candidate | 3,173,715 | 128,605 | 3,302,320 |
| 20,000 | Baseline | 65,701,519 | 2,685,497 | 68,387,016 |
| 20,000 | Candidate | 60,549,261 | 2,365,523 | 62,914,784 |

Total live heap fell 333,624 bytes (9.18%) at 1,000 owners and 5,472,232
bytes (8.00%) at 20,000 owners. The 20,000-owner baseline profile contains 62
detailed-snapshot frames under `ClauseSet as core::clone::Clone` in the lazy TSM
parse path; the final candidate contains none.

## Falsification checks

- A focused real-KB regression verifies that the signature-aware path retains a
  feature snapshot before first evaluation, produces the same expected clause
  value, and releases the snapshot afterward.
- The existing owned-axiom lazy path remains covered and now explicitly verifies
  that its fallback target is released after initialization.
- The weight-function dispatcher test exercises both `TSMWeight` and
  `TSMRWeight` through a signature-bearing production-shaped context.
- The candidate did not eagerly load the knowledge base: the deferred target is
  stored independently of KB parsing, and the same first-evaluation boundary is
  retained.
- Both repeated-term and unique-symbol corpora reduced RSS, ruling out term
  sharing as the sole explanation.
- Massif and process RSS independently agree on direction, and the clone stack
  disappears rather than merely moving to another peak snapshot.
- The five-run analyzer accepted all 150 final samples, with five successful
  runs per implementation/shape/count group.
- Preliminary `scaling.csv` and `candidate-repeated-*.massif` measurements were
  taken before Clippy required both deferred enum variants to be boxed. They are
  retained only for auditability. The decision uses `scaling-final.csv` and
  `candidate-final-repeated-*.massif`, generated from the exact final layout.

## Repository-wide acceptance

The final candidate passed:

- `git diff --check` and `cargo fmt --all -- --check`;
- `cargo check --all-targets --all-features`;
- `cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic`;
- all 4,090 library tests, every binary target, and all three schedule
  integration tests under `cargo test --all-targets --all-features`;
- locked optimized Linux and Windows `eprover` builds.

The standard 50-case C-vs-Rust report is
`.artifacts/e-compare/20260716-043232-229293/comparison.json`. It completed all
cases with the same six stable mismatches as the accepted baseline:
`BOO020-1.p`, `GEO288+1.p`, `HEN011-2.p`, `SWV851-1.p`, `sledgehammer.p`, and
the synthetic CPU-limit case. No mismatch name or field was added. The prior
intermittent normalized-output difference for `LCL365-1.p` did not recur.

The standard five-run benchmark report is
`.artifacts/e-compare/20260716-044432-421712-benchmark/benchmark.json`. Its
aggregate Rust/C wall-time ratio is 3.304x, slightly better than the preceding
3.314x result. `BOO020-1.p` retained its known differing resource outcome and
was excluded from the aggregate. The sustained large cases remained
behaviorally matched:

| Case | C wall median (s) | Rust wall median (s) | Ratio | C max RSS (KiB) | Rust max RSS (KiB) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `LUSK6.lop` | 0.978 | 2.700 | 2.762x | 119,680 | 257,760 |
| `LUSK6ext.lop` | 2.353 | 5.998 | 2.549x | 231,840 | 503,908 |

## Conclusion and limits

Retain the compact lazy TSM target. It restores C's intended low-cost borrowed
target lifetime with safe owned Rust data, removes a measured deep-clone peak,
and produces repeatable 6-9% focused memory reductions without a compatibility
or aggregate-performance regression.

This is a partial improvement, not completion of the port's performance work.
Focused Rust RSS remains 1.79-1.86 times C at 20,000 owners, and the standard
benchmark remains 3.304 times C. Clause storage, term-bank ownership, evaluation
indexes, and remaining proof-state duplication still contribute to the gap.
