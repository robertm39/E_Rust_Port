# Pre-CNF input-owner closure

## Status

Completed for Bead `E_Rust_Port-j76.2.6`. Ordinary non-watchlist CNF/LOP input
remains a clause-backed `WrappedFormula` until raw automatic classification and
the represented `FormulaSetCNF2` drain. The fresh main matrix now makes all
three proof cases named by the migrated item byte-normalized exact. The
vendored C checkout remains unchanged.

## Owner boundary

- Main parsing collects clause and formula records into a temporary
  `FormulaSet` and moves it into proof-state `f_axioms`; watchlist clauses stay
  in their separate clause set.
- Automatic preprocessing selection consumes the raw formula-owner features
  before formula clausification.
- `form_clause_alloc` marks the wrapper clause-backed and moves clause source
  metadata into it. Direct wrapped-clause CNF reconstructs one clause and
  pushes `DCFofQuote` to the wrapper source.
- `FormulaSetArchive` moves originals and leaves working copies with their own
  formula-level `DCFofQuote`, preserving the original/copy edge across later
  CNF phases.
- Permanent regressions pin the automatic `FSMSSMSSSSSNFFN` class, formula-set
  ownership, answer-proof copy ancestry, and the full ALL_RULES proof ancestry.

The original diagnosis and implementation are retained in
[`experiment 003`](../2026-07-09-003-pre-cnf-input-ownership/FINDINGS.md). The
complete represented formula-pipeline and higher-order scope audit is retained
in [`experiment 095`](../2026-07-18-095-formula-pipeline-scope/FINDINGS.md).
Its static JSON is historical: later helper splits renamed the executable
allocation, event-aware proof-control initialization, and definition-parser
routes. This closure therefore audits the current narrow owner contracts
directly instead of rewriting the completed experiment 095 evidence.

## Fresh executable evidence

The complete report at
`.artifacts/e-compare/20260719-025033-940384/comparison.json` records
`ALL_RULES.p`, `ans_test06.p`, and `socrates.p` as exact. Archived C and native
Rust exit 0 with `Theorem` for all three; normalized output is equal, and none
has a mismatch or expected-difference declaration.

[`audit_pre_cnf_owners.py`](audit_pre_cnf_owners.py) pins seven source/test
contracts and those three stable case projections. [`reference.json`](reference.json)
rejects source-owner or executable-result drift.

## Reproduction

```powershell
cargo test --locked --test eprover_schedule auto_mode_classifies_cnf_inputs_as_pre_cnf_formula_owners
cargo test --locked --all-features formula_set_cnf2_drains_inputs_and_archives_originals_then_cnf_copies
cargo test --locked --all-features run_answer_proof_object_preserves_formula_copy_ancestry

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-132-pre-cnf-owner-closure\audit_pre_cnf_owners.py `
  --repo . `
  --report .artifacts\e-compare\20260719-025033-940384\comparison.json `
  --output target\pre-cnf-owner-check.json `
  --expected experiments\2026-07-18-132-pre-cnf-owner-closure\reference.json
```
