# Preregistration: rational/real Fourier–Motzkin slice

Frozen: 2026-07-30, before prototype outcome generation, held-out production
CNF capture, external-solver execution, or held-out saturation.

Bead: `E_Rust_Port-9jt.5.6`

## Question

Can an exact, proof-replayable rational/real Fourier–Motzkin inference slice
add useful arithmetic closure or clause simplification at bounded clause
growth, while coexisting with ordinary propositional resolution?

This is a feasibility experiment, not an authorization to implement full
ALASCA or enable arithmetic inference in production.

## Provenance boundary

The architectural reference is the BSD-3-Clause Vampire tree, specifically:

- `Kernel/ALASCA/Normalization.hpp`;
- `Kernel/ALASCA/SelectionPrimitves.hpp`;
- `Inferences/ALASCA/FourierMotzkin.hpp`;
- `Inferences/ALASCA/FourierMotzkin.cpp`;
- `Inferences/ALASCA/Normalization.cpp`;
- the ALASCA engine registration in
  `Saturation/SaturationAlgorithm.cpp`; and
- the ALASCA options in `Shell/Options.cpp`.

The experiment may describe observable invariants and independently implement
the mathematical rule. It must not copy implementation expression. The VIRAS
implementation and `Inferences/ALASCA/VIRAS.cpp` are excluded from inspection
and use.

The pinned Vampire 5.0.1 reference executable is ignored, local-only, and
non-redistributable because it statically contains an unlicensed VIRAS
revision. It may be run only as an external reference with
`--alasca on --viras off`. No Vampire or Z3 source or binary enters an Umlaut
package.

## Frozen calculus slice

### Supported terms and sorts

- Sorts: `$rat` and `$real`.
- Variables and exact integer or rational constants.
- Addition, subtraction, unary minus, and multiplication or division by a
  nonzero exact constant.
- No uninterpreted function occurs inside the selected arithmetic monomial.
- Every clause is universally quantified; premise variables are
  standardized apart before inference.

### Supported arithmetic literals

Each literal is normalized to

`a_1*x_1 + ... + a_n*x_n + c > 0`

or

`a_1*x_1 + ... + a_n*x_n + c >= 0`,

where every coefficient is an exact rational. A positive scaling is
canonicalized without changing strictness. A constant true literal deletes
the clause; a constant false literal is removed from its clause.

Equality, disequality, integers, floor/ceiling, nonlinear products,
nonconstant division, mixed sorts, and interpreted functions outside the
declared linear fragment are unsupported and remain `Unknown`.

### Fourier–Motzkin inference

For standardized-apart clauses

`C ∨ (j*x + p >_1 0)` and
`D ∨ (-k*x + q >_2 0)`,

with exact `j > 0` and `k > 0`, derive

`C ∨ D ∨ (k*p + j*q > 0)`

when either premise inequality is strict, and use `>= 0` when both are
non-strict. The selected variable is absent from `p` and `q`.

The rule is a positive linear combination of two true inequalities after the
context literals are false. It is therefore sound over rationals and reals.
The prototype will not implement ALASCA ordering restrictions, abstracting
unification, integer correction, floor rules, equality generation,
superposition into arithmetic terms, factoring, coherence, or demodulation.

### Ordinary resolution

The same bounded saturation engine may apply exact propositional resolution
to complementary opaque literals. Arithmetic and propositional inference
certificates remain distinct so interaction can be measured.

## Corpus

### Synthetic corpus

A tracked generator with a frozen seed will create:

- satisfiable neutral systems;
- one- and multi-variable arithmetic contradictions;
- strict/non-strict boundary cases;
- disjunctive cases requiring both Fourier–Motzkin and propositional
  resolution;
- unsupported cases that must return `Unknown`; and
- clause-growth stress cases.

Template families, not individual generated instances, are separated across
train, validation, and test. At least 12 workloads are generated per
partition, with balanced expected SAT/UNSAT/Unknown classes where the
templates permit.

### Production-like corpus

The source population is the committed CASC-30 TFA manifest. Within each
manifest partition, select up to five smallest source files per family that
contain a `$rat` or `$real` declaration and a linear arithmetic token. Rank by
manifest `size_bytes`, then `problem_id`. Production
`umlaut --cnf --tstp-out` supplies the clause transcript.

The extractor retains only source-derived clauses wholly inside the supported
rational/real linear fragment plus opaque propositional context that can be
represented without term unification. All excluded clauses and exact
exclusion reasons remain in the report. It must not claim an end-to-end theorem
from a retained subset: subset UNSAT may diagnose an inconsistent source
subset, but subset SAT is never a source result.

### Prior exposure disclosure

Before this freeze, earlier arithmetic studies opened selected CASC TFA
sources and production CNF, including train families DAT/HWV/ITP/SWC/SWW/SYO
and held-out families ARI/NUM/ANA/SEV. Those studies did not implement or run
the clause-level Fourier–Motzkin slice. The manifest family split remains
fixed, but results on those named sources are not treated as blind model
selection evidence.

## Comparison arms

1. `normalize+resolution`: native normalization, trivial evaluation,
   subsumption, and propositional resolution only.
2. `native-fm`: the same engine plus the frozen Fourier–Motzkin rule.
3. `z3-control`: pinned Z3 at commit
   `2d48fd119ce5074b880944c2b1c59e537c99cd46`, run on a canonical SMT-LIB
   rendering. Z3 is an outcome control, never trusted proof evidence.
4. `vampire-theory-axioms`: pinned Vampire with ALASCA disabled and its normal
   arithmetic-axiom path.
5. `vampire-alasca-no-viras`: pinned Vampire with ALASCA enabled and VIRAS
   explicitly disabled.
6. Production Umlaut baseline on source problems, without experiment clauses
   or feature changes.

The synthetic native comparison uses identical clause, inference, and time
bounds for arms 1 and 2. External arms use the same wall-time limit and fixed
seed where supported. At least one warmup precedes five measured native
repetitions.

## Proof and independent checking

Every trusted native inference records:

- normalized parent clauses and their stable hashes;
- rule name;
- selected literal indices and eliminated variable;
- exact positive multipliers;
- standardized-apart variable map;
- normalized conclusion; and
- resource counters.

A separate checker reparses the original workload, recomputes every
normalization, propositional resolution, and Fourier–Motzkin conclusion, and
accepts UNSAT only when a replayed derivation reaches the empty clause.

At least six mutation classes must be rejected:

1. parent hash substitution;
2. wrong eliminated variable;
3. negative or zero multiplier;
4. altered multiplier;
5. changed strictness;
6. changed conclusion coefficient or constant;
7. deleted context literal; and
8. forged empty-clause status.

Unsupported syntax, missing parents, bounds, timeout, cancellation, malformed
records, and checker failure are `Unknown`.

## Frozen resource bounds

Per workload and arm:

- at most 256 input clauses;
- at most 64 literals per clause;
- at most 32 variables per arithmetic literal;
- at most 10,000 retained clauses;
- at most 100,000 inference attempts;
- at most 30 seconds wall time;
- exact numerators and denominators limited to 256 bits; and
- cancellation must terminate within one second without a trusted result.

Crossing any bound returns `Unknown` with the crossed bound recorded.

## Measurements

- eligible sources, clauses, literals, sorts, and exclusion reasons;
- SAT/UNSAT/Unknown outcomes;
- uniquely closed workloads relative to normalize+resolution;
- agreement with Z3 and the two Vampire controls;
- input, generated, retained, subsumed, and peak clause counts;
- arithmetic versus propositional inference counts;
- clause-growth ratio and maximum coefficient bit length;
- wall time and per-inference time;
- proof replay coverage and mutation rejection;
- neutral outcome and work deltas;
- production Umlaut and Vampire search statistics where available; and
- source/package/dependency impact.

## Advancement gates

The experiment may recommend a production prototype only if all gates pass:

1. all trusted steps and every empty-clause result replay independently;
2. all mutation, malformed-input, bound, timeout, and cancellation tests fail
   closed;
3. held-out native and Z3 statuses agree on every supported synthetic
   workload;
4. at least 20 held-out production-derived eligible clauses occur across at
   least three families;
5. native FM closes at least three held-out workloads not closed by
   normalize+resolution, including at least one production-derived workload;
6. no baseline-closed or neutral workload is lost;
7. median retained-clause growth is at most 4x and p95 at most 10x;
8. held-out native p95 wall time is at most 50 ms per workload;
9. coefficient bounds are never silently exceeded;
10. the implementation is optional, removable, dependency-free, and adds at
    most 256 KiB to a release binary; and
11. the comprehensive Ubuntu lifecycle remains clean.

Failure of any gate keeps production unchanged. Synthetic success alone is
insufficient.
