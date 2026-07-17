# ClauseSet-owned FV-index subsumption integration

## Status

Completed for Bead `E_Rust_Port-j76.2.133`. Production subsumption callers now
select indexed or plain lookup through the FV anchor owned by their `ClauseSet`.
The vendored C source remained unchanged.

## C behavior

C stores the feature-vector anchor in `ClauseSetCell`. Its public subsumption
entry points inspect that owned field, while indexed insertion and extraction
keep the corresponding index contents synchronized with set membership.

Simplify-reflect also emits proof documentation and records the simplifying
clause as a derivation parent. The raw `Clause_p` parent is only safe under C's
coupled clause/archive lifetime; it is not an independently reproducible output
format requirement.

## Rust equivalence

Rust already stored an optional `FvIndexAnchor` in `ClauseSet` and maintained it
through `indexed_insert_clause_owned` and extraction. The remaining production
callers redundantly fetched that anchor and passed it into lower-level APIs.
Set-owned wrappers now make the C ownership boundary explicit and are used by
proof-control forward subsumption, contextual simplify-reflect, watchlists, and
split-definition variant lookup. Explicit-anchor functions remain available for
tests and interop that intentionally own an index separately.

The lifecycle regression inserts a unit clause through the owned index, finds
it through both ordinary and mutable-bank owned wrappers, extracts it by ID,
and confirms that the same owned lookup no longer returns it and the index count
is zero. A unit query distinguishes the indexed route from the plain fallback,
whose C-shaped precondition requires a non-unit candidate.

Rust preserves represented simplify-reflect side effects through explicit
`ProofDocSession` output and compact `DCSR` clause references. Stable proof
object parent handles remain separate reconstruction work; copying C's raw
parent pointer would weaken lifetime safety without improving current output.

## Performance

The owned functions are thin delegating wrappers over the existing indexed and
plain algorithms. They add no traversal or allocation and remove redundant
anchor plumbing from callers, so no algorithmic or hot-loop performance change
is expected.

## Validation

- focused subsumption tests cover owned-index insertion, ordinary and
  mutable-bank lookup, extraction, and empty-index state;
- production-source searches leave explicit-anchor calls only in lower-level
  definitions and their compatibility tests;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
