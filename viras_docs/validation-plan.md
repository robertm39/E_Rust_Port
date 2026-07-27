# VIRAS Validation Plan and Test Vectors

The method has many locally simple recurrences whose interactions are easy to
miscode. Validation should be layered: exact numeric helpers, term profiles,
grids, elimination sets, virtual substitution, one-quantifier equivalence, and
finally recursive QE/CD-VIRAS.

This is a test specification, not an implementation task list. When Rust work
begins, all Rust builds and tests must run through the repository's required
ephemeral Linode workflow.

## 1. Exact rational helpers

Minimum deterministic vectors:

```text
lcm_Q({1/3, 1/2})       = 1
lcm_Q({2/3, 4/5})       = 4
lcm_Q({3/10, 9/14})     = 9/2

quot_3(7)                = 2
rem_3(7)                 = 1
quot_3(-1)               = -1
rem_3(-1)                = 2

quot_(2/3)(-1/2)         = -1
rem_(2/3)(-1/2)          = 1/6
```

For generated positive rationals `p` and rationals `x`, assert:

```text
x = p*quot_p(x) + rem_p(x)
0 <= rem_p(x) < p
quot_p(x) has denominator 1
```

Include large coprime numerators/denominators to expose accidental
machine-integer overflow.

## 2. Grid rounding and intersection

For `G = 1 + 2*Z`:

```text
ceil_G(a)       = a + rem_2(1-a)
floor_G(a)      = a - rem_2(a-1)
ceil_G(a+eps)   = floor_G(a+2)
floor_G(a-eps)  = ceil_G(a-2)
```

The paper's example must produce:

```text
G INTERSECT [a,a+4)
  = {
      a + rem_2(1-a),
      a + rem_2(1-a) + 2
    }
```

After expanding remainder, these are equivalent to:

```text
1 - 2*floor((1-a)/2)
3 - 2*floor((1-a)/2)
```

Boundary matrix:

| Grid/interval | Expected enumeration shape |
| --- | --- |
| `Z INTERSECT [a,a+1)` | one candidate |
| `Z INTERSECT (a,a+1)` | one covering candidate, possibly an excluded endpoint after parameter instantiation |
| `Z INTERSECT [a,a]` | one safe covering candidate |
| `Z INTERSECT (a,a)` | empty |
| `(1/2+2Z) INTERSECT [a,a+4]` | three covering candidates |

For random concrete rational substitutions of parameters, assert that every
actual grid point in the interval occurs among the evaluated generated terms.
Do not assert equality of sets: the construction intentionally permits extras.

## 3. Term-profile golden vectors

All profiles below are relative to `x`; `z` and `c` are parameters.

| Term `t` | `oslp` | `sslp` | `per` | `deltaY` | `distYminus` | `lim(t)` | `breaks(t)` |
| --- | ---: | ---: | ---: | ---: | --- | --- | --- |
| `x` | `1` | `1` | `0` | `0` | `0` | `x` | empty |
| parameter `z` | `0` | `0` | `0` | `0` | `z` | `z` | empty |
| `floor(3*x)` | `3` | `0` | `1/3` | `1` | `-1` | `floor(3*x)` | `{(1/3)*Z}` |
| `-floor(-3*x+z)-x` | `2` | `-1` | `1/3` | `1` | `-z` | `floor(3*x-z)+1-x` | `{z/3+(1/3)*Z}` |
| `ceil(x)-x-c` | `0` | `-1` | `1` | `1` | `-c` | `floor(x)+1-x-c` | `{Z}` |

For `-floor(-3*x+z)-x`, also assert:

```text
distYplus = -z + 1
core = [(z-1)/2, z/2]
negative-infinity limit of (t > 0) = false
positive-infinity limit of (t > 0) = true
```

For `ceil(x)-x-c`, assert that the literal is periodic because its outer slope
is zero even though its segment slope is `-1`.

### 3.1 Structural property tests

For generated LIRA terms and rational parameter assignments:

**Linear bounds**

```text
oslp*x + distYminus <= value(t) <= oslp*x + distYplus
```

at many rational `x` values, including negative values and exact floor
boundaries.

**Periodic shift**

When `per != 0`, for generated rational `x,y`:

```text
t(x + per*floor(y))
  = t(x) + oslp*per*floor(y)
```

**Right limit**

For a concrete point `b`, select a sufficiently small positive rational `d`
that crosses no computed covering break and compare:

```text
t(b+d) = lim(t)(b+d)
```

and, on the segment:

```text
t(b+d) = sslp*(b+d) + dseg(b)
```

**Break coverage**

For a bounded rational window and concrete parameter values, independently
find every point where a floor argument crosses an integer. Assert that every
actual discontinuity belongs to at least one evaluated break grid.

Coincident discontinuities and cancellations must not invalidate coverage;
extra break grids are acceptable.

## 4. Elimination-set branch vectors

### 4.1 Linear, no-break cases

```text
elim(x >= 0)       = {0}
elim(x > 0)        = {0+epsilon}
elim(-x >= 0)      = {-infinity}
elim(-x > 0)       = {-infinity}
elim(x = 0)        = {0}
elim(x != 0)       = {-infinity, 0+epsilon}
```

For an `x`-independent literal with empty breaks and zero segment slope, the
paper's raw per-literal result is `{-infinity}`. The normalization wrapper
should instead factor that literal outside the `x`-dependent kernel.

### 4.2 Periodic literal

For:

```text
L = ceil(x)-x-c >= 0
```

the expected sets are:

```text
Ebreak = {Z}
Eseg   = {Z+epsilon}
elim(L)= {Z, Z+epsilon}
```

This exercises `oslp = 0`, `sslp < 0`, and a nonempty break set.

### 4.3 Safe over-approximation

For:

```text
L = floor(3*x) >= 0
```

the exact solution lower bound is `0`, but the paper's open-core grid
intersection can conservatively add the upper endpoint. A structurally faithful
implementation may produce:

```text
{0, 1/3, 1/3+epsilon}
```

All candidates are sound because the final virtual substitutions are checked.
This vector prevents an incorrect test assumption that elimination sets must be
minimal.

### 4.4 Every Figure 2 branch

Construct a table-driven test covering every combination actually used by
Figure 2:

- empty/nonempty breaks;
- periodic/aperiodic;
- segment slope negative, zero, positive;
- `Eq`, `Ne`, `Gt`, `Ge`;
- both infinity-limit truth values;
- `oslp = sslp` and `oslp != sslp`.

For each row, check candidate kinds and exact terms separately. This is more
diagnostic than checking only the final union.

## 5. Epsilon virtual-substitution vectors

At base zero:

```text
VS(x = 0,      x, epsilon) = false
VS(x != 0,     x, epsilon) = true
VS(x >= 0,     x, epsilon) = true
VS(x > 0,      x, epsilon) = true
VS(-x >= 0,    x, epsilon) = false
VS(-x > 0,     x, epsilon) = false
VS(floor(x)=0, x, epsilon) = true
```

Also test parameterized bases:

```text
VS(x-a > 0,  x, a+epsilon) = true
VS(a-x >= 0, x, a+epsilon) = false
```

For a zero segment slope, verify that the original `Gt` versus `Ge` relation is
preserved on the right-limit term.

No output formula may contain epsilon.

## 6. Infinity virtual-substitution vectors

For aperiodic literals:

```text
VS(x >= 0, x, +infinity) = true
VS(x >= 0, x, -infinity) = false
VS(x = 0,  x, +/-infinity) = false
VS(x != 0, x, +/-infinity) = true
```

For a periodic literal `P`, compare:

```text
VS(P,x,t+infinity) = VS(P,x,t)
VS(P,x,t-infinity) = VS(P,x,t)
```

This test specifically catches the reversed periodic/aperiodic conditions
printed in Definition 9.

No output formula may contain infinity.

## 7. `Z`-flattening vectors

### 7.1 V1: a satisfying unbounded tail

Use:

```text
F = (x >= 0) and
    (rem_2(x) = rem_2(c))

candidate = c + 2*Z
```

`x >= 0` is aperiodic and true at positive infinity; the congruence literal is
periodic with period `2`. V1 uses `lambda = 2`, one representative of the grid
in `[c,c+2)`, and decorates it with positive infinity.

Verify that periodic substitution retains base `c` while the lower bound
becomes true.

### 7.2 V2: equality confines the grid

Use:

```text
F = (x = a) and
    (rem_2(x) = rem_2(c))

candidate = c + 2*Z
```

V1 cannot apply because equality is false at both infinities. V2 intersects the
grid with the equality's closed, possibly zero-width core. The resulting
formula must express whether `a` belongs to the residue class of `c`.

This exercises the `k = 0` closed grid-intersection extension.

### 7.3 V3: paper example

Use the extended paper's Example 15:

```text
t = -floor(-3*x+z)-x

F = (t > 0)
    and (x < 0)
    and (rem_3(x) = rem_3(c))
    and (rem_2(x) != 1)

candidate = c + 3*Z
```

The periodic common period is `lambda = 6`. V3 covers:

```text
(c + 3*Z) INTERSECT [(z-1)/2, z/2+6]
```

and produces three symbolic representatives under the paper's covering
intersection. Check both the number and the symbolic start/step structure.

No output formula may contain `Z`.

## 8. End-to-end base VIRAS examples

### 8.1 Motivating example

Use the original ceiling literal, not the typo in Example 10:

```text
exists x.
    floor(a)+1/3 <= x
    and x <= floor(a)+2/3
    and ceil(x)-x >= c
```

The per-literal union contains:

```text
floor(a)+1/3
-infinity
Z
Z+epsilon
```

The first candidate is positive `floor(a)+1/3`; the paper later prints its
negation by mistake.

After virtual substitution and simplification, the exact result is:

```text
c <= 2/3
```

Check equivalence for:

- integer and non-integer values of `a`;
- `c` below, equal to, and above `2/3`;
- negative values of both parameters.

### 8.2 Integer/non-integer validity example

Check the paper's universally quantified formula:

```text
forall x,z.
    (ceil(x+z) > floor(x+z)
     and ceil(z) = floor(z))
    ->
    floor(x) != x
```

Normalize it through negated existentials, eliminate `z`, then eliminate `x`.
The result must simplify to true.

This exercises:

- two quantifiers;
- negation normalization;
- equality and strict inequality;
- periodic `Z` and `Z+epsilon` candidates;
- branch simplification between eliminations.

### 8.3 Pure LRA regression

Because floor-free LRA is a subset:

```text
exists x. (a < x and x <= b)
```

must reduce to:

```text
a < b
```

Include all strict/non-strict endpoint combinations and equal endpoints.

### 8.4 Encoded integer quantifier

After adding the typed-integer adapter, validate:

```text
exists z:Int. (a < z and z < a+1)
```

against:

```text
exists z:Real.
    z = floor(z)
    and a < z
    and z < a+1
```

The result characterizes whether the open interval contains an integer. Check
negative `a`, where truncation-toward-zero mistakes are common.

## 9. Independent semantic oracle

For small formulas, do not use VIRAS's own candidate generator as the only
oracle.

### 9.1 Concrete cell decomposition

After assigning rational values to all parameters:

1. enumerate actual floor discontinuities in a bounded window;
2. add actual zeros of affine segments;
3. evaluate every point cell, open interval cell, and the two tails;
4. decide the existential formula exactly with rational arithmetic.

For aperiodic terms, the computed core interval supplies a safe finite window;
verify tails separately from their slope signs. For periodic-only formulas,
check one concrete common period.

This oracle is intentionally simpler and concrete, even if slower.

### 9.2 Differential solver

Translate closed test formulas to a separately trusted arithmetic solver that
supports reals, integers, and floor through `to_int`-style encodings. Compare
SAT/UNSAT results on generated small formulas.

The repository already carries a separately licensed Z3 source tree, but the
test harness must validate the exact negative-floor semantics used by its
translation before treating it as an oracle.

### 9.3 Random formula generation

Generate bounded-size terms with:

- rational coefficients of both signs;
- negative and positive parameters;
- nested floors;
- additions that cancel outer or segment slopes;
- shared subterms;
- coincident and distinct break grids.

Keep formulas small enough for the independent oracle. Compare:

```text
exists x. F
```

with the evaluated VIRAS result under many rational parameter assignments.

## 10. Metamorphic properties

The result should remain equivalent under:

- `ceil(t)` versus `-floor(-t)`;
- associativity/commutativity of addition;
- insertion/removal of `+0` and scaling by one;
- positive rational rescaling of both sides of a normalized literal;
- relation-preserving normalization of `<` and `<=`;
- alpha-renaming the eliminated variable;
- reordering conjunction literals;
- adding a duplicate literal;
- adding an `x`-independent true literal.

Candidate sets may differ structurally after these transformations; semantic
equivalence is the assertion.

## 11. Edge-case corpus

Include dedicated cases for:

- negative floor inputs and exact negative integers;
- zero segment slope with nonzero outer slope;
- zero outer slope with nonzero segment slope;
- both slopes zero;
- `oslp = sslp` and `oslp != sslp`;
- a zero-width core interval;
- grids whose normalized period was originally negative;
- an empty periodic-literal set in V1;
- both infinity signs qualifying in V1;
- several aperiodic equalities in V2;
- lower and upper aperiodic bounds forcing V3;
- parameter values that make two symbolic break grids coincide;
- cancellation that makes an over-approximated break non-actual;
- very large rational LCMs;
- an empty conjunction and an empty disjunction;
- a quantified variable absent from the body;
- formulas that simplify to false before candidate generation;
- resource-limit exits, which must return unknown rather than false.

## 12. CD-VIRAS validation

Before full search, test the blocking-lemma contract independently for every
virtual-term shape:

```text
not VS(F,x,v)
  -> for all x. (F -> lemma_F(x != v))

not VS(lemma_F(x != v),x,v)
```

Specific vectors:

- plain conflict learns `x != t`;
- epsilon conflict for
  `x > 0 and x > 1 and 7 > x` at `epsilon` learns an interval-excluding
  formula such as `x <= 0 or 1 <= x`;
- the paper's two-literal epsilon example at `epsilon` simplifies to
  `x <= 0 or 1/2 < x`;
- positive/negative infinity conflicts caused by an aperiodic limit;
- infinity conflict caused only by a periodic residue class;
- epsilon-plus-infinity periodic conflict;
- exhaustion of the first variable reaches UNSAT;
- a satisfiable branch terminates early without enumerating every candidate.

For the state machine, assert after every transition:

- exactly one rule is applicable;
- the stack-marker invariant holds;
- every learned lemma is implied by `F`;
- the just-rejected candidate makes the new learned disjunction false;
- the active search node count or an executable equivalent decreases as
  required by the termination proof.

Retain complete traces for any failing generated test.
