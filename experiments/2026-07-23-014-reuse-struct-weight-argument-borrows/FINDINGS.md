# Experiment 252: Reuse structural-weight argument borrows

## Status

Rejected in Experiment 252 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

`term_struct_weight_compare` currently borrows each nonvariable term's
`RefCell`-backed argument vector to read arity and then borrows it again for
recursive traversal. Experiment 251 measured 711,744 arity accessor calls,
while more than 98% of the affected comparisons immediately borrow both
argument slices.

Borrow each argument slice once, derive arity from its length, and retain the
immutable borrows for the existing lexicographic traversal. This matches C's
single-cell access shape without changing ordering, recursion, or normalized
comparison results.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851

## Candidate

Move the existing immutable `arguments()` borrows before the arity check and
compare `left_args.len()` with `right_args.len()`. The recursive loop then
uses those same borrows. No function-code, variable, weight, type, ordering,
or recursion logic changes.

A focused regression constructs:

- equal-weight terms of arity one and two, proving exact `-1`/`1` normalized
  arity results;
- equal-weight unary parents whose children differ by that arity shape,
  proving that retaining the parent argument borrows across recursion is valid.

## Validation

- The focused library test passes.
- Strict library pedantic Clippy passes.
- Formatting and `git diff --check` pass.
- The exact LUSK6 Callgrind run proves `Unsatisfiable` and exits zero.
- Three independent native blocks each use four alternating warmup pairs and
  64 alternating measured pairs. All 24 warmup processes and all 384 measured
  processes prove and exit zero.
- The candidate executable is 8,654,336 bytes, identical in size to the
  accepted executable.

A broad default test command encountered Windows paging-file error 1455 while
compiling an unrelated binary after the focused library test had passed. The
focused slice was rerun explicitly with one build job and passed. Full
repository gates are not repeated after decisive native rejection.

## Deterministic measurement

The candidate retires 9,859,782,941 instructions, 38,651,825 below the
9,898,434,766-instruction parent. This is a 0.390484% whole-prover reduction,
and the hypothetical Rust/C ratio improves from 1.883851 to 1.876495.

The reduction is larger than the direct cost of 711,744 removed `arity()`
accessor calls. The release profile no longer retains a separate structural
weight-comparator symbol, indicating that the smaller borrow shape also
changed inlining or surrounding code generation.

## Native measurement

Production Windows timing reverses the deterministic result:

| Sample | Wall mean | CPU mean | Wall wins | CPU wins | CPU ties |
| --- | ---: | ---: | ---: | ---: | ---: |
| Block 1, 64 pairs | +0.721860% | +1.297134% | 26 | 24 | 7 |
| Block 2, 64 pairs | +0.093219% | +0.253727% | 26 | 24 | 11 |
| Block 3, 64 pairs | +1.452544% | +1.539942% | 18 | 17 | 9 |
| Combined, 192 pairs | +0.753032% | +1.032864% | 70 | 65 | 27 |

Positive percentages are candidate regressions. Mean paired changes across
all 192 pairs are +0.791632% wall and +1.053808% CPU. Combined medians regress
0.123398% wall and tie on the quantized CPU clock.

The host changed performance state during the first block, so each block's
last 32 pairs were also combined. That stable 96-pair aggregate still
regresses wall and CPU means by 0.317105% and 0.277748%; mean paired changes
regress 0.348425% and 0.315313%. The candidate wins only 38 stable wall pairs
and 36 stable CPU pairs, with 16 CPU ties. Block three's own stable half is
clearly worse at +0.933248% wall and +1.002912% CPU, ruling out acceptance as
a temperature-sensitive tie.

## Result

Reject. Reusing the argument borrow is semantically sound and improves the
instrumented Linux instruction count, but three production blocks establish
a Windows wall/CPU regression. Preserve the former separate `arity()` and
`arguments()` access shape.

Candidate production source and its focused test are removed. The accepted
baseline remains Experiment 245 at 9,898,434,766 instructions, or 1.883851
times C. Compatibility and resource matrices are skipped after replicated
native rejection.

## Artifacts

- Callgrind:
  `.artifacts/experiments/2026-07-23-014-reuse-struct-weight-argument-borrows/rust-callgrind-reuse-struct-weight-argument-borrows.out`
- Native blocks: `native-lusk.csv`, `native-lusk-2.csv`, and
  `native-lusk-3.csv`.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-reuse-struct-weight-argument-borrows.out \
  target-wsl-252-reuse-struct-weight-argument-borrows/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-245-single-maximal-candidate-vector\release\eprover.exe `
  -CandidateExe .\target\native-252-reuse-struct-weight-argument-borrows\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-014-reuse-struct-weight-argument-borrows\native-lusk.csv
```
