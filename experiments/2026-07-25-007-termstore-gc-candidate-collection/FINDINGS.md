# Experiment 308: Term-store GC candidate collection

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`.

## Question

Does matching C's term-store garbage-collection sweep more closely—collecting
only unmarked deletion candidates into one reusable buffer—remove Rust's
survivor-clone and survivor-drop overhead without changing proof behavior or
resource use?

## Baseline

Accepted source is commit `3fb7249c`. Comprehensive Linode run
`.artifacts/linode/260725-214142-abc8/` reports:

- 50 main-prover cases with zero unexpected differences;
- 216 support-tool cases with zero unexpected differences;
- a ten-case aggregate Rust/C wall-time ratio of `1.3220166677x`; and
- smoke Callgrind counts of `20,531,747` Rust versus `7,590,630` C
  instructions.

The Rust smoke profile attributes `15,360,467` inclusive instructions to
`TermCellStore::gc_sweep`, including `5,327,115` in `TermTree::terms`,
`4,523,222` dropping `Vec<Term>` iteration values, and `2,272,481` dropping
the collected vector. The corresponding C profile attributes `4,526,328`
inclusive instructions to `TermCellStoreGCSweep`.

The accepted focused LUSK6 profile has `8,309,402,629` Rust instructions.
Garbage collection is principally a startup-heavy cost, so this long proof is
included as a regression control rather than as the primary expected win.

Baseline raw artifacts:

```text
.artifacts/linode/260725-214142-abc8/validation-summary.json
.artifacts/linode/260725-214142-abc8/callgrind-rust.out
.artifacts/linode/260725-214142-abc8/callgrind-rust.txt
.artifacts/linode/260725-214142-abc8/callgrind-c.out
.artifacts/experiments/2026-07-25-006-static-autoschedule-tables/callgrind-lusk-candidate.out
```

## Candidate

The candidate retains the existing safe tree walk but appends only matching
garbage candidates to a deletion vector allocated once per store sweep. Each
bucket then pops and deletes those candidates before the next bucket,
matching upstream's `del_stack` lifetime and LIFO deletion order.

`eprover/TERMS/cte_termcellstore.c` is the source reference and remains
unchanged.

## Setup and exact commands

Focused validation used fresh dedicated worker
`e-rust-codex-260725-221358-d769` with Rust 1.97.1. The accepted commit was
archived locally, uploaded to a separate source tree, given the identical
unchanged `eprover/` submodule snapshot, and built beside the exact candidate.
The relevant commands were:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo test --locked `
  --manifest-path /opt/e-rust-port/source/Cargo.toml `
  terms::termcellstore::tests
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo test --locked `
  --manifest-path /opt/e-rust-port/source/Cargo.toml `
  terms::termtrees::tests
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo clippy --locked `
  --manifest-path /opt/e-rust-port/source/Cargo.toml `
  --all-targets --all-features -- -D warnings -D clippy::pedantic
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo build --locked --release `
  --manifest-path /opt/e-rust-port/source/Cargo.toml --bin eprover
git archive --format=tar.gz `
  --output=.artifacts/baseline-3fb7249c.tar.gz 3fb7249c
.\linode-runner.ps1 exec -- cp -a `
  /opt/e-rust-port/source/eprover /opt/e-rust-port/baseline/eprover
.\linode-runner.ps1 exec -- /root/.cargo/bin/cargo build --locked --release `
  --manifest-path /opt/e-rust-port/baseline/Cargo.toml --bin eprover
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- bash `
  /opt/e-rust-port/source/experiments/2026-07-25-007-termstore-gc-candidate-collection/remote_measure.sh `
  /opt/e-rust-port/source `
  /opt/e-rust-port/artifacts/experiment-308
.\linode-runner.ps1 down
.\linode-runner.ps1 run
```

`remote_measure.sh` profiles parent and candidate on `socrates.p` and
`LUSK6.lop`, then runs eight warmups plus 256 alternating short-proof pairs
and four warmups plus 32 alternating LUSK6 pairs. It reuses the established
resource-usage measurement and paired-analysis scripts from Experiment 302.

## Results

The candidate removes the intended work:

| Metric | Parent | Candidate | Delta |
| --- | ---: | ---: | ---: |
| Smoke instructions | 20,519,453 | 9,598,346 | -10,921,107 (-53.2232%) |
| Smoke GC-sweep instructions | 15,360,468 | 4,439,203 | -10,921,265 (-71.1008%) |
| LUSK6 instructions | 8,309,597,604 | 8,305,759,465 | -3,838,139 (-0.0462%) |
| Release executable | 8,284,944 bytes | 8,284,240 bytes | -704 (-0.0085%) |

The candidate's smoke sweep is slightly below the independent C profile's
`4,526,328` instructions. The old `TermTree::terms` and
`IntoIter<Term>`-drop frames disappear from the candidate profile.

Smoke native production timing:

| Metric | Parent mean | Candidate mean | Delta | Candidate wins |
| --- | ---: | ---: | ---: | ---: |
| Wall | 0.003480663 s | 0.002272688 s | -34.7053% | 256/256 |
| CPU | 0.003368953 s | 0.002164594 s | -35.7488% | 256/256 |

LUSK6 native production timing:

| Metric | Parent mean | Candidate mean | Delta | Candidate wins |
| --- | ---: | ---: | ---: | ---: |
| Wall | 1.515092511 s | 1.516711286 s | +0.1068% | 17/32 |
| CPU | 1.514650938 s | 1.516230750 s | +0.1043% | 17/32 |

The LUSK6 median wall/CPU changes are improvements of 0.1583%/0.1435%;
mixed means and medians at roughly one tenth of one percent are treated as
timing noise. The deterministic LUSK6 profile independently establishes that
the candidate performs slightly less work.

Parent and candidate have one exact stdout hash on each workload:

- `socrates.p`:
  `4ddee43213d19ebf397dc377e2dc0166eb28eb422da5eedc8b5761ed8384f900`;
- `LUSK6.lop`:
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

Focused tests pass 7/7 for `termcellstore` and 5/5 for `termtrees`, including
multiple deletion candidates plus a survivor in one bucket. Rustfmt and
strict all-target/all-feature Clippy pass.

Comprehensive run `.artifacts/linode/260725-223007-9e34/` validates the
performance result but is not an acceptable behavior gate:

- all 4,404 Rust tests across 32 result groups, Rustfmt, strict
  all-target/all-feature Clippy, native release builds, Windows GNU x64
  test/release cross-builds, and clean FOL/HO C builds pass;
- all 216 support-tool cases match, apart from the 15 declared differences;
- all ten benchmark cases preserve behavior and reduce the aggregate Rust/C
  wall-time ratio from `1.3220166677x` to `1.141231315084483x`;
- smoke Callgrind falls from `20,531,747` to `9,610,482` Rust instructions
  against `7,590,630` C instructions; and
- the 50-case main matrix has one unexpected difference plus its one declared
  difference.

The unexpected `SWB008+1.p` result is a duplicate hard-timeout failure/SZS
banner and duplicate fatal diagnostic. Both implementations still exit 8
with `ResourceOut`, and the pending proof-search bytes otherwise match. The
faster candidate exposed a race between Rust's cooperative hard-deadline
finalizer and a nearly simultaneous kernel `SIGXCPU`; both paths emitted the
same terminal report.

Experiment 309 isolates the single-owner signal fix. Its fresh combined
comprehensive run `.artifacts/linode/260725-231530-96af/` accepts the GC
candidate:

- all 4,405 Rust tests across 33 result groups, Rustfmt, strict Clippy, native
  release builds, Windows GNU x64 compile-only builds, and clean FOL/HO C
  builds pass;
- the 50-case main matrix returns to zero unexpected differences and one
  declared difference, including exact `SWB008+1.p`, BOO020, and SWV851
  resource behavior;
- the 216-case tool matrix has zero unexpected and 15 declared differences;
- all ten benchmark cases preserve behavior, with a fresh-worker aggregate of
  `1.1481929570688398x`; and
- smoke Callgrind remains at `9,610,372` Rust versus `7,590,630` C
  instructions.

The aggregate's movement from the first candidate run's `1.1412x` is normal
fresh-worker timing noise. Both runs retain essentially identical
deterministic smoke work and the large reduction from the accepted parent.

Raw focused artifacts:

```text
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/binary-size.csv
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/binary-sha256.txt
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/callgrind-instructions.txt
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/callgrind-smoke-parent.out
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/callgrind-smoke-candidate.out
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/callgrind-lusk-parent.out
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/callgrind-lusk-candidate.out
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/native-smoke.csv
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/native-smoke-summary.json
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/native-lusk.csv
.artifacts/experiments/2026-07-25-007-termstore-gc-candidate-collection/native-lusk-summary.json
```

## Falsification checks and limits

- Focused same-bucket tests must cover multiple deletion candidates plus a
  survivor, because deletion splays and reorganizes the bucket tree.
- Parent and candidate must use the same compiler, worker, problem bytes, and
  invocation.
- Exact stdout, stderr, and exit status must match on every timed process.
- Callgrind instructions decide whether the intended survivor work was
  removed; alternating same-worker timings decide whether that translates to
  production throughput.
- The long LUSK6 workload must show no meaningful deterministic or native
  regression even though the expected benefit is concentrated at startup.
- Acceptance still requires the full maintained compatibility, resource, and
  portability matrix.

## Decision

Accept. The candidate matches C's candidate-only, reusable-buffer GC sweep,
removes 10.92 million deterministic startup instructions, improves the
maintained aggregate from `1.322x` to approximately `1.15x`, and has exact
proof, compatibility, resource, portability, and quality-gate evidence on the
combined tree. The overall performance Bead remains open against the normal
`1.10x` target.
