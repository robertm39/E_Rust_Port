# Recursive neural clause-guidance feasibility

This experiment addresses Bead `E_Rust_Port-9jt.3.4`.

It performs a family-held-out, offline ranking study over proof-bearing
given-clause traces captured by the earlier
[`2026-07-29-018-tsm-learning-baseline`](../2026-07-29-018-tsm-learning-baseline/)
experiment. The candidate is a dependency-free frozen recursive encoder with
a small trained neural ranking head. It is compared with chronological given
order and an interpretable linear structural model before any online prover
integration is permitted.

The split, labels, metrics, resource budgets, and advancement gates are frozen
in [`PREREGISTRATION.md`](PREREGISTRATION.md). Raw archives and generated run
artifacts belong below
`.artifacts/experiments/2026-07-30-018-recursive-neural-guidance/`.

The completed study stopped at validation. The recursive candidate did not
beat the linear baseline on AP or top-10% recall, did not meet the simulated
proof-prefix effect, and exceeded the in-process latency threshold. Test and
online runs were therefore not performed. See [`FINDINGS.md`](FINDINGS.md).
