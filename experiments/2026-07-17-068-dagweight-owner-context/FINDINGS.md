# DAG-weight owner-context reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.70`. `RDAGweight` runs through Rust's
live proof-control owners while preserving C's conditional maximal-term
marking, term-flag clearing, and scoring order. The other three DAG scorers
retain C's non-marking paths. The vendored C checkout remained unchanged.

## Question

Does refined DAG weighting borrow the current OCB, mutable owner bank, and
clause through production HCB evaluation without incorrectly imposing that
owner requirement on the three C scorers that do not mark maximal terms?

## Method

[`compare_dagweights.py`](compare_dagweights.py) checks the four C compute
surfaces against the four Rust initializers, the refined-DAG banked helper and
callback, and the four-weight proof-control regression. It also compares every
advertised evaluator through the Rust and isolated C executables on the same
equational LOP fixture.

## Findings

The source audit passes all seven ownership and ordering contracts:

- C `RDAGWeightCompute` conditionally marks maximal terms, clears `TPOpFlag`,
  and only then performs DAG scoring. Rust preserves that sequence through its
  banked helper and core scorer.
- `RDAGweight` is the only initializer in this family that registers a banked
  callback, and that callback forwards to the owner-aware helper.
- C `DAGWeightCompute`, `RDAGWeight2Compute`, and `RDAGWeight3Compute` never
  call `ClauseCondMarkMaximalTerms`; their Rust initializers deliberately use
  ordinary immutable callbacks rather than introducing extra mutation.
- A proof-control regression installs all four definitions in one parsed
  active HCB with `RDAGweight` first. It evaluates two initially unmarked
  clauses through the same WFCBs, obtains four bit-identical evaluation slots,
  and proves the refined-DAG callback oriented and maximally marked both
  clauses.

All four executable cases are byte-exact against the isolated C reference:

| Evaluator | Exact |
| --- | :---: |
| `DAGweight` | yes |
| `RDAGweight` | yes |
| `RDAGweight2` | yes |
| `RDAGweight3` | yes |

Exact exit codes, byte counts, hashes, and future mismatch payloads are retained
in [`results-summary.json`](results-summary.json).

## Ownership decision

C stores `OCB_p` only in the `RDAGweight` parameter block because only that
compute function conditionally marks. Rust keeps the parameter block
pointer-free and lends the active mutable OCB, term bank, and clause at HCB
evaluation. The other three scorers continue to borrow immutable clause/bank
views, matching their C mutation boundary exactly.

The immutable refined-DAG callback remains a deliberate low-level/test adapter
for already-marked clauses. Production HCB evaluation has no immutable RDAG
call site, as recorded by
[`experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md`](../2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md).
Removing that adapter is optional public-API simplification, not missing
proof-search ownership behavior.

## Validation

- reproducible source audit: all seven contracts passed;
- focused four-weight proof-control regression: passed;
- executable C/Rust matrix: 4/4 byte-exact;
- strict all-target/all-feature tests, Clippy, release build, and formatting:
  passed;
- all four C-source documentation integrity gates: passed; and
- experiment script compilation, rerun/diff check, and vendored-tree check:
  passed.
