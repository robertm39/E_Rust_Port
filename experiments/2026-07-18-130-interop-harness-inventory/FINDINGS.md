# Interop harness inventory closure

## Retirement note

On 2026-07-25 the active Windows/WSL harness was retired when build, runtime,
compatibility, benchmark, and Callgrind validation moved to the ephemeral
Linode. The wrapper is retained here only as
[`retired-e-interop.ps1.txt`](retired-e-interop.ps1.txt), an inert input to the
historical inventory audit. Commands below document the completed experiment;
they are not current project instructions.

## Status

Completed for Bead `E_Rust_Port-j76.2.9`. The maintained Windows/WSL harness
has one checked inventory spanning its reference archive, command surfaces,
50-case main matrix, and 216-case support-tool matrix. No new comparison run is
needed: experiments 127 and 129 already retain fresh complete reports against
the same archived C manifest. The vendored C checkout remains unchanged.

## Reference archive

The manifest pins unchanged upstream commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` on Ubuntu 24.04. It contains both
the FOL `eprover` and `--enable-ho` `eprover-ho` builds, with successful
`Theorem` smoke tests and retained SHA-256 values. The FOL archive contains all
25 configured support binaries. `termprops` and `tsm_classify`, whose normal
upstream targets are commented out, are rebuilt from the archived source copy;
the source compatibility patches are idempotent and never touch `eprover/`.

## Comparison surfaces

- The PowerShell wrapper exposes `setup`, `build-reference`, `compare`,
  `compare-tools`, and `benchmark`; its Python driver exposes the corresponding
  commands plus the internal `doctor` action.
- The main matrix contains 36 file cases plus 14 synthetic stdin, syntax-only,
  print-formulas, prune, CNF, app-encode, malformed-input, CPU-limit, and
  memory-limit cases. Six cases use the HO reference and 44 use FOL.
- Every one of the 25 archived support tools has functional comparison cases;
  22 also expose the archived long-version surface. The full matrix contains
  216 cases.

## Retained evidence

[`audit_harness_inventory.py`](audit_harness_inventory.py) combines the fresh
main report at `.artifacts/e-compare/20260719-025033-940384/comparison.json`
with the support-tool report at
`.artifacts/e-compare/20260719-014142-789717-tools/tool-comparison.json`.
[`reference.json`](reference.json) retains the stable command, binary, tool,
mode, scenario, and archived-source-link inventories. The audit rejects a
different archived commit, a missing tool, or coverage drift.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-130-interop-harness-inventory\audit_harness_inventory.py `
  --main-report .artifacts\e-compare\20260719-025033-940384\comparison.json `
  --tool-report .artifacts\e-compare\20260719-014142-789717-tools\tool-comparison.json `
  --output target\interop-harness-inventory-check.json `
  --expected experiments\2026-07-18-130-interop-harness-inventory\reference.json
```
