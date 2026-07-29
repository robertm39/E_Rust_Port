# Periodic ground-SAT trigger findings

## Conclusion

Leave periodic ground-SAT checking default-off. Do not promote either
processed-step trigger, and reject the proof-state-size trigger evaluated
here.

The common generated-schedule policy, a check every 5,000 nontrivial processed
clauses, was cheap enough but added no solve and did not improve common-solve
CPU. A 10,000-step interval barely fired. Triggering at proof-state size
10,000 called SAT 656 times, spent 55.8% of telemetry-confirmed reached-run
CPU inside SATCheck, increased common-solve CPU to `1.542813` of baseline, and
increased maximum resident pages to `1.132813`.

Persistent SAT state remains worth a separate architectural design, not an
implementation patch: prior captures retained a median 68.2% of exact clauses
between consecutive calls, but only 41/126 pairs were add-only. Stable atom
and source-clause identities plus selector retirement are prerequisites.

Nonterminal core feedback is not applicable to the current sound abstraction.
An UNSAT pseudo-grounded subset is already a terminal proof; SAT and
decision-limited calls have no core.

## Question and frozen setup

This experiment evaluates Bead `E_Rust_Port-9jt.4.4`. The hypothesis,
candidate-blind corpus selection, strategy arguments, correctness gates,
budgets, and decision thresholds were frozen in `PREREGISTRATION.md` before
candidate execution.

The 24 held-out problems contain four SHA-256-ranked problems from each of six
complete CASC-2025 training families: COL, COM, KLE, SET, MGT, and MSC. All
families used by the earlier incremental-service and CaDiCaL production-gate
studies were excluded before ranking. `corpus.jsonl` has SHA-256
`3fec449651e6fa9feb004ee4e43ffa67b936c135785cc5050fff6d9f32add6c8`.

Every strategy used the same fixed clause-selection heuristic, KBO6,
forward-demodulation level 2, binary, two repetitions, and 10/13-second
soft/hard CPU limits. The candidates were:

- `step5000`: `ConjMinMinFreq` every 5,000 nontrivial processed clauses;
- `step10000`: the same grounding every 10,000 such clauses; and
- `size10000`: the same grounding whenever proof-state cardinality crossed a
  multiple of 10,000.

The optimized all-feature binary retained the production runtime default
`UMLAUT_CADICAL_MODE=off`; therefore held-out calls used the current internal
SAT service. The all-feature build and explicit CaDiCaL tests exercised the
optional selector/core mapping without silently changing the production
runtime policy.

The measured source revision was
`4e24b38c223617f7f2a55c23ab2295de7addd10e`. The binary SHA-256 was
`5d5e63ec77531e432823974f7fbf41e3f5205adfaa5cd707fbf28c2fc7cbb8c9`.
The source snapshot uploaded by the Linode controller had SHA-256
`97984559967441ff6df679d9740d3e307b4a94e6b73eba25f6bd1f66d31e46b5`.

## Results

All four strategies reproducibly solved the same five problems:
`COM125+1`, `KLE145-10`, `MGT067+1`, `SET090+1`, and `SET637+3`. Every one of
the 144 candidate/baseline status pairs matched exactly. There were zero
proof/model polarity disagreements, zero candidate-only solves, and zero
baseline-only solves.

| Strategy | Reached coordinates / problems | Calls | SAT / UNSAT / limited | SAT CPU per call | SAT CPU share | Common-solve CPU | Generated | High-water | RSS | Decision |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `step5000` | 8 / 4 | 10 | 10 / 0 / 0 | `0.036845` | `0.4606%` | `1.001514` | `1.0` | `1.0` | `0.999448` | keep off |
| `step10000` | 2 / 1 | 2 | 2 / 0 / 0 | `0.118120` | `1.1813%` | `1.003103` | `1.0` | `1.0` | `0.998741` | keep off |
| `size10000` | 38 / 19 | 656 | 650 / 0 / 6 | `0.269156` | `55.8187%` | `1.542813` | `1.0` | `1.0` | `1.132813` | reject |

Ratios are candidate/baseline medians across the ten paired coordinates for
the five common reproducible solves. The size trigger also increased median
term-storage estimate to `1.315431`; its maximum paired CPU ratio was
`1.987062`, and its maximum paired RSS ratio was `1.152280`.

The 5,000-step result is not a hidden speedup: its `1.001514` CPU ratio is
inside observed two-repeat noise, whose median paired-ratio relative range was
`0.024914` and maximum was `0.052943`. It nevertheless fails the frozen
`0.95` benefit gate and adds no solve.

The 10,000-step candidate fails the frozen reach gate, firing in only two
coordinates on `MSC024-1`. The size trigger decisively fails the material
regression and 3% SAT-cost guards. Two size-trigger and one 10,000-step hard
`ResourceOut` results lacked final telemetry; all had the exact same terminal
status as baseline. Size-trigger call and cost totals are therefore
conservative lower bounds over 46/48 telemetry-bearing coordinates.

All held-out checks returned SAT or a decision-limited result, so held-out
terminal core-size lists are empty. Production telemetry records clause and
core sizes only for terminal UNSAT calls; it deliberately does not claim
input-size distributions for the SAT-only held-out stream.

## Incremental reuse

The current implementation has zero cross-call reuse. Each call enters
`with_fresh_incremental_service`, resets before and after solving, rebuilds a
proof-state snapshot, and locally renumbers atoms. `proof_state_sat_check`
also resets the selected service after applying the report.

`analyze_reuse.py` independently read the earlier raw capture archive
(`85356e073a26234f51e07898019d0a9a7685066eff21dd9350d621ede3158375`)
and verified every session hash. Across 178 unique sessions, 46 call streams,
and 126 consecutive pairs:

- median clauses retained from the previous call were `0.681818`;
- median current clauses reusable from the previous call were `0.681308`;
- 41/126 pairs (`32.54%`) were add-only; and
- 38/126 pairs (`30.16%`) were byte-level clause-multiset identical.

The exact-clause metric is conservative because local atom renumbering can
hide logically unchanged clauses. Even so, the 68.2% retention clears the
frozen “worth a future design” threshold. The deletion-heavy majority rejects
simple append-only reuse. A future design must first supply stable atom IDs,
stable source-clause IDs, selector activation/retirement, bounded database
growth, and proof reconstruction across retired selectors.

## Core reconstruction and proof validation

Because the held-out stream produced no SATCheck UNSAT, a four-clause
proof-only differential witness was run after the performance result. It is
excluded from all solve, timing, reach, and trigger decisions.

With preprocessing disabled and a one-step `GlobalMin` trigger, both
repetitions:

- terminated through telemetry outcome `sat_check`;
- reported one check, four input clauses, four post-purity clauses, and a
  four-clause UNSAT core;
- emitted byte-identical proof SHA-256
  `53a7a1c320e7ca5ed7888648e6c936b7c38909b40950f247407a647a7f1c40b9`;
  and
- reconstructed `$false` through `cdclpropres` with four distinct source-core
  parents.

The reconstructed four-clause propositional core was independently solved
UNSAT by integrity-pinned CaDiCaL 3.0.1 with exit code 20. ProofCheck 1.0
self-certified all 117 checks and returned `VerifiedGood` through the
repository fail-closed validation gate. The validation report id is
`ca881498a41c926f779853e884d992363ccc9887b5f7ddcbacc2f0f22553ad81`.

This confirms the terminal core path, not nonterminal feedback. Under current
semantics there is no sound event at which a core can merely reprioritize
clauses: an UNSAT core closes the proof, while SAT and limited calls expose no
core.

## Correctness and falsification checks

Ubuntu 24.04 validation passed:

- Cargo formatting;
- 14 SATCheck-focused all-feature tests, including the four new threshold
  controller tests;
- minimized-core separation and CaDiCaL failed-selector/source-core mapping;
- all-target, all-feature Clippy with warnings denied;
- the optimized all-feature release build;
- ten Python selection, packing, reuse, contract, and analysis tests;
- 192/192 result production and 192/192 unchanged resume;
- all raw stdout, stderr, and telemetry SHA-256 checks;
- the experiment-result schema and all declared artifact hashes and sizes;
- independent pinned-CaDiCaL core re-solving; and
- ProofCheck 1.0 external proof validation.

Two premeasurement harness failures were falsification successes, not result
changes. The corpus-pack test initially assumed ignored corpus files existed
inside the uploaded source snapshot, so it was made self-contained. The run
wrapper initially followed an indirect status helper one layer too few, so it
now classifies the repository's proof statuses directly. Rustfmt/import/Clippy
issues in the new threshold tests were also corrected before the final build.
No candidate result existed when these repairs were made; the corpus,
strategies, thresholds, budgets, and decision rule did not change.

The first proof-only witness was closed by normal preprocessing before
SATCheck. A four-clause propositional witness replaced it, with the failed
attempt retained in the proof archive. This change affected proof-boundary
coverage only and occurred after the performance decision.

## Commands and artifacts

The main held-out command was:

```text
python3 experiments/2026-07-29-017-ground-sat-trigger-evaluation/run.py
  --source-revision=4e24b38c223617f7f2a55c23ab2295de7addd10e
  --phase heldout
  --manifest experiments/2026-07-29-017-ground-sat-trigger-evaluation/corpus.jsonl
  --problem-root /opt/e-rust-port/ground-sat-corpus
  --binary target/release/umlaut
  --output-root /opt/e-rust-port/ground-sat-results
  --workers 4
  --memory-mib 1536
```

The complete held-out archive is
`.artifacts/experiments/2026-07-29-017-ground-sat-trigger-evaluation/results.tar.gz`,
4,832,702 bytes, SHA-256
`b5f7da29bdbd5e6844d6188d9bf95ab8e4bef015bad116edc467f4e5be16ee5a`.

The complete proof/core archive is
`.artifacts/experiments/2026-07-29-017-ground-sat-trigger-evaluation/proof-validation.tar.gz`,
16,769,416 bytes, SHA-256
`2b4fae05e3d2fb57f4b3879abf71ebd906d9b47d7a14a2da218c04564832188f`.

`results-summary.json` is the machine-readable source analysis,
`RESULTS.md` is the compact table, and `experiment-result.json` is the
validated version-1 decision contract.

## Limits

This is a 24-problem, six-family, 10-second study of the current production
internal SAT service and `ConjMinMinFreq` grounding. It does not evaluate a
new abstraction, persistent selector lifecycle, CaDiCaL-always runtime
policy, adaptive learned trigger, or longer CASC budget. The negative trigger
decision is valid for the evaluated policies; the captured overlap result
justifies design work only after the identity and retirement invariants exist.
