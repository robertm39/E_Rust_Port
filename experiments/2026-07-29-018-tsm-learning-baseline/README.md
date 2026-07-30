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
preserves the recorded SZS status. The PCL proof-parent retention fix at
`477fa727355bace7de39d043d9b18734bd16adf4` is the label-extraction revision
only; measured search results remain frozen at `812323618aaa42d0f5e24bba8a0ef146ff1757cd`.
Classifier metadata records the label-extraction revision and binary hash, and
new label traces are isolated below the fresh classifier-input root. A split
with no successful repetition-1 control proof is recorded as unavailable
instead of receiving fabricated labels; its classifier workload is skipped and
the frozen insufficient-coverage rule yields an `uncertain` decision. Classifier
commands use the executable's `IndexIdentity` spelling for the frozen identity
index mode and retain warm-up diagnostics on failure. Classifier execution uses
`fc72bb24ee57fd796b19657621c0ff32c2afc4a5`, which combines the KB
clause-pattern parser fix with fixed helper-code reservation for production
symbol sets. The controller records that separate revision and its binary hash.
This does not change the frozen measured search or label-extraction revisions.
One-class held-out labels are reported explicitly with one-sided and
calibration metrics; balanced accuracy remains unavailable and no missing
class is fabricated.
