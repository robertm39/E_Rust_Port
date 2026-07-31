# Preregistration: IPASIR-UP-style theory propagation

Bead: `E_Rust_Port-9jt.4.6`

Date frozen: 2026-07-30

## Question

Can a small, independently checkable external-propagator contract reduce
Boolean search relative to lazy conflict-only theory communication while
preserving deterministic assignment, reason, and backtrack semantics?

This is a simulation before live SAT integration. It does not authorize a new
CaDiCaL callback dependency, production AVATAR, or automatic schedule.

## Source and workload boundary

The contract follows pinned CaDiCaL 3.0.1's `ExternalPropagator` surface:
observed variables, batched assignment notification, decision levels,
backtrack notification, final-model checking, propagated literals, reason
clauses, and external conflict clauses. Every reason contains its propagated
literal and only observed variables. Learned external clauses trigger a
deterministic root backtrack in the simulation.

The earlier SATCheck archives cannot support a real theory replay: they retain
locally numbered propositional clauses but no stable atom meaning or theory
provenance. They are therefore excluded rather than decorated with invented
semantics.

The frozen workload uses the same separation as CaDiCaL's active-propagator
example. Boolean atom `P[p,h]` means pigeon `p` occupies hole `h`. CNF contains
only one at-least-one-hole clause per pigeon. The external theory enforces:

- at most one hole per pigeon; and
- at most one pigeon per hole.

Each theory reason is consequently a binary at-most-one clause `(-a or -b)`.
An independent checker validates group membership, distinctness, observed
variables, clause truth under every Boolean valuation of its two atoms, and
that the current trail makes a conflict clause false or a propagation clause
unit with the advertised literal.

## Frozen treatments

All treatments use the same stable atom order, negative-first Boolean
decisions, CNF unit propagation, learned-clause database, and one-million-step
limit.

1. `lazy`: consult the theory only on complete CNF models; reject a bad model
   with one validated binary conflict clause and restart from level zero.
2. `conflict`: also detect the first theory conflict after each Boolean unit
   propagation fixpoint; learn its validated clause and restart from level
   zero, but never request a theory propagation.
3. `propagate`: detect conflicts as above, then request the first stable
   theory-implied negative literal and its validated binary reason clause;
   learn it and restart from level zero.
4. `encoded`: a reference control with every at-most-one clause present in CNF
   from the start and no external callbacks.

Every restart logs the complete pre-backtrack trail, decision levels, clause,
reason kind, requested target zero, and empty post-backtrack trail. The
independent replay rejects missing propagated literals, non-unit reasons,
invalid pairs, out-of-order notifications, stale assignments, and nonempty
post-backtrack state.

## Frozen corpus

The hand corpus covers one SAT instance, one UNSAT instance, direct conflict,
direct propagation, and an injected invalid reason.

The generated corpus uses seed `0x4950415349525550` and 100 treatment-blind
atom permutations:

- 50 SAT instances with four pigeons and four holes; and
- 50 UNSAT instances with four pigeons and three holes.

Expected status follows the independently checked pigeonhole criterion. The
permutations, CNF, theory groups, and corpus hash are fixed before treatment
execution. Both statuses and at least 40 cases of each status must remain.

## Evidence and gates

Run two complete repetitions on the retained Ubuntu runner. Record decisions,
Boolean decisions, assignments, CNF propagations, theory callbacks, theory
propagations, conflicts, learned clauses, root backtracks, restarts, maximum
depth, elapsed time, and canonical trace hashes.

Correctness requires:

1. all four treatments and an exhaustive finite oracle agree on every case;
2. every external reason passes the independent structural and trail replay;
3. every complete SAT model satisfies CNF and both theory partitions;
4. every root backtrack empties the non-root trail;
5. both repetitions have identical semantic hashes; and
6. all injected reason, trail, and backtrack mutations are rejected.

On generated UNSAT cases, `propagate` must use at most 70% of `conflict`
Boolean decisions and at most 30% of `lazy` decisions in aggregate, and must
strictly improve at least 80% of cases against both. Median aggregate
`propagate/conflict` elapsed time must be at most 1.5. No treatment may hit a
resource limit.

The experiment is `prototype-supported` only if every correctness, reduction,
and overhead gate passes. It is `stop` on any correctness failure and `defer`
otherwise. Production remains deferred regardless: a follow-up would still
need a live CaDiCaL callback prototype, proof-trace integration for externally
added clauses, cancellation, stable production atom identities, and a real
recorded arithmetic or AVATAR workload.
