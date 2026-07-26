# LEARN Change Later reconciliation

## Status

Accepted for the five remaining `learn` records under Beads
`E_Rust_Port-j76.4`. The low-level Rust APIs preserve the defined C behavior
that existing E call paths can observe. User-facing knowledge-base tools
reject duplicate names before the non-atomic insertion quirk is reachable,
and Rust makes the temporary/parser/destination term-bank ownership explicit
without changing the learned-data format. The original C checkout remains
unchanged.

## Decisions

- `j76.4.890`: preserve `AnnoSetFlatten`'s implemented zero return. The value is
  documented as C-compatible rather than as a meaningful remaining-term count,
  and a focused regression test asserts both the mutations and zero result.
- `j76.4.895` and `.904`: preserve `ExampleSetInsert`/`KBAxiomsInsert` ordering
  and its duplicate-name partial insertion at the low-level compatibility
  boundary. Tests pin the inconsistent numeric/name-index side effect.
  `ekb_insert` and `ekb_ginsert` both reject an existing example name before
  copying or generating files, with dedicated tests, so supported persisted-KB
  construction remains consistent.
- `j76.4.897`: retain the double-to-integer source-count cast. Valid generated
  learned data starts from integral occurrence counts and only adds counts, so
  the conversion is exact in its supported domain. Fractional and negative
  values truncate toward zero in both languages while representable. C has no
  portable defined result for NaN, infinity, or an out-of-range integer
  conversion; those malformed values are outside the compatibility contract,
  and Rust's defined saturating/NaN cast result is preferable to emulating one
  target's undefined behavior.
- `j76.4.905`: retain explicit Rust ownership. A temporary term bank owns axiom
  parsing and feature extraction; the callers then provide a parser bank cloned
  from the reserved signature and the persistent annotation destination bank.
  Tests cover the phase separator, feature extraction, term translation,
  duplicate-pattern merging, and no destination mutation when pattern search is
  skipped. No learned-data case requires an implicit shared-signature mutation.

[`audit_learn_reconciliation.py`](audit_learn_reconciliation.py) pins all five
migrated identities and content hashes plus six grouped behavior, safety,
ownership, and validation checks. The audit is independent of Beads status.

## Validation

This review changes only Beads and experiment documentation. The exact
Experiment 041 snapshot already passes 4,427 tests, strict
all-target/all-feature pedantic Clippy, native and Windows GNU x64 builds, and
both maintained compatibility matrices with zero unexpected differences. The
216-case tool matrix includes `ekb_insert` and `ekb_ginsert`; focused unit tests
cover every low-level decision above. Local source and documentation audits
pass; no Rust/C toolchain ran locally.

Reproduce locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-044-learn-reconciliation/audit_learn_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-044-learn-reconciliation/audit-reference.json
```
