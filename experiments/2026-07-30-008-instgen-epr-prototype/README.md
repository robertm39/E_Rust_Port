# Bounded model-guided EPR instantiation

This experiment evaluates Bead `E_Rust_Port-9jt.4.5` with a clean-room,
equality-free, function-free CNF prototype. The worker uses a propositional
model to find false finite-domain ground instances, adds those instances, and
repeats until it has a checked refutation, a complete finite Herbrand model, or
its fixed budget expires.

The experiment is isolated from production. See `PREREGISTRATION.md` for the
frozen fragment, corpus, budgets, proof boundary, and decision rule.

The result is negative. On held-out validation/test, saturation, the
equal-budget portfolio, and cooperation reproducibly solved the same seven
problems. Standalone instantiation solved four, added no solve, and produced no
refutation. Exchanging 4,353 replayable instances added no solve; cooperation
used 18.30 times saturation's median user CPU on common measured solves.
Production remains unchanged. See `FINDINGS.md`.

Tracked components:

- `select_corpus.py` and `corpus.jsonl`: syntax-only family-separated selection;
- `instgen.py` and `cadical_driver.cpp`: bounded model-guided grounding and the
  experiment-only public CaDiCaL adapter;
- `verify_certificate.py`: independent source-instance, model, DIMACS, and
  DRAT replay;
- `run_experiment.py`, `analyze.py`, and `validate_results.py`: equal-budget
  comparison, frozen decision, and full artifact replay;
- `test_instgen.py` and `integration_test.py`: focused and corruption tests;
- `prepare_corpus.py`: hash-checked extraction of selected CASC sources; and
- `PREREGISTRATION.md`, `RESULTS.md`, `FINDINGS.md`, and `COMMANDS.md`: frozen
  contract, compact results, decision record, and reproducibility commands.
