# Conflict-Driven VIRAS

Base VIRAS eagerly constructs a disjunction over all elimination candidates.
CD-VIRAS instead explores those candidates depth-first, learns formulas that
block conflicting partial assignments, and backtracks.

The VIRAS paper does not restate the complete search calculus. It says to use
the CDVS calculus of Korovin, Košta, and Sturm and replace its `Leaf Conflict`
and `Inner Conflict` learned lemmas. This document combines the two published
descriptions.

## 1. Intended scope

The CDVS source and VIRAS Appendix B describe a satisfiability decision
procedure for a conjunction whose variables are existentially quantified:

```text
exists x1 ... xn. F
```

The result is SAT or UNSAT. Base VIRAS remains the source-backed route for
constructing a symbolic quantifier-free formula with free parameters or
arbitrary quantifier alternation.

Start with basic CD-VIRAS. The enhanced linear-algebra learning rule from the
CDVS paper is an optional later optimization and is not needed for the VIRAS
soundness/completeness argument.

## 2. Flatten `Z` candidates before search

A virtual substitution of `t + p*Z` returns a disjunction. CDVS expects each
search branch to remain a conjunction, so CD-VIRAS replaces the elimination
set by a flattened set.

For every candidate:

```text
base + epsilon_flag*epsilon + p*Z
```

with `p > 0`, compute `fin(F,base+p*Z)` using V1, V2, or V3 and replace the
candidate by:

```text
{w + epsilon_flag*epsilon | w in fin(F,base+p*Z)}
```

Plain, epsilon, and infinity candidates pass through unchanged. No search
candidate contains a `Z` component after flattening.

The flattened set satisfies the same elimination theorem:

```text
OR_{v in elim_flat_x(F)} VS(F,x,v)
  =
OR_{v in elim_x(F)} VS(F,x,v)
```

## 3. Search state

A nonterminal state is:

```text
(F, S, Learned)
```

where:

- `F` is the original conjunction and never changes;
- `S` is a stack of successive virtual assignments;
- `Learned` is a set of learned LIRA formulas.

Terminal states are `SAT` and `UNSAT`.

An ordinary stack entry is:

```text
x_i <- <virtual candidate t_i, originating literal J_i>
```

The origin records which literal's elimination set produced `t_i`. The last
stack entry may instead be:

```text
x <- ?       choose another admissible candidate
x <- bottom  every candidate has been exhausted
```

Write:

```text
F / S
```

for successive, not simultaneous, virtual substitution of the concrete
assignments in `S` from left to right. Apply the same definition to learned
formulas.

A formula is "trivially inconsistent" in the cited calculus when it is ground
and simplifies to false. A production simplifier may recognize more
inconsistency, but any stronger check must be sound.

## 4. Candidate enumeration

For a concrete stack prefix `S` and a variable `x` in `F/S`, compute:

```text
E = elim_flat_x(F/S)
```

while retaining each candidate's originating literal.

A candidate `t` is `Learned`-admissible when:

```text
Learned / (S | x <- t)
```

is not trivially inconsistent.

The enumerator `eterm(F,S,Learned,x)` returns each admissible candidate in a
stable order and eventually returns `bottom`. Its enumeration position belongs
to the search node identified by `(S,x)` and must be restored correctly after
backtracking.

## 5. Base CDVS rules retained by CD-VIRAS

### Decide

```text
(F,S,Learned)
  ->
(F,S | x <- ?,Learned)
```

when `S` has no marker, `F/S` is not trivially inconsistent, and `x` occurs in
`F/S`.

Variable choice is a heuristic. It does not affect correctness.

### Substitute

```text
(F,S | x <- ?,Learned)
  ->
(F,S | x <- eterm(F,S,Learned,x),Learned)
```

The returned item is either a candidate/origin pair or `bottom`.

### Leaf Backtrack

For a stack:

```text
S0 | x_i <- t_i | ... | x_k <- t_k
```

replace the conflicting suffix by:

```text
S0 | x_i <- ?
```

when the learned formulas are already trivially inconsistent after
`S0 | x_i <- t_i`, but are not inconsistent after `S0`.

### Inner Backtrack

```text
(F,S0 | x_{k-1} <- t_{k-1} | x_k <- bottom,Learned)
  ->
(F,S0 | x_{k-1} <- ?,Learned)
```

when `Learned / (S0 | x_{k-1} <- t_{k-1})` is trivially inconsistent.

### Fail

```text
(F,<x_1 <- bottom>,Learned) -> UNSAT
```

### Succeed

```text
(F,S,Learned) -> SAT
```

when `F/S` has no variables and simplifies to true.

Handle a ground initial `F` before entering the state machine.

## 6. VIRAS replacement conflict rules

CDVS learns disjunctions derived from the originating zero constraints. VIRAS
replaces them with the function `lemma_F`, which can block proper virtual terms
containing epsilon or infinity.

### Leaf Conflict

For:

```text
S = <x_1 <- t_1, ..., x_k <- t_k>
```

when `F/S` is trivially inconsistent and `Learned/S` is not, add:

```text
OR_{i=1..k} lemma_F(x_i != t_i)
```

to `Learned`.

### Inner Conflict

For:

```text
S = <x_1 <- t_1, ..., x_{k-1} <- t_{k-1}>
```

in a state ending `S | x_k <- bottom`, when `Learned/S` is not trivially
inconsistent, add:

```text
OR_{i=1..k-1} lemma_F(x_i != t_i)
```

to `Learned`.

An empty disjunction in the first-variable inner conflict is false, leading to
the `Fail` state.

## 7. False intervals for epsilon conflicts

Suppose `VS(F,x,s+epsilon)` is false. A false interval formula describes a
nonempty interval immediately to the right of `s` on which `F` remains false.

### 7.1 Ordering virtual endpoints

Expand the paper's auxiliary relation `precedes` as:

```text
a+epsilon precedes b          <=> a < b
a+epsilon precedes b+epsilon  <=> a < b
a         precedes b          <=> a < b
a         precedes b+epsilon  <=> a <= b
```

After expansion, no epsilon remains in the learned LIRA formula.

### 7.2 Next possible false-to-true points of a literal

For `L = (u relation 0)`, compute `nxt_true_L(s+epsilon)`.

If `breaks(u)` is empty:

```text
sslp(u) = 0:
    empty

relation = Eq:
    {zero_u(0)}

relation = Ne:
    empty

relation = Ge and sslp(u) > 0:
    {zero_u(0)}

relation = Gt and sslp(u) > 0:
    {zero_u(0)+epsilon}

relation in {Gt,Ge} and sslp(u) < 0:
    empty
```

If `breaks(u)` is nonempty:

```text
nextBreak_u(s) = {
    grid_ceil_after(Grid(base,p),s) |
    Grid(base,p) in breaks(u)
}

if sslp(u) = 0:
    nxt_true_L(s+epsilon) = nextBreak_u(s)

if sslp(u) != 0:
    nxt_true_L(s+epsilon) =
        nextBreak_u(s) union curZero_L(s)
```

where:

```text
curZero_L(s) =
    {zero_u(s)+epsilon}
        if relation = Gt and sslp(u) > 0

    {zero_u(s)}
        if (relation = Ge and sslp(u) > 0)
           or relation = Eq

    empty
        if (relation in {Gt,Ge} and sslp(u) < 0)
           or relation = Ne
```

### 7.3 False-interval formula

```text
inFalseInterval_F,s+epsilon(x) =
    s < x
    and
    AND over L in F:
      AND over e in nxt_true_L(s+epsilon):
        ((s+epsilon precedes e) -> (x precedes e))
```

Expand every `precedes` occurrence with the four rules above. The epsilon
blocking lemma is:

```text
lemma_F(x != s+epsilon) =
    not inFalseInterval_F,s+epsilon(x)
```

The formula can and should be simplified. In the paper's worked example the
false interval is `0 < x and x <= 1/2`, producing the blocking lemma
`x <= 0 or 1/2 < x`.

## 8. Infinity conflict lemmas

Let `A` be the aperiodic literals of `F`.

### 8.1 An aperiodic literal is false in the selected tail

For:

```text
v = t + epsilon_flag*epsilon + positive_infinity
```

if some `L in A` has positive-infinity limit false:

```text
lemma_F(x != v) = x <= distXplus(L)
```

For negative infinity, if some `L in A` has negative-infinity limit false:

```text
lemma_F(x != v) = distXminus(L) <= x
```

Any such literal is sound. A deterministic implementation can choose the
tightest structural candidate or the first literal in stable order.

### 8.2 Only periodic residue information causes the conflict

If every aperiodic literal is true in the selected tail, a false virtual
substitution must come from a variable-dependent periodic literal.

Let `lambda > 0` be a common period of all such periodic literals:

```text
lambda =
  lcm_Q({period(L) |
         L periodic, period(L) != 0})
```

If the finite base has no epsilon:

```text
lemma_F(x != t +/- infinity) =
    rem_lambda(x) != rem_lambda(t)
```

If it has epsilon:

```text
shifted_base(x) =
    t + lambda*(quot_lambda(x) - quot_lambda(t))

lemma_F(x != t+epsilon +/- infinity) =
    not inFalseInterval_F,shifted_base(x)+epsilon(x)
```

The latter is a LIRA formula even though `shifted_base` contains `x`; quotient
is represented using floor.

The paper does not bind `lambda` in Definition 12. The common-period
construction above is the proof-required conservative resolution; see
[sources-and-errata.md](sources-and-errata.md).

## 9. Lemma correctness contract

For every conjunction `F` and virtual term `v` without a `Z` component, the
paper proves:

```text
if VS(F,x,v) is false:
    for all x:
        F(x) -> lemma_F(x != v)

VS(lemma_F(x != v), x, v) is false
```

The first property makes learning sound: a learned lemma cannot exclude a real
solution of `F`. The second makes progress: the current virtual assignment is
deactivated.

These two properties should be executable property tests for every lemma
branch before enabling CD-VIRAS in proof search.

## 10. Termination and search invariants

The CDVS termination proof labels the finite candidate search tree with active
and inactive nodes. A learned lemma makes at least the current node inactive,
and the lexicographic state measure decreases on every rule.

Operational invariants:

- `S` contains distinct assigned variables in assignment order.
- Only the final stack entry may be `?` or `bottom`.
- Every concrete candidate came from `elim_flat_x(F/S_prefix)`.
- Every concrete candidate is free of its assigned variable.
- `F/S` and `Learned/S` use successive substitution.
- A candidate is returned only if it is learned-admissible.
- Conflict learning strictly deactivates the current assignment.
- The candidate set at every search node is finite.

## 11. Important notation gap to resolve during implementation

The published conflict rules write `lemma_F(x_i != t_i)` for every stack entry,
while candidates after the first are constructed from successively substituted
residual formulas. The appendix proves the necessary invariant only as a short
induction.

A conservative implementation should retain, with every stack entry:

- the original formula `F`;
- the exact prefix at which the candidate was generated;
- the residual conjunction `F/S_prefix`;
- the originating literal;
- the generated blocking lemma before it is lifted into the learned formula.

Then validate the two lemma correctness properties and the paper's global
invariant at each rule in debug/property tests. Do not discard this context
until the notation has been formalized in code and the invariant tests pass.

This is the largest remaining specification risk in CD-VIRAS. It does not
affect the eager base VIRAS algorithm.

## 12. Minimal CD-VIRAS pseudocode

```text
function cd_viras(F):
    F = normalized existential conjunction
    if F is ground:
        return evaluate(F)

    state = (F, empty_stack, empty_learned_set)

    loop:
        match the unique applicable rule:
            Succeed:
                return SAT with the concrete virtual stack

            Fail:
                return UNSAT

            Decide:
                choose variable and push ?

            Substitute:
                replace ? by next learned-admissible flattened candidate
                or bottom

            Leaf Conflict:
                learn disjunction of VIRAS blocking lemmas

            Leaf Backtrack:
                pop conflicting suffix and retry its first assignment

            Inner Conflict:
                learn disjunction for the exhausted prefix

            Inner Backtrack:
                pop exhausted suffix and retry the previous assignment
```

Log the rule, stack, chosen candidate, candidate origin, learned lemma, and
simplified residual formula. Those traces are essential for validating the
calculus against the paper's derivations.
