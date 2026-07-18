# Orphan-liveness stable identity audit

## Status

Completed for Bead `E_Rust_Port-j76.2.75`. Stable generation-qualified clause
references already replace C's raw derivation pointers in both liveness paths.
The vendored C checkout remained unchanged.

## Question

Has stable parent identity replaced C's raw derivation pointers in both HCB
selection and periodic cleanup, including when visible clause identifiers are
reused?

## Method

[`audit_liveness.py`](audit_liveness.py) checks the C raw-pointer contract, the
Rust stable-reference equality/hash/conversion rules, both proof-control
liveness paths, their owner scopes, and permanent same-ID/different-generation
regressions.

## Findings

The audit passes all fourteen contracts:

- `ClauseDerivationRef` stores an immutable process-local generation and uses
  it for equality, ordering, and hashing whenever it is nonzero. Visible clause
  identifier and CSSCPA source remain rendering metadata; generation-zero
  references preserve legacy behavior.
- Selection compares the exact stable reference after the processed-set ID
  index finds a candidate. It falls back across duplicate visible identifiers
  before checking stable source/archive owners, so a live clause with the same
  printed ID cannot revive a dead generation.
- Generated children waiting in tmp/eval/unprocessed are deliberately excluded
  from selection-time parent owners. Periodic cleanup, which mutates the whole
  unprocessed set, uses a compact hash snapshot collected from every live
  proof-state owner.
- Low-level HCB selection retains an injected orphan predicate, preserving the
  reusable clause-set boundary without storing proof-state pointers in the HCB.

The strengthened regressions use the same visible identifier with different
nonzero generations in both the compact snapshot and direct stable-owner
lookup. The end-to-end process-clause regression continues to prove that an
orphaned best candidate is removed before the next clause is processed.

Exact audit results are retained in
[`results-summary.json`](results-summary.json).

## Compatibility decision

The migrated request for stable handles is complete. A `ClauseDerivationRef`
is the safe equivalent of C's long-lived `Clause_p` identity without depending
on a Rust address that changes when owned clauses move or sparse stores compact.
The selection and cleanup snapshots are lookup strategies over stable handles,
not substitutes for identity.

A proof-wide maintained liveness registry would trade the current candidate-
local indexed lookup and occasional bulk snapshot for mutation bookkeeping and
per-clause memory. It is optional optimization work and should be attempted only
after representative profiling; it is not a correctness or owner-lifetime gap.

## Validation

- reproducible source/owner audit: 14/14 contracts passed;
- same-ID/different-generation snapshot regression: passed;
- same-ID/different-generation stable-owner regression: passed;
- end-to-end orphan selection and cleanup regressions: passed;
- all-target/all-feature suite: 4,265 library tests plus every auxiliary target
  passed;
- strict all-target/all-feature pedantic Clippy and formatting: passed;
- all four C-source documentation integrity gates: passed; and
- experiment script compilation, diff check, and vendored-tree check: passed.
