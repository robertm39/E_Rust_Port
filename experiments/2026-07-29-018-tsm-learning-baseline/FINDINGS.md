# TSM learning baseline findings

## Outcome

Experiment 018 satisfies the investigation scope for Bead
`E_Rust_Port-9jt.3.3`, but it does not justify enabling proof-derived TSM
guidance in an automatic schedule.

The frozen decision is `uncertain` because the test split produced no
successful repetition-1 control proof and therefore no non-leaking test
labels. Correctness passes, no reproducible test solve is lost, and the learned
strategy retains all four reproducible validation solves. The available cost
evidence is unfavorable:

- median validation classifier CPU cost is 400.32 microseconds per weighted
  pattern, above the 50-microsecond adoption gate and the 100-microsecond stop
  boundary;
- median learned/control CPU on the eight common solved validation coordinates
  is 6.648;
- median learned/control maximum RSS is 1.424;
- median learned/control processed-clause ratio is 1.112;
- neither strategy reproducibly solves a test problem at the frozen budget.

The production effect is
`leave_tsm_out_of_automatic_schedules`. This exact TSM formulation should be
deferred unless a follow-up first demonstrates much cheaper ranking and enough
proof coverage to create two-class held-out test labels.

## Evidence identity

| Evidence | Identity |
| --- | --- |
| Measured search revision | `812323618aaa42d0f5e24bba8a0ef146ff1757cd` |
| Search binary SHA-256 | `82db6c558f64d24b46e7b9eb5562b803874a3653d8a1ee99d0ec378d8449802d` |
| Search-source archive SHA-256 | `c92dac44515014857ef5538d28b539dbf6a9fc4bf46fc38d3f90aaed6cf0226b` |
| Label-extraction revision | `477fa727355bace7de39d043d9b18734bd16adf4` |
| Label-extraction binary SHA-256 | `22abd227725da25af6143ae4f3159a05ccd477bd0f00d0aa955c49f7392aecd8` |
| Classifier revision | `fc72bb24ee57fd796b19657621c0ff32c2afc4a5` |
| Classifier binary SHA-256 | `7e19de722558fc71c3fb890bd1996bd21e782aef34bc8b9552033ced0e89364c` |
| Corpus SHA-256 | `28b6ac9d59d2871877a7b784b41bc70fe5c09386da6214123791e660819b67c1` |
| Final preregistration SHA-256 | `c362b61591089a33e62b8486e63e0b99008ea1ef4ace3b2982cbc599b6a8e3d2` |
| Knowledge-base tree SHA-256 | `838a4f14137344c8d1c0c17a0503fb8fc0a136dbcb206b35f6927c898fe7d13f` |
| Final analysis SHA-256 | `60fedc2ad05323a1f110fe1fd324ee18a300f5d8ff1acce541cca71e6abe35ff` |

The complete 12,702,874-byte raw artifact archive is ignored at
`.artifacts/experiments/2026-07-29-018-tsm-learning-baseline/tsm-learning-018-81232361-complete.tar.gz`.
Its SHA-256 is
`8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156`.
It contains every valid result, all disclosed invalid/partial roots, diagnostic
traces, classifier inputs and outputs, and the final analysis.

## Reachable learning paths and formats

The production audit established these live paths:

1. `umlaut --record-gcs --pcl-out` emits proof and given-clause PCL traces.
2. `umlaut-direct-examples` derives proof-positive and sampled-negative clause
   examples.
3. `umlaut-kb-create`, `umlaut-kb-ginsert`, and `umlaut-kb-insert` maintain
   `description`, `signature`, `problems`, `clausepatterns`, and `FILES/*`.
4. `TSMWeight` and `TSMRWeight` are live weight-function control blocks.
   `UseTSM1` and `UseTSM2` are opt-in built-ins and are absent from automatic
   schedules.
5. `umlaut-tsm-classify` consumes `Training:` and `Test:` annotated-term sets
   with `(source_count, class)` annotations and now accepts the recursive
   `$or`/`$cnil` clause-pattern surface emitted by the KB tools.

The activated KB path was verified with a real proof, PCL-to-KB insertion, and
live `TSMWeight` loading. Production repairs made during the investigation are:

- `81232361`: preserve generated-KB activation, normalized untyped patterns,
  sparse-signature helper codes, persisted recursive patterns, and KB
  persistence;
- `477fa727`: remove recorded-GC archive entries by exact derivation identity,
  preserving PCL proof parents;
- `9e15487c`: parse KB clause-pattern syntax in `umlaut-tsm-classify`;
- `fc72bb24`: reserve fixed helper-symbol codes before classifier pattern
  symbols.

The final production gate passed 4,498 tests and warning-free all-target,
all-feature Clippy on Ubuntu with pinned CaDiCaL 3.0.1.

## Corpus, labels, and non-leakage

The tracked corpus uses whole-family-disjoint train, validation, and test
splits across FNE, FEQ, EPU, and UEQ.

- Training attempted 16 fixed-control problems and inserted 5 successful
  proofs spanning all four categories.
- The training classifier set has 132 unique patterns and 224 weighted
  occurrences: 167 positive and 57 negative.
- Validation has 126 unique patterns and 150 weighted occurrences derived only
  from four successful repetition-1 control proofs. All are positive, so
  balanced accuracy is undefined.
- All eight test repetition-1 controls ended `ResourceOut`. The test label set
  and classifier workload are explicitly unavailable; no negative labels were
  fabricated.
- Candidate runs never contributed a label or KB entry.

## Classification and calibration

The classifier completed one warm-up and five measured repetitions for the
training-self and validation workloads. Output was byte-identical across
repetitions and every stderr hash was the empty-file SHA-256.

Validation is explicitly reported as `single_class`:

- accuracy and positive recall: 1.0;
- negative recall and balanced accuracy: unavailable;
- calibrated Brier score: `2.8715565563719188e-08`;
- constant training-prior Brier score: `0.06475207270408162`;
- expected calibration error: `0.0001671829912108791`;
- median CPU time: `0.060048` seconds;
- median wall time: `0.060212616` seconds;
- CPU and wall relative ranges: `0.01039` and `0.01046`.

These calibration values describe positive-only validation evidence and must
not be interpreted as two-class discrimination. Test calibration and ranking
cost are unavailable.

## Held-out search

The original immutable control root supplies 16 results per split. The first
learned root is invalid because all candidate processes stopped in option
parsing before loading a problem. After the preregistered path-only correction,
the fresh learned root supplies exactly 16 results per split using the original
search binary.

Validation:

- control: 2 `Theorem`, 6 `Unsatisfiable`, 8 `ResourceOut`;
- learned: 2 `Theorem`, 6 `Unsatisfiable`, 8 `ResourceOut`;
- both strategies reproducibly solve `LCL026-10`, `LCL365+1`, `PUZ037-2`, and
  `ROB005-1`;
- no learned-only, control-only, or one-repetition-only solve remains;
- all 32 combined results have valid status/telemetry semantics, with no bad
  status, KB-load failure, or telemetry failure.

Test:

- control: 16 `ResourceOut`;
- learned: 16 `ResourceOut`;
- neither strategy solves a coordinate;
- all 32 combined results pass correctness and telemetry checks.

The learned-only validation and test contract IDs are
`4c0a9d89d2164c1e9e858ef60b7c276b24fe1620911bf29e38859cb15a649bc0`
and
`4467aee33570458f78cdc4d15c95034575f2d9e15de80646cc11b4cab49ac4bc`.

## Failure classes and amendments

All invalid artifacts remain in the complete archive. No invalid result was
silently reused.

- The initial measured revision activated TSM but could not persist a
  nontrivial generated KB; the source revision was amended before held-out
  observation.
- Search proof output was TSTP while label insertion requires PCL; label-only
  reruns were isolated and required the same SZS status.
- Recorded-GC cleanup could delete a proof parent with the same visible clause
  identifier; the exact-derivation production repair was isolated to label
  extraction.
- Test controls yielded no proofs, making test labels unavailable.
- The classifier harness first used unsupported `Identity` instead of
  `IndexIdentity`.
- The classifier then rejected recursive negated KB syntax until routed through
  the clause-pattern parser.
- A lightweight classifier signature allowed a normalized symbol to collide
  with reserved helper code 17 until internal codes were reserved.
- Validation labels were one-class; the analyzer reports one-sided metrics and
  leaves balanced accuracy unavailable.
- Learned search paths were initially single-quoted inside `TSMWeight(...)`;
  those processes never entered search. Only the zero-evidence learned
  coordinates were rerun with an unquoted filename-token path.
- The first corrected-path smoke used a later mutable binary. It was invalidated
  before the full rerun, and the controller now rejects any binary hash other
  than the original search hash.

## Reproduction

All Rust builds, tests, classifier executions, and prover runs were performed
on the Ubuntu Linode runner. The tracked controllers are the authoritative
commands:

- `build_kb.py` builds the training KB;
- `run.py` executes frozen search coordinates and can isolate the learned
  strategy with `--strategy learned`;
- `make_classifier_inputs.py` builds control-only labels;
- `classify.py` runs and times the frozen classifier workloads;
- `analyze.py` combines separate immutable control and learned roots with
  `--learned-search-root`.

The final analyzer used:

```text
python3 analyze.py \
  --classifier-input-root classifier-inputs-v4 \
  --classifier-output-root classifier-output-v7 \
  --search-root search \
  --learned-search-root search-learned-v2 \
  --output analysis-v9.json
```

The analyzed output prints `verdict: uncertain`, passes correctness and
no-reproducible-test-loss, and leaves TSM out of automatic schedules.
