# Blocked-clause and definition-oriented preprocessing evaluation

Bead: `E_Rust_Port-9jt.7.1`

## Outcome

Keep blocked-clause elimination, singular predicate elimination, and goal
definitions available as explicit default-off compatibility behavior. Do not
add any of them to generated schedules from this evidence.

All three passes were already production-connected before this study. The
generated schedules enable none of them, so the experiment added only opt-in
transformation telemetry and measured the existing implementations. No
logical transformation, search schedule, or command-line default changed.

Each candidate passed the correctness gate, fired on more than the minimum
four held-out coordinates, preserved the baseline solve set, and stayed below
the material-regression thresholds. None added a unique solve or met the
preregistered common-solve CPU threshold of `0.95`:

| Candidate | Active held-out coordinates | Candidate/baseline CPU | Generated | High-water | Maximum RSS | Decision |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| BCE | 10 | `0.987599` | `1.0` | `1.0` | `0.951625` | retain default-off |
| Predicate elimination | 12 | `1.005064` | `1.0` | `1.0` | `0.956224` | retain default-off |
| Goal definitions | 14 | `0.979426` | `1.0` | `1.0` | `0.954859` | retain default-off |

The three common reproducible solves were `NUN086+2`, `NUN134-1`, and
`REL005-1`. Baseline and every candidate solved exactly those three problems.

## Static inventory and implementation

The pre-measurement inventory found production implementations for formula
definition introduction, equation-definition unfolding, BCE, singular
predicate elimination, goal definitions, SInE, relevance pruning, formula
splitting, and FOOL/ITE/LET/lambda lowering. Generated schedules contained
zero occurrences of:

- `bce: true`;
- `pred_elim: true` or predicate gate recognition; and
- positive, negative, or recursive-subterm goal definitions.

That evidence rejected a speculative new transformation prototype. The
experiment instead added these fields under the existing opt-in
`input_funnel.transformations` telemetry object:

- `blocked_clause_elimination.removed`;
- `predicate_elimination.removed`;
- `predicate_elimination.generated`; and
- `goal_definitions.added`.

A zero remains deliberately neutral: the pass may be disabled, inapplicable,
or enabled without finding a candidate. The recorded strategy disambiguates
those cases.

The implementation retains each pass's existing proof boundary. BCE archives
removed originals, predicate resolvents retain `DC_PE_RESOLVE`, and goal
definitions retain `DC_INTRO_DEF`.

## Correctness and provenance

Exact end-to-end telemetry regressions passed:

- the BCE fixture removed two clauses and reported zero predicate/goal
  activity;
- the predicate fixture removed one clause and generated one resolvent; and
- the goal-definition fixture added one definition.

The focused BCE, predicate-elimination, goal-definition, SInE, splitting, and
formula-CNF suites passed on Ubuntu 24.04 with all Cargo features. Formatting,
all-target/all-feature Clippy with warnings denied, and the optimized
all-feature release build also passed before measurement.

The integrity-pinned ProofCheck 1.0 path selected 20 representative proof
claims. It verified 15, reported five explicit `coverage_gap` results, and
rejected zero:

- all four strategies received `Unknown` on the synthetic `bce-proof`
  artifact;
- `goal_defs/NUN134-1` received `Unknown` because the checker does not certify
  that goal-definition proof shape;
- no artifact received `VerifiedBad` or a structural rejection.

The preregistered proof gate requires at least one independently verified,
transformed proof for every candidate that produces one. The verified
witnesses were:

| Candidate | Verified transformed witness |
| --- | --- |
| BCE | `NUN086+2` |
| Predicate elimination | `predicate-elimination-proof` |
| Goal definitions | `goal-definitions-proof` |

The first verifier implementation accidentally required all 20
representatives to verify and restricted BCE validity to its differential
fixture. After observing the explicit coverage gaps, the wrapper was corrected
to implement the preregistered rule: coverage gaps remain non-verifications,
any `rejected` verdict is fatal, and candidate validity may use any verified
transformed representative. No workload, threshold, result, proof, or prover
binary changed. The final proof report id is
`794147d79081186ca0ea9b11392a40c3b28d80e4d79ebde3cc0ee3b5ed046fa4`.

All 138 candidate/baseline status pairs matched exactly, with no proof/model
polarity disagreement.

## Frozen execution

The measured Rust source revision was
`23a8a9700dffb18df57502cb600accaee3513887`. A later commit changed only the
proof-validation and analysis wrappers.

| Artifact | SHA-256 |
| --- | --- |
| Optimized all-feature Umlaut binary | `5ddef5ac691a078e015d21e79a7ecc31e628da26353fca82b4c6c095bcb785a3` |
| Frozen CASC manifest | `31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d` |
| Frozen targeted manifest | `71192e97c9a97f6b6331ef269228910183571818723db2ad5aa1253ffe83100d` |
| Frozen CASC corpus description | `ec86e63151524c04d74d12d6764ea84433213f5e35318515f0ccd7ceccd50d50` |
| Uploaded CASC corpus archive | `f519c5da9f0d723bd8aeadadac472547f87c2a657addc6feb818c7a6be45833b` |

Every strategy used the same fixed clause-selection heuristic, KBO6, and
forward-demodulation level 2. At most four jobs ran concurrently with 1,536
MiB per job.

The frozen phases were:

- three differential problems, four strategies, two repetitions, and
  5/8-second soft/hard CPU limits: 24 runs; and
- 20 candidate-blind CASC test problems, four strategies, two repetitions,
  and 10/13-second soft/hard CPU limits: 160 runs.

The final contract ids are
`a7c81c2cfb2c204d097121e5f4c432f18383b40ecd8b9196f14c7df80be4d248`
for the differential phase and
`6795b3c153931161de30d83f0004e34dfcd010eb8f1a1c92695803be21829d5d`
for the held-out phase. Unchanged replays resumed 24/24 and 160/160
coordinates without executing a job.

## Transformation reach

Held-out telemetry measured:

| Candidate | Active problems | Removed | Generated or added |
| --- | ---: | ---: | ---: |
| BCE | 5 | 6,538 | 0 |
| Predicate elimination | 6 | 3,986 | 4,306 |
| Goal definitions | 7 | 0 | 30 |

The differential phase established enabled/disabled behavior and produced
the required transformed proof witnesses:

| Candidate | Active coordinates | Removed | Generated or added |
| --- | ---: | ---: | ---: |
| BCE | 2 | 4 | 0 |
| Predicate elimination | 4 | 10 | 6 |
| Goal definitions | 2 | 0 | 2 |

Eight held-out records lacked terminal telemetry. They were exactly the two
repetitions of `PLA007-10` under each strategy, all with expected
`ResourceOut` status and return code 8. No successful or otherwise unexpected
result lacked telemetry.

## Decision and limits

The final report decision is `retain_explicit_default_off` for all three
candidates, report id
`087ca97086d69404c000dc3c6895e7fd5df1cbcdc370902ceb96f942b935c667`.
There is no generated-schedule follow-up from this experiment.

The sample has only three common held-out solves and two repetitions, so the
small CPU differences are not broad speed estimates. Transformation reach is
real but family-dependent. Predicate elimination removed many clauses while
generating slightly more, and goal definitions changed proof shape enough to
expose a checker coverage gap. Future work may reconsider trigger policies
with a larger, separately frozen corpus, but it should not reinterpret these
results as evidence for enabling the passes by default.

## Evidence

The compact tracked summary is `results-summary.json`, SHA-256
`ad1576d50fb5f6dcdb2cd48aa209d24f4ba4247a82788dce5eb9ad7f89d5bcbc`.
It embeds the final proof-validation report and exact per-phase aggregates.

The complete ignored archive is 21,224,572 bytes at
`.artifacts/experiments/2026-07-29-015-preprocessing-evaluation/preprocessing-results.tar.gz`,
SHA-256
`6da8483fe84a98e328e1d08e8a457451a962b1ca338204934cddf2389ef4f733`.
It contains contracts, commands, stdout, stderr, telemetry, proof objects,
checker reports, and final analyses.
