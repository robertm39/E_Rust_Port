# Variable-weight owner-context reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.74`. The eight C variable-weight
evaluators that orient clauses before scoring all run through Rust's live
proof-control OCB and term-bank owner. The vendored C checkout remained
unchanged.

## Question

Does every variable-weight scorer that calls C `ClauseCondMarkMaximalTerms`
receive the active proof-control ordering and mutable owner bank in Rust, from
parser installation through HCB evaluation?

## Method

[`compare_varweights.py`](compare_varweights.py) audits all eight C
mark-then-score functions against their Rust initializer, banked callback, and
bank-backed maximality preparation. It also compares each advertised evaluator
through the Rust and isolated C executables on the same equational fixture.

## Findings

The source audit passes every owner contract:

- C has eight variable-weight compute functions that conditionally mark
  maximal terms before applying their scorer-specific formula.
- All eight Rust initializers install `wfcb_alloc_with_bank`, all eight banked
  callbacks forward to the matching banked compute helper, and every helper
  marks through the supplied mutable OCB and term bank before scoring.
- A proof-control regression installs parsed `Depthweight` inside an active
  HCB, evaluates an initially unmarked clause, and proves that the production
  HCB boundary attaches its evaluation after setting the clause/literal
  maximality cache.

All eight executable cases are byte-exact against the isolated C reference:

| Evaluator | Exact |
| --- | :---: |
| `TPTPTypeweight` | yes |
| `Sigweight` | yes |
| `Proofweight` | yes |
| `Depthweight` | yes |
| `WLessDWeight` | yes |
| `NLweight` | yes |
| `PNRefinedweight` | yes |
| `SymbolTypeweight` | yes |

Exact exit codes, byte counts, hashes, and mismatch payloads are retained in
[`results-summary.json`](results-summary.json).

## Ownership decision

C stores `OCB_p` in each `VarWeightParamCell` and recovers the term bank through
clause terms. Rust's explicit `&mut OrderControlBlock`, `&mut TermBank`, and
`&mut Clause` callback borrows are the completed safe equivalent: they name the
current proof-state owners without a raw back-pointer or movable
self-reference, and they preserve C's mark-then-score ordering without copying
the clause or allocating another bank.

The immutable compute callbacks remain deliberate low-level/test adapters for
already-marked clauses. Production HCB evaluation has no immutable call site,
as recorded by the generic banked-path audit in
[`experiments/2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md`](../2026-07-17-060-banked-wfcb-production-audit/FINDINGS.md).
Removing those adapters would be optional public-API cleanup, not missing
proof-search ownership work.

## Validation

- reproducible source audit: all five aggregate contracts passed across all
  eight mark-then-score families;
- focused proof-control owner-context regression and all 21 variable-weight
  unit tests: passed;
- executable C/Rust matrix: 8/8 byte-exact;
- all-target/all-feature suite: 4,266 library tests plus every auxiliary target
  passed;
- strict all-target/all-feature pedantic Clippy, release `eprover`, and
  formatting: passed;
- all four C-source documentation integrity gates: passed; and
- experiment script compilation, diff check, and vendored-tree check: passed.
