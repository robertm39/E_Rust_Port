# Optional CaDiCaL production gate findings

Bead: `E_Rust_Port-9jt.4.8`

Status: complete; production integration is accepted as optional, while
automatic clause-count dispatch remains nondefault because the preregistered
workload gate failed.

## Decision

Keep `UMLAUT_CADICAL_MODE=off` as the default. Ship the safe incremental
service and the removable `cadical-static` feature, with `always`, `auto-128`,
and `auto-256` available only by explicit runtime opt-in.

The production boundary passed its semantic, model, core, cancellation, reset,
native-limit, scoped-proof, checker-failure, Linux, and Windows-GNU gates.
However, the new workload's internal fallback exceeded the frozen 30-second
process guard in all five repetitions of three AVATAR-style sessions (128,
320, and 512 clauses). The preregistered rule makes any process failure a hard
rejection. Strong descriptive results on the remaining complete pairs cannot
override that rule.

## Run contract

The experiment ran on normal-profile Ubuntu 24.04 runner
`e-rust-codex-260729-000403-2cbf` (run ID `260729-000403-2cbf`, Linode ID
`101648425`). Tool versions were Rust 1.97.1, Cargo 1.97.1, GCC/G++ 13.3.0,
and POSIX-thread MinGW GCC/G++ 13.

CaDiCaL was exact revision
`c60730422e758ef1cebe7aeddf2dda31c996bf04`, reporting 3.0.1. This runner used
a local `git archive` of that commit, SHA-256
`8de219030b899e1a1fe50efd465e4b1115d1965bee795c7fe8f8dc1e206252a5`.
The routine bootstrap now downloads the upstream archive of the same commit
and requires SHA-256
`ad639a302b7c4cb4a24f37b7cd0cf7533674e6069c20a561505bccef1c2b4444`.

The frozen CASC selection contains 36 problems, six each from `ALG`, `GEO`,
`ITP`, `LAT`, `NLP`, and `SCT`. None of those families appeared in any
selection from the prior backend bake-off. The selection SHA-256 is
`a1b1f79367a2dbe43b7a7c32749607adb0d86a35720deeaf2aa3e839fd29ac03`.
The deterministic problem-plus-recursive-axiom archive contains 448 files,
is 1,371,451 bytes, and has SHA-256
`2d1475bda056eb89c38cf28115838435c679cba1baf8be412e0c3962617e4091`.

The service benchmark used seed `20260729`, one prior unmeasured warm-up, five
measured repetitions, CPU 0, randomized backend/session order, and a
30-second process guard. The only evaluated dispatch thresholds were the
preregistered 128 and 256 clauses.

## Workload

Thirty-four of 36 selected problems produced SATCheck traffic. The recorder
wrote 257 raw post-grounding, post-pure-filter snapshots; hash deduplication
left 134 sessions:

| Property | Value |
| --- | ---: |
| Unique SATCheck sessions | 134 |
| Sessions at least 128 clauses | 35 |
| Sessions at least 256 clauses | 25 |
| Clause range | 0--2,448 |
| `ALG` / `GEO` / `ITP` sessions | 20 / 14 / 24 |
| `LAT` / `NLP` / `SCT` sessions | 30 / 27 / 19 |

The separate deterministic abstraction workload contains 12 selector-guarded
pigeonhole sessions at 96, 127, 128, 160, 192, 238, 255, 256, 320, 372, 384,
and 512 clauses. Per-call assumptions activate and deactivate components and
repeat UNSAT queries, exercising reuse and failed cores. These are
AVATAR-style service sessions; they do not claim that Umlaut already has a
production AVATAR saturation loop.

## Correctness result

The final benchmark contains 7,240 records. There are 143 sessions and 715
session-repetitions with complete paired internal/CaDiCaL results. Those
complete pairs have zero SAT/UNSAT/Unknown status mismatches.

There are 15 censored internal process failures: five each for the 128-, 320-,
and 512-clause abstraction sessions. Each timeout emitted its quick inactive
SAT query before spending the remaining guard on an activated component.
CaDiCaL completed the corresponding sessions. A separate 120-second warm-up
of the 512-clause internal session also timed out. These are performance
failures rather than unsound statuses, but they fail the preregistered
completeness gate.

Focused production tests passed:

- 7/7 CaDiCaL wrapper tests, including complete model validation, independently
  re-solved failed cores, native decision limits, cancellation/reset,
  assumption expiry, real independently checked DRAT, and checker rejection;
- 7/7 backend-neutral internal-service tests; and
- the end-to-end SATCheck selector-to-clause-core test.

For proof-required UNSAT, the wrapper creates a new proof-enabled solver,
replays the exact permanent clauses and assumptions, writes the exact DIMACS
scope, finalizes DRAT, invokes the separately compiled `drat-trim`, and returns
UNSAT only after checker success. The configured-checker test passed. The
known-failing-checker test returned an error instead of UNSAT.

The final repository-wide runner pass used the same Ubuntu 24.04 host and
toolchain. It passed `cargo fmt --all -- --check`, 4,464 library tests plus all
other all-target/all-feature test harnesses, and warnings-plus-pedantic Clippy.
Linux release binaries built, and Windows-GNU compiled every all-feature test
target plus every release binary. The pinned E comparison reported zero
unexpected mismatches across 50 main-prover cases and 216 support-tool cases.
The ten-case timing benchmark reported zero behavior mismatches and a 1.079x
aggregate Umlaut/E wall-time ratio. The complete validation artifact directory
is 7,826,573 bytes; its `runner.log` SHA-256 is
`2078e8c51d2478d9a34461a36c1756daaf8fddb879763ab2f9645b03e2105b98`.

## Descriptive threshold comparison

The following measurements include only the 715 complete paired
session-repetitions and therefore are descriptive, censored, and ineligible
to pass the gate:

| Policy | Total cost / internal | Query p95 / internal | AVATAR-style total / internal | Fresh SATCheck total / internal | Cost / other threshold |
| --- | ---: | ---: | ---: | ---: | ---: |
| 128 clauses | 0.004740 | 0.000455 | 0.011161 | 0.000775 | 0.013733 |
| 256 clauses | 0.345144 | 0.000455 | 0.902911 | 0.000745 | 72.815307 |

The complete-pair internal query p95 was 456,836,548 ns; both dispatch
policies measured 207,739 ns. The 128 policy measured 1,584,404,544 ns
aggregate versus 115,368,902,739 ns for 256. These results strongly favor 128
on the observed complete subset, but the censored cases and hard gate forbid a
default change.

## Build and package evidence

The clean recorder-free Linux `--release --all-targets --features
cadical-static` build passed. The primary ELF:

- is a 64-bit x86-64 PIE;
- is 9,922,496 bytes;
- has SHA-256
  `691a23aa6651cd978a14a3f6c746ff64e0835c29024aafadfc885897cd774b4b`;
- contains the CaDiCaL objects through the static
  `libumlaut_cadical.a`; and
- dynamically needs only the Linux loader, `libstdc++`, `libgcc_s`, `libm`,
  and `libc`, with no solver shared object.

The clean compile-only Windows-GNU all-target build passed. The primary PE32+
executable:

- is 9,596,246 bytes;
- has SHA-256
  `129e7de4b235c239f627893e7487a0c5fb97669b538b4bfb919a0c53ce126fc1`;
  and
- imports the expected Windows runtime DLLs plus `libstdc++-6.dll`, with no
  CaDiCaL DLL.

The final default package audit passed on the same host:

| Measured artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Source `.tgz` (314 members) | 1,955,806 | `2d82e62955b0f2eb1a9a1c2c77007e05fefc3af0c4130aee83618416664a5b3f` |
| StarExec `.tgz` (five members) | 2,794,141 | `e79448ef845c83e1f7022a2b9b12949a16db722812862a15e104526197c687a3` |
| Uncompressed default Linux ELF | 8,255,312 | `84897ed61fd114a08582780a67665ad321b923cfa270bce334a71e17be8dba17` |

The source package includes the independent C ABI shim and CaDiCaL MIT notice,
but no upstream source. It built all 26 binaries offline. The default runtime
remains solver-free, its dynamic-library audit found only the Linux loader,
`libgcc_s`, `libm`, and `libc`, and the StarExec include/signal/wrapper
emulation passed. A feature build requires the pinned external source and
C++17 toolchain. Omitting the feature is complete build-time disablement;
runtime `off` is the default fallback.

## Deviations and harness repairs

The first attempted full 369 MB CASC corpus transfer hit the controller
timeout after 248,414,208 bytes. The partial remote file was deleted, and the
deterministic 1.37 MB selection-only archive above was used instead. Problem
membership did not change.

The first timed attempt exposed that Python's `TimeoutExpired` can carry byte
output even with `text=True`; JSON serialization aborted after the first
timeout. `benchmark.py` was corrected to decode such prefixes with replacement.
That attempt had already completed the preregistered warm-up, so the final
five-repetition run did not repeat it. A separate analyzer repair changed the
expected backend label from `cadical` to the production name
`cadical-3.0.1-static`. Neither repair changed a workload, threshold, result,
or decision rule.

## Reproduction commands

The significant remote commands were:

```text
python3 instrument_capture.py src/clauses/satinterface.rs
UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-src \
  cargo build --locked --release --features cadical-static --bin umlaut
python3 capture.py fresh-selection.jsonl /opt/e-rust-port/source \
  target/release/umlaut CAPTURE_DIR capture-results.jsonl \
  --cpu-seconds 3 --wall-seconds 6 --capture-max 8 --cpu 0
python3 prepare_captures.py capture-results.jsonl CAPTURE_DIR WORKLOAD/satcheck
python3 generate_avatar_workloads.py WORKLOAD/avatar
python3 benchmark.py target/release/umlaut-sat-service-probe \
  --sessions WORKLOAD/satcheck --sessions WORKLOAD/avatar \
  --repetitions 5 --warmups 0 --timeout-seconds 30 --cpu 0 \
  --seed 20260729 --output benchmark.jsonl
python3 analyze_gate.py benchmark.jsonl --output gate.json
```

The ignored raw result archive is
`.artifacts/experiments/2026-07-29-001-cadical-production-gate/results.tar.gz`.
It contains captures, prepared sessions/manifests, final JSONL/JSON, build and
link metadata, focused test logs, and the tool/source hash contract. It is
435,835 bytes with SHA-256
`9ea2d266075d7680dc398bb8ff78d2ea6f17a33539efff035efb416e7801b064`.

The ignored final validation/package archive is
`.artifacts/experiments/2026-07-29-001-cadical-production-gate/evidence-9jt-4-8.tar.gz`.
It contains the complete comprehensive-run artifact directory and package
audit, is 5,446,735 bytes, and has SHA-256
`315de1153e29c95a5fdae6e673579dd6851de68c60c0660849276a49dfa98162`.
