# Preregistration

## Question

Can a bounded, restart-based AVATAR worker soundly use Umlaut's incremental SAT
service to control variable-disjoint clause components, and is there enough
held-out search benefit to justify integrating assertions into the live
saturation loop?

The experiment is allowed to conclude that the restart architecture is too
expensive. Such a negative result closes this prototype Bead if all soundness
and reporting gates pass.

## Provenance and semantic contract

The design follows the published descriptions in:

- Andrei Voronkov, *AVATAR: The Architecture for First-Order Theorem Provers*,
  CAV 2014;
- Giles Reger, Martin Suda, and Andrei Voronkov, *AVATAR: The Architecture for
  First-Order Theorem Provers*, ARW 2015 overview;
- Laura Kovács, Simon Robillard, and Andrei Voronkov, *Coming to Terms with
  Quantified Reasoning*, POPL 2017; and
- Christoph Weidenbach, *Unifying Splitting*, Journal of Automated Reasoning
  2023.

No Vampire source is inspected, copied, translated, linked, or included.

For an input clause `C = C1 | ... | Cn`, a component is a maximal set of
literals connected by shared variables. Distinct components therefore have
disjoint variable sets. The prototype introduces selector `[Ci]`, reusing a
selector only for alpha-equivalent components, and adds the propositional clause
`[C1] | ... | [Cn]`.

A complete SAT model activates every component whose selector is true. A fresh
Umlaut process receives all unsplit input clauses and all active components. If
ProofCheck independently validates that branch refutation, the prototype adds
the conservative conflict `| -[Ci]` over *every* active selector in the model.
No finer proof ancestry is inferred. When the propositional constraints plus
verified conflicts become UNSAT, the original problem is reported UNSAT.

The prototype deliberately excludes:

- non-CNF inputs and `include` statements;
- generated-clause splitting;
- live assertion propagation;
- clause locking or deletion;
- unverified conflict learning; and
- model claims.

These bounds avoid known completeness hazards around unrestricted locking and
make every successful meta-result independently replayable.

## Frozen corpus

The immutable source is
[`benchmarks/casc_2025_manifest.jsonl`](../../benchmarks/casc_2025_manifest.jsonl).
`select_corpus.py` uses only manifest fields and parsed syntax. It does not run a
prover or inspect a result.

The frozen `corpus.jsonl` has SHA-256
`17a829a2c18465d43f0765578e4a3da9a67054e1c8411cb91fdaa124b1b3cea5`.
Its 46 first-order CNF problems are all expected UNSAT, contain no includes, are
at most 3,500,000 bytes and 20,000 clauses, and come from EPR or UEQ. Complete
source families remain confined to the CASC-30 train, validation, or test
partition.

| Partition | Split-sensitive | Neutral |
| --- | ---: | ---: |
| Train | 12 | 12 |
| Validation | 4 | 8 |
| Test | 3 | 7 |

Split-sensitive problems have at least one decomposable input clause and at most
32 selectors when the six highest-ranked clauses are selected. Neutral problems
have no decomposable input clause. Within each cell, records are ranked by
SHA-256 of a fixed salt, partition, cohort, and problem ID.

Train is for implementation diagnosis only. No algorithm or threshold is
selected from validation or test outcomes.

## Fixed methods and resources

Every problem is run in three modes from the same release build:

1. **baseline**: `--auto` with proof output;
2. **static split**: baseline plus `--split-clauses=7 --split-method=2
   --split-aggressive --split-reuse-defs`; and
3. **bounded AVATAR**: at most six ranked input clauses, at most 32 SAT models,
   and fresh baseline branch searches.

Each mode receives 20 seconds of cumulative prover wall time and 2,048 MiB.
Proof checking time is measured separately and is not charged to the prover.
The AVATAR budget is shared by all branch restarts; it is not 20 seconds per
branch. Runs are sequential per problem. The experiment may parallelize
different problems, but reports individual process measurements and does not
compare aggregate host wall time.

The SAT layer is Umlaut's `InternalSatService`, accessed through the persistent
experiment-only Rust driver. It is not a second SAT implementation. The
certificate verifier uses a separate Python DPLL implementation only for
independent replay of the final propositional claim.

The external proof checker is ProofCheck 1.0 with SHA-256
`92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`.

## Correctness and falsification gates

All of the following are mandatory:

1. focused lexical, decomposition, alpha-reuse, ranking, and branch-rendering
   tests pass;
2. Rust driver protocol tests pass and an integration transcript demonstrates
   incremental clauses, a complete model, conflict learning, and UNSAT;
3. the independent verifier reparses the original without importing
   `tptp_split.py`, checks the variable-disjoint partition, selector reuse,
   split clauses, every SAT model, every learned conflict, and every rendered
   branch;
4. every branch, baseline, or static proof counted as solved is `VerifiedGood`
   under ProofCheck;
5. the verifier independently derives propositional UNSAT before accepting an
   AVATAR success;
6. mutations to a component, active-selector set, SAT model, conflict, branch,
   proof hash, and final status are rejected; and
7. malformed or incomplete result records fail closed.

A timed-out or unrefuted active branch stops that AVATAR run as unknown. It is
never excluded by assumption.

## Measurements

For every problem and mode the report includes claimed and verified status,
prover wall and user CPU time, maximum RSS, proof-check time, and raw artifact
hashes. AVATAR additionally reports:

- selected split clauses, distinct selectors, SAT calls and SAT time;
- branch count, verified conflicts, active selectors per branch;
- inactive component clauses (`selector count - active count`) and fraction;
- cumulative branch time and peak branch RSS; and
- termination reason.

The analysis reports each cohort and partition separately, plus the combined
held-out validation and test sets. It reports verified solves, unique solves,
pairwise wins/losses, medians and p95 where defined, memory maxima, and every
soundness-gate result. Tiny held-out samples are described as feasibility
evidence, not a general performance estimate.

## Advancement rule

Production AVATAR integration is justified only if:

1. every correctness and falsification gate passes;
2. there is no baseline-only verified solve on the combined held-out
   split-sensitive set;
3. bounded AVATAR has at least one held-out split-sensitive verified unique
   solve, or its paired median prover wall time is at most 90% of baseline with
   at least three paired solves;
4. on held-out neutral problems it loses no baseline solve and paired median
   prover wall is at most 110% of baseline; and
5. its maximum RSS is at most 115% of baseline's maximum.

Failure to advance leaves production unchanged. Interesting proof traces or
activation rates may motivate a narrower follow-up Bead but cannot override
these gates.
