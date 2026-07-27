# Vendored PicoSAT CONTRIB reconciliation

## Status

Accepted for the five remaining `contrib` records under Beads
`E_Rust_Port-j76.4`. These records distinguish PicoSAT's vendored standalone
utilities and broad upstream API from the narrow SAT-solver boundary E itself
uses. Rust's explicit runtime-loaded wrapper plus tested internal fallback is
the supported deployment decision; porting the unrelated PicoSAT command-line
utilities or the complete upstream API is not required for E drop-in
compatibility. The vendored C checkout remains unchanged.

## Decisions

- `j76.4.502`, `.503`, and `.504`: do not import `app.c` shell decompression,
  the forwarding PicoSAT `main`, or the separate grouped-CNF parser into E's
  parser and executable surface. They are upstream PicoSAT utilities, not
  E-owned entry points or E's SAT integration path.
- `j76.4.510`: retain the documented deployment policy. `eprover` first honors
  `E_RUST_PORT_PICOSAT_LIBRARY`, then checks executable-adjacent bundle
  locations, and otherwise uses the internal solver. This gives deployments a
  stable opt-in without embedding the large vendored C implementation into
  the Rust crate.
- `j76.4.511`: keep the safe wrapper limited to the eight reentrant symbols E
  uses. Solver/library ownership, initialization, trace enablement, clause
  export, solve results, core extraction, reset, missing-library behavior, and
  internal-backend fallback all have lifecycle tests. Broader upstream
  allocator, assumptions, proof trace, and utility APIs are outside the
  compatibility surface.

[`audit_contrib_reconciliation.py`](audit_contrib_reconciliation.py) pins all
five migrated identities and content hashes plus six grouped source,
deployment, lifecycle, and validation checks. The audit is independent of
Beads status.

## Validation

This review changes only Beads and experiment documentation. The exact
Experiment 041 snapshot already passes 4,427 tests, strict
all-target/all-feature pedantic Clippy, native and Windows GNU x64 builds, and
both maintained compatibility matrices with zero unexpected differences. Its
default executable path exercises the internal solver fallback. Local source
and documentation audits pass; no Rust/C toolchain ran locally.

Reproduce locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-043-contrib-picosat-reconciliation/audit_contrib_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-043-contrib-picosat-reconciliation/audit-reference.json
```
