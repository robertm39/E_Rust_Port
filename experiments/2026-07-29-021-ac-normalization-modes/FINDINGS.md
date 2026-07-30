# Experiment 021: AC normalization modes

## Question

Can Umlaut's existing associative-commutative canonical equality and
proof-producing AC redundancy modes reduce equivalent term structure or
inference cost and improve family-held-out algebraic solves?

The Bead also proposed AC-aware indexing and joinability. The architecture
audit narrowed the experiment before implementation: the existing
canonicalizer and proof-producing redundancy modes were measured first, while
AC matching, unification, indexing, term rewriting, and unproved joinability
criteria remained out of scope.

## Architecture audit

`src/terms/acterms.rs` recursively normalizes terms into a separate `AcTerm`
view. An explicitly AC binary symbol is flattened and its normalized arguments
are sorted; an explicitly commutative binary symbol has its two normalized
arguments sorted. Variable identity and multiplicity remain part of the view.
Logical terms are not rewritten.

The canonical equality is used by `Eqn::is_ac_trivial`. Axiom scanning records
associativity and commutativity properties plus proof parents. AC resolution
and forward contraction then use the existing `None`, `DiscardAll`,
`KeepUnits`, and `KeepOrientable` modes. Their clause deletion and proof
ancestry were already production paths. There is no AC matcher, unifier,
fingerprint index, or dedicated ground-joinability engine.

This experiment added opt-in, per-search telemetry around that existing
canonical equality:

- equality checks and successful hits;
- top-level normalizations;
- input, normalized, and flattened node counts.

The counters use the same scoped relaxed-atomic pattern as other aggregate
search telemetry. The disabled path performs one enabled-flag check and does
not collect structural counts. The change does not alter a term, inference,
clause-deletion decision, or proof.

## Semantic and integration tests

Nine focused AC tests pass on Ubuntu, including:

- every permutation of three constants under both binary associations;
- nested commutative terms inside an AC term;
- preservation of constant and variable multiplicity;
- variable-identity distinctions;
- weight-mismatch and phony-application rejection;
- exact scoped telemetry checks, hits, and flattening.

The aggregate telemetry integration test emits the additive `ac` schema
object. The four search-telemetry unit tests, rustfmt, and default-feature
clippy also pass. The repository's complete ephemeral Ubuntu lifecycle passed
after the experiment.

## Frozen corpus and contracts

Selection was outcome-blind: the audit scanned all CASC-30 UEQ and FEQ
presentations for an explicit binary associativity equality and matching
commutativity equality. The final population is 41 problems with disjoint
source families:

| Phase | Problems | Families |
| --- | ---: | --- |
| Calibration | 21 | KLE, LAT, SWW |
| Validation | 16 | LCL, NUM, RNG, SWV |
| Test | 4 | NUN, REL |

Audit report ID:
`38962276d29963fc39ae390e7f18fcd04aab6a9c9f65dfe07b57fad576332e25`.

The initially committed selector recognized only one associativity equality
orientation. The frozen audit failed before the release binary was built or
any candidate ran, identifying 15 reverse-orientation cases. Commit
`eecd962a` records the pre-outcome correction from 26 to all 41 matches.

All four treatments used the same completion-shaped heuristic, KBO6, no
literal selection, disabled equality factoring, full forward demodulation, and
presaturation simplification. Only `--ac-handling` varied.

- Calibration contract:
  `6ed9d738384cc32be88cd3de89e63d85bf5fdbc884208bd3c9766d0903dd4908`
- Validation contract:
  `4c8849779edfacab32062502af5ff19e8a2b400ce6486d5d5fe8b2140878ecb6`
- Corrected test contract:
  `bd956f0b1440bd90f62a9c8eff50920252c0cd58b31cf64c2bc976682a08986e`
- Release binary SHA-256:
  `9e0f79d09da472e223a1195013fa0d91c5fa814870198414c564fd9c06d1ae8c`

The first test execution used eight workers. Although CPU limits were
per-process, contention caused the base harness's hard-limit-plus-ten-second
wall watchdog to kill 22 of 32 larger-budget coordinates without a status.
That entire 64-coordinate test execution was discarded. The complete test
matrix was rerun with four workers; it has no external timeout, missing
status, missing telemetry, or contradictory status. Both executions remain in
the raw archive.

## Solve results

The final matrix contains 276 runs.

| Phase | `none` | `discard_all` | `keep_units` | `keep_orientable` |
| --- | ---: | ---: | ---: | ---: |
| Calibration, 4 s | 3 | 3 | 3 | 3 |
| Validation, 8 s | 6 | 7 | 6 | 7 |
| Test, 5 s | 1 | 1 | 1 | 1 |
| Test, 20 s | 1 | 1 | 1 | 1 |

`DiscardAll` and `KeepOrientable` added `LCL109-6` on validation. That signal
did not transfer to the untouched test families. Every test mode solved only
`NUN134-1`; no mode solved `REL016-10`, `REL029-10`, or `REL040-10`.

On the single common larger-budget test solve, candidate/baseline paired
medians were:

| Mode | CPU ratio | Generated-clause ratio |
| --- | ---: | ---: |
| `discard_all` | 0.954112 | 0.947100 |
| `keep_units` | 0.977362 | 0.948492 |
| `keep_orientable` | 0.966475 | 0.948492 |

The best reduction was 5.29% in generated clauses and 4.59% in CPU, below the
preregistered 10% efficiency threshold. These ratios cover one solved problem
and two repetitions, so they are descriptive rather than broad performance
claims.

## Canonicalization activity

The telemetry demonstrates that the measured path is active:

- Calibration modes performed 49,073 to 58,620 equality checks. Hit rates
  were 7.13% to 9.12%, and flattening removed 9.44% to 10.01% of nodes that
  reached normalization.
- Validation modes performed 14.89 million to 20.41 million checks and
  1.37 million to 1.66 million normalizations. Only 0.0188% to 0.0227% of
  checks were hits, while flattening removed 223,214 to 249,052 nodes, or
  6.12% to 7.51% of normalized input volume.
- Larger-budget test modes performed 17,325 to 18,553 checks, recorded
  8,526 to 9,257 hits, and removed 7,066 to 7,308 nodes. The high test hit
  rate did not produce a solve delta.

The validation figures are the clearest warning against costlier AC-aware
indexing: millions of candidate checks led to only thousands of canonical
equalities, and the one validation-only solve did not replicate on test.

## Independent proof validation

Every reproducible larger-budget test claim requested a TSTP proof object.
ProofCheck 1.0 first passed all 117 self-certification tests. The repository
adapter changed only source-controller paths and alpha-equivalent source
spelling; logical proof fields were unchanged.

All four claims for `NUN134-1`, one from each mode, passed the TPTP solution
gate and ProofCheck:

- expected: 4;
- verified: 4;
- report ID:
  `19219471288bdf9ab63b3359a588a66ad79b5e6bf6ec66f0db3604494b66d4da`.

There were no satisfiable/counter-satisfiable contradictions.

## Decision

The frozen verdict is `defer_ac_indexing_and_joinability`.

The existing canonicalization and redundancy path is semantically covered,
produces independently checkable proofs, and measurably reduces normalized
structure. It produced no held-out solve delta and did not reach the 10%
held-out efficiency threshold. The four-problem test set, with only one common
solve, is too small to support a stronger negative claim, but it is also
insufficient evidence for the substantially more expensive AC matching,
indexing, or joinability work proposed by the Bead.

Keep the existing default and proof-producing modes. Retain the additive
telemetry for future corpus studies. Do not add AC-aware indexing, AC
unification, logical-term normalization, or a joinability criterion from this
experiment.

## Raw evidence

The complete ignored archive is:

```text
.artifacts/experiments/2026-07-29-021-ac-normalization-modes/ac-021-complete.tar.gz
```

It is 45,779,848 bytes with SHA-256
`3b8297f076795721928efa4d60614736c9c0801669353de2b25b973570937f08`.
It contains the audit, all three primary phase contracts and raw records, the
discarded first test execution, the corrected complete test execution, both
proof-validation runs, final analysis, and the exact measured release binary.
Local verification inspected all 3,677 members, rejected absolute or parent
paths, recomputed the audit/summary/proof report identifiers, and reproduced
the embedded binary hash.

Final machine-readable summary ID:
`95bcf05e5f2b2090eed4192b5d53a01d74039e5f3cd48709f2b9f9667bffd156`.

## Reproduction

On Linux, after extracting the pinned CASC-30 corpus into the repository root
and building the release binary:

```text
python3 experiments/2026-07-29-021-ac-normalization-modes/audit.py \
  --manifest benchmarks/casc_2025_manifest.jsonl \
  --problem-root /opt/e-rust-port/source \
  --output /opt/e-rust-port/ac-audit.json

python3 experiments/2026-07-29-021-ac-normalization-modes/run.py \
  --phase calibration \
  --manifest benchmarks/casc_2025_manifest.jsonl \
  --problem-root /opt/e-rust-port/source \
  --binary target/release/umlaut \
  --output-root /opt/e-rust-port/ac-runs \
  --workers 8
```

Repeat for validation. Run test with four workers, then invoke `verify.py` and
`analyze.py` using the paths documented in their `--help` output.
