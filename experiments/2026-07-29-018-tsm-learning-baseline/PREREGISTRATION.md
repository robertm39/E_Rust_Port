# TSM learning baseline preregistration

## Scope and source revision

This experiment addresses Bead `E_Rust_Port-9jt.3.3`. The measured prover
revision is `812323618aaa42d0f5e24bba8a0ef146ff1757cd`, including the generated-KB
activation and persistence fixes.

This revision supersedes the initially frozen
`9263a0d362d5ec297f9e5305870b641626de107e`. An invalid training-only attempt
under that revision observed two fixed-control `ResourceOut` results
(`CSR052+3`, `CSR039+2`) and one theorem trace (`MGT067+1`), then failed while
integrating the first proof into the knowledge base. No learned treatment,
validation, test, or classifier result was observed. The invalid artifacts are
not reused. This amendment changes only the source revision: the corpus,
whole-family split, treatments, budgets, metrics, gates, and frozen decision
rule remain unchanged.

After the held-out search runs completed, classifier preparation found that the
search harness had retained TSTP proof text while `umlaut-kb-ginsert` requires
PCL. Search outcomes and telemetry are not rerun or replaced. For label
extraction only, each already-successful repetition-1 control coordinate is
re-executed with its recorded command, changing only `--tstp-out` to
`--pcl-out` and redirecting telemetry to a separate file. The rerun must return
the same SZS status before its trace is admitted. No failed coordinate,
candidate coordinate, family, budget, metric, gate, or threshold changes.

The first PCL extraction attempt under the measured revision was also invalid.
It inserted three validation proofs, then `ROB005-1` exposed negative internal
clause identifiers in the PCL justification field. No classifier metric or
final analysis was observed. Investigation found that recorded-GC cleanup
removed a contracted-away archive entry by its non-unique visible identifier,
which could instead delete an older proof parent with the same identifier.
Production fix `477fa727355bace7de39d043d9b18734bd16adf4` removes the exact stable
derivation identity and passed both a collision regression and a real
`ROB005-1` PCL-to-KB round trip.

That fix is used only as the label-extraction prover revision. The measured
search revision remains `812323618aaa42d0f5e24bba8a0ef146ff1757cd`; its
training, validation, test, telemetry, and search outcomes are not rerun or
replaced. A fresh classifier-input root re-executes only already-successful
repetition-1 controls from their recorded commands, changes TSTP output to PCL,
redirects telemetry, and requires the same SZS status. The label-extraction
revision and executable hash are recorded in classifier metadata, and all new
traces are written below that fresh root so the failed extraction evidence
remains immutable. No family, treatment, budget, metric, gate, or threshold
changes.

The patched extraction then found that all eight frozen test repetition-1
controls ended `ResourceOut`, so no successful control proof exists from which
to derive test labels. No classifier run or metric was observed before adding
explicit missing-coverage reporting. The controller records that split as
unavailable, does not fabricate negative labels or a classifier workload, and
the analyzer applies the already-frozen `uncertain` rule for insufficient
classifier coverage. Validation extraction and all search evidence are
unchanged; the failed partial extraction root is preserved and the reporting
rerun uses a fresh root.

The production audit found these reachable paths:

1. `umlaut --record-gcs --pcl-out` emits proof and given-clause traces.
2. `umlaut-direct-examples` derives positive proof clauses and sampled negative
   clauses.
3. `umlaut-kb-create`, `umlaut-kb-ginsert`, and `umlaut-kb-insert` maintain the
   `description`, `signature`, `problems`, `clausepatterns`, and `FILES/*`
   knowledge-base formats.
4. `TSMWeight` and `TSMRWeight` are live WFCBs. `UseTSM1` and `UseTSM2` are
   opt-in built-in heuristics; no automatic schedule references them.
5. `umlaut-tsm-classify` consumes `Training:` and `Test:` annotated-term sets
   with two-value `(source_count, class)` annotations.

The historical performance evidence in experiment 059 covers only a trivial
`$cnil` KB. This experiment is the first nonempty proof-derived activation and
held-out search study.

## Falsifiable hypothesis

A proof-derived TSM trained only on CASC-30 training families can rank
structural clause patterns cheaply enough to run in process, show held-out
class signal after calibration, and improve or complement an otherwise
structure-matched clause-selection heuristic.

The result may reject that hypothesis. No automatic schedule or default is
changed unless every adoption gate below passes.

## Frozen corpus and leakage controls

`select_corpus.py` selects only first-order `FNE`, `FEQ`, `EPU`, and `UEQ`
theorem/unsatisfiable problems from the immutable
`benchmarks/casc_2025_manifest.jsonl` whole-family partition.

- Training: four q1/q2 ordinal-proxy problems per category, 16 total.
- Validation: two q1-q4 problems per category, 8 total.
- Test: two q1-q4 problems per category, 8 total.
- Selection is SHA-256 ranked with salt `umlaut-tsm-family-heldout-v1`.
- Train, validation, and test family sets must be pairwise disjoint.
- Problem bytes and included axioms are checked against the source manifest.

Only the fixed non-learning control generates training and held-out
classification labels. Candidate runs never contribute labels or KB entries.
Validation and test are reported separately; neither may alter the fixed TSM
weights, index, queue ratios, or decision thresholds.

## Training and treatments

Training traces use the deterministic fixed heuristic
`(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))`,
KBO6, forward-demodulation level 2, an 8-second soft / 10-second hard CPU
budget, and complete PCL given-clause recording. Only successful proof traces
are inserted. The KB is valid only with at least four solved training problems,
two represented categories, both positive and negative pattern labels, and no
held-out family.

Search treatments have identical four-queue schedules:

- `control`: the TSM slot is
  `Clauseweight(ConstPrio,1,1,2)`;
- `learned`: that slot alone becomes the proof-derived
  `TSMWeight(ConstPrio,1,1,2,flat,KB,100000,1,1,Flat,IndexIdentity,100000,-20,20,-2,-1,0,2)`.

Both retain the two goal/non-goal `Refinedweight` queues, the same FIFO queue,
ratios `10:10:5:1`, KBO6, and forward-demodulation level 2. Each validation and
test coordinate receives two deterministic repetitions at 8 seconds soft /
10 seconds hard CPU with search telemetry and proof objects.

## Ranking and calibration measurements

The training KB patterns are converted to two-class annotated terms by
aggregating source counts and labeling a pattern positive when at least half of
its occurrences are proof occurrences. Validation/test labels are built from
successful control traces only.

`umlaut-tsm-classify` scores training-as-test and each held-out set. A
one-dimensional logistic calibrator is fit only to training scores and labels.
Held-out reports include weighted accuracy, balanced accuracy, Brier score,
expected calibration error with ten equal-width probability bins, constant
training-prior Brier score, label balance, and unmappable/parser failures.

Ranking cost is the median of five timed classifier repetitions after one
warm-up, reported as whole-process CPU and wall microseconds per held-out
pattern. This is a conservative upper bound because it includes parsing and TSM
construction. Search telemetry additionally reports common-coordinate CPU,
processed-clause throughput, and maximum RSS.

## Correctness and validity gates

The experiment is valid only if:

- all source, corpus, executable, KB, contract, and raw-result hashes verify;
- every run produces an SZS status and either telemetry or a documented hard
  resource stop;
- candidate and control report no status outside the expected proof class;
- TSM loading has zero panic, signature-code, parser, or unmapped-term fatal
  failures;
- at least four training proofs from at least two categories enter the KB;
- each reported classifier split has both labels and at least 20 weighted test
  patterns; and
- every claimed unique or lost proof is retained for independent checking.

## Frozen decision rule

`continue` requires all correctness gates plus:

1. held-out balanced accuracy above `0.55`;
2. held-out calibrated Brier score below the constant-prior Brier score and
   ECE at most `0.20`;
3. ranking CPU below `50` microseconds per weighted pattern;
4. no reproducible control solve lost on test; and
5. either at least one reproducible learned-only test solve, or a common-solve
   median CPU ratio at most `0.95` on both validation and test.

`stop` is selected for a correctness failure, a reproducible lost test solve,
ranking cost above `100` microseconds per pattern, or valid evidence that fails
the solve/calibration gates. Marginal sample coverage, one-repeat-only solves,
or invalid timing yields `uncertain`.

Failure classes are reported explicitly: training proof scarcity, label
imbalance, parser/load/signature failure, unmapped pattern, wrong status,
telemetry failure, learned-only solve, control-only solve, common timeout,
common solve, CPU/RSS regression, and calibration failure.
