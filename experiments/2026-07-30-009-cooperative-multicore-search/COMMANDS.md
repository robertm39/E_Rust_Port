# Reproduction commands

All Rust builds and prover executions were run on the retained Ubuntu runner
through `linode-runner.ps1`. The paths below are the measured paths.

## Focused tests and release build

```text
.\linode-runner.ps1 exec -- python3 -m unittest discover \
  -s /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search \
  -p test_*.py -v

.\linode-runner.ps1 exec -- cargo build --locked --release --bin umlaut
```

## Corpus reconstruction

```text
.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/prepare_corpus.py \
  --archive \
  /opt/e-rust-port/artifacts/prop-sat-007/casc_2025_corpus.tar.gz \
  --manifest \
  /opt/e-rust-port/source/experiments/\
2026-07-29-018-tsm-learning-baseline/corpus.jsonl \
  --output-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/corpus \
  --report \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/corpus-report.json
```

## Train

```text
.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/run_experiment.py \
  --repo-root /opt/e-rust-port/source \
  --manifest \
  /opt/e-rust-port/source/experiments/\
2026-07-29-018-tsm-learning-baseline/corpus.jsonl \
  --problem-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/corpus \
  --output-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/train-v2 \
  --binary /opt/e-rust-port/source/target/release/umlaut \
  --proofcheck \
  /opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/\
proofcheck-linux-x86_64/proofcheck \
  --validation-gate \
  /opt/e-rust-port/source/tools/validation/validate_tptp_solution.py \
  --source-revision 77a42527467d01f17a6045852f57d3498d93de23 \
  --phase train \
  --selection-output \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/selection-v2.json

.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/analyze.py \
  --root /opt/e-rust-port/artifacts/cooperative-multicore-009/train-v2 \
  --phase train \
  --output \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/train-v2-analysis.json

.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/validate_results.py \
  --root /opt/e-rust-port/artifacts/cooperative-multicore-009/train-v2 \
  --phase train \
  --proofcheck \
  /opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/\
proofcheck-linux-x86_64/proofcheck \
  --validation-gate \
  /opt/e-rust-port/source/tools/validation/validate_tptp_solution.py \
  --replay-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/\
train-v2-validation-replays \
  --output \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/\
train-v2-validation.json
```

## Validation

```text
.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/run_experiment.py \
  --repo-root /opt/e-rust-port/source \
  --manifest \
  /opt/e-rust-port/source/experiments/\
2026-07-29-018-tsm-learning-baseline/corpus.jsonl \
  --problem-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/corpus \
  --output-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/validation-v1 \
  --binary /opt/e-rust-port/source/target/release/umlaut \
  --proofcheck \
  /opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/\
proofcheck-linux-x86_64/proofcheck \
  --validation-gate \
  /opt/e-rust-port/source/tools/validation/validate_tptp_solution.py \
  --source-revision 77a42527467d01f17a6045852f57d3498d93de23 \
  --phase validation \
  --selection \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/selection-v2.json

.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/analyze.py \
  --root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/validation-v1 \
  --phase validation \
  --output \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/\
validation-analysis.json

.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/validate_results.py \
  --root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/validation-v1 \
  --phase validation \
  --proofcheck \
  /opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/\
proofcheck-linux-x86_64/proofcheck \
  --validation-gate \
  /opt/e-rust-port/source/tools/validation/validate_tptp_solution.py \
  --replay-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/\
validation-validation-replays \
  --output \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/\
validation-validation.json
```

## Test and final decision

```text
.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/run_experiment.py \
  --repo-root /opt/e-rust-port/source \
  --manifest \
  /opt/e-rust-port/source/experiments/\
2026-07-29-018-tsm-learning-baseline/corpus.jsonl \
  --problem-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/corpus \
  --output-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/test-v1 \
  --binary /opt/e-rust-port/source/target/release/umlaut \
  --proofcheck \
  /opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/\
proofcheck-linux-x86_64/proofcheck \
  --validation-gate \
  /opt/e-rust-port/source/tools/validation/validate_tptp_solution.py \
  --source-revision 77a42527467d01f17a6045852f57d3498d93de23 \
  --phase test \
  --selection \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/selection-v2.json \
  --validation-report \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/\
validation-analysis.json

.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/analyze.py \
  --root /opt/e-rust-port/artifacts/cooperative-multicore-009/test-v1 \
  --phase test \
  --output \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/test-analysis.json

.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/validate_results.py \
  --root /opt/e-rust-port/artifacts/cooperative-multicore-009/test-v1 \
  --phase test \
  --proofcheck \
  /opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/\
proofcheck-linux-x86_64/proofcheck \
  --validation-gate \
  /opt/e-rust-port/source/tools/validation/validate_tptp_solution.py \
  --replay-root \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/\
test-validation-replays \
  --output \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/test-validation.json

.\linode-runner.ps1 exec -- python3 \
  /opt/e-rust-port/source/experiments/\
2026-07-30-009-cooperative-multicore-search/analyze.py \
  --validation \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/\
validation-analysis.json \
  --test \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/test-analysis.json \
  --output \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/final-decision.json
```

## Evidence packaging

The archive was built from explicit accepted inputs. It does not include the
source corpus, checker distribution, smoke trees, or invalid first training
attempt.

```text
.\linode-runner.ps1 exec -- tar \
  -C /opt/e-rust-port/artifacts/cooperative-multicore-009 \
  -czf /opt/e-rust-port/artifacts/cooperative-multicore-009/evidence.tar.gz \
  train-v2 validation-v1 test-v1 \
  train-v2-validation-replays \
  validation-validation-replays \
  test-validation-replays \
  selection-v2.json \
  train-v2-analysis.json train-v2-validation.json \
  validation-analysis.json validation-validation.json \
  test-analysis.json test-validation.json \
  final-decision.json corpus-report.json

.\linode-runner.ps1 download \
  /opt/e-rust-port/artifacts/cooperative-multicore-009/evidence.tar.gz \
  .artifacts/experiments/\
2026-07-30-009-cooperative-multicore-search/evidence.tar.gz \
  --overwrite
```
