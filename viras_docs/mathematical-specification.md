# Mathematical Specification of Base VIRAS

This document reconstructs the base VIRAS quantifier-elimination procedure from
the May 7, 2024 extended paper. It is intended to be executable as a
specification. Corrections and conservative edge-case extensions are identified
in [sources-and-errata.md](sources-and-errata.md).

## 1. LIRA kernel language

For the arithmetic kernel, use:

```text
term ::= variable
       | 1
       | q * term              where q is rational
       | term + term
       | floor(term)

atom ::= term = 0
       | term != 0
       | term > 0
       | term >= 0
```

All variables range over the reals. Define:

```text
0          := 0 * 1
q          := q * 1
-t         := (-1) * t
s - t      := s + (-t)
ceil(t)    := -floor(-t)
```

No multiplication of two non-constant terms is permitted. Floors may be nested.
Variables other than the variable currently being eliminated are parameters.

### 1.1 Literal normalization

Normalize every comparison before entering VIRAS:

```text
l =  r  ->  (l - r) = 0
l != r  ->  (l - r) != 0
l >  r  ->  (l - r) > 0
l >= r  ->  (l - r) >= 0
l <  r  ->  (r - l) > 0
l <= r  ->  (r - l) >= 0
```

Push negations to atoms:

```text
not(t = 0)   -> t != 0
not(t != 0)  -> t = 0
not(t > 0)   -> (-t) >= 0
not(t >= 0)  -> (-t) > 0
```

The one-variable kernel accepts a non-empty conjunction of normalized literals.
Handle `true`, `false`, and a variable absent from the formula before invoking
it.

### 1.2 Exact rational conventions

Every rational is stored in reduced form with a positive denominator. Periods
are canonicalized as nonnegative; every actual grid period is strictly
positive.

For a finite nonempty set `Q` of positive rationals, define:

```text
lcm_Q(Q) = lcm({abs(num(q)) | q in Q})
           --------------------------------
           gcd({den(q)      | q in Q})
```

Then `lcm_Q(Q) / q` is an integer for every `q` in `Q`.

For `p > 0`:

```text
quot_p(t) = floor(t / p)
rem_p(t)  = t - p * quot_p(t)
```

The required identities are:

```text
t = p * quot_p(t) + rem_p(t)
0 <= rem_p(t) < p
quot_p(t) is integer-valued
```

Divisibility and congruence can be represented as:

```text
p divides t    <=> rem_p(t) = 0
s == t mod p   <=> rem_p(s) = rem_p(t)
```

## 2. Term profile relative to an elimination variable

Every quantity in this section is relative to a fixed variable `x`. Cache
profiles by `(term identity, x identity)`.

### 2.1 Outer slope, segment slope, and period

The outer slope `oslp_x(t)` describes the common slope of the parallel linear
bounds on `t`. The segment slope `sslp_x(t)` is the slope between
discontinuities. The period `per_x(t)` describes a rational shift after which
the graph repeats up to its outer linear drift.

For any variable `y`:

```text
oslp(y) = 1 if y = x, otherwise 0
oslp(1) = 0
oslp(k*t) = k * oslp(t)
oslp(s+t) = oslp(s) + oslp(t)
oslp(floor(t)) = oslp(t)
```

```text
sslp(y) = 1 if y = x, otherwise 0
sslp(1) = 0
sslp(k*t) = k * sslp(t)
sslp(s+t) = sslp(s) + sslp(t)
sslp(floor(t)) = 0
```

```text
per(y) = 0
per(1) = 0
per(k*t) = per(t)

per(s+t) =
    per(s)                         if per(t) = 0
    per(t)                         if per(s) = 0
    lcm_Q({per(s), per(t)})        otherwise

per(floor(t)) =
    0                              if per(t) = 0 and oslp(t) = 0
    1 / abs(oslp(t))               if per(t) = 0 and oslp(t) != 0
    abs(num(per(t))) * den(oslp(t)) otherwise
```

Simplify `0*t` before this recurrence. In the final branch, the denominator of
zero is conventionally one, so a periodic, zero-outer-slope inner term with a
nonzero period remains well-defined.

The periodic-shift property is:

```text
t[x + per(t) * floor(y)]
  = t[x] + oslp(t) * per(t) * floor(y)
```

whenever `per(t) != 0`.

### 2.2 Symbolic linear bounds

Compute a symbolic lower-bound intercept `distYminus(t)`, a nonnegative rational
width `deltaY(t)`, and derive
`distYplus(t) = distYminus(t) + deltaY(t)`.

```text
deltaY(y) = 0
deltaY(1) = 0
deltaY(k*t) = abs(k) * deltaY(t)
deltaY(s+t) = deltaY(s) + deltaY(t)
deltaY(floor(t)) = deltaY(t) + 1
```

```text
distYminus(y) = 0 if y = x, otherwise y
distYminus(1) = 1

distYminus(k*t) =
    k * distYminus(t)              if k >= 0
    k * distYplus(t)               if k < 0

distYminus(s+t) = distYminus(s) + distYminus(t)
distYminus(floor(t)) = distYminus(t) - 1
distYplus(t) = distYminus(t) + deltaY(t)
```

The invariant is:

```text
oslp(t)*x + distYminus(t)
  <= t <=
oslp(t)*x + distYplus(t)
```

The bounds deliberately over-approximate. Tighter bounds improve performance
but are not needed for correctness.

### 2.3 Right-limit term

`lim_x(t)` is the value approached immediately to the right of the current
`x`. Define it structurally:

```text
lim(y) = y
lim(1) = 1
lim(k*t) = k * lim(t)
lim(s+t) = lim(s) + lim(t)

lim(floor(t)) =
    floor(lim(t))                  if sslp(t) >= 0
    ceil(lim(t)) - 1               if sslp(t) < 0
```

The condition uses the segment slope of the floor's argument.

### 2.4 Segment line and its zero

For a term `t` and an `x`-free point `b`:

```text
dseg_t(b) = -sslp(t)*b + lim(t)[x := b]
zero_t(b) = b - lim(t)[x := b] / sslp(t)
```

`zero_t(b)` is defined only when `sslp(t) != 0`.

Between neighboring discontinuities, the term agrees with:

```text
sslp(t)*x + dseg_t(b)
```

and `zero_t(b)` is the zero of that line.

## 3. Symbolic grids and finite grid intersection

A grid descriptor:

```text
Grid(base=s, period=p)
```

represents `{s + p*z | z in integers}`. Require `p > 0` and require `s` not to
contain the eliminated variable.

For the grid `s + p*Z`, define symbolic rounding:

```text
ceil_grid(t)       = t + rem_p(s - t)
floor_grid(t)      = t - rem_p(t - s)
ceil_grid(t+eps)   = floor_grid(t + p)
floor_grid(t-eps)  = ceil_grid(t - p)
```

For an interval with lower-bound kind `L` and upper-bound kind `M`:

```text
L in {"[", "("}
M in {"]", ")"}
```

and a rational width `k >= 0`, define:

```text
start = ceil_grid(t)      if L = "["
start = ceil_grid(t+eps)  if L = "("

Grid(s,p) INTERSECT L t, t+k M
  = {start + n*p | n in naturals and
                     n*p <= k if M = "]", else n*p < k}
```

`naturals` includes zero. The result is a finite set of ordinary terms. It is a
safe superset of the actual symbolic grid/interval intersection, not
necessarily an exact set. For `k = 0`, the safe extension is `{start}` for a
closed right endpoint and the empty set for an open right endpoint.

The exact finite loop bounds are:

```text
closed right: n = 0 .. floor(k/p)
open right:   n = 0 .. ceil(k/p)-1
```

with an empty range when its upper index is negative.

## 4. Discontinuity grids

`breaks_x(t)` is a finite set of grid descriptors whose union covers every
discontinuity of `t` as a function of `x`. It may contain extra points.

```text
breaks(y) = empty
breaks(1) = empty
breaks(k*t) = breaks(t)
breaks(s+t) = breaks(s) union breaks(t)
```

For `floor(t)`:

```text
if sslp(t) = 0:
    breaks(floor(t)) = breaks(t)

else if breaks(t) is empty:
    breaks(floor(t)) = {
        Grid(zero_t(0), per(floor(t)))
    }

else:
    breaks(floor(t)) = breaks(t) union breaksInSeg(t)
```

Compute `breaksInSeg(t)` as follows:

```text
P     = per(floor(t))
p_min = minimum grid period appearing in breaks(t)
q     = abs(1 / sslp(t))
out   = empty set

for each Grid(base=b0_prime, period=p) in breaks(t):
    B0 = Grid(b0_prime,p) INTERSECT [b0_prime, b0_prime+P)

    for each b0 in B0:
        B = Grid(zero_t(b0),q) INTERSECT [b0, b0+p_min)

        for each b in B:
            add Grid(b,P) to out

return out
```

Preconditions for the non-empty branch are `P > 0`, `p_min > 0`,
`sslp(t) != 0`, and every grid base being `x`-free. Assert them.

The denotation used in proofs is:

```text
breaks_infinity(t) =
    {base + period*z |
       Grid(base,period) in breaks(t), z in integers}
```

and every actual discontinuity must occur in `breaks_infinity(t)`.

## 5. Periodic and aperiodic literals

For a normalized literal `L = (t relation 0)`:

```text
L is periodic   iff oslp(t) = 0
L is aperiodic  iff oslp(t) != 0
period(L) = per(t)
```

A periodic literal repeats:

```text
L[x] <=> L[x + per(t)*floor(y)]
```

### 5.1 Aperiodic core interval

For `oslp(t) != 0`, define:

```text
distYsigned(t) =
    distYplus(t)  if oslp(t) > 0
    distYminus(t) if oslp(t) < 0

distXminus(t) = -distYsigned(t) / oslp(t)
deltaX(t)     = deltaY(t) / abs(oslp(t))
distXplus(t)  = distXminus(t) + deltaX(t)
```

The core interval is `[distXminus, distXplus]`. Both endpoints are terms not
containing `x`, and `deltaX` is a nonnegative rational.

### 5.2 Truth values at infinity

For sign `sigma` equal to `-1` or `+1`, an aperiodic literal has:

```text
limit_sigma(t = 0)   = false
limit_sigma(t != 0)  = true
limit_sigma(t > 0)   = (sigma * oslp(t) > 0)
limit_sigma(t >= 0)  = (sigma * oslp(t) > 0)
```

Below `distXminus`, the literal equals its negative-infinity truth value.
Above `distXplus`, it equals its positive-infinity truth value.

## 6. Virtual terms

Represent a virtual term as:

```text
VirtualTerm {
    base: ordinary LIRA term,
    epsilon: false or true,
    z_period: nonnegative rational,
    infinity: None, Negative, or Positive
}
```

It denotes the paper's:

```text
base + epsilon_flag*epsilon + z_period*Z + infinity_sign*infinity
```

The `Z` and infinity components are mutually exclusive. Canonicalize
`z_period = 0` away. A plain virtual term has only `base`.

These are syntax for virtual substitution, not values in the output logic.

## 7. Virtual substitution without a `Z` component

Write `VS(phi, x, v)` for virtual substitution.

### 7.1 Conjunction

```text
VS(AND_i L_i, x, v) = AND_i VS(L_i, x, v)
```

### 7.2 Plain term

```text
VS(t relation 0, x, base) =
    t[x := base] relation 0
```

### 7.3 Positive infinitesimal

For equality or disequality:

```text
VS(t = 0, x, base+eps) =
    false                         if sslp(t) != 0
    lim(t)[x := base] = 0         if sslp(t) = 0

VS(t != 0, x, base+eps) =
    true                          if sslp(t) != 0
    lim(t)[x := base] != 0        if sslp(t) = 0
```

For either `relation` in `{>, >=}`:

```text
VS(t relation 0, x, base+eps) =
    lim(t)[x := base] >= 0        if sslp(t) > 0
    lim(t)[x := base] relation 0  if sslp(t) = 0
    lim(t)[x := base] > 0         if sslp(t) < 0
```

### 7.4 Infinity

For `v` with no `Z` component and sign `sigma`:

```text
VS(L, x, v + sigma*infinity) =
    limit_sigma(L)                if L is aperiodic
    VS(L, x, v)                   if L is periodic
```

This is the corrected version of Definition 9. It preserves the residue/base
information needed by periodic literals and replaces aperiodic literals by
their constant tail values.

## 8. Eliminating a `Z` component during substitution

Suppose:

```text
v = base + epsilon_flag*epsilon + p*Z
p > 0
P = periodic literals in phi
A = aperiodic literals in phi
lambda = lcm_Q({p} union
               {period(L) | L in P and period(L) != 0})
```

Any variable-independent periodic literals must already have been simplified.

Construct a finite set `fin(phi, base+p*Z)`. Then:

```text
VS(phi, x, base + epsilon_flag*epsilon + p*Z)
  = OR_{w in fin(phi,base+p*Z)}
      VS(phi, x, w + epsilon_flag*epsilon)
```

Apply the following cases in order.

### V1. An unbounded direction satisfies every aperiodic literal

For each sign `sigma` such that:

```text
for every L in A: limit_sigma(L) = true
```

compute:

```text
R = Grid(base,p) INTERSECT [base, base+lambda)
```

and add:

```text
{s + sigma*infinity | s in R}
```

to `fin`. If both signs qualify, take the union for both. If at least one sign
qualifies, V1 is the selected case.

### V2. An aperiodic equality exists

If V1 did not apply and some `L = (u = 0)` lies in `A`, choose one such
equality and set:

```text
fin =
  Grid(base,p) INTERSECT
    [distXminus(u), distXplus(u)]
```

Any equality is sound. Choosing the narrowest core interval is a deterministic
performance heuristic.

### V3. Bounded from below by one or more aperiodic literals

Otherwise:

```text
fin =
  UNION over L in A with limit_negative(L) = false:
    Grid(base,p) INTERSECT
      [distXminus(L), distXplus(L)+lambda]
```

If neither V1 nor V2 applied, the indexed literal set should be nonempty.
Assert that fact.

## 9. Elimination set for one literal

Write `elim_x(L)` for the virtual lower-bound witnesses of a literal
`L = (t relation 0)`.

### 9.1 No discontinuities

If `breaks(t)` is empty:

```text
if sslp(t) = 0:
    elim(L) = {-infinity}

else if relation is !=:
    elim(L) = {-infinity, zero_t(0)+epsilon}

else if relation is =:
    elim(L) = {zero_t(0)}

else if relation is >= and sslp(t) > 0:
    elim(L) = {zero_t(0)}

else if relation is > and sslp(t) > 0:
    elim(L) = {zero_t(0)+epsilon}

else if relation is > or >= and sslp(t) < 0:
    elim(L) = {-infinity}
```

### 9.2 Discontinuities exist

First construct `Ebreak`.

For a periodic literal:

```text
Ebreak = {
    base + period*Z |
    Grid(base,period) in breaks(t)
}
```

For an aperiodic literal:

```text
Ebreak =
  UNION over Grid(base,period) in breaks(t):
    Grid(base,period) INTERSECT
      (distXminus(t), distXplus(t))
```

Next construct `Ezero`.

For a periodic literal:

```text
Ezero = {
    zero_t(base) + period*Z |
    Grid(base,period) in breaks(t)
}
```

For an aperiodic literal with `oslp(t) = sslp(t)`:

```text
Ezero = {
    zero_t(base) |
    Grid(base,period) in breaks(t)
}
```

For an aperiodic literal with `oslp(t) != sslp(t)`:

```text
Ezero =
  UNION over Grid(base,period) in breaks(t):
    Grid(
      zero_t(base),
      abs((1 - oslp(t)/sslp(t))*period)
    )
    INTERSECT (distXminus(t), distXplus(t))
```

The final branch is used only when `sslp(t) != 0`.

Now construct `Eseg`:

```text
if sslp(t) = 0
   or (sslp(t) < 0 and relation in {>,>=}):
    Eseg = {b+epsilon | b in Ebreak}

else if sslp(t) > 0 and relation is >=:
    Eseg = {b+epsilon | b in Ebreak}
           union Ezero

else if sslp(t) > 0 and relation is >:
    Eseg = {b+epsilon | b in Ebreak}
           union {z+epsilon | z in Ezero}

else if sslp(t) != 0 and relation is !=:
    Eseg = {v+epsilon | v in Ebreak union Ezero}

else if sslp(t) != 0 and relation is =:
    Eseg = Ezero
```

For an aperiodic literal, add core-boundary candidates:

```text
Ebound_plus =
    {distXplus, distXplus+epsilon}
        if limit_positive(L) = true
    {distXplus}
        if limit_positive(L) = false

Ebound_minus =
    {distXminus, -infinity}
        if limit_negative(L) = true
    {distXminus}
        if limit_negative(L) = false
```

Finally:

```text
periodic L:
    elim(L) = Ebreak union Eseg

aperiodic L:
    elim(L) =
      Ebreak union Eseg union Ebound_plus union Ebound_minus
```

Structural duplicates may be removed but need not be for correctness.

## 10. Elimination set for a conjunction

For a non-empty conjunction:

```text
elim_x(AND_i L_i) = UNION_i elim_x(L_i)
```

Keep optional provenance `(candidate, originating literal)` even when the
candidate set is deduplicated. The base disjunctive algorithm needs only the
candidate; conflict-driven search uses its origin.

## 11. Quantifier-elimination theorem

For every non-empty conjunction `phi` of normalized LIRA literals:

```text
exists x. phi
  <=>
OR_{v in elim_x(phi)} VS(phi, x, v)
```

The proof has two directions:

1. Every satisfying value lies in a solution interval of each literal. At least
   one candidate lower bound from a literal is also a lower bound for a
   solution interval of the whole conjunction. Virtual substitution is true at
   that candidate.
2. If virtual substitution succeeds for a plain, epsilon, infinity, or grid
   candidate, the semantics of that candidate constructs an ordinary real
   witness.

Candidate sets and discontinuity sets may over-approximate. Extra disjuncts do
not threaten soundness because virtual substitution must still make the whole
conjunction true.

## 12. Recursive quantifier elimination

A direct symbolic wrapper is:

```text
function qe(formula):
    formula = normalize_arithmetic(formula)
    formula = to_negation_normal_form(formula)

    recursively process innermost quantifiers:
        exists x. body:
            branches = lazy_DNF(body)
            return OR over conjunction branch C:
                if C is false: false
                if x not in C: C
                else:
                    OR over v in elim_x(C):
                        simplify(VS(C,x,v))

        forall x. body:
            return not qe(exists x. not body)

    simplify and share common subexpressions
```

It is not necessary to materialize full DNF. Traverse disjunction branches
lazily and hash-cons shared terms/formulas.

## 13. Runtime invariants worth asserting

- All rationals are reduced and have positive denominators.
- Every `Grid.period` is positive.
- Every grid base, core endpoint, and elimination candidate is free of the
  variable being eliminated.
- `deltaY >= 0`, `deltaX >= 0`, and term periods are nonnegative.
- `zero_t(b)` is called only when `sslp(t) != 0`.
- Every period passed to `lcm_Q` is positive.
- A virtual term never contains both a nonzero `Z` component and infinity.
- A zero `Z` coefficient is canonicalized away.
- No `epsilon`, `Z`, or infinity symbol survives virtual substitution.
- All literals entering the one-conjunction kernel are normalized to the four
  supported relations against zero.
