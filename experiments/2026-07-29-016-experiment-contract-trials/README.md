# Experiment-contract version-1 trials

This directory evaluates Bead `E_Rust_Port-9jt.2.5` by applying the reusable
experiment-result contract to two completed, independently frozen studies:

- `rewrite-cache-result.json` is a performance comparison between the
  production shared rewrite cache and its proof-preserving recomputation
  ablation; and
- `bce-toggle-result.json` is a default-off preprocessing toggle comparing
  `--bce=true` with the production baseline.

No prover run was added or reinterpreted. `verify_trials.py` reads all 364
preserved raw run records from their integrity-pinned ignored archives,
recomputes the selected solve coverage, status pairing, common-solve CPU
ratios, and within-coordinate repeat variation, and then compares independent
proof and source-decision fields with the original compact analyzers.

From the repository root:

```text
python -m unittest \
  tools/experiment_contract/test_validate.py \
  experiments/2026-07-29-016-experiment-contract-trials/test_verify_trials.py

python tools/experiment_contract/validate.py \
  --verify-artifacts \
  experiments/2026-07-29-016-experiment-contract-trials/rewrite-cache-result.json \
  experiments/2026-07-29-016-experiment-contract-trials/bce-toggle-result.json

python experiments/2026-07-29-016-experiment-contract-trials/verify_trials.py \
  --verify-artifacts
```

See [`FINDINGS.md`](FINDINGS.md) for the outcome and
[`docs/experiment-contract.md`](../../docs/experiment-contract.md) for the
reusable practice.
