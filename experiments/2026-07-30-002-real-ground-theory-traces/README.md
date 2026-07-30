# Real ground-theory branch traces

Bead: `E_Rust_Port-9jt.5.10`

This experiment evaluates proof-aware ground arithmetic cooperation on
production-like branch streams derived from real CASC-30 TFA problems.
Umlaut's production `--cnf` path supplies the normalized clauses. An
experiment-only, deterministic grounding and propositional search layer turns
those clauses into bounded branch streams while preserving source, clause,
literal, and grounding ancestry.

The comparison keeps production unchanged and evaluates four dispatch modes:

1. no theory checker;
2. a dependency-free native difference-logic checker;
3. the pinned persistent Z3 process protocol from
   `2026-07-30-001-ground-theory-smt-cooperation`; and
4. that experiment's pinned C API prototype.

Every accepted arithmetic decision must carry an exact model or negative-cycle
core that is independently replayable. Unsupported terms, strict real
inequalities, missing evidence, timeouts, cancellation, and backend errors are
`Unknown`.

See `PREREGISTRATION.md` for the frozen protocol and advancement gates. Raw
outputs belong under
`.artifacts/experiments/2026-07-30-002-real-ground-theory-traces/`.

The completed result is in `FINDINGS.md`: every correctness, evidence,
determinism, cancellation, latency, package, and neutral no-loss gate passed,
but the held-out efficacy gates failed. Production therefore remains
unchanged.

## Tracked experiment components

- `select_sources.py` applies the frozen family-separated source selection.
- `capture_cnf.py` captures production `umlaut --cnf --tstp-out` output.
- `trace_model.py` parses, grounds, and searches the bounded abstraction.
- `build_traces.py` materializes no-checker branch traces.
- `reference_search.py` runs the independent exact difference-logic oracle.
- `prepare_query_corpus.py` writes native, replay, and backend protocols while
  retaining the original TPTP query ID as provenance.
- `native_difference_driver.rs` is the dependency-free Rust checker.
- `run_backend_comparison.py` measures native, pinned-Z3 process, and
  pinned-Z3 FFI modes after a warmup for five repetitions.
- `mutate_certificates.py` requires six corrupt evidence classes to fail.
- `measure_native_package.py` measures the native checker against an
  identical-profile empty Rust binary.
- `analyze_heldout.py` recomputes every held-out advancement gate.

The `cargo-bin.patch` file registers experiment-only binaries against a
temporary runner copy of `Cargo.toml`. It is not applied to the production
tree.

## Rechecking retained artifacts

From the repository root, after restoring the ignored artifact tree:

```powershell
python -m unittest discover -v `
  -s experiments/2026-07-30-002-real-ground-theory-traces `
  -p 'test_*.py'

python experiments/2026-07-30-002-real-ground-theory-traces/analyze_heldout.py `
  --artifact-root .artifacts/experiments/2026-07-30-002-real-ground-theory-traces `
  --selection-root experiments/2026-07-30-002-real-ground-theory-traces `
  --package-report .artifacts/experiments/2026-07-30-002-real-ground-theory-traces/native-package.json `
  --output .artifacts/experiments/2026-07-30-002-real-ground-theory-traces/heldout-analysis.json
```

All Rust builds, tests, replay, solver execution, and benchmarks must use
`linode-runner.ps1` as required by `DOCS.md`.
