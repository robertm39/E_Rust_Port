# Conflict-driven VIRAS feasibility

This experiment evaluates Bead `E_Rust_Port-9jt.5.4` without changing Umlaut's
production behavior.

The frozen protocol is in [PREREGISTRATION.md](PREREGISTRATION.md). The
prototype will reuse the clean-room exact base-VIRAS kernel from Experiment
004, compare eager enumeration with two learned-search controls, validate
every inserted clause through a separate exact affine feasibility checker,
and retain complete deterministic derivation traces.

Only the finite equality-guarded affine slice is eligible. Epsilon, infinity,
periodic, and grid learning remain explicit production blockers.

The completed outcome is documented in [FINDINGS.md](FINDINGS.md). All frozen
gates passed, but production remains deferred: basic learning was slower than
eager enumeration, while the fast focused control reduced every UNSAT conflict
to an empty clause using a complete affine feasibility check.

Run the focused tests with:

```text
python3 -m unittest discover \
  -s experiments/2026-07-30-014-conflict-driven-viras \
  -p 'test_*.py' -v
```

Run the frozen comparison on Ubuntu with:

```text
python3 experiments/2026-07-30-014-conflict-driven-viras/run_experiment.py \
  --z3 /path/to/z3 \
  --output /path/to/report.json \
  --trace-output /path/to/traces.jsonl.gz
```
