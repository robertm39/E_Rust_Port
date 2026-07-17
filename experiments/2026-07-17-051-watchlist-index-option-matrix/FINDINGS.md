# Watchlist and index option matrix

## Status

Completed for Bead `E_Rust_Port-j76.2.87`. The focused executable surface is
13/13 byte-exact against the pinned C reference. No production code changed,
and the vendored C source remained unchanged.

## Question

Does the broad migrated watchlist/index item still own an executable behavior
gap, and is its claim that full PDT traversal is incomplete still current?

## Method

[`compare_surfaces.py`](compare_surfaces.py) runs the Windows release binary and
the isolated WSL C executable from commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. Exit status, stdout, and stderr are
compared without normalization. Windows fixture paths are translated to WSL;
only actual shell metacharacters are shell-quoted so C receives the inline
watchlist sentinel as one ordinary argv value.

The fixtures exercise:

- dynamic and static file watchlists;
- the explicit and optional-default inline-watchlist sentinel;
- disabled watchlist simplification;
- aggressive forward subsumption, direct/permuted FV indexing, feature type,
  maximum-feature, and slack settings;
- combined and independently selected fingerprint indexes;
- disabled PDT size and age constraints;
- conventional subsumption and C's repeated-option state retention; and
- invalid subsumption style, feature type, maximum-feature, and fingerprint
  index diagnostics.

## Results

All 13 cases are byte-exact:

| Case group | Cases | Result |
| --- | ---: | :---: |
| dynamic/static/no-simplification watchlist | 3 | exact |
| explicit/default inline watchlist | 2 | exact |
| combined/split/conventional/order-dependent indexes | 4 | exact |
| invalid index arguments | 4 | exact |

The compact hashes and complete mismatch payloads, if a future run regresses,
are retained in [`results-summary.json`](results-summary.json).

## PDT boundary audit

The migrated sentence predates the completed discrimination-tree work. Rust's
`PdTree::search_next_matching_occurrence_with_subst` performs incremental
first-order traversal/backtracking, retains accepted bindings in the caller's
substitution, checks type, weight, repeated-variable, size, and age constraints,
and preserves the configured symbols-first/variables-first order. Production
clause-set leaves carry an exact `ClauseDerivationRef` plus equation side and
resolve through the set-owned exact-position map across deletion and sparse
compaction.

That behavior is already covered by the live-substitution performance study in
`experiments/2026-07-14-002-pdt-live-substitution` and the exact-occurrence audit
in `experiments/2026-07-17-030-clausesets-exact-demod-occurrences`. The latter
closed `E_Rust_Port-j76.2.107`, the durable owner for the former
`PDTreeFindNextDemodulator`/live-`ClausePos` gap.

Post-compatibility questions about higher-order bank/type behavior, raw
standalone index contracts, allocator-sensitive C leaf ordering, cached-path
storage accounting, and global constraint/traversal state remain under the
existing `.3.214` and `ccl_pdtrees` Change Later Beads. They are not missing
watchlist/index option integration.

## Permanent Rust coverage

Existing regressions pin option-to-config conversion, FV-index installation in
`ProofControl`, fingerprint-index propagation to heuristic parameters, scoped
PDT constraint application/restoration, dynamic/static/file/inline watchlist
loading and documentation, global watchlist-index lifecycle, incremental PDT
matching with a live substitution, and exact duplicate-visible-id occurrence
selection/deletion.

## Validation

- focused C/Rust matrix: 13/13 exact;
- focused watchlist, index-option, PDT-constraint, and exact-occurrence Rust
  regressions: passed;
- unchanged-production full baseline: 4,257 library tests plus every binary and
  integration target passed under Cargo's default parallel runner; and
- unchanged-production strict pedantic Clippy baseline: passed; and
- formatting and all four C-source documentation integrity gates: passed.
