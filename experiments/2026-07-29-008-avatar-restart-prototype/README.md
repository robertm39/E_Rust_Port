# Bounded AVATAR restart prototype

Bead: `E_Rust_Port-9jt.4.2`

This experiment evaluates a clean-room, bounded AVATAR-style worker before any
live saturation-loop integration. It decomposes selected input CNF clauses into
variable-disjoint components, maps alpha-equivalent components to propositional
selectors, lets Umlaut's incremental SAT service choose active components, and
runs a fresh proof-producing Umlaut branch for each SAT model. A verified branch
refutation contributes the conservative conflict clause containing the negation
of every active selector.

The prototype is deliberately narrower than production AVATAR:

- only first-order CNF files without includes are accepted;
- only selected input clauses are split;
- the first-order prover restarts for each SAT model;
- no clause deletion, locking, or live assertion propagation is implemented;
- every claimed refutation requires independently checked branch proofs and an
  independently checked propositional/meta certificate.

The implementation does not inspect or copy Vampire source code. Its semantics
come from the published AVATAR papers listed in `PREREGISTRATION.md`.

The preregistered result is negative: the sound fail-closed worker verified one
branch conflict but no complete problem, lost one held-out verified baseline
solve, and produced no unique solve. Production splitting remains unchanged.
See [`FINDINGS.md`](FINDINGS.md).

Tracked components:

- `tptp_split.py`: restricted CNF parser, component decomposition, selector
  reuse, and branch rendering;
- `select_corpus.py` and `corpus.jsonl`: outcome-blind frozen CASC-30 cohorts;
- `avatar_sat_driver.rs` and `cargo-bin.patch`: disposable persistent bridge to
  Umlaut's incremental SAT service;
- `avatar_replay.py`: fixed three-method comparison and certificate generation;
- `verify_certificate.py`: independent parser, semantic replay, proof gate, and
  Python DPLL;
- `driver_integration.py` and `test_scripts.py`: protocol, semantic, and
  corruption tests;
- `prepare_corpus.py`, `analyze.py`, and `run_experiment.py`: packaging,
  reporting, and guarded Ubuntu orchestration; and
- `PREREGISTRATION.md` and `FINDINGS.md`: frozen contract and measured decision.
