# `ccl_neweval` owner reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.106`. The migrated statement describes
an already implemented ownership split rather than a missing compatibility
surface. No runtime behavior changed, and the vendored C source remains
unchanged.

## C surface

`ccl_neweval` allocates a flexible `EvalCell` whose simple evaluations contain
one left/right pointer pair per heuristic position. The same cell is inserted
intrusively into every `ClauseSet::eval_indices` splay root. Set insertion and
extraction add/remove all roots; `ClauseSetFindBest` takes the smallest cell and
uses its raw `object` back-pointer to recover the owning clause.

The exported unit also defines priority mutation, C `float` heuristic storage,
FIFO count allocation, printing, comparison/greater-than quirks, tree
insertion/find/extraction/deletion, smallest lookup, traversal, and unsafe
detached-cell freeing.

## Rust compatibility adapter

`EvalTree` represents the complete standalone splay API with arena handles.
Existing tests pin root movement on insertion and hit/miss lookup, duplicate
return behavior, extraction relinking, deletion/freeing, smallest lookup,
in-order traversal, and print shape. `EvalCell` separately pins priority,
`f32`, FIFO count, NaN, equal-count, and formatting behavior.

This adapter intentionally retains `take_node`'s C-shaped unsafe ownership
contract for direct callers: removing a still-linked node can invalidate tree
handles. Production clause sets do not use that API.

## Production owner

`ClauseSet` stores exclusive clauses and owns a `BTreeSet<EvalIndexEntry>` per
evaluation position. Each inserted evaluation receives a set-local object
handle; `eval_object_slots` resolves that handle to the exact private sparse
slot. Extraction removes all evaluation roots before moving the clause, and
bounded compaction rebuilds the object map. Reweighting removes cells and then
rebuilds all roots after evaluation.

The ordered entry reproduces `EvalCompare`: priority first; equal FIFO counts
compare equal before heuristic; otherwise heuristic precedes the FIFO fallback.
The object handle is only a final tie-breaker for cloned equal evaluation cells,
which prevents a safe set from dropping an owner under a C invariant violation.
Unique production evaluation cells therefore retain exact C order.

`BTreeSet` insertion/removal is logarithmic and smallest lookup is direct, so
the production owner retains the splay tree's amortized asymptotic behavior
without raw multi-tree back-pointers.

## Production routing

- proof-state axiom initialization reweights with `Uniq` and consumes evaluation
  object handles in root order;
- active-HCB evaluation and `prefer_initial_clauses` priority changes happen
  before insertion into `unprocessed`;
- standard and single-weight selection use `find_best`/`extract_best`, including
  exact orphan deletion;
- `HCBClauseSetDelProp` consumes snapshots of each root's exact object order;
- processed reset evaluates and adjusts the requeued clause before insertion;
  and
- the C `eval_store` evaluate-after-insert quirk is represented by
  extract/evaluate/reinsert so the safe roots remain synchronized while visible
  clause order is preserved.

## Compatibility decision

The handle-backed splay adapter and safe clause-owned roots should remain
separate. Replacing production roots with the adapter would recreate raw-style
multi-owner linkage without adding behavior. The existing post-compatibility
items continue to track C's `f32` truncation, equal-count comparison assumption,
and unsafe detached free; they are not missing `ccl_neweval` functionality.

## Validation

- 14 `clauses::neweval` tests and nine focused clause-set evaluation-index,
  object-slot, HCB selection/traversal, proof-state initialization, eval-store,
  and move-to-unprocessed tests;
- source-document generation, Change Later wording, links, and regeneration
  preservation; and
- the immediately preceding unchanged-runtime baseline: 4,231 default library
  tests, 4,236 all-feature library tests plus every target, strict Clippy, and
  formatting.
