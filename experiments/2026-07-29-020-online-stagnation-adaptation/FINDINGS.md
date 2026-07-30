# Experiment 020: bounded online stagnation adaptation

## Question

Can a telemetry decision after a short saturation probe improve held-out
coverage or efficiency over an equal-budget static portfolio?

The tested hypothesis was that unusually high non-trivial clause generation per
non-trivial processed clause indicates clause-growth stagnation. A bounded
outer-loop controller could then restart with goal hard priority, while
low-growth searches would restart the global age/weight heuristic.

## Architecture tested

This experiment deliberately did not mutate a live `ProofState`. Each policy
uses ordinary Umlaut processes:

1. run global age/weight for one soft CPU second;
2. read aggregate search telemetry;
3. make one deterministic decision;
4. either stop on a probe proof or restart global/goal priority for four soft
   CPU seconds.

The policy cannot oscillate and does not alter clauses, inferences, or proof
output. Its PCL proof is exactly the proof produced by the probe or continuation
process.

The equal-budget comparators are:

- a global probe followed by a fresh global continuation;
- a global probe followed by a fresh goal-priority continuation.

Five-second uninterrupted global and goal-priority arms provide context but are
not the primary equal-restart-cost comparison.

## Frozen setup and leakage controls

The measured source revision was
`42bfa440729dfe214042020898f7ba87fed7ab4f`. The Ubuntu release binary SHA-256
was `22abd227725da25af6143ae4f3159a05ccd477bd0f00d0aa955c49f7392aecd8`.
No Rust source changed.

The candidate-blind corpus selector used only the immutable CASC-30 manifest,
category, family, expected class, size, identity, and a frozen hash salt. The
24-problem corpus SHA-256 is
`a4c67ef7bb3ecc5f00a7ef1d4e4d3dbf2ba022c0f522f2ee9a40fc1e698cfe4b`.
It contains eight calibration, eight validation, and eight test problems in EPU
and UEQ. Entire source families are disjoint:

- calibration: GRP, HWV, KLE, LAT;
- validation: LCL, PUZ, ROB, SWV;
- test: PLA, REL.

Threshold selection used calibration only. Validation was hash-pinned in
`VALIDATION.md` before test execution. Test was not opened until the validation
report passed its correctness gate.

Every full search used a five-second soft CPU limit and seven-second kernel CPU
limit. Restart policies used one/three seconds for probe soft/hard limits and
four/six seconds for continuation limits. Each process had a 1,536 MiB memory
limit. There were two repetitions and four controller workers.

## Calibration

Calibration captured 80 primitive coordinates: uninterrupted global/goal,
global probe, global continuation, and goal continuation.

The candidate clause-growth thresholds were `4`, `8`, `16`, `32`, and `64`.
Every threshold reproduced the same single solve, `LAT260-2`, with no loss
against static global restart and no win against static goal. The
preregistered conservative tie-break therefore selected threshold `64`.

- Calibration contract:
  `70bba21867ab8615333ea75b3efc9eb52dc5924d9d231a2344b914d531975bb6`
- Calibration report:
  `8b35b51407c8d122b37c142a8e7f3fe5a1e28b21b88373a834648e2c9e70946c`
- Selection ID:
  `a53a56b93b549dd8258801e91535590e65a5e6e4394df16fe145bb0648c1d15c`

At threshold 64, six calibration coordinates had valid low-growth signals and
restarted global. Ten used the deterministic goal fallback because telemetry
was absent or fewer than 64 non-trivial clauses were processed. Every branch
agreed across repetitions.

## Held-out results

All five policies reproduced the same one solve in each held-out split:

| Split | Adaptive | Static global restart | Static goal | Adaptive-only | Lost |
| --- | --- | --- | --- | ---: | ---: |
| Validation | `PUZ008-2` | `PUZ008-2` | `PUZ008-2` | 0 | 0 |
| Test | `REL024-1` | `REL024-1` | `REL024-1` | 0 | 0 |

There were no one-repeat solves or status-polarity disagreements.

On validation, adaptive/static-goal CPU was `0.760391325` on the two
repetitions of the only common solve. On test it was `0.997074634`, again on
only two coordinates. The preregistered efficiency gate requires four common
solved repetition coordinates in both splits, so neither ratio supports an
efficiency claim. Adaptive/static-global-restart ratios were `0.998159116` on
validation and `0.992956588` on test.

## Intervention traces and overhead

| Split | Global | Goal fallback/switch | Probe solved | Unstable problems |
| --- | ---: | ---: | ---: | ---: |
| Calibration | 6 | 10 | 0 | 0 |
| Validation | 8 | 6 | 2 | 0 |
| Test | 0 | 14 | 2 | 0 |

Maximum decision wall overhead was 24.267 microseconds on validation and 9.314
microseconds on test, below the frozen 10-millisecond limit. Maximum measured
decision CPU was 24.687 and 105.678 microseconds, respectively. The larger test
CPU sample reflects process-wide Python CPU accounting while other controller
threads were active; wall overhead is the direct per-decision latency.

The main limitation is observability. Forty-eight calibration, 58 validation,
and 112 test phase telemetry files were absent after kernel hard stops. Most
importantly, 14 of 16 test adaptive probes lacked decision telemetry, so the
registered policy took its deterministic goal fallback. It therefore behaved
like the static goal portfolio on seven of eight test problems rather than
testing an informative clause-growth split.

Missing telemetry was never synthesized. The analyzer hash-verified the retained
stdout/stderr and the recorded absence. This does not invalidate a proof or
status result, but it makes the adaptive signal evidence insufficient.

## Correctness, fairness, and reproducibility

The 240 primary calibration/validation/test coordinates all completed and
hash-resumed:

- 80 calibration coordinates resumed without execution;
- 80 validation coordinates resumed without execution;
- 80 test coordinates resumed without execution.

Reanalysis produced byte-identical calibration, selection, validation, and test
JSON. Across all phases there was no bad satisfiable/counter-satisfiable status,
proof status without PCL steps, external timeout, configured-budget violation,
contract/hash failure, or adaptive branch-rule violation.

Every restart policy configured the same one-plus-four soft CPU budget and at
most two prover processes. Early probe proofs stopped rather than spending the
remaining budget. The static and adaptive restart comparators therefore have
the same process/preprocessing boundary and resource configuration.

The Linux experiment tests passed 11 cases, the release build completed, and
the one-problem five-path smoke passed raw analysis before calibration.

## Decision

The frozen verdict is `uncertain`.

Correctness, branch reproducibility, overhead, and no-loss gates passed, but the
policy produced no held-out solve delta, had too few common solves for an
efficiency claim, and lacked successful test probe telemetry on 14 of 16
coordinates. The preregistration classifies insufficient probe telemetry as
uncertain.

Leave online stagnation adaptation out of automatic schedules. There is no
integrated Rust change in this experiment.

Follow-up `E_Rust_Port-9jt.3.10` tracks the prerequisite for another trial:
make at least 95% of non-proof probes emit decision telemetry before
termination, using a deterministic processed-clause probe or atomic
intermediate checkpoint, then rerun against the same equal-budget comparators.

## Raw evidence

The complete ignored archive is:

```text
.artifacts/experiments/2026-07-29-020-online-stagnation-adaptation/online-adaptation-020-complete.tar.gz
```

It is 29,733,382 bytes with SHA-256
`5594302c52397cd5d3aaff29fd7efe90bdf60620e71dcac39d571a91c7f7a5cc`.
It contains four contracts, 245 result records including smoke, every raw
stdout/stderr/telemetry artifact, both copies of each byte-identical analysis,
both selection copies, and the exact 27-file corpus subset archive.

Local archive verification rejected absolute/parent paths and independently
verified five embedded hashes: calibration, selection, validation, test, and
corpus subset. The compact machine-readable summary is
`results-summary.json`.

## Reproduction

After synchronizing the preregistered source snapshot to the Ubuntu runner,
building the release `umlaut`, and extracting the pinned corpus subset, run:

```text
python3 experiments/2026-07-29-020-online-stagnation-adaptation/run.py \
  --problem-root /opt/e-rust-port/corpus-020 \
  --binary /opt/e-rust-port/target-020/release/umlaut \
  --source-revision 42bfa440729dfe214042020898f7ba87fed7ab4f \
  --output-root /opt/e-rust-port/online-adaptation-020/calibration-v1 \
  --phase calibration --workers 4 --repetitions 2
```

Analyze calibration with `analyze.py --phase calibration` and
`--selection-output selection.json`. Use that selection for validation. Test
additionally requires the hash-valid validation JSON through
`--validation-report`; the controller rejects test execution without it.
