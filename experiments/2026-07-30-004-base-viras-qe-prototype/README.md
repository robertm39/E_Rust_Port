# Clean-room base VIRAS QE prototype

Bead: `E_Rust_Port-9jt.5.2`

This experiment implements the paper-derived one-conjunction base VIRAS kernel
inside this folder. It uses exact rationals, symbolic profiles and grids,
finite virtual substitution, derivation records, and explicit `Unknown`
outcomes for unsupported or resource-limited requests.

The supported surface and frozen advancement gates are in
`PREREGISTRATION.md`. This is deliberately not a production arithmetic mode:
the arbitrary Boolean wrapper and the typed frontend contract from experiment
023 remain outside the milestone.

Run the dependency-free focused tests from the repository root:

```text
python experiments/2026-07-30-004-base-viras-qe-prototype/test_prototype.py -v
```

Run the frozen seeded experiment with a caller-supplied pinned Z3 executable:

```text
python experiments/2026-07-30-004-base-viras-qe-prototype/run_experiment.py \
  --z3 /absolute/path/to/z3 \
  --seed 0xB451E2026 \
  --cases 1000 \
  --output .artifacts/experiments/2026-07-30-004-base-viras-qe-prototype/report.json
```

The candidate imports no Umlaut arithmetic implementation and never inspects
the unlicensed VIRAS source tree.
