# Compact proof-trace storage

This experiment supports Bead `E_Rust_Port-9jt.8.2`. It evaluates a
deterministic framed proof-output log, exact reconstruction, independent
checking, spooling, and fail-closed recovery before deciding whether any live
search representation should change.

The frozen design and decision rules are in
[`PREREGISTRATION.md`](PREREGISTRATION.md). Raw proofs, checker binaries,
profiles, timing samples, and malformed logs are retained outside Git.

The study is complete. The output log passed: four proofs reconstructed
deterministically, all 12 original/replayed proof objects were independently
`VerifiedGood`, aggregate storage was 4.34% of eager bytes, and every malformed
or interrupted replay failed closed. Production remains unchanged because a
post-render byte log cannot release the live derivation parents and archived
formula/clause bodies responsible for search memory. See
[`FINDINGS.md`](FINDINGS.md) and [`results.json`](results.json).

No prover or checker command may run locally. Controller tests may run locally
after the harness is added; all empirical work uses the Ubuntu runner.

Local controller test:

```powershell
python experiments/2026-07-29-012-compact-proof-trace/test_proof_trace.py
```

Remote experiment command:

```bash
python3 experiments/2026-07-29-012-compact-proof-trace/run_experiment.py \
  --repo-root /opt/e-rust-port/source \
  --artifact-root /opt/e-rust-port/compact-proof-trace/results \
  --umlaut /opt/e-rust-port/source/target/release/umlaut \
  --proofcheck /opt/e-rust-port/compact-proof-trace/proofcheck-linux-x86_64/proofcheck \
  --held-out-root /opt/e-rust-port/compact-proof-trace/held-out \
  --source-commit SOURCE_COMMIT \
  --source-snapshot-sha256 SOURCE_SNAPSHOT_SHA256 \
  --repetitions 25
```
