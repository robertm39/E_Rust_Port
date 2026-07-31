# Preregistration: conflict-driven VIRAS feasibility

Bead: `E_Rust_Port-9jt.5.4`

Date frozen: 2026-07-30

## Question

Can conflict-driven exploration of base-VIRAS candidates reduce the number of
virtual substitutions for closed existential conjunctions while every learned
clause is independently proved sound and progress-making?

This is a clean-room feasibility experiment. It does not authorize production
CD-VIRAS, an automatic schedule, an external solver dependency, or use of the
unlicensed implementation under `viras/`.

## Frozen scope

The search prototype imports only the tracked clean-room exact kernel from
Experiment 004. Its comparative slice is a deliberately narrow subset of
VIRAS:

- a closed, nonempty conjunction over existential real variables;
- exact rational affine terms without `floor`;
- relations `=`, `>=`, and `>`;
- stable lexical variable order;
- at every non-ground search node, the selected variable occurs with nonzero
  coefficient in at least one residual equality; and
- only the exact, finite zero candidates originating in those equalities are
  enumerated.

The equality-origin restriction is complete on this slice: every model must
satisfy each such equality, so it must agree with one of the enumerated exact
zeros. Candidates containing epsilon, infinity, or a `Z` grid are rejected as
unsupported rather than approximated. Ground-only simplification is used so
the experiment measures the documented candidate calculus rather than an
unrelated simplex prepass.

Every stack entry retains the original formula, exact prefix, residual
conjunction, candidate origin, and candidate-generation context. This is the
minimum context required by the notation-gap warning in
`viras_docs/conflict-driven-extension.md`.

The experiment also records, but does not implement, the documented epsilon,
aperiodic-infinity, periodic-residue, and epsilon-plus-infinity lemma branches.
Those unsupported branches prevent a production-readiness recommendation even
if the finite slice performs well.

## Frozen treatments

All treatments use identical stable variable and candidate orders.

1. `eager`: construct the complete reachable candidate tree, including every
   substitution, before taking the disjunction of leaves. It performs no
   learning and does not stop after the first satisfying leaf.
2. `basic`: depth-first search with early SAT termination. A ground conflict
   learns the documented disjunction of plain disequalities for the complete
   assignment stack. Exhausted subtrees learn the corresponding full-prefix
   clause.
3. `focused`: the same search, but a ground conflict starts from the
   transitive assignment support of a falsified original literal, and both leaf
   and inner clauses are deletion-minimized in stable order when an independent
   exact affine checker proves the smaller clause sound.

The stronger treatment is intentionally an upper-bound control, not a proposed
production architecture. Its exact conflict checks count toward runtime and
may erase any search-count advantage.

For a learned clause

```text
(x1 != t1) or ... or (xk != tk)
```

the independent checker proves exact infeasibility of:

```text
original_F and x1 = t1 and ... and xk = tk
```

using rational Fourier-Motzkin elimination implemented separately from the
candidate search. Before insertion, a second progress check successively
substitutes the rejecting stack into the clause and requires ground `false`.
An empty learned clause is permitted only when the affine checker proves the
original conjunction infeasible.

## Frozen workloads

The hand-authored corpus contains at least these state-machine boundaries:

- ground true and ground false inputs;
- first-variable exhaustion to UNSAT;
- an early satisfying branch that leaves candidates unvisited;
- a cross-variable candidate whose term depends on a later variable;
- a conflict with assignments irrelevant to its falsified source literal; and
- an unsupported epsilon/documented-periodic boundary.

The generated corpus uses seed `0x43445649524153` and 300 cases:

- 100 satisfiable affine equality graphs;
- 100 equality graphs with a tail conflict; and
- 100 equality graphs with a sparse cross-variable conflict.

Each supported case has three to seven variables and stable exact rational
coefficients. Corpus construction is treatment-blind. At least 50 supported
SAT and 50 supported UNSAT cases must remain after eligibility checks, or the
experiment is invalid.

Pinned Z3 is an external differential reference on the retained Ubuntu runner.
Each original supported case and every unique learned-clause implication is
checked in an incremental exact-real SMT-LIB session. Z3 is not used to
generate candidates, choose conflicts, or accept a learned clause during
search.

## Recorded evidence

For every case and treatment, record the decision, candidate generations,
virtual substitutions, explored leaves, learned clauses, admissibility
prunes, affine-check combinations, peak learned-clause count, elapsed
process time, and a canonical semantic trace hash. Full traces include every
candidate origin, prefix/residual formula, conflict, learned clause, validation
result, backtrack, and terminal rule.

The run is repeated twice. Decisions, counters, learned clauses, and semantic
trace hashes must be identical; elapsed timings are summarized separately.

Focused unit tests cover affine extraction, strict and non-strict
Fourier-Motzkin feasibility, exact candidate completeness on the declared
slice, progress checking, sound/unsound clause rejection, deterministic
enumeration, early SAT, first-variable UNSAT, dependency closure, inner
conflicts, unsupported virtual terms, and injected unsound-learning
mutations.

## Frozen gates and decision

Correctness gates:

1. every supported decision agrees across all three treatments and with pinned
   Z3;
2. every inserted learned clause passes the independent affine implication
   check and the progress check;
3. Z3 reports `unsat` for every learned-clause implication query;
4. both repetitions have identical semantic outputs; and
5. all mutations are rejected.

Search-reduction gate:

- on generated supported UNSAT cases, `focused` must use at most 75% of
  `basic` virtual substitutions in aggregate and improve at least 60% of cases;
  and
- across all generated supported cases, `basic` must use fewer substitutions
  than `eager` in at least 60% of cases.

Overhead gate:

- median `focused/basic` elapsed time must be at most 2.0, and no treatment may
  hit its frozen one-million-step or 100,000-affine-combination limit.

The result is `prototype-supported` only if all correctness, reduction, and
overhead gates pass. Even then, production CD-VIRAS remains deferred until the
epsilon, both infinity, periodic residue, grid flattening, and multi-variable
lemma-lifting contracts are implemented and independently validated.

The result is `stop` if a correctness gate fails. It is `defer` if correctness
holds but reduction, overhead, corpus, or unsupported-branch coverage fails.
No threshold or eligibility rule changes after comparative results are
observed.
