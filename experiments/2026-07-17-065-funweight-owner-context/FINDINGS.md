# Funweight owner-context reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.73`. Generic and symbol-offset
funweights run through Rust's live proof-control OCB and term-bank owner while
preserving lazy initialization and occurrence-state cleanup. The vendored C
checkout remained unchanged.

## Question

Do all generic funweight initializers and `SymOffsetWeight` preserve C's lazy
initialize, conditionally mark, score, and reset ordering through production
HCB evaluation?

## Method

[`compare_funweights.py`](compare_funweights.py) checks the ordered C and Rust
compute steps, every Rust WFCB initializer that shares the generic callback,
the symbol-offset callback, and the proof-control regression. It also compares
explicit `FunWeight` and `SymOffsetWeight` definitions through the Rust and
isolated C executables.

## Findings

The source audit passes all seven contracts:

- C `GenericFunWeightCompute` calls its lazy initializer, conditionally marks
  maximal terms, then scores with the initialized symbol/type vectors. Rust's
  banked helper preserves the same order.
- All eight Rust WFCB initializers that share generic funweight evaluation use
  the banked callback. This covers explicit weights, conjecture symbol/type
  variants, and clause/formula-backed relevance variants.
- C `SymOffsetWeightCompute` initializes and marks before base scoring, records
  distinct function symbols, adds one offset per symbol, and clears every
  touched occurrence slot. Rust's banked callback preserves that sequence.
- A proof-control regression installs explicit generic and offset weights in
  one parsed active HCB, evaluates two initially unmarked clauses through the
  same WFCBs, and obtains identical evaluation bits after both clauses are
  oriented and marked. This pins lazy reuse and occurrence-array reset at the
  production boundary.

Both executable cases are byte-exact against the isolated C reference:

| Evaluator | Exact |
| --- | :---: |
| `FunWeight` | yes |
| `SymOffsetWeight` | yes |

Exact exit codes, byte counts, hashes, and future mismatch payloads are retained
in [`results-summary.json`](results-summary.json).

## Ownership decision

C stores `OCB_p` and, for some lazy initializers, `ProofState_p` inside the
parameter cell. Rust keeps parse-time clause/formula context as owned snapshots
and lends the current mutable OCB, term bank, and clause at evaluation. This
avoids raw pointers into movable owners without changing executable timing:
both C and Rust install proof control after formula CNF and first evaluate the
active HCB while the full clause-axiom owner remains available. Formula-aware
relevance behavior is independently pinned by
[`experiments/2026-07-17-058-formula-relevance-ownership/FINDINGS.md`](../2026-07-17-058-formula-relevance-ownership/FINDINGS.md).

The immutable compute callbacks remain deliberate low-level/test adapters for
already-marked clauses. Production HCB evaluation has no immutable call site,
as recorded by
[`experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md`](../2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md).
Borrowing rather than snapshotting parse context and removing immutable
adapters are optional post-compatibility API/performance work, not missing
proof-search ownership behavior.

## Validation

- reproducible source audit: all seven contracts passed;
- focused proof-control owner-context regression and all 18 funweight unit
  tests: passed;
- executable C/Rust matrix: 2/2 byte-exact;
- all-target/all-feature suite: 4,267 library tests plus every auxiliary target
  passed;
- strict all-target/all-feature pedantic Clippy, release `eprover`, and
  formatting: passed;
- all four C-source documentation integrity gates: passed; and
- experiment script compilation, diff check, and vendored-tree check: passed.
