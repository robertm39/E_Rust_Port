# Banked WFCB production audit

## Status

Completed for Bead `E_Rust_Port-j76.2.78`. The migrated claim that remaining
non-proof-control scoring paths still require immutable term-bank-only WFCB
evaluation is stale. No production behavior change was required, and the
vendored C checkout remained unchanged.

## Source audit

[`audit_banked_paths.py`](audit_banked_paths.py) scans the production prefix of
all 283 Rust source files, excluding each module's `#[cfg(test)]` section. It
rejects direct immutable evaluation outside the explicit WFCB/HCB adapter
modules and rejects immutable proof-control reweight calls outside their
defining compatibility module.

The audit reports zero forbidden production calls for:

- direct `.compute_eval(...)` outside `wfcb.rs`;
- immutable `.add_evaluation(...)` outside the WFCB/HCB adapters;
- immutable `hcb_clause_evaluate(...)` outside `hcb.rs`;
- immutable HCB set reweight outside the HCB/proof-control adapters; and
- immutable proof-control set reweight outside `proofcontrol.rs`.

The remaining immutable functions are deliberate low-level/test adapters. They
preserve a compact stateless API, while `compute_eval_with_bank` transparently
falls back to the immutable callback for WFCBs that do not need mutable owner
context.

## Production banked paths

The same audit records eight non-definition banked proof-control calls:

- active-clause evaluation during axiom initialization;
- processed-set reset evaluation;
- eval-store/generated-clause evaluation;
- initial `Uniq` set reweight;
- the central banked HCB set-reweight bridge;
- forward-contract set reweight;
- filter/trivial-cleanup reweight; and
- unprocessed cleanup reweight.

These cover every compatibility boundary named by the migrated issue. The
permanent WFCB/HCB tests already pin callback dispatch and clause-owned
evaluation cells; the higher-order lambda-order regression pins mutable OCB
preparation; the preceding learned-strategy regression pins proof-state
signature mutation through the active HCB.

## Ownership decision

Explicit `&mut OrderControlBlock`, `&mut TermBank`, and `&mut Clause` borrows are
the completed Rust replacement for C scorer data that reaches owner state
through raw `Clause`/`Eqn` back-pointers. Production callers borrow the actual
proof-state owner, so no movable self-reference, cached bank pointer, clause
copy, allocation, or extra traversal is introduced.

The exact call records are retained in
[`results-summary.json`](results-summary.json). Earlier executable and
performance evidence remains in the explicit-bank audit
[`experiments/2026-07-17-055-explicit-bank-wfcb-ownership/FINDINGS.md`](../2026-07-17-055-explicit-bank-wfcb-ownership/FINDINGS.md)
and the learned scorer benchmark
[`experiments/2026-07-17-059-shared-tsm-proof-state-bank/FINDINGS.md`](../2026-07-17-059-shared-tsm-proof-state-bank/FINDINGS.md).

## Validation

- reproducible production source audit: 283 files, zero forbidden immutable
  calls, eight banked proof-control calls;
- experiment script compilation: passed;
- all C-source documentation gates and formatting: passed; and
- immediate verified production baseline: strict pedantic Clippy, release
  `eprover`, and 4,264 all-feature library tests plus every auxiliary target.

## Residual scope

Collapsing the public immutable/banked adapter split is optional
post-compatibility API cleanup. It is not missing proof-search lifecycle work.
Narrow scorer-specific ownership questions remain tracked independently and do
not justify a generic stored proof-state pointer in `Wfcb` or `HcbCell`.
