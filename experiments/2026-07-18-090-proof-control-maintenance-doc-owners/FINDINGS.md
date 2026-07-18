# Saturation-maintenance proof-documentation owners

## Status

Completed for Bead `E_Rust_Port-j76.2.47`. The saturation loop now preserves
the active proof-documentation session through both periodic unprocessed-set
cleanup and SAT-check normalization. Plain non-documenting callers retain the
same paths, and the vendored C checkout remained unchanged.

## Owner reconciliation

C's `cleanup_unprocessed_clauses` invokes `ForwardContractSet` after the
configured processed-clause interval. The contraction can rewrite, minimize,
or otherwise modify every unprocessed clause before the cleanup status line is
printed. Because C proof output is global, those modifications use the live
`DocOut` session automatically.

Rust now exposes a documenting cleanup wrapper and dispatches to it from the
documenting saturation loop. The shared cleanup implementation accepts an
optional session, retains its original plain fallback, restores the state-owned
set on errors and early empty-clause returns, and leaves reweighting behavior
unchanged. The proof event is written before the cleanup status text, matching
C.

C's `SATCheck` similarly invokes `ForwardContractSetReweight` when
unprocessed-clause normalization is enabled. Rust now carries the saturation
session through the SAT-check gate and selects a new documenting
contraction-plus-reweight wrapper. The plain SAT-check path remains available,
and an empty clause found in normalization still returns before the SAT solver
or its result counters are touched.

[`audit_maintenance_doc_owners.py`](audit_maintenance_doc_owners.py) pins all
17 C-owner, Rust-dispatch, fallback, and regression-test contracts.

## Executable reference comparison

[`compare_maintenance_docs.py`](compare_maintenance_docs.py) runs the current
optimized Rust prover and the unchanged first-order C executable from commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. Both cases use deterministic FIFO
selection, no preprocessing or generation, output level six, and forced
maintenance triggers.

Both 2/2 cases are exact after normalizing generated identifiers:

- periodic cleanup rewrites and minimizes its remaining clause, producing
  `rw` then `cn` before the special-forward-contraction status line; and
- SAT-check normalization rewrites a negative unit to false and minimizes it
  to the empty clause, again producing `rw` then `cn`.

C and Rust have identical normalized clauses, event order, parent roles,
processed/redundant/rewrite/SAT counters, SZS statuses, exit codes, and empty
stderr. The retained comparison payload is 3,872 bytes with SHA-256
`6C6F400B765C5E01A0C95C4C96B7FE2896330581E10DD2F94B0C5FB8CEF84223`.

## Permanent regressions

Four focused unit tests cover:

- documenting set contraction followed by HCB reweighting;
- direct documenting cleanup returning a minimized empty clause;
- saturation cleanup session continuity and event-before-status order; and
- SAT-check normalization session continuity and preprocessing-refutation
  classification.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-090-proof-control-maintenance-doc-owners\audit_maintenance_doc_owners.py `
  --repo . `
  --output target\maintenance-doc-owner-audit.json `
  --expected experiments\2026-07-18-090-proof-control-maintenance-doc-owners\audit-reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-090-proof-control-maintenance-doc-owners\compare_maintenance_docs.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\maintenance-doc-surface.json `
  --expected experiments\2026-07-18-090-proof-control-maintenance-doc-owners\comparison-reference.json
```

## Validation

- source/test audit: 17/17 contracts passed;
- live unchanged-C/current-Rust comparison: 2/2 cases exact;
- four focused proof-control regressions passed; and
- full suite, strict lint/format gates, documentation gates, optimized build,
  and vendored-C cleanliness are recorded in the completing commit.
