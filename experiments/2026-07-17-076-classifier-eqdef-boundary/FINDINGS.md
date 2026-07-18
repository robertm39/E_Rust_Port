# Classifier Equality-definition Boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.61`. Rust now preserves every production
caller boundary for `ccl_unfold_defs`: proof search performs the separate
equality-definition normalization pass, prune exits before it, and
`classify_problem` never performs it. The vendored C checkout remained
unchanged.

## Original gap and discovered mismatch

The migrated item described the low-level unfolding implementation as present
but left broader formula-backed ownership unresolved. Auditing all C callers
showed that C exposes no formula-set unfolding operation. Formula-origin
definitions first become clauses through `FormulaSetCNF2`; the later owner is
always a `ClauseSet`.

The audit also found a real caller mismatch. C `classify_problem` invokes only
`ClauseSetPreprocess`, whose current body removes superfluous literals and
tautologies, optionally replaces injectivity definitions, and canonizes. It
does not call `ClauseSetUnfoldEqDefNormalize`. Rust's classifier had imported
proof search's extra normalization pass after the same prefix.

Before the fix, both focused inputs produced these divergent classes:

| Owner | Clause count | Symbolic class |
| --- | ---: | --- |
| C | `2` | `FUUS-GFFSF1-SSFFFFFNN` |
| Rust | `1` | `FUUN-GFFSF0-SSFFFFFNN` |

Rust had removed the definition and rewritten `p(f(a))` to `p(a)` before
feature computation.

## Implementation

`preprocess_real_input_clauses` now performs formula CNF, archives the clause
axioms, and conditionally calls only `clause_set_preprocess`, exactly like the
C classifier. The equality-unfolding options remain accepted there because C
accepts and passes them too, even though the current `ClauseSetPreprocess` body
does not consume them.

Proof search is intentionally unchanged: its `ProofStateClausalPreproc` owner
still calls `ClauseSetUnfoldEqDefNormalize` after the optional prefix and
threads active clauses, passive watchlist clauses, archives, gates, statistics,
and proof documentation through that pass.

## Direct executable comparison

[`compare_classifier_eqdef.py`](compare_classifier_eqdef.py) runs the isolated
unchanged C `classify_problem` at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release executable on:

- direct CNF definition/use clauses; and
- equivalent FOF formula owners that first pass through CNF.

After canonicalizing only the platform-specific filename prefix, both complete
feature vectors and symbolic classes are byte-exact. Exit code `0` and empty
stderr also match in both cases. [`reference.json`](reference.json) retains the
results and has SHA-256
`D59575E0FF02703C48C83CDEB83C5883B23C0D5FECC3AC1DA05B94D072DB836D`.

## Owner audit and permanent regressions

[`owner-audit.json`](owner-audit.json) records all ten checks passing: the C
prefix body, classifier caller, and proof-search caller; the corresponding Rust
owners; formula-CNF ordering; the public Rust unfolding surface; two focused
classifier regressions; and the retained formula-origin proof-search unfolding
regression.

The two new classifier tests inspect the state immediately after the caller
pipeline. Both direct CNF and formula-origin cases retain two clauses and the
unrewritten `p(f(a))` target.

## Reproduction

```powershell
cargo build --locked --release --bin classify_problem --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-076-classifier-eqdef-boundary\audit_unfold_owners.py `
  --output experiments\2026-07-17-076-classifier-eqdef-boundary\owner-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-076-classifier-eqdef-boundary\compare_classifier_eqdef.py `
  --c-exe /home/rober/.cache/e-rust-port/sources/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/PROVER/classify_problem `
  --rust-exe target\release\classify_problem.exe `
  --output target\classifier-eqdef-reference.json `
  --expected experiments\2026-07-17-076-classifier-eqdef-boundary\reference.json

cargo test --locked --all-features `
  standard_real_input_preprocessing_keeps_eq_definitions_at_c_caller_boundary
cargo test --locked --all-features `
  standard_formula_input_preprocessing_keeps_cnf_eq_definitions
```

## Compatibility decision

There is no remaining formula-backed unfolding owner to add: C's unfolding API
is clause-set-only, and represented formula definitions reach it after CNF in
the callers that actually normalize. Stable clause handles remain a future
representation improvement, not a missing drop-in behavior.
