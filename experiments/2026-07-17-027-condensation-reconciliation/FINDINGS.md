# `ccl_condensation` reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.110`. This was a migrated completion
statement, not an unimplemented algorithm. The vendored C source remains
unchanged.

## Algorithm and side-effect mapping

The unchanged C and current Rust implementations agree on the compatibility
sequence:

1. Every public fixed-point condensation call increments the process-wide
   attempt counter, even when the polarity-count gate prevents any work.
2. A clause is prepared only when it contains at least two positive or at least
   two negative literals: standard weight is cached and literals are sorted in
   subsumption order.
3. `CondenseOnce` visits literal pairs in list order. If either literal is
   unoriented it performs C's second `swap=true` call, although the private C
   helper never reads that argument; Rust deliberately retains the same no-op
   retry.
4. The first literal is one-way unified with the second. While that substitution
   is live, all literals except the second are copied through the owner term
   bank; the substitution is then backtracked, duplicate and resolved literals
   are removed, and the candidate is weighed and subsumption-sorted.
5. The live source clause receives the candidate's owned literal list only if
   the candidate subsumes it. Cached polarity counts and standard weight are
   restored on the same clause object.
6. The public operation repeats until no pair succeeds. One successful
   fixed-point run increments the success counter, emits the optional
   condensation modification record, then pushes the parentless `DCCondense`
   operation.

Rust makes the term bank, fallible operations, and proof-documentation session
explicit. It uses atomics for the C writable globals and transfers an owned
literal vector instead of nulling a raw candidate pointer. Those are the
already-documented safe ownership/API adaptations, not observable algorithm
changes.

## Production integration

C `ForwardModifyClause` orients the clause before condensation and orients it
again only when condensation changed the clause. Rust's production
`proof_state_forward_modify_clause` path has the same ordering. Its shared
helper selects `condense_with_docs` whenever the caller supplied a
`ProofDocSession`, otherwise it selects plain `condense`.

The production regression requires a live clause id of 4089 to be replaced by
the documentation session's id 1, the exact `condense(4089)` parent expression
to be rendered, the input-formula property to be cleared by documentation, the
clause to shrink to one literal, and the derivation stack to contain only
`DCCondense`.

## External comparison status

This reconciliation is supported by direct source mapping and focused Rust
tests. The checked C executable is a Linux binary, but the active Windows
account has no installed WSL distribution; no unobserved executable comparison
is claimed.

## Validation

- all focused condensation tests pass, including fixed-point, counter,
  candidate-transfer, no-op-swap, and direct documentation coverage;
- the production proof-control condensation/documentation regression passes;
- the immediately preceding full baseline has 4,229 passing library tests plus
  every integration and binary target test, with strict all-feature Clippy;
  this reconciliation changes no Rust code; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass after the documentation update.
