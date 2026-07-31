# Bounded model-guided EPR instantiation findings

## Conclusion

Do not integrate this Inst-Gen-style worker or its clause exchange into
production. The sound prototype works as designed, but it is not complementary
to Umlaut under the frozen four-second aggregate budget.

On combined validation/test, saturation, the equal-budget independent
portfolio, and cooperation reproducibly solved the same seven problems.
Standalone instantiation solved four of those seven and added none. Cooperation
lost no solve, but it also added none versus either comparator. Its median user
CPU was 18.30 times the independent saturation worker on common coordinates
with measurable baseline CPU, and 0.995 times portfolio, missing the frozen
0.90 alternative-benefit gate.

Production source and schedules are unchanged.

## Scope and design

This experiment evaluates Bead `E_Rust_Port-9jt.4.5`. The prototype accepts
only equality-free, function-free, untyped TPTP CNF. It initially grounds each
source clause at the first domain constant, obtains a complete propositional
model from integrity-pinned CaDiCaL 3.0.1, enumerates false finite-domain source
instances, adds at most 64 distinct instances per refinement, and repeats.

UNSAT is claimed only for a propositionally UNSAT ground-instance subset. SAT
is claimed only after exhaustive enumeration establishes a complete finite
Herbrand model. An incomplete scan returns UNKNOWN. This is a clean-room,
finite-fragment, Inst-Gen-style architecture, not an implementation of the full
selected-literal/unification calculus described by Ganzinger and Korovin.

The corpus was frozen before candidate execution from syntax and CASC manifest
metadata only. `corpus.jsonl` has SHA-256
`830fc799926ecac212378586a02ab6d2832c9b8451639cfd7a2d837703c1ddf2`.
Its 29 problems contain:

- 11 diagnostic train problems from GRP, MSC, NLP, and SYN;
- 16 validation problems from PUZ and SWV; and
- two test problems from PLA.

Complete families stay in the manifest's original partition. Fourteen problems
have an expected satisfiable class and 15 an expected unsatisfiable class.

## Frozen comparison

Every train problem ran once. Every validation/test problem ran twice. The
methods were:

1. production `umlaut --auto` for four seconds;
2. standalone instantiation for four seconds;
3. the solve union of independent two-second saturation and instantiation
   workers; and
4. two seconds of instantiation followed by two seconds of Umlaut over the
   original source plus replayable generated instances.

The compound arms each received four aggregate worker-seconds. Cooperation
passed no model assignment, SAT-internal lemma, unverified learned clause, or
priority hint—only checked ground substitution instances of source clauses.

## Held-out results

| Method | Reproducible solves | Unique solves |
| --- | ---: | ---: |
| saturation | 7 | 0 |
| standalone instantiation | 4 | 0 |
| equal-budget portfolio | 7 | 0 |
| cooperative | 7 | 0 |

The identical seven-solve set for saturation, portfolio, and cooperation is:

`PUZ001-3`, `PUZ018-2`, `PUZ028-1`, `PUZ028-2`, `PUZ028-4`,
`PUZ036-1.005`, and `PUZ037-2`.

Standalone instantiation solved the four satisfiable `PUZ001/028` problems and
no refutation. Saturation supplied the other satisfiable solve and both
refutations. No method solved a held-out SWV or PLA problem. Every held-out
method/problem outcome was identical across both repetitions.

| Method | Verified coordinates | Median user CPU (s) | Maximum RSS (KiB) | Refutation bytes |
| --- | ---: | ---: | ---: | ---: |
| saturation | 14 | `0.015000` | 167,300 | 1,164,858 |
| standalone | 8 | `0.065981` | 21,880 | 0 |
| portfolio | 14 | `0.192985` | 167,296 | 1,164,858 |
| cooperative | 14 | `0.182985` | 167,288 | 1,087,284 |

Cooperation exchanged 4,353 ground instances in 36 train/held-out coordinates.
It added and lost zero solve relative to both saturation and portfolio.
Cooperative/portfolio ratios on common verified held-out coordinates were:

- median user CPU `0.995075`, p95 and maximum `1.0`;
- maximum proof bytes `1.000353`; and
- maximum RSS `1.005783`.

Cooperative/saturation user CPU had median `18.298500` and maximum `202.141`
over the ten common coordinates with nonzero measured saturation CPU. The
exchange therefore did not beat the equal-budget independent saturation worker
in solve yield or cost.

## Refinement behavior and proof size

The 94 long/short instantiation runs made 465 SAT calls, performed 371
refinement iterations, generated 10,374 distinct ground clauses, and enumerated
14,570,530 substitutions. Per run:

- generated instances had median 58.5, p95 396, and maximum 473; and
- refinement iterations had median 0, p95 30, and maximum 77.

The candidate returned 22 complete SAT models and 72 UNKNOWN results. It
returned no UNSAT result. Its measured candidate proof count and proof bytes
are therefore zero—not an unchecked proof gap. The synthetic integration
fixture separately required four refinements, reached UNSAT, produced a
non-unit DRAT trace, and passed `drat-trim`.

The two reproducible held-out refutations each occurred twice in saturation,
portfolio, and cooperation. Their aggregate emitted proof-solution sizes were
1,164,858 bytes for saturation and portfolio and 1,087,284 bytes for
cooperation.

## Correctness and falsification

The final Ubuntu replay passed:

- 13 focused parser, grounding, deduplication, DIMACS, model, independent
  verifier, and corpus tests;
- strict `-Wall -Wextra -Wpedantic -Werror` C++ compilation against the pinned
  public CaDiCaL API;
- a SAT fixture with exhaustive independent model validation;
- a four-refinement UNSAT fixture with independent DRAT checking;
- all 94 measured candidate certificate replays;
- all 22 measured complete-model replays;
- all 12 measured Umlaut proof replays under ProofCheck 1.0;
- all 36 augmented-input hashes; and
- seven rejected mutations: source hash, substitution, ground clause, complete
  model, DRAT trace, augmented clause, and Umlaut proof.

ProofCheck 1.0 self-certified all 117 bundled tests. The measured candidate had
no UNSAT result, so zero measured DRAT replays is the correct applicable count.
Malformed, unsupported, incomplete, or timed-out cases remained UNKNOWN.

## Diagnostics before held-out execution

Three implementation diagnostics were resolved without changing the frozen
corpus, algorithm, batch, budget, or decision rule:

1. an initial unit-refutable synthetic proof mutation legitimately allowed an
   empty DRAT trace, so the mutation fixture was strengthened to a non-unit
   refutation before corpus execution;
2. the third train problem exposed two source clauses producing the same false
   instance in one refinement batch; the implementation now distinguishes
   clauses solved by the preceding SAT call from duplicates added earlier in
   the current batch, matching the frozen distinct-instance semantics; and
3. two resume attempts found that a runner source sync removes release build
   output; binary identity is now captured at phase start and the final release
   binary was rebuilt once before held-out execution.

The first two completed train coordinates were safely reused because neither
encountered the corrected duplicate case. Every held-out coordinate used the
final implementation. No partial held-out outcome was inspected before the
matrix completed.

## Decision

All correctness gates pass, no method has a polarity disagreement, cooperation
loses no saturation solve, and repeated outcomes are stable. Three mandatory
advancement gates fail:

- cooperation adds no solve over saturation;
- cooperation neither adds a solve over portfolio nor reaches the 0.90 median
  common-solve CPU ratio; and
- standalone instantiation adds no solve over saturation.

The preregistered decision is `leave_production_unchanged`.

## Evidence and limits

The measured release Umlaut SHA-256 is
`c3493604f0d5be15c04a5b2a3f14dfa30e672edea6ae4bab94c5353169d55e65`.
The experiment-only CaDiCaL adapter is
`93a9acd041a220ed1f81498dc144a269a63c057c17e4d32537175b3f38d4f3c4`;
`drat-trim` is
`d55cfb5a2bd0d09884141515be0da78bbbcf796fae277aee8da3e96e73aa2c9a`;
and ProofCheck is
`92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`.

The machine-readable analysis has SHA-256
`1d13670bffc65e68375bf8dd172db3905552e9b9ff225052ac923bf01c040d8f`;
the full validation report is
`4f176254385e011737ddbe3f4bf65195c31dfa1ff5b29fb456c044053a338ac1`.
The ignored evidence archive is
`.artifacts/experiments/2026-07-30-008-instgen-epr-prototype/evidence.tar.gz`,
10,064,880 bytes, 1,863 entries, SHA-256
`e11a921e419e9c6f32af04340d04bdec57d5a661e6c1b8263f39ad6764582f3c`.
It excludes the redistributable checker bundle and the CASC source corpus.

This is a 29-problem, seven-family, four-second study of a deliberately finite
CNF fragment. The exhaustive counterexample scan is much simpler than full
Inst-Gen selected-literal inference, equality handling, dismatching
constraints, or calculus cooperation. The negative decision applies to this
bounded architecture; it does not establish that every future instantiation
calculus is uncompetitive.
