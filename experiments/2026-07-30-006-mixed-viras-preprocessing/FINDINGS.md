# Mixed-problem VIRAS preprocessing findings

Bead: `E_Rust_Port-9jt.5.12`

## Decision

The native-checked mixed-problem path advances as an explicit
`viras-qe`-feature option, `umlaut --viras-qe-preprocess`. It does not advance
into default features, automatic schedules, or the CASC runtime package.

The path passed source-ancestry proof publication, independent formula-level
recomputation, typed re-embedding, four corruption classes, pass-through
Unknown/unsupported tests, and the 100-file family-held-out TFI evaluation.
It corrected one arithmetic status but added no raw one-second solve. The
evidence supports continued explicit use and arithmetic development, not
automatic schedule adoption.

## Production boundary

Normal Umlaut parsing owns the complete mixed TPTP document. After initial
formula documentation and source archival, the opt-in preprocessor considers
each active formula that contains a quantifier. The unchanged typed LIRA
importer accepts or rejects each formula independently. An importer rejection
or bounded kernel `Unknown` leaves that wrapper unchanged.

A successful result is inserted only after:

1. the kernel has replayed every one-conjunction derivation;
2. a fresh formula-level elimination has reproduced the complete result,
   resource counts, candidates, grids, and branch derivations;
3. the result is closed and quantifier-free; and
4. canonical TFF rendering, parsing through the live typed term bank, and
   exact re-import reproduce the checked LIRA result.

The original wrapper remains in the formula archive. Its active copy receives
`DC_VIRAS_QE`; TSTP and PCL render the unary `viras_qe` rule with
`status(thm)`. A final proof that uses the transformed formula therefore
contains the original TFF source leaf followed by the arithmetic rule.

Focused tests corrupt the source, published result, replay flag, and a branch
candidate list. All fail native validation. A separate publication mutation
test demonstrates that corruption prevents a replacement term from being
returned. Mixed integration tests retain an unsupported nonlinear quantified
formula while a checked arithmetic formula produces a source-linked final
refutation.

## Held-out corpus and protocol

All 100 untouched CASC-2025 TFI documents were grouped by their ten filename
families. None was used for implementation or tuning. The standard 300-file
`Axioms/` include tree was supplied through the `TPTP` root; no problem or
formula was rewritten or extracted.

Both arms used the same all-feature release binary with `--auto`,
`--cpu-limit=1`, `--memory-limit=2048`, `--tstp-format`,
`--output-level=4`, and `--proof-object=1`. The opt-in arm added only
`--viras-qe-preprocess`. Eight workers ran on the dedicated eight-core Ubuntu
runner, so latency is concurrent whole-process latency under the frozen
evaluation load.

Fourteen opt-in runs reached the CPU cutoff before clausification could emit
the preprocessing record. The other 86 documents reported 20,977 active
formulas, of which 10,465 contained quantifiers. The conservative importer
rejected 9,578 quantified formulas and accepted 887. All 887 accepted formulas
were eliminated, independently checked, typed-round-tripped, and inserted;
there were no kernel resource or unsupported-fragment Unknowns after import.

Coverage by held-out family was:

| Family | Documents | With record | Documents applied | Formulas applied |
| --- | ---: | ---: | ---: | ---: |
| ARI | 20 | 20 | 1 | 1 |
| CSR | 2 | 2 | 0 | 0 |
| DAT | 6 | 6 | 0 | 0 |
| HWV | 10 | 5 | 0 | 0 |
| ITP | 12 | 11 | 10 | 886 |
| NUM | 1 | 1 | 0 | 0 |
| SEV | 1 | 1 | 0 | 0 |
| SWC | 9 | 9 | 0 | 0 |
| SWW | 38 | 30 | 0 | 0 |
| SYO | 1 | 1 | 0 | 0 |

The 887 formula publications contain 6,421 checked branch proofs. The output
contains 888 visible `viras_qe` inferences: one preprocessing record per
publication plus the repeated final-proof occurrence for the used ARI result.
Proof-publication success was 100%.

## Search outcomes

The raw one-second SZS solve count was eight in both arms. Both returned 66
CPU cutoffs and 26 `GaveUp` outcomes. Seven baseline `Theorem` outcomes
remained `Theorem`.

`ARI056_1.p` changed from baseline `CounterSatisfiable` to opt-in `Theorem`.
Its complete conjecture is `? [X:$int] : X != 12`, which is true in the
standard interpreted integer domain. The checked result is `$true`; this is
an arithmetic status correction, not an unsound disagreement. The corpus file
has no status comment, so the report preserves both raw statuses rather than
inventing reference metadata.

The ten applied ITP documents added no one-second solve. `ITP414_1.p` remained
the sole solved applied ITP problem and remained `Theorem`. Accordingly the
raw solve delta is zero even though one baseline status was corrected.

## Latency and formula growth

Concurrent whole-process latency was:

| Arm | Median | p95 | Maximum |
| --- | ---: | ---: | ---: |
| Baseline | 2,044.768 ms | 2,903.341 ms | 3,317.935 ms |
| Opt-in | 1,916.412 ms | 2,904.572 ms | 3,521.853 ms |

The paired opt-in/baseline latency ratio had median 0.977, p95 1.586, and
maximum 2.737. This noisy one-second concurrent surface is reported without
claiming a speedup.

The 887 imported source formulas contained 8,793 canonical LIRA nodes and the
closed results contained 887 nodes. Aggregate result/source growth was 0.1009.
Across the 11 transformed documents, the median ratio was 0.0977, p95 was
0.25, and maximum was 0.25. This held-out surface shrank formulas; it does not
bound future open-formula or broader-theory growth.

## Determinism, package boundary, and retained evidence

Repeated opt-in runs were timing-normalized and byte-identical for transformed
`ARI056_1.p` and pass-through `ARI186_1.p`. Their normalized SHA-256 values
were respectively
`aa302bb0c664a49975cd4881b2ab8c295825978001f11ef56d69f6cf1c7fc823`
and
`dd671de3016cce02646ebcd62b0afb4750cecaa1e70da8e4737c399de4d22a9e`.

The final canonical report SHA-256 is
`7617d0088e29ed2fe59c5e89b93cd85fcbdf7c30060e8d69cfe84f85144541b8`.
The exact TFI-plus-Axioms transfer archive SHA-256 is
`a52c4096f31d9092d043b35a11b5efa741e4bd60b193311dccacf55ecac2ab9d`.
Raw reports and the transfer archive are retained under:

```text
.artifacts/experiments/2026-07-30-006-mixed-viras-preprocessing/
```

The final report retained the preregistered schedule SHA-256
`491145ab45477620ed02ed8cd789d6b5e3e6e0d38f413fdbc62163e09a9cb068`
and an empty default feature list.

A preliminary controller pass used the correct 100 TFI files but omitted the
standard support-axiom directory, producing four missing-include exits. No
implementation or threshold was changed after that pass. The support tree and
explicit `TPTP` root were added, the controller was strengthened to reject
file/type/syntax/controller-timeout exits, and the complete final report above
supersedes that incomplete transfer run.

## Release validation

Focused validation on the Ubuntu runner passed `cargo fmt --all -- --check`,
strict Clippy for every target with `viras-qe`, and the complete feature test
surface: 4,528 library tests plus all binary and integration targets. The
Windows GNU compile-only gate also built every target with `viras-qe`. A
separate default release build and help check confirmed that
`--viras-qe-preprocess` is absent when the feature is disabled.

The clean package audit passed all 13 checks, including offline reconstruction
from the source archive, StarExec wrapper/resource-limit emulation, the
five-member rootless runtime archive, and the absence of optional backend
linkage. Its `package-audit.json` SHA-256 is
`2bc2caa9840621ff23581ea3a74ce4397b79e1dcdadd3b9289f696103ff2c72e`.
The runtime archive is 2,807,969 bytes with SHA-256
`2c2642d439651be5e7e23b5d491945c47652e893f007bd2bb6523ea70ce3d034`;
the 328-member source archive is 2,063,648 bytes with SHA-256
`6946ec21f046434d4523127497436c251a8ec8abc314cefff69d0532c9b62fb4`.

The authoritative fresh comprehensive run was
`260731-012713-e183`, using snapshot
`b1daf0c9ad3f3056a5e8e6a26c7dab9faf0db7045c5f30e0459c37da7ce9ebaa`.
It passed Linux build/format/test/Clippy, Windows release and test
cross-compilation, native smoke tests, Callgrind, 50 main compatibility cases,
216 support-tool cases, and the ten-case timing benchmark. There were zero
unexpected compatibility mismatches and zero benchmark behavior mismatches;
the aggregate Rust/C wall-time ratio was 1.0877909422. The downloaded
`validation-summary.json` SHA-256 is
`93a1ecc1a22ac07a1f12fe9f1739a724bf3bd2b1ff263641a637ed0b5004af19`.
