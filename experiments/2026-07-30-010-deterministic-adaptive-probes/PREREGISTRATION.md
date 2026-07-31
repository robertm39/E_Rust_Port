# Preregistration: deterministic adaptive probes

## Question and prior limitation

This experiment addresses Bead `E_Rust_Port-9jt.3.10`.

Can a deterministic processed-clause checkpoint plus an atomic pre-input
telemetry checkpoint make the registered clause-growth decision observable
before resource termination, at low overhead, and does the now-observable
branch policy improve a fresh family-held-out search comparison?

Experiment 020 used a one-soft-CPU-second probe. Fourteen of sixteen test
adaptive probes reached the kernel hard stop before schema-v1 telemetry was
written, so the policy mostly took its goal fallback. Experiment 009 later
showed that bounded processed-clause returns write complete telemetry without
changing proof semantics. This follow-up tests that boundary directly.

No production schedule or prover default changes in this experiment.

## Frozen source and corpus

The measured source revision is
`f03259698d81e8fbc25c8b64deb4e7c35e3ffd77`. This revision atomically writes a
schema-v1 `checkpoint` before input processing and atomically replaces it with
the ordinary `final` record. It changes no proof-search choices or defaults.

An initial train-only diagnostic at the preceding revision showed that
NLP262+1 exhausted its hard CPU limit during preprocessing, before the
processed-clause limit could fire. No validation or test result had been
opened. The diagnostic contract and results are excluded, the implementation
was corrected on train evidence only, and this experiment is restarted and
re-frozen at the revision above before a fresh train run.

The source manifest is:

```text
benchmarks/casc_2025_manifest.jsonl
SHA-256 31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d
```

`select_corpus.py` reads metadata only. It selects four FNE and four FEQ
theorem problems per split, one from each frozen available difficulty band:
train uses q1-q4 for both categories; validation uses FNE q2-q5 and FEQ q1-q4;
test uses FNE q1, q2, q3, and q5 and FEQ q1-q4. The asymmetric FNE bands are
the candidate-blind availability correction recorded before corpus generation:
validation has no eligible fresh q1 problem and test has no eligible fresh q4
problem. Selection uses the manifest's whole-family `holdout_split`, size
bounds 200 through 100,000 bytes, the frozen hash salt
`umlaut-adaptive-probe-observability-v1`, and excludes every exact problem in
experiment 018. The result has eight train, eight validation, and eight test
problems and disjoint source families across splits.

The frozen generated corpus SHA-256 is
`5b3b2bf5c86bf6537742705a49a15e224dd1062b9d5ad96d56913e2dfdddc923`.

No prover output may be inspected before this document, the generated corpus,
controller, analyzer, validator, and focused tests are complete. Train may
exercise correctness and report diagnostics but may not retune the probe,
threshold, strategies, budgets, metrics, or decision rule. Validation may not
retune test.

## Fixed strategies and branch rule

Both strategies use KBO6 and a two-queue age/weight heuristic.

Global:

```text
(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),
 1*FIFOWeight(ConstPrio))
```

Goal:

```text
(5*Refinedweight(PreferGoals,2,1,1.5,1.1,1.1),
 1*FIFOWeight(ConstPrio))
```

The adaptive rule is inherited unchanged from experiment 020:

- compute `clause_growth` as generated non-trivial clauses divided by at least
  one processed non-trivial clause;
- choose goal when `clause_growth >= 64`;
- otherwise choose global; and
- fall back to goal only for missing, unknown, incomplete, or fewer than 64
  processed non-trivial clauses.

There is one branch and at most one restart. The policy cannot oscillate.

## Deterministic probe and resources

The telemetry-enabled probe uses:

```text
--processed-clauses-limit=256
--soft-cpu-limit=2
--cpu-limit=4
--memory-limit=1536
--search-telemetry=PATH
```

The processed-clause limit is the primary signal checkpoint. The atomic
pre-input record preserves a complete schema-v1 fallback if preprocessing
reaches the hard stop before saturation begins. The soft limit bounds unusually
expensive main-search work, while the hard limit gives graceful return and
final telemetry two additional CPU seconds. A controller wall limit, separate
process group, SIGTERM/SIGKILL escalation, and isolated temporary directory
prevent surviving workers.

Continuations receive three soft and five hard CPU seconds. Full contextual
arms receive five soft and seven hard CPU seconds. Every restart policy
therefore has the same configured 2 + 3 soft-CPU-second ceiling and identical
process/preprocessing boundaries.

One otherwise identical probe omits only `--search-telemetry`. `/usr/bin/time
-v` supplies external CPU, wall, and maximum-RSS measurements for both overhead
arms. Both probes request ordinary `--print-statistics` output so their
processed-set sizes can be compared without using telemetry as its own
overhead clock.

## Arms and repetitions

Each split runs these seven arms:

1. `probe_without_telemetry`;
2. `probe_with_telemetry`;
3. `global_full`;
4. `goal_full`;
5. `static_global_restart`;
6. `static_goal`; and
7. `adaptive`.

Train runs once. Validation and test run twice. A probe proof ends that arm
without a continuation. The three restart policies use separately executed
but byte-identical telemetry probe commands apart from artifact paths.

Test requires a hash-valid accepted validation analysis. Completed coordinates
resume only after contract identity and every referenced artifact hash match.

## Proof and correctness gates

Primary races do not need to render a full proof. Every proof-status result is
immediately rerun alone with the exact strategy, input, checkpoint/budget, and:

```text
--tstp-out --proof-object=1 --force-deriv=2
```

The proof class must reproduce. The repository validation gate and ProofCheck
1.0 must accept the annotated TSTP proof against the untouched original
problem.

Correctness fails on a source/include/binary/script/contract hash mismatch,
unexpected satisfiable or counter-satisfiable status, proof-status result
without a verified replay, status mismatch between telemetry overhead probes,
external timeout, configured-budget violation, surviving process, temporary
residue, or adaptive branch outside the fixed rule.

## Measurements

Validation and test are reported separately:

- telemetry presence, schema validity, successful signal rate, and fallback
  reason for every non-proof telemetry-enabled probe;
- probe processed/generated counts and clause-growth/passive-pressure signals;
- paired telemetry/no-telemetry CPU, wall, and maximum-RSS ratios;
- status and processed-checkpoint agreement between overhead probes;
- every adaptive branch and repeat stability;
- reproducible, one-repeat, unique, and lost solve sets;
- paired adaptive/static CPU, wall, and RSS ratios on common solved
  coordinates;
- decision overhead;
- proof replay counts and hashes; and
- cancellation, cleanup, and every correctness failure.

The observability numerator is a non-proof telemetry-enabled probe with a
valid schema-v1 record. The denominator is every non-proof probe in
`probe_with_telemetry`, `static_global_restart`, `static_goal`, and `adaptive`.
Probe proofs are excluded because they do not require a branch decision.

## Frozen decision

`continue` toward an integrated prototype only if:

1. all correctness and proof gates pass;
2. validation and test each achieve at least 95% successful non-proof probe
   telemetry;
3. every adaptive branch is identical across repetitions;
4. telemetry/no-telemetry probe status and processed counts agree, and median
   CPU, wall, and peak-RSS ratios are each at most 1.05 in validation and test;
5. adaptive loses no reproducible solve versus either equal-budget restart
   comparator in validation or test; and
6. adaptive has at least two reproducible test-only solves versus both
   comparators, or validation and test each have at least four common solved
   repetition coordinates with both comparators and median CPU ratio at most
   0.95.

Otherwise `stop` this policy and keep it outside automatic schedules. Failure
of the 95% gate specifically rejects the processed-clause observability
mechanism; passing observability but missing efficacy rejects the current
clause-growth intervention, not telemetry itself.
