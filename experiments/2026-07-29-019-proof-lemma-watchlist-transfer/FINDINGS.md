# Proof-derived lemma and watchlist transfer findings

## Outcome

Experiment 019 satisfies Bead `E_Rust_Port-9jt.3.5`, but this raw
proof-clause transfer formulation provides no net value and should not enter an
automatic schedule.

The frozen decisions are:

- `lemma_same`: `stop_no_value`;
- `lemma_cross`: `stop_no_value`;
- `watch_same`: `uncertain`;
- `watch_cross`: `uncertain`.

The explicit-lemma result is negative rather than merely underpowered. The
selector mined 20 clauses cheaply, but none of 296 target-axiom admissibility
checks was proved at the frozen budget. Consequently, zero logical clauses
were added. The safety gate prevented unsound reuse and spent 578.071 aggregate
CPU-seconds establishing the absence of an admissible candidate.

The watchlist result is coverage-limited on test, where all 80 runs ended
`ResourceOut`, but the available validation evidence is unfavorable.
Same-category and cross-category watchlists preserved the same four
reproducible solves as control, produced no unique solve or watchlist-hit
marker, shortened no proof, and changed neither generated nor processed clause
counts. Their median common-solve CPU ratios were 1.095 and 1.128,
respectively.

The production effect is
`keep_proof_clause_transfer_out_of_automatic_schedules`. The existing
watchlist and PCL lemma tools remain valid opt-in mechanisms; this experiment
does not modify their implementation or defaults.

## Evidence identity

| Evidence | Identity |
| --- | --- |
| Measured prover source revision | `ce75ea3b68c34ab1640e0f362438a656626a5b0e` |
| Umlaut binary SHA-256 | `22abd227725da25af6143ae4f3159a05ccd477bd0f00d0aa955c49f7392aecd8` |
| PCL selector binary SHA-256 | `2c529f2a16193352aa673aa23c3853619af529efba2c68c0754d8a252998f6be` |
| Experiment 018 source archive SHA-256 | `8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156` |
| Corpus SHA-256 | `28b6ac9d59d2871877a7b784b41bc70fe5c09386da6214123791e660819b67c1` |
| Corrected preparation contract | `62054a51a1fd3fe1ee5e33a9b895176f4f3777652f2da8b4bf8ba18634eca175` |
| Prepared manifest SHA-256 | `cb1fae150e4bd6c390f95ed69edb65b203499709bcde7bcd3bd117297ee7bc92` |
| Validation contract | `0a9e5fd7d60fec0f7483b3de1edd2264cc3b0966160308b6cba6bb80f416e8c1` |
| Test contract | `d4625e2b52f1f8361ce6b0f430c8adf01a4827809cf23ccad8aa806058cf7c0c` |
| Final report ID | `44f4b2ebc1910cf2c4f15d26a59b0b23ac36e64b0413c8361aaa118faf3ef7f5` |
| Final analysis SHA-256 | `521e19ef4a01aee99bc3788a9e89b1bf4477036f9e62f7182e41e55df0e55054` |

The complete ignored raw archive is
`.artifacts/experiments/2026-07-29-019-proof-lemma-watchlist-transfer/lemma-watchlist-019-complete.tar.gz`.
It is 41,959,858 bytes with SHA-256
`fbae6d65079fb3677a89973f4453fff6044952102e865ae69ba167eb224b274a`.
It contains the invalid first preparation root, corrected preparation root,
synthetic and search smokes, all 160 measured runs, contracts, wrappers,
admissibility problems and outputs, telemetry, PCL proofs, and final analysis.

## Source selection and leakage controls

The preparation controller accepted only the five successful experiment 018
training traces:

- FNE: `MGT067+1`;
- FEQ: `SWW967+1`;
- EPU: `LAT265-2`;
- UEQ: `KLE145-10` and `SYN563-10`.

Their archived result records, problem hashes, proof statuses, and trace hashes
all verified. No failed training trace and no validation/test proof entered
selection.

Train, validation, and test source families are pairwise disjoint.
Same-category pools admit only a matching TPTP category; cross-category pools
exclude it. Candidate ordering depends only on the preregistered salt, transfer
mode, source problem, selected-record index, and clause body. It sees no
held-out result, timing, telemetry, proof, target identity, or symbol-overlap
score.

The selector returned:

| Source proof | Selected clauses | CPU seconds | Wall seconds |
| --- | ---: | ---: | ---: |
| `MGT067+1` | 4 | 0.003362 | 0.003573 |
| `SWW967+1` | 7 | 0.001476 | 0.001626 |
| `LAT265-2` | 2 | 0.000958 | 0.001110 |
| `KLE145-10` | 7 | 0.003366 | 0.003492 |
| `SYN563-10` | 0 | 0.001767 | 0.001887 |
| **Total** | **20** | **0.010929** | **0.011688** |

After deterministic per-target/mode deduplication and caps, the watchlist
wrappers contained 76 same-category and 220 cross-category guidance-clause
placements across all 16 held-out targets. These are guidance records, not
logical premises.

## Explicit-lemma safety and cost

Raw proof clauses are consequences of their source problems, not automatically
of an unrelated target. Each candidate was therefore re-proved from an
axiom-only target wrapper before it could be added as a `lemma`.

All 296 corrected checks parsed and ended with the normal `ResourceOut` status
at the 1-second soft / 2-second hard CPU budget:

| Split and mode | Attempts | Admitted | Rejected | CPU seconds |
| --- | ---: | ---: | ---: | ---: |
| validation, same-category | 38 | 0 | 38 | 75.439779 |
| validation, cross-category | 110 | 0 | 110 | 205.095741 |
| test, same-category | 38 | 0 | 38 | 76.402771 |
| test, cross-category | 110 | 0 | 110 | 221.132432 |
| **Total** | **296** | **0** | **296** | **578.070723** |

No explicit treatment therefore had an added clause. Its measured search was
logically identical to control through a wrapper include, and it preserved
identical solves, generated/processed work, and proof lengths. On the eight
common solved validation repetition coordinates, raw search-only CPU ratios
were 1.011 for same-category and 1.006 for cross-category. Including the
required one-time target admissibility cost raised the median ratios to 6.541
and 27.348. Both frozen decisions are `stop_no_value`.

## Held-out search and proof shortening

Every measured treatment contains 8 problems × 2 repetitions per split.
Validation and test contracts each completed 80/80 and then resumed 80/80 from
their content-hashed records.

Validation status counts were identical for every treatment:

- 2 `Theorem`;
- 6 `Unsatisfiable`;
- 8 `ResourceOut`.

Every treatment reproducibly solved exactly `LCL026-10`, `LCL365+1`,
`PUZ037-2`, and `ROB005-1`. There was no treatment-only, control-only, or
one-repetition-only solve.

| Treatment | Median CPU ratio | Median RSS ratio | Generated ratio | Processed ratio | PCL-step ratio | Watch hit markers |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `watch_same` | 1.095367 | 1.002189 | 1.000000 | 1.000000 | 1.000000 | 0 |
| `watch_cross` | 1.128481 | 1.003548 | 1.000000 | 1.000000 | 1.000000 | 0 |
| `lemma_same`, search only | 1.011263 | 1.000085 | 1.000000 | 1.000000 | 1.000000 | 0 |
| `lemma_cross`, search only | 1.006005 | 1.000057 | 1.000000 | 1.000000 | 1.000000 | 0 |

The four control proof lengths were 72, 33, 17, and 47 PCL steps; each
treatment reproduced the exact corresponding step count in both repetitions.
There is therefore no proof shortening.

All 16 runs per treatment on test ended `ResourceOut`. Test has no solved
coordinate on which to measure CPU or proof-length ratios and no unique solve.
The frozen watchlist decisions are consequently `uncertain`, not `stop`, even
though the validation direction is unfavorable.

## Correctness and independent checking

All 160 measured runs had a valid SZS status and either valid telemetry or the
documented hard-resource-stop semantics. No contradictory status, parser
failure, external timeout, missing proof, or contract/hash failure occurred.
Every proof status had a nonempty PCL protocol.

Watchlist clauses are never logical premises, so their presence cannot make a
refutation unsound. Explicit clauses would require a separately preserved
target-axiom PCL certificate, but none passed admission and none was injected.
No treatment-only proof exists, so the preregistered focused replay gate was
not triggered. All common proof traces remain in the raw archive.

## Invalid preparation root

The first preparation root is preserved and excluded. Its controller copied
implicitly universally quantified PCL clause variables into FOF conjectures
without writing explicit quantifiers. That produced 280 parse failures; the
remaining 16 attempts reached ordinary `ResourceOut`. No candidate was
admitted and no held-out control/watchlist/lemma search ran from that root.

The amended controller explicitly universally quantifies each untyped free
variable, has two focused regression tests for extraction and rendering, and
uses the fresh preparation contract above. It changes no trace, candidate
order, pool, cap, budget, strategy, metric, threshold, or decision rule.

## Validation

- local experiment-controller tests: 13 passed;
- Ubuntu experiment-controller tests: 13 passed;
- synthetic production-path smoke: selector and inline static watchlist
  passed;
- corrected preparation: 5/5 source traces selected, 296/296 admissibility
  checks parsed with explicit status;
- measured validation: 80/80 complete and 80/80 resumable;
- measured test: 80/80 complete and 80/80 resumable;
- local final-analysis recomputation: byte-identical SHA-256
  `521e19ef4a01aee99bc3788a9e89b1bf4477036f9e62f7182e41e55df0e55054`;
- tracked diff whitespace check: passed.

No Rust source changed. The release `umlaut` and `umlaut-pcl-lemma` binaries
were built on Ubuntu from the frozen source revision with pinned CaDiCaL 3.0.1.
