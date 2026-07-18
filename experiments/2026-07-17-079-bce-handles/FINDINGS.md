# Blocked-Clause-Elimination Handles

## Status

Completed for Bead `E_Rust_Port-j76.2.58`. BCE task ownership, formula-CNF
routing, and cross-bank equational blockedness are represented. The vendored C
checkout remained unchanged.

## Stable Clause Ownership

C stores raw `Clause_p` values in signed predicate occurrence stacks, each
task's original and candidate clauses, the blocker map, and archive-liveness
checks. Rust used visible `Clause::ident()` values in all of those positions.
Two live clauses with the same visible ID could therefore be mistaken for the
same self-candidate, collapse in an occurrence stack, resume the wrong blocker,
or move the wrong archive owner.

All BCE owners now use `ClauseDerivationRef`. Its nonzero generation is stable
across moves and distinguishes same-visible-ID proof nodes. The permanent
regression assigns visible ID `41` to opposite unit clauses with different
generations. BCE correctly checks them as distinct partners and retains both;
the old ID representation skipped each partner as the clause itself and could
incorrectly eliminate both.

## Formula Ownership

Unchanged C invokes `FormulaSetCNF2` before `ProofStateClausalPreproc`, and BCE
is clause-only inside that later function. There is no broader formula-set BCE
entry point to port. Rust follows the same route for represented FOF owners.
Permanent executable tests now also pin supported first-order-shaped THF owners:
formula CNF drains the owners, then BCE sees and eliminates the two produced
clauses before proof-control initialization.

## Cross-Bank Equational Blockedness

The prior slice repaired `ClauseIsTautologyReal(..., false)` for Rust scratch
banks with distinct canonical `$true` handles. BCE's equational checker is the
other production caller. The retained fixture forces that branch with `a=b`
and checks complementary predicate residuals from `p(a) | q(a)` and
`~p(a) | ~q(a)`.

[`compare_bce.py`](compare_bce.py) is exact between unchanged C at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` and Rust for both BCE lines, the
single retained clause `a=b`, five statistics, exit code `1`, SZS status, and
empty stderr. [`reference.json`](reference.json) has SHA-256
`00CF45889533B82976A050FDF13BD309734A325AA360353F0CC00BA72230FD4E`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-079-bce-handles\compare_bce.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\bce-reference.json `
  --expected experiments\2026-07-17-079-bce-handles\reference.json
```

## Compatibility Decision

`ClauseDerivationRef` completes the safe replacement for C's BCE pointers.
Formula owners require no parallel BCE path because both implementations lower
them before clause preprocessing.
