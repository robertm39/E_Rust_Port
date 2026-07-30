# Preregistration

Recorded before implementing the adapter or running a conformance case.

## Question

Can a conservative, deterministic adapter preserve the typed TPTP semantics
of a useful linear `$int`/ground-`$rat`/`$real` fragment while lowering it to
VIRAS's one-real-domain LIRA representation, and can unsupported or
underspecified arithmetic fail with stable, specific reasons?

## Architecture boundary

The prototype is an experiment-only Python program. It accepts exactly one
closed `tff` axiom or conjecture containing only arithmetic. It performs its
own typing and linearity checks before constructing:

```text
Term =
    Var(real variable)
  | Rational(exact value)
  | Add(terms)
  | Scale(exact rational, term)
  | Floor(term)

Formula =
    True | False | Atom(term relation 0)
  | And(formulas) | Or(formulas)
  | Exists(real variable, formula)
  | Forall(real variable, formula)
```

The adapter returns canonical JSON, a canonical real-sorted TFF re-embedding,
and a derivation trace. It does not clausify, eliminate quantifiers, invoke a
solver, or modify Umlaut.

## Exact semantics

The candidate must:

- parse integer, rational, and finite-decimal/exponent numerals exactly;
- keep TPTP's three input sorts disjoint and require explicit coercions;
- lower existential integer binders to real binders conjoined with
  `X = floor(X)`;
- lower universal integer binders to real binders whose body is guarded by
  `X = floor(X)`;
- embed integer and ground rational values exactly in the real LIRA domain;
- map `$to_int` to mathematical floor, including on negative values;
- treat `$to_real` as the value embedding;
- accept `$to_rat` only from `$int` or `$rat`, where the real-domain value is
  preserved;
- rewrite ceiling as `-floor(-X)`;
- accept sum, difference, unary minus, and product by a compile-time rational
  constant;
- accept exact quotient only by a compile-time nonzero rational constant;
- normalize equality, disequality, and the four order predicates to a term
  related to zero;
- eliminate implication, equivalence, and negation into negation normal form;
  and
- alpha-rename output binders deterministically.

The renderer must use explicit `$to_real` coercions for exact integer and
rational constants, so its output is well typed without relying on implicit
numeric coercion.

## Conservative rejection policy

The candidate must reject with stable codes:

- quantified `$rat`, because the rationals are not definable as a subset of
  the VIRAS real-plus-floor domain;
- `$to_rat` or `$is_rat` on a `$real`, whose rationality semantics cannot be
  represented by the LIRA grammar;
- variable-by-variable products;
- nonconstant or zero divisors;
- integral quotient/remainder families, truncate, round, and unsupported
  defined operators;
- arithmetic-valued uninterpreted functions;
- implicit mixed-sort arithmetic or equality;
- free variables, non-TFF dialects, multiple formulas, and malformed input.

Rejecting a construct is a supported result. No rejected formula may be
approximated, silently treated as uninterpreted, or converted to `false`.

## Frozen conformance population

`cases.json` contains 12 accepted and 16 rejected cases. Accepted cases cover:

- existential and universal integer guards;
- negative `$to_int`;
- floor and ceiling;
- ground rational comparison;
- exact integer quotient re-embedded through `$to_real`;
- explicit integer/real coercion;
- `$is_int`;
- scalar multiplication;
- all comparison orientations; and
- Boolean normalization.

Rejected cases cover every policy class above. Case names, source text,
expected disposition, and expected error code are fixed before
implementation.

## Independent equivalence checks

`independent_oracle.py` must not import the candidate adapter. It will parse
the original TFF and canonical re-embedding independently and evaluate both
with exact `fractions.Fraction` arithmetic over:

- integer values `-3` through `3`; and
- real values `-3, -5/2, ..., 5/2, 3`.

For translated integer quantifiers, the real domain contains exactly those
integer samples plus nonintegral samples, so missing or reversed integrality
guards are observable. The oracle will also evaluate the canonical LIRA JSON
with a separate interpreter.

The frozen accepted cases must agree across original TFF, canonical LIRA, and
re-embedded TFF. A fixed seed `0xA11DA7A` will generate 500 additional closed,
well-typed linear formulas from the accepted grammar; all three views must
agree. This is a finite conformance oracle, not a complete proof over the
infinite domains.

At least four mutations must be rejected by the equivalence gate:

1. remove a universal integer guard;
2. replace mathematical floor with identity on a negative input;
3. reverse a comparison;
4. change a rational scale coefficient.

## Stability and syntax gates

- Repeated imports must be byte-identical.
- Whitespace-only input variants must yield identical logical JSON, TFF, and
  trace output.
- Every rejected case must reproduce its exact code twice.
- The re-embedded formula must pass an independent syntax/type parser.
- If Umlaut cannot parse the official predefined arithmetic types without
  redundant declarations, that implementation limitation is reported
  separately and does not weaken the adapter contract.

## Decision rule

Recommend this boundary for a future production adapter only if:

1. all 12 accepted cases agree in all three semantic views;
2. all 16 rejected cases fail with their exact preregistered code;
3. all 500 generated cases agree;
4. every mutation is detected;
5. stability and independent syntax/type checks pass; and
6. no construct is accepted through underspecified semantics.

If the boundary passes but its supported fragment is too narrow for arbitrary
TPTP arithmetic, retain it as an implementation contract for the later LIRA
kernel rather than claiming general arithmetic support.

