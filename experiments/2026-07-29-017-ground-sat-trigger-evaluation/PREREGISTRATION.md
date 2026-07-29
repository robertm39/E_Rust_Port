# Preregistration: periodic ground-SAT trigger evaluation

Bead: `E_Rust_Port-9jt.4.4`

Date frozen: 2026-07-29

## Question

Do periodic pseudo-grounded SAT checks provide enough complementary solves or
end-to-end benefit to justify enabling a trigger in Umlaut's default search?
Can the current call stream safely reuse incremental SAT state, and can a
nonterminal unsatisfiable core feed clause selection?

## Static implementation audit

This audit was completed before the held-out benchmark.

- The production SATCheck path emits per-run totals for checks, SAT and UNSAT
  outcomes, input clauses, post-purity clauses, unsatisfiable-core clauses,
  preprocessing time, encoding time, and solver time.
- Every SATCheck call uses `with_fresh_incremental_service`, which resets
  CaDiCaL before and after the call. Actual cross-call clause reuse is zero.
- Every call rebuilds the current proof-state snapshot and locally renumbers
  its atoms. Clauses can be simplified, deleted, or replaced between calls.
  Persistent reuse therefore needs stable atom and source-clause identities
  plus selector retirement; the current state has no sound lifecycle for it.
- An UNSAT pseudo-grounded subset is already a sound refutation. Its minimized
  core is reconstructed as a terminal `DC_SAT_GEN` inference. SAT and
  decision-limited results have no core. Thus the current abstraction offers
  no sound nonterminal core-feedback event.
- Generated schedules contain 419 SATCheck configurations: 256 use
  `NoGrounding`; 161 use `ConjMinMinFreq`; two use `GlobalMax`. Of the 163
  active configurations, 153 use a 5,000 processed-clause interval.

No production algorithm or default is changed before measurement. Existing
telemetry and proof objects are sufficient for the frozen questions.

## Held-out corpus

`select_corpus.py` freezes 24 problems: four hash-ranked problems from each of
six hash-ranked complete families. Inputs must:

- come from the CASC-2025 manifest's training partition;
- have an expected theorem or unsatisfiable result in EPR, FOF, or UEQ;
- contain 500 through 250,000 bytes; and
- belong to no family used by the incremental-service capture selection or
  the production CaDiCaL gate.

The salt is `umlaut-ground-sat-trigger-v1`. Selection observes only manifest
metadata and prior selection files, never Umlaut outcomes. `corpus.jsonl`,
its source hashes, and this document are frozen before candidate execution.

## Strategies

Every strategy uses:

- `--expert-heuristic=(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))`
- `--term-ordering=KBO6`
- `--forward-demod-level=2`

The four variants are:

1. `off`: `--satcheck=NoGrounding`;
2. `step5000`: `--satcheck=ConjMinMinFreq`,
   `--satcheck-proc-interval=5000`, and
   `--satcheck-decision-limit=10000`;
3. `step10000`: the same grounding and decision limit with
   `--satcheck-proc-interval=10000`; and
4. `size10000`: the same grounding and decision limit with
   `--satcheck-gen-interval=10000`.

Only one interval is explicitly enabled per candidate. Other threshold fields
retain the fixed heuristic's disabled value. The 5,000-step candidate
represents the dominant generated-schedule policy, 10,000 steps tests lower
frequency, and size 10,000 tests state-sensitive triggering.

Each coordinate runs twice with a 10-second soft and 13-second hard CPU
budget, 1,536 MiB, and at most four concurrent processes on one ephemeral
Ubuntu 24.04 Linode. This produces 192 runs. Ordering is hash-shuffled and an
unchanged second invocation must resume all coordinates.

## Correctness and proof gates

Before interpreting performance:

1. focused SATCheck, threshold-controller, core-minimization, and proof-output
   tests must pass on Linux;
2. all build-paired terminal statuses must agree in proof/model polarity;
3. every emitted telemetry object must be well formed and internally
   consistent;
4. at least one reproducible SATCheck refutation, if produced, must retain a
   parseable proof object and its reported core must independently re-solve
   as UNSAT with the integrity-pinned CaDiCaL path; and
5. the proof must be tried with the repository's integrity-pinned ProofCheck
   1.0 path. If ProofCheck does not support `cdclpropres`, focused ancestry
   tests and independent core re-solving are the explicit coverage boundary,
   not a claimed external proof validation.

A failure makes the relevant candidate inconclusive and forbids promotion.

## Reported measurements

For each candidate, report:

- calls, reached problems, calls per reached run, SAT/UNSAT/limited yield;
- total and per-call preprocessing, encoding, solver, and combined cost;
- input, post-purity, and terminal core clause totals and ratios;
- reproducible solve set, candidate-only and baseline-only solves;
- common-solved CPU, generated-clause, processed-clause, proof-state
  high-water, term-storage, and maximum-RSS ratios;
- proof reconstruction and independent core validation coverage; and
- actual reuse (zero by implementation) plus the exact-clause overlap between
  consecutive captured calls in the prior incremental-service archive.

Timeout-limited all-run CPU totals are diagnostics, not speedups.

## Trigger decision

Promote a trigger for schedule follow-up only when correctness passes, it
reaches at least eight paired coordinates across at least four problems, it
loses no reproducible baseline solve, and either:

- it adds at least one reproducible candidate-only solve; or
- on common reproducible solves its median CPU ratio is at most `0.95`, while
  generated clauses and proof-state high-water are at most `1.02`, maximum
  RSS is at most `1.05`, and SATCheck consumes at most 3% of candidate CPU on
  reached runs.

Keep SATCheck default-off when no candidate passes that gate. Reject a
candidate when it loses a baseline solve or any common-solved CPU, generated,
high-water, or RSS ratio exceeds `1.05`.

Do not implement persistent reuse unless the prior captured stream shows a
median consecutive exact-clause retention of at least 50% and a design can
provide stable atom/source identities and selector retirement without
weakening proof reconstruction.

Do not implement nonterminal core feedback for the present abstraction:
UNSAT is terminal and other results contain no core. A future abstraction may
reopen this only with a separately proved soundness contract.

No post-hoc workload or threshold may change these decisions. Exploratory
diagnostics must be labeled separately.
