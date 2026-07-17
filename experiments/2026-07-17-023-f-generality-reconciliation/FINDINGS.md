# Generalized SInE distribution reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.114`. Every exported operation and
supported generality branch in `ccl_f_generality` is represented in Rust. The
vendored C source remains unchanged.

## Source comparison

The paired C implementation and header were checked against
`src/clauses/f_generality.rs`, and `GSinEParse` was checked against the Rust
axiom-filter parser. Both parsers recognize the shared ten-value name table but
reject every selection measure except term and formula counts. Both D-relation
implementations sort only symbols in the current clause or formula, retain the
inclusive internal-symbol boundary, apply the truncating benevolence limit,
cap it by the generosity-indexed entry, and use the same three-key comparator
orders.

The existing Rust implementation already covered clause/formula accumulation,
set-stack add and backtrack operations, scratch reset, implication trimming,
internal-symbol filtering, comparator tie-breakers, and C-shaped debug output.
A new end-to-end selection regression constructs counts for which term order
and formula order deliberately disagree; `CountTerms` selects the lower term
frequency while `CountFormulas` selects the lower clause/formula frequency.

## Resize correction

C `GenDistribSizeAdjust` reallocates the dense distribution and initializes
only the appended f-code range. Rust previously extended the vector and then
rescanned every historical entry to repair f-codes, even though existing
entries cannot be malformed through the safe API. Repeated one-symbol growth
could therefore accumulate quadratic scan work.

Rust now extends directly from the old size through the new size, preserving
historical cells and initializing each appended f-code once. It still recreates
the entire scratch vector as zeroes, exactly as C does. The regression dirties
an old scratch slot before growth and verifies preserved historical counts,
the sequential new cell, and complete scratch reset.

## Validation

- all 14 focused `f_generality` tests pass;
- all 4,227 library tests plus every integration and binary target test pass;
- formatting and strict Clippy pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
