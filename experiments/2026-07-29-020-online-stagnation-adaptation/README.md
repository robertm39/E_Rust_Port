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

The final verdict is `uncertain`: all policy arms reproduced the same one solve
per held-out split, while 14 of 16 test adaptive probes hard-stopped before
decision telemetry was written. The controller therefore took its deterministic
fallback too often to judge the clause-growth signal. See `FINDINGS.md`,
`CALIBRATION.md`, `VALIDATION.md`, and `results-summary.json`.

The complete ignored raw archive is
`.artifacts/experiments/2026-07-29-020-online-stagnation-adaptation/online-adaptation-020-complete.tar.gz`
(29,733,382 bytes, SHA-256
`5594302c52397cd5d3aaff29fd7efe90bdf60620e71dcac39d571a91c7f7a5cc`).
