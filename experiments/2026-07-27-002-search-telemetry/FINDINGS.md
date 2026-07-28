# Search telemetry overhead and diagnosis

## Question

Can an opt-in, stable JSON record expose the search funnel, inference and
simplification activity, index use, term/storage pressure, proof outcome, and
resource use without changing prover semantics or exceeding a 5% aggregate
child-CPU and 10% aggregate wall-time overhead budget?

Can a diagnosis made from that record be reproduced independently on fresh
processes?

The experiment is falsified if any matched telemetry-on/off pair changes exit
status, standard output, or standard error; if schema or high-water invariants
fail; if either overhead budget is exceeded; or if two fresh diagnosis trials
do not show the same low-limit stop and higher-limit proof.

## Setup

The tracked [`run_benchmark.py`](run_benchmark.py) harness ran on normal-profile
Ubuntu 24.04 Linode `e-rust-codex-260728-024950-6433` (run ID
`260728-024950-6433`, Linode `101571734`). The host used Linux
`6.8.0-134-generic`, x86-64, Python 3.12.3, Rust 1.97.1, and Cargo 1.97.1.
The optimized `umlaut` executable had SHA-256
`a8184939cdc05629eb252ddd89238f3703adccb6426a56e785f6fb20abf528a7`.

Three reference problems exercised a fixed 20,000-step search:

| Workload | SHA-256 |
| --- | --- |
| `LCL365-1.p` | `6883bc766b68fbf54db312c9e9bb0b4dfcdb2b64b57050cfc44f921c432608d4` |
| `SEU027+1.p` | `f771ce0cc3b0fd4b70af0256f0a78f23ae28617830dac1089f3055111f63ee43` |
| `SWV851-1.p` | `5c542260939f4d4095e554e6eafe5d10e50402da7d12393fa473ae8c4ca88e70` |

Each workload received one telemetry-off and one telemetry-on warmup, followed
by six matched pairs. Pair order alternated to reduce ordering and thermal
bias. The timed boundary used Python `perf_counter` and cumulative
`RUSAGE_CHILDREN`, so it includes record collection, JSON formatting, and file
I/O. Every pair compared the process return code and SHA-256 hashes of both
output streams.

The exact remote commands were:

```text
cargo fmt --all -- --check
cargo test --locked run_proof_search_writes_opt_in_aggregate_json_telemetry -- --nocapture
cargo build --locked --release --bin umlaut
python3 experiments/2026-07-27-002-search-telemetry/run_benchmark.py \
  --repo /opt/e-rust-port/source \
  --binary target/release/umlaut \
  --artifact-dir /opt/e-rust-port/telemetry-artifacts \
  --repetitions 6
```

Tracked metrics are in [`results.json`](results.json). The 119 raw files
(14,519,469 bytes) are retained outside Git under
`.artifacts/search-telemetry/2026-07-27-002/`; its authoritative
`summary.json` has SHA-256
`0a0321370c77c68ea6a693f22e4673867946bfc809e420a65bd9141f597bd255`.

## Results

All 36 measured searches preserved exit status, standard output, and standard
error. Every telemetry record parsed as JSON, identified schema
`umlaut.search-telemetry` version 1, and satisfied the tested outcome and
clause-set high-water invariants.

| Metric | Disabled | Enabled | Overhead | Budget |
| --- | ---: | ---: | ---: | ---: |
| Aggregate child CPU | 45.104985 s | 44.894482 s | -0.4667% | 5% |
| Aggregate wall time | 45.112985 s | 44.902391 s | -0.4668% | 10% |

Median per-process child CPU was 0.583790/0.583685 seconds
(disabled/enabled) for LCL365, 4.118781/4.073813 seconds for SEU027, and
2.819473/2.816632 seconds for SWV851. The small negative aggregate delta means
the enabled runs were slightly faster within measurement noise; it is not
treated as a speedup. The aggregate, rather than any noisy single short
process, is the acceptance metric.

For the diagnosis, both independent fresh-process trials on `SYN190-1.p`
(SHA-256
`923ab1c09931072a5cd9924b39325ef3632f14258b3b5b14ecb4e39cae9ab5f0`)
reported:

- at a 1,000 processed-step limit: `stopped`, reason `step_limit`, exactly
  1,000 steps, and 1,169 clauses at the total high-water mark;
- at a 10,000 processed-step limit: `returned`, reason `generated_clause`, a
  proof after 6,384 steps, and 18,799 clauses at the total high-water mark.

The independently reproduced diagnosis is that the 1,000-step configuration
truncates a search that needs more given-clause steps; increasing the limit to
10,000 permits the proof. The telemetry distinguishes this from saturation,
memory pressure, or a parser/preprocessing failure.

## Falsification attempts

- Required the normal E-compatible stdout/stderr bytes and exit code to remain
  identical in each on/off pair.
- Validated stable schema identity, all metric groups, exit-status agreement,
  nonnegative steps, and final-versus-high-water inequalities for every
  enabled run.
- Alternated on/off order and excluded warmups from the reported totals.
- Measured externally so JSON serialization and file writing cannot hide
  outside the timed boundary.
- Repeated the limit diagnosis twice with independent processes.
- Probed the kernel hard-CPU-limit boundary separately. Linux exits from the
  asynchronous `SIGXCPU` handler before ordinary JSON formatting is safe, so
  hard-killed and OOM-killed processes are explicitly documented as requiring
  external resource telemetry.

The final comprehensive run `260728-032639-9aff` reproduced the exact release
binary hash used above and wrote both `VALIDATION_COMPLETE` and `SUCCESS`.
Linux Rustfmt, all-target/all-feature tests, pedantic Clippy, release builds,
Windows-GNU x64 test/release compile-only gates, FOL/HO C builds, native smoke
tests, and Callgrind completed. Its 50 main and 216 support-tool comparisons
had zero unexpected mismatches. The separate broad timing suite reported a
1.100289 Rust/C wall ratio, a non-failing warning 0.000289 above its 1.100
advisory threshold; benchmark behavior mismatches remained zero.

## Conclusion

Schema version 1 passes the semantic and performance contracts. Its observed
aggregate child-CPU delta is -0.47%, with no measurable overhead against the
5% budget, and the same telemetry fields independently reproduce a concrete
resource-limit diagnosis.
