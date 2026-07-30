# AC normalization mode evaluation

This experiment evaluates Umlaut's existing proof-producing
associative-commutative redundancy modes on every CASC-30 UEQ/FEQ problem
whose presented source contains an explicit associativity axiom and a matching
commutativity axiom for at least one binary symbol.

The experiment is associated with Bead `E_Rust_Port-9jt.6.5`. Read
`PREREGISTRATION.md` before running it. The durable results are written outside
Git and summarized in `FINDINGS.md` after all checks complete.

The harness deliberately reuses the contract/resume implementation from
`experiments/2026-07-28-007-unit-equality-completion/run.py`. The wrapper pins
that base harness's SHA-256 into every strategy record.

Typical Linux execution:

```text
python3 audit.py \
  --manifest ../../benchmarks/casc_2025_manifest.jsonl \
  --problem-root /opt/e-rust-port/source \
  --output audit.json

python3 run.py --phase calibration \
  --manifest ../../benchmarks/casc_2025_manifest.jsonl \
  --problem-root /opt/e-rust-port/source \
  --binary ../../target/release/umlaut \
  --output-root /opt/e-rust-port/ac-runs --workers 8

python3 analyze.py \
  --experiment-root /opt/e-rust-port/ac-runs \
  --output /opt/e-rust-port/ac-runs/summary.json \
  --markdown /opt/e-rust-port/ac-runs/RESULTS.md
```

Run `run.py` once for each of `calibration`, `validation`, and `test`.
`verify.py` is a narrow wrapper around the independently checkable ProofCheck
gate used by the earlier unit-equality experiment.
