# Reproduction commands

All Rust builds and prover executions ran on the retained Ubuntu runner through
`linode-runner.ps1`.

## Paths

```powershell
$Exp = "/opt/e-rust-port/source/experiments/2026-07-30-010-deterministic-adaptive-probes"
$Root = "/opt/e-rust-port/artifacts/adaptive-probe-010"
$Binary = "/opt/e-rust-port/source/target/release/umlaut"
$Manifest = "$Exp/corpus.jsonl"
$Problems = "$Root/corpus"
$CorpusReport = "$Root/corpus-report.json"
$ProofCheck = "/opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/proofcheck-linux-x86_64/proofcheck"
$Gate = "/opt/e-rust-port/source/tools/validation/validate_tptp_solution.py"
$Revision = "f03259698d81e8fbc25c8b64deb4e7c35e3ffd77"
```

## Source validation and release build

```powershell
.\linode-runner.ps1 exec -- cargo fmt `
  --manifest-path /opt/e-rust-port/source/Cargo.toml --all -- --check
.\linode-runner.ps1 exec -- cargo check `
  --manifest-path /opt/e-rust-port/source/Cargo.toml --all-targets
.\linode-runner.ps1 exec -- cargo clippy `
  --manifest-path /opt/e-rust-port/source/Cargo.toml --all-targets `
  -- -D warnings
.\linode-runner.ps1 exec -- cargo clippy `
  --manifest-path /opt/e-rust-port/source/Cargo.toml --all-targets `
  -- -D warnings -W clippy::pedantic
.\linode-runner.ps1 exec -- cargo test `
  --manifest-path /opt/e-rust-port/source/Cargo.toml --all-targets
.\linode-runner.ps1 exec -- cargo build `
  --manifest-path /opt/e-rust-port/source/Cargo.toml `
  --release --bin umlaut
```

## Controller tests and corpus

```powershell
python experiments/2026-07-30-010-deterministic-adaptive-probes/test_scripts.py
python -m py_compile `
  experiments/2026-07-30-010-deterministic-adaptive-probes/common.py `
  experiments/2026-07-30-010-deterministic-adaptive-probes/select_corpus.py `
  experiments/2026-07-30-010-deterministic-adaptive-probes/prepare_corpus.py `
  experiments/2026-07-30-010-deterministic-adaptive-probes/run.py `
  experiments/2026-07-30-010-deterministic-adaptive-probes/analyze.py `
  experiments/2026-07-30-010-deterministic-adaptive-probes/validate_results.py

.\linode-runner.ps1 exec -- python3 "$Exp/test_scripts.py"
.\linode-runner.ps1 exec -- python3 -m py_compile `
  "$Exp/common.py" "$Exp/select_corpus.py" "$Exp/prepare_corpus.py" `
  "$Exp/run.py" "$Exp/analyze.py" "$Exp/validate_results.py"

.\linode-runner.ps1 exec -- python3 "$Exp/prepare_corpus.py" `
  --archive /opt/e-rust-port/artifacts/prop-sat-007/casc_2025_corpus.tar.gz `
  --manifest "$Manifest" `
  --output-root "$Problems" `
  --report "$CorpusReport"
```

## Train

```powershell
.\linode-runner.ps1 exec -- python3 "$Exp/run.py" `
  --binary "$Binary" --manifest "$Manifest" `
  --problem-root "$Problems" --corpus-report "$CorpusReport" `
  --output-root "$Root/train-v2" --phase train `
  --source-revision "$Revision" --proofcheck "$ProofCheck" `
  --validation-gate "$Gate"

.\linode-runner.ps1 exec -- python3 "$Exp/analyze.py" `
  --root "$Root/train-v2" --phase train `
  --output "$Root/train-v2-analysis.json"

.\linode-runner.ps1 exec -- python3 "$Exp/validate_results.py" `
  --root "$Root/train-v2" --phase train `
  --problem-root "$Problems" --proofcheck "$ProofCheck" `
  --validation-gate "$Gate" `
  --replay-root "$Root/train-v2-validation-replays" `
  --output "$Root/train-v2-validation.json"
```

## Validation

```powershell
.\linode-runner.ps1 exec -- python3 "$Exp/run.py" `
  --binary "$Binary" --manifest "$Manifest" `
  --problem-root "$Problems" --corpus-report "$CorpusReport" `
  --output-root "$Root/validation-v1" --phase validation `
  --source-revision "$Revision" --proofcheck "$ProofCheck" `
  --validation-gate "$Gate"

.\linode-runner.ps1 exec -- python3 "$Exp/analyze.py" `
  --root "$Root/validation-v1" --phase validation `
  --output "$Root/validation-analysis.json"

.\linode-runner.ps1 exec -- python3 "$Exp/validate_results.py" `
  --root "$Root/validation-v1" --phase validation `
  --problem-root "$Problems" --proofcheck "$ProofCheck" `
  --validation-gate "$Gate" `
  --replay-root "$Root/validation-validation-replays" `
  --output "$Root/validation-validation.json"
```

## Test and decision

```powershell
.\linode-runner.ps1 exec -- python3 "$Exp/run.py" `
  --binary "$Binary" --manifest "$Manifest" `
  --problem-root "$Problems" --corpus-report "$CorpusReport" `
  --output-root "$Root/test-v1" --phase test `
  --validation-report "$Root/validation-analysis.json" `
  --source-revision "$Revision" --proofcheck "$ProofCheck" `
  --validation-gate "$Gate"

.\linode-runner.ps1 exec -- python3 "$Exp/analyze.py" `
  --root "$Root/test-v1" --phase test `
  --output "$Root/test-analysis.json"

.\linode-runner.ps1 exec -- python3 "$Exp/validate_results.py" `
  --root "$Root/test-v1" --phase test `
  --problem-root "$Problems" --proofcheck "$ProofCheck" `
  --validation-gate "$Gate" `
  --replay-root "$Root/test-validation-replays" `
  --output "$Root/test-validation.json"

.\linode-runner.ps1 exec -- python3 "$Exp/analyze.py" `
  --validation "$Root/validation-analysis.json" `
  --test "$Root/test-analysis.json" `
  --output "$Root/final-decision.json"
```

## Resume, integrity, and deterministic analysis

Rerun each phase command above against the same root. The accepted summaries
are `completed=0,resumed=56` for train and `completed=0,resumed=112` for both
held-out splits.

The negative control temporarily appends bytes to:

```text
test-v1/runs/probe_with_telemetry/FEQ/NUN086+2/rep-1/phase-1/stdout.txt
```

Running `analyze.py` must reject the changed `output_path` hash. Restore the
byte-exact backup before reanalysis. Then:

```powershell
.\linode-runner.ps1 exec -- python3 "$Exp/analyze.py" `
  --root "$Root/test-v1" --phase test `
  --output "$Root/test-reanalysis.json"
.\linode-runner.ps1 exec -- sha256sum `
  "$Root/test-analysis.json" "$Root/test-reanalysis.json"

.\linode-runner.ps1 exec -- python3 "$Exp/analyze.py" `
  --validation "$Root/validation-analysis.json" `
  --test "$Root/test-analysis.json" `
  --output "$Root/final-decision-reanalysis.json"
.\linode-runner.ps1 exec -- sha256sum `
  "$Root/final-decision.json" "$Root/final-decision-reanalysis.json"
```

## Evidence archive

```powershell
.\linode-runner.ps1 exec -- tar -czf "$Root/evidence-v1.tar.gz" `
  -C "$Root" `
  train-v2 validation-v1 test-v1 corpus-report.json `
  train-v2-analysis.json train-v2-validation.json `
  validation-analysis.json validation-validation.json `
  test-analysis.json test-validation.json final-decision.json

.\linode-runner.ps1 exec -- sha256sum "$Root/evidence-v1.tar.gz"
```

Expected archive SHA-256:

```text
910ccaf961ea6c906d90cc35778f08fb95dfea7e0115d02b181a8e8912ea3a87
```
