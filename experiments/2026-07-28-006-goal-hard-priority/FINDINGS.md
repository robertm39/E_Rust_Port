# Experiment 331: Goal hard priority at larger resources

## Question

Experiment 330 rejected explicit layered clause selection but found an
unexpected simpler signal: the existing `PreferGoals` hard-priority control
solved three five-second held-out problems that the global age/weight baseline
and scalar goal-relevance control did not solve.

Does that gain generalize to entirely unobserved problem families, broader CASC
categories, and a four-times-larger CPU budget without trading away baseline
coverage?

The preregistered decision rule evaluates the 20-second budget. Goal hard
priority advances only if it has a net held-out coverage gain of at least two,
or identical coverage with a paired median CPU ratio no greater than 0.8,
together with zero contradictory statuses and zero queue-schedule fairness
violations.

## Fresh-family design

The exact prior observation boundary is pinned in
[`prior-selection.json`](prior-selection.json). It records Experiment 330's
canonical contract ID, raw contract SHA-256, manifest SHA-256, 44 selected
problem IDs, and the 18 families those problems expose.

This experiment selects only CASC-30 test-split problems whose family is absent
from that record. Up to four evenly spaced problems are selected from each
broader category:

| Category | Division | Fresh family or families | Problems |
| --- | --- | --- | ---: |
| FEQ | FOF equality | `SWX` | 4 |
| ICU | intuitionistic | `EEE` | 4 |
| SLH | higher-order Sledgehammer | `Digit_Expansions`, `Interpolation_Polynomials_HOL_Algebra` | 4 |
| TEQ | THF equality | `ANA` | 3 |
| TFE | typed first-order | `ANA` | 4 |
| UEQ | unit equality | `MVA` | 4 |

The final 23 problems span six families and are strictly disjoint from every
family observed in Experiment 330. This is a confirmatory set: no selector is
chosen on it.

Three fixed strategies are compared:

1. the global refined-weight/FIFO age/weight baseline;
2. the existing `PreferGoals` hard-priority control;
3. scalar conjecture-relative symbol weighting.

Every strategy uses `KBO6`, 1,536 MiB of memory, four concurrent workers, and
two repetitions. The short budget is 5 seconds soft / 7 seconds hard; the
larger budget is 20 seconds soft / 23 seconds hard. The 23 problems, three
strategies, two budgets, and two repetitions produce 276 coordinates.

## Provenance and validation

The authoritative run used normal-profile Ubuntu 24.04 runner
`e-rust-codex-260728-141450-4212` (run ID `260728-141450-4212`) and source
snapshot SHA-256
`c35d53b51e919cf5a944f3772e6780f7c24ccc9ef062e11d76d16ba500e04570`.

The immutable CASC-30 manifest SHA-256 was
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`.
The ignored corpus archive SHA-256 was
`efcebc55298d4c6770113c095e8cefdd77b9e8cbe3afa3078201f541893d1a7d`;
safe extraction reverified all 2,901 problems and 2,425 axioms. The release
binary SHA-256 was
`bfa6905a29c80c50420279ded641d46f0517de03ea85a9f4c28140a0c9065ea0`.

The final canonical contract ID is
`0bad4c08f71d0fe1ff0e90c4f7f1780bba1dae6d0f152c41dfb7347c8ad42d4b`.
All 276 coordinates completed. An identical second invocation independently
reverified every result/stdout/stderr/telemetry hash and reported
`276 resumed` in 1.2 seconds.

The complete source change was already validated by comprehensive run
`.artifacts/linode/260728-122336-6b18/`: 4,438 library tests, strict formatting
and Clippy, native and Windows GNU x64 builds, clean FOL/HO C builds, 50 main
and 216 tool compatibility cases with zero unexpected differences, ten
behavior-matching benchmarks at `1.0693048764x` C wall time, and Callgrind
smoke.

The final ignored raw artifact is
`.artifacts/goal-hard-priority/goal-hard-full.tar.gz`. It is 6,224,674 bytes
with SHA-256
`6c50e9a316c370f59eb4f45a8c12f1afc076d61543e2537c0aa375278a456166`
and contains all raw runs, the verified analysis, and the 276/276 resume
transcript.

## Results

The complete verified tables and problem-level comparisons are in
[`RESULTS.md`](RESULTS.md); the machine-readable result is
[`results-summary.json`](results-summary.json).

At five seconds, the global baseline and goal hard priority both reproducibly
solve `ANA127^1` and `EEE001+1`. Neither has a unique solve. Scalar goal
relevance additionally solves `SLH0044^1`.

At twenty seconds, all three strategies reproduce the exact same coverage:

- `ANA127^1` in TEQ;
- `EEE001+1` in ICU;
- `SLH0044^1` in SLH.

Goal hard priority therefore has zero gains, zero baseline losses, and a net
coverage gain of zero. Its paired median CPU ratio against baseline on the
common twenty-second solves is `1.047415`, a 4.7% regression rather than the
required 20% improvement. The corresponding short-budget ratio is `0.988121`,
also far from the efficiency threshold.

Across all 276 runs there are zero contradictory statuses and zero schedule
fairness violations. There are 203 valid telemetry records, zero invalid
records, and 73 hard-stop paths with no telemetry file. Missing telemetry is
excluded from metric aggregation.

## Decision

Reject goal hard priority as a general production selector. The original three
short-budget wins do not generalize beyond their observed families. On wholly
fresh families, hard priority matches baseline coverage at both budgets and is
slower on paired common solves at the larger budget.

No additional Bead is opened for scalar goal relevance from this experiment.
Its one short-budget SLH lead disappears at twenty seconds, where all strategies
have identical coverage. That isolated timing effect is insufficient to justify
another tuning dimension.

## Limits

Strictly excluding all prior families leaves only six fresh families in the
selected broader categories. Several categories therefore represent one family,
and the confirmatory set contains no fresh satisfiable/non-theorem family. The
20-second budget is four times the discovery budget but remains below the
official 240-second problem limits in the pinned CASC-30 manifest.

These limits prevent a universal claim that goal priority can never help. They
do support the narrower engineering decision required here: the observed
five-second signal is not robust enough to justify adopting or further tuning
the selector.

## Reproduction

After provisioning and synchronizing an Ubuntu 24.04 runner, separately
upload/hash-check/extract the ignored corpus and build the release binary:

```text
python3 experiments/2026-07-28-006-goal-hard-priority/run.py \
  --manifest benchmarks/casc_2025_manifest.jsonl \
  --problem-root /opt/e-rust-port/source \
  --binary target/release/umlaut \
  --prior-selection experiments/2026-07-28-006-goal-hard-priority/prior-selection.json \
  --output-root /opt/e-rust-port/goal-hard-full
```

Run the identical command again to prove exact resume, then analyze:

```text
python3 experiments/2026-07-28-006-goal-hard-priority/analyze.py \
  --run-root /opt/e-rust-port/goal-hard-full \
  --json-output results-summary.json \
  --markdown-output RESULTS.md
```
