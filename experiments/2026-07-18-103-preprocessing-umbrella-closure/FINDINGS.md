# Preprocessing umbrella closure

## Status

Completed for Bead `E_Rust_Port-j76.2.36`. The migrated umbrella is fully
owned by narrower closed Beads and retained executable references. The audit
found one evidence gap rather than a production defect: induction
preinstantiation had exact option materialization and direct helper tests, but
no retained C/Rust executable case in which the pass generated a clause. The
new two-case comparison closes that gap. The vendored C checkout remained
unchanged.

## Ownership decision

The umbrella does not define a separate implementation boundary. Its phases
are owned by these completed slices:

| Surface | Owner |
| --- | --- |
| SInE formula owners | `E_Rust_Port-j76.2.37` |
| formula relevance | `E_Rust_Port-j76.2.38` |
| BCE, predicate elimination, goal definitions, and presaturation | `E_Rust_Port-j76.2.39` |
| higher-order option effects and induction routing | `E_Rust_Port-j76.2.43` |
| option-to-parameter and `ProofControl` bridges | `E_Rust_Port-j76.2.46`, `E_Rust_Port-j76.2.47` |
| AC scan and activation | `E_Rust_Port-j76.2.57` |
| clause preprocessing, archive copies, equality unfolding, and watchlists | `E_Rust_Port-j76.2.61` |
| defined-choice recognition | `E_Rust_Port-j76.2.105` |

The C and Rust production order is the same: SInE, relevance, formula CNF,
archive copy and optional clause preprocessing, equality-definition unfolding,
choice recognition, induction preinstantiation, BCE, predicate elimination,
goal definitions, initial documentation, proof-control initialization, and
later presaturation. Both implementations retain the unusual C boundary where
`--no-preprocessing` skips `ClauseSetPreprocess` but does not skip equality
definition unfolding.

## Fresh induction comparison

[`compare_induction.py`](compare_induction.py) runs unchanged upstream C commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release executable on
[`induction.p`](induction.p), once with induction preinstantiation disabled and
once enabled.

Both implementations behave identically:

- the disabled case starts saturation with the two original logical clauses;
- the enabled case adds the same `g(f(b)) = g(b)` trigger instance in the same
  final clause-set position;
- the initial saturation count and literal count both increase from two to
  three;
- parsed, initial, and clause-preprocessing counters agree; and
- both cases return exit code 0, status `Unknown`, and empty stderr.

The retained [`reference.json`](reference.json) is exact in 2/2 cases and has
SHA-256 `221DF9DF39DC6350F51D5DE2AF2EBF5ED558F8097A53F1894A33C1A22EE8C7BC`.
The compared Rust executable has SHA-256
`9BA09DFDDFA8EE9119BC78011AA7CD227FBDC4D77134455EFE36E07174404ED6`.

## Combined audit

[`audit_preprocessing_umbrella.py`](audit_preprocessing_umbrella.py) validates
29/29 facts:

- all nine dedicated owner Beads are closed;
- C and Rust preserve the complete phase order and the no-preprocessing/equality
  unfolding boundary;
- every named umbrella field reaches the C-shaped `HeuristicParmsCell` and
  active proof-control path;
- permanent regressions cover option parsing, materialization, archive copies,
  formula-origin relevance/SInE/clause passes, watchlist unfolding,
  live induction generation, presaturation, and AC activation; and
- retained references for equality unfolding, goal definitions, predicate
  elimination, BCE, AC scanning, higher-order option effects, clausal formula
  ownership, relevance, SInE, and live induction are exact.

The retained [`owner-audit.json`](owner-audit.json) has SHA-256
`8DC71196565D0BFF6D847D9DC8DC96B43BB9725AA10F9427FB280F0BBB46F86A`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-103-preprocessing-umbrella-closure\compare_induction.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/eprover-ho `
  --rust-exe target\release\eprover.exe `
  --output target\preprocessing-umbrella-induction.json `
  --expected experiments\2026-07-18-103-preprocessing-umbrella-closure\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-103-preprocessing-umbrella-closure\audit_preprocessing_umbrella.py `
  --induction-reference experiments\2026-07-18-103-preprocessing-umbrella-closure\reference.json `
  --output target\preprocessing-umbrella-owner-audit.json
```
