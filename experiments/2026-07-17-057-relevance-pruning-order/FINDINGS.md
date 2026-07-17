# Relevance-pruning order reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.81`. Rust preserves C's observable
reversed relevance-bucket order while using stable `PListHandle` values instead
of allocator addresses inside the function-symbol index. The vendored C
checkout remained unchanged.

## C ordering surfaces

`ClauseSetSplitConjectures` and `FormulaSetSplitConjectures` traverse their
source sets in order but call `PListStoreP(anchor, ...)`, which inserts after
the anchor. Conjecture and rest buckets are therefore reversed. Rust's
`PListArena::store_after` deliberately retains that reversal.

C's `extract_new_core` repeatedly reads the root of a splay `PTree` whose keys
are raw `PList` cell addresses. The most recently indexed cell becomes the
initial root, but later roots depend on pointer ordering, splay history, and the
global size allocator's free-list state. Those addresses are not a portable or
reproducible semantic key.

Rust indexes the same list cells by stable handle index and takes the smallest
handle in each function-code bucket before moving the cell after the new core
anchor. This preserves the ordinary C traversal shape without coupling proof
behavior to allocator addresses or introducing unsafe pointer ordering.

## Reference fixtures

[`compare_order.py`](compare_order.py) compares the observable TSTP entry-name
sequence, exit status, and stderr against the cached C reference at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. Each C case is repeated in five
fresh processes.

| Fixture | Surface | Observed C/Rust order |
| --- | --- | --- |
| `problem.p` | two conjectures plus six same-bucket clauses | `goal2, goal1, ax6..ax1` |
| `formulas.p` | formula-owner equivalent | `goal2, goal1, ax6..ax1` |
| `layers.p` | three levels with overlapping `f`/`g`/`h` buckets | `goal, bridge3..bridge1, g3..g1, h2..h1` |

All 3/3 cases match across all five C repetitions. The complete sequences and
compact output hashes are retained in
[`results-summary.json`](results-summary.json). C renders parsed CNF owners as
`fof` in this output mode while Rust retains `cnf`; that pre-existing rendering
difference is outside this ordering audit, so the comparison intentionally
uses entry names rather than normalized formula text.

## Permanent regression

`relevance_data_preserves_c_observed_split_and_same_bucket_order` pins the
multi-conjecture and same-function bucket sequence at the data-structure
boundary. Existing relevance tests continue to cover multi-level symbol
expansion, formula/clause owner interaction, rest buckets, pruning levels, and
removed-count accounting; `FIndex` tests cover stable `PListHandle` insertion
and removal.

## Validation

- order reference matrix: 3/3 cases, five C runs per case;
- `cargo test --locked --all-features --lib relevance --quiet`: 19 passed;
- `cargo test --locked --all-features --lib findex --quiet`: 8 passed;
- experiment script compilation, formatting, strict pedantic Clippy, and all
  C-source documentation gates: passed; and
- the immediately preceding release/full-suite baseline passed 4,260
  all-feature library tests plus every target.

## Residual scope

Source-order vectors remain optional post-compatibility readability work under
`E_Rust_Port-j76.4.367`; further review of C's raw-root quirk remains `.4.368`
and `.4.203`, and clause/formula traversal deduplication remains `.4.369`.
Those items do not require allocator-address ordering in the Rust port.
