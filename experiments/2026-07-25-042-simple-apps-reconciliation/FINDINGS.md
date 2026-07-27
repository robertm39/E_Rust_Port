# SIMPLE_APPS reconciliation

## Status

Accepted for the three remaining `simple_apps` records under Beads
`E_Rust_Port-j76.4`. All three describe deliberate `term2dag` compatibility
boundaries already implemented and exercised by the permanent support-tool
matrix. No implementation change is required, and the vendored C checkout
remains unchanged.

## Decisions

- `j76.4.1213`: retain the C help banner's four spacer runs. The Rust help
  builder names each run explicitly, and the automatic help comparison makes
  the otherwise cosmetic whitespace part of the drop-in surface.
- `j76.4.1215`: retain direct sorting of collected shared terms by `entry_no`.
  It preserves `TBPrintBankInOrder` output without C's temporary `NumTree`;
  there is no measured reason to restore the extra allocation.
- `j76.4.1216`: retain C's mixed `SigPrint` stream behavior only for
  `term2dag` file output. The general signature writer remains single-stream;
  the executable explicitly selects the compatibility side-channel writer.

[`audit_simple_apps_reconciliation.py`](audit_simple_apps_reconciliation.py)
pins all three migrated identities and content hashes plus five grouped source
and matrix checks. The audit is independent of Beads status.

## Validation

This review changes only Beads and experiment documentation. The exact
Experiment 041 Linux snapshot already passes 4,427 Rust tests, strict
all-target/all-feature pedantic Clippy, native and Windows GNU x64 builds, and
the 216-case support-tool matrix with zero unexpected differences. That matrix
includes `term2dag` help, basic stdin, shared/typed DAG output, and missing-file
behavior. Local documentation validators and this source audit pass; no
Rust/C toolchain ran locally.

Reproduce locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-042-simple-apps-reconciliation/audit_simple_apps_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-042-simple-apps-reconciliation/audit-reference.json
```
