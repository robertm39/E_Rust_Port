# Specsig clause-owner boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.55`. The current C
`che_specsigfeatures` surface is fully represented without inventing the
formula-set collector mentioned only as a possible future extension in the
file header. The vendored C checkout remained unchanged.

## Exported surface

[`audit_specsig_boundary.py`](audit_specsig_boundary.py) verifies that the C
header exports initialization/printing plus exactly the represented term,
clause, clause-compute, and clause-set collection levels. Neither the header
nor implementation defines `FormulaCollectSigFeatures` or
`FormulaSetCollectSigFeatures`, and the Rust module has no `FormulaSet` or
`WrappedFormula` dependency.

The live C classifier likewise confirms the owner boundary. It runs
`FormulaSetPreprocConjectures` and `FormulaSetCNF2`, optionally preprocesses
the resulting clauses, and only then calls `ClauseSetCollectSigFeatures` on
`fstate->axioms`. Rust preserves the same sequence and calls
`clause_set_collect_sig_features` on `ProofState::axioms`.

## Executable evidence

[`compare_specsig.py`](compare_specsig.py) compares the complete stdout,
stderr, and exit code for three cases:

- a mixed equality/predicate CNF clause set;
- the equivalent FOF formula-owner input; and
- that FOF input with `--no-preprocessing`.

All three cases are byte-exact between C commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and Rust. They also produce the
same 643-byte output and the same 91-field vector, with stdout SHA-256
`B0B626D7E5510751FAFDE5C58F7D2C5BC7B049666CB42E237734033FEECA3D56`.
This proves formula owners contribute only through their generated clauses;
the preprocessing gate does not introduce a second collection surface.

[`reference.json`](reference.json) retains the 3/3 result and has SHA-256
`CA96EBBDF721602B4A523875E0A1F462820F6F218E4E9CF7671C8D5DFC151DA0`.
A permanent classifier regression independently proves the formula and clause
inputs return identical status, stdout, and stderr and confirms the rendered
vector has exactly 91 fields.

## Compatibility decision

No direct formula-set signature-vector API should be added for compatibility
with this checkout. Doing so would be a new cleaned-design feature, not port
completion. If a future C reference introduces such an export, it can be
ported against that concrete behavior without changing the current
clause-owner path.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-082-specsig-clause-boundary\audit_specsig_boundary.py `
  --repo .

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-082-specsig-clause-boundary\compare_specsig.py `
  --c-exe /home/rober/.cache/e-rust-port/sources/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/PROVER/classify_problem `
  --rust-exe target\release\classify_problem.exe `
  --output target\specsig-reference.json `
  --expected experiments\2026-07-17-082-specsig-clause-boundary\reference.json
```
