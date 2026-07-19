# Explicit formula-owner include policy

## Status

Completed for Bead `E_Rust_Port-j76.2.12`. Every supported production
formula-owner path uses explicit include parsing. The automatic scanner
splicing constructor remains represented and regression-tested, but no
production formula owner calls it. The vendored C checkout remains unchanged.

## Ownership audit

[`audit_include_owners.py`](audit_include_owners.py) records the exact ten
`FormulaAndClauseSetParse` call sites in unchanged C. Their supported Rust
counterparts converge on two explicit owners:

- the general TPTP/TSTP parser in `src/prover/eprover.rs`, shared directly or
  through its crate-visible wrapper by `eprover`, `classify_problem`,
  `eground`, `epatternize`, and `enormalizer`; and
- the TSTP batch parser in `src/control/batch_spec.rs`, shared by batch/SInE
  loading and interactive `ADD`, `LOAD`, and `RUN` input.

Both owners now call the same scanner-level selector helpers. App-encode stays
separate because unchanged C deliberately calls `ignore_include` in that mode.
The only two `from_file_following_includes` references in `src/` are its
definition and its scanner-module regression.

## Corrected nested-selector behavior

C recursively parses an included set, filters that completed set, and only
then merges it into the parent. The general Rust parser already represented
that order with a selector stack. The batch parser previously applied a
parent selector to entries written directly in the child but merged selected
grandchild entries without applying the parent frame.

The shared `include_entry_selected_by_stack` helper tests frames from inner to
outer, and the batch parser now pushes and pops one frame around every opened
include. A grandchild rejected by its inner frame cannot mark an outer frame;
an inner-selected grandchild that the outer frame rejects no longer leaks into
the batch formula set. A second regression proves that caller-registered
includes are skipped without missing-selector errors while an unseeded include
declared twice is still parsed twice, matching this C snapshot.

## Exact unchanged-C comparison

[`capture_nested_include.py`](capture_nested_include.py) runs the pinned C and
rebuilt Rust `eprover --print-formulas --tstp-in` over the retained nested and
repeated include fixture. Both exit successfully, write nothing to stderr, and
emit exactly:

```text
fof(outer_selected, axiom, r(a)).
fof(repeated, axiom, s(a)).
fof(repeated, axiom, s(a)).
fof(main, axiom, t(a)).
```

The missing inner-selected formula proves outer filtering is applied after the
inner selection. The duplicate `repeated` formula proves the visible unseeded
skip tree remains unpopulated.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-126-explicit-include-policy\audit_include_owners.py `
  --output target\explicit-include-owner-audit-check.json `
  --expected experiments\2026-07-18-126-explicit-include-policy\owner-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-126-explicit-include-policy\capture_nested_include.py `
  --output target\nested-include-check.json `
  --expected experiments\2026-07-18-126-explicit-include-policy\reference.json
```
