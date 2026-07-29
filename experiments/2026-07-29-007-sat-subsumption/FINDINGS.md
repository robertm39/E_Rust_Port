# Findings

Bead: `E_Rust_Port-9jt.4.3`

## Decision

Do not integrate the evaluated SAT-based ordinary-subsumption dispatcher.
Keep Umlaut's recursive matcher unchanged.

The preregistered calibration gate found no eligible dispatch policy. The
experimental path agreed with the existing checker on every captured ordinary
subsumption call, but was more than an order of magnitude slower. Because no
calibration policy advanced, the frozen contract prohibited a validation or
held-out full-prover dispatcher run. Validation and test remained
observational captures only.

Do not open a subsumption-resolution integration follow-up from this evidence.
Its prospective activation rate was too small to justify the proof-production
and ancestry work.

## Source and semantic basis

The prototype is a clean-room implementation derived from the published
encodings in
[First-Order Subsumption via SAT Solving](https://cca.informatik.uni-freiburg.de/papers/RathBiereKovacs-FMCAD22.pdf),
[SAT Solving for Variants of First-Order Subsumption](https://cca.informatik.uni-freiburg.de/papers/CoutelierRathRawsonBiereKovacs-FMSD25.pdf),
and the related
[CADE 2023 paper](https://rawsons.uk/michael/papers/CADE-2023-subsumption.pdf).
No Vampire implementation was inspected, copied, linked, or distributed.

The ordinary encoding requires one same-polarity match per side literal,
target-literal multiplicity conservation, and pairwise-compatible
substitutions. The prospective subsumption-resolution encoding additionally
requires at least one opposite-polarity match, forces all opposite matches to
one target literal, and forbids a positive match to that resolution target.

## Correctness

- The independent Python matcher and DPLL oracle passed 10,000 seeded generated
  clause pairs. It exercised 5,473 ordinary-subsumption positives and 3,022
  subsumption-resolution positives.
- Three focused Rust tests passed for shared substitutions, literal
  multiplicity, complementary-target selection, uniqueness, and coherence.
- Across 52,147 captured calls and 49,863 phase-separated unique clause pairs,
  the SAT ordinary result had zero disagreements with Umlaut's existing
  checker.
- Patch application, `rustfmt`, the focused Rust tests, Clippy with warnings
  and pedantic lints denied, and the release build all passed on Ubuntu 24.04.

## Corpus results

The family-separated CASC-30 split contained 24 calibration, 24 validation,
and 20 held-out problems. Some problems produced no eligible non-unit
bank-aware checker calls, so the captured problem counts are smaller.

| Phase | Problems with captures | Records | Unique pairs | Ordinary disagreements | Aggregate SAT/checker | p95 SAT/checker | Median checker / SAT (ns) | Max estimated encoding bytes | Prospective SR hits / unique pairs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Calibration | 16 / 24 | 21,097 | 20,753 | 0 | 11.491 | 13.399 | 181 / 2,113 | 61,976 | 1 / 1 |
| Validation | 18 / 24 | 15,505 | 15,066 | 0 | 13.545 | 8.053 | 241 / 2,794 | 399,688 | 11 / 9 |
| Held-out test | 13 / 20 | 15,545 | 14,044 | 0 | 13.018 | 16.749 | 111 / 1,992 | 104,968 | 7 / 3 |

Ordinary SAT checking has no additional pruning because it is semantically
equivalent to the baseline checker. Ordinary subsumption succeeded in 1,325
of 52,147 records (2.541%). Prospective subsumption resolution succeeded in
19 records representing only 13 unique pairs: 0.0364% of records and 0.0261%
of unique pairs.

The analytical per-call storage estimate stayed below the 256 KiB calibration
gate on calibration, but reached 399,688 bytes in the observational validation
set. Full-process maximum RSS was 1,558,740 KiB on calibration, 538,808 KiB on
validation, and 1,464,056 KiB on held-out test; these values include the whole
prover workload and are not an incremental encoder allocation measurement.

## Crossover surface

The preregistered grid covered minimum side-literal thresholds 2 through 8,
minimum main-literal thresholds 2 through 12, and minimum positive-choice
thresholds 0, 4, 8, 16, 32, and 64. Calibration had 204 regimes with at least
200 records from at least six problems. None passed the `0.80` aggregate and
`0.90` p95 latency gates.

Even the best sample-eligible calibration regime by aggregate latency
(`side >= 8`, `main >= 5`, `positive choices >= 8`) took 7.148 times the
baseline in aggregate over 633 records from six problems, with a 5.831 p95
ratio. The best calibration p95 was still 5.700, with a 7.907 aggregate ratio.
There was therefore no observed crossover.

After the frozen decision, `posthoc_surface.py` exported every populated
threshold point for audit and visualization. The resulting
[`POSTHOC_CROSSOVER.csv`](POSTHOC_CROSSOVER.csv) contains 1,309 rows and has
SHA-256
`590421ab208cb6af2100a92ff423addb53a92c96a031d3119daac6ca4e36659a`.
This post-hoc surface does not alter the preregistered decision.

## End-to-end consequence

There is no proposed production dispatch policy: the calibration selection
contains zero eligible policies and decision `no-calibration-policy`.
Consequently, the preregistered safety rule correctly prevented an end-to-end
dispatcher benchmark, and no production source was changed. This is the
measured end-to-end decision, not missing benchmark work.

The negative result applies to this prototype's fresh per-call matching,
encoding, and use of Umlaut's current internal SAT service. It does not
establish that specialized or incrementally reused SAT subsumption is
universally uncompetitive.

## Evidence

- Frozen selection ID:
  `57c24b6fe5901acd61b917360ee006d30ab3a4e11ad3b55916568ad20fb2c811`
- Final report ID:
  `708581795616d749c1794d6277998fd4ff32cae3295afee9356990b0f924864a`
- Evidence archive SHA-256:
  `5081fa615ec86b3d111ba21c228a7f02078f7a614022858e5b911ba394cd9d22`
- Download wrapper SHA-256:
  `89b9438b8a94f35ef1127961cf13f59b8fce136666cca67c5396584f5b90ec30`
- Corpus archive SHA-256:
  `cf455869fd120048b47dec601ce00e84c05d228c97d148884d4185835dd71c47`
- Corpus report ID:
  `6d2a3607228be79afdbefd3e2c9626859504eb3018b471a26f7b6f1c5dd4170a`
- Prototype SHA-256:
  `d7e807d0c7c4f61d687247099042325b071dfe0e893c3138786c475e5b3bb76d`
- Capture patch SHA-256:
  `87cb74dbb7340c270720531912375f6baed1f546015548b81ba1b747c02d1b86`
- Instrumented release binary SHA-256:
  `a2ea6a879bb53440f04f78748f4a12400897daa9dbc56d43773b7871db40f39b`
- Final repository validation run: `260729-091017-d57c`; all Rust tests,
  formatting, Clippy, Linux builds/smokes, Windows-GNU compile-only gates, 50
  main compatibility cases, 216 support-tool cases, 10 timing cases, and
  Callgrind smokes passed with zero unexpected compatibility or benchmark
  behavior mismatches. The aggregate Rust/C wall-time ratio was `1.074`.

The ignored local evidence is under
`.artifacts/experiments/2026-07-29-007-sat-subsumption/`. The ephemeral runner
and its firewall were deleted after the hashes were verified locally.

## Limitations

- Sampling covers eligible calls reached by one fixed KBO6/FIFO search
  configuration, not every subsumption call or strategy.
- The checker always ran before the experimental path, so timings do not
  randomize execution order. The SAT path could benefit from already-hot
  clause data; this makes the rejection conservative rather than favorable to
  the baseline.
- The storage estimate models encoded choices, bindings, clauses, and literals;
  it is not allocator-level peak memory.
- Subsumption-resolution hits are prospective only. The experiment neither
  mutates clauses nor constructs proof ancestry.
