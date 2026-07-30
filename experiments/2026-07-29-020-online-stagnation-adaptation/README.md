# Online stagnation adaptation

This experiment evaluates a bounded, deterministic outer-loop policy for
`E_Rust_Port-9jt.3.6`.

The policy runs a one-CPU-second global age/weight probe, reads Umlaut search
telemetry, and restarts for four CPU seconds with either the same heuristic or
goal hard priority. It is compared with an equal-budget static restart and an
equal-budget static global-to-goal portfolio. The restart boundary deliberately
avoids mutating a live proof state, so completeness and proof production remain
those of ordinary Umlaut runs.

The frozen design and decision rules are in `PREREGISTRATION.md`. Generated raw
artifacts belong under:

```text
.artifacts/experiments/2026-07-29-020-online-stagnation-adaptation/
```

All prover execution must use `linode-runner.ps1` on Ubuntu. Local Python is
used only for controller tests, candidate-blind corpus construction, and
artifact analysis.
