# Propositional SAT preprocessing

Bead: `E_Rust_Port-9jt.4.7`

This experiment evaluates modern SAT preprocessing on a deliberately exact
whole-problem propositional fragment and on every held-out SATCheck session
retained by the incremental SAT service study. It compares Umlaut's current
internal SAT service with pinned CaDiCaL 3.0.1 in `plain` and default
configurations.

The result is negative for production adoption. No Umlaut option, schedule,
backend, dependency, package, or production source file changes in this
experiment.

See `PREREGISTRATION.md` for the frozen corpus, budgets, correctness gates, and
decision thresholds, `COMMANDS.md` for exact runner commands, and
`FINDINGS.md` for the final evidence and decision.

## Components

- `run_experiment.py` prepares the frozen inputs, runs the process-isolated
  benchmark, certifies models and proofs, analyzes overlap, and applies the
  preregistered decisions.
- `cadical_probe.cpp` is an experiment-only adapter over CaDiCaL's public C++
  API. It separates insertion, `simplify(3)`, and solve costs and can emit DRAT
  and simplified DIMACS.
- `internal_probe.rs` is an experiment-only adapter over Umlaut's public
  `InternalSatService`. It totalizes the service's partial SAT assignment over
  all declared DIMACS variables before publication.
- `test_run_experiment.py` covers the exact fragment, ISAT query
  materialization, model and mapping validation, proof-checker output parsing,
  corruption rejection, and the frozen decision gates.

## Pinned inputs and runner

- Umlaut source commit: `24c833f3b36a3f4c6742392a02f3cabcacf012c2`
- Timed-run source snapshot:
  `51e458c1954a1a817a6d65b8770baa477577f25f5d1b772a4677137bf60b8315`
- Certification and analysis source snapshot:
  `bdacdf8be90d1cfbfa92a7e5cab00f6253b662aad1673c7ebe65364ed1384102`
- Comprehensive-validation source snapshot:
  `8af6586ade1900de78ba8dbae32155b453aacb409460911f11784c2096303654`
- Runner: `260731-021324-06a2`, Ubuntu 24.04, dedicated eight-core worker
- CaDiCaL 3.0.1:
  `c60730422e758ef1cebe7aeddf2dda31c996bf04`
- CASC-30 corpus archive:
  `efcebc55298d4c6770113c095e8cefdd77b9e8cbe3afa3078201f541893d1a7d`
- Incremental SAT service archive:
  `85356e073a26234f51e07898019d0a9a7685066eff21dd9350d621ede3158375`

The two large input archives remain ignored and separately retained. The
experiment uses only the pinned CaDiCaL public API and does not copy external
implementation code into Umlaut.

## Reproduction

Python-only checks may run locally:

```powershell
.\.venv\Scripts\python.exe -m unittest experiments/2026-07-30-007-propositional-sat-preprocessing/test_run_experiment.py
.\.venv\Scripts\python.exe -m py_compile experiments/2026-07-30-007-propositional-sat-preprocessing/run_experiment.py
```

Rust, CaDiCaL, proof-checker, and prover execution must use
`linode-runner.ps1`. The controller exposes four subcommands:

```text
run_experiment.py prepare ...
run_experiment.py run ... --workers 8 --repetitions 20
run_experiment.py certify ...
run_experiment.py analyze ...
```

The measured matrix contains 38,100 records: 635 query scopes, three arms, and
20 repetitions. Repeating `run` against the completed JSONL must print:

```text
completed 0 new records; resumed 38100
```

Raw artifacts are retained under
`.artifacts/experiments/2026-07-30-007-propositional-sat-preprocessing/`.
The final `results.tar.gz` SHA-256 is
`c1523f2126f3d976be63a1ac50b5b2417a8b993bbcd1d56b856eb055c45a631b`.
It includes both pre-finalization diagnostics described in `FINDINGS.md`, but
excludes the separately pinned input archives.

The separate comprehensive lifecycle archive is `comprehensive.tar.gz` at
SHA-256
`cc9ba84ea2d3e8196c99d46cd6e8b80cc84ad34759179beb0a553f0239714f28`.
