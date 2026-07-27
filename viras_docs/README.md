# VIRAS Clean-Room Implementation Research

This directory is a research packet for implementing Virtual Integer-Real
Arithmetic Substitution (VIRAS) without using the unlicensed implementation
published on GitHub. It contains no VIRAS implementation.

The short conclusion is:

- The base VIRAS quantifier-elimination procedure is described precisely enough
  to implement from the authors' extended paper.
- The conflict-driven CD-VIRAS layer is also implementable as a conservative
  first version when combined with the referenced CDVS calculus.
- The papers contain several typographical errors and a few underspecified edge
  cases. They are isolated in
  [sources-and-errata.md](sources-and-errata.md), with corrections or safe
  implementation choices justified from the surrounding definitions and
  proofs.
- Exact rational arithmetic, symbolic term canonicalization, and aggressive
  simplification are correctness and scalability requirements, not optional
  polish.

## Clean-room boundary

This research was prepared from published papers and repository-local
architecture inspection. The unlicensed VIRAS GitHub source, its file layout,
and its implementation details were not inspected or used.

The primary description is the authors' 57-page extended preprint:

> Johannes Schoisswohl, Laura Kovács, and Konstantin Korovin, "VIRAS:
> Conflict-Driven Quantifier Elimination for Integer-Real Arithmetic (Extended
> Version)," EasyChair Preprint 13150, version dated May 7, 2024.

The conflict-driven state machine comes from the independently published CDVS
paper that VIRAS incorporates by reference. Full citations, stable links,
provenance, and the paper's publication license are recorded in
[sources-and-errata.md](sources-and-errata.md).

This is technical provenance, not legal advice.

## What VIRAS decides

The paper's LIRA language is the first-order theory over the real numbers with:

- rational constants and rational scalar multiplication;
- addition;
- order and equality;
- the floor function.

All arithmetic variables range over the reals. Integer behavior is represented
through floor. The language permits nested floors but not products of two
non-constant terms. It does not directly include uninterpreted functions,
transcendentals, or non-linear arithmetic.

VIRAS eliminates one existential quantifier from a conjunction of normalized
LIRA literals by finding a finite set of symbolic lower-bound witnesses. The
witness language contains three temporary devices:

- `t + epsilon`, representing values immediately to the right of `t`;
- `t +/- infinity`, representing the appropriate unbounded tail while
  preserving periodic residue information;
- `t + p*Z`, representing the rational grid `{t + p*z | z in integers}`.

Virtual substitution eliminates all three devices immediately, so none occurs
in the output formula.

Arbitrary Boolean formulas and quantifier alternations are handled outside this
one-conjunction kernel: push negations inward, rewrite universal quantifiers
through negated existentials, distribute an existential over disjunctions
(preferably lazily), and eliminate quantifiers from the inside out.

## Core idea in one pass

For an elimination variable `x`:

1. Normalize every literal to `t relation 0`, where `relation` is one of
   equality, disequality, strict-greater, or greater-or-equal.
2. Analyze `t` recursively to compute:
   - its long-run or outer slope;
   - its slope between discontinuities;
   - a rational period;
   - symbolic linear bounds;
   - a right-limit term;
   - a finite set of periodic grids covering all discontinuities.
3. Split literals into periodic ones (outer slope zero) and aperiodic ones
   (outer slope nonzero).
4. For each literal, construct virtual terms covering every possible lower
   bound of a solution interval:
   discontinuities, zero crossings between discontinuities, core-interval
   boundaries, and unbounded tails.
5. Union those per-literal sets to form `elim_x(phi)`.
6. Virtually substitute every candidate into the whole conjunction.
7. A `Z` candidate is reduced to finitely many grid representatives using
   common periods and aperiodic core intervals.
8. Disjoin the substituted formulas:

   `exists x. phi  <=>  OR_{v in elim_x(phi)} virtual_substitute(phi, x, v)`.

The detailed mathematics is in
[mathematical-specification.md](mathematical-specification.md). A proposed
software decomposition and executable pseudocode are in
[implementation-blueprint.md](implementation-blueprint.md).

## Recommended reading order

1. [mathematical-specification.md](mathematical-specification.md) - the
   reconstructed calculus, including every recursive definition needed by the
   base procedure.
2. [implementation-blueprint.md](implementation-blueprint.md) - data
   structures, algorithms, invariants, normalization, and integration
   boundaries.
3. [conflict-driven-extension.md](conflict-driven-extension.md) - the CDVS
   search state machine and VIRAS-specific conflict lemmas.
4. [validation-plan.md](validation-plan.md) - unit vectors, end-to-end examples,
   proof obligations, fuzzing, and differential checks.
5. [sources-and-errata.md](sources-and-errata.md) - provenance, definite paper
   errors, ambiguities, and conservative resolutions.

## Confidence and remaining decisions

| Area | Confidence | Reason |
| --- | --- | --- |
| LIRA syntax and normalization | High | Explicitly defined in the paper; standard transformations fill the Boolean wrapper. |
| Term profiles, bounds, limits, and periods | High | Complete structural recurrences and proofs are provided. |
| Discontinuity-grid construction | High with edge-case guards | The recurrence is explicit; negative/zero grid periods need canonical handling that the paper omits. |
| Elimination sets | High | The full case split and correctness proof are provided. |
| Virtual substitution | High after one definite correction | The periodic/aperiodic parentheticals in one infinity rule are reversed in the paper; prose and proofs determine the intended rule. |
| `Z`-term finite reduction | High | All three cases and their correctness argument are given; the `+/- infinity` metavariable should be implemented as an explicit sign loop. |
| Basic CD-VIRAS | Medium-high | VIRAS supplies the new lemmas and cites a complete CDVS state machine. |
| Optimized conflict learning | Medium | The enhanced CDVS calculus is optional and VIRAS proves only the stated replacement lemmas. |
| Direct TPTP `$int`/`$rat`/`$real` integration | Separate adapter work | The paper uses one real domain plus floor, not the full typed TPTP arithmetic surface. |

## Repository integration observation

As of this research, the Rust source tree has general first-order term, formula,
and type infrastructure but no dedicated LIRA/VIRAS subsystem. A safe
implementation should therefore begin as an isolated exact-arithmetic kernel
with adapters to and from the prover's general term representation. It should
not make the generic term bank responsible for VIRAS-specific invariants such
as positive grid periods, normalized rational coefficients, or `x`-free
elimination candidates.
