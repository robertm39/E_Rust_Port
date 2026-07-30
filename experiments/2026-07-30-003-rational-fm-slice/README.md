# Rational/real Fourier–Motzkin slice

Bead: `E_Rust_Port-9jt.5.6`

This experiment maps and evaluates a deliberately small ALASCA-style
arithmetic inference slice: exact polynomial normalization plus
Fourier–Motzkin inference over rational or real linear inequalities.

The prototype is experiment-only. It does not copy Vampire implementation
code, inspect VIRAS implementation code, alter production Umlaut, or add a
dependency. Vampire's BSD-3-Clause ALASCA sources are used as architectural
reference; the pinned local-only Vampire binary and pinned MIT-licensed Z3
source are external controls.

See `PREREGISTRATION.md` for the frozen boundary and advancement gates. Raw
artifacts belong under
`.artifacts/experiments/2026-07-30-003-rational-fm-slice/`.

## Components

- `fm_core.py` implements exact normalization, propositional resolution, and
  the bounded Fourier-Motzkin slice.
- `fm_replay.py` independently reconstructs every trusted inference.
- `generate_corpus.py` creates the frozen family-separated synthetic corpus.
- `select_production.py`, `capture_production.py`, and
  `extract_production.py` implement the preregistered CASC-30 source path.
- `render_controls.py`, `run_external.py`, and `run_source_controls.py`
  drive the pinned Z3 and Vampire controls.
- `run_native.py` performs one warmup plus five measured native repetitions.
- `run_robustness.py` records the mutation and fail-closed test suite.
- `analyze_results.py` scores every frozen gate.
- `FINDINGS.md` reports the final evidence and recommendation.

## Local Python checks

Python experiment code may run locally under the repository execution policy:

```text
python generate_corpus.py --output synthetic_corpus.json
python -m unittest -v test_fm.py test_production.py
python run_native.py synthetic_corpus.json --output-directory ARTIFACT_DIR
```

Rust Umlaut capture and every Z3/Vampire executable invocation must run on the
Ubuntu worker through `linode-runner.ps1`. The external controls use pinned Z3
source commit `2d48fd119ce5074b880944c2b1c59e537c99cd46` and the canonical
Vampire 5.0.1 artifact documented in `DOCS.md`. The ALASCA arm spells out
Vampire's full option names and explicitly disables VIRAS; no external binary
or source is distributed with this experiment.
