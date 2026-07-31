# Cooperative multicore search experiment

This directory contains the preregistered process-isolated comparison for
Bead `E_Rust_Port-9jt.3.7`.

The experiment compares uninterrupted equal and unequal portfolios, a
restart-only control, and three periodic same-problem peer-watchlist caps. It
also measures the upper bound available from shared preprocessing. All Rust
builds and prover executions run through `linode-runner.ps1`.

Tracked scripts and final findings live here. Complete raw stdout, stderr,
wrappers, proofs, contracts, resource samples, and reports are retained in
the ignored `.artifacts/experiments/` tree.

The frozen comparison is complete. `share_64` added one reproducible
validation-only solve but no test solve, while the efficiency comparison had
too few common solved coordinates. The verdict is `uncertain`; production
remains unchanged. See [`FINDINGS.md`](FINDINGS.md) for the evidence and
[`COMMANDS.md`](COMMANDS.md) for exact reproduction commands.
