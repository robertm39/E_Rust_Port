# VIRAS Implementation Blueprint

This document turns the mathematical specification into a proposed software
design. Statements marked **Paper** are direct consequences of the published
calculus. Statements marked **Reconstruction** fill routine executable detail.
Statements marked **Recommendation** are engineering choices for this Rust
project.

No code is implemented here.

## 1. Architectural boundary

**Recommendation:** Build VIRAS as an isolated exact-arithmetic kernel with a
small adapter to the prover's general term and formula representation.

The existing Rust tree has mature generic terms, formulas, parsing, and
printing, but it currently has no exact LIRA reasoning layer or arbitrary
precision rational arithmetic dependency. VIRAS-specific assumptions should
therefore not be implicit in general-purpose term cells.

The kernel boundary should expose operations equivalent to:

```text
import_lira_formula(general_formula) -> Result<LiraFormula, Unsupported>
eliminate_exists(variable, conjunction) -> LiraFormula
quantifier_eliminate(formula) -> LiraFormula
export_lira_formula(lira_formula, general_term_bank) -> GeneralFormula
```

The importer is responsible for proving that the input lies in the supported
linear fragment. The kernel may then rely on strong invariants.

## 2. Exact numeric backend

**Paper:** Slopes, periods, coefficients, core widths, and grid widths are
rational. Floor is semantic floor, including for negative values. Period
composition uses exact numerator/denominator GCD and LCM operations.

**Recommendation:** Use arbitrary-precision integers and canonical rational
numbers from the first implementation. Machine integers plus overflow checks
are not a robust substitute: nested periods and repeated `lcm_Q` operations can
grow quickly even when the input numerals are small.

Required numeric operations:

- normalized construction with positive denominator;
- addition, subtraction, multiplication, division, sign, and absolute value;
- numerator and denominator access;
- integer GCD and LCM;
- exact rational floor and ceiling;
- comparison and hashing;
- conversion to an integer only after proving integrality.

Never use binary floating point in term profiling, grid enumeration, formula
simplification, or tests.

## 3. Kernel data model

### 3.1 Arithmetic terms

**Recommendation:**

```text
LiraTerm =
    Var(VariableId)
  | Rational(Rational)
  | Add(TermId, TermId)
  | Scale(Rational, TermId)
  | Floor(TermId)
```

The paper uses `1` rather than a rational leaf, but a canonical rational leaf is
an equivalent and more compact internal representation.

Hash-cons terms and perform local canonicalization:

- flatten additions;
- combine rational constants;
- combine like scalar wrappers;
- eliminate `+0`, `1*t`, and `0*t`;
- normalize `-floor(-t)` only when an explicit ceiling must be imported;
- preserve floor boundaries;
- sort commutative addition children by stable structural key.

Avoid expansions that duplicate large terms. Symbolic bounds and candidate
terms share much of the input DAG.

### 3.2 Literals and formulas

```text
Relation = Eq | Ne | Gt | Ge
Literal  = { lhs: TermId, relation: Relation }   // rhs is zero

Formula =
    True
  | False
  | Lit(Literal)
  | And(shared children)
  | Or(shared children)
  | Not(child)                // removed before the kernel
  | Exists(variable, child)
  | Forall(variable, child)
```

Within the one-conjunction kernel, use a compact vector of literals. Outside
it, keep formulas as DAGs and flatten nested `And`/`Or` nodes.

### 3.3 Virtual terms and grids

Use distinct types:

```text
Grid {
    base: TermId,             // free of current x
    period: PositiveRational
}

InfinitySign = Negative | Positive

VirtualTerm {
    base: TermId,             // free of current x
    epsilon: bool,
    grid_period: Option<PositiveRational>,
    infinity: Option<InfinitySign>
}
```

Constructor invariants:

- grid and infinity are mutually exclusive;
- a zero grid period becomes `None`;
- a negative grid period is made positive;
- the base is checked to be free of the elimination variable;
- `epsilon` is at most one because only a right-neighborhood marker is needed.

Do not represent `epsilon`, `Z`, or infinity as general first-order function
symbols. Doing so would make it possible to leak them into output formulas.

### 3.4 Term profile

```text
TermProfile {
    outer_slope: Rational,
    segment_slope: Rational,
    period: NonnegativeRational,
    delta_y: NonnegativeRational,
    dist_y_minus: TermId,
    right_limit: TermId,
    breaks: shared set<Grid>
}
```

Derive `dist_y_plus`, core endpoints, `delta_x`, segment distance, and segment
zero through methods. Cache the profile by `(TermId, VariableId)`.

`breaks` is the expensive field. A staged cache can compute slopes/bounds first
and discontinuities only if the caller needs them.

## 4. Import and normalization pipeline

### 4.1 Fragment recognition

The importer should accept only expressions that can be proven equivalent to
the LIRA grammar:

- rational numerals;
- real variables;
- addition and subtraction;
- multiplication/division by a rational constant;
- floor and ceiling;
- supported comparisons and Boolean connectives;
- quantifiers over a supported arithmetic domain.

Reject or route elsewhere:

- variable-by-variable multiplication;
- division by a non-constant;
- powers other than zero or one;
- non-linear casts;
- arithmetic-valued uninterpreted functions depending on the elimination
  variable;
- transcendental or partial operations.

An `x`-independent foreign arithmetic term could be treated as a parameter only
if the surrounding solver gives it the same ordered-real semantics expected by
LIRA. The paper does not provide such a theory-combination result, so the first
implementation should reject it.

### 4.2 Typed integer adapter

**Reconstruction:** The paper has a single real domain. If the frontend has an
integer sort, lower integer quantifiers into real quantifiers with integrality
guards:

```text
exists z:Int. phi
  -> exists z:Real. (z = floor(z) and phi)

forall z:Int. phi
  -> forall z:Real. (z = floor(z) -> phi)
```

Rational and integer constants embed exactly into reals. Cast normalization
must be completed before fragment recognition. Preserve the distinction
between mathematical floor and truncation toward zero for negative inputs.

### 4.3 Boolean normalization

**Paper:** VIRAS needs an existential quantifier over a conjunction of literals;
existentials distribute over disjunction and universals are expressed through
negated existentials.

**Recommendation:** Use lazy NNF/DNF traversal:

```text
normalize(atom) -> normalized literal
normalize(not atom) -> normalized complementary literal
branches(A or B) -> branches(A) followed by branches(B)
branches(A and B) -> lazy Cartesian product
```

Apply simplification during the Cartesian product. Do not build a complete DNF
tree before elimination.

### 4.4 Elimination-variable-independent literals

Before profiling:

1. separate literals that do not contain `x`;
2. simplify any that are ground;
3. if an `x`-independent literal is false, return false for the branch;
4. retain the remaining parameter condition as a conjunct outside the
   elimination result;
5. run VIRAS only on the nonempty `x`-dependent conjunction.

This prevents zero-period constants from entering period LCMs and reduces every
candidate disjunct.

## 5. Profile computation

**Paper:** All profile values are structural recurrences.

**Recommendation:** Implement one memoized visitor returning the cheap profile
fields. Compute `breaks` through a second memoized visitor because it recursively
uses profiles, grid intersection, `right_limit`, and `zero`.

Conceptual shape:

```text
function cheap_profile(term,x):
    if cached: return cached

    match term:
        variable / rational:
            use base cases

        scale(k,u):
            p = cheap_profile(u,x)
            transform slopes, bounds, width, limit, period

        add(u,v):
            pu = cheap_profile(u,x)
            pv = cheap_profile(v,x)
            combine fields

        floor(u):
            p = cheap_profile(u,x)
            preserve outer slope
            set segment slope to zero
            widen symbolic bounds
            compute floor period
            compute right-limit floor case

    assert invariants
    cache and return
```

For addition, use the paper's exact period rule rather than multiplying periods.
For floor, use the argument's segment slope in the right-limit rule and the
argument's outer slope/period in the period rule.

### 5.1 Symbolic substitution in profile helpers

`right_limit(t)[x := b]`, `dseg_t(b)`, and `zero_t(b)` occur frequently.
Provide a capture-avoiding term substitution that:

- operates on the LIRA DAG;
- memoizes by `(TermId, x, replacement TermId)`;
- simplifies as it rebuilds;
- checks that `b` is `x`-free.

## 6. Grid operations

Implement grid rounding and finite grid intersection as a dedicated module.

```text
grid_ceil(Grid(s,p), t)
grid_floor(Grid(s,p), t)
grid_ceil_after(Grid(s,p), t)       // ceil(t+epsilon)
grid_floor_before(Grid(s,p), t)     // floor(t-epsilon)
grid_intersection(grid, lower_kind, lower, width, upper_kind)
```

`grid_intersection` enumerates rational `n` indices, not parameter values. Each
output is a symbolic term `start + n*p`.

Do not test whether an output symbolically lies inside the interval. The paper
needs only a covering set and the symbolic comparison may itself require
quantifier elimination. Optional exact pruning is safe only when a separate
decision procedure proves it.

Deduplicate grids structurally after positive-period normalization. Do not try
to decide semantic equality of parameterized grid bases in the first version.

## 7. Discontinuity construction

Use the recurrence in
[mathematical-specification.md](mathematical-specification.md#4-discontinuity-grids).

Important implementation details:

- The `floor(u)` recurrence asks for `breaks(u)`, not `breaks(floor(u))`, while
  constructing `breaksInSeg(u)`.
- `p_min` is the minimum stored positive period in `breaks(u)`.
- The outer descriptor period emitted by `breaksInSeg(u)` is
  `per(floor(u))`.
- Normalize `1/sslp(u)` to its absolute value.
- A descriptor denotes a covering grid. It is acceptable for two descriptors
  to overlap or for a descriptor to contain non-discontinuities.

Use resource counters for:

- number of descriptors per term;
- number of generated representatives per grid intersection;
- term-DAG nodes created;
- rational numerator/denominator bit lengths.

Those counters support safe limits and later performance work.

## 8. Literal elimination candidates

Implement the Figure 2 case split as a total function over:

```text
(periodic?, breaks_empty?, segment_slope_sign, relation,
 positive_limit?, negative_limit?, outer_equals_segment?)
```

Avoid a deeply nested ad hoc conditional. A small decision table makes it
possible to test every branch independently.

Recommended internal result:

```text
Candidate {
    virtual_term: VirtualTerm,
    origin_literal: LiteralId,
    origin_kind:
        LinearZero
      | Discontinuity
      | SegmentZero
      | CoreLower
      | CoreUpper
      | NegativeTail
}
```

Origin metadata is not part of the logic. It is useful for debugging, proof
objects, deterministic ordering, and CD-VIRAS.

Candidate ordering affects only performance. A reasonable initial order is:

1. plain terms;
2. epsilon terms;
3. grid terms;
4. infinities.

Within each class, use stable literal order and structural term order.

## 9. Virtual substitution engine

Use a closed dispatch on the virtual-term shape:

```text
function virtual_substitute(phi,x,v):
    assert x not in v.base

    if v has grid:
        finite = flatten_grid_candidate(phi,x,v)
        return OR(virtual_substitute(phi,x,w) for w in finite)

    if v has infinity:
        return AND(
            infinity_substitute_literal(L,x,v)
            for L in phi
        )

    if v has epsilon:
        return AND(
            epsilon_substitute_literal(L,x,v.base)
            for L in phi
        )

    return ordinary_substitute(phi,x,v.base)
```

For a term with both epsilon and infinity, infinity dispatches first:
aperiodic literals become tail constants, while periodic literals recurse on
the epsilon-bearing finite base.

### 9.1 `Z` flattening cases

Compute the `A`/`P` literal partition once per `(conjunction,x)`. Compute
positive and negative limit truth vectors once.

V1:

- loop explicitly over both infinity signs;
- use the common period of the grid and all variable-dependent periodic
  literals;
- add an infinity candidate for every representative and qualifying sign.

V2:

- select one aperiodic equality deterministically;
- grid-intersect its closed core interval.

V3:

- generate one finite set per aperiodic literal false at negative infinity;
- union and structurally deduplicate.

The result of flattening contains no `Z` component, but may contain epsilon
and/or infinity.

### 9.2 Simplification after every substitution

Minimum simplifier rules:

- exact ground rational comparisons;
- `and false`, `and true`, `or false`, `or true`;
- duplicate conjunct/disjunct removal;
- complementary literal detection;
- normalized comparison direction;
- rational coefficient collection;
- `floor(q)` for rational `q`;
- `floor(t+n) = floor(t)+n` for provably integer rational `n`;
- `rem_p`/`quot_p` construction through ordinary floor terms, followed by
  local arithmetic simplification.

The method is correct without sophisticated simplification, but formula growth
can make it unusable.

## 10. Quantifier wrapper and formula growth

Eliminate innermost quantifiers first. For:

```text
exists x. (C1 or C2 or ...)
```

process each conjunction branch independently. Share:

- input subterms;
- term profiles keyed by variable;
- repeated elimination candidates;
- repeated virtual substitutions;
- repeated simplified atoms.

**Paper:** The authors show VIRAS avoids an exponential preprocessing
normalization used by the earlier mixed real-integer procedure for a family
containing sums of floors.

That result does not imply that arbitrary Boolean normalization is cheap.
Retain branch laziness and stop a branch immediately when simplification proves
it true or false.

## 11. Correctness-oriented API layering

A useful separation is:

```text
profiles:
    slopes_period(term,x)
    bounds(term,x)
    right_limit(term,x)
    breaks(term,x)

geometry:
    grid_round(...)
    grid_intersect(...)
    core_interval(literal,x)
    segment_zero(term,x,at)

candidates:
    literal_elim(literal,x)
    conjunction_elim(conjunction,x)
    flatten_z(conjunction,x,virtual_term)

substitution:
    ordinary(...)
    epsilon(...)
    infinity(...)
    virtual(...)

qe:
    eliminate_conjunction(...)
    eliminate_quantifier(...)
    eliminate_all(...)
```

Each layer can be validated against a separate theorem from the paper.

## 12. Proof and explanation data

VIRAS is well-suited to proof-producing execution because each generated
candidate has a local justification:

- a literal and Figure 2 case;
- a break-grid descriptor;
- a segment zero;
- a core boundary or infinity limit;
- a V1/V2/V3 grid-flattening case;
- a virtual-substitution rule.

**Recommendation:** Preserve this derivation metadata even if the first
implementation only logs it. A later proof object can show:

1. why the elimination set covers every solution interval lower bound;
2. which virtual-substitution rule removed each auxiliary marker;
3. which simplified disjunct provided a witness.

This is valuable for theorem-prover trust and debugging independently of
whether a formal external certificate format is immediately available.

## 13. Resource limits and failure behavior

The calculus is a decision procedure, but intermediate term and candidate sets
can be large.

If the production prover imposes limits, return an explicit
`Unknown(ResourceLimit)` rather than treating an incomplete candidate set as
unsatisfiable. Safe limit points include:

- rational bit length;
- number of break grids;
- number of candidates;
- formula DAG nodes;
- recursive quantifier branches;
- elapsed inference budget.

Never silently truncate `breaks`, `Ezero`, V3 unions, or candidate disjunctions.

## 14. Recommended implementation sequence

The lowest-risk sequence is:

1. exact rationals and the isolated LIRA AST;
2. normalization and an exact evaluator for ground/parameter-instantiated
   terms;
3. slopes, periods, bounds, right limits, and their unit properties;
4. grids and discontinuity construction;
5. per-literal elimination sets;
6. epsilon and infinity substitution;
7. `Z` flattening;
8. one-conjunction existential elimination;
9. Boolean/quantifier wrapper;
10. typed-integer frontend adapter;
11. CD-VIRAS as a separate search mode.

Durable implementation work and discovered follow-ups should be tracked in
Beads rather than by adding task lists to these research documents.
