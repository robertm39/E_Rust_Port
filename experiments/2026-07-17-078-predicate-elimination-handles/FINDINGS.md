# Predicate-Elimination Handles And Gate Validation

## Status

Completed for Bead `E_Rust_Port-j76.2.59`. Predicate-elimination task ownership,
cross-bank gate tautology validation, and executable runtime-PicoSAT selection
are represented. The vendored C checkout remained unchanged.

## Findings

C's five cheap clause sets store raw `Clause_p` values. Rust stored visible
`Clause::ident()` values, so two live clauses with the same display identifier
could collapse in a task, resolve through the wrong owner, or move only one
owner to the archive. Predicate tasks and the offending-clause worklist now use
`ClauseDerivationRef`. Its generation-qualified identity remains stable across
set moves and preserves exact `DCPEResolve` parents. The permanent regression
uses two clauses with visible ID `17` and different generations, then verifies
that both exact owners are archived and both exact parents appear in the
resolvent derivation.

The executable fixture exposed a separate behavior mismatch. Unchanged C
recognized the bidirectional `p` gate and reported `% PE eliminated: 2`, while
Rust's internal and runtime-loaded SAT backends both reported zero. Both Rust
backends returned the correct unsatisfiable core and constructed the exact
candidate tautology `q(X) | ~q(X)`. The failure occurred afterward:
`ClauseIsTautologyReal(..., false)` normalized main-bank terms in a distinct
Rust scratch bank. Unlike C, Rust term banks own different canonical `$true`
handles, so the final pointer-identity check could not see the normalized
predicate atom and its source-bank truth term as equal.

Rust now creates a bank-local work clause for both values of C's public
copy/no-copy flag. This preserves the non-mutating Rust API and C's final
pointer-equality semantics. It also repairs the same latent boundary in the BCE
no-copy caller.

## Three-Way Executable Comparison

[`compare_predicate_elimination.py`](compare_predicate_elimination.py) compiles
[`mock_picosat.rs`](mock_picosat.rs) as a small ABI-compatible runtime library,
then runs:

1. unchanged isolated C at commit
   `17026b1bfe61aaf223cfaae54947c8d2679c31a0`;
2. Rust with the internal `SatClauseSet` solver; and
3. Rust with `E_RUST_PORT_PICOSAT_LIBRARY` selecting the runtime library.

All three executions are exact for the two PE progress lines, retained clauses
`q(a) | ~q(b)` and `q(c) | q(d)`, five preprocessing/search statistics, exit
code `1`, SZS status, and empty stderr. The retained
[`reference.json`](reference.json) has SHA-256
`05B586ADD2DE7EB5C768805F85DF057D32157A7DF61F9E9947BD3193E51960FA`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-078-predicate-elimination-handles\compare_predicate_elimination.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --mock-library target\predicate-elimination-mock.dll `
  --output target\predicate-elimination-reference.json `
  --expected experiments\2026-07-17-078-predicate-elimination-handles\reference.json
```

## Compatibility Decision

Runtime PicoSAT selection changes only the SAT solver owner; it does not change
validated gate eliminations or proof-state ownership. `ClauseDerivationRef` is
the completed safe replacement for C's long-lived task pointers in this owner.
