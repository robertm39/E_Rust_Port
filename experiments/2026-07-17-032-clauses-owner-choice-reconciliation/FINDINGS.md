# `ccl_clauses` owner and choice reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.105`. The parser, TSTP owner routing,
evaluation-object owner recovery, and clause-set identity portions of the
migrated statement had been completed by their dedicated owner slices. The
remaining runtime gap was C's explicit eta-before-beta normalization during
defined-choice recognition. The vendored C source remains unchanged.

## Source comparison

C `ClauseRecognizeChoice` rejects non-binary or equational candidates, then
normalizes both literal left sides with `LambdaEtaReduceDB` followed by
`BetaNormalizeDB`. A null map performs detection only. With a map, duplicate
choice symbols are rejected and the normalized terms replace the original
literal heads only after every structural check passes. `ClauseSetRecognizeChoice`
does not use its archive argument or move the recognized clause, despite the
nearby stale source comment.

Rust already had separate detection-only and mutating recording APIs, duplicate
rejection, set scanning, source-clause retention, and production choice
instantiation. It beta-normalized candidate heads but did not first eta-reduce
them.

## Runtime change

`clause_choice_candidate` now applies `lambda_eta_reduce_db` and then
`beta_normalize_db` to each candidate literal head in the same order as C. The
existing candidate object keeps normalization transactional: detection-only
does not mutate, while the recording path replaces both terms only after the
shape and duplicate checks succeed.

The regression constructs `~(lambda Y. P Y) X | (lambda Y. P Y)
(choice (lambda Y. P Y))`. It verifies recognition succeeds, both live literal
heads become the eta-reduced C shape, and the stored choice snapshot retains
the live clause's exact `ClauseDerivationRef`.

## Reconciled owner surfaces

- `ClauseParse` and `ClausePCLParse` reach the banked `TBTermParse`-equivalent
  parser through `EqnList`/`Eqn`, including distinct numeric/object forms,
  lists, direct equation encodings, FOOL, and higher-order terms.
- Direct clause TSTP rendering owns typed first-order and higher-order closure;
  formula-owner records use the separate `WrappedFormula`/formula-set renderer,
  matching the C ownership boundary in production mixed output.
- Formula closure uses the safe term-identity splay and root-right-left stack
  traversal corresponding to C's pointer-keyed `PTree`; allocation addresses
  are intentionally process-local in both implementations, so a cross-language
  same-sort address tie is not a stable textual compatibility requirement.
- Clause-set insertion assigns evaluation-object handles and maintains exact
  handle-to-sparse-slot recovery across extraction, sorting, and compaction.
- Clause sets retain private sparse ownership plus exact generational lookup;
  derivations and indexes use storage-independent `ClauseDerivationRef` values
  rather than recreating intrusive raw linkage.
- Choice-definition snapshots preserve exact generational derivation identity.
  Instantiation needs immutable definition terms and an exact proof parent, not
  pointer aliasing to the source set's storage.

## Validation

- 42 focused tests: four choice-recognition/proof-control tests, 33 clause
  parser/renderer/copy tests, two distinct-term equation-list parser tests, two
  evaluation-object sparse-slot tests, and the C-shaped formula free-variable
  `PTree` traversal test;
- 4,232 default library tests;
- 4,237 all-feature library tests plus every binary and integration target,
  run serially to avoid cross-process fixture races;
- strict all-target/all-feature Clippy, formatting, and release `eprover`
  build;
- source-document generation, Change Later wording, links, and regeneration
  preservation; and
- clean nested `eprover` status.

The eta-normalization comparison is source-shaped because this Windows
environment has no runnable upstream C executable. The Rust regression uses
the exact C normalization order and side-effect boundary; no performance
benchmark is needed for the two additional normalization calls on the rare
two-literal choice-axiom recognition path.
