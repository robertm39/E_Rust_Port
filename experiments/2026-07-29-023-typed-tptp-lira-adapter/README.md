# Typed TPTP-to-LIRA adapter

This experiment evaluates a narrow, standalone frontend adapter for Bead
`E_Rust_Port-9jt.5.3`. It does not connect arithmetic reasoning to production
Umlaut.

TPTP arithmetic has three disjoint numeric sorts, `$int`, `$rat`, and `$real`.
The paper-derived VIRAS kernel instead has one real domain with integrality
represented through floor. The candidate adapter imports a closed, pure
arithmetic TFF formula into the small LIRA AST recommended by
`viras_docs/implementation-blueprint.md`, then re-embeds that AST as a
canonical real-sorted TFF formula.

The authoritative language references are:

- <https://tptp.org/UserDocs/TPTPLanguage/ArithmeticSystem.html>
- <https://tptp.org/UserDocs/TPTPLanguage/SyntaxBNF.html>

The supported boundary, rejection policy, frozen examples, independent
bounded equivalence oracle, and decision rule are preregistered in
`PREREGISTRATION.md`. Raw results belong under
`.artifacts/experiments/2026-07-29-023-typed-tptp-lira-adapter/`.

