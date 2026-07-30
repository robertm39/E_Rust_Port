# Real ground-theory branch traces: findings

Bead: `E_Rust_Port-9jt.5.10`

Verdict: **do not advance the native checker into production**.

The candidate is sound, deterministic, small, fast, fail-closed, removable,
and neutral on branches it cannot prune. It did not produce enough held-out
search benefit to pass the frozen efficacy gates. No production source,
default feature, dependency, schedule, or package changed.

## Frozen evaluation

The preregistration was frozen before held-out clausification or search.
Production `umlaut --cnf --tstp-out` generated the normalized clauses. The
experiment then applied deterministic typed grounding and bounded DPLL search
with exact source, clause, literal, and grounding ancestry.

The source split was family-separated:

| Partition | Selected families | Sources | Eligible families |
| --- | --- | ---: | --- |
| Train | DAT, HWV, ITP, SWC, SWW, SYO | 26 | DAT, SWC, SWW |
| Validation | ARI, NUM | 6 | NUM |
| Test | ANA, SEV | 6 | ANA, SEV |

All 12 held-out production CNF captures succeeded twice. The repeated stdout
and stderr bytes, exit status, source identity, and output hashes were
identical for every source.

The held-out abstraction contained 2,989 eligible query occurrences across
three eligible families. Exact caching reduced them to 90 unique constraint
sets, with 2,901 cache hits. Five measured repetitions followed one warmup for
each backend.

## Correctness and evidence

The dependency-free native checker, persistent pinned-Z3 process, and pinned
Z3 C API driver agreed on all 3,320 training and all 90 held-out unique
queries.

| Corpus | SAT | UNSAT | Backend agreement | Five-run evidence |
| --- | ---: | ---: | --- | --- |
| Train | 2,814 | 506 | Exact across all three | Deterministic |
| Held-out validation | 48 | 1 | Exact across all three | Deterministic |
| Held-out test | 37 | 4 | Exact across all three | Deterministic |

Independent Python verification accepted every exact model and every negative
cycle. The Rust replay checker accepted all 9,960 combined training backend
certificates, all 270 combined held-out backend certificates, and all 90
independent held-out reference certificates.

For both held-out partitions, the replay checker rejected all six mutation
classes:

1. empty UNSAT core;
2. unknown core label;
3. bounds changed to remove the negative cycle;
4. missing model variable;
5. a constraint tightened below the submitted model;
6. status/evidence mismatch.

Missing, malformed, timed-out, cancelled, unsupported, and solver-Unknown
answers remain `Unknown`. Native process termination, Z3 process termination,
and Z3 FFI interruption all completed within the one-second gate and did not
publish a trusted result.

The held-out dotted TPTP problem IDs exposed an adapter-only protocol issue.
`prepare_query_corpus.py` now deterministically encodes unsafe punctuation in
the protocol ID while preserving the original query ID in the query index.
Focused tests cover safe, dotted, and leading-digit identifiers.

## Held-out efficacy

The candidate failed every frozen efficacy threshold:

| Gate | Required | Observed | Result |
| --- | ---: | ---: | --- |
| Verified theory prunes | at least 20 | 6 | Fail |
| Prunes / eligible checker decisions | at least 5% | 6 / 2,991 = 0.20% | Fail |
| Closed or at least 10% fewer nodes | at least 3 workloads | 1 | Fail |

No held-out abstraction closed. `SEV422_1` was the sole improved workload:
four verified theory prunes reduced its complete search from 1,407 to 1,063
nodes, a 24.45% reduction. `NUM861_1` produced two prunes, but its
leaf-bounded search reached the same 1,024 open-leaf limit and used 2,062
nodes versus 2,058 without the checker. The five ANA workloads produced no
prunes.

Ten no-prune held-out workloads retained identical status, node count, leaf
count, assignments, decisions, propagation, parents, and outcomes. The
comparison deliberately ignores only `theory_*` telemetry such as a cached
query ID or verified SAT annotation; those fields cannot alter control flow.
This passes the neutral no-loss gate.

## Performance and package boundary

The native latency result is comfortably below the frozen 0.25 ms p95 limit:

| Corpus | Native p95 | Z3 FFI p95 | Z3 process p95 |
| --- | ---: | ---: | ---: |
| Train, 16,600 measured calls | 0.041 ms | 0.899 ms | 2.192 ms |
| Held-out validation, 245 calls | 0.0044 ms | 0.442 ms | 1.764 ms |
| Held-out test, 205 calls | 0.0036 ms | 0.391 ms | 1.496 ms |

An identical release-profile empty Rust binary measured 371,544 bytes; the
native driver measured 461,696 bytes. The incremental file cost was 90,152
bytes and the incremental loaded-section cost was 73,798 bytes, both below
the 262,144-byte gate. It added no dynamic dependency. The experiment-only
Cargo patch registers binaries only, so the default production package delta
is zero.

The pinned Z3 identity remained
`2d48fd119ce5074b880944c2b1c59e537c99cd46`, with source-archive SHA-256
`9b78c0cc9f330dab9f39c132aba39c92fdba2dbc0aac26dd07b3946592dd21d8`.
Z3 remains an external experiment control and is not a production dependency.

## Reproducibility artifacts

Raw outputs are intentionally ignored under
`.artifacts/experiments/2026-07-30-002-real-ground-theory-traces/`.
Key retained hashes are:

| Artifact | SHA-256 |
| --- | --- |
| `heldout-analysis.json` | `8a45ea82083243aebefe9ccd2ee78ec840c1a4340fb85b9fcc952e3297d5ae02` |
| `native-package.json` | `c41cfcea1120004aca7d2131213b871b678a091a37cba8e8ce95e8660ed5f8fb` |
| Training backend report | `1e76d1f16f899d07e9decc477e2bf070eb11f0f9f7e02c46ce2fb4efa2dcb942` |
| Validation backend report | `4c8015ec3a254826c98e4d52fef571d69394162db492cfb68423f2eb3120c173` |
| Test backend report | `4d9ffa1fd2736a5b96605528f4d7d1c842e972cf1492b8f02ab3b2353095273d` |
| `heldout-results.tar.gz` | `70e21bbca7b2b4d50379f6cce71518f8b62c4d3489b101846d5f4f56f724fb96` |
| `train-backends.tar.gz` | `f452eca629c727359616cc8c74eabe3e6d65f7dcda063cee7aa007d3d4ef0bf5` |

`analyze_heldout.py` recomputes the source effects and every advancement gate
from those raw artifacts. Its final verdict is `do_not_advance`.

## Follow-up direction

The limiting factor is not solver correctness or checker overhead. The exact
difference fragment is too sparse in these real normalized branch streams,
and SAT results dominate. Future work should first improve eligible
arithmetic extraction or target a richer native fragment under a new
preregistration. Re-running this same checker behind production search would
not satisfy the evidence required for activation.
