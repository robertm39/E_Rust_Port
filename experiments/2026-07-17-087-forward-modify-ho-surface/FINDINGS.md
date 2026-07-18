# Higher-order ForwardModifyClause surface

## Status

Completed for Bead `E_Rust_Port-j76.2.50`. The migrated gap was stale: the
production path already preserves C's higher-order mutation hooks and admits
the complete optimized-C ordering surface. This slice strengthens the shallow
prune-args integration test and consolidates current executable evidence. The
vendored C checkout remained unchanged.

## Hook and owner audit

C calls `NormalizeEquations` four times in each `ForwardModifyClause` loop:
before demodulation, after demodulation, after a successful local rewrite, and
before simplify-reflect. It checks triviality before optional `ClausePruneArgs`,
then normalizes the pruned clause before positive and negative
simplify-reflect. Rust keeps the same order and uses the live mutable proof-
state term bank for normalization, pruning, rewriting, and both orientation
passes.

[`audit_forward_modify.py`](audit_forward_modify.py) pins those C/Rust hook
positions, the six-ordering admission gate, owner-bank orientation, LFHO and
Lambda-order regressions, encoded-equality normalization, and a real prune-args
mutation. All 9/9 contracts pass in
[`audit-reference.json`](audit-reference.json).

The prior production prune test only enabled the option on a clause with no
candidate. It now sends two occurrences of the same applied higher-order
variable through `ForwardModifyClause`, with one constant argument and one
varying argument. The production hook removes the constant position, rebuilds
both applications with the remaining argument, and records `DCPruneArg`.

## Executable ordering evidence

[`compare_orderings.py`](compare_orderings.py) reuses the eta/DB, flex-flex,
and applied-variable rigid-prefix fixtures from the original direct-LFHO
reconciliation. It runs current optimized Rust and unchanged higher-order C
commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` under KBO, KBO6, LPO,
LPOCopy, LPO4, and LPO4Copy.

All 18/18 configurations have the expected processed-clause-limit status 9,
empty equal stderr, exact normalized inference traces, exact processed/
generated/paramodulation counters, and a nonzero paramodulation count. The
combined comparison payload is 34,941 bytes with SHA-256
`3E6ED2315AF642D4C4D8FFA5D5CDB65963D053FF6A734D86331BC256DF1988D6`;
the retained result is [`ordering-reference.json`](ordering-reference.json).

Classic KBO and legacy LPO/copy preserve optimized C behavior after source
assertions are compiled out. KBO6 and LPO4 retain their bank-backed LFHO
normalization paths. Immutable/no-bank helper behavior is already retained as
post-compatibility API and performance review work; every supported executable
`ForwardModifyClause` owner has the live bank, so it is not an open drop-in
compatibility gap here.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-087-forward-modify-ho-surface\audit_forward_modify.py `
  --repo . `
  --expected experiments\2026-07-17-087-forward-modify-ho-surface\audit-reference.json

cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-087-forward-modify-ho-surface\compare_orderings.py `
  --c-eprover /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b/PROVER/eprover-ho `
  --rust-eprover target\release\eprover.exe `
  --output target\forward-modify-orderings.json `
  --expected experiments\2026-07-17-087-forward-modify-ho-surface\ordering-reference.json
```

## Validation

- focused higher-order `ForwardModifyClause` tests passed;
- source/test audit: 9/9 contracts passed;
- live C/Rust LFHO comparison: 18/18 configurations exact;
- all-target/all-feature suite: 4,299 library tests plus every auxiliary target
  passed;
- strict all-target/all-feature pedantic Clippy and formatting passed;
- all four C-source documentation integrity gates passed;
- optimized all-feature `eprover` build and experiment script compilation
  passed; and
- vendored C worktree remained clean.
