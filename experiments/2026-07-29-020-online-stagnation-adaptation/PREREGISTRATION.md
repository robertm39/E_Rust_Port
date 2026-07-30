# Preregistration: bounded online stagnation adaptation

## Question and hypothesis

Can a deterministic telemetry decision after a short saturation probe improve
held-out coverage or efficiency over an equal-budget static two-strategy
portfolio?

The hypothesis is that global age/weight searches with unusually high
non-trivial clause generation per non-trivial processed clause are experiencing
clause-growth stagnation. Restarting those searches with goal hard priority may
recover coverage, while retaining global age/weight for low-growth searches
may avoid the losses of switching every problem.

## Frozen source and prior evidence

The measured source revision is
`42bfa440729dfe214042020898f7ba87fed7ab4f`.

The policy shape was chosen from architecture and already published evidence,
not from this experiment's outcomes:

- search telemetry schema version 1 exposes processed, generated, passive-set
  high-water, queue, CPU, and resident-memory aggregates;
- experiment 330 found goal hard priority complementary to global age/weight
  on a prior FNE/FEQ/EPS/SLH study;
- replay of experiment 330 validation telemetry found global
  generated/processed ratios ranging from below 1 to above 380, supporting a
  small logarithmic threshold grid rather than one fitted cut point.

No result from the new EPU/UEQ corpus may be inspected before this document,
the corpus selector, controller, analyzer, and tests are committed.

## Corpus and leakage control

`select_corpus.py` selects only EPU and UEQ problems from the immutable CASC-30
manifest whose SHA-256 is
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`.
Selection uses only category, family, size, expected class, problem identity,
and the frozen salt `umlaut-online-stagnation-v1`; it does not read outcomes.

The exact family partition from experiment 018 is reused:

- calibration: CSR, GEO, GRP, HWV, KLE, LAT, MGT, SWB, SWW, SYN;
- validation: LCL, PUZ, ROB, SWV;
- test: NUN, PLA, REL, SEU, SWX.

The splits are therefore whole-source-family disjoint. Calibration and
validation contain four EPU and four UEQ problems. CASC-30 contains only two
eligible test-family EPU problems, so test contains both plus six
deterministically selected UEQ problems. Every selected problem is expected
unsatisfiable. Problem and include hashes are verified before execution.

Threshold selection uses calibration only. Validation is not used to retune.
Test is not run until the calibration selection and validation report have been
written and hash-pinned.

## Strategies and policy arms

All searches use `KBO6`, PCL proof output, a 1,536 MiB memory limit, and the same
release binary.

The global heuristic is:

```text
(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))
```

The complementary goal-priority heuristic changes only the refined-weight
priority function:

```text
(5*Refinedweight(PreferGoals,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))
```

Calibration captures five primitive arms for replay:

1. global for five CPU seconds;
2. goal priority for five CPU seconds;
3. global probe for one CPU second;
4. fresh global continuation for four CPU seconds;
5. fresh goal-priority continuation for four CPU seconds.

Validation and test execute five independent policy arms:

1. `global_full`: global for five CPU seconds;
2. `goal_full`: goal priority for five CPU seconds;
3. `static_global_restart`: global probe, then fresh global for four seconds;
4. `static_goal`: global probe, then fresh goal priority for four seconds;
5. `adaptive`: global probe, then the telemetry-selected continuation.

A proof during the probe terminates that policy arm without a continuation.
Otherwise each restart policy has the same configured five-CPU-second total
budget and at most two ordinary prover processes.

## Signals and frozen intervention

After a non-proof probe with valid schema-version-1 telemetry, the controller
computes:

- `clause_growth` =
  `generated_non_trivial / max(processed_non_trivial, 1)`;
- `passive_pressure` =
  `high_water_unprocessed / max(processed_non_trivial, 1)`;
- resident pages and total CPU, for diagnostic resource reporting.

`clause_growth` alone controls the intervention. Candidate thresholds are
`4`, `8`, `16`, `32`, and `64`. A threshold switches to goal priority when
`clause_growth >= threshold`; otherwise it restarts global age/weight.

If telemetry is missing, has an unknown schema, or reports fewer than 64
non-trivial processed clauses, the deterministic fallback is goal priority.
There is one decision and at most one restart, so the policy cannot oscillate.

Calibration selects the threshold by this exact order:

1. greatest number of two-repeat reproducible solves;
2. fewest reproducible losses versus the static global restart;
3. greatest reproducible wins versus the static goal portfolio;
4. lowest median total CPU on common solved repetition coordinates;
5. highest threshold, favoring fewer interventions;
6. numeric threshold as a final stable tie break.

The selected threshold and complete candidate table are written to a
hash-identified selection file before validation.

## Resources, repetitions, and execution

Each split uses two repetitions and four concurrent controller workers.

- probe: one-second soft CPU limit, three-second kernel CPU limit;
- continuation: four-second soft CPU limit, six-second kernel CPU limit;
- full strategy: five-second soft CPU limit, seven-second kernel CPU limit;
- controller subprocess timeout: kernel limit plus ten wall seconds;
- memory: 1,536 MiB per prover process.

No random seed is consumed by the strategies or controller. Contract IDs bind
the source revision, release binary, corpus, scripts, preregistration,
strategies, budgets, split, and selection file. Completed coordinates resume
only after every raw artifact hash is reverified.

## Measurements and correctness gates

The analyzer must report:

- every adaptive intervention trace, signal, branch, and fallback reason;
- decision CPU and wall overhead;
- configured and observed aggregate CPU per policy;
- processed/generated/passive-pressure and resident-memory diagnostics;
- reproducible, one-repeat, unique, and lost solves;
- status polarity disagreements;
- branch reproducibility across repetitions;
- missing/invalid telemetry, external timeouts, and contract failures;
- PCL proof-step counts for every proof status.

Every theorem/unsatisfiable claim must retain a non-empty PCL proof. Any
satisfiable/counter-satisfiable status, proof status without PCL steps,
contract/hash failure, branch outside the frozen rule, or configured budget
violation fails correctness. The adaptive policy never edits clauses or
derivations; each proof is an ordinary proof from one of the two frozen
strategies.

## Frozen decision

Continue toward an integrated prototype only if validation and test both pass
correctness, every adaptive branch is reproducible, decision overhead is at
most 10 milliseconds per coordinate, no reproducible solve is lost versus
either equal-budget restart comparator, and test has at least two reproducible
adaptive-only solves versus both comparators or at least a 5% median CPU
reduction on at least four common solved repetition coordinates in both
validation and test.

Stop if correctness fails, a reproducible test solve is lost, or complete
validation and test show neither a unique solve nor the required efficiency
signal. A single unique test solve, branch instability, fewer than four common
solved coordinates for an efficiency claim, or insufficient successful probe
telemetry yields `uncertain`.

No production schedule or prover behavior changes in this experiment.
