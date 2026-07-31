# Reproduction commands

All Rust builds and prover executions ran on the retained Ubuntu runner through
`linode-runner.ps1`.

## Paths

```powershell
$Exp = "/opt/e-rust-port/source/experiments/2026-07-30-017-complementary-connection-worker"
$Root = "/opt/e-rust-port/artifacts/connection-017"
$ProblemRoot = "/opt/e-rust-port/artifacts/adaptive-probe-010/corpus"
$Binary = "$Root/bin/umlaut"
$ProofCheck = "/opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/proofcheck-linux-x86_64/proofcheck"
$Gate = "/opt/e-rust-port/source/tools/validation/validate_tptp_solution.py"
```

The production source revision used to build the binary was:

```text
b80150e336b8c2da7b2d5fcefbd01cf71f7001c5
```

## Build and script checks

```powershell
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- "cd /opt/e-rust-port/source && cargo build --release --bin umlaut"
.\linode-runner.ps1 exec -- "mkdir -p $Root/bin && cp /opt/e-rust-port/source/target/release/umlaut $Binary"

.\linode-runner.ps1 exec -- "cd $Exp && python3 -m unittest discover -p 'test_*.py' -v"
.\linode-runner.ps1 exec -- "cd $Exp && python3 integration_test.py --repo-root /opt/e-rust-port/source --binary $Binary"
```

Expected script result: 10 unit tests pass; the integration result proves two
hand cases, returns `Unknown` on the satisfiable case, and rejects nine
mutations.

## Train

```powershell
.\linode-runner.ps1 exec -- "cd $Exp && python3 run_experiment.py --phase train --repo-root /opt/e-rust-port/source --corpus $Exp/corpus.jsonl --problem-root $ProblemRoot --binary $Binary --proofcheck $ProofCheck --validation-gate $Gate --output-root $Root/train-v1 --workers 4"
.\linode-runner.ps1 exec -- "cd $Exp && python3 audit_results.py --root $Root/train-v1 --phase train --corpus corpus.jsonl --output $Root/train-audit.json"
.\linode-runner.ps1 exec -- "cd $Exp && python3 analyze.py --root $Root/train-v1 --phase train --corpus corpus.jsonl --output $Root/train-analysis.json"
```

## Validation

```powershell
.\linode-runner.ps1 exec -- "cd $Exp && python3 run_experiment.py --phase validation --repo-root /opt/e-rust-port/source --corpus $Exp/corpus.jsonl --problem-root $ProblemRoot --binary $Binary --proofcheck $ProofCheck --validation-gate $Gate --output-root $Root/validation-v1 --workers 4"
.\linode-runner.ps1 exec -- "cd $Exp && python3 audit_results.py --root $Root/validation-v1 --phase validation --corpus corpus.jsonl --output $Root/validation-audit.json"
.\linode-runner.ps1 exec -- "cd $Exp && python3 analyze.py --root $Root/validation-v1 --phase validation --corpus corpus.jsonl --output $Root/validation-analysis.json"
```

## Test and complete replication

The first full execution is retained as `test-v1`. Its audit must reject the
goal-priority `NUN085+1` theorem/resource repetition disagreement.

```powershell
.\linode-runner.ps1 exec -- "cd $Exp && python3 run_experiment.py --phase test --repo-root /opt/e-rust-port/source --corpus $Exp/corpus.jsonl --problem-root $ProblemRoot --binary $Binary --proofcheck $ProofCheck --validation-gate $Gate --validation-analysis $Root/validation-analysis.json --output-root $Root/test-v1 --workers 4"
.\linode-runner.ps1 exec -- "cd $Exp && python3 audit_results.py --root $Root/test-v1 --phase test --corpus corpus.jsonl --output $Root/test-audit.json"
```

Run the same complete matrix into a fresh root; do not replace one repetition:

```powershell
.\linode-runner.ps1 exec -- "cd $Exp && python3 run_experiment.py --phase test --repo-root /opt/e-rust-port/source --corpus $Exp/corpus.jsonl --problem-root $ProblemRoot --binary $Binary --proofcheck $ProofCheck --validation-gate $Gate --validation-analysis $Root/validation-analysis.json --output-root $Root/test-v2 --workers 4"
.\linode-runner.ps1 exec -- "cd $Exp && python3 audit_results.py --root $Root/test-v2 --phase test --corpus corpus.jsonl --output $Root/test-v2-audit.json"
.\linode-runner.ps1 exec -- "cd $Exp && python3 analyze.py --root $Root/test-v2 --phase test --corpus corpus.jsonl --output $Root/test-analysis.json"
.\linode-runner.ps1 exec -- "cd $Exp && python3 analyze.py --validation-analysis $Root/validation-analysis.json --test-analysis $Root/test-analysis.json --output $Root/final-decision.json"
```

## Resume and integrity

Rerun the three primary phase commands. Expected summaries are:

```text
train:      completed=0, resumed=12
validation: completed=0, resumed=24
test-v2:    completed=0, resumed=24
```

For the negative control, preserve the exact certificate outside the run root,
append one byte to:

```text
test-v2/runs/connection/NUN081+1/rep-1/worker/certificate.json
```

`audit_results.py` must report
`artifact_hash_mismatch:NUN081+1/connection/1`. Restore the exact certificate;
the audit must become clean and reproduce SHA-256
`ad60aa0825f01f2022b88e0d66a7a5bd31dea2771033eb9029abb875b0438c4a`.

## Result hashes

```text
train contract:       185f38cd12fc146fd52a09f7c1df2f74ba55d4beec731d03914f24eae8b876e4
train results:        4324d5edbac607b1c3f801aa7fc802eec33d6389f413860543c288046fac98fc
validation contract:  926ff286b179654418b37bfc45902c095c76d053929ad3801eb1263b7a967a24
validation results:   9ac04900600b6cf04b20f9a65992a620e5bef3738123b08f73e7c3848fa5ac92
test-v1 contract:     b12b563c5f2f7779c3d40efbc1f3492ded87fb5cd7210310546e9e18caf5c480
test-v1 results:      08fe276ecacf822ce8971bace1452ba8f77e660163e4cfd31dc87c742be11267
test-v2 contract:     b12b563c5f2f7779c3d40efbc1f3492ded87fb5cd7210310546e9e18caf5c480
test-v2 results:      97fd9d74917a632ff948cde72db01fe60c9d30be9fd9a2047ec2bc5afb7ba5e1
train analysis:       929b08896dda5b05706513c66200a5ea4348f448865aad54fb7f69d1a9084df2
validation analysis:  46d60196a390d00d489b5118c31a02e4fe860cea8f281c9cfd9087593f3e7007
test analysis:        938fe54aa81c30f3671869675ddd631d1c4aa2ca1194e865de804518c2cfa9b2
final decision:       e5d8b33cf5417d891648c5267db5ad7dde79c292ac16f418eb3482c244aa1d39
```

## Evidence archive

```powershell
.\linode-runner.ps1 exec -- "cd $Root && tar -czf evidence-v1.tar.gz train-v1 validation-v1 test-v2 test-v1/contract.json test-v1/results.jsonl test-v1/runs/goal_hard_priority/NUN085+1 train-audit.json train-analysis.json validation-audit.json validation-analysis.json test-audit.json test-v2-audit.json test-v2-audit-restored.json test-analysis.json final-decision.json negative-control-audit.json unit-tests.stdout.txt unit-tests.stderr.txt integration-test.json"
.\linode-runner.ps1 exec -- "sha256sum $Root/evidence-v1.tar.gz"
```

Expected archive SHA-256:

```text
095fd55e349f037d8c296c16b9055d858f364dcfaf5c8615bae9cec26f60b743
```

