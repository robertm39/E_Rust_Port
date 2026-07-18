# ForwardModifyClause proof-documentation surface

## Status

Completed for Bead `E_Rust_Port-j76.2.48`. Every C mutation that emits proof
documentation is represented in Rust, every deliberately silent mutation stays
silent, and all direct production owners now supply the optional session when
one exists. The vendored C checkout remained unchanged.

## Mutation audit

C has five documenting mutation families inside `ForwardModifyClause`:

- `ClauseComputeLINormalform` emits side-specific rewrite steps at output level
  four or higher;
- the explicit superfluous-literal cleanup emits `inf_minimize`;
- `ClauseRemoveACResolved` emits `inf_ac_resolution` with signature AC parents;
- `Condense` emits `inf_condense`; and
- both positive and negative simplify-reflect emit `inf_simplify_reflect` with
  the simplifying unit as partner.

Rust routes those through
`clause_compute_li_normalform_plain_with_docs`, explicit minimization,
`clause_remove_ac_resolved_with_docs_and_axioms`, `condense_with_docs`, and the
bank-backed positive/negative simplify-reflect documenting helpers. The direct
owners are the `forward_contract_keep` path and both generated-clause
`insert_new_clauses` calls; all use the shared optional-session dispatcher.
Set-level and post-saturation owners were completed by the preceding experiment
088.

C does not document `NormalizeEquations`, `ClausePruneArgs`, literal
orientation, or `ClauseLocalRW`; pruning and local rewriting only push their
derivation codes. The strengthened Rust regressions run local rewriting and
higher-order normalize/prune under an output-level-six session, prove the real
mutations and derivations occur, and prove that no proof ID or output is
allocated.

[`audit_forward_modify_docs.py`](audit_forward_modify_docs.py) pins all 18
documenting, silent-mutation, owner, and test contracts.

## Executable reference comparison

[`compare_forward_modify_docs.py`](compare_forward_modify_docs.py) runs current
optimized Rust and unchanged first-order C commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` with deterministic FIFO selection,
no preprocessing or generation, output level six, and focused process limits.
It normalizes only generated identifiers and external-parent identities.

All 4/4 cases are exact:

- external demodulation followed by duplicate cleanup emits `rw` then `cn`;
- condensation emits `condense`;
- unorientable positive-unit simplify-reflect emits `sr`; and
- negative-unit simplify-reflect emits `sr`.

For every case, C and Rust have identical normalized clauses, event order,
parent roles, processed/rewrite/condensation counters, exit status 10, and empty
stderr. The retained payload is 5,364 bytes with SHA-256
`7303D5036BCD463B27100FAAA1E48376386B091826867456526E4F55E25D25A9`. It is
[`comparison-reference.json`](comparison-reference.json).

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-089-forward-modify-doc-surface\audit_forward_modify_docs.py `
  --repo . `
  --output target\forward-modify-doc-audit.json `
  --expected experiments\2026-07-18-089-forward-modify-doc-surface\audit-reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-089-forward-modify-doc-surface\compare_forward_modify_docs.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\forward-modify-doc-surface.json `
  --expected experiments\2026-07-18-089-forward-modify-doc-surface\comparison-reference.json
```

## Validation

- source/test audit: 18/18 contracts passed;
- live unchanged-C/current-Rust comparison: 4/4 cases exact;
- focused local-rewrite and higher-order prune tests passed; and
- full suite, strict lint/format gates, documentation gates, optimized build,
  and vendored-C cleanliness are recorded in the completing commit.
