# Complementary connection-worker preregistration

## Bead and question

This experiment addresses `E_Rust_Port-9jt.6.7`.

Can a small, bounded connection-tableau worker add independently verified
coverage or a credible proof-cost advantage over Umlaut's saturation search on
an equality-free first-order theorem cluster?

The prototype is deliberately narrower than Connect++/SATResetCoP, CSI++, or
iProver. It tests one architectural hypothesis without importing code from
another prover and without changing Umlaut production search.

## Frozen revision and corpus

- Source revision before prototype implementation:
  `b80150e336b8c2da7b2d5fcefbd01cf71f7001c5`.
- Corpus: the 12 FNE records from the already candidate-blind,
  family-separated adaptive-probe manifest.
- Train: four `GEO`/`CSR`/`NLP` problems.
- Validation: four `LCL` problems.
- Test: four `NUN` problems.
- Every record is FOF, equality-free, expected theorem, and selected before
  this connection worker existed.
- The source manifest, problem bytes, includes, frozen corpus, binary, scripts,
  and preregistration are hashed into the run contract.

Earlier Umlaut saturation experiments used these problems, so validation and
test are connection-worker-blind but not globally unseen. No connection result
on validation or test may be inspected until the calculus, bounds, checker,
analysis, and decision rules below are implemented and unit/integration tested.

## Frozen calculus

Umlaut is the shared front end. Each connection run first executes:

```text
umlaut --cnf --no-preprocessing --tstp-format PROBLEM
```

The worker accepts only equality-free `cnf` clauses and uses:

1. a start-clause restriction to `negated_conjecture` clauses;
2. first-order complementary extension with variables standardized apart;
3. path reduction against a complementary ancestor;
4. an occurs-checking first-order unifier;
5. regularity pruning when the current instantiated literal already occurs on
   the branch;
6. deterministic fail-first goal choice, reduction before extension, and
   shortest-residue/input-order extension choice; and
7. iterative deepening through at most 12 extensions on one branch, a
   500,000-search-node cap, and a five-second wall deadline including
   clausification.

The calculus may return `Theorem` only for a closed tableau. Every exhaustion,
deadline, unsupported construct, or parser failure is `Unknown`; it must never
make a negative theorem claim.

The experiment does not use accumulated ground SAT. A connection path does not
provide the stable ground clause stream needed to justify that extra mechanism,
and the narrow FNE hypothesis can be tested without it.

## Certificate and independent replay

The certificate contains the selected start clause and a proof tree whose
nodes identify only:

- the selected goal position and instantiated diagnostic literal;
- a reduction path position; or
- an extension source-clause position, source-literal position, and fresh
  standardization identifier.

It does not contain a trusted final substitution. A separate verifier reparses
the CNF transcript, reruns clausification byte-for-byte with the frozen binary,
implements its own dereferencing, occurs check, unifier, standardization, and
proof-state transition, and reconstructs every substitution while replaying
the tree. It checks start-clause provenance, freshness, goal diagnostics,
regularity, complementary polarity, complete branch closure, artifact hashes,
and the original problem hash.

Hand integration cases must exercise extension, reduction, shared variables,
function terms, and bounded `Unknown`. Mutations to the start clause,
extension clause, extension literal, reduction path, goal position or
diagnostic, transcript, and problem must be rejected when the corresponding
field is present.

## Arms and budgets

Each train coordinate runs once. Each validation and test coordinate runs
twice.

1. `connection`: the frozen worker, five wall seconds including CNF.
2. `global_aw`: Umlaut with KBO6 and
   `(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))`,
   five soft CPU seconds and seven hard CPU seconds.
3. `goal_hard_priority`: Umlaut with KBO6 and
   `(5*Refinedweight(PreferGoals,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))`,
   the same limits.

Saturation theorem claims are rerun with proof output and checked through the
repository's positive-only validation gate using ProofCheck 1.0. Connection
claims are independently replayed as above.

The `independent_portfolio` result is the per-coordinate union of
`connection` and `goal_hard_priority`. It represents a two-worker,
five-second-wall portfolio and is reported separately from single-worker cost;
it is not presented as an equal-core comparison.

No clause-exchange arm is run. An unfinished tableau branch is not a logical
consequence suitable for saturation ingestion, while a closed tableau already
ends the search. Designing proof-producing lemma extraction is a different
calculus and is outside this bounded experiment. This is a soundness-driven
architecture rejection, not missing data.

All arms run with four-way outer scheduling on the same retained four-core
Ubuntu runner. Per-run wall, process user/system CPU, maximum RSS, proof bytes,
and method-native inference counts are recorded. Timing is descriptive;
coverage and checked proof evidence are primary.

## Correctness gates

The experiment is invalid if any of these fail:

1. a source problem or include hash differs from the frozen corpus;
2. either held-out repetition has a polarity disagreement;
3. a reproducible connection theorem lacks successful independent replay;
4. a reproducible saturation theorem lacks a successful ProofCheck gate;
5. held-out repetitions disagree on terminal theorem versus unknown status;
6. a run artifact differs from its recorded hash;
7. the independent-portfolio union is inconsistent with its components; or
8. the hand-case and mutation matrix does not pass on Ubuntu.

## Decision rules

Only validation may be inspected before the final test launch. There is no
parameter selection: the frozen worker and all arms advance unchanged.

The result is `advance-native-prototype` if all correctness gates pass and
either:

- `connection` has at least one reproducible, independently checked test solve
  not solved reproducibly by `goal_hard_priority`; or
- on at least two common reproducible test solves, connection retains every
  goal-priority test solve, uses at most half as many proof-tree rule nodes in
  aggregate as the saturation proofs contain annotated proof formulas, and
  has no more than 1.5 times the median wall time.

It is `validation-only-signal` if the unique-solve rule holds on validation but
not test. It is `stop` if the worker adds no reproducible held-out solve and
does not meet the proof-cost alternative, or if it loses a reproducible
goal-priority test solve. Any correctness failure is `invalid`.

An advance decision still does not change production. It permits a follow-up
native Rust worker design covering typed terms, cancellation, equality,
portfolio scheduling, proof-object integration, and only then proof-producing
lemma exchange. A stop decision retains saturation as the production core.

