# Goal-definition Ownership And Order

## Status

Completed for Bead `E_Rust_Port-j76.2.60`. The production proof-search owner,
formula-origin route, sign selection, recursive subterm behavior, and
compatibility-visible definition order are represented. The vendored C checkout
remained unchanged.

## Questions

The migrated item left three concerns open:

1. whether formula owners needed a separate goal-definition path;
2. whether Rust preserved C's pointer-ordered goal-term traversal; and
3. whether stable proof-state handles were required for the production owner.

## Ownership audit

C exposes only `ClauseSetGDTransform`. `ProofStateClausalPreproc` calls it after
blocked-clause elimination and predicate elimination, and formula conjectures
have already become clauses through `FormulaSetCNF2`. There is no formula-set
goal-definition entry point to add. The prune branch exits before this later
proof-search transformation.

Rust preserves the same path: formula CNF, clause preprocessing, BCE, predicate
elimination, goal-definition transformation, then initial clause documentation
and proof-control initialization. [`owner-audit.json`](owner-audit.json)
records all fourteen source-owner and permanent-regression checks passing.

The transform collects shared terms before inserting any definition clauses.
The cloned shared-term handles remain valid across that second phase, so stable
clause arenas are not needed for current behavior.

## Pointer-order decision

C stores candidate terms in a `PTree` keyed by raw `Term_p` and traverses it in
pointer order. Rust's collector uses `BTreeMap<usize, Term>`, but the key is not
structural or an entry number: `term_identity_id` is the live `Rc` allocation
address. Rust therefore preserves C's allocator-dependent pointer-order policy.

`gd_transform_assigns_definitions_in_live_term_identity_order` computes the
runtime address order of two independent terms, runs the transformation, and
asserts that the generated clause left sides follow that exact order. This
guards against accidentally replacing pointer order with encounter, structural,
or term-entry order.

## Direct executable comparison

[`compare_goal_defs.py`](compare_goal_defs.py) runs isolated unchanged C at
commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release executable
over four cases:

| Case | Behavior pinned | Focused result |
| --- | --- | --- |
| all signs | four independent positive/negative goal terms | exact five-clause trace |
| negative only | excludes the positive equality's terms | exact three-clause trace |
| recursive subterms | child-first definitions and definition normal forms | exact five-event trace |
| formula origin | FOF conjecture through formula CNF | exact three-clause trace |

For every case, generated `edef` names, clause bodies and roles, rewritten goal,
initial-saturation and processed counts, total rewrite steps, exit code `10`,
and empty stderr match. The retained [`reference.json`](reference.json) has
SHA-256
`EE9345869F907B24D35651FDCC2A90AE58C49A32A25158CC2F9D3510391FD97C`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-077-goal-definition-order\audit_goal_def_owners.py `
  --output experiments\2026-07-17-077-goal-definition-order\owner-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-17-077-goal-definition-order\compare_goal_defs.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\goal-definition-reference.json `
  --expected experiments\2026-07-17-077-goal-definition-order\reference.json

cargo test --locked --all-features `
  gd_transform_assigns_definitions_in_live_term_identity_order
```

## Compatibility decision

The remaining old limitations were ownership-description gaps, not missing
production paths. Rust now has explicit regression evidence for C's live
pointer-order policy and exact normal-reference traces. A future stable clause
arena would be an API/performance cleanup rather than required drop-in behavior.
