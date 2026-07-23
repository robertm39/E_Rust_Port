# Experiment 247: Retain bounded diversity function-code output

## Question

Can the diversity WFCB retain only `Clause::return_fcodes`' output vector,
eliminating its 90,343 one-growth allocations without retaining the complete
function-subterm traversal stack rejected by Experiment 139?

## Baseline

- Accepted source: Experiment 245, commit `e4555196`.
- Exact LUSK6 Callgrind: 9,898,434,766 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.883851.
- `Clause::return_fcodes` owns exactly 90,343 `RawVec` growth calls in the
  accepted profile, one per diversity evaluation.
- Accepted whole-program Rust allocator calls: 4,290,002.

## Candidate

Add a function-code output vector to the existing `DiversityWeightScratch`.
The WFCB path clears and reuses this vector between evaluations and drops
capacity above the existing 1,024-entry scratch limit.

The complete `Clause::collect_subterms` stack and its operation flags remain
fresh inside `Clause::return_fcodes`. Variable traversal remains independent,
so stale `TP_OP_FLAG` state cannot alter variable diversity. Public
`diversity_weight_compute` also retains its compatibility-oriented
operation-local storage.

## Validation

- All six focused diversity tests pass for the candidate. The scratch
  regression verifies repeated function counts, retained capacity, empty
  post-call storage, and independence from stale variable operation flags.
- Strict all-feature library pedantic Clippy and formatting pass.
- The candidate reaches the exact 4,873-processed-clause LUSK6 proof and exits
  zero under Callgrind.
- Four alternating warmup pairs and 64 measured native pairs all prove and
  exit zero.
- After native rejection, production source/test changes are restored
  byte-for-byte. All six accepted diversity tests and formatting pass again.
- Compatibility and resource matrices were skipped after decisive native
  rejection.

## Measurement

The candidate retires 9,877,531,675 instructions, 20,903,091 below the
9,898,434,766-instruction parent. This is a 0.211176% whole-prover reduction,
and the hypothetical Rust/C ratio improves from 1.883851 to 1.879873.

Coalesced `RawVec` growth calls fall from 545,710 to 455,368, and Rust
allocator calls fall from 4,290,002 to 4,199,660. Both reductions are exactly
90,342: every targeted evaluation after the retained vector's first
allocation. Total allocator calls fall 2.105873%.
`diversity_weight_compute_reusing_scratch` falls from 164,261,951 to
163,087,492 instructions, down 1,174,459 or 0.714992%.
`Clause::return_fcodes` falls from 41,734,173 to 41,463,147, down 271,026 or
0.649410%.

Native production evidence reverses the deterministic result. Across 64
alternating pairs, candidate wall mean regresses 1.864135% and process-CPU
mean regresses 1.581778%. Wall and CPU medians regress 1.395086% and
1.020408%; mean paired wall and CPU changes regress 1.899339% and 1.627086%.
The candidate wins only 17 wall and 17 CPU pairs, with eight CPU ties.

The stable last 32 pairs are stronger: wall and CPU means regress 1.790048%
and 2.114707%, medians regress 1.270705% and 1.538462%, and mean paired
changes regress 1.807103% and 2.151798%. The candidate wins only seven wall
and seven CPU pairs, with two CPU ties. Both executables are exactly 8,654,336
bytes.

## Result

Reject. Reusing only the bounded output vector removes the measured
allocations and improves exact instrumented work, but the production binary
regresses consistently across means, medians, paired statistics, and the
stable half. Preserve operation-local function-code output along with the
already operation-local subterm traversal stack. Production source is
restored exactly to Experiment 245 at 9,898,434,766 instructions, or 1.883851
times C.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-bounded-diversity-fcodes.out \
  target-wsl-247-bounded-diversity-fcodes/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-245-single-maximal-candidate-vector\release\eprover.exe `
  -CandidateExe .\target\native-247-bounded-diversity-fcodes\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-009-bounded-diversity-fcodes\native-lusk.csv
```
