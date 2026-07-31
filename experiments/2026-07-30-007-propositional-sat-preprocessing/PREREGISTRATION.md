# Preregistration: propositional SAT preprocessing

Bead: `E_Rust_Port-9jt.4.7`

Date frozen: 2026-07-30

## Question

Does modern SAT preprocessing materially improve complete propositional or
soundly extracted ground-clause workloads relative to Umlaut's current SAT
path and to the same CDCL backend with preprocessing disabled? Can any
transformed state be reused across recurring SATCheck snapshots without losing
source, model, or proof reconstruction?

This is an investigation, not an adoption patch. No production option,
automatic schedule, default backend, or package feature may change during the
experiment.

## Static audit completed before implementation

- `umlaut-dpll` intentionally preserves E's unfinished parser/state-constructor
  behavior and does not solve. The production comparison baseline is the
  recursive solver behind `src/clauses/satinterface.rs`.
- Pinned CaDiCaL 3.0.1 exposes a `plain` configuration documented in its source
  as disabling all internal preprocessing. Its default enables bounded
  variable elimination, failed-literal probing and hyper-binary reasoning,
  subsumption, and vivification, among other CDCL facilities.
- The existing incremental SAT contract already requires complete model
  validation, independently checked UNSAT proof output, explicit limits, and
  fail-closed errors. The experiment must not weaken those obligations.
- Production SATCheck snapshots use local atom numbering and fresh solver state.
  Earlier measurements found median exact-clause retention of 68.2% but only
  41/126 add-only transitions. Integer equality between two snapshots is not a
  stable source identity and must not be treated as one.

## Frozen inputs

### Whole-problem coverage

Scan every one of the 2,901 problem records in
`benchmarks/casc_2025_manifest.jsonl` against the separately retained archive
`.artifacts/casc-benchmark/casc_2025_corpus.tar.gz`. Both the manifest record
SHA-256 and archive member SHA-256 must match before classification.

The declared complete whole-problem fragment is deliberately narrow:

1. the problem has no `include` statement;
2. every non-comment statement is a `cnf` record;
3. no record has role `conjecture`;
4. every clause is a disjunction of zero-arity, uninterpreted predicate
   literals, `$true`, or `$false`;
5. equality, inequality, variables, function application, quantified/formula
   syntax, and unparsed trailing input are rejected; and
6. every accepted source record and atom receives a deterministic DIMACS
   mapping.

For this fragment, Boolean SAT/UNSAT is complete for the original problem.
Rejected ground first-order or EPR inputs are not silently called
propositional.

### Extracted ground-clause workloads

Use every unique session named by
`workloads/captured-test-final/manifest.json` in the retained experiment-012
archive:

```text
.artifacts/experiments/2026-07-28-012-incremental-sat-service/results.tar.gz
```

The archive is integrity-pinned at SHA-256
`85356e073a26234f51e07898019d0a9a7685066eff21dd9350d621ede3158375`.
The prior manifest describes family-held-out CASC sessions and their source
capture paths. The harness must validate every session hash and reject
duplicate or malformed sessions rather than changing their meaning. Every
declared query is a coordinate: permanent clauses are materialized to DIMACS
and query assumptions are appended as unit clauses, so the checked
model/proof scope is exact. No workload or query may be selected or discarded
using a candidate result.

These integer clauses are sound SATCheck abstractions, not complete models of
their originating first-order problems. Their capture path is retained, but
the old archive has no stable source-clause identity suitable for production
proof publication or cross-call state reuse.

## Frozen solver arms

Each accepted DIMACS query scope is run from fresh state:

1. `internal`: Umlaut's current recursive SAT solver;
2. `plain`: pinned CaDiCaL 3.0.1 configured with `plain`, followed by
   `simplify(3)` and `solve`; and
3. `default`: the same CaDiCaL build with default preprocessing, followed by
   `simplify(3)` and `solve`.

The explicit simplify call separates initial transformation cost from solve
cost. The default solve may additionally inprocess; the reported solve cost
therefore includes inprocessing after the initial simplification phase.

Every extracted session/query/arm coordinate runs 20 times. Ordering is
SHA-256-shuffled independently per repetition. At most eight processes run on
the dedicated eight-core Ubuntu runner. Each query has a one-second external
wall budget, 512 MiB process memory budget, and the session's frozen native
decision limit. Controller overhead is outside the recorded solver timings.
An unchanged second invocation must resume without executing a solver.

Accepted whole problems, if any, compare `umlaut --auto`, `plain`, and
`default` under the same one-second wall and 512 MiB budgets with five
repetitions.

## Correctness and reconstruction gates

Interpret performance only if:

1. every completed arm agrees on SAT/UNSAT polarity;
2. every CaDiCaL SAT result supplies a complete assignment satisfying the
   exact original DIMACS clauses;
3. every CaDiCaL UNSAT result in a separate proof pass emits a DRAT trace
   accepted against the exact original DIMACS formula by an independently
   built `drat-trim`;
4. every accepted whole-problem mapping round-trips source clause names,
   literal polarities, and atom names, and its SAT assignment satisfies those
   mapped source clauses;
5. small formulas are also exhaustively enumerated as an independent oracle;
6. timeouts and resource limits return `Unknown`, never SAT or UNSAT; and
7. corruption tests reject a mutated model, clause mapping, input hash, and
   proof.

Any false status, invalid model, rejected proof, or unchecked claimed result
fails the relevant arm and forbids promotion.

## Reported measurements

Report:

- whole-problem recognition count and rate by category, division, family, and
  rejection reason;
- extracted-session coverage by category, division, family, variables, and
  clauses;
- materialization, insertion, initial simplify, solve, total wall, and maximum
  RSS costs;
- variables and irredundant clauses before and after simplification;
- SAT/UNSAT/Unknown counts and equal-budget arm-only solves;
- complete-model and checked-proof success rates, trace bytes, and checker
  cost;
- per-problem whole-solver solve deltas versus `umlaut --auto`; and
- consecutive-session original and post-simplification clause overlap,
  add-only rate, and the exact stable-identity blocker.

Timeout-filled aggregate CPU is diagnostic only. Ratios use paired medians;
p95 and maxima are reported so a favorable median cannot hide a bad tail.

## Decisions frozen before execution

Recommend CaDiCaL default preprocessing over CaDiCaL `plain` for extracted
SAT workloads only if correctness is perfect, it loses no solve, and either:

- it adds at least one solve; or
- common-completed median total cost is at most `0.85`, p95 is at most `1.05`,
  maximum RSS is at most `1.10`, and at least 20% of sessions reduce active
  variables or irredundant clauses by at least 10%.

Recommend a production whole-problem specialist follow-up only if at least 20
accepted held-out problems span at least four families, correctness is
perfect, no baseline solve is lost, and it either adds two solves or reduces
common-solved median wall time by at least 20% with p95 no worse than `1.05`.

Do not recommend cross-call transformed-state reuse unless at least half of
consecutive pairs are add-only, median post-simplification clause retention is
at least 50%, and a stable atom/source-clause identity design exists. The
known absence of stable identity is an independent veto: favorable integer
overlap alone cannot pass.

If no adoption gate passes, retain the evidence and close the candidate
without production changes. No post-hoc corpus, option, repetition count,
budget, or threshold may change these decisions.
