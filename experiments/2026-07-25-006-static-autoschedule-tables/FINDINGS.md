# Experiment 307: Build-time static autoschedule tables

## Status

Accepted for Bead `E_Rust_Port-j76.5.5`.

## Question

Does converting upstream's generated `HEURISTICS/schedule.vars` into borrowed
Rust static tables at build time remove measurable native-Linux startup work
without changing strategy text, schedule selection, proof output, or resource
behavior?

## Baseline

Accepted source is commit `ee64d1eb`. Comprehensive Linode run
`.artifacts/linode/260725-203453-d4fc/` reports:

- 50 main-prover cases with zero unexpected differences;
- 216 support-tool cases with zero unexpected differences;
- a ten-case aggregate Rust/C wall-time ratio of `2.649x`; and
- smoke Callgrind counts of `99,794,981` Rust versus `7,590,630` C
  instructions.

The Rust smoke annotation attributes approximately 37.6 million instructions
to runtime `schedule.vars` C-string parsing. Rust currently embeds the 2 MiB
source text and initializes owned strategy and schedule maps through
`OnceLock`; C includes the generated arrays as compiled static data.

Baseline raw artifacts:

```text
.artifacts/linode/260725-203453-d4fc/validation-summary.json
.artifacts/linode/260725-203453-d4fc/callgrind-rust.out
.artifacts/linode/260725-203453-d4fc/callgrind-rust.txt
.artifacts/linode/260725-203453-d4fc/callgrind-c.out
```

## Candidate

The candidate adds a standard-library-only Cargo build script which parses the
unchanged upstream `schedule.vars` and emits borrowed Rust tables into
`OUT_DIR`. Production lookup scans those tables directly and allocates only
the selected per-run schedule copy. The parser remains available to tests,
which compare every generated strategy, schedule cell, and class-map entry
against the upstream source.

No dependency is added and `eprover/` remains unchanged.

## Setup and exact commands

Focused validation used fresh dedicated worker
`e-rust-codex-260725-211951-f7e7` with Rust 1.97.1. The parent commit was
archived locally, uploaded to a separate remote source tree, and built beside
the exact candidate snapshot. The relevant commands were:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo test --locked `
  --manifest-path /opt/e-rust-port/source/Cargo.toml `
  heuristics::new_autoschedule::tests
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo clippy --locked `
  --manifest-path /opt/e-rust-port/source/Cargo.toml `
  --all-targets --all-features -- -D warnings -D clippy::pedantic
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo build --locked --release `
  --manifest-path /opt/e-rust-port/source/Cargo.toml --bin eprover
git archive --format=tar.gz `
  --output=.artifacts/baseline-ee64d1eb.tar.gz ee64d1eb
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo build --locked --release `
  --manifest-path /opt/e-rust-port/baseline/Cargo.toml --bin eprover
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- bash `
  /opt/e-rust-port/source/experiments/2026-07-25-006-static-autoschedule-tables/remote_measure.sh `
  /opt/e-rust-port/source `
  /opt/e-rust-port/artifacts/experiment-307
.\linode-runner.ps1 down
.\linode-runner.ps1 run
```

`remote_measure.sh` runs parent/candidate Callgrind profiles on `socrates.p`
and `LUSK6.lop`, then runs eight warmup plus 256 alternating short-proof pairs
and four warmup plus 32 alternating LUSK6 pairs. It reuses the established
resource-usage measurement and paired-analysis scripts from Experiment 302.

## Results

The table representation removes the intended work and reduces code size:

| Metric | Parent | Candidate | Delta |
| --- | ---: | ---: | ---: |
| Smoke instructions | 99,786,144 | 20,519,597 | -79,266,547 (-79.4364%) |
| LUSK6 instructions | 8,388,855,525 | 8,309,402,629 | -79,452,896 (-0.9471%) |
| Release executable | 8,563,664 bytes | 8,284,944 bytes | -278,720 (-3.2547%) |

Smoke native production timing:

| Metric | Parent mean | Candidate mean | Delta | Candidate wins |
| --- | ---: | ---: | ---: | ---: |
| Wall | 0.011333559 s | 0.003561589 s | -68.5748% | 256/256 |
| CPU | 0.011218348 s | 0.003451148 s | -69.2366% | 256/256 |

LUSK6 native production timing:

| Metric | Parent mean | Candidate mean | Delta | Candidate wins |
| --- | ---: | ---: | ---: | ---: |
| Wall | 1.530431245 s | 1.523842768 s | -0.4305% | 18/32 |
| CPU | 1.529972969 s | 1.523394344 s | -0.4300% | 18/32 |

The LUSK6 median wall/CPU improvements are 0.5495%/0.5498%. Parent and
candidate have one exact stdout hash on each workload:

- `socrates.p`:
  `4ddee43213d19ebf397dc377e2dc0166eb28eb422da5eedc8b5761ed8384f900`;
- `LUSK6.lop`:
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

Focused Rust tests pass 14/14, including the complete emitted-table
comparison; Rustfmt and strict all-target/all-feature Clippy pass.

Comprehensive clean-room run `.artifacts/linode/260725-214142-abc8/`
confirms:

- 4,402 Rust tests across 32 result groups;
- Rustfmt and strict all-target/all-feature Clippy;
- native optimized binaries plus Windows GNU x64 test and release builds;
- clean pinned FOL and HO C reference builds;
- 50 main-prover cases with zero unexpected and one expected difference;
- 216 support-tool cases with zero unexpected and 15 expected differences;
- ten timing cases with zero behavior mismatches;
- aggregate Rust/C wall ratio reduced from `2.649440x` to `1.322017x`;
- short-case Rust RSS reduced by roughly 4.7 MiB, with no long-case resource
  regression; and
- smoke Callgrind at 20,531,747 Rust versus 7,590,630 C instructions.

The exact focused candidate binary SHA-256
`0cb51ef18a9e2e8ef449ca23cd00bd641e4bd9e170723dfeaa2c23801a901d96`
is also the binary validated by the comprehensive run.

Raw focused artifacts:

```text
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/binary-size.csv
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/callgrind-instructions.txt
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/callgrind-smoke-parent.out
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/callgrind-smoke-candidate.out
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/callgrind-lusk-parent.out
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/callgrind-lusk-candidate.out
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/native-smoke.csv
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/native-smoke-summary.json
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/native-lusk.csv
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/native-lusk-summary.json
.artifacts/linode/260725-214142-abc8/validation-summary.json
.artifacts/linode/260725-214142-abc8/callgrind-rust.out
.artifacts/linode/260725-214142-abc8/callgrind-c.out
```

## Falsification checks and limits

- Generated tables must match all 419 predefined strategies, all 1,618
  schedule arrays, and both class maps, not only sampled entries.
- All native timing processes have identical zero exit status, stdout bytes,
  and empty stderr.
- Instruction counts decide whether the intended startup work was removed;
  alternating same-worker native timings decide whether it improves production
  throughput.
- The parent and candidate use the same compiler, release profile, dedicated
  worker, problem bytes, and invocation. Alternating order prevents a fixed
  first-run advantage.
- The long-proof timing win is intentionally described as modest because only
  18 of 32 candidate runs win; its exact 79.45-million-instruction reduction
  independently establishes less work.
- The full maintained compatibility and resource matrix is exact, but the
  remaining `1.322x` aggregate ratio is still above the normal whole-port
  `1.10x` acceptance threshold.

## Decision

Accept. The candidate removes a large, C-absent fresh-process cost, improves
both deterministic profiles and both native workloads, shrinks the executable,
preserves exact proofs, and keeps the generated upstream data fully checked
without adding a dependency. The aggregate ratio improves by approximately
half but remains above the whole-port target, so Bead `E_Rust_Port-j76.5.5`
stays open for the next measured optimization.
