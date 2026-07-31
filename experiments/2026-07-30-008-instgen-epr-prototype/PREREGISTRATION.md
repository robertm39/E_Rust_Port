# Preregistration: bounded model-guided EPR instantiation

Bead: `E_Rust_Port-9jt.4.5`

Date frozen: 2026-07-30

## Question

Can a clean-room, fragment-scoped Inst-Gen-style worker complement Umlaut's
superposition search on equality-free, function-free CNF? In particular:

1. can model-guided propositional abstraction and refinement be checked
   soundly end to end;
2. does the worker add verified solves under a fixed budget; and
3. does passing only replayable source-clause instances to Umlaut beat both an
   equal-budget independent saturation worker and an equal-budget portfolio?

A negative answer completes this feasibility study. It does not require a
production implementation.

## Provenance and scope

The architecture is derived from the published Inst-Gen descriptions:

- Harald Ganzinger and Konstantin Korovin, *New Directions in
  Instantiation-Based Theorem Proving*, LICS 2003;
- Konstantin Korovin, *Inst-Gen - A Modular Approach to
  Instantiation-Based Automated Reasoning*, Programming Logics 2013; and
- Ahmed Bhayat and Konstantin Korovin, *Implementing Superposition in
  iProver*, IJCAR 2020.

The implementation does not inspect, copy, translate, or link iProver or
Vampire source. It is deliberately described as Inst-Gen-style rather than an
implementation of the full Inst-Gen calculus: refinement uses exhaustive
finite-domain counterexample search instead of the calculus's selected-literal
unification rules.

The accepted input is untyped TPTP `cnf` with:

- no `include`, FOF, TFF, THF, equality, disequality, interpreted predicate, or
  interpreted term;
- predicates applied only to variables or constants;
- no function symbols of positive arity;
- at most 200,000 source bytes, 1,500 clauses, 512 constants, 4,096 predicate
  signatures, 64 variables per clause, and 128 literals per clause.

Variables use the TPTP uppercase-or-underscore convention. Quoted tokens are
constants. If the input contains no constant, the prototype adds one fresh
Herbrand constant. This fragment has a finite Herbrand universe.

## Frozen algorithm

For every source clause, the worker initially adds the ground instance that
maps every variable to the first sorted domain constant. Ground atoms are
mapped injectively to propositional variables and submitted to integrity-pinned
CaDiCaL 3.0.1 using its public API.

After each SAT result, the worker completes unmentioned atoms to false and
enumerates source clauses and their ground substitutions in deterministic
lexicographic order. It adds the first false instance of each clause, up to 64
new distinct instances per refinement. It then solves the enlarged abstraction
again.

The outcomes are:

- **UNSAT** only when the current ground instance set is propositionally
  unsatisfiable;
- **SAT** only when every ground substitution has been enumerated and the
  complete interpretation satisfies every instance; and
- **UNKNOWN** on the wall limit, an external-solver limit, a fragment error, or
  any incomplete scan.

For this finite fragment, every ground instance is a first-order consequence
of its universally quantified source clause. Propositional UNSAT of a subset is
therefore a sound refutation. A complete satisfying Herbrand interpretation is
a sound model. The worker does not claim completeness outside the frozen
fragment.

## Frozen corpus

`select_corpus.py` reads only
`benchmarks/casc_2025_manifest.jsonl` and source syntax. It never runs a prover
or observes candidate outcomes. Within each fixed cell it ranks by
`SHA-256("umlaut-instgen-epr-v1", partition, family, expected class, problem
ID, source SHA-256)`.

The quotas are:

| Partition | Family | Expected class | Problems |
| --- | --- | --- | ---: |
| train | GRP | satisfiable | 3 |
| train | SYN | satisfiable | 3 |
| train | NLP | satisfiable | 2 |
| train | MSC | unsatisfiable | 3 |
| validation | PUZ | satisfiable | 6 |
| validation | PUZ | unsatisfiable | 4 |
| validation | SWV | unsatisfiable | 6 |
| test | PLA | unsatisfiable | 2 |

The manifest's complete-family partitioning is retained. Train is used only
for implementation diagnosis. No algorithm, batch size, corpus cell, budget,
or decision threshold may change after candidate execution begins.

The resulting 29-problem `corpus.jsonl` has SHA-256
`830fc799926ecac212378586a02ab6d2832c9b8451639cfd7a2d837703c1ddf2`.
It contains 11 train, 16 validation, and two test problems; 14 have an expected
satisfiable class and 15 have an expected unsatisfiable class.

## Fixed methods and budgets

All processes run sequentially on one Ubuntu 24.04 Linode. Each coordinate is
run once on train and twice on validation/test. The long budget is four
seconds; the short budget is two seconds. Each process is limited to 1,536 MiB.
Proof-checking and certificate replay are measured separately and are not
charged to search.

The measured methods are:

1. **saturation**: production `umlaut --auto` for four seconds;
2. **standalone**: the Inst-Gen-style worker for four seconds;
3. **portfolio**: the union of an independent two-second Umlaut run and an
   independent two-second instantiation run; and
4. **cooperative**: a two-second instantiation run followed by a two-second
   Umlaut run over the original problem plus every replayable generated ground
   instance. If the first phase is already terminal and verified, that result
   is the cooperative result.

The two compound methods each receive four aggregate worker-seconds.
`saturation` receives the same aggregate budget in one worker. The portfolio
is executed sequentially to avoid host contention, but represents the solve
union of two workers that could run concurrently.

The cooperative input contains the original source clauses verbatim followed
by ground clauses. It exchanges no SAT assignment, heuristic priority,
unverified learned clause, or solver-internal lemma.

## Correctness and falsification gates

All applicable gates must pass before performance is interpreted:

1. parser, grounding, atom-map, counterexample, termination, rendering, and
   resource-limit unit tests pass;
2. an independent verifier reparses the source without importing the candidate
   module and replays every recorded substitution and rendered instance;
3. every standalone/cooperative UNSAT result has a CaDiCaL DRAT trace accepted
   by pinned `drat-trim`;
4. every standalone SAT result is independently checked against every ground
   source instance;
5. every Umlaut theorem/unsatisfiable claim is accepted by the repository's
   fail-closed validation gate and ProofCheck 1.0;
6. expected-class polarity, repeated held-out result polarity, hashes, resource
   records, and certificate schemas are consistent; and
7. mutations to the source hash, substitution, ground clause, complete model,
   DRAT trace, augmented clause, and Umlaut proof are rejected.

An unsupported or malformed result fails closed to UNKNOWN. Timed-out work is
never interpreted as SAT.

## Measurements

For every run, record status, wall and user CPU time, maximum RSS, SAT calls and
SAT time, refinement iterations, generated instances, unique ground clauses,
enumerated substitutions, ground-universe size, termination reason, and raw
artifact hashes. For verified refutations, record propositional and Umlaut
proof bytes plus checker time.

For each partition, expected class, family, and method, report verified solves,
unique solves, pairwise wins/losses, repeated-result stability, and medians and
p95s where defined. Specifically report whether cooperation beats the
four-second independent saturation worker and the equal-budget portfolio.

## Decision

A production follow-up is justified only if every correctness gate passes and,
on combined validation/test:

1. cooperation loses no reproducible verified saturation solve;
2. cooperation adds at least one reproducible verified solve over saturation;
3. cooperation adds at least one reproducible verified solve over the
   equal-budget portfolio, or reduces median common-solve user CPU to at most
   90% of portfolio without a proof-size or maximum-RSS increase above 15%;
4. standalone instantiation has at least one reproducible verified solve not
   solved by four-second saturation; and
5. no candidate/source expected-class polarity disagreement remains.

Otherwise production remains unchanged. An interesting refinement count or a
single training result cannot override these gates.
