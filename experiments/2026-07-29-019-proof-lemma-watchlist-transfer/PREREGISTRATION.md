# Proof-derived lemma and watchlist transfer preregistration

## Scope and frozen evidence

This experiment addresses Bead `E_Rust_Port-9jt.3.5`. It measures the live
`umlaut-pcl-lemma` selector, inline static watchlists, `PreferWatchlist`
guidance, ordinary lemma input, PCL proof output, and search telemetry. No
automatic schedule or default is changed by this experiment.

The measured prover source revision is
`ce75ea3b68c34ab1640e0f362438a656626a5b0e`. All Rust builds, prover runs, and
PCL selection runs execute on the pinned Ubuntu Linode runner with CaDiCaL
3.0.1. The executable hashes and experiment-script hashes are recorded in the
raw contracts.

The frozen source evidence is:

- experiment 018 corpus:
  `experiments/2026-07-29-018-tsm-learning-baseline/corpus.jsonl`, SHA-256
  `28b6ac9d59d2871877a7b784b41bc70fe5c09386da6214123791e660819b67c1`;
- complete experiment 018 raw archive, SHA-256
  `8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156`;
- five successful fixed-control training traces, and no failed trace:
  `MGT067+1`, `SWW967+1`, `LAT265-2`, `KLE145-10`, and `SYN563-10`.

The source traces were produced only from experiment 018 training families.
Their archived result records, proof status, problem hash, and trace hash must
verify before selection. The preparation controller rejects any other source
problem or archive.

## Falsifiable hypothesis

Small, deterministic sets of intermediate clauses selected from successful
training proofs can improve family-held-out search either as soundness-neutral
watchlist progress signals or as target-entailed explicit lemmas. The benefit
may be limited to same-category transfer; cross-category transfer is expected
to be harder and is measured separately.

The result may reject this hypothesis. In particular, a zero-overlap
watchlist, no independently admissible explicit lemmas, increased selection
cost, longer proofs, or lost solves is useful negative evidence.

## Corpus and leakage controls

The experiment reuses the 32 first-order FNE, FEQ, EPU, and UEQ problems from
experiment 018:

- 16 training problems, used only through the five already-successful PCL
  traces named above;
- 8 validation problems;
- 8 test problems.

Train, validation, and test source-family sets are pairwise disjoint. The
preparation controller verifies this property, every problem hash, every
included-axiom path, the corpus hash, and the source archive hash.

Candidate selection sees only the five frozen training traces. Candidate
ordering is a SHA-256 order over the selection salt, transfer mode, source
problem, selected-record index, and clause body. It does not use held-out
status, timing, proof, telemetry, problem identity, or symbol overlap.

Two transfer pools are built for every target:

- `same`: candidates whose source problem has the target's TPTP category;
- `cross`: candidates whose source problem has a different TPTP category.

Both are family-held-out. `cross` additionally prevents same-category source
evidence. Validation and test use the same frozen construction; validation
results cannot change test candidates, budgets, strategies, or thresholds.

Candidate clauses, target wrappers, and validation certificates are content
hashed. Candidate or result material is never written into the tracked corpus.

## Candidate extraction

Each of the five training PCL traces is processed once with:

```text
umlaut-pcl-lemma
  --flat-lemmas
  --max-lemmas=8
  --min-lemma-quality=0
  --tstp-out
  --output-level=1
```

Only complete clause-valued `cnf` or `tcf` lemma records are accepted. The
selector status text and formula-valued proof owners are not candidates
because inline first-order watchlists accept represented clauses. Empty-clause
candidates are removed because they would terminate or trivialize the target.
Exact duplicate bodies are deduplicated within each target/mode pool while
retaining all source provenance. At most 16 deterministically ordered
candidates enter either pool.

Preparation records selector return code, output hashes, selected-record
count, wall time, and child CPU time for each trace. The sum and per-source
distribution are the offline selection-overhead measurements.

## Why explicit lemmas require a safety gate

A clause derived from one problem is not automatically a theorem of an
unrelated target. Injecting it as an axiom without further evidence would make
the comparison unsound.

For every target and transfer mode, the controller therefore tries candidates
in frozen order until four are admitted or all 16 are exhausted. It constructs
an axiom-only version of the target by retaining includes and all ordinary
axiom/definition/hypothesis records while removing `conjecture`,
`negated_conjecture`, and `question` records. The candidate body is appended as
a fresh FOF conjecture and run with the frozen control strategy at a 1-second
soft / 2-second hard CPU limit.

A candidate is admitted for explicit injection only when that independent
problem returns `Theorem`, `Unsatisfiable`, or `ContradictoryAxioms`. The
complete PCL certificate, command, timing, status, and hashes are preserved.
Unknown, resource-limited, malformed, or contrary candidates are rejected.

This target-side re-proof is part of explicit-lemma selection overhead. It may
use target axioms but never the target conjecture or any target search result.

## Treatments

All five treatments use KBO6, forward-demodulation level 2, complete PCL proof
objects, forced derivation retention level 2, and the same four-queue schedule.
The base queues and ratios are:

```text
10 * Refinedweight(PreferGoals,1,2,2,2,0.5)
10 * Refinedweight(PreferNonGoals,2,1,2,2,2)
 5 * OrientLMaxWeight(PRIORITY,2,1,2,1,1)
 1 * FIFOWeight(PRIORITY)
```

The treatments are:

- `control`: `PRIORITY=ConstPrio`, original problem;
- `watch_same`: `PRIORITY=PreferWatchlist`, original problem plus an inline
  static `same` watchlist;
- `lemma_same`: `PRIORITY=ConstPrio`, original problem plus up to four
  independently admitted `same` lemmas;
- `watch_cross`: `PRIORITY=PreferWatchlist`, original problem plus an inline
  static `cross` watchlist;
- `lemma_cross`: `PRIORITY=ConstPrio`, original problem plus up to four
  independently admitted `cross` lemmas.

Watchlist wrappers contain the selected candidate bodies with role
`watchlist`. Static watchlists cannot terminate the search and never become
logical premises. Explicit wrappers contain only independently admitted
candidate bodies with role `lemma`. A wrapper first includes the untouched
original target problem.

Each validation and test coordinate receives two deterministic repetitions at
an 8-second soft / 10-second hard CPU limit and 1,536 MiB memory limit. Four
workers run on the dedicated runner. Job order is contract-hashed. Every
treatment is run even when its candidate count is zero; this makes zero-value
and wrapper overhead observable.

## Measurements

The final analysis reports validation and test separately and includes:

- selector CPU and wall time per source trace and in total;
- explicit admissibility attempts, accepted/rejected clauses, CPU and wall
  time per target and in total;
- watchlist guidance-clause count and explicit added-clause count per target;
- statuses and reproducible solves, including treatment-only and control-only
  solves;
- common-solve total CPU, maximum RSS, generated clauses, processed clauses,
  and processed throughput;
- PCL proof-step count and treatment/control proof-step ratio on common solved
  repetition coordinates;
- watchlist hit evidence from PCL/watchlist documentation when present;
- total explicit cost with one-time admissibility CPU amortized over the two
  repetitions;
- corpus, family, source-trace, candidate, wrapper, certificate, executable,
  contract, stdout, stderr, telemetry, and proof hashes.

PCL proof steps are non-comment protocol records matching the UPCL2 step
surface. Missing proof objects on a proof status are a validity failure.

Watchlist proofs cannot depend logically on watchlist clauses. Explicit proofs
are accompanied by target-axiom admissibility certificates for every added
lemma. Any reproducible treatment-only solve is retained in full for focused
independent replay before an adoption decision.

## Correctness and validity gates

The experiment is valid only if:

- all frozen source hashes, corpus hashes, family partitions, executable
  hashes, wrapper hashes, and contracts verify;
- candidate extraction succeeds for every admitted source trace;
- candidate material comes only from the five named training traces;
- every explicit added clause has a successful target-axiom certificate;
- every run emits an SZS status and either telemetry or a documented hard
  resource stop;
- no treatment reports a status outside the target's expected proof class;
- every proof status has a nonempty PCL proof protocol;
- validation and test each contain exactly 8 problems × 5 treatments × 2
  repetitions; and
- every claimed reproducible treatment-only proof is retained and passes the
  focused replay gate.

## Frozen decision rule

Decisions are made separately for `watch_same`, `lemma_same`, `watch_cross`,
and `lemma_cross`.

`adopt` requires all correctness gates, no reproducible test solve lost, and
one of:

1. at least one reproducible treatment-only test solve with every such proof
   replayed successfully; or
2. at least four common solved test repetition coordinates, median
   treatment/control CPU at most `0.95`, and median proof-step ratio at most
   `0.95`.

For explicit lemmas, the CPU comparison includes target-specific
admissibility CPU amortized over the two repetitions. Adoption also requires a
nonzero admitted-clause count.

`stop` is selected for a correctness/soundness failure, any reproducible lost
test solve, or sufficient common-solve evidence with both median CPU and proof
steps at least `1.05` of control. A treatment with zero effective clauses,
zero unique solves, and no common-solve improvement is also `stop_no_value`.

All other outcomes are `uncertain`, including fewer than four common solved
test repetition coordinates, one-repetition-only solve changes, or unstable
timing. No automatic schedule changes unless an `adopt` decision is reached.
