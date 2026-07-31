# Deterministic adaptive-probe findings

## Conclusion

Keep Umlaut's atomic search-telemetry checkpoint, but stop the tested
clause-growth adaptive policy and keep it outside automatic schedules.

The observability problem is solved. The post-fix train split produced valid
decision telemetry for all 28 non-proof probes, and validation and test each
produced valid telemetry for all 64 non-proof probes. The previously invisible
training hard stop on `NLP262+1` now leaves a complete schema-v1 `checkpoint`
record and takes the registered insufficient-signal fallback deterministically.
All held-out branches repeated exactly.

The policy still fails the frozen advancement rule:

- telemetry wall overhead was 1.0534 times the disabled probe on validation
  and 1.0756 times on test, above the 1.05 limit;
- adaptive search added no reproducible solve versus both restart controls;
- on test it lost `NUN081+1` versus the static goal continuation while adding
  `NUN086+2`, which the static global restart already solved; and
- it therefore failed both efficacy and no-loss gates.

The final decision is `stop`, with primary reason
`telemetry_overhead_exceeded`. The telemetry option remains opt-in, and no
production search schedule or default changed.

## Production checkpoint

The first train-only diagnostic at source revision
`eb8b7a99fa5073ed7384d153d4a340b5f2fcb256` found that `NLP262+1` exhausted
its hard CPU limit during preprocessing. A processed-clause limit cannot fire
before saturation begins, so four decision-bearing arms still had no record.
No validation or test output had been opened.

Source revision `f03259698d81e8fbc25c8b64deb4e7c35e3ffd77` fixes that boundary:

1. telemetry-enabled saturation writes a complete schema-v1
   `record_kind=checkpoint` record before input processing;
2. the file is flushed through a temporary sibling;
3. rename atomically publishes the checkpoint; and
4. an ordinary return atomically replaces it with
   `record_kind=final`.

A hard stop during preprocessing therefore leaves a complete record with zero
search counters instead of a missing or partial file. Consumers distinguish
that state through `record_kind` and the existing insufficient-processed
fallback. Search choices, proof construction, disabled-mode behavior, and
defaults are unchanged.

The Rust change passed formatting, all-target checks, standard and pedantic
Clippy, a release build, and all 4,496 library tests plus all executable and
integration targets. Focused tests cover final replacement, temporary-file
cleanup, and a valid checkpoint surviving an early parse failure. A direct
Linux reproduction of the `NLP262+1` hard stop retained the expected
checkpoint.

## Frozen experiment

The restarted experiment binds:

| Artifact | SHA-256 or identity |
| --- | --- |
| Source revision | `f03259698d81e8fbc25c8b64deb4e7c35e3ffd77` |
| Release `umlaut` | `cbfec550d0b135d9f3ae0233c6b385f6ffec8305ee5fbc36e8b3ed19ca4f5be1` |
| Corpus manifest | `5b3b2bf5c86bf6537742705a49a15e224dd1062b9d5ad96d56913e2dfdddc923` |
| CASC source manifest | `31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d` |
| ProofCheck 1.0 | `92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e` |
| Repository proof gate | `4c90eea3faa207af374f6c000276f7d1268e64ecbf13a78800b29abf399733d0` |

The candidate-blind corpus contains 24 exact CASC-30 FNE/FEQ theorem problems:
eight train, eight validation, and eight test. Whole source families are
disjoint across splits. Each coordinate runs seven arms: telemetry-disabled
probe, telemetry-enabled probe, full global, full goal, static global restart,
static goal continuation, and adaptive continuation. Train runs once;
validation and test run twice.

The accepted matrix contains 280 coordinates:

- 56 train coordinates under contract
  `6590e8bf9f3b94db956bf563690fff8bcdafcab04f0fee2bb3770281370df310`;
- 112 validation coordinates under contract
  `d3ecfb4edb85e51c468aad85e27ffd06ab99562c26e11e5beb8b634f9a2b1d6e`;
  and
- 112 test coordinates under contract
  `634f85a71abd2f86d31389b3f42cf873d710ec1dd58ee8b9460336f3b015862f`.

## Observability and overhead

| Split | Valid non-proof telemetry | CPU ratio | Wall ratio | Peak-RSS ratio | Status mismatches | Processed mismatches |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Train | 28/28 | 1.0000 | 0.9904 | 1.0017 | 0 | 1 |
| Validation | 64/64 | 1.0000 | 1.0534 | 1.0051 | 0 | 0 |
| Test | 64/64 | 1.0000 | 1.0756 | 1.0091 | 0 | 0 |

The one train processed mismatch is `NLP262+1`: both probes stop in
preprocessing, so ordinary statistics cannot expose a processed count for the
telemetry-disabled control. This train-only diagnostic does not enter the
held-out decision. The atomic record nevertheless makes all four
decision-bearing probes observable and reproducibly selects the goal fallback.

All validation and test probes reached the 256 processed-clause checkpoint.
The held-out pairs have identical statuses and processed counts. The atomic
write adds no measurable median CPU cost at the timer's precision and about
0.5% to 0.9% median peak RSS. Its roughly 2.6 to 3.2 millisecond absolute wall
cost is proportionally larger on these 29 to 45 millisecond probes and misses
the frozen relative gate.

Validation selected global on 14 adaptive repetition coordinates and goal on
both `LCL982+1` repetitions. Test selected global on all 16 coordinates.
Every problem made the same decision in both repetitions. Maximum recorded
controller decision time was 35 microseconds.

## Search result

| Policy | Validation reproducible solves | Test reproducible solves |
| --- | --- | --- |
| Adaptive | `LCL982+1`, `SWV092+1` | `NUN086+2` |
| Static global restart | `LCL982+1`, `SWV092+1` | `NUN086+2` |
| Static goal | `LCL982+1`, `SWV092+1` | `NUN081+1` |

Validation supplies four common solved repetition coordinates against each
control. Adaptive's median CPU ratio is 0.554 versus static global restart but
1.000 versus static goal, so it does not meet the rule requiring at most 0.95
against both controls.

Test supplies only two common coordinates against static global restart, with
a 1.056 median CPU ratio. It has no common solved coordinate against static
goal: adaptive solves `NUN086+2`, static goal solves `NUN081+1`. Because
`NUN086+2` is already solved by static global restart, adaptive has no
test-only solve versus both controls. Losing `NUN081+1` also fails the no-loss
gate.

The full five-second contextual arms show the same architectural split:
full global solves `NUN086+2`, while full goal solves `NUN081+1` and
`NUN085+1`. The frozen clause-growth threshold sends every test problem to
global and cannot retain goal's complementary coverage.

## Correctness and reproducibility

All held-out correctness lists are empty. Independent validation accepted all
280 coordinates and reran all 41 stored proof claims through the repository
gate and ProofCheck: 9 train, 20 validation, and 12 test. Every problem,
include, source, binary, script, contract, stdout, stderr, timing, telemetry,
proof, and gate hash matched. No subprocess group survived and no isolated
temporary directory retained a file.

Twelve focused Python tests pass locally and on Linux. They cover the frozen
threshold and fallback, checkpoint/final record kinds, status and statistics
parsing, corpus selection and archive traversal, GNU timing, overhead mismatch
detection, and final decisions.

Exact resumes reused 56/56 train, 112/112 validation, and 112/112 test
coordinates without rerunning prover work. An intentional append to one test
stdout artifact was rejected with the expected hash mismatch. Restoration
reproduced `test-analysis.json` byte-for-byte at SHA-256
`808513fd208365790b667e3f9a1bc51408897aac9a95abd117f325595a0a9b2d`.
Repeated final analysis reproduced SHA-256
`efd77bbaa4b3dc787bf72a57069d7044de7c22bf245353245ca13054b9e6f30f`.

The compressed accepted evidence is:

```text
.artifacts/experiments/2026-07-30-010-deterministic-adaptive-probes/evidence-v1.tar.gz
SHA-256 910ccaf961ea6c906d90cc35778f08fb95dfea7e0115d02b181a8e8912ea3a87
```

The archive contains only the post-fix train, validation, and test roots plus
their corpus report, primary analyses, independent validations, and frozen
decision. Pre-fix diagnostics and smoke roots are excluded.
