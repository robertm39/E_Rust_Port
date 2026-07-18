# Raw-spec owner and bridge compensation boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.54`. The Rust raw-feature computation
now has permanent coverage for both represented formula owners and the
fallback clause-lowering compensation path. The vendored C checkout remained
unchanged.

## Ownership boundary

C `RawSpecFeaturesCompute` counts the current clause and formula owners in a
`ProofState` exactly once. It reads clause and formula cardinality, standard
weight, conjecture and hypothesis roles, formula order, and active/archive
formula definition statistics. Rust reads the same represented owners from
`ProofState::axioms`, `ProofState::f_axioms`, and
`ProofState::f_ax_archive`.

Most current executable FOF, TFF, TCF, and THF inputs are retained as formula
owners. When the fallback parser cannot retain an exact formula owner, it may
materialize one or more generated clause wrappers instead. The accumulated
`RawFormulaFeatures` metadata records both the original formula vector and the
generated clause footprint. `raw_spec_features_compute` subtracts that
footprint before restoring the original sentence, weight, role, order,
lambda, and applied-variable values.

The permanent
`compute_replaces_bridge_lowered_clauses_with_original_formula_features`
regression constructs a two-clause lowered footprint and proves it is replaced
by one original conjecture formula with its original weight and higher-order
fields. Existing formula-set coverage independently exercises represented
formula cardinality, weight, roles, order, definitions, lambdas, and applied
variables.

## Executable evidence

[`compare_rawspec.py`](compare_rawspec.py) compares complete stdout, stderr,
and exit status for represented FOF and THF formula-owner inputs. Both cases
are byte-exact between unchanged C commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and Rust. Each emits a 109-byte
raw-feature line; the FOF stdout SHA-256 is
`E86985A4FAD0C1EFFE94BEEC582EFCEFF0A7574927BFC97F7BD3C421501EEEDC`
and the THF stdout SHA-256 is
`4D841241CB3CCB10DAD751605F3A41B1659C706CA14FDE1F331B1BC0CB53E185`.

[`reference.json`](reference.json) retains the 2/2 exact result and has
SHA-256 `75A910F2CB63761C3C3FA60353CB7160A2D2A4674F803A56F00AE7A0D01AB9A4`.

## Compatibility decision

Replacing the remaining fallback parser and its preprocessing bridge is not a
missing operation in `che_rawspecfeatures`. It is broader parser, formula-owner,
and CNF work already retained in Beads `E_Rust_Port-j76.2.41` and
`E_Rust_Port-j76.2.42`. The compensation remains necessary and exact while
that bridge exists; this slice closes only the raw-feature ownership gap and
does not claim the broader parser is complete.

## Reproduction

```powershell
cargo test --locked --all-features `
  compute_replaces_bridge_lowered_clauses_with_original_formula_features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-083-rawspec-bridge-compensation\compare_rawspec.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/classify_problem `
  --rust-exe target\release\classify_problem.exe `
  --output target\rawspec-reference.json `
  --expected experiments\2026-07-17-083-rawspec-bridge-compensation\reference.json
```
