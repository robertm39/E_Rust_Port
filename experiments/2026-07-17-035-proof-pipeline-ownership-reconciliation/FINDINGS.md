# Proof-pipeline ownership reconciliation

## Objective

Reconcile the migrated all-in-one proof-control item `E_Rust_Port-j76.2.102` against the current executable, its dedicated implementation issues, and the completed derivation work. This is an ownership and regression audit; the vendored C source remains unchanged.

## Ownership map

| Migrated concern | Current evidence | Durable owner |
| --- | --- | --- |
| Formula parsing and preprocessing breadth | represented `WFormula` owners, `FormulaSetPreprocConjectures`, the higher-order/FOOL `FormulaSetCNF2` phase pipeline, and executable formula-origin preprocessing paths | executable formula-owner gaps such as `E_Rust_Port-j76.2.89` and their parser/CNF reviews |
| Exact fork-state scheduler parity and resource accounting | owned schedule coordinator, explicit preprocessing/search workers, stdin snapshot replay, nested default retry, and platform resource propagation; the safe transfer decision and startup benchmark are already recorded in the 2026-07-16 multicore-fork experiment | `E_Rust_Port-j76.2.35`, `E_Rust_Port-j76.2.77`, and `cco_scheduling` reviews `E_Rust_Port-j76.4.729` through `.735` |
| Ordered proof-object extraction and renumbering | owner-aware mixed `ProofObjectGraph`, C parent-before-child order, formula/clause interleaving, AC parents, and display-only renumbering | completed item `E_Rust_Port-j76.1.12` and experiment 021 |
| Full higher-order proof search | represented higher-order formula CNF, ordering, immediate clausification, extension indexes/inferences, and several executable THF proof paths, with intentionally narrower remaining ordering/unification/inference boundaries | `E_Rust_Port-j76.2.43` plus control reviews such as `E_Rust_Port-j76.4.717` and `.719` |

The umbrella item no longer identifies an independently actionable missing surface. Keeping it open would duplicate the narrower owners and incorrectly retain ordered proof extraction as pending after that work was completed.

## Validation

- 90 distinct focused tests passed: 13 `FormulaSetCNF2` pipeline tests, 15 proof-object-list ordering/rendering tests, 29 scheduling tests, 2 higher-order executable ordering/paramodulation tests, and 31 executable proof tests spanning FOF/FOOL and THF formulas.
- This documentation/tracking-only reconciliation retains the exact runtime baseline from commit `ae2e0762`: 4,233 default-feature library tests; 4,238 all-feature library tests; all binary targets; 7 integration tests; strict all-target, all-feature pedantic Clippy; and a release `eprover` build.
- `cargo fmt --all -- --check` passed.
- C-source documentation coverage, Change Later wording, Markdown-link integrity, and regeneration-preservation gates passed.
- The vendored `eprover/` worktree remained clean.
