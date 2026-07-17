# TermPos debug-mode audit

## Status

Completed for Bead `E_Rust_Port-j76.2.136`. Both branches of C
`TermPosDebugPrint` already had Rust counterparts; this audit added exact
regression evidence for the last formatting details. The vendored C source
remained unchanged.

## C behavior

The nullable `Sig_p` selects one of two output modes for every stored
superterm/index pair:

- a null signature prints the superterm address as `<%p>`; and
- a non-null signature calls `TermPrint` with `DEREF_NEVER`, writes a literal
  `...`, calls `TermPrint` with `DEREF_ALWAYS`, then prints the child index.

Both modes use the same comment-prefixed opening, record, and closing lines.
The term mode inherits the conventional problem-type printer. The
higher-order path handles FOOL formulae and lambdas specially but does not
invoke the first-order `$let` printer, so `$let` is rendered as an ordinary
`@` application.

## Rust equivalence

`TermPos::write_debug_addresses` represents the null-signature branch with a
stable hexadecimal identity for the live Rust term handle. Its regression now
checks the exact complete record rather than only its prefix and suffix.

`TermPos::write_debug_terms` represents the non-null branch and routes both
dereference passes through the term bank's problem-type-aware printer. Existing
tests cover dereference bindings, first-order `$let`, higher-order application,
FOOL equality, and DB-lambda output. The new higher-order `$let` regression
pins the inherited ordinary-application surface explicitly.

## Validation

- focused `terms::termpos` tests cover both modes and inherited term surfaces;
- formatting and strict Clippy pass;
- full all-target/all-feature tests pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
