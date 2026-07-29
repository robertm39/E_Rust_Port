# SAT-based subsumption crossover experiment

Bead: `E_Rust_Port-9jt.4.3`

This experiment evaluates a clean-room Boolean encoding of first-order clause
subsumption and subsumption resolution against Umlaut's existing recursive
matcher. It does not copy or link any reference-prover implementation.

The experiment was preregistered in
[`PREREGISTRATION.md`](PREREGISTRATION.md). The measured negative integration
decision and evidence hashes are in [`FINDINGS.md`](FINDINGS.md); the complete
post-hoc threshold surface is in
[`POSTHOC_CROSSOVER.csv`](POSTHOC_CROSSOVER.csv).

Tracked components:

- `sat_subsumption.rs`: experiment-only Rust encoding and capture support;
- `capture.patch`: reversible wiring into the existing bank-aware checker;
- `capture.py`: family-separated CASC-30 workload capture;
- `analyze.py`: correctness, crossover, memory, and pruning analysis;
- `oracle.py`: independent exhaustive and randomized semantic oracle;
- `prepare_corpus.py`: minimal hash-checked CASC-30 corpus packaging;
- `test_scripts.py`: focused contract tests; and
- `run_experiment.py`: guarded Ubuntu 24.04 experiment controller;
- `posthoc_surface.py`: post-decision crossover-surface exporter; and
- `POSTHOC_CROSSOVER.csv`: all populated threshold regimes from the retained
  captures.

Rust compilation, tests, prover execution, and performance measurement must run
through `.\linode-runner.ps1`. The tracked patch is applied only to a disposable
runner worktree and is not a production source change.
