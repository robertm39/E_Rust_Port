# Typed TPTP-to-LIRA adapter findings

## Decision

Retain the conservative adapter boundary for a future LIRA kernel. Do not
claim or enable general TPTP arithmetic support in production Umlaut.

The experiment passed every preregistered gate:

- 12/12 accepted cases agreed across the original typed TFF, canonical LIRA
  JSON, and canonical real-sorted TFF re-embedding;
- 16/16 unsupported cases returned their exact frozen code twice;
- 500/500 seeded generated formulas agreed in all three views;
- 4/4 semantic mutations were detected;
- repeated and whitespace-varied inputs produced byte-identical outputs; and
- Vampire 5.0.1 independently parsed and typed all 512 re-embedded formulas.

Final conformance report:
`18b22798975bcb9936304a162bb9a563a2639168433fd8829d4bc627fcacecc2`.
Vampire validation report:
`f95abd22e810a7e8bfd8919fae683f53dc416aea8e48c439fec4961e873f0687`.

## Why the boundary is narrow

The current [TPTP arithmetic system](https://tptp.org/UserDocs/TPTPLanguage/ArithmeticSystem.html)
gives `$int`, `$rat`, and `$real` disjoint types and ad-hoc polymorphic
operators. It also makes division by zero unspecified and `$to_rat` from an
arbitrary real only partially specified. VIRAS instead uses one real domain
plus floor.

Integer binders therefore have an exact floor-guard lowering. Real binders are
direct. Ground rational values embed exactly, but arbitrary rational
quantification does not: the set of rationals is not expressible by the
target linear real-plus-floor grammar. The adapter rejects that surface,
along with real rationality tests/coercion, instead of silently widening a
`$rat` variable to `$real`.

The same policy rejects variable products, variable/zero divisors, integral
division and remainder variants, truncate/round, arithmetic-valued user
functions, and implicit mixed-sort terms. These are deliberate supported
outcomes, not missing error handling.

## Exact semantics exercised

The frozen cases cover existential and universal integer guards, mathematical
floor on negative input, ceiling as `-floor(-x)`, ground rationals, integer
exact quotient returning `$rat`, explicit integer-to-real and integer-to-rat
coercion, `$is_int`, constant scaling, comparison orientation, implication,
equivalence, and exponent-form real literals.

The independent oracle imports no candidate code. Its separate parser,
type/evaluation rules, and LIRA interpreter use exact `fractions.Fraction`
arithmetic. It compares the three closed-formula views over integer samples
`-3..3` and real half-step samples `-3..3`. The generated seed was
`0xA11DA7A`; 449 formulas evaluated true and 51 false, with aggregate ID
`f0106a51f8a6776efd74cd617210d16a52eb4bdfd75b9d17c8f7a2594425b864`.

The oracle rejected each injected fault:

| Mutation | Source | LIRA after mutation | Re-embedding |
| --- | --- | --- | --- |
| removed universal integer guard | true | false | true |
| negative floor replaced by identity | true | false | true |
| non-strict comparison made strict | true | false | true |
| rational scale sign changed | true | false | true |

This finite-domain agreement is strong conformance evidence, not a proof over
the infinite arithmetic domains. The structural lowering rules in
`CONTRACT.md` remain the normative implementation boundary.

## Independent TFF validation

The output renderer uses explicit `$to_real` coercions for all exact constants.
Pinned Vampire 5.0.1, SHA-256
`3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665`,
ran theory clausification on the 12 frozen and 500 generated re-embeddings.
All 512 returned successfully with no syntax or type marker.

Umlaut itself currently lacks the required predefined ad-hoc-polymorphic
arithmetic signatures. The preceding induction experiment needed redundant
single-signature declarations even for `$sum` and `$greatereq`; the full
adapter output may use multiple `$to_real` overloads in one formula. That
frontend limitation belongs to production TFA parsing/theory work and does
not justify weakening the standard typed output.

## Stable output and trace

The adapter canonicalizes exact rationals, flattened additions, constant
scales, Boolean nodes, atom orientation, and binder names. Each output includes
an ordered trace of:

- binder sort lowering;
- exact coercions;
- floor/ceiling and linear term rewrites; and
- predicate/relation normalization.

The trace makes every experiment transformation inspectable and is included
in the canonical hash. It is not a formal proof of later quantifier
elimination. Production proof objects must connect the typed source, import
equivalence, kernel derivation, and re-embedded result.

## Recommendation

Use `CONTRACT.md` as the frontend contract when
`E_Rust_Port-9jt.5.2` supplies a trusted LIRA kernel:

1. translate from Umlaut's typed internal AST rather than the experiment's
   narrow text parser;
2. preserve the exact rejection codes and never approximate unsupported
   arithmetic;
3. reuse an arbitrary-precision exact rational backend;
4. retain explicit import and export derivation metadata; and
5. keep broader `$rat`, nonlinear, partial, and mixed-theory support outside
   this kernel unless separately justified.

No Rust source, package dependency, parser default, or prover schedule changed
in this study.

The complete ignored evidence archive is
`.artifacts/experiments/2026-07-29-023-typed-tptp-lira-adapter/lira-023-complete.tar.gz`.
It is 79,676 bytes with SHA-256
`2102fe4050dcbdad2eb29dee50c0e04c8ac7b67c53c2bf5a2c86527a78cc3056`.
