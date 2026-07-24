# Experiment 300: Force-inline term argument borrowing under fat LTO

## Status

Rejected experiment for Bead `E_Rust_Port-j76.5.3`; accepted Experiment 298
source is restored byte-for-byte.

## Question

Does forcing the still-out-of-line `Term::arguments` boundary into its
structural-weight comparison callers improve production throughput now that
the accepted release profile uses fat LTO and one codegen unit?

## Baseline

- Accepted source and release profile: commit `51aa9926`
  (`perf: enable whole-program release optimization`).
- Exact default-feature LUSK6 Callgrind: `8,400,364,984` instructions.
- Original FOL C Callgrind: `5,254,361,329` instructions.
- Exact Rust/C ratio: `1.598741`.
- Accepted native executable:
  `target/native-298-manifest-fat-lto-one-cgu/release/eprover.exe`.

Experiment 299's representative LTO line profile attributes `56,190,374`
instructions and `3,127,396` calls to `Term::arguments`. Fat LTO reduced that
boundary from its pre-LTO `65,174,512` instructions but left it out-of-line.
Experiment 294 rejected the same source attribute under the old release
profile; this experiment tests whether the new whole-program code-generation
regime changes its production result.

## Candidate

Add only `#[inline(always)]` to `Term::arguments`.

Argument representation, `RefCell` borrowing, `Ref::map`, structural
comparison order, borrow lifetime, every caller, and the manifest release
profile remain unchanged.

## Validation

- All 478 focused term-related library tests pass.
- Formatting and `git diff --check` pass.
- WSL and native fingerprints record exactly `features=["default"]`; the
  executable profile hash is the accepted `11264489599640293354`.
- The candidate reaches the exact LUSK6 proof under Callgrind.
- Three parent and eight candidate native runs produce identical 378-byte
  stdout with SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
  empty stderr, and exit zero.
- All 256 measured native processes prove and exit zero.
- The native executable remains `8,617,472` bytes, identical in size to the
  accepted parent.

## Deterministic measurement

The candidate retires `8,354,307,960` instructions, `46,057,024` below the
accepted parent. This is a `0.548274%` reduction, and the hypothetical Rust/C
ratio improves from `1.598741` to `1.589976`.

Raw profile:

```text
.artifacts/experiments/2026-07-24-027-inline-term-arguments-lto/callgrind-inline-term-arguments-lto.out
```

## Native production measurement

Each independent block uses four uncounted warmup pairs followed by 64
alternating measured pairs. Negative deltas favor the candidate; positive
deltas are regressions.

The first block's whole-run wall/CPU means improve `0.751703%` and
`0.909241%`, while paired means improve `0.376080%` and `0.437928%`.
However, its final 32 pairs reverse to wall/CPU mean regressions of
`1.005746%` and `1.140940%`, with paired regressions of `1.090370%` and
`1.243240%`. The candidate wins only 18/32 wall and 11/32 CPU pairs in that
stable half, with three CPU ties.

The second block contains several large early outliers. Its whole-run
wall/CPU means nominally improve `1.543698%` and `0.538827%`, but paired means
are only `-0.142047%` and `-0.028449%`, while unpaired medians regress
`0.519239%` and `1.554404%` and paired medians regress `0.543974%` and
`0.495050%`. Its final 32 pairs decisively regress:

| Metric | Wall | CPU |
| --- | ---: | ---: |
| Mean delta | +2.558380% | +1.434034% |
| Paired mean delta | +2.573159% | +1.538764% |
| Median delta | +1.306728% | +1.030928% |
| Paired median delta | +1.308121% | +1.052632% |
| Candidate wins | 11/32 | 10/32 |

Across all 128 pairs, outlier-sensitive means nominally favor the candidate
by `1.160733%` wall and `0.720123%` CPU, but medians regress `0.350305%` and
`1.063830%`. The candidate wins only 70/128 wall and 55/128 CPU pairs, with 13
CPU ties.

Combining both final 32-pair halves removes the inconsistent early regions and
gives the production decision:

| Metric | Wall | CPU |
| --- | ---: | ---: |
| Mean delta | +1.804229% | +1.291272% |
| Paired mean delta | +1.831765% | +1.391002% |
| Median delta | +1.415666% | +1.052632% |
| Paired median delta | +0.717605% | +1.100097% |
| Candidate wins | 29/64 | 21/64 |

Tracked raw measurements:

```text
experiments/2026-07-24-027-inline-term-arguments-lto/native-warmup.csv
experiments/2026-07-24-027-inline-term-arguments-lto/native-lusk.csv
experiments/2026-07-24-027-inline-term-arguments-lto/native-warmup-2.csv
experiments/2026-07-24-027-inline-term-arguments-lto/native-lusk-2.csv
```

## Result

Reject. Fat LTO makes the deterministic instruction reduction slightly larger
than Experiment 294's pre-LTO result, but it does not repair native
throughput. Both independent stable halves regress, and their combined means,
medians, paired statistics, and CPU win count all reject the candidate.

Restore accepted Experiment 298 source byte-for-byte. Compatibility and full
repository gates are skipped after the decisive production-native rejection.
`Term::arguments` forced inlining is now closed under both the pre-LTO and
fat-LTO release profiles.
