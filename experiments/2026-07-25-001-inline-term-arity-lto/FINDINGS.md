# Experiment 302: Force-inline `Term::arity` under fat LTO

## Status

Rejected experiment for Bead `E_Rust_Port-j76.5.3`; accepted Experiment 301
source is restored byte-for-byte.

## Question

Does forcing the hot immutable `Term::arity` accessor inline under the accepted
fat-LTO and single-codegen-unit release profile improve native-Linux proof
search without changing proof behavior, resource outcomes, or compatibility?

## Baseline

- Accepted source: commit `e4e9d089`, including Experiment 301 and the
  Linode-only execution policy.
- Parent release binary SHA-256:
  `4cdca9212bd1da5780fee1bf91d5e580b4b1b5c193b97ed5e84e541eaab685e6`.
- Historical accepted LUSK6 profile: `8,368,891,139` instructions.
- Deterministic workload: upstream `LUSK6.lop` with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Comprehensive Linode parent run:
  `.artifacts/linode/260725-062100-db53`.

The focused comparison rebuilt both variants on one fresh Ubuntu 24.04 Linode
with Rust 1.97.1. Its exact parent binary reproduced the accepted SHA-256
above and retired `8,367,710,262` instructions. The small difference from the
historical profile is a fresh-toolchain measurement; the same-worker
parent/candidate delta is authoritative for this experiment.

## Candidate

Add only the measured `clippy::inline_always` allowance and
`#[inline(always)]` to `Term::arity`.

The term representation, `RefCell` borrow, slice conversion, accessor body,
all callers, fat LTO, and single codegen unit remain unchanged.

## Comprehensive validation

The exact dirty-worktree snapshot was validated with:

```powershell
.\linode-runner.ps1 run
```

Run `.artifacts/linode/260725-161150-7408` completed:

- Rustfmt;
- all-target/all-feature Rust tests;
- strict pedantic Clippy;
- native default-feature release builds;
- Windows GNU x64 test-target and release cross-compilation;
- disposable upstream FOL and HO builds;
- the 50-case main and 216-case support-tool matrices;
- five-trial native timing, smoke proofs, and smoke Callgrind.

The candidate and parent runs have the same three main mismatch cases and
kinds: `SWB008+1.p`, `BOO020-1.p`, and `SWV851-1.p`. They also have the same
33 support-tool mismatch sequence. Each retains one declared main difference
and eight declared tool differences. The candidate therefore introduces no
new compatibility mismatch, but this fresh Linux baseline does not satisfy
the Bead's zero-mismatch acceptance criterion.

The runner's aggregate Rust/C wall ratio nominally changes from `3.184x` to
`3.116x`. That aggregate is dominated by millisecond-scale startup cases and
independent-worker noise. The two relevant LUSK medians improve after
normalizing by their same-run C results, while the candidate's raw Rust
medians are slower. This conflict required a same-worker focused decision.

## Focused setup

The focused lifecycle was:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec -- bash `
  /opt/e-rust-port/source/experiments/2026-07-25-001-inline-term-arity-lto/remote_measure.sh `
  /opt/e-rust-port/source `
  /opt/e-rust-port/artifacts/experiment-302
.\linode-runner.ps1 down
```

`remote_measure.sh` builds and preserves the candidate, removes only the exact
attribute block in the disposable remote source, builds the parent, restores
the candidate source, profiles both binaries, and runs four uncounted warmup
pairs followed by 64 alternating measured pairs. A second independent block
uses the preserved binaries and the same four-warmup/64-pair protocol.

All 272 focused native proof processes, including warmups, exit zero with
empty stderr and exact 378-byte stdout SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
The two Callgrind proofs have that hash as well.

## Deterministic result

| Variant | Instructions | Native binary bytes |
| --- | ---: | ---: |
| Parent | 8,367,710,262 | 8,555,520 |
| Candidate | 8,389,064,593 | 8,573,272 |
| Delta | +21,354,331 (+0.255199%) | +17,752 (+0.207492%) |

The forced inline boundary therefore increases exact work and code size.

Raw profiles and metadata:

```text
.artifacts/experiments/2026-07-25-001-inline-term-arity-lto/callgrind-parent.out
.artifacts/experiments/2026-07-25-001-inline-term-arity-lto/callgrind-candidate.out
.artifacts/experiments/2026-07-25-001-inline-term-arity-lto/callgrind-instructions.txt
.artifacts/experiments/2026-07-25-001-inline-term-arity-lto/binary-size.csv
```

## Native production measurement

Negative deltas favor the candidate; positive deltas are regressions.

Block 1:

| Scope | Wall mean | CPU mean | Wall wins | CPU wins |
| --- | ---: | ---: | ---: | ---: |
| All 64 pairs | -0.062150% | -0.061529% | 33/64 | 33/64 |
| Final 32 pairs | -0.399414% | -0.396880% | 23/32 | 23/32 |

Block 2 reverses that nominal improvement:

| Scope | Wall mean | CPU mean | Wall wins | CPU wins |
| --- | ---: | ---: | ---: | ---: |
| All 64 pairs | +0.133478% | +0.133946% | 26/64 | 26/64 |
| Final 32 pairs | +0.134866% | +0.133920% | 13/32 | 13/32 |

Combined 128 pairs:

| Metric | Wall | CPU |
| --- | ---: | ---: |
| Mean delta | +0.035710% | +0.036255% |
| Paired mean delta | +0.042520% | +0.043066% |
| Median delta | +0.145371% | +0.141820% |
| Paired median delta | +0.123123% | +0.114807% |
| Candidate wins | 59/128 | 59/128 |

The combined final 32-pair halves nominally improve wall/CPU means by
`0.132875%`/`0.132072%`, but the blocks disagree and the whole 128-pair
sample's means, medians, paired metrics, and win counts all reject the
candidate.

Tracked measurements and reusable harnesses:

```text
experiments/2026-07-25-001-inline-term-arity-lto/native-warmup.csv
experiments/2026-07-25-001-inline-term-arity-lto/native-lusk.csv
experiments/2026-07-25-001-inline-term-arity-lto/native-warmup-2.csv
experiments/2026-07-25-001-inline-term-arity-lto/native-lusk-2.csv
experiments/2026-07-25-001-inline-term-arity-lto/measure_pairs.py
experiments/2026-07-25-001-inline-term-arity-lto/analyze_pairs.py
experiments/2026-07-25-001-inline-term-arity-lto/remote_measure.sh
```

## Falsification checks and limits

- Parent and candidate were built with the same compiler and release profile
  on one unchanged dedicated worker.
- The parent binary hash exactly matches the accepted comprehensive baseline.
- Proof bytes, exit status, and stderr are exact across both variants.
- Alternating order prevents a fixed first-run advantage.
- Two independent blocks disagree in direction, so neither block's favorable
  tail is treated as a stable win.
- The comprehensive parent/candidate compatibility mismatch sequences are
  identical; the remaining Linux mismatches are real follow-up work, not an
  effect of this attribute.

## Decision

Reject and restore accepted Experiment 301 source byte-for-byte. Forced
inlining of `Term::arity` regresses exact instructions and binary size, while
two native blocks provide no stable compensating throughput improvement. The
accessor remains closed under the current fat-LTO/single-CGU profile.
