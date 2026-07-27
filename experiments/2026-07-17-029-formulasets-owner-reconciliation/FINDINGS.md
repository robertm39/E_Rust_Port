# `ccl_formulasets` owner reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.108`. The migrated statement accumulated
many incremental limitations that no longer describe the production port. No
Rust implementation change was required; this slice reconciles the C owner
contract, current Rust integration, and the remaining post-compatibility/parser
work. The vendored C source remains unchanged.

## Set and wrapper surface

The exported `ccl_formulasets` set operations are represented by the owned
`FormulaSet`/`WrappedFormula` surface: append-order insertion and set transfer,
first/exact extraction, deletion, cardinality, clearing, conjecture splitting
and counts, untyped/interpreted-symbol scans, standard weight, f-code
collection, stack helpers, printing/app encoding, GC marking, polarity marking,
and definition statistics. Wrapper allocation, source metadata, formula-owned
derivations, clause conversion, simplification, question handling, FOOL and
definition phases, CNF, and output helpers preserve the source-shaped side
effects documented with the neighboring C wrapper unit.

Rust uses `VecDeque` ownership instead of mutating intrusive list links. Each
wrapper has a storage-independent `entry_id`; extraction and movement transfer
the complete wrapper, while flat-copy operations allocate a new entry identity
and preserve the visible formula id. This retains C's append/drain behavior
without exposing raw pointers.

## Exact proof ownership

Formula derivation parents contain both the visible formula id and the stable
wrapper `entry_id`. `ProofState::proof_formula_by_derivation_ref` resolves that
exact source key across all four proof owners:

- `definition_formula_archive`;
- `f_archive`;
- `f_ax_archive`; and
- `f_axioms`.

Existing regressions distinguish flat-copy sources with the same visible id and
follow formula quote chains through the archives. This is the safe equivalent
of C's retained wrapper pointers. A stable arena could make lookup constant-time
after compatibility, but pointer-address stability is not a current correctness
or proof-object gap.

## Production routing

Supported FOF/TFF/TCF/THF owners populate `ProofState::f_axioms` in `eprover`.
The represented route covers syntax-only ownership, pretty printing, app
encoding, conjecture preprocessing, initial and CNF proof documentation,
formula archives, clause generation, pruning, and proof search. Higher-order
named-to-DB, ITE, LET, definition-symbol, lambda-to-forall, and post-CNF lambda
lifting occur in C phase order.

`classify_problem`, `eground`, `enormalizer`, and `epatternize` request the
CNF-oriented represented-owner path for their supported FOOL/lambda inputs.
Those helpers intentionally use non-documenting formula transformations because
they do not own an `eprover` proof-document stream.

## Compatibility decision

The remaining unsupported parser spellings are tracked as broader parser/root
backlog, not as missing `ccl_formulasets` ownership. First-entry term-bank
coupling, process-global output/id policies, exact temporary scratch-flag side
effects, and a possible constant-time formula arena remain explicit
post-compatibility design items. They do not block the supported owner/CNF
pipeline or drop-in behavior already covered by executable tests and archived
C/Rust reports.

Existing evidence includes the formula lifecycle/allocation study in
`experiments/2026-07-16-061-formula-lifecycle/` and the executable formula-owner
comparisons linked from `docs/rust-port-status.md`. This documentation-only
reconciliation changes no behavior, so it does not manufacture a new external
comparison claim.

## Validation

- all 85 `clauses::formulasets` unit tests pass;
- all three exact proof-state formula-parent lookup tests pass;
- all 41 executable tests selected by `formula_owner` pass;
- all 13 `formula_set_cnf2` tests pass;
- the immediately preceding unchanged-code baseline has 4,230 passing default
  library tests, 4,235 passing all-feature library tests, every binary and
  integration target, and strict all-target/all-feature Clippy; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass after the reconciliation.
