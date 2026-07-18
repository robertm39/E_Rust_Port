# Conjecture-term weight owner-context reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.72`. Relative-term, prefix, TF-IDF,
Levenshtein, structural, and tree-distance weights run through Rust's live
proof-control owners while preserving C's lazy initialization and scoring
order. The vendored C checkout remained unchanged.

## Question

Do all six conjecture-term scorer families preserve C's proof-state-backed
lazy initialization, conditional maximality marking, and scoring order through
production HCB evaluation?

## Method

[`compare_term_weights.py`](compare_term_weights.py) checks each C compute
sequence against its Rust initializer, banked helper, and callback; separately
pins TF-IDF's score-before-document-update rule; audits the six-weight
proof-control regression; and compares every advertised evaluator through the
Rust and isolated C executables on a TPTP conjecture fixture.

## Findings

The source audit passes all six aggregate contracts across all six families:

- C lazily builds related-conjecture state, conditionally marks maximal terms,
  then performs term-extension scoring. Each Rust initializer installs its
  matching banked callback, and each callback preserves init/mark/score order
  with the active mutable OCB, term bank, and clause.
- TF-IDF keeps its additional state transition after scoring: generated-clause
  terms are inserted into the document-frequency store only after the current
  score is fixed.
- A proof-control regression installs all six definitions in one parsed active
  HCB using a represented negated-conjecture owner. It evaluates two initially
  unmarked clauses through the same lazy WFCBs, obtains six identical
  evaluation slots, and proves both clauses are oriented and maximally marked.

All six executable cases are byte-exact against the isolated C reference:

| Evaluator | Exact |
| --- | :---: |
| `ConjectureRelativeTermWeight` | yes |
| `ConjectureTermPrefixWeight` | yes |
| `ConjectureTermTfIdfWeight` | yes |
| `ConjectureLevDistanceWeight` | yes |
| `ConjectureStrucDistanceWeight` | yes |
| `ConjectureTreeDistanceWeight` | yes |

Exact exit codes, byte counts, hashes, and future mismatch payloads are retained
in [`results-summary.json`](results-summary.json).

## Ownership decision

C stores `ProofState_p` so lazy initialization can inspect the axiom owner and
stores `OCB_p` for conditional marking. Rust snapshots the clause axioms at
parse time and lends the current mutable OCB, term bank, and clause at
evaluation. This is behaviorally current in the executable: proof control is
installed after formula CNF, and the active HCB first evaluates while the full
clause-axiom context remains available. Owned snapshots avoid raw pointers into
movable proof state; borrowing them later is an optional profiled optimization.

The immutable compute callbacks remain deliberate low-level/test adapters for
already-marked clauses. Production HCB evaluation has no immutable call site,
as recorded by
[`experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md`](../2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md).
Removing those adapters is optional public-API simplification, not missing
proof-search ownership behavior.

## Validation

- reproducible source audit: all six aggregate contracts passed across all six
  families;
- focused six-weight proof-control regression and 55 module unit tests: passed;
- executable C/Rust matrix: 6/6 byte-exact;
- all-target/all-feature suite: 4,268 library tests plus every auxiliary target
  passed;
- strict all-target/all-feature pedantic Clippy, release `eprover`, and
  formatting: passed;
- all four C-source documentation integrity gates: passed; and
- experiment script compilation, diff check, and vendored-tree check: passed.
