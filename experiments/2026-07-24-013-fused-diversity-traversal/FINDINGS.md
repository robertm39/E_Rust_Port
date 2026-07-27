# Fused diversity traversal

## Question

Can the private production `Diversityweight` WFCB count distinct function
codes and variables in one operation-flag traversal, while preserving the
public C-shaped helpers and the accepted stale-variable behavior?

## Baseline

- Accepted source: Experiment 270, commit `a4519370`.
- Archived default-feature exact LUSK6 Callgrind: 8,992,812,925 instructions.
- Fresh unchanged-source default-feature control: 8,991,960,325 instructions.
- Original C exact LUSK6 Callgrind: 5,254,361,329 instructions.
- Accepted Rust/C ratio: 1.711495.
- Validated line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.

The line profile records 90,343 production diversity evaluations.
`diversity_weight_compute_reusing_scratch` retires 446,913,051 instructions,
including 265,728,045 in `Clause::return_fcodes`. Its separate variable
traversal accounts for most of the remaining subtree; shared literal weighting
is comparatively small.

## Candidate

Leave `Clause::return_fcodes`, `Clause::collect_variables`, and the public
immutable diversity helper unchanged. The private WFCB path uses a single
operation-local subterm stack:

- non-variable shared terms retain the existing `TP_OP_FLAG` visited behavior;
- free variables are recorded independently of `TP_OP_FLAG`, so stale variable
  flags cannot suppress the variable count;
- all flags set on collected non-variable terms are cleared after the walk;
- distinct function codes retain the existing `BTreeSet` count semantics; and
- only the bounded variable-ID vector remains retained by the WFCB.

This specifically avoids the rejected Experiment 139 fusion, which reused the
operation flag for variables and changed proof search.

## Result

Accepted for Bead `E_Rust_Port-j76.5.3`.

### Deterministic profile

The default-feature candidate fingerprint records exactly
`features=["default"]`. It proves LUSK6 and retires 8,828,399,104
instructions:

| Comparison | Delta | Percent |
| --- | ---: | ---: |
| Fresh default-feature control, 8,991,960,325 | -163,561,221 | -1.818972% |
| Archived accepted profile, 8,992,812,925 | -164,413,821 | -1.828280% |

The hypothetical Rust/C ratio improves from 1.711495 to 1.680204. The
candidate's smaller traversal is fully inlined in this release build, so
Callgrind does not retain a separate source-level diversity function owner;
the whole-program result and exact proof are the stable comparison boundary.

### Proof determinism and native layout

The matched native candidate fingerprint also records exactly
`features=["default"]`. The executable is 8,964,608 bytes, 12,288 bytes
larger than the 8,952,320-byte parent.

Three parent and eight candidate direct native runs all exit zero and emit the
same 33,637-character proof object with SHA-256
`14aa78066797e58a0b40b44122654b44c64930b037412ebe88bae537f4d27a89`.

### Native timing

Two independent native blocks each discard four alternating warmup pairs and
retain 64 alternating measured pairs. Negative percentages are candidate
improvements.

| Sample | Wall mean | CPU mean | Wall median | CPU median | Wall wins | CPU wins | CPU ties |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| First 64 pairs | -1.546921% | -1.384417% | -1.868917% | -2.061856% | 38 | 33 | 7 |
| Second 64 pairs | -1.679356% | -1.599716% | -2.245906% | -1.734104% | 48 | 41 | 3 |
| Combined 128 pairs | -1.610070% | -1.486738% | -1.808386% | -1.075269% | 86 | 74 | 10 |

Combined mean paired wall and CPU changes improve 1.418319% and 1.315310%.
The combined final halves remain favorable: aggregate wall and CPU means
improve 0.719267% and 0.546635%, and mean paired changes improve 0.550805%
and 0.403506%. The combined final quarters improve aggregate wall and CPU
means 1.036969% and 0.716724%.

The first block's final-32 CPU mean was effectively flat at +0.064872%, which
triggered the independent replication. The second block's final 32 pairs
improve wall and CPU means 0.816761% and 1.226994%, and its final 16 improve
1.199996% and 1.440922%.

### Compatibility

The maintained C-vs-Rust report at
`.artifacts/e-compare/20260724-084321-259845/` completes all 50 cases with
zero unexpected mismatches and one declared `sledgehammer.p`
`normalized_stdout` difference. Both sides still report `Theorem` and exit
zero for that declared case.

## Validation

- six focused diversity tests pass, including repeated fused counts with a
  stale `TP_OP_FLAG` on the shared variable;
- default-feature WSL and native release fingerprints both record exactly
  `features=["default"]`;
- Callgrind and every native timing process prove and exit zero;
- three parent and eight candidate native proof objects are byte-identical;
- all 4,393 library tests and every binary/integration target pass with all
  features; the first concurrent link attempt exhausted the Windows paging
  file, and the identical suite passed serially with `-j 1`;
- strict all-target/all-feature pedantic Clippy passes serially;
- formatting and diff checks pass; and
- the vendored C checkout remains unchanged.

## Decision

Accept. The private production path now shares one operation-local function
subterm traversal with variable discovery while retaining public C-shaped
helpers, exact operation-flag side effects, and bounded retained variable-ID
storage. It removes 1.82% of deterministic instructions and produces
replicated 1.49-1.61% native CPU/wall improvements without changing proof or
resource behavior.

## Reproduction

```bash
cargo build --locked --release --bin eprover \
  --target-dir target-wsl-286-fused-diversity-traversal

valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-013-fused-diversity-traversal/callgrind-candidate.out \
  target-wsl-286-fused-diversity-traversal/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-286-fused-diversity-traversal\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-013-fused-diversity-traversal\native-lusk.csv
```

The independent replication is retained as `native-lusk-2.csv`.
