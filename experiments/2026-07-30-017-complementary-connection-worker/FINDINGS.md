# Complementary connection-worker findings

## Outcome

Bead `E_Rust_Port-9jt.6.7` is complete with a **stop** decision.

The bounded equality-free connection tableau produced one reproducible,
independently replayed test proof, `NUN081+1`. That solve is unique versus the
global age/weight baseline, but the stronger goal-priority saturation control
also solves it. The independent two-worker portfolio therefore adds no train,
validation, or test solve over goal-priority saturation.

The connection proof is compact—6 rule nodes versus 20 annotated formulas in
the saturation proof—but the signal occurs on only one common test problem and
the connection worker's paired median wall ratio is 2.083. It fails the frozen
two-problem, at-most-0.5-node-ratio, at-most-1.5-wall-ratio proof-cost gate.
There is no held-out unique solve, no accepted cost advantage, and no basis for
a native production worker.

Production remains unchanged.

## Prototype boundary

The experiment-only worker uses Umlaut's current
`--cnf --no-preprocessing --tstp-format` front end and then applies:

- a `negated_conjecture` start-clause restriction;
- complementary extension and path reduction;
- variables standardized apart;
- an occurs-checking first-order unifier;
- regularity pruning;
- deterministic fail-first goal and shortest-residue extension ordering; and
- iterative deepening bounded by 12 extensions per branch, 500,000 search
  nodes, and five wall seconds including clausification.

It supports the selected equality-free FOF cluster only. Exhaustion, deadline,
or an unsupported matrix returns `Unknown`; it never makes a negative theorem
claim.

The search certificate contains rule choices rather than a trusted
substitution. [`verify_connection.py`](verify_connection.py) separately
reparses the matrix, reruns clausification byte-for-byte, reimplements
standardization, dereferencing, occurs checks, unification, and proof-state
transitions, and reconstructs every substitution through complete branch
closure.

## Corpus and controls

The frozen corpus is the 12-problem FNE subset of the earlier candidate-blind,
family-separated adaptive-probe manifest:

| Split | Families | Problems |
| --- | --- | ---: |
| train | `GEO`, `CSR`, `NLP` | 4 |
| validation | `LCL` | 4 |
| test | `NUN` | 4 |

Every problem is an equality-free FOF theorem. The problem and included-axiom
hashes are locked in [`corpus.jsonl`](corpus.jsonl). These splits predate the
connection worker, although prior saturation studies mean they are not
globally unseen.

The three five-second single-worker arms are:

1. `connection`;
2. `global_aw`, Umlaut's KBO6 global refined-weight/FIFO control; and
3. `goal_hard_priority`, the stronger KBO6 `PreferGoals` refined-weight/FIFO
   control.

The `independent_portfolio` is the solve union of connection and
goal-priority saturation and represents two workers, not an equal-core
comparison.

No clause exchange was run. An unfinished tableau branch is not a logical
consequence that can soundly enter saturation, while a closed tableau is
already terminal. Proof-producing lemma extraction would be a different
calculus and should not be smuggled into this prototype.

No accumulated ground-SAT service was used: the worker did not expose a stable
ground clause stream that justified the extra mechanism.

## Primary results

The primary train, validation, and clean replicated-test matrix contains 60
coordinates. The audit checked 279 retained artifact files and all nine
theorem-run proof claims. Every held-out repetition agrees on theorem versus
unknown/resource outcome.

| Split | Connection | Global age/weight | Goal priority | Independent portfolio |
| --- | --- | --- | --- | --- |
| train | none | none | `CSR115+98` | `CSR115+98` |
| validation | none | `LCL982+1` | `LCL982+1` | `LCL982+1` |
| test | `NUN081+1` | none | `NUN081+1` | `NUN081+1` |

Thus:

- connection adds `NUN081+1` over the weak global control;
- connection adds nothing over goal-priority saturation;
- connection loses no reproducible solve to goal priority on the clean test
  replication;
- the independent portfolio adds nothing over its goal-priority component; and
- validation contains no connection signal.

The worker times out on the other three test problems. On train it solves none;
three searches reach the five-second search deadline around iterative depth
four, while `NLP262+1` spends the enclosing budget in clausification under the
four-way schedule.

## Common-proof cost

`NUN081+1` is the only common reproducible connection/goal-priority test solve.
Across its two clean test repetitions:

| Measure | Connection | Goal-priority saturation |
| --- | ---: | ---: |
| Proof units | 12 rule nodes | 40 annotated formulas |
| Per-proof units | 6 | 20 |
| Median solve wall | 0.028785 s | 0.013872 s |

The aggregate proof-unit ratio is 0.300, but the paired median wall ratio is
2.083 and only one common problem exists. These different native proof units
are useful architecture evidence, not interchangeable inference counts. They
do not satisfy the preregistered cost alternative.

## Invalid first test execution

The first complete 24-coordinate test execution is preserved rather than
silently replaced. It is invalid under the repetition gate because
goal-priority saturation proves `NUN085+1` once and returns `ResourceOut` once.
Both proof-producing runs in the experiment are externally checked; this is
resource-limit instability, not a proof disagreement.

A complete, unchanged second execution has the same contract hash and is
clean: both `NUN085+1` repetitions return `ResourceOut`. Across all four
attempts, the one-of-four goal-priority solve is not reproducible.
`NUN081+1` remains solved by both connection and goal priority in all four
attempts, so the recovery cannot manufacture complementarity and does not
change the stop decision.

## Correctness and robustness

- 10 focused Python tests pass on Windows and Ubuntu.
- Two hand theorem proofs independently replay; a satisfiable hand case
  returns `Unknown`.
- The integration matrix rejects mutations to the start clause, extension
  clause, extension literal, reduction path, goal diagnostic, goal index,
  fresh-instance identity, transcript, and original problem.
- Every connection theorem reruns CNF byte-for-byte and replays through the
  independent verifier.
- Every saturation theorem passes the repository's positive-only validation
  gate using ProofCheck 1.0.
- Exact reruns resume 12/12 train, 24/24 validation, and 24/24 replicated-test
  coordinates.
- An extra byte in a retained connection certificate makes the artifact audit
  fail; restoring the exact byte sequence restores the clean audit hash.
- No theorem polarity disagreement occurs.

## Engineering and integration decision

The experiment contains 1,233 lines in the parser/common, worker, and
independent-verifier core before orchestration and tests. Even this narrow
prototype omits typed terms, equality, cancellation, production proof-object
integration, native portfolio scheduling, and proof-producing lemma exchange.

That engineering cost is not supported by the measured coverage:

- no solve over the strongest control;
- no portfolio addition;
- one compact common proof, but slower and below the frozen sample-size gate;
  and
- observable saturation resource instability near the five-second boundary.

Do not port this generic worker to Rust, do not add it to production
portfolios, and do not build clause exchange on its open branches. A future
connection-calculus effort should begin only with a different, demonstrably
connection-favorable cluster or a substantially stronger native design and
must establish held-out complementarity before integration work.

## Evidence

- Local ignored archive:
  `.artifacts/experiments/2026-07-30-017-complementary-connection-worker/evidence-v1.tar.gz`
- Archive SHA-256:
  `095fd55e349f037d8c296c16b9055d858f364dcfaf5c8615bae9cec26f60b743`
- Frozen corpus SHA-256:
  `7a58785f6f65e57c469ac406d188aa744ba24e94ce10dd57074ccfeed097a5f4`
- Preregistration SHA-256:
  `af9262b780d8336a9c1f43407bfd11b749e5e2012f092b8164a05e2f6807796d`
- Ubuntu binary SHA-256:
  `8c093b91e7e0de5f37d2f8066199f9b57aaea3a1041f9fa9eb21d116ae1decda`
- Final decision SHA-256:
  `e5d8b33cf5417d891648c5267db5ad7dde79c292ac16f418eb3482c244aa1d39`

Exact commands and result hashes are in [`COMMANDS.md`](COMMANDS.md).

