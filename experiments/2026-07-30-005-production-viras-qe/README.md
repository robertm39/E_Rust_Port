# Production base VIRAS QE evaluation

This experiment validates the opt-in Rust base VIRAS kernel, typed importer,
standalone CLI, and package boundary implemented for `E_Rust_Port-9jt.5.11`.

The evaluation has two held-out surfaces:

- every untouched CASC-2025 `TFI` document under `problems/casc_2025/TFI`,
  used to measure real document coverage and rejection reasons; and
- six deterministic, analytically decidable TFA families that do not reuse
  the prototype's seeded conjunction generator, used for exact semantics,
  latency, formula growth, and complementarity with default `umlaut`.

Run only on the repository's Ubuntu runner:

```text
python3 experiments/2026-07-30-005-production-viras-qe/run_evaluation.py \
  --viras-binary /opt/e-rust-port/source/target/release/umlaut-viras-qe \
  --umlaut-binary /opt/e-rust-port/source/target/release/umlaut \
  --tfi-corpus /opt/e-rust-port/source/problems/casc_2025/TFI \
  --output /opt/e-rust-port/artifacts/viras-005/report.json
```

The tracked files contain the frozen design and summarized findings. Raw JSON
reports and command logs belong under the ignored
`.artifacts/experiments/2026-07-30-005-production-viras-qe/` tree.
