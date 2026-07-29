# Inference and simplification gap audit

This experiment supports Bead `E_Rust_Port-9jt.7.5`. It maps reference-prover
rule names to Umlaut semantics, production reachability, proof ancestry, and
executable witnesses. It then evaluates the highest-ranked low-risk candidate:
the already-implemented but default-off local/inner rewriting path.

The frozen protocol and decision rules are in
[`PREREGISTRATION.md`](PREREGISTRATION.md). The machine-readable capability
matrix is [`capability-matrix.json`](capability-matrix.json).

No Rust build, Rust test, prover, or reference-prover command may run locally.
The static matrix controller may run locally:

```powershell
python experiments/2026-07-29-013-inference-gap-audit/audit_matrix.py `
  --repo-root .
```

All empirical commands use the ephemeral Ubuntu runner. Raw proof objects,
telemetry, timing samples, and extracted corpus files remain outside Git.

