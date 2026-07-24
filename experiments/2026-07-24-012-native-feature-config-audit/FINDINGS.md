# Experiment 285: Native feature-configuration audit

## Status

Completed diagnostic for Bead `E_Rust_Port-j76.5.3`.

## Question

Did corrected native checks for Experiments 279 through 282 compare
all-feature candidate binaries against the accepted default-feature parent,
and can that mismatch change proof output or production timing?

## Setup

- Source: commit `575edcf7`; production source is byte-identical to accepted
  Experiment 270.
- Accepted parent:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- The accepted parent's Cargo fingerprint records exactly
  `features=["default"]`.
- Audit candidate: unchanged source built with `--all-features`.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

## Results

The Cargo fingerprints establish the mismatch:

- accepted parent: `features=["default"]`;
- audit candidate: `features=["default", "instrument-perf-ctr",
  "measure-expensive", "measure-unification", "pdt-count-nodes",
  "print-index-stats"]`.

The unchanged all-feature executable is 9,012,224 bytes versus 8,952,320 for
the accepted default-feature executable. The 9,012,224-byte size exactly
matches the size previously attributed to Experiment 279's corrected native
candidate, confirming that candidate was not built in the parent's production
configuration.

Three default-feature and five all-feature direct proof runs are all
byte-identical and exit zero. The feature set does not change the unchanged
source's proof on this gate.

After four alternating warmup pairs, 32 alternating measured pairs show the
all-feature configuration penalty:

- wall and CPU means regress 8.832227% and 8.972648%;
- wall and CPU medians regress 9.127234% and 10.000000%;
- mean paired wall and CPU changes regress 8.871555% and 9.006131%;
- the all-feature binary wins one wall pair and zero CPU pairs, with one CPU
  tie;
- all 64 measured processes exit zero and prove the expected result.

This penalty subsumes the previously recorded native regressions for
Experiment 279 (7.831316% wall and 7.383242% CPU) and Experiment 282
(9.347007% wall and 10.652976% CPU). Those measurements compared an
all-feature candidate to the default-feature parent and are invalid candidate
effects.

Experiments 280 and 281 also used all-feature candidate binaries for their
repeated proof checks. Although the unchanged all-feature audit is
proof-stable, production compatibility must be decided from a default-feature
candidate because code layout and instrumentation interact with each source
change.

## Decision

Rerun the native proof and timing gates for Experiments 279 through 282 using
candidate fingerprints that record exactly `features=["default"]`. Preserve
their corrected default-feature WSL Callgrind profiles and focused functional
tests. Supersede every native timing and repeated proof-output conclusion
derived from the all-feature candidates.

## Reproduction

```powershell
cargo build --locked --release --all-features --bin eprover `
  --target-dir target/native-285-all-features-audit

& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-285-all-features-audit\release\eprover.exe `
  -Pairs 32 `
  -OutputCsv .\experiments\2026-07-24-012-native-feature-config-audit\native-lusk-all-features-audit.csv
```
