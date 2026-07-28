# Fingerprint-index retrieval bake-off

Bead: `E_Rust_Port-9jt.7.3`

## Question and preregistration

Can an existing fingerprint or perfect-discrimination-tree variant replace
the default `FP7` retrieval index for backward rewriting and paramodulation,
while preserving exact retrieval results and improving held-out end-to-end
search with bounded memory?

This section is preregistered before benchmark results are inspected.

## Correctness replay

The repository test
`all_index_variants_replay_dynamic_workload_with_exact_result_sets` exercises
every selectable fingerprint/discrimination variant over a mixed shallow,
deep, wide, variable, repeated-variable, insert, and delete workload. For
both matchable and unifiable queries, it filters the index candidates with
the production exact matcher and compares the resulting term IDs with a slow
scan of every live term. Any missing or additional exact result fails the
test. This protects retrieval completeness independently of end-to-end SZS
outcomes.

## Workload telemetry

Schema-version-1 search telemetry gains additive fingerprint fields for:

- insertions and deletions;
- match/unification query, compatible-leaf, and candidate-term counts;
- candidate and exact-unifiable paramodulation terms; and
- final nodes, payload leaves, and entries for all four global fingerprint
  structures.

The existing backward-rewrite attempt/success counters supply exact match
precision. CPU, resident pages, term storage, generated clauses, and
high-water clauses remain the end-to-end cost and memory authorities.
Telemetry candidate counting and atomic updates are active only under the
existing scoped `--search-telemetry` guard.

## Corpus and split

The immutable CASC-30 manifest and source-family partition are reused. No
family crosses train, validation, and test. Each phase contains six problems
from each of:

- FEQ, theorem problems with equality;
- UEQ, unsatisfiable unit-equality problems; and
- FNE, theorem problems without equality as an overhead control.

Calibration has 18 train problems, validation has 18 validation problems, and
the held-out test has 18 test problems.

## Fixed search and candidates

Every run fixes KBO6, full forward demodulation, and the same
`5*Refinedweight + 1*FIFO` given-clause policy. A candidate changes all three
user-selectable global index roles together. Calibration compares default
`FP7` with `FP0`, `FP1`, `FP2`, `FP3D`, `FP3W`, `FP4M`, `FP7M`,
`FP4X2_2`, and `NPDT`.

Calibration ranks candidates without examining validation or test and
advances three. Validation uses two repetitions and selects one without
examining test. The held-out test compares the frozen winner with `FP7` at
5- and 20-second soft CPU budgets, with two repetitions and proof objects.

## Metrics and decision rule

The report will include:

- reproducible coverage, unique solves, and proof/model polarity;
- paired CPU, generated, processed, high-water, term-storage, and RSS ratios;
- query volume, candidates per query, exact-success precision, updates, and
  final structure sizes by category;
- crossover regimes associated with query selectivity and structure size; and
- independent ProofCheck verdicts for all reproducible larger-budget proof
  claims.

The selected variant is integrated into defaults only if exact replay remains
green, every checked proof verifies, no reproducible held-out coverage is
lost, maximum RSS is at most 1.05 of `FP7`, and either:

1. it has at least two selected-only reproducible solves and no baseline-only
   solve; or
2. paired held-out median CPU is at most 0.95 of `FP7`, with generated and
   high-water clauses no worse than 1.02.

Otherwise `FP7` remains the default. A category-specific crossover alone is
reported as evidence, not integrated as an unvalidated dispatcher.

## Results

### Reproducible execution

The authoritative run used Ubuntu 24.04 runner
`e-rust-codex-260728-145514-6af9` and release binary
`564789b688591b0a1127650fe85f54121e398075010c55eb57d5b274f053a6f9`.
All 468 coordinates completed:

- 180 calibration runs under contract
  `eb48f86fc26972517f2971ac00a68a76b0f4c3068140765d8d3b9de07c2428b7`;
- 144 validation runs under contract
  `34055089860ed76d7532c8c03c616ec77b6b94ca02ca6656812f7e9acd0d99f0`;
  and
- 144 held-out test runs under contract
  `164bb35125aba4d1ce97a2c100251fc530ed440f309dfb9a2b5b784bf662ec72`.

A second invocation hash-validated and resumed 180/180, 144/144, and 144/144
coordinates. The exact calibration controller is preserved separately as
`run_calibration.py` with its contract-recorded SHA-256
`46d83097ef9a1f23ff1fd2401d7a9606d186379ea1824ea9427f14e71c53c3e1`;
validation and test use `run.py` at SHA-256
`37a9faaaf9e38d40f6bacfa572b3b3cb61cbb3cb993dc8708be17c92c6d96eba`.

The ignored raw archive is
`.artifacts/index-retrieval/index-retrieval-raw.tar.gz`. It is 15,608,944
bytes with SHA-256
`9a294e25d7d6c2e64676e57aa02d10feaed447db55409b94d88a8969cb2c369e`.
The compact tracked summary has SHA-256
`f831d6c380b240fbf3f68277d10d4f52657f885abe1257b8cc59d75684da7f0b`;
the full ignored summary has SHA-256
`d28e2375979d619a78189babbadbbde478f922273ea5a4b3a5befbbe4cc46ca7`.

### Staged selection

Calibration advanced `FP3D`, `NPDT`, and `FP2`. Every candidate except the
unselective `FP0` slow filter reproducibly solved the same one calibration
problem, so CPU broke the tie. `FP0` solved none.

On disjoint validation, `NPDT` reproducibly solved five problems. `FP3D`,
`FP2`, and default `FP7` each solved four. The additional `NPDT` solve was
`PUZ039-10`; its median solved CPU was 1.410702 seconds, versus 0.735716 for
`FP3D`, 0.744443 for `FP2`, and 0.817906 for `FP7`. Its median solved maximum
resident set was also 111,338 pages, versus 83,930 for `FP7`. The sealed
coverage-first rule nevertheless advanced `NPDT` to test, preventing an
after-the-fact rejection of its validation-only solve.

### Held-out result and crossover regimes

At both held-out budgets, `NPDT` and `FP7` reproducibly solved exactly
`NUN086+2`, `NUN134-1`, and `REL005-1`. The validation-only solve did not
generalize. On the six paired larger-budget solved coordinates, `NPDT/FP7`
ratios were:

- CPU 1.004952;
- generated and high-water clauses 1.0;
- maximum resident pages 0.994903;
- unification candidates per query 0.994926; and
- index nodes 0.897498.

Across all 36 larger-budget coordinates, CPU was 1.007903, RSS 1.000766, and
unification candidates per query 0.992161. The result exposes a genuine
structure-shape crossover without an end-to-end win:

- FEQ: 0.654792 as many nodes, 0.991977 candidates/query, and 1.021802 CPU;
- FNE: 0.693561 as many nodes, 0.993407 candidates/query, and 1.006630 CPU;
  and
- UEQ: 2.566308 as many nodes, 0.991302 candidates/query, and 1.005825 CPU.

`NPDT` greatly reduced backward-rewrite candidate counts on several workloads,
but fingerprint paramodulation filtering was already precise: its exact
paramodulation precision improved only about one percent. Extra
discrimination-tree traversal and the UEQ node expansion consumed the
filtering benefit. This evidence does not support either a global `NPDT`
default or a category dispatcher.

### Soundness and decision

The dynamic replay test proved exact matchable and unifiable result-set
equivalence for all selectable variants. ProofCheck 1.0 self-certified and
verified all 6/6 reproducible larger-budget proof claims. There were zero
proof/model polarity disagreements. The proof-validation report ID is
`796b56b0cd4f470947004ab68cf6f4c9baa272ec83e7493e2f8a47dc4fc12e0e`;
its file SHA-256 is
`c375c7523183fa3c000c2e34bdf12ff9a736675910742a6698ce84bc54bfd6f7`.

`NPDT` had no held-out unique solve and missed the required 0.95 CPU ratio.
The decision is therefore `retain_fp7_default`. No index implementation or
dispatcher is integrated from this study; the additive, opt-in workload
telemetry and exact replay regression remain available for future index work.
The final report ID is
`b73b6ea7b2d77731daab3a58b5b8da8aca797569cc528a8432c358cf56c7a823`.
