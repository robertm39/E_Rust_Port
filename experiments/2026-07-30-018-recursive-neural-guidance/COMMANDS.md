# Commands

Run Python-only correctness tests locally:

```powershell
$env:PYTHONDONTWRITEBYTECODE = "1"
python experiments/2026-07-30-018-recursive-neural-guidance/test_neural_guidance.py -v
```

Synchronize to the retained Ubuntu runner and repeat the tests:

```powershell
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- cd /opt/e-rust-port/source/experiments/2026-07-30-018-recursive-neural-guidance '&&' PYTHONDONTWRITEBYTECODE=1 python3 test_neural_guidance.py -v
```

Run a validation phase. The output directory must not already exist:

```text
cd /opt/e-rust-port/source/experiments/2026-07-30-018-recursive-neural-guidance
python3 run_study.py validation \
  --archive /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018.tar.gz \
  --output /opt/e-rust-port/artifacts/neural-guidance-018/validation-v2
```

Audit, summarize, and compare the two complete replications:

```text
python3 audit_results.py \
  /opt/e-rust-port/artifacts/neural-guidance-018/validation-v2/validation-result.json
python3 analyze.py \
  /opt/e-rust-port/artifacts/neural-guidance-018/validation-v2/validation-result.json
python3 compare_results.py \
  /opt/e-rust-port/artifacts/neural-guidance-018/validation-v1/validation-result.json \
  /opt/e-rust-port/artifacts/neural-guidance-018/validation-v2/validation-result.json
```

The conditional `test` phase is documented by `run_study.py`, but it was not
run because `validation-result.json` has verdict
`stop-offline-validation`; the script rejects that result as test authority.
