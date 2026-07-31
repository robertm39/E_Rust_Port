# TSM ranking-cost and held-out-label feasibility preregistration

## Scope

This experiment addresses Bead `E_Rust_Port-9jt.3.9`. It follows Experiment
018 without changing the proof-derived training knowledge base, the original
family split, the learned weight, or any automatic schedule.

Experiment 018 established:

- a proof-derived training set with 224 weighted occurrences and both labels;
- a positive-only validation set with 150 weighted occurrences;
- no test labels because all eight repetition-1 test controls exhausted their
  ten-second resource budget;
- 400.32 microseconds of whole-process classifier CPU per validation
  occurrence;
- a 6.648 median learned/control CPU ratio on common validation solves; and
- no learned-only held-out solve.

The complete prior archive is
`.artifacts/experiments/2026-07-29-018-tsm-learning-baseline/tsm-learning-018-81232361-complete.tar.gz`,
with SHA-256
`8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156`.
The reused knowledge-base tree SHA-256 is
`838a4f14137344c8d1c0c17a0503fb8fc0a136dbcb206b35f6927c898fe7d13f`.

## Question

Can a narrow, semantics-preserving ownership or lookup change reduce live
proof-derived TSM ranking to at most 50 microseconds per weighted pattern and
at most 1.10 times the CPU of a structure-matched control on common work?
Only if both cost gates are plausible will the experiment spend additional
proof-search budget obtaining candidate-blind, two-class held-out labels.

## Frozen workloads

All Rust builds, executions, and profiles run on the retained Ubuntu 24.04
runner. The baseline source is the `main` revision recorded by the profiling
controller at execution. Release builds use the repository's pinned
dependencies and fat LTO. A debug-info release is used only for Callgrind
symbol attribution; native timings use the ordinary release executable.

The phase-isolation workload reuses these immutable Experiment 018 artifacts:

- `classifier-inputs-v4/validation.tsm`;
- `E_KNOWLEDGE/`; and
- `problems/casc_2025/UEQ/LCL026-10.p`, SHA-256
  `b0e8c769ae659ad7d89f632be19849c9bcdb0c9a34e72380466a8c7eaa556111`.

`LCL026-10` is chosen before profiling because Experiment 018 produced the
same status, proof hash, processed steps, generated clauses, and inference
counters for learned and control. Its only material treatment difference was
ranking cost: 2.035742 versus 6.499034 CPU seconds in repetition 1.

## Phase-isolated profile

`profile.py` creates an empty-test classifier input by retaining the exact
training section and replacing only the validation `Test:` set with an empty
set. It rejects a changed training prefix.

Native measurement uses one warm-up and eleven paired repetitions, alternating
execution order:

1. `startup`: `umlaut-tsm-classify --help`;
2. `translation_and_build`: the empty-test classifier workload;
3. `heldout_lookup_and_scoring`: the full validation workload minus its paired
   empty-test workload; and
4. `live_clause_ranking`: learned versus structure-matched control on
   `LCL026-10`, each stopped at 128 processed clauses.

Classifier commands retain Experiment 018's `Flat`, `IndexIdentity`, depth
100000, and limit 1 configuration. Search commands retain its four queues,
ratios `10:10:5:1`, KBO6 ordering, and forward-demodulation level 2. The only
search-bound change is the identical 128-processed-clause diagnostic stop.

Callgrind profiles one fresh execution of the same startup, empty classifier,
full classifier, control search, and learned search workloads. Instruction
deltas are reported alongside inclusive and self-attributed functions. The
search profiles are valid for cost attribution only if control and learned
telemetry have identical processed, generated, inference, and simplification
counters at the diagnostic stop.

Every classifier stdout must be byte-stable and stderr-empty. Every search
must emit valid final telemetry and an SZS status. Native results, Callgrind
files, annotations, executable hashes, source snapshot hash, input hashes,
commands, and host identity are retained.

## Optimization eligibility

A production edit is eligible only when the baseline profile identifies a
semantics-preserving removable ownership, allocation, or comparison cost that
accounts for at least 25% of the learned-only live-clause instruction delta.
The edit is limited to that measured path. It must preserve:

- total pattern ordering and dense TSM keys;
- classifier stdout byte-for-byte;
- proof status and proof-object hash;
- the diagnostic search's processed, generated, inference, simplification,
  and selected-clause counters; and
- all existing unit and integration tests.

The candidate repeats the complete phase-isolated profile. It advances only
if median full-minus-empty classifier CPU is below 50 microseconds per
weighted validation occurrence and median candidate/control CPU is at most
1.10 on the identical-work diagnostic.

It then runs three fresh repetitions of each treatment on all four
Experiment 018 common-solved validation problems at the original 8-second
soft/10-second hard limits. Candidate/control median CPU must remain at most
1.10 in aggregate, no reproducible solve may be lost, and proof objects must
replay. Failing any cost, correctness, or proof gate selects `reject` and
skips new held-out label collection.

## Conditional two-class held-out labels

Label collection begins only after the optimized candidate clears every cost
and common-solve gate. Candidate results never select problems, contribute
labels, or enter the knowledge base.

The control-only coverage pool uses the existing whole-family test partition.
Within the test families, problems are ordered solely by the immutable
CASC-2025 manifest SHA-256 rank with salt
`umlaut-tsm-two-class-coverage-v1`. The first four eligible problems per
family are attempted with the unchanged structure-matched control at 30
seconds soft/35 seconds hard CPU. Successful repetition-1 PCL proofs alone
produce direct examples. The pool is frozen before any candidate run.

The classifier test set is valid only with:

- no train or validation family;
- at least 20 weighted occurrences;
- at least one positive and one negative occurrence;
- exact source/problem/proof hashes; and
- no candidate-derived trace or pattern.

If coverage remains insufficient, the result is `reject` for practical label
scarcity; labels are never fabricated or rebalanced from candidate evidence.

With sufficient labels, the original Experiment 018 calibrator and frozen
quality gates apply: balanced accuracy above 0.55, calibrated Brier below the
constant-prior Brier, ECE at most 0.20, no reproducible test solve lost, and
either a reproducible learned-only test solve or at most 0.95 common-solve CPU
on both validation and test. The paired held-out search uses the same frozen
problem pool and equal budgets for both treatments.

## Decision

- `continue`: every cost, correctness, two-class label, calibration, no-loss,
  and complementarity gate passes.
- `reject`: an optimization is not profile-justified, either optimized cost
  gate fails, control-only two-class coverage is unavailable, a correctness
  gate fails, or valid held-out evidence fails the original adoption rule.

Automatic schedules remain unchanged unless `continue` is selected.
