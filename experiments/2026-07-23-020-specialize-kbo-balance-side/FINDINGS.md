# Experiment 258: Specialize the KBO balance side

## Status

Rejected in Experiment 258 for Bead `E_Rust_Port-j76.5.3`.

## Hypothesis

The accepted LUSK6 profile assigns 193,087,516 exclusive instructions to the
first-order KBO6 `mfy_vwb` balance walker. Original C implements separate
`mfyvwblhs` and `mfyvwbrhs` functions, so weight and variable-balance direction
is statically known throughout each traversal. Rust passes a Boolean into one
shared function and branches on it for every variable and function node.

Monomorphize only the first-order walker on a const side flag. Preserve the
reusable traversal stack, dereference state, argument order, variable-bank
growth, weight arithmetic, and the separate LFHO/lambda paths.

## Baseline

Accepted Experiment 245:

- Rust instructions: 9,898,434,766
- C instructions: 5,254,361,329
- Rust/C ratio: 1.883851
- accepted `mfy_vwb`: 193,087,516 exclusive instructions

## Candidate

Use a const side parameter for the first-order walker; LFHO and lambda paths
remain unchanged. All 20 focused KBO6 tests, strict library Clippy, formatting,
and the exact proof pass.

The candidate retires 9,888,092,261 instructions, down 10,342,505 or
0.104486%; the hypothetical C ratio improves to 1.881883. Specialized left
and right walkers cost 105,233,251 and 77,104,257 instructions, totaling
182,337,508. That is 10,750,008 or 5.567428% below the accepted
193,087,516-instruction shared walker. The Windows binary grows 2,048 bytes.

After four warmup pairs, 64 alternating native pairs decisively reverse the
instruction gain. Mean paired wall/CPU regress 1.699411%/2.112909%, aggregate
wall/CPU regress 1.648178%/2.056936%, and the candidate wins only 9 wall and
14 CPU pairs. The stable last 32 regress 3.010346%/3.046037%, with only one
wall win and five CPU wins.

Reject and restore the shared Boolean walker. Accepted Experiment 245 remains
the baseline. Compatibility/resource matrices are skipped after native
failure. Measured samples are in `native-lusk.csv`; ignored raw artifacts are:

```text
.artifacts/experiments/2026-07-23-020-specialize-kbo-balance-side/rust-callgrind-specialize-kbo-balance-side.out
.artifacts/experiments/2026-07-23-020-specialize-kbo-balance-side/native-warmup.csv
```
