# Experiment 293: Fuse the always-dereference applied-variable check

## Question

Can the already forced-inline `deref_always_step` test its applied-variable
head directly, removing the dominant outlined predicate edge without
duplicating `Term::is_applied_free_var` across all 30 callers?

## Baseline

- Accepted executable source: Experiment 290.
- Current commit before the candidate: `365403fd`.
- Exact default-feature LUSK6 Callgrind: 8,800,386,737 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.674873.
- The accepted profile calls standalone `Term::is_applied_free_var` 8,945,092
  times. `Substitution::norm_term` owns the largest edge at 2,550,102 calls
  and 28,051,122 inclusive instructions.

## Candidate

Inside only `deref_always_step`, replace the public predicate followed by a
second head lookup with one fused expression:

- retain the exact `is_phony_app` classification;
- borrow argument zero once;
- require that head to be a free variable with an active binding; and
- retain the existing expansion helper and all other callers unchanged.

The expression is semantically the conjunction already computed by the two
accepted checks. It is in the existing forced-inline step, so the dominant
normalizer can eliminate its call edge while avoiding Experiment 292's global
30-caller inlining boundary.

## Validation

- All 18 focused term-cell and dereference tests pass.
- Formatting and `git diff --check` pass.
- Candidate WSL and native fingerprints record exactly
  `features=["default"]`.
- The candidate reaches the exact 4,873-processed-clause LUSK6 proof and exits
  zero under Callgrind.
- Three parent and eight candidate native proof runs are byte-identical:
  378-byte stdout with SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
- All 256 measured native processes prove successfully and exit zero.
- The maintained report
  `.artifacts/e-compare/20260724-142510-164927` has 50 cases, zero
  mismatches, and the one declared `sledgehammer.p` difference.
- The complete serial suite passes 4,394 library tests plus every binary and
  integration target under all features.
- Strict all-target/all-feature pedantic Clippy passes.
- The locked all-feature release `eprover` build passes.
- Formatting, `git diff --check`, all four C-source documentation gates, and
  vendored-C cleanliness pass.

## Measurement

The candidate retires 8,718,487,029 instructions, 81,899,708 below the
8,800,386,737-instruction parent. This is a 0.930638% whole-prover reduction,
and the Rust/C ratio improves from 1.674873 to 1.659286.

The standalone `Term::is_applied_free_var` owner falls from 98,396,012 to
14,797,871 instructions. `Substitution::norm_term` itself changes from
443,318,643 to 444,445,091, so the comparable pair falls by 82,470,693
instructions. The dominant PD-tree cursor reproduces exactly at
1,560,083,792 instructions, confirming that proof search is unchanged.

The WSL binary shrinks 20,552 bytes. The native binary shrinks from 8,931,840
to 8,928,256 bytes, a 3,584-byte reduction.

Four alternating warmup pairs preceded two independent blocks of 64
alternating measured pairs:

| Native metric | Block 1 | Block 2 | Combined 128 |
| --- | ---: | ---: | ---: |
| Wall mean | -0.135324% | -1.160269% | -0.649607% |
| CPU mean | -0.610433% | -0.829493% | -0.720155% |
| Paired wall mean | -0.014984% | -0.947289% | -0.481136% |
| Paired CPU mean | -0.562540% | -0.629194% | -0.595867% |
| Candidate wall wins | 31/64 | 37/64 | 68/128 |
| Candidate CPU wins/ties | 32/7 | 29/11 | 61/18 |

The combined final 32 pairs from each block are stronger:

- wall mean: -0.802468%;
- CPU mean: -1.107011%;
- paired wall mean: -0.629916%;
- paired CPU mean: -0.942747%;
- wins: 38/64 wall and 31/64 CPU, with 9 CPU ties.

Negative percentages are improvements. Unlike Experiment 292's global
annotation, the selective fused check improves the combined whole blocks and
their combined stable halves.

## Result

Accept. The fused check preserves the public macro-shaped predicate while
removing its dominant redundant call boundary from the always-dereference
step. It improves exact instructions by 0.930638%, improves combined native
wall and CPU throughput with stronger stable halves, preserves exact proof
output and the complete compatibility matrix, and passes every repository
gate.

Keep the broader performance Bead open: the accepted deterministic ratio is
now 1.659286 times C rather than comparable performance.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-fused-always-deref-app-check.out \
  target-wsl-293-fuse-always-deref-app-check/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-290d-divide-clause-sign-partition\release\eprover.exe `
  -CandidateExe .\target\native-293-fuse-always-deref-app-check\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-020-fuse-always-deref-app-check\native-lusk.csv
```
