# Proof-derived TSM learning baseline

This experiment evaluates Bead `E_Rust_Port-9jt.3.3`.

`PREREGISTRATION.md` freezes the non-leaking family split, structure-matched
control, budgets, calibration metrics, ranking-cost measurement, and decision
rule. `select_corpus.py` creates the tracked `corpus.jsonl`.

Raw traces, knowledge bases, search outputs, telemetry, classifier output, and
timing samples belong under
`.artifacts/experiments/2026-07-29-018-tsm-learning-baseline/`.

The measured prover revision is
`812323618aaa42d0f5e24bba8a0ef146ff1757cd`. This supersedes the invalid
training-only attempt at `9263a0d362d5ec297f9e5305870b641626de107e`; see the
transparent amendment in `PREREGISTRATION.md`.

Classifier label extraction reruns only already-successful repetition-1
control coordinates to serialize PCL traces; it verifies that every rerun
preserves the recorded SZS status.
