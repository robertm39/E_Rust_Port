# Contextual Simplify-reflect Proof Owner

## Status

Completed for Bead `E_Rust_Port-j76.2.62`. Documented executable saturation now
uses the forward-contraction wrapper that owns contextual simplify-reflect proof
output. The vendored C checkout remained unchanged.

## Original gap

Rust already had a documenting contextual simplify-reflect helper and a
documenting forward-contraction wrapper. The live selected-clause
`ProcessClause` path nevertheless always called the output-free wrapper, even
when saturation supplied a `ProofDocSession`. Contextual simplification changed
the clause and incremented its counter, but the executable omitted C's `csr`
proof event.

## Ownership audit

C has one production mutation owner: `ForwardContractClause` calls
`ClauseContextualSimplifyReflect`. Backward contextual simplify-reflect in
`cco_simplification` only discovers affected processed clauses and moves them
for later processing; it does not perform the mutation at discovery time.

Rust retains the same split. The backward owner emits its represented movement
quote before requeueing, while the selected-clause owner now chooses
`proof_state_forward_contract_clause_with_docs` whenever its documentation
context exists. [`owner-audit.json`](owner-audit.json) records all seven source
and regression checks passing.

## Direct executable comparison

[`compare_context_sr_docs.py`](compare_context_sr_docs.py) runs a two-clause
fixture against the isolated unchanged C executable at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release executable.
It extracts the saturation proof events after AC-axiom scanning and canonicalizes
proof parents as external references or earlier focused events.

Both executables return `10` with empty stderr and report:

- `Processed clauses: 2`;
- `Backward-subsumed: 1`; and
- `Contextual simplify-reflections: 1`.

The focused proof sequence is exact: `new_given`, `csr`, `new_given`,
`subsumed`, `exists`. Clause bodies and every parent role also match. The full
normalized result is retained in [`reference.json`](reference.json), whose
SHA-256 is
`1BEB3BCF9D5DFA26F02740F8D9E2737C3132F80A5B07AAA6F152AEE68CC3129F`.

## Permanent regression

`proof_state_process_clause_with_docs_emits_forward_context_sr_modification`
constructs the production selected-clause path with a live proof session. It
pins the `csr`-before-`new_given` order, contextual-SR statistic, final proof-id
advance, one-literal survivor, and exact `DCContextSR` derivation parent.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-075-context-sr-doc-owner\audit_context_sr_owners.py `
  --output experiments\2026-07-17-075-context-sr-doc-owner\owner-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-075-context-sr-doc-owner\compare_context_sr_docs.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\context-sr-doc-reference.json `
  --expected experiments\2026-07-17-075-context-sr-doc-owner\reference.json

cargo test --locked --all-features `
  proof_state_process_clause_with_docs_emits_forward_context_sr_modification
```

## Compatibility decision

The explicit output/session wrapper remains useful for output-free library
callers, but it is no longer an executable integration gap. A future unified
proof-output owner would be an API cleanup rather than required compatibility
work.
