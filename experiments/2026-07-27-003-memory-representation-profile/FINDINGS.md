# Current memory representation profile

## Status

Complete diagnostic experiment for Bead `E_Rust_Port-9jt.8.1`. Production
source is unchanged. The profile rejects a new general term/clause/index
layout prototype on current evidence and identifies proof-trace storage as a
separate measured target for existing Bead `E_Rust_Port-9jt.8.2`.

## Question and decision rule

Which term, clause, and index owners dominate current retained memory on a
representative proof and bounded searches? Do object sizes, garbage-collection
cost, or cache behavior justify an isolated reversible layout prototype?

A structural prototype is warranted only if the current profile identifies a
large owner that is not already covered by an accepted or falsified
experiment, and if its safety, proof identity, and engineering risk are
proportionate to the observed share. A tempting small-node or arena design is
not accepted from peak bytes alone.

## Setup

The tracked [`run_profile.py`](run_profile.py) harness ran serially on Ubuntu
24.04 Linode run `260728-040506-50ab`, using Linux
`6.8.0-134-generic`, x86-64, Python 3.12.3, Rust 1.97.1, and Valgrind 3.22.0.
The source commit was
`069a5f7f0ecd9bc27f0b5df3472fddffc27550bb`; the exact uploaded snapshot
SHA-256 was
`ac193354a772ba8562ea9efaef21ba974d6db3fa061905bf23fecaf6a73fefbb`.
The profiled release executable had SHA-256
`a8184939cdc05629eb252ddd89238f3703adccb6426a56e785f6fb20abf528a7`.

Five native repetitions measured a trivial proof, the complete SYN190 and
LUSK6 proofs, and 20,000-step LCL365/SWV851 bounded searches. Massif profiled
LCL365 and LUSK6 with byte-based snapshots, stacks, 0.1% allocation-tree
thresholds, and 100 snapshots. Cachegrind simulated the first 1,000 SYN190
and 2,000 LCL365 processed steps. Callgrind measured the complete Socrates
startup/proof/collection path.

The LUSK6 command retained its 2 GiB prover memory cap:

```text
umlaut --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new LUSK6.lop
```

All five focused 64-bit layout tests passed. The current gates pin or bound:

| Owner | Bytes |
| --- | ---: |
| `Term` and `Option<Term>` | 8 each |
| `TermLinkData`, `TermLinks`, `TermArgs`, `TermArguments` | 24 each |
| `TermCell` | 136 |
| `Clause` | at most 136 |
| `SimpleEvalCell`, `EvalCell` | 32 each |
| `PdtTraversalFrame` | 40 |
| `DerivationEntry` | 32 |

## Native pressure and throughput

Every repetition of a workload preserved exit status and stdout/stderr hashes.
The bounded runs stopped with the expected step-limit status; the other three
proved and exited zero.

| Workload | Median CPU | Processed/CPU-s | Median max RSS | Clause high water | Shared terms |
| --- | ---: | ---: | ---: | ---: | ---: |
| SYN190 proof | 0.32 s | 19,950 | 52,672 KiB | 18,799 | 4,106 |
| LCL365, 20k limit | 0.58 s | 34,483 | 72,268 KiB | 63,608 | 15,255 |
| SWV851, 20k limit | 2.86 s | 6,993 | 335,028 KiB | 344,374 | 14,897 |
| LUSK6 proof | 1.35 s | 3,610 | 189,672 KiB | 84,631 | 524,364 |

The very different term and clause funnels matter: LUSK6 retains a large
shared-term DAG, while SWV851's RSS follows 344,374 live/high-water clauses
despite only 14,897 final shared terms. A single universal memory layout is
therefore unlikely to address both shapes.

## Retained-memory attribution

### LUSK6 proof

Massif records 186,313,522 useful heap bytes, 11,328,126 allocator-extra
bytes, 58,640 stack bytes, and a 197,700,288-byte total peak.

| Attributed owner | Bytes | Useful heap |
| --- | ---: | ---: |
| Term cells | 79,659,856 | 42.76% |
| Clause-set ownership | 37,467,520 | 20.11% |
| Rewrite derivation stacks | 35,000,832 | 18.79% |
| Heuristic evaluations | 10,664,704 | 5.72% |
| Paramodulation equation storage | 8,174,336 | 4.39% |
| Overlap/subterm/fingerprint index storage | 7,361,152 | 3.95% |

Fat LTO folds identical vector-growth monomorphizations and may assign their
shared code an arbitrary Rust type name. In particular, the 35 MB branch is
labelled as a `Vec<Option<VarBankNamedCell>>::grow_one` symbol, but its complete
allocation caller chain is recursive `term_compute_rw_sequence` reached from
clause normal-form derivation recording. It is proof metadata, not a retained
variable-name environment. Attribution therefore uses the caller tree rather
than the folded leaf symbol.

### LCL365 bounded growth

Massif records 62,809,105 useful heap bytes, 2,568,191 allocator-extra bytes,
58,640 stack bytes, and a 65,435,936-byte total peak.

| Attributed owner | Bytes | Useful heap |
| --- | ---: | ---: |
| Generated equations and paramodulation storage | 24,362,976 | 38.79% |
| Clause-set ownership | 18,928,192 | 30.14% |
| Overlap/subterm/fingerprint index storage | 7,659,944 | 12.20% |
| Heuristic evaluations | 4,156,544 | 6.62% |
| Term cells | 2,360,560 | 3.76% |

LCL365 is dominated by the number and literal content of generated/passive
clauses, not a large term-node representation. Reducing that pressure is more
likely to come from selection, redundancy, or generation work than from
compressing the already compact term node.

## Collection cost

The current Socrates Callgrind run retires 9,592,522 instructions. Parsing the
`TermBank::gc_sweep` call tree gives:

| GC boundary | Instructions | Whole run |
| --- | ---: | ---: |
| Self | 2,166,785 | 22.59% |
| Inclusive | 4,438,882 | 46.27% |

This intentionally tiny workload makes the startup/final collection visible;
it recovered 11 terms. On the longer native runs, recorded recovered counts
were 1,461 for SYN190, 8 for LCL365, 1,303 for SWV851, and 2 for LUSK6.
Experiment 308 already optimized the same sweep from 15.36 million to 4.44
million smoke instructions and found only a 0.0462% exact-work effect on
LUSK6. The current 4,438,882 result reproduces that accepted boundary.

## Cache behavior proxy

| Workload segment | Instructions | L1 I miss | L1 D miss | LL I miss | LL D miss |
| --- | ---: | ---: | ---: | ---: | ---: |
| SYN190, first 1,000 steps | 239,477,100 | 0.916% | 1.746% | 0.0046% | 0.1061% |
| LCL365, first 2,000 steps | 424,491,605 | 0.672% | 1.008% | 0.0024% | 0.0676% |

These end-to-end simulation rates do not show a cache-miss crisis large
enough to justify replacing safe shared identities with compressed IDs or an
arena. They are locality proxies, not hardware performance-counter results.

## Prototype decision and engineering complexity

No new production prototype is accepted or attempted for this Bead:

- Term cells are the largest LUSK6 owner, but the current 136-byte node is the
  result of recently accepted compact metadata and argument experiments that
  improved exact work by 4.27% and 3.18%. The isolated safe indexed arena
  prototype already regressed exact work by 0.91% (0.99% when forced inline).
- Index storage is 3.95% of LUSK6 and 12.20% of LCL365 useful heap, while
  Cachegrind reports low last-level miss rates. A new arena/ID representation
  would have high proof-identity and implementation risk for a bounded target.
- The 35 MB rewrite-derivation branch is large and isolated, but it is
  specifically proof-trace storage. Existing Bead `E_Rust_Port-9jt.8.2`
  requires compact/lazy proof reconstruction plus independent checking.
  A prior local 32-to-24-byte `DerivationEntry` packing experiment preserved
  proof counts but regressed LUSK6 by 12.12%, so blindly shrinking the enum is
  falsified. The next valid design must change ownership/laziness while
  preserving reconstruction, not repeat the packing.
- LCL365 and SWV851 pressure primarily follows generated clause volume.
  Clause selection, redundancy, and indexing bake-offs already own those
  higher-level decisions.

This is the negative-result freedom allowed by the task: the current evidence
narrows the next memory work to proof-trace architecture and search-volume
control instead of authorizing a broad representation rewrite.

## Validation, artifacts, and reproduction

The experiment ran the release build and five focused layout tests before
measurement. Every native repetition was behavior-stable. Both Massif,
Cachegrind, and Callgrind invocations produced complete parseable profiles.
No production source changed, so repository-wide code validation was not
repeated after the already-complete telemetry milestone validation.

The 139 raw files (61,813,486 bytes) are retained outside Git at
`.artifacts/linode/260728-040506-50ab/`. Its `summary.json` SHA-256 is
`38e0d23fbc94c1ebc3d19e2348b97b6020f5da657470728cf8db3ab8a1ca11c4`.
Compact tracked metrics and raw-profile hashes are in
[`results.json`](results.json). The experiment-local
[`analyze_callgrind.py`](analyze_callgrind.py) parser reproduces the tracked
[`gc-analysis.json`](gc-analysis.json) directly from the raw profile:

```text
python experiments/2026-07-27-003-memory-representation-profile/analyze_callgrind.py \
  .artifacts/linode/260728-040506-50ab/callgrind/socrates-gc.out \
  --function "TermBank>::gc_sweep"
```

The guarded lifecycle was:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- python3 `
  /opt/e-rust-port/source/experiments/2026-07-27-003-memory-representation-profile/run_profile.py `
  --repo /opt/e-rust-port/source `
  --artifact-dir /opt/e-rust-port/artifacts/2026-07-27-003-memory-representation-profile-final `
  --repetitions 5 `
  --source-commit 069a5f7f0ecd9bc27f0b5df3472fddffc27550bb `
  --source-snapshot-sha256 ac193354a772ba8562ea9efaef21ba974d6db3fa061905bf23fecaf6a73fefbb
.\linode-runner.ps1 down
```

Both the successful worker and its firewall were deleted after artifact
collection.
