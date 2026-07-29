# Findings

Bead: `E_Rust_Port-9jt.4.2`

## Decision

Do not integrate the evaluated restart-based AVATAR worker. Keep production
splitting unchanged.

The prototype met its soundness and reporting goals, but failed the
preregistered advancement rule. Every AVATAR run eventually reached a branch
whose proof was not independently accepted, so no original problem received a
verified AVATAR UNSAT result. On the held-out split-sensitive set, baseline had
one verified solve (`PUZ008-2`) and AVATAR had none. There were no AVATAR unique
solves and no paired verified timings from which to establish a speed benefit.

This is a negative result for the bounded restart architecture, not for a live
AVATAR saturation loop. The experiment demonstrates a sound fail-closed
component/SAT/certificate boundary, but the lack of assertion-aware proof
ancestry and broad external proof-checker coverage prevents it from learning
enough conflicts to justify production work.

## Clean-room semantic basis

The prototype was derived from the published
[CAV 2014 AVATAR paper](https://w2.cs.uni-saarland.de/op/f/conferences/VSL2014/VSL2014-pages/proceedings_paper_1048.pdf),
the shorter
[ARW 2015 overview](https://www.cs.man.ac.uk/~regerg/papers/arw15.pdf), and
[Unifying Splitting](https://link.springer.com/article/10.1007/s10817-023-09660-8).
No Vampire implementation was inspected, copied, linked, or included.

Selected input clauses are partitioned into maximal literal components
connected by shared variables. Alpha-equivalent components reuse propositional
selectors. A complete SAT model enables its positive selectors, and a fresh
Umlaut process searches the stronger problem containing unsplit clauses plus
the active components. Only a ProofCheck `VerifiedGood` branch can add the
conservative conflict negating every active selector. An independent parser and
Python DPLL must replay the full meta-certificate before original UNSAT can be
accepted.

The worker implements no locking, deletion, generated-clause splitting, live
assertions, or unverified conflict learning.

## Correctness

- ProofCheck 1.0 self-certified all 117 bundled tests, including E and Mace4
  backend discovery. The checker executable SHA-256 was
  `92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`.
- Twelve focused Python tests passed. They cover comments and quoted tokens,
  connected components, ground components, alpha reuse, deterministic
  selection, branch rendering, fragment rejection, corpus selection, a sound
  two-conflict certificate, and rejection of seven corruption classes:
  component, active set, SAT model, conflict, branch hash, proof hash, and
  final status.
- Three Rust protocol tests passed under Clippy with warnings and pedantic
  lints denied. The release driver then returned complete models `[1,-2]` and
  `[-1,2]`, retained both learned conflicts, and reported incremental UNSAT.
- Every one of the 46 problem certificates passed independent semantic replay.
  Across 47 branches, exactly one proof was `VerifiedGood`; only that branch
  learned a conflict. The next branch was not accepted, so the corresponding
  run stopped unknown. No unverified proof contributed a conflict or final
  claim.
- Patch application, Rustfmt, focused Rust tests, strict Clippy, and the
  release build passed on Ubuntu 24.04 with Rust 1.97.1.

## Frozen corpus

The syntax-only selector froze 46 CASC-30 CNF problems before any prover run.
Complete source families remain isolated by the existing train, validation,
and test holdout.

| Partition | Split-sensitive | Neutral |
| --- | ---: | ---: |
| Train | 12 | 12 |
| Validation | 4 | 8 |
| Test | 3 | 7 |

The corpus manifest SHA-256 is
`17a829a2c18465d43f0765578e4a3da9a67054e1c8411cb91fdaa124b1b3cea5`.
The minimal 46-file archive SHA-256 is
`b94bb68b24e1a9c887863bb74a4fcd2f477b503b2d20033ed17dc2ca14ec454d`.

## End-to-end results

All three methods received 20 seconds of prover wall time per problem and a
2,048 MiB limit. Proof-checking time was separate. A solve below means
independently verified, not merely an Umlaut SZS claim.

| Method | Problems | Umlaut UNSAT claims | Verified solves | Unique solves | Total prover wall (s) | Median / p95 wall (s) | Maximum RSS (KiB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Baseline | 46 | 17 | 1 | 1 | 415.062 | 2.477 / 20.047 | 1,855,324 |
| Static split | 46 | 17 | 0 | 0 | 413.881 | 2.271 / 20.050 | 1,855,204 |
| Bounded AVATAR | 46 | 0 final | 0 | 0 | 411.069 | 2.261 / 20.052 | 1,855,352 |

Baseline's sole verified and unique solve was validation problem `PUZ008-2`.
Static splitting also emitted UNSAT there, but its proof was not independently
accepted. AVATAR's first `PUZ008-2` branch was verified in 0.0094 seconds and
learned conflict `[-1,-3,-5,-7,-9,-11]`. The second SAT model changed selector
11 to 12; ProofCheck returned a coverage gap for that branch, so the run
correctly stopped unknown.

### Held-out split-sensitive set

The seven validation/test problems contained 34 selected split clauses and 106
distinct selectors. AVATAR explored eight models, activated 37 component
clauses, left 81 inactive, and learned one verified conflict. Thus 68.6% of
available component clauses were inactive across those branches. Cumulative
branch-prover wall time was 61.141 seconds and peak RSS was 29,312 KiB.

Baseline had one verified solve, static splitting and AVATAR had zero, and
AVATAR had no unique solve. This fails both the no-baseline-loss and
split-benefit gates.

### Held-out neutral set

The 15 neutral problems had no split selectors. Each AVATAR run therefore
performed one baseline-equivalent branch and stopped when that proof was not
independently accepted. None of the three methods had a verified solve in this
cohort, so no paired verified median exists. AVATAR used 203.094 cumulative
prover seconds and peaked at 1,855,352 KiB. The missing paired median fails the
neutral timing gate; it is not treated as evidence of a speed regression.

### Activation and SAT cost

Across all splits, the abstraction selected 92 clauses and introduced 202
per-problem selectors. The 20 split-sensitive branch instances activated 90
components and left 124 inactive, a 57.9% inactive fraction. Neutral instances
add no selector counts. The persistent SAT service made 47 calls in 0.108 ms
total. First-order search and proof acceptance, rather than propositional
solving, dominate this prototype.

## Proof-validation boundary

The positive-only gate prevented broad Umlaut UNSAT claims from becoming
experimental evidence. Among outputs sent to ProofCheck, 44 were
`VerifiedBad`, four were `Unknown`, two had unterminated proof blocks, and two
were `VerifiedGood` (baseline and the first AVATAR branch for `PUZ008-2`).
Most `VerifiedBad` reports cite a generated proof input leaf whose body does
not match its cited source formula; several others concern a negated-conjecture
body. The experiment does not infer that the theorem result is false, but it
must treat each proof as invalid for conflict learning.

This broad first-order leaf/provenance failure is follow-up work. Weakening
ProofCheck or accepting solver self-replay would invalidate the experiment's
trust boundary.

## Harness diagnostics

Two pre-evidence runs were retained locally but excluded from the findings:

1. the first passed equal hard and soft CPU limits, which Umlaut rejected
   before search;
2. the second uploaded the ProofCheck frontend without its sibling backends,
   causing every external check to return `Unknown`.

Neither failure changed the frozen corpus, method, resource envelope, or
decision rule. The final controller now omits the invalid soft limit, uploads
the full pinned ProofCheck release, and requires its 117-test self-certification
before any benchmark.

## Evidence

- Final report ID:
  `9a5082519ada96864cf2f2ec85eed1f72294c31e0dd49ec1243a413e88faf6da`
- Analysis report ID:
  `5d4ee9d919eb8c16b1975353b9eb6cb84da9e340fbcd4dc4abb9ef21987d0fec`
- Evidence archive SHA-256:
  `63b3b10d79a9ab9607a617207d2d1b38df90ea99f1f2d17c2148d1329b4bcd6d`
- Downloaded report SHA-256:
  `ed62b71ea9f2447e57437dcab1da8a4c5e911ac05f5026b253f2fc225d8bb0d6`
- Corpus report ID:
  `8c7a5e4c4873c07c4eef020670d3d4cbb0809063a3ff75b4a9eb5275f4964eb9`
- Release Umlaut SHA-256:
  `6872e8a383719516ed101175392e42270e4c1366ec511989e7008b5cd3eb8c8f`
- Release SAT driver SHA-256:
  `78fa4f01921f875d679ecc800e7187f369f6d35695b9b46b68fb3e774cb318d9`
- Final repository validation run: `260729-104324-675e`; Rust tests,
  formatting, strict Clippy, Linux builds and smokes, Windows-GNU compile-only
  gates, 50 main cases, 216 support-tool cases, 10 timing cases, and Callgrind
  smokes passed with zero unexpected compatibility or benchmark behavior
  mismatches. The aggregate Rust/C wall-time ratio was `1.070`.

The ignored evidence is under
`.artifacts/experiments/2026-07-29-008-avatar-restart-prototype/`. The
ephemeral runner and firewall were deleted after download and local hash
verification.

## Limitations

- The restart prototype does not propagate assertion ancestry through the
  saturation loop, so one externally unaccepted branch blocks all further
  exploration of that model.
- The held-out split-sensitive cohort has only seven problems and one verified
  baseline solve. These results establish feasibility and a rejection, not a
  general AVATAR performance estimate.
- The first-order proof-validation failures dominate verified solve counts.
  Umlaut's raw UNSAT claims are reported for transparency but never counted.
- Complete-model conflicts negate every active selector. Finer proof-dependent
  conflicts could explore more models, but require the live ancestry machinery
  this bounded experiment intentionally excludes.
- Process RSS includes the full prover and varies substantially with the
  selected problems; it is not the incremental memory cost of selectors.
