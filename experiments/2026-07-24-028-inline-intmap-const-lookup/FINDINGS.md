# Experiment 301: Force-inline immutable IntMap lookup

## Status

Accepted experiment for Bead `E_Rust_Port-j76.5.3`.

## Question

Can forcing the immutable `IntMap<usize>` lookup into the first-order PD-tree
cursor remove a large remaining call boundary under the accepted fat-LTO and
single-CGU release profile, without changing `IntMap` representation,
compatibility behavior, or native throughput?

## Baseline

- Accepted production source and release profile: commit `51aa9926`
  (`perf: enable whole-program release optimization`).
- Documentation-only head before this experiment: commit `48313180`.
- Exact default-feature LUSK6 Callgrind: `8,400,364,984` instructions.
- Original FOL C Callgrind: `5,254,361,329` instructions.
- Exact Rust/C ratio: `1.598741`.
- Accepted native executable:
  `target/native-298-manifest-fat-lto-one-cgu/release/eprover.exe`.
- Deterministic workload: upstream `LUSK6.lop` with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.

Experiment 299's representative post-LTO profile attributes `111,096,998`
instructions to the monomorphized `IntMap<usize>::get_val_const`. It is called
`3,689,170` times; `3,686,541` calls come directly from the first-order
PD-tree matching cursor. The method already had ordinary `#[inline]`, but fat
LTO retained it as a standalone function. The experiment archive contains the
accepted PD-tree `IntMap` representation change and compatibility boundary,
but no forced-inline trial of this immutable lookup.

## Candidate

Change only the method's inline strength from `#[inline]` to
`#[inline(always)]`.

The candidate retains:

- all `Empty`, `Single`, `Array`, and `Tree` representation branches;
- the non-growing lookup behavior below a range-array offset;
- key bounds, `Option` handling, and return lifetime;
- both production callers and all mutation paths;
- the manifest's fat-LTO and single-CGU release profile.

The measured boundary carries the repository's established narrow
`clippy::inline_always` allowance with a benchmark-specific reason.

## Focused validation

- All 18 `IntMap`-filtered library tests pass, including const lookup,
  C-compatible mutating lookup, representation switching, deletion,
  iteration, and storage accounting.
- All 41 PD-tree-filtered library tests pass.
- Formatting and `git diff --check` pass.
- WSL and native fingerprints record exactly `features=["default"]`; the
  executable profile hash is the accepted `11264489599640293354`.
- Three parent and eight initial candidate native runs produce identical
  378-byte stdout with SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
  empty stderr, and exit zero.
- After adding the lint-only measured-inline allowance, the exact final source
  was rebuilt and three further candidate runs reproduced the same proof hash,
  length, empty stderr, and zero exit.
- All 256 measured native processes prove and exit zero.

## Deterministic measurement

The candidate retires `8,368,891,139` instructions, `31,473,845` below the
accepted parent. This is a `0.374672%` reduction, and the Rust/C ratio improves
from `1.598741` to `1.592751`.

The standalone `IntMap<usize>::get_val_const` owner disappears. The parent
attributes `1,489,925,474` instructions to the first-order PD-tree cursor and
`111,096,998` to the standalone lookup, or `1,601,022,472` combined. The
candidate's merged cursor owns `1,568,589,289`, a `32,433,183`-instruction
reduction at that boundary. Small layout changes elsewhere make the exact
whole-program improvement `31,473,845`.

The native executable shrinks from `8,617,472` to `8,615,424` bytes
(`-2,048` bytes).

Raw evidence:

```text
.artifacts/experiments/2026-07-24-028-inline-intmap-const-lookup/callgrind-inline-intmap-const-lookup.out
.artifacts/experiments/2026-07-24-028-inline-intmap-const-lookup/callgrind-inline-intmap-const-lookup.annotate.txt
```

## Native production measurement

Each independent block uses four uncounted warmup pairs followed by 64
alternating measured pairs. Negative deltas favor the candidate.

Block 1:

| Metric | Wall | CPU |
| --- | ---: | ---: |
| Mean delta | -1.237989% | -1.203125% |
| Paired mean delta | -1.028342% | -1.001948% |
| Median delta | -2.157964% | -1.030928% |
| Paired median delta | -1.346607% | -1.081113% |
| Candidate wins | 36/64 | 36/64 |

Its final 32 pairs improve wall/CPU means by `2.621861%` and `2.725621%`,
paired means by `2.534432%` and `2.626950%`, and paired medians by `2.240930%`
and `2.588058%`. The candidate wins 21/32 wall and 22/32 CPU pairs, with one
CPU tie.

Block 2:

| Metric | Wall | CPU |
| --- | ---: | ---: |
| Mean delta | -0.280667% | -0.614345% |
| Paired mean delta | -0.170485% | -0.487583% |
| Median delta | -1.308275% | -0.505051% |
| Paired median delta | -1.032263% | -0.520833% |
| Candidate wins | 36/64 | 32/64 |

Its final 32 pairs improve wall/CPU means by `1.458973%` and `1.577287%`,
paired means by `1.256322%` and `1.400604%`, and paired medians by `1.294841%`
and `1.058231%`. The candidate wins 22/32 wall and 19/32 CPU pairs, with four
CPU ties.

Combined 128-pair result:

| Metric | Wall | CPU |
| --- | ---: | ---: |
| Mean delta | -0.756592% | -0.906204% |
| Paired mean delta | -0.599414% | -0.744766% |
| Median delta | -1.268260% | -1.020408% |
| Paired median delta | -1.071448% | -1.052632% |
| Candidate wins | 72/128 | 68/128 |

The six CPU ties are excluded from the CPU win count.

Combined final halves:

| Metric | Wall | CPU |
| --- | ---: | ---: |
| Mean delta | -2.054715% | -2.163164% |
| Paired mean delta | -1.895377% | -2.013777% |
| Median delta | -2.968659% | -1.538462% |
| Paired median delta | -1.464277% | -2.030509% |
| Candidate wins | 43/64 | 41/64 |

Tracked raw measurements:

```text
experiments/2026-07-24-028-inline-intmap-const-lookup/native-warmup.csv
experiments/2026-07-24-028-inline-intmap-const-lookup/native-lusk.csv
experiments/2026-07-24-028-inline-intmap-const-lookup/native-warmup-2.csv
experiments/2026-07-24-028-inline-intmap-const-lookup/native-lusk-2.csv
```

## Compatibility and full gates

The maintained comparison report
`.artifacts/e-compare/20260724-193627-764481` has 50 cases, zero mismatches,
and one declared sledgehammer difference. HEN011, the synthetic one-second
LUSK6 case, BOO020, and SWV851 all match the archived C outcomes.

All acceptance gates pass:

- serial `cargo test --locked --all-targets --all-features -j1 --
  --test-threads=1`: 4,394 library tests plus every binary and integration
  target;
- strict `cargo clippy --locked --all-targets --all-features -j1 --
  -D warnings -D clippy::pedantic`;
- exact-final-source default-feature and all-feature fat-LTO release builds;
- formatting and locked Cargo metadata;
- generated C-source documentation coverage: 492 source files and 266 unit
  documents;
- Change Later wording across 269 Markdown files;
- local Markdown links across 269 files;
- documentation regeneration preservation across 268 manual Markdown files;
- vendored `eprover/` remains clean at `master...origin/master`.

## Result

Accept. The forced-inline immutable `IntMap` lookup removes a measured
standalone PD-tree boundary, improves exact instructions, shrinks the native
binary, preserves exact proof output and the full compatibility matrix, and
improves both independent native blocks and their stable halves. Retain the
attribute and its measured Clippy allowance.
