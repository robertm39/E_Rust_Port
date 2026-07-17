# Frequency-vector index fidelity audit

## Status

Completed for Bead `E_Rust_Port-j76.2.116`. Rust now exposes the exact output
fragments produced by C `FVIndexPrint` when its requested stream differs from
`stderr`, and the existing successor-map accounting already matches C's
constant-memory compatibility counter. The vendored C source remains unchanged.

## Output-routing oracle

C does not consistently use `FVIndexPrint`'s `out` parameter. The root marker,
alternative labels, and the space-plus-newline after each leaf go to `stderr`;
tree indentation and `ClausePrint` bodies go to `out`. For an index path
`[2, 0]`, the exact LOP fragments are:

- `out`: `--------fv_index_a=fv_index_b <- .`
- `stderr`: `* ROOT *\nAlternative 2: \nAlternative 0: \n \n`

Rust's existing combined renderer remains the useful representation of the
`out == stderr` case. New split-stream LOP and format-aware renderers return the
two independent strings without mutating process-global output. Exact tests pin
LOP bytes and verify that TPTP clause dispatch changes only the `out` fragment.

## Storage accounting

C `FVIAnchorCell.storage` is an insertion-side compatibility counter, not live
memory use. A new successor charges the change in `IntMapStorage` plus one
16-byte `FVIndexCell`; deletion leaves the tree and counter unchanged. Because
C subtracts the old map storage before adding the new storage, a representation
transition may produce a negative insertion delta.

Rust maintains a parallel C-shaped `IntMap<()>` solely for these estimates and
applies its signed delta to the cumulative anchor value. Existing focused tests
cover the 36-byte first successor, 72-byte dense and 64-byte sparse second
successors, and the exact `-144` delta when the tested tree becomes an array.
The latter test also asserts the map representation before and after the switch.

Actual Rust heap use and logical index statistics remain separate
post-compatibility profiling work; they must not change `FVIndexStorage`.

## Validation

- focused all-feature FV-index tests pass, including exact split-stream LOP and
  TPTP routing plus dense/sparse/tree-to-array storage transitions;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
