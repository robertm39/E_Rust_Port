# Experiment 281: Specialize no-documentation forward modification

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the production saturation path statically select the ordinary
`ForwardModifyClause` implementation instead of routing every generated clause
through the runtime proof-documentation `Option` dispatcher?

## Setup

- Parent source: commit `12d78512` (`perf: reject borrowed rewrite sequence
  arguments`); executable source remains accepted Experiment 270.
- Fresh unchanged-source default-feature control: 8,991,960,325
  instructions.
- Archived accepted default-feature profile: 8,992,812,925 instructions.
- Representative optimized line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: const-specialize the generated-clause admission and
  forward-contraction implementations on whether a proof-documentation session
  exists. Their shared forward-modification dispatcher calls the ordinary
  implementation directly in the production specialization and preserves the
  documented implementation in the opt-in specialization.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The accepted profile sends 121,036 ordinary forward modifications through the
generated-clause loop plus 5,130 through forward contraction. The complete
generated-clause admission owner accounts for 4,705,517,773 inclusive
instructions, so this experiment changes only its repeated documentation
dispatch boundary, not contraction semantics.

## Results

The original 9,069,312,582-instruction candidate profile was accidentally
built with `--all-features` and is not comparable with the default-feature
accepted baseline. A configuration audit measured the unchanged source at
9,078,864,096 instructions with all features versus 8,991,960,325 with
default features, a build-configuration penalty of 86,903,771 instructions
or 0.966461%. The original rejection metric is therefore superseded.

The corrected default-feature candidate executes 8,957,241,257 instructions.
That is 34,719,068 or 0.386112% below the fresh unchanged-source control and
35,571,668 or 0.395557% below the archived accepted profile. Its hypothetical
ratio to the 5,254,361,329-instruction C reference improves to 1.704725.

The first repeated native check used an all-feature candidate against the
default-feature parent and is superseded by Experiment 285's native feature
audit. The matched default-feature candidate is 8,937,472 bytes, 14,848 bytes
smaller than the 8,952,320-byte parent. Three parent and eight candidate
direct proof runs are all byte-identical and exit zero. The production
candidate is proof-compatible.

Native timing reverses the deterministic win. Two independent blocks each
exclude four alternating warmup pairs and retain 64 alternating measured
pairs:

| Block | Wall mean | CPU mean | Wall wins | CPU wins | CPU ties |
| --- | ---: | ---: | ---: | ---: | ---: |
| First 64 pairs | -0.251801% | -0.081833% | 31 | 29 | 4 |
| Second 64 pairs | +1.010134% | +1.154925% | 21 | 21 | 2 |
| Combined 128 pairs | +0.376725% | +0.534056% | 52 | 50 | 6 |

Negative percentages are candidate improvements and positive percentages are
regressions. The first block's tiny full-sample gains are front-loaded: its
final 32 pairs regress 0.607913% wall and 0.831947% CPU, while its final 16
regress 2.258310% and 2.711864%.

The combined final halves regress 1.100277% wall and 1.293747% CPU by
aggregate means. Combined wall and CPU medians regress 0.422381% and
2.105263%; mean paired wall and CPU changes regress 0.640535% and 0.793799%.

## Validation

- All 219 proof-control tests pass in default and all-feature configurations.
- Strict all-feature library pedantic Clippy and formatting pass.
- Corrected default-feature WSL Callgrind proves LUSK6 and exits zero.
- The matched candidate fingerprint records exactly `features=["default"]`.
- Three parent and eight candidate direct native proof runs are byte-identical
  and exit zero.
- All 256 matched-feature measured timing processes and 16 warmup processes
  prove and exit zero.
- The full maintained compatibility matrix is skipped after replicated native
  timing rejects the performance-only change.
- After rejection, the const parameters and specialized dispatch are removed
  and accepted `proofcontrol.rs` is restored byte-for-byte.

## Decision

Reject. Const-specializing the ordinary documentation path improves corrected
default-feature instructions by 0.386112% and is proof-exact in the matched
production build, but native wall and CPU timing regress across the combined
sample and stable halves. Keep Experiment 270 as the accepted baseline at
8,992,812,925 instructions, or 1.711495 times C.

The no-documentation specialization boundary is exhausted because its
production timing does not reproduce the deterministic instruction win.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-008-specialize-no-doc-forward-modify/rust-callgrind-specialize-no-doc-forward-modify-default-corrected.out \
  target-wsl-281-corrected-no-doc-specialization/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-281-default-no-doc-specialization\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-008-specialize-no-doc-forward-modify\native-lusk-default.csv
```

Run the native command twice independently; the second retained block is
`native-lusk-default-2.csv`.
