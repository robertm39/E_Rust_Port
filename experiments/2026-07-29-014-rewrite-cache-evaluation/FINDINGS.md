# Shared rewrite-link and normal-form cache evaluation

Bead: `E_Rust_Port-9jt.7.4`

## Outcome

Retain Umlaut's full shared rewrite cache unchanged.

Umlaut already had the architecture proposed by the Bead: canonical terms
carry shared rewrite links, rewrite paths are reused across clauses, and rule
and full-normal-form dates invalidate stale negative results when the
demodulator set grows. This study added opt-in measurement and compared that
production behavior with a compile-time, proof-preserving recomputation
ablation.

The production cache passed the preregistered efficiency gate:

- all 6/6 representative proof claims were independently verified;
- all 90 paired coordinates had matching polarity, with no disagreement;
- the cache and recomputation builds each reproducibly solved the same three
  larger-budget CASC problems;
- cache/recomputation median CPU was `0.890751` on those common CASC solves;
- generated and high-water ratios were both `1.0`;
- the maximum-of-runs larger-budget RSS ratio was `0.958982`; and
- the rewrite-heavy common-solve CPU ratio was `0.847211`.

The cache added no unique solve in this small frozen sample, but it met the
alternative preregistered performance path. No cache policy, search schedule,
or command-line default changed.

## Evaluated implementation

The production implementation uses:

- canonical top and structural rewrite links stored in the term bank;
- restricted-link checks before a stored path is reused;
- rule and full-normal-form dates compared with the current demodulator date;
  and
- retained rewrite-chain ancestry for proof reconstruction.

The experiment added search-telemetry counters for link lookups, link hits,
followed edges, normal-form-date checks and hits, and uncached links. Counters
are enabled only while `--search-telemetry` is active.

The ablation is selected only at build time through
`UMLAUT_EXPERIMENT_REWRITE_CACHE_ABLATION=1`. It ignores persistent rewrite
links and normal-form-date fast returns, while retaining fresh intra-call
links needed to reconstruct every demodulator ancestor. It is deliberately
not a Cargo feature, so `--all-features` never changes prover semantics.

## Correctness and invalidation

Focused regressions cover the three acceptance boundaries:

- `plain_li_normalform_reuses_shared_link_and_records_cache_activity` checks
  reuse and counter activity;
- `newer_rule_epoch_invalidates_negative_normal_form_date` checks that a new
  rule invalidates a prior negative normal-form result;
- `newer_rule_extends_existing_shared_rewrite_chain` checks rule growth after
  an existing link; and
- `cached_shared_term_reuse_preserves_each_demodulator_ancestor` checks proof
  reconstruction across cached shared terms.

On Ubuntu 24.04, both normal and ablation focused rewrite suites passed. The
normal all-target/all-feature suite passed all 4,486 tests with zero failures.
Formatting, all-target/all-feature Clippy with warnings denied, focused
telemetry tests, and optimized release builds also passed.

ProofCheck 1.0 then verified one reproducible proof per solved category and
build:

| Phase | Category | Cache | Recompute |
| --- | --- | --- | --- |
| CASC | FEQ | `NUN086+2` | `NUN086+2` |
| CASC | UEQ | `NUN134-1` | `NUN134-1` |
| Rewrite-heavy | REWRITE | `COL042-8` | `COL042-8` |

All six returned `VerifiedGood`. The checker executable SHA-256 is
`92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`;
the pinned release archive SHA-256 is
`4c4c6f71f9d8235450c6889863963ba242249c2d8d63d0461ea3acb7814b6aaa`.
The proof-validation report id is
`2f5a2de81fb9e91b2644b98feae82ab8ee4adcfee2161e124d389513cc8fd590`.

## Frozen execution

The measured source revision was
`e3393c26c53ca9d59947963a221b309ef91655d6`. Later commits only added and
repaired the proof-validation wrapper.

| Artifact | SHA-256 |
| --- | --- |
| Production cache binary | `3656385cda4b785f008caa289189900636e8fd3036767aeed2f67cc2d682afb1` |
| Recompute ablation binary | `730d6f98e5ef7beca0f45949e3d061eca16308f221e4d55abf15499aa906a247` |
| Frozen CASC manifest | `31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d` |
| Frozen targeted manifest | `e7da67acb54c3db49aab53d9f339ff18d7fa0789aa9a8ba141ff08206456cd32` |
| Uploaded CASC corpus archive | `f519c5da9f0d723bd8aeadadac472547f87c2a657addc6feb818c7a6be45833b` |

Both builds used the same fixed clause-selection heuristic, KBO6, and forward
demodulation level 2. The Ubuntu 24.04 worker reported Linux 6.8, glibc 2.39,
and four CPUs. At most four jobs ran concurrently with 1,536 MiB per job.

The frozen phases were:

- 20 candidate-blind CASC test problems, two builds, two repetitions, and
  5/7-second plus 20/23-second soft/hard CPU budgets: 160 runs; and
- five rewrite-heavy examples, two builds, two repetitions, and a
  30/33-second soft/hard CPU budget: 20 runs.

Rerunning the unchanged controllers resumed 160/160 CASC and 20/20 targeted
coordinates without executing a job. The final contract ids are
`6c4c6b6e78a1839454fd4d86d0df2b185e1457a72c39aa1535fadc14937ba86d`
for CASC and
`15cb32c2997c911bf3b243df3713b694cf33e5b42b91df2c012ca650ca9f65ed`
for the rewrite-heavy phase.

## Results

Ratios are production cache over recomputation. Common-solved metrics include
only problems solved in both repetitions by both builds.

| Phase | Budget | Cache solves | Recompute solves | Common CPU | Generated | High-water | Term storage | Maximum RSS |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| CASC | larger | 3 | 3 | `0.890751` | `1.0` | `1.0` | `0.999827` | `0.958982` |
| CASC | short | 3 | 3 | `0.897382` | `1.0` | `1.0` | `0.999827` | `1.118367` |
| Rewrite-heavy | targeted | 2 | 2 | `0.847211` | `1.000915` | `1.000589` | `1.000734` | `1.007123` |

The three common CASC solves were `NUN086+2`, `NUN134-1`, and `REL005-1`.
The two common rewrite-heavy solves were `COL042-8` and `LUSK6`. Neither build
had a unique solve.

The short-budget maximum-RSS ratio exceeded 1.05, but the preregistered memory
gate uses the larger-budget maximum, which favored the cache at `0.958982`.
The common-solve short-budget median RSS ratio was `1.0`. Timeout-limited
all-run totals are retained in the machine-readable report and are not
interpreted as speedups.

## Cache activity

The direct shared-link hit rate was low, but normal-form dates eliminated a
large amount of repeated work:

| Workload | Link hit rate | NF-date hit rate | Cached rewrite fraction | Saved traversal proxy |
| --- | ---: | ---: | ---: | ---: |
| CASC larger | `0.037540` | `0.843960` | `0.855017` | 470,505,887 |
| CASC short | `0.036371` | `0.809318` | `0.740158` | 146,453,047 |
| Rewrite-heavy | `0.047784` | `0.795771` | `0.789889` | 88,554,478 |

The saved-traversal proxy is followed rewrite-link edges plus successful
normal-form-date checks. It is an operation-count proxy, not a CPU-cycle
estimate. The low direct-link rate alone does not trigger a selective-cache
follow-up because the full cache already passed the higher-priority retention
gate, with most measured savings coming from normal-form dates.

Twelve results lacked terminal telemetry, but every one was an expected
`ResourceOut`; there was no missing telemetry on a successful or otherwise
unexpected outcome. All 90 build-paired statuses matched exactly.

## Decision and limits

The report decision is `retain_full_shared_rewrite_cache`, report id
`b1cc9a9fadc407373a0cd052b1d0f31398942bb2f6a26f48db124d2cb9b864a9`.
The existing production cache remains enabled, and the experimental build
switch remains an evaluation mechanism rather than a user-facing option.

This is evidence for the current fixed strategy and frozen samples, not a
claim about every schedule or TPTP family. There were only three common CASC
solves and two rewrite-heavy solves, with two repetitions each. The ablation
also changes the search trajectory because a valid shared rewrite shortcut
can affect when work is performed. Those limits make the exact speedup
estimate uncertain, but they do not weaken the soundness result or the
preregistered retain decision.

## Evidence

The compact tracked summary is `results-summary.json`, SHA-256
`39d324849d4760ac7907b36b545fdecec8075f85bd23c0f0c125bbf2a97d3ad1`.
It embeds the proof-validation report and exact per-phase aggregates.

The complete ignored evidence archive is 23,397,005 bytes at
`.artifacts/experiments/2026-07-29-014-rewrite-cache-evaluation/rewrite-cache-results-final.tar.gz`,
SHA-256
`4c154b26822193578198a4bb54a3caa554076df52aef51476bd65392ebaa6fe6`.
It contains contracts, commands, stdout, stderr, telemetry, proof objects,
checker reports, run logs, and both preliminary and final analyses.
