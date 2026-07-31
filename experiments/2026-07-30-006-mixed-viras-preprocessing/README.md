# Mixed-problem VIRAS preprocessing evaluation

This experiment validates the feature-gated, explicit VIRAS preprocessing
path implemented for `E_Rust_Port-9jt.5.12`.

The held-out surface is every untouched CASC-2025 `TFI` document under
`problems/casc_2025/TFI`, grouped by its TPTP problem-family prefix. No member
of that corpus is used to design or tune the transformation. The evaluation
compares ordinary `umlaut` with the same binary plus
`--viras-qe-preprocess`, checks proof publication and deterministic
corruption rejection, and reports formula-level coverage, saturation solve
delta, latency, and formula growth.

Run the controller only on the repository's Ubuntu runner. Raw reports and
command logs belong under the ignored
`.artifacts/experiments/2026-07-30-006-mixed-viras-preprocessing/` tree.

The TFI documents require the standard sibling `Axioms/` include tree. The
controller sets `TPTP` to the parent of the supplied `TFI` directory without
editing or extracting formulas:

```text
python3 experiments/2026-07-30-006-mixed-viras-preprocessing/run_evaluation.py \
  --umlaut-binary /opt/e-rust-port/source/target/release/umlaut \
  --tfi-corpus /opt/e-rust-port/artifacts/viras-006/corpus-full/TFI \
  --repo-root /opt/e-rust-port/source \
  --output /opt/e-rust-port/artifacts/viras-006/report-full.json \
  --workers 8 \
  --timeout 20
```
