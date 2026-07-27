# Experiment 288: Generation-marked diversity function codes

## Status

In progress for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the fused private production diversity traversal count distinct function
codes with signature-indexed generation marks, eliminating a fresh
`BTreeSet` and its comparison/tree-node work for every clause evaluation?

## Baseline

- Parent commit: `9030d336`; executable source is accepted Experiment 286.
- Exact default-feature LUSK6 Callgrind: 8,828,399,104 instructions.
- Original C Callgrind: 5,254,361,329 instructions.
- Rust/C ratio: 1.680204.
- Native parent:
  `target/native-286-fused-diversity-traversal/release/eprover.exe`.

## Candidate

The private WFCB scratch retains a dense `u32` generation vector indexed by
nonvariable function code. Each evaluation advances one nonzero generation;
the vector is cleared only on generation wrap. The existing operation-flag
walk still records every visited nonvariable term so all flags are cleared,
and stale variable flags remain independent. Public ordered function-code
collection and immutable diversity helpers are unchanged.

The focused repeated-count regression now uses `f(X)=f(a)`, covering two
distinct shared `f` cells with one function code, a second constant code, and
one stale-flagged variable.

## Result

Rejected for Bead `E_Rust_Port-j76.5.3`.

### Deterministic profile

The default-feature candidate proves LUSK6 and retires 8,779,205,530
instructions, 49,193,574 or 0.557220% below accepted Experiment 286. The
hypothetical Rust/C ratio improves from 1.680204 to 1.670842.

### Proof determinism and native layout

The WSL and native candidate fingerprints both record exactly
`features=["default"]`. The native executable shrinks 33,280 bytes, from
8,964,608 to 8,931,328 bytes.

Three parent and eight candidate native proof-output runs all exit zero and
emit the same 10,024-character output with SHA-256
`fea5c4b25a841dfe9de0bc50879dd47c8b5674a4c7786e70f4999812758fc408`.

### Native timing

Two independent blocks each discard four alternating warmup pairs and retain
64 measured pairs. Positive percentages are candidate regressions.

| Sample | Wall mean | CPU mean | Wall median | CPU median | Wall wins | CPU wins | CPU ties |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| First 64 pairs | +0.892748% | +0.895994% | +0.464116% | +1.123596% | 24 | 25 | 3 |
| Second 64 pairs | +0.563825% | +0.165593% | +1.252852% | +1.190476% | 23 | 21 | 8 |
| Combined 128 pairs | +0.731840% | +0.539229% | +1.089246% | +1.176471% | 47 | 46 | 11 |

Combined mean paired wall and CPU changes regress 0.856000% and 0.639347%.
The combined final halves regress wall/CPU means 1.038503%/0.850277%, and
the combined final quarters regress 1.773786%/1.983533%. Only 22/21 of the
64 final-half pairs are candidate wall/CPU wins, with eight CPU ties.

## Validation

- all six focused diversity tests pass, including repeated counts with two
  distinct shared cells carrying the same function code;
- strict all-feature library pedantic Clippy and formatting pass;
- default-feature WSL and native fingerprints are exact;
- Callgrind, all direct proof checks, both warmup blocks, and all 256 measured
  native processes prove and exit zero;
- accepted source is restored byte-for-byte; and
- the vendored `eprover/` checkout remains unchanged.

## Decision

Reject. Dense generation marks improve exact instructions and binary size but
reproduce a native throughput regression in two independent blocks and in
stable tails. Preserve the accepted fused `BTreeSet` implementation from
Experiment 286. The full compatibility matrix and repository-wide gates are
skipped after the decisive production-native rejection.

Raw evidence:

- `.artifacts/experiments/2026-07-24-015-generation-marked-diversity-fcodes/callgrind-candidate.out`
- `.artifacts/experiments/2026-07-24-015-generation-marked-diversity-fcodes/native-warmup.csv`
- `.artifacts/experiments/2026-07-24-015-generation-marked-diversity-fcodes/native-warmup-2.csv`
