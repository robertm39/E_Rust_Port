# Formula relevance-owner reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.80`. Relevance pruning consumes the
represented clause and formula owners before clausification, reports C-shaped
combined removal counts, and exposes the same owner context to reusable
relevance-level WFCB initialization. The vendored C checkout remained
unchanged.

## Executable pipeline audit

Both C and Rust perform relevance pruning before formula CNF. The mixed fixture
contains a formula conjecture, a relevant formula bridge, an unrelated formula
axiom, a relevant clause bridge, and an unrelated clause axiom. At level three,
both implementations retain:

1. `goal` from the formula owner;
2. `formula_bridge` from the formula owner; and
3. `clause_bridge` from the clause owner.

The unrelated formula and clause are removed. The later CNF/statistics path in
both executables reports five parsed axioms, two relevance/SInE removals, three
initial clauses, three clauses entering saturation, and three current
unprocessed clauses.

Formula clausification then archives/drains the live `f_axioms` set before
`ProofControlInit`, so both executables report zero current archived/live
formula owners at that later statistics boundary. This is C's pipeline order,
not a missing parser-owner connection: the formula owners already participated
in pruning and remain represented in the proof archives/derivations where
required.

## Relevance-level WFCB context

Rust's reusable `proof_control_init_heuristics_with_formula_axioms` supports a
nonempty formula-owner context even though the ordinary executable initializes
proof control after CNF. The new
`proof_control_weight_context_preserves_formula_relevance_levels` regression
installs an option-defined `RelevanceLevelWeight` through
`WeightParseContext`, lazily computes its vector from a formula conjecture and
formula bridge, and obtains the expected score `15.0`.

This complements the direct funweight parser regression and proves that the
formula context survives the proof-control/WFCB admin boundary. Snapshot versus
direct `ProofState` ownership remains a narrower lifecycle/API question, not a
functional relevance gap.

## C/Rust comparison

[`compare_ownership.py`](compare_ownership.py) compares the cached C reference
at commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` with Rust. It checks retained
entry labels for `--prune` and the six stable ownership/accounting fields for
`--cnf --print-statistics`, plus exit status and stderr. Both 2/2 cases match;
compact output hashes and extracted values are retained in
[`results-summary.json`](results-summary.json).

## Validation

- mixed-owner C/Rust comparison: 2/2 cases;
- all relevance-filtered library tests: passed;
- focused proof-control formula-context, proof-state formula-pruning, and
  direct relevance-weight formula tests: passed;
- formatting, strict all-target/all-feature pedantic Clippy, experiment script
  compilation, and all C-source documentation gates: passed; and
- the complete all-target/all-feature suite: 4,262 library tests plus every
  auxiliary target passed; this and the preceding slice add two regression-only
  tests to the previously verified production baseline.

## Residual scope

Direct proof-state/OCB ownership for generic funweights remains
`E_Rust_Port-j76.2.73`; snapshot/owner cleanup remains `.4.776`, the special-
symbol initializer distinction remains `.3.209`, and formula-proof archive
lifecycle work remains independently tracked. None blocks formula-aware
relevance pruning or WFCB initialization.
