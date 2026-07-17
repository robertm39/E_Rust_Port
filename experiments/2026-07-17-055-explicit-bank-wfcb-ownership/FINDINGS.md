# Explicit heuristic-bank ownership reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.83`. The explicit `TermBank` borrow in
Rust priority and WFCB interfaces is a completed ownership decision, not a
missing C back-pointer port. No Rust implementation change was needed, and the
vendored C checkout remained unchanged.

## C and Rust ownership boundary

C equations retain an `eq->bank` pointer, so priority functions and WFCB
callbacks can recover the term bank from a clause literal. Some C WFCB data
also retains an `OCB` pointer, and ordering-aware callbacks may mutate clause
orientation and maximality while scoring.

Rust clauses and equations contain shared term handles but deliberately do not
retain a raw or self-referential pointer to their owning `TermBank`. Instead,
`ClausePrioFun` receives `&TermBank`, while ordering-aware WFCBs receive the
active `&mut OrderControlBlock`, `&mut TermBank`, and `&mut Clause`. This makes
the owner relationship explicit at the call site, prevents a stored bank
pointer from becoming stale when owners move, and adds only borrowed-reference
passing rather than a structural traversal or allocation.

## Production call-graph audit

All proof-control evaluation sites use the mutable banked HCB path:

- initial `Uniq` reweighting and active-HCB axiom evaluation;
- processed-clause reset and requeue evaluation;
- forward-contraction and unprocessed-set reweighting; and
- `eval_store` evaluation before clauses enter the unprocessed set.

`hcb_clause_evaluate_with_bank` dispatches every WFCB through
`compute_eval_with_bank`. WFCBs whose C callbacks conditionally mark maximal
terms are registered with `wfcb_alloc_with_bank`, so their mutable callback is
used. The fallback to the immutable callback applies only to WFCBs registered
without an ordering-dependent callback.

The immutable `compute_eval`, `hcb_clause_evaluate`, and set-reweight helpers
remain useful as low-level adapters and unit-test surfaces for stateless
scorers, but no prover production evaluation call site uses them. Collapsing
that public API split remains optional post-compatibility cleanup under
`E_Rust_Port-j76.4.844`; adding a bank back-pointer is neither required for
behavior nor desirable for Rust ownership.

## Focused evidence

- `cargo test --locked --all-features --lib with_bank --quiet`: 12 passed;
- `cargo test --locked --all-features --lib proof_state_eval_clause_set
  --quiet`: 2 passed;
- `cargo test --locked --all-features --lib prio_funs --quiet`: 10 passed;
- the banked regressions cover WFCB dispatch, HCB dispatch, set reweighting,
  mutable owner-bank ordering preparation, and maximal-term side effects; and
- the immediately preceding unchanged-production baseline passed 4,260
  all-feature library tests plus every target, strict pedantic Clippy, release
  build, formatting, and all C-source documentation gates.

## Residual scope

`E_Rust_Port-j76.3.606` separately tracks C's same-bank assertion in
`EqnUnifyDirected`. Direct proof-state/OCB cleanup for generic funweights and
varweights remains under `E_Rust_Port-j76.2.73` and `.2.74`. Those narrower
items do not require or justify storing a C-style bank pointer in each Rust
equation.
