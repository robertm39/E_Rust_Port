# AC-resolution proof-ancestry closure

## Status

Completed for Bead `E_Rust_Port-j76.2.8`. The previously implemented AC
cleanup/documentation path remains covered from its mutation boundary through
the final executable proof, and the fresh archived-C/native-Rust matrix now
makes `ALL_RULES.p` byte-normalized exact. The vendored C checkout remains
unchanged.

## Compatibility boundary

- Plain AC cleanup removes AC-trivial negative literals and records `DCACRes`
  plus the current signature AC-axiom count.
- Documenting cleanup resolves the live signature parents, emits C's `ar`
  inference, and records the same count-only derivation entry.
- Proof-state forward modification supplies exact active/archive parent
  generations. Derivation extraction expands `DCACRes` parents without
  collapsing them through the direct-parent dummy-quote policy.
- The permanent `run_all_rules_proof_records_ac_resolution_ancestry` regression
  pins the rewrite chain, all three AC parents, final equality resolution, and
  absence of negative or `922337...` internal identifiers.

The implementation and original falsification work are retained in
[`experiment 006`](../2026-07-09-006-ac-resolution-proof-ancestry/FINDINGS.md).
The later all-owner `ForwardModifyClause` audit and focused four-case exact
comparison are retained in
[`experiment 089`](../2026-07-18-089-forward-modify-doc-surface/FINDINGS.md).

## Fresh executable evidence

The complete report at
`.artifacts/e-compare/20260719-025033-940384/comparison.json` runs both binaries
with auto mode, deterministic rewrite/new-clause sorting, a proof object, and
the same CPU/memory limits. Both exit 0 with `Theorem`, their normalized output
is equal, and `ALL_RULES.p` has no mismatch or expected-difference declaration.

[`audit_all_rules_case.py`](audit_all_rules_case.py) extracts that stable case
projection. [`reference.json`](reference.json) rejects a changed archived
commit, command surface, status, exit, output equality, or declaration.

## Reproduction

```powershell
cargo test --locked --all-features run_all_rules_proof_records_ac_resolution_ancestry
cargo test --locked --all-features proof_state_forward_modify_clause_with_docs_records_ac_resolution
cargo test --locked --all-features clause_push_ac_res_derivation

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-131-ac-proof-ancestry-closure\audit_all_rules_case.py `
  --report .artifacts\e-compare\20260719-025033-940384\comparison.json `
  --output target\all-rules-case-check.json `
  --expected experiments\2026-07-18-131-ac-proof-ancestry-closure\reference.json
```
