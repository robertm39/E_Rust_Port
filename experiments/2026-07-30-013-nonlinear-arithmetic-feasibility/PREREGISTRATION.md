# Preregistration: nonlinear arithmetic and projection feasibility

Bead: `E_Rust_Port-9jt.5.8`

Date frozen: 2026-07-30

## Question

Does the pinned CASC-30 corpus contain enough directly useful nonlinear real
arithmetic to justify either a narrow external decision service or a
clean-room Rust reimplementation before CASC 2027, given Umlaut's requirement
that accepted theorem steps remain independently checkable?

This is a feasibility study. It does not authorize a production dependency,
source translation, schedule change, or acceptance of unverified solver
answers.

## Frozen demand census

The census uses every record in
`benchmarks/casc_2025_manifest.jsonl` and verifies each recorded problem hash.
The manifest's family-preserving train, validation, and test split is reported,
but no split is used to select cases or tune thresholds: this is an exhaustive
demand inventory.

The classifier reports four nested populations:

1. `all`: every manifest problem;
2. `arithmetic_active`: a problem containing a TPTP numeric sort, numeric
   literal, defined arithmetic operator, or defined arithmetic relation;
3. `nonlinear_active`: an arithmetic-active problem containing a symbolic
   product, symbolic divisor, or another non-polynomial numeric operation; and
4. `whole_real_polynomial`: a TFF problem whose non-type formulas use only
   Boolean connectives, equality, real quantifiers/constants, exact numeric
   constants, `$sum`, `$difference`, `$product`, `$uminus`, division by a
   nonzero numeric constant, and the four ordered real relations.

`whole_real_polynomial` excludes integer/rational variables, coercions,
rounding, symbolic or zero division, user functions and predicates, mixed
theories, unsupported roles, includes, and parse failures. A member is
`whole_qf_nra` when it is quantifier-free and has polynomial degree at least
two, and `whole_quantified_nra` when it has a quantifier and degree at least
two. Declared real constants are SMT constants, not numeric coefficients.

The script records a stable reason for every exclusion. It also records the
maximum polynomial degree, formula and quantifier counts, source family,
category, and manifest split. Seeded unit tests cover comments, nesting,
numeric constants, symbolic products, constant division, user-symbol
exclusion, role handling, and hash failure.

## Smallest candidate service

The smallest adoption candidate is whole-problem, pure, quantifier-free
nonlinear real arithmetic (`whole_qf_nra`). A theorem problem is translated to
one satisfiability query by asserting every axiom-like formula and the negation
of every conjecture. This boundary matches Z3's explicitly named
`qfnra-nlsat` tactic and avoids integer nonlinearity, mixed theories,
quantifier instantiation, and a live in-search protocol.

Quantified pure NRA is measured separately with `nlqsat`, but cannot expand the
smallest candidate after outcomes are observed. Model-based projection is
assessed as a possible implementation mechanism and in-search service, not
silently conflated with whole-problem decision coverage.

## Pinned external comparison

The external reference is the already audited MIT-licensed Z3 source commit
`2d48fd119ce5074b880944c2b1c59e537c99cd46`. The runner must verify that
commit, source archive, executable hash, and reported version. No Z3 source or
binary enters Umlaut.

Each eligible query is run twice in a fresh, shell-free process with:

- exact SMT-LIB rational constants;
- a 10,000 ms solver timeout and 15,000 ms harness deadline;
- `(check-sat-using qfnra-nlsat)` for `whole_qf_nra`;
- `(check-sat-using nlqsat)` for `whole_quantified_nra`; and
- deterministic normalized status output.

The two repetitions must agree. `unsat` is compared with the manifest's
theorem classification and `sat`, `unknown`, timeout, malformed output, or
process failure remains explicit. This comparison measures raw solver
coverage, not trusted proof coverage.

The `return_unknown` baseline classifies every eligible query as `Unknown`,
has zero dependency bytes, and accepts no theorem step.

## Proof and model-based-projection boundary

Before result observation, the following are hard facts from the pinned
source and hard gates for interpretation:

- `nlsat_tactic.cpp` calls `fail_if_proof_generation("nlsat", g)`;
- an unsatisfiable core identifies input assertions but is not a checkable
  nonlinear arithmetic derivation;
- a matching CASC theorem status or second raw solver answer is not proof;
- SAT models with algebraic values require exact polynomial/root validation;
- NLSAT's conflict projection and the QE subsystem's model-based projection
  are internal algorithms, not a stable proof-certificate protocol; and
- `nlqe` is declared but marked `TBD_TACTIC` rather than registered as a
  supported command in the pinned source.

Accordingly, trusted coverage is zero unless the study demonstrates a
dependency-independent checker for every counted result. The experiment will
run a proof-generation probe on a nonlinear unsatisfiable formula and retain
the exact diagnostic. It will inventory projection/QE source, public tactic
surface, caveats, and implementation size. It will not report raw status,
core, model, or QE output as a trusted theorem proof.

## Reimplementation estimate

The report counts physical and nonblank non-comment C/C++ lines and files in
the pinned `nlsat`, `qe`, `math/polynomial`, and `math/realclosure`
subsystems. It separately identifies the minimum algorithmic obligations for
a clean-room implementation:

1. canonical multivariate polynomials and exact rational arithmetic;
2. real algebraic numbers, root isolation, sign determination, and comparison;
3. Boolean abstraction, conflict search, variable ordering, and resource
   control;
4. sound cell/projection construction with degeneracy handling;
5. model construction and exact validation;
6. independently checkable UNSAT evidence or a smaller trusted checker; and
7. parser/type integration, cancellation, fuzzing, differential tests, and
   cross-platform packaging.

For planning only, a paper-level clean-room port is `large` when the audited
reference surface exceeds 20,000 nonblank non-comment lines or spans all seven
obligations. `large` means at least 12 engineer-months before production
hardening and is a mandatory defer signal for this Bead; it is not a project
estimate or implementation commitment.

## Frozen decision rule

Recommend **pursue a narrow follow-up** only if all of these hold:

1. at least five distinct CASC-30 problems are `whole_qf_nra`;
2. pinned Z3 returns the expected `unsat` result deterministically for at least
   80% of them within the fixed timeout;
3. 100% of accepted `unsat` and `sat` results have a
   dependency-independent replay path demonstrated by this study;
4. the candidate adds no unresolved LGPL/MIT notice, StarExec, cancellation,
   Windows, or package-size blocker; and
5. the reimplementation classification is not `large`.

Recommend **defer** when measurable demand or raw external coverage is useful
but any proof, deployment, or engineering-cost gate fails. Recommend
**reject this candidate boundary** when fewer than five problems are
`whole_qf_nra` or raw expected-result coverage is below 80%.

Quantified-NRA and model-based-projection results may justify a separate future
research Bead, but they cannot rescue the preregistered whole-QF-NRA adoption
decision. No thresholds, eligibility rules, or trust definitions change after
solver outcomes are observed.
