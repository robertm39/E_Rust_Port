# `ccl_eqn` parser reconciliation

## Status

Completed for Bead `E_Rust_Port-j76.2.111`. The vendored C source remains
unchanged.

## Source comparison

The Rust `EqnParse`, `EqnFOFParse`, and `EqnParseInfix` paths already delegate
term operands to `TermBank::parse_term_with_distinct_checks`, the represented
`TBTermParse` equivalent. This was stale documentation rather than a remaining
simple-parser implementation gap. The list-level integer, rational, float, and
object regression in `ccl_eqnlist` independently exercises those literal paths.

Two exported C parser helpers were still absent:

- `EqnHOFParse` parses its left operand as a banked term, preserves C's unusual
  close-parenthesis/continuation protocol, parses a non-Boolean equality right
  operand as a term, and hands a Boolean equality right operand to
  `TFormulaTSTPParse`. Before that formula handoff, C encodes the Boolean left
  operand as `$eq(left,$true)`.
- `EqnTBTermParse` reuses the ordinary CNF-literal parser and returns the result
  as a shared `$eq`/`$neq` term in normal argument order without allocating an
  intermediate literal cell.

Rust now exposes both shapes. The ordinary parser was factored into a
term-triple helper so `eqn_tb_term_parse` likewise avoids an intermediate
`Eqn` allocation.

## Regressions

Focused tests require:

- a Boolean left operand with a compound disjunctive right operand, proving the
  right side uses the formula parser rather than `TBTermParse`;
- exact `$eq(left,$true)` encoding on the Boolean left side and represented
  formula-atom encoding on the right;
- C's close-before-equality continuation result and negative polarity;
- close consumption for a predicate literal without equality;
- rejection of a function symbol in predicate position; and
- `$neq(left,right)` output from `EqnTBTermParse` with normal argument order.

## External comparison status

The parser branches were reconciled directly against the unchanged
`eprover/CLAUSES/ccl_eqn.c` implementation. The checked C executable is a Linux
binary, while the active Windows account has no installed WSL distribution
(`wsl --list --quiet` is empty and `Ubuntu-24.04` previously returned
`WSL_E_DISTRO_NOT_FOUND`). No unobserved executable comparison is claimed.

## Validation

- all 16 focused equation parser/printing tests pass;
- the full library, integration, and binary target suites pass;
- formatting and strict Clippy pass; and
- source-document coverage, Change Later wording, links, and regeneration
  preservation pass.
