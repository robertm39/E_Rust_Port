# Nonlinear arithmetic feasibility study

This experiment supports Bead `E_Rust_Port-9jt.5.8`. Its preregistration
freezes the demand taxonomy, smallest candidate fragment, pinned-Z3 protocol,
trust boundary, cost rubric, and decision rule before solver execution.

The tracked harness will:

1. verify and classify the complete CASC-30 manifest;
2. translate only whole-problem pure real polynomial TFF;
3. compare pinned Z3 delegation with a fail-closed `Unknown` baseline;
4. retain proof-generation and failure diagnostics; and
5. inventory the implementation and proof obligations of NLSAT and
   model-based projection.

No experiment artifact is a production dependency or an accepted Umlaut proof
step.

## Reproduction

Run the focused tests:

```text
python3 experiments/2026-07-30-013-nonlinear-arithmetic-feasibility/test_run_experiment.py -v
```

Create the complete local inventory and content-addressed queries:

```text
python3 experiments/2026-07-30-013-nonlinear-arithmetic-feasibility/run_experiment.py \
  --repo-root . \
  --manifest benchmarks/casc_2025_manifest.jsonl \
  --z3-source-root z3 \
  --output /absolute/path/inventory.json \
  --query-dir /absolute/path/queries \
  --inventory-only
```

On Ubuntu, resume that inventory with the pinned Z3 build:

```text
python3 experiments/2026-07-30-013-nonlinear-arithmetic-feasibility/run_experiment.py \
  --repo-root . \
  --manifest benchmarks/casc_2025_manifest.jsonl \
  --z3 /opt/e-rust-port/z3-build/z3 \
  --z3-source-root /opt/e-rust-port/z3-src \
  --output /absolute/path/report.json \
  --query-dir /absolute/path/queries \
  --inventory-input /absolute/path/inventory.json
```

The resume stage verifies every query hash against the inventory before making
a solver call. See `FINDINGS.md` for the measured result and recommendation.
