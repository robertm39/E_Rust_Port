# Protocol amendment 1

Date: 2026-07-29

Timing: before any Rust build, test, prover run, or benchmark result from this
experiment.

`PREREGISTRATION.md` names a Cargo feature for the proof-preserving cache
ablation. Implementation review found that a semantic ablation must not be
included in Cargo's ordinary `--all-features` production quality gate.

The control is therefore activated by the build-only environment variable
`UMLAUT_EXPERIMENT_REWRITE_CACHE_ABLATION`. `build.rs` converts its presence
into the checked internal cfg `umlaut_rewrite_cache_ablation` and declares the
environment input for Cargo rebuild invalidation. The control semantics,
workloads, metrics, correctness requirements, and decision thresholds are
unchanged. The ablation binary must use a separate Cargo target directory, and
the run contract must record both binary hashes.

