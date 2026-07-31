# Recursive neural clause-guidance findings

## Result

The preregistered verdict is `stop-offline-validation`.

The frozen recursive candidate does not justify test-set access or online
integration. Production search, dependencies, schedules, and packaging are
unchanged.

## Evidence integrity and split

The source archive SHA-256 is
`8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156`.
All nine member hashes and all frozen extraction counts matched.

The executed phases read:

- train: 2,918 given clauses and 154 content-level proof positives from five
  problems in five families; and
- validation: 1,298 clauses and 72 positives from `LCL026-10` and `LCL365+1`.

The validation process did not extract, parse, featurize, or score either the
`PUZ` or `ROB` trace member. Archive-wide SHA-256 calculation and compressed
container indexing still read container bytes, so this is a test-evaluation
boundary rather than a claim that the storage device never read those bytes.
Test ranking metrics are `not-run`. End-to-end solves, prover CPU, and prover
memory are also `not-run`, because the preregistered offline validation gate
failed before an online experiment was authorized.

The artifact auditor independently recomputed the selected seed, every gate,
and the verdict; checked all six model hashes and sizes; and rejected a
one-byte model mutation. Two complete validation executions produced identical
model hashes, score checksums, extraction, metrics, selected seed, gate values,
and verdict. Their decision-projection SHA-256 is
`959229068b877ceccd08c1cd92ed9da0cf1900f19e92de766f7cf7b209ac32ef`.

## Held-out proof-clause ranking

The macro averages across the two validation problems are:

| Ranker | AP | ROC AUC | Top-10% recall | All-positive prefix |
| --- | ---: | ---: | ---: | ---: |
| Chronological given order | 0.418232 | 0.751893 | 0.435319 | 1.000000 |
| Linear structural baseline | 0.224704 | 0.607589 | 0.357872 | 0.972260 |
| Recursive selected seed 23 | 0.221671 | 0.640751 | 0.357872 | 0.934416 |

The recursive candidate is 0.003033 AP below linear, ties linear top-10%
recall, and reaches all positives at 0.9611 times linear's prefix rather than
the required 0.80. Chronological order substantially outperforms both learned
rankers on AP and top-10% recall.

Recursive macro AP by seed was:

| Seed | AP | Top-10% recall | All-positive prefix |
| ---: | ---: | ---: | ---: |
| 11 | 0.198130 | 0.287234 | 0.901235 |
| 23 | 0.221671 | 0.357872 | 0.934416 |
| 37 | 0.230551 | 0.254043 | 0.960694 |
| 53 | 0.200902 | 0.275319 | 0.998677 |
| 71 | 0.231362 | 0.347234 | 0.990778 |

The AP range, 0.033232, passes the 0.10 stability bound, but zero of five seeds
strictly beat linear on both AP and top-10% recall. Seed 23 is the
preregistered median-AP seed.

## Cost and packaging

On the retained four-core Ubuntu runner, the instrumented replication used
23.814 seconds user CPU, 0.029 seconds system CPU, 23.859 seconds wall, and
44,040,192 bytes peak process RSS. Linear training took 2.238 seconds wall;
each recursive seed took 3.072–3.125 seconds.

Selected-model inference was checksum-identical across repeats:

- in process: 141.206 microseconds per clause, failing the 100-microsecond
  gate; and
- persistent external process, 64-clause batches including JSON and IPC:
  207.824 microseconds per clause, passing the 500-microsecond gate with
  20,701,184 bytes worker peak RSS.

The selected model is 7,609 bytes. The shared experimental Python
implementation, external worker, and selected model total 43,834 bytes,
excluding the Python interpreter. The prototype adds zero repository
dependencies. ONNX Runtime, PyTorch, NumPy, and scikit-learn were unavailable
on the runner; the preregistration forbade adding them before the quality gate,
so ONNX packaging is `not-evaluated`.

## Gate disposition

Passed:

- AP seed range;
- external-process latency;
- model size;
- peak RSS; and
- repeat determinism.

Failed:

- AP improvement;
- top-10% recall improvement;
- simulated all-positive-prefix improvement;
- at least four of five seeds beating linear; and
- in-process latency.

This small frozen recursive encoder also performs worse than chronological
order, so a larger dependency or an end-to-end trainable graph/RNN stack has
not earned an online experiment from this evidence. Any revival should start
with substantially broader family-separated proof logs, a causal
candidate-availability simulation, and a stronger non-neural baseline before
considering runtime or production integration.

## Preserved artifacts

- Complete first run:
  `.artifacts/experiments/2026-07-30-018-recursive-neural-guidance/evidence-v1.tar.gz`
  (`d86b7d87a85e365aee6af2d4702b2d75910e8219b076150be86475bdecdd8eb9`).
- Instrumented exact replication:
  `.artifacts/experiments/2026-07-30-018-recursive-neural-guidance/evidence-v2.tar.gz`
  (`ee287263062eeda9c5cabd268c686d08e8026665a8aacd525ed622648b0432b7`).
