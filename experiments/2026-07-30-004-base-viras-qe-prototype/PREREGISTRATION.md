# Base VIRAS one-conjunction prototype preregistration

Bead: `E_Rust_Port-9jt.5.2`

## Question

Can the paper-derived base VIRAS calculus be implemented as an isolated,
clean-room exact kernel for one existentially quantified conjunction, with
finite virtual substitution, auditable derivation records, and fail-closed
resource limits?

## Provenance boundary

The candidate implementation may use only:

- the tracked `viras_docs/` clean-room packet;
- the separately implemented oracle in
  `tools/validation/arithmetic_qe_oracle.py`;
- Python's standard-library exact `fractions.Fraction`; and
- the caller-supplied pinned Z3 source archive used by experiment 005.

It must not inspect, import, execute, or derive tests from the unlicensed VIRAS
source tree. The prototype remains experiment-local and is not production
Umlaut code.

## Declared fragment

The prototype accepts one nonempty conjunction of normalized literals
`t = 0`, `t != 0`, `t > 0`, or `t >= 0`, where `t` contains exact rationals,
real variables, addition, rational scaling, and nested floor. One named real
variable is eliminated; every other variable is a free real parameter.

The result is a quantifier-free Boolean DAG over the same ordinary term
language. The declared milestone does not include arbitrary Boolean
normalization, quantifier alternation, typed TPTP import/export, conflict-driven
search, or production scheduling.

## Frozen gates

The prototype advances as a trustworthy kernel milestone only if all of the
following pass:

1. The exact rational LCM/remainder vectors and term-profile golden vectors in
   `viras_docs/validation-plan.md` pass.
2. Grid intersection covers every concrete grid point for at least 1,000
   generated cases, including open/closed and zero-width boundaries.
3. Every listed no-break elimination branch, epsilon/infinity substitution
   vector, and V1/V2/V3 grid-flattening example passes.
4. The corrected motivating example is equivalent to `c <= 2/3` for a frozen
   parameter matrix containing negative, integral, and nonintegral values.
5. At least 1,000 seeded generated closed conjunctions, including nested floors
   and slope cancellation, agree with both the separately implemented exact
   cell oracle on a conservative bounded window and pinned Z3 on the unbounded
   query. Candidate output evaluation must agree with those decisions.
6. At least four deliberately corrupted calculus mutations are rejected by the
   frozen corpus: truncating negative floor, reversing infinity periodicity,
   dropping epsilon strictness, and omitting a generated candidate.
7. Reordering and duplicating literals and canonical term rewrites preserve
   every generated decision.
8. Tiny candidate, grid, step, rational-bit, and formula-node limits each
   return `Unknown(ResourceLimit)`; unsupported formula shape returns
   `Unknown(UnsupportedFragment)`. No partial result may be classified false.
9. No successful output contains the eliminated variable or a virtual
   epsilon, grid, or infinity marker, and each output includes candidate and
   substitution derivation records.
10. Two runs with the same seed produce byte-identical canonical reports.

## Stop rules

Stop without production integration if any differential disagreement cannot be
minimized and explained, if a paper recurrence is still ambiguous after the
tracked errata, if required candidate growth cannot be bounded fail-closed, or
if the milestone needs the excluded implementation to proceed.
