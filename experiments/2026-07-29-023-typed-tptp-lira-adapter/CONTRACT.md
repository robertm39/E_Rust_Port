# Conservative typed TPTP-to-LIRA contract

This is the implementation boundary recommended by experiment 023. It is not
a claim that Umlaut currently supports TPTP interpreted arithmetic.

## Input contract

The importer accepts one closed `tff` formula with role `axiom` or
`conjecture`. Every term must be pure arithmetic, every variable must have an
explicit `$int` or `$real` binder, and every operation must pass both TPTP type
checking and the linear-fragment check.

Input numeric types remain disjoint. There is no implicit `$int` to `$rat` or
`$real` conversion; the source must use `$to_rat` or `$to_real`. This follows
the [TPTP arithmetic system](https://tptp.org/UserDocs/TPTPLanguage/ArithmeticSystem.html)
and its [typed grammar](https://tptp.org/UserDocs/TPTPLanguage/SyntaxBNF.html).

## Sort lowering

| TPTP input | LIRA representation | Condition |
| --- | --- | --- |
| integer literal | exact rational value in the real domain | always |
| rational literal | exact rational value in the real domain | ground terms only |
| finite real literal | exact decimal value in the real domain | always |
| `$real` variable | real variable | direct |
| `? [I:$int] : F` | `? [I:$real] : (I=floor(I) & F)` | exact |
| `! [I:$int] : F` | `! [I:$real] : (I!=floor(I) \| F)` | exact |
| `$rat` variable | rejected | the rational subset is not LIRA-definable |

The output alpha-renames binders as `LIRA_V1`, `LIRA_V2`, and so on in source
traversal order. The importer rejects free and duplicate binders.

## Accepted term operations

| Source operation | Required types | LIRA result |
| --- | --- | --- |
| numeric literal | lexical `$int`, `$rat`, or `$real` | exact rational |
| `$uminus(X)` | any numeric sort | `Scale(-1,X)` |
| `$sum(X,Y)` | same sort | `Add(X,Y)` |
| `$difference(X,Y)` | same sort | `Add(X,Scale(-1,Y))` |
| `$product(X,Y)` | same sort; one translated operand constant | `Scale(c,T)` |
| `$quotient(X,c)` | same sort; nonzero translated constant divisor | `Scale(1/c,X)` |
| `$floor(X)` | any numeric sort | `Floor(X)`; identity on `$int` |
| `$ceiling(X)` | any numeric sort | `Scale(-1,Floor(Scale(-1,X)))` |
| `$to_int(X)` | any numeric sort | `Floor(X)`; identity on `$int` |
| `$to_rat(X)` | `$int` or `$rat` only | value embedding |
| `$to_real(X)` | any numeric sort | value embedding |

TPTP exact quotient on two `$int` operands has result sort `$rat`; the value
translation remains `Scale(1/c,X)`. Division by zero stays rejected because
TPTP leaves it unspecified.

## Accepted formulas

Equality, disequality, `$less`, `$lesseq`, `$greater`, and `$greatereq` require
same-sort operands and normalize to `Eq`, `Ne`, `Gt`, or `Ge` against zero.
`$is_int(X)` becomes `X=floor(X)` except on `$int`, where it is true.
`$is_rat(X)` is true on `$int`/`$rat` terms and rejected on `$real`.

The importer accepts `~`, `&`, `|`, `=>`, `<=`, `<=>`, `<~>`, `!`, and `?`.
It eliminates implication/equivalence and pushes negation through atoms and
quantifiers before returning the LIRA formula. `And` and `Or` nodes are
flattened, deduplicated, and structurally sorted. Exact constant arithmetic
and comparisons are folded.

## Fail-closed surface

| Code | Rejected condition |
| --- | --- |
| `UNSUPPORTED_RAT_QUANTIFIER` | a `$rat` binder |
| `UNSUPPORTED_REAL_TO_RAT` | `$to_rat` from `$real` |
| `UNSUPPORTED_REAL_RATIONALITY` | `$is_rat` on `$real` |
| `NONLINEAR_PRODUCT` | neither product operand is constant |
| `NONCONSTANT_DIVISOR` | quotient divisor is not constant |
| `ZERO_DIVISOR` | quotient divisor is zero |
| `UNSUPPORTED_ROUNDING` | `$truncate` or `$round` |
| `UNSUPPORTED_OPERATOR` | integral quotient/remainder or another unsupported defined symbol |
| `UNINTERPRETED_ARITHMETIC` | arithmetic-valued user function |
| `TYPE_MISMATCH` | implicit mixed-sort operation or equality |
| `UNBOUND_VARIABLE` | free variable |
| `UNSUPPORTED_DIALECT` | non-TFF input |
| `UNSUPPORTED_DOCUMENT` | more than one annotated formula |
| `MALFORMED_INPUT` | malformed token, grammar, or duplicate binder |

Unsupported input must return its error. It must never enter the LIRA kernel as
an uninterpreted term, incomplete candidate set, or Boolean constant.

## Output and derivation record

The canonical JSON contains:

- the source formula name and role;
- the canonical LIRA formula;
- one canonical real-sorted TFF re-embedding;
- ordered binder, coercion, term-rewrite, predicate, and relation trace steps;
  and
- a SHA-256 `canonical_id` over all logical output and trace fields.

The TFF renderer uses `$to_real` explicitly around exact integer and rational
constants. This prevents the output from relying on implicit numeric
coercions. The trace is an audit/explanation record, not yet a formal TSTP
proof. A production implementation must attach checkable derivations when the
later LIRA kernel performs quantifier elimination.

## Production placement

The production importer should consume Umlaut's already parsed and typed AST,
not reparse TPTP text. It should return `Result<LiraFormula, Unsupported>` and
keep the exact same failure taxonomy. The isolated LIRA kernel may assume
linearity, exact rationals, normalized Boolean structure, and one real domain
only after this adapter succeeds.
