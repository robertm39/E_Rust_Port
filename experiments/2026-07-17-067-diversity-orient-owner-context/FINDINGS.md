# Diversity/orient weight owner-context reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.71`. `Diversityweight`, `Orientweight`,
and `OrientLMaxWeight` run through Rust's live proof-control owners while
preserving C's conditional maximal-term marking and scoring order. The
vendored C checkout remained unchanged.

## Question

Do all three diversity/orient scorer entry points borrow the current OCB,
mutable owner bank, and clause through production HCB evaluation, rather than
depending on stale immutable scoring state?

## Method

[`compare_weights.py`](compare_weights.py) checks each C compute sequence
against its Rust initializer, banked helper, and callback; audits the
three-weight proof-control regression; and compares every advertised evaluator
through the Rust and isolated C executables on the same equational LOP fixture.

## Findings

The source audit passes all nine behavioral and integration contracts:

- C's diversity scorer conditionally marks first, computes ordinary clause
  weight second, and applies function-symbol and variable diversity penalties
  last. Rust preserves that order through its banked helper.
- C's orient and orient-LMax scorers both conditionally mark first and then
  apply orientation/maximal-literal penalties. Both Rust helpers preserve the
  same order.
- All three Rust initializers register their matching banked callback, and all
  three callbacks forward to the owner-aware compute helper.
- A proof-control regression installs all three definitions in one parsed
  active HCB. It evaluates two initially unmarked clauses through the same
  WFCBs, obtains three bit-identical evaluation slots, and proves both clauses
  are oriented and maximally marked.

All three executable cases are byte-exact against the isolated C reference:

| Evaluator | Exact |
| --- | :---: |
| `Diversityweight` | yes |
| `Orientweight` | yes |
| `OrientLMaxWeight` | yes |

Exact exit codes, byte counts, hashes, and future mismatch payloads are retained
in [`results-summary.json`](results-summary.json).

## Ownership decision

C stores `OCB_p` inside each parameter block and reaches the proof-state term
bank through that owner. Rust keeps parameter blocks pointer-free and lends the
active mutable OCB, term bank, and clause when the HCB evaluates a generated
clause. This is the production path for all three parsed WFCBs and gives the
conditional marker the live bank it may need for ordering preparation.

The immutable compute callbacks remain deliberate low-level/test adapters for
already-marked clauses. Production HCB evaluation has no immutable call site,
as recorded by
[`experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md`](../2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md).
Removing those adapters is optional public-API simplification, not missing
proof-search ownership behavior.

## Validation

- reproducible source audit: all nine contracts passed;
- focused three-weight proof-control regression: passed;
- executable C/Rust matrix: 3/3 byte-exact;
- strict all-target/all-feature tests, Clippy, release build, and formatting:
  passed;
- all four C-source documentation integrity gates: passed; and
- experiment script compilation, rerun/diff check, and vendored-tree check:
  passed.
