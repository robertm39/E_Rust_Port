# Formula variable-name map closure

## Status

Completed for Bead `E_Rust_Port-j76.2.7`. The asymmetric C parser policy is
explicit and covered by every migrated production owner: clauses clear the
external-name map under the default local-variable mode, while following
formula records reuse that map. The vendored C checkout remains unchanged.

## Compatibility boundary

`ClauseParseOptions::default()` enables local clause variables and disables
disjoint variables. Clause parsing therefore calls `clear_ext_names`, resetting
the shared name bindings and per-sort allocation state. Formula parsing does
not perform that local-clause reset. A clause that encounters names as
`X3,X4,X1,X2` consequently leaves the following formula's source-order
`X1,X2,X3,X4,X5` binders encoded and printed as `X3,X4,X1,X2,X5`, matching C.

Permanent regressions pin that exact permutation in:

- main `--app-encode` and `--print-formulas`;
- the main `ALL_RULES.p` proof path;
- concrete batch problem loading; and
- `enormalizer` formula targets.

The original diagnosis and implementation evidence is retained in
[`experiment 005`](../2026-07-09-005-formula-variable-name-state/FINDINGS.md).

## Fresh live evidence

The complete main report at
`.artifacts/e-compare/20260719-025033-940384/comparison.json` records
`ALL_RULES.p` as exact: both binaries exit 0 with `Theorem`, normalized proof
output is equal, and there is no mismatch declaration. The support-tool report
at `.artifacts/e-compare/20260719-014142-789717-tools/tool-comparison.json`
records the enormalizer TSTP formula-target case as exact on stdout, stderr, and
exit behavior. Both reports use the same archived C manifest.

[`audit_formula_variable_map.py`](audit_formula_variable_map.py) pins eight
source/test contracts plus those two live case projections.
[`reference.json`](reference.json) rejects policy, owner, binder, manifest, or
result drift.

## Reproduction

```powershell
cargo test --locked --all-features run_app_encode_reuses_last_clause_variable_name_map_like_c
cargo test --locked --all-features run_print_formulas_reuses_last_clause_variable_name_map_like_c
cargo test --locked --all-features load_problem_from_file_reuses_formula_variable_name_map_like_c
cargo test --locked --all-features tstp_formula_targets_reuse_external_name_map_like_c

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-133-formula-variable-map-closure\audit_formula_variable_map.py `
  --repo . `
  --main-report .artifacts\e-compare\20260719-025033-940384\comparison.json `
  --tool-report .artifacts\e-compare\20260719-014142-789717-tools\tool-comparison.json `
  --output target\formula-variable-map-check.json `
  --expected experiments\2026-07-18-133-formula-variable-map-closure\reference.json
```
