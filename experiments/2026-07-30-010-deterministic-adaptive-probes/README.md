# Deterministic adaptive probe experiment

This directory contains the frozen follow-up for Bead
`E_Rust_Port-9jt.3.10`. It replaces hard-stop-only online-adaptation probes
with deterministic processed-clause checkpoints and an atomic pre-input
fallback record, measures telemetry overhead, and re-evaluates the prior
clause-growth branch rule against equal-budget static restart controls.

All Rust builds and prover executions run on Linux through
`linode-runner.ps1`.

The atomic checkpoint reached 100% held-out observability, but the frozen
policy verdict is `stop`: held-out wall overhead exceeded 1.05, adaptive search
added no solve versus both controls, and it lost one test solve versus the
static goal continuation. See [FINDINGS.md](FINDINGS.md) and
[COMMANDS.md](COMMANDS.md).
