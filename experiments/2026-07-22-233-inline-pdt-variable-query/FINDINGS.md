# Rejected forced inline PD-tree variable-query step

## Question

Does forcing the single-caller `advance_variable_query` helper into the
const-specialized PD-tree cursor reduce whole-prover work without changing the
query traversal state machine?

## Setup

- Parent source: commit `298e0d64` (`Specialize PD-tree search cursor mode`),
  accepted Experiment 231.
- Candidate: add only a measured `#[inline(always)]` directive and its narrow
  Clippy justification to `advance_variable_query`.
- Deterministic workload: upstream `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.
- Parent profile:
  `.artifacts/experiments/2026-07-22-231-specialize-pdt-cursor/rust-callgrind-specialize-pdt-cursor.out`.
- Candidate profile:
  `.artifacts/experiments/2026-07-22-233-inline-pdt-variable-query/rust-callgrind-inline-pdt-variable-query.out`.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, profiled at
  5,254,361,329 instructions.

## Deterministic result

The candidate preserves the exact 4,873-processed-clause LUSK6 proof and
retires 9,888,238,514 instructions. This is 35,326,258 below the
9,923,564,772-instruction parent, a 0.355984% whole-prover reduction. The
hypothetical C/Rust ratio improves from 1.888634 to 1.881911.

The standalone helper disappears. The comparable cursor aggregate falls from
1,709,361,574 to 1,673,857,611 instructions, saving 35,503,963 or 2.077031%.
That localized reduction explains the whole-program change; traversal counts,
the exact proof, and the executable size remain unchanged.

## Native falsification

Four warmup pairs and two independent 64-pair alternating Windows blocks use
fixed parent and candidate binaries. All 256 measured processes prove and exit
zero. The first block is unstable: its first 32 pairs improve wall and CPU
means by 1.969856% and 1.778846%, while its last 32 regress by 1.928491% and
1.523367%. Its full means remain slightly positive at 0.095411% wall and
0.186730% CPU, but mean paired changes already regress 0.201695% and
0.114962%.

The independent second block confirms the regression. Wall and CPU means
regress 0.799818% and 0.732747%; mean paired changes regress 0.908538% and
0.824324%. The candidate wins only 30 wall pairs and 25 CPU pairs, with five
CPU ties.

Across all 128 pairs per binary, parent versus candidate wall means are
1.947112 versus 1.953685 seconds, a 0.337617% candidate regression. CPU means
are 1.896851 versus 1.901733 seconds, a 0.257417% regression. Wall and CPU
medians regress 2.240515% and 2.127660%. Both executables are 8,654,336 bytes.

## Validation

- All 41 focused PD-tree tests pass with the candidate.
- Strict all-feature library pedantic Clippy and formatting pass.
- The candidate produces the exact LUSK6 proof and exit zero under Callgrind.
- Source is restored byte-for-byte; focused tests and formatting pass again.
- Compatibility and full repository gates are skipped after replicated native
  rejection.
- The vendored C checkout is unchanged.

## Decision

Reject forced `advance_variable_query` inlining and restore Experiment 231.
The instruction reduction is real and localized, but two 64-pair blocks show
worse production timing overall, with the replication and combined sample
both negative. Preserve the two CSVs to prevent accepting the deterministic
proxy over contradictory native evidence. The accepted baseline remains
9,923,564,772 instructions, or 1.888634 times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-inline-pdt-variable-query.out \
  target-wsl-233-inline-pdt-variable-query/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-231-specialize-pdt-cursor\release\eprover.exe `
  -CandidateExe .\target\native-233-inline-pdt-variable-query\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-22-233-inline-pdt-variable-query\native-lusk.csv
```
