# Formula-owner executable-mode compatibility matrix

## Status

Completed for Bead `E_Rust_Port-j76.2.89`. The represented first-order formula
owner surface now has exact C/Rust evidence for `--syntax-only`,
`--print-formulas`, `--prune`, and `--cnf`; the separately tracked full parser,
CNF, and relevance work remains open.

## Question

Do the four executable modes preserve C's distinct parse-only, formula-print,
preprocessing-prune, and clausification boundaries across representative
formula-owner inputs, including nested quantifiers and old-TPTP formulas?

## Reference and matrix

The unchanged C reference is commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
[`compare_modes.py`](compare_modes.py) runs both executables on seven canonical
repository fixtures:

- nested quantification (`CNFTest.p`);
- nested existential quantification (`GROUP1st.p`);
- an FOF conjecture (`socrates.p`);
- a question formula (`ans_test06.p`);
- mixed CNF and FOF records (`ALL_RULES.p`); and
- two old-TPTP formula inputs (`SET366+4+rm_eq_rstfp.tptp` and
  `RNG019-6+rm_eq_rstfp.tptp`).

Each fixture is exercised with `--syntax-only --silent`,
`--print-formulas --silent`, `--prune --silent`, and `--cnf --silent`. All 28
cases match exactly in exit status, stdout, and stderr after normalizing only
Windows line endings. [`results-summary.json`](results-summary.json) records
byte counts and SHA-256 digests for every stream without duplicating the
complete transcripts.

The THF smoke fixture is intentionally absent from this FOL-reference matrix:
the cached non-`ENABLE_LFHO` C executable rejects the `thf(...)` wrapper while
Rust deliberately accepts its represented higher-order surface. Remaining
higher-order and fallback-bridge integration is tracked separately rather
than being hidden by this first-order compatibility result.

## Permanent regression

The executable regression
`run_formula_owner_modes_match_nested_quantifier_smoke_fixture` uses the
vendored `CNFTest.p` read-only and pins the exact stable output of all four
modes. It covers syntax-only success framing, formula-set pretty output,
prune-phase output, and the generated Skolem clause from CNF conversion.

`--error-on-empty` is part of the same user-visible input-mode group, but its
selected-owner semantics and rejection framing already have a stronger
12-case exact comparison in
[`../2026-07-17-045-error-on-empty-owner-count/FINDINGS.md`](../2026-07-17-045-error-on-empty-owner-count/FINDINGS.md).

## Compatibility boundary

This closes the mode-level evidence gap, not the entire formula pipeline.
Bead `E_Rust_Port-j76.2.42` retains full CNF transformation, parser-symbol
policy, and remaining formula-helper work; `E_Rust_Port-j76.2.38` retains full
formula-owner relevance integration; and `E_Rust_Port-j76.2.54` retains the
remaining raw-spec feature and parser-bridge replacement. No production-code
change was warranted because the expanded live matrix found no mismatch.

## Reproduction

```powershell
& 'C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' `
  experiments\2026-07-17-048-formula-owner-mode-matrix\compare_modes.py `
  --rust-exe target\release\eprover.exe `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --distro Ubuntu-24.04 `
  --output experiments\2026-07-17-048-formula-owner-mode-matrix\results-summary.json `
  --quiet
```

## Validation

- live 28-case C/Rust executable comparison: exact
- focused permanent executable regression: passed
- full serial suite: 4,256 library tests plus all binary/integration targets
- strict all-target/all-feature pedantic Clippy: passed
- formatting and all four C-source documentation integrity gates: passed
