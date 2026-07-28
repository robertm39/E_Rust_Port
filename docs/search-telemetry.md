# Search telemetry

Umlaut can write one stable, aggregate JSON record for each completed
saturation search:

```text
umlaut --search-telemetry=run.json problem.p
```

The option is an Umlaut extension to the E-compatible command-line surface. It
is opt-in, requires a file path other than `-`, and does not change normal
standard output or standard error. The parent directory must already exist.
Syntax checking, applicative encoding, pruning, CNF-only conversion, and
strategy-printing modes reject the option because they do not run saturation.

The disabled path does not allocate a record or inspect proof-state sets.
There are two predictable conditional checkpoints per given-clause iteration.
When enabled, those checkpoints update constant-time clause-set high-water
counts. Formatting and file I/O happen once, after the search outcome is
known.

## Schema contract

The top-level `schema` is `umlaut.search-telemetry` and `schema_version` is
`1`. Version 1 contains these groups:

| Group | Diagnostic purpose |
| --- | --- |
| `problem` | Input file names and first-order/higher-order classification |
| `configuration` | Effective heuristic name |
| `outcome` | Returned/stopped classification, stable snake-case reason, processed steps, and exit status |
| `input_funnel` | Parsed axioms, relevancy removals, raw clauses, and preprocessing removals |
| `search_funnel` | Processed/redundant/generated counts plus final and high-water clause-set sizes |
| `inferences` | Paramodulation, factoring, equation-resolution, disequality-decomposition, and negative-extensionality totals |
| `simplification` | Rewrite, subsumption, condensation, and related contraction totals |
| `indices` | Subsumption, demodulation, and backward-rewrite lookup activity |
| `sat` | SAT checks, clause volumes, outcomes, and CPU-time components |
| `terms` | Shared term nodes, insertions, recoveries, and storage estimate |
| `proof` | Answer count, returned-clause depth, and proof/search given-clause counts |
| `resources` | User/system/total CPU seconds and maximum resident pages |

All process-global counters are captured at search entry and emitted as
saturating per-run deltas. This matters for library tests and other callers
that execute multiple searches in one process.

The JSON field names and outcome-reason spellings are compatibility contracts
within schema version 1. Additive fields may be introduced without changing
the version; removing or redefining a field requires a new schema version.
Consumers should reject an unknown major schema version and ignore unknown
additive fields.

## Limits and schedules

Step, clause, and cooperatively observed soft-time limits produce normal
stopped records. Linux's kernel-enforced hard CPU limit terminates from its
asynchronous `SIGXCPU` handler and cannot safely format JSON; it is not
guaranteed to produce a record. Termination before search initialization, an
uncatchable kill, an operating-system OOM kill, or a filesystem error can also
prevent a record from being written. Memory pressure that does complete is
visible through resident-page and term-storage evidence; hard-killed processes
require external resource telemetry.

In a direct run, the configured path is used exactly. Schedule workers append
a collision-free suffix:

```text
BASE.preprocessing-PREPROCESSING_INDEX-pid-PID.json
BASE.search-PREPROCESSING_INDEX-SEARCH_INDEX-pid-PID.json
```

The scheduling parent does not synthesize a combined record. Aggregate worker
records by schema version, problem, configuration, and outcome instead of
assuming a single schedule file.

## Performance evidence

The acceptance budget for schema version 1 is at most 5% aggregate child-CPU
overhead and at most 10% aggregate wall-time overhead when enabled, compared
with the same release binary and matched telemetry-disabled searches. Normal
validation also requires identical exit status, standard output, and standard
error in every matched pair.

The repository preserves the reproducible harness, pinned input hashes,
raw-artifact layout, results, and independently repeated search-limit
diagnosis under `experiments/2026-07-27-002-search-telemetry/`. Benchmarks must
run on the Ubuntu 24.04 Linode authority; do not run the harness on the local
workstation.
