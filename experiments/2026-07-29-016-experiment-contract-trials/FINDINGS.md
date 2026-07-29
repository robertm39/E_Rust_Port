# Experiment-contract version-1 trial findings

## Question

Can a small common result contract make ablation decisions reproducible,
separate correctness from performance, quantify timing noise without
overclaiming statistical confidence, and produce an unambiguous
continue/stop/uncertain outcome?

## Setup

This is a retrospective contract trial, not a new prover benchmark. It uses two
studies frozen and executed earlier on 2026-07-29:

| Trial | Source experiment | Treatment | Runs |
| --- | --- | --- | ---: |
| Performance comparison | `2026-07-29-014-rewrite-cache-evaluation` | cache versus proof-preserving recomputation | 180 |
| Harmless default-off toggle | `2026-07-29-015-preprocessing-evaluation` | `--bce=true` versus baseline | 184 |

Both source studies used Ubuntu 24.04, Linux 6.8, glibc 2.39, four worker
CPUs, 1,536 MiB per job, a candidate-blind CASC test selection, fixed search
arguments, and two repetitions. Neither controller uses an RNG. Their exact
controller commands, source revisions, resource budgets, binary hashes, and
artifact identities are in the two result records.

The retained raw archives are:

| Archive | Bytes | SHA-256 |
| --- | ---: | --- |
| `.artifacts/experiments/2026-07-29-014-rewrite-cache-evaluation/rewrite-cache-results-final.tar.gz` | 23,397,005 | `4c154b26822193578198a4bb54a3caa554076df52aef51476bd65392ebaa6fe6` |
| `.artifacts/experiments/2026-07-29-015-preprocessing-evaluation/preprocessing-results.tar.gz` | 21,224,572 | `6da8483fe84a98e328e1d08e8a457451a962b1ca338204934cddf2389ef4f733` |

The commands used for this trial were:

```text
python -m unittest tools/experiment_contract/test_validate.py experiments/2026-07-29-016-experiment-contract-trials/test_verify_trials.py

python tools/experiment_contract/validate.py --verify-artifacts experiments/2026-07-29-016-experiment-contract-trials/rewrite-cache-result.json experiments/2026-07-29-016-experiment-contract-trials/bce-toggle-result.json

python experiments/2026-07-29-016-experiment-contract-trials/verify_trials.py --verify-artifacts
```

## Result

The verifier independently read 364 raw result records and their telemetry.
It did not take the CPU ratios, coverage, status counts, or noise values from
the compact result records.

| Trial | Exact status pairs | Common solves | Candidate / baseline CPU | Median paired-ratio repeat range | Maximum paired-ratio repeat range | Decision |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Rewrite cache | 90/90 | 3 | `0.890751` | `0.042882` | `0.055405` | `continue` |
| BCE toggle | 46/46 | 3 | `0.987599` | `0.004215` | `0.058499` | `stop` |

The relative range for each two-repetition coordinate is
`(maximum - minimum) / median`. The records separately give the median and
maximum range for baseline CPU, candidate CPU, and the paired CPU ratio. This
does not claim a confidence interval. It exposes the scale of observed repeat
movement beside the measured effect.

The cache's roughly 10.9% common-solve CPU reduction exceeded its 5.54%
maximum paired-ratio repeat movement and passed the frozen 0.95 CPU threshold,
1.02 search-size guards, and 1.05 larger-budget memory guard. The source proof
gate independently verified 6/6 representative claims and found no status or
polarity discrepancy. The contract decision is therefore `continue`, with the
production cache retained.

BCE removed 6,538 clauses on transformation-active held-out coordinates but
added no solve. Its roughly 1.24% CPU reduction missed the frozen 0.95
threshold and is smaller than the 5.85% maximum paired-ratio repeat movement.
The correctness gate still passes: all 46 BCE/baseline status pairs match,
zero proof claim was rejected, and `NUN086+2` is an independently verified
transformation-active witness. The contract decision is therefore `stop` for
enabling BCE by default, while retaining the existing explicit option.

## Falsification checks

- The validator rejects duplicate JSON keys, missing sections, path traversal,
  artifact size/hash mismatches, inconsistent solve arithmetic, performance
  without a noise record, and `continue` without passing correctness.
- Twelve unit tests cover the reusable validator and trial computations.
- Artifact verification checked every declared preregistration, controller,
  analyzer, proof wrapper, compact summary, and raw archive by bytes and
  SHA-256.
- The trial verifier recomputed status pairing over every relevant coordinate,
  not just successful runs.
- CPU ratios include only common reproducible proof solves with telemetry in
  both treatments and both repetitions.
- The verifier checks secondary search/memory values and final decisions
  against the original source analyzers, and checks that every declared
  candidate witness is independently verified.

All checks passed.

## Conclusion and limits

Version 1 meets the Bead's acceptance criteria. Two different kinds of trial
can be reproduced from preserved commands and artifacts, correctness is
structurally and operationally separate from performance, run-to-run
variation is explicit, and the same three-way decision vocabulary yields one
clear `continue` and one clear `stop`.

The practice remains intentionally lightweight. Each source study has only
three common held-out solves and two repetitions, so the exact speedup
estimates are narrow-sample evidence. Large raw evidence stays in the ignored
artifact tree and is integrity-addressed from the tracked record; clones
without those archives can validate structure but cannot recompute the trial.
Future experiments should use stronger resampling or more repetitions when
the decision margin is close, but the schema does not require that machinery
for deterministic counters or plainly decisive paired results.
