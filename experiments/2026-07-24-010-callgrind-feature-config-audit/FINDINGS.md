# Experiment 283: Callgrind feature-configuration audit

## Status

Completed diagnostic for Bead `E_Rust_Port-j76.5.3`.

## Question

Did Experiments 279 through 282 compare candidate `--all-features` release
profiles against the accepted default-feature release profile?

## Setup

- Source: commit `2668987b`; production source is byte-identical to accepted
  Experiment 270.
- Archived accepted default-feature profile: 8,992,812,925 instructions.
- Fresh default-feature build/profile:
  `target-wsl-283-default-config-audit` and
  `.artifacts/experiments/2026-07-24-010-callgrind-feature-config-audit/rust-callgrind-default-config-audit.out`.
- Fresh all-feature build/profile:
  `target-wsl-283-all-features-config-audit` and
  `.artifacts/experiments/2026-07-24-010-callgrind-feature-config-audit/rust-callgrind-all-features-config-audit.out`.
- Exact LUSK6 workload and command-line options are otherwise identical.

## Results

The fresh default-feature profile retires 8,991,960,325 instructions, only
852,600 or 0.009481% below the archived accepted profile. This is a suitable
fresh control for candidate correction runs.

The unchanged all-feature build retires 9,078,864,096 instructions:
86,903,771 or 0.966461% above the fresh default build. Features such as
unification measurement add hot-path work, so an all-feature candidate cannot
be compared to the default-feature accepted baseline.

Experiments 279 through 282 were built with `--all-features` for their compact
Callgrind runs. Their functional tests, strict all-feature Clippy, native
all-feature binary comparisons, and proof-object comparisons remain valid,
but their whole-program Callgrind rejection decisions must be rerun in the
default feature configuration.

## Decision

Use 8,991,960,325 as the fresh correction control and rerun Experiments 279
through 282 in order. Do not carry any of their prior whole-program rejection
metrics into the performance decision. Correct each durable finding and Beads
note after its default-feature candidate result is known.

## Reproduction

```bash
cargo build --locked --release --bin eprover \
  --target-dir target-wsl-283-default-config-audit
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-default-config-audit.out \
  target-wsl-283-default-config-audit/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
