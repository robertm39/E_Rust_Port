# Experiment 330: Layered clause selection and Limited Resource Strategy

## Question

Can an explicit layered given-clause policy add reproducible held-out coverage or
efficiency beyond Umlaut's global age/weight and scalar goal-relevance controls?
Is Vampire's Limited Resource Strategy (LRS) a sound architectural candidate for
Umlaut's current saturation loop?

The preregistered advance rule was strict: validation selects one layered
candidate, and that fixed candidate advances only if it produces at least two
reproducible test-only solves, no contradictory SZS statuses, and no
schedule-fairness bound violations.

## Research and architecture decision

The published layered-clause-selection design places preferred clauses in a
specialized queue as well as broader queues, then interleaves the queues at a
configured layer ratio. Its intended benefit is to preserve attention for a
feature-defined subset without permanently excluding the rest:

- [Layered Clause Selection for Theory Reasoning](https://www.eprover.org/EVENTS/PAAR-2020/papers2020/PAAR_2020_paper_11.pdf)
- [Layered Theory Reasoning in Vampire](https://arxiv.org/abs/2001.09705)
- [First-Order Theorem Proving and Vampire](https://lara.epfl.ch/w/_media/fv20/vampirepaper.pdf)

Umlaut already has the required mechanism. Its heuristic control block
interleaves weighted evaluation queues fairly, and its priority functions
partition preferred, normal, and deferred clauses inside each queue. A
`PreferGoals`, `PreferHorn`, or `PreferUnits` queue plus a `ConstPrio` global
queue therefore reproduces the relevant two-layer shape without a new selector
architecture.

LRS is different. The published strategy estimates which unprocessed clauses an
Otter-style loop can activate in the remaining time and dynamically restricts
that set:

- [Limited Resource Strategy in Resolution Theorem Proving](https://www.sciencedirect.com/science/article/pii/S0747717103000403)

Umlaut inherits E's DISCOUNT loop. A direct LRS port would therefore require a
different activation/deletion owner rather than merely another heuristic queue.
The experiment includes Umlaut's existing `--delete-bad-limit=1000000` only as
a falsification control for aggressive passive pruning. It is not labeled LRS.

## Implementation

Opt-in search telemetry now records the active heuristic control block's exact
queue schedule and observed selection behavior:

- schedule quota and scheduled selections per evaluation queue;
- preferred, normal, deferred, and empty/orphaned selections;
- preferred-clause bypass steps and maximum preferred wait;
- maximum observed schedule gap.

Instrumentation is enabled only by `--search-telemetry`. The disabled path
stores no telemetry object, performs no queue inspection, and allocates nothing.
The existing additive telemetry schema also now reports non-redundant deletion
accounting. Unit tests pin the exact `2:1` schedule, maximum queue gaps, and
preferred-clause wait.

The experiment scripts provide:

- immutable manifest, corpus, binary, harness, strategy, and resource hashes in
  one canonical JSON-normalized contract;
- deterministic family-disjoint stratum selection;
- atomic per-run output and hash-validated exact resume;
- independent SZS classification for theorem/unsatisfiable and
  non-theorem/satisfiable manifest classes;
- explicit capture of missing or interrupted telemetry without converting a
  hard-stopped prover into a harness failure;
- strict coordinate, artifact-hash, telemetry-schema, and schedule-total
  verification before analysis.

## Final contract

The authoritative run used normal-profile Ubuntu 24.04 runner
`e-rust-codex-260728-124835-e88e` (run ID `260728-124835-e88e`) and source
snapshot SHA-256
`6d5cc7cd736eb7e1c6febd8c229f0731a68cc3f6d10862cc27159b49750b9d9f`.
The quality-gated release binary SHA-256 was
`bfa6905a29c80c50420279ded641d46f0517de03ea85a9f4c28140a0c9065ea0`.

The immutable CASC-30 manifest SHA-256 was
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`.
The separately transferred ignored corpus archive SHA-256 was
`efcebc55298d4c6770113c095e8cefdd77b9e8cbe3afa3078201f541893d1a7d`;
safe extraction reverified all 2,901 problems and 2,425 axioms.

The matrix selected 44 problems:

| Split | FNE theorem | FEQ theorem | EPS satisfiable | SLH theorem |
| --- | ---: | ---: | ---: | ---: |
| Validation | 6 | 6 | 6 | 6 |
| Test | 6 | 6 | 2 | 6 |

Every strategy used `KBO6`, a 5-second soft CPU limit, a 7-second hard CPU
limit, 1,536 MiB of memory, four concurrent workers, and two repetitions. The
eight strategies were:

1. global refined-weight/FIFO age-weight baseline;
2. goal hard priority in the refined-weight queue;
3. scalar conjecture-relative symbol weighting;
4. goal layering at `4:1`;
5. goal layering at `1:4`;
6. Horn layering at `4:1`;
7. unit layering at `4:1`;
8. global baseline plus the static delete-bad falsification control.

The canonical contract ID is
`2c8c13e468c19741c8fbc1ee8f56629c9d3d1519cc92da205a93781b20bfa42a`.
All 704 coordinates completed. An identical second invocation reverified every
stored hash in 1.4 seconds and reported `704 resumed`.

The final raw archive is ignored at
`.artifacts/layered-clause-selection/layered-full-robust-final.tar.gz`. It is
17,004,285 bytes with SHA-256
`30d7ffb29da7a3923ddcccb7bdaa9d5841c242b8a44f3243abf4f48a5fff5bcc`.
It contains the raw runs, final analysis, and resume transcript.

Comprehensive source validation is retained at
`.artifacts/linode/260728-122336-6b18/`: 4,438 library tests, strict formatting
and Clippy, native and Windows GNU x64 builds, clean FOL/HO C builds, 50 main
and 216 tool compatibility cases with zero unexpected differences, ten
behavior-matching benchmarks at `1.0693048764x` C wall time, and Callgrind
smoke.

## Results

The complete verified tables and problem-level comparisons are in
[`RESULTS.md`](RESULTS.md); the machine-readable result is
[`results-summary.json`](results-summary.json).

All layered candidates reproduced the same seven validation solves. Solved-case
efficiency selected `goal_layered_4_1`: its median solved CPU time was
0.784381 seconds, versus 1.055072 for goal `1:4`, 1.058242 for Horn `4:1`,
and 1.193833 for unit `4:1`.

On the untouched test split:

| Strategy | Reproducible solves | Test-only versus baseline |
| --- | ---: | --- |
| Global age/weight baseline | 0 | none |
| Validation-selected goal layer `4:1` | 1 | `NUN060+1` |
| Scalar goal relevance | 0 | none |
| Goal hard priority | 3 | `NUN060+1`, `NUN085+1`, `SEU025+1` |
| Static delete-bad control | 0 | none |

The selected layer therefore failed the preregistered two-unique-solve
threshold. Across the complete contract there were zero contradictory statuses
and zero schedule-fairness bound violations. Of 704 runs, 546 produced valid
telemetry, none produced invalid telemetry, and 158 hard-stop paths produced no
telemetry file. Missing telemetry is excluded from metric aggregation and is not
treated as a soundness event.

The static pruning control lost three validation solves relative to baseline,
added no test solve, deleted 6,085,669 validation clauses and 5,715,333 test
clauses across repetitions, and substantially reduced its generated/processed
ratio. This is evidence that indiscriminate non-redundant passive pruning saves
work by destroying useful search, not evidence for LRS.

## Decision

Reject the tested layered-clause-selection candidates for production. The best
validation-selected layer added only one held-out solve, below the declared
advance threshold, while adding tuning dimension and queue state. The telemetry
does show that the existing heuristic control block enforces its schedule
bounds, so no new layering architecture is required if a later hypothesis
identifies a stronger predicate.

Reject a direct Vampire LRS port. Its Otter-loop reachability policy does not map
cleanly to Umlaut's DISCOUNT ownership, and the available static deletion proxy
lost coverage while deleting millions of clauses.

Preserve the unexpected goal-hard-priority signal as follow-up
`E_Rust_Port-9jt.3.8`. It is simpler than explicit layering and earned three
reproducible held-out wins at this short budget, but it must survive larger
resources, broader categories, and a baseline-loss audit before adoption.

## Discarded setup attempts

No discarded result contributes to the final contract:

- an initial smoke used the invalid E-compatible CLI option
  `--term-ordering=Auto`; the final contract fixes `KBO6`;
- the first category draft mislabeled `EPU` as non-theorem; the final contract
  uses satisfiable `EPS` and explicit four-class normalization;
- the first resume comparison compared serialized lists with in-memory tuples;
  the final contract JSON-normalizes before hashing and comparison;
- one pre-final matrix encountered an empty telemetry file after a hard stop;
  final capture hashes and records interrupted telemetry without parsing it.

Each correction changed the harness hash and used a fresh output root. Only the
final contract and archive above are acceptance evidence.

## Reproduction

After provisioning and synchronizing a normal Ubuntu 24.04 runner, separately
upload and hash-check the ignored corpus archive, safely extract it, build or
provide the exact release binary, and run:

```text
python3 experiments/2026-07-28-005-layered-clause-selection/run.py \
  --manifest benchmarks/casc_2025_manifest.jsonl \
  --problem-root /opt/e-rust-port/source \
  --binary /root/umlaut-layered-final \
  --output-root /opt/e-rust-port/layered-full-robust
```

Run the identical command again to verify exact resume, then analyze:

```text
python3 experiments/2026-07-28-005-layered-clause-selection/analyze.py \
  --run-root /opt/e-rust-port/layered-full-robust \
  --json-output results-summary.json \
  --markdown-output RESULTS.md
```
