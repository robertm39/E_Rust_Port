# HCB parser-context integration

## Status

Completed for Bead `E_Rust_Port-j76.2.76`. Full executable/proof-control
installation and parser context are present. The vendored C checkout remained
unchanged.

## Question

Does Rust's `WeightParseContext` preserve the live behavior for C's
`HeuristicParse`/`HeuristicDefParse` `OCB_p` and `ProofState_p` parameters all
the way through executable HCB installation and evaluation?

## Method

[`compare_context.py`](compare_context.py) audits the parser/install/evaluation
handoff and compares proof-state-backed, ordering-dependent, axiom-derived, and
named context-backed HCB definitions against the isolated C reference.

## Findings

The source audit passed all ten contracts. Rust's live path is:

1. `eprover` lends its term bank, clause axioms, and formula axioms to
   `proof_control_init_with_formula_axioms`;
2. proof control creates `WeightParseContext` with clause axioms, formula
   axioms, and the proof-state signature;
3. default/configured HCB parsing forwards that same context through inline
   `WeightFunDefParse` equivalents; and
4. banked HCB evaluation receives the live mutable OCB and term bank.

All four executable cases are byte-exact:

| Case | Exit | Stdout bytes | Exact |
| --- | ---: | ---: | :---: |
| proof-state-backed inline WFCB | 0 | 192 | yes |
| ordering-dependent inline WFCB | 0 | 192 | yes |
| axiom-derived inline WFCB | 0 | 192 | yes |
| named context-backed HCB | 0 | 192 | yes |

The differing hashes for the ordering-dependent case also confirm that the
fixture exercises a different selected-clause order rather than only proving
that each parser accepts its arguments. Exact hashes and mismatch payloads, if
a future run regresses, are retained in
[`results-summary.json`](results-summary.json).

## Ownership decision

C's uniform parser callback signature passes `OCB_p` and `ProofState_p` even
when a parser does not need both. Several C WFCBs retain those raw pointers for
later scoring. Rust deliberately does not store a pointer into movable
`ProofControl` or `ProofState`: `WeightParseContext` exposes the immutable
axiom/formula/signature inputs needed during construction, and the WFCB compute
callback receives the current mutable OCB and proof-state term bank during
evaluation. This preserves current owner identity without a self-reference or
stale pointer.

A permanent HCB-admin regression now proves that context reaches an inline
proof-state-backed WFCB and that the same inline parser rejects an empty
context. Formula-aware, signature-aware learned, and banked evaluation paths
remain pinned by their narrower proof-control/WFCB tests.

## Validation

- new HCB parser-context regression: passed;
- source audit: all ten contracts passed;
- focused C/Rust executable matrix: 4/4 byte-exact; and
- all-target/all-feature suite: 4,265 library tests plus every auxiliary target
  passed;
- strict all-target/all-feature pedantic Clippy, release `eprover`, and
  formatting: passed;
- all four C-source documentation integrity gates: passed; and
- experiment script compilation, diff check, and vendored-tree check: passed.
