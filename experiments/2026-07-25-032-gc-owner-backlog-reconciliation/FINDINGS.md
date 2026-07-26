# Experiment 333: GC-owner backlog reconciliation

## Status

Completed for Beads `E_Rust_Port-j76.3.593`,
`E_Rust_Port-j76.4.243`, `E_Rust_Port-j76.4.288`, and
`E_Rust_Port-j76.4.363`.

## Question

Do the migrated GC-owner records still identify implementation work, or were
their requested typed proof-state roots and formula-session contexts completed
by the retained owner work in Experiments 060 and 061?

## Baseline

- The migrated records describe explicit formula marker slices, temporary
  handle-owner scans, and hard-coded untyped proof-state root arrays.
- Experiment 060 replaced those production paths with typed clause/formula root
  variants, direct owner resolution, and a borrow-checked
  `FormulaSetGcContext`.
- Experiment 061 completed the remaining formula-wrapper lifecycle/allocation
  owner work and closed `E_Rust_Port-j76.1.8`.

## Exact commands

```powershell
rg -n "tb_gc_collect" src
rg -n "ProofStateClauseGcRoot::ALL|ProofStateFormulaGcRoot::ALL" `
  src/clauses/proofstate.rs
rg -n "proof_state_collect_term_garbage_marks_every_registered_owner_root|proof_state_formula_cnf_gc_marks_unrelated_registered_owners" `
  src/clauses/proofstate.rs
git -C eprover status --short
```

Current validation evidence is the native-Linux all-target/all-feature run in
Experiment 332:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
```

## Results

- `tb_gc_collect` occurs only in its low-level compatibility module: one
  definition, its test import, and four unit-test calls. There are no
  production formula or proof-state callers.
- Direct proof-state collection and formula CNF both iterate
  `ProofStateClauseGcRoot::ALL` and `ProofStateFormulaGcRoot::ALL`, check live
  term-bank registration, and resolve typed variants to the 12 clause and four
  formula owners.
- The permanent regressions populate every registered owner, preserve an
  unrelated formula-archive term during CNF collection, and recover unrooted
  terms.
- Experiment 060 already validated the owner conversion with focused tests,
  full Rust gates, executable comparisons, and memory/performance checks.
- Experiment 061 completed the remaining formula lifecycle and allocation
  boundary.
- Experiment 332 freshly passed strict formatting/Clippy and all `4,421`
  current Rust tests on native Linux, including both GC-owner regressions.
- The nested C checkout is clean.

## Falsification rule

Reopen implementation work if a production caller constructs `tb_gc_collect`
marker slices, a registered proof-state owner is missing from either typed root
enum, watchlist deregistration is ignored, or CNF collection can sweep a term
held only by an unrelated registered owner.

## Conclusion

The four migrated records are stale duplicates of completed owner work and are
closed without another implementation change. Rust retains C's global-root
semantics while avoiding raw-pointer lifetime hazards: proof-state collection
uses typed root variants, formula transformations receive an owner context,
and standalone tools keep a distinct explicit local root domain. The generic
slice helper remains test-only as the low-level compatibility surface.

## Limits

- Stable numeric handles remain an internal bridge between typed proof-state
  variants and the term-bank registry; production callers cannot register
  arbitrary proof-state owners.
- The split Rust definition clause/formula stores are a safe representation
  choice and do not omit either owner from GC.
- C was not modified.
