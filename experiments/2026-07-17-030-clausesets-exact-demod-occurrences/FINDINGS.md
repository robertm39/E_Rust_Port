# Exact clause-set demodulator occurrences

## Status

Completed for Bead `E_Rust_Port-j76.2.107`. The vendored C source remains
unchanged.

## Question

Does the remaining `ccl_clausesets` compatibility note identify a missing
`PDTreeFindNextDemodulator` traversal, or only an incomplete owner payload at
the PDT leaf?

## C/Rust trace

C stores live `ClausePos*` values at PDT leaves. The returned position carries
both the exact clause pointer and equation side, and the accepted matcher
bindings remain active in the caller substitution.

Rust already had the equivalent incremental first-order traversal in
`PdTree::search_next_matching_occurrence_with_subst`: search-local frames keep
branch/backtracking state, the returned match leaves accepted bindings active,
and production rewrite consumes that state directly. Unit simplification uses
the same recorded leaf order and validates the other equation side. The
remaining mismatch was the leaf owner payload: it contained only the visible
clause identifier and side, then resolved the identifier through the first
matching set entry.

## Change

`PdtIndexedOccurrence` now carries the exact `ClauseDerivationRef` while
retaining the visible `clause_id` compatibility accessor used by standalone and
formula indexes. Clause-set PDT insertion, normalized-path caching, deletion,
rewrite, unit simplification, and their subsumption consumers preserve that
exact reference.

`ClauseSet` maintains a second ordered position map keyed by exact derivation
reference. It is updated on insertion/extraction and rebuilt after sparse-store
compaction, so candidate resolution remains logarithmic and does not re-scan
the set. This is the safe equivalent of C's live `ClausePos*` for the current
owner model.

## Regression

`demod_index_candidates_resolve_duplicate_visible_ids_exactly` inserts two
unit clauses with the same visible identifier and different generations under
one PDT key. It verifies reverse leaf priority resolves the second exact owner,
extracts that owner, and then verifies deletion leaves and resolves the first
owner. This would select or delete the wrong clause through the old first-ID
bridge.

The existing candidate-side regression was updated to assert the exact
reference payload. Existing rewrite, unit-simplification, subsumption, PDT, and
executable suites continue to exercise traversal order and binding behavior.

## Reconciled surface

- `ClauseSetParseList` delegates to the banked clause/equation parser, which now
  covers the represented `TBTermParse` token classes and preserves the
  `ClauseStartsMaybe` loop boundary.
- Initial clause documentation and property-filtered quote documentation are
  wired in `eprover` through explicit output/session owners rather than C
  globals.
- `split_conjecture_refs` preserves C's borrowed conjecture/rest partition and
  count; formula splitting remains correctly owned by `FormulaSet`.
- The full first-order indexed rewrite path already uses incremental PDT
  traversal/backtracking with accepted substitutions live. Higher-order
  matching follows the represented bank-aware materialized path because its
  bank/type contract differs, not because the clause-set owner is missing.

## Validation

- `cargo test demod_index_candidates_resolve_duplicate_visible_ids_exactly --lib`
- `cargo test demod_index_search_candidates_identify_current_clause_sides --lib`
- `cargo check --all-targets --all-features`
- all 4,231 default-feature library tests;
- all 4,236 all-feature library tests plus every binary and integration target
  with `--test-threads=1`; and
- strict all-target/all-feature Clippy and formatting.

The parallel all-feature run exposed two existing `enormalizer` fixture races
(process-global output capture and a colliding output-directory fixture). Both
tests passed in isolation, and the full serial run passed. No failure touched
the clauseset/PDT/rewrite/unit paths changed here.

No benchmark was added: the traversal algorithm and branch order did not
change. The owner lookup remains an ordered-map lookup, and the additional map
is rebuilt only at the same bounded sparse-compaction points as the existing ID
map.
