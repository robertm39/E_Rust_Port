# Inference and simplification gap audit

Bead: `E_Rust_Port-9jt.7.5`

## Outcome

The semantic audit found substantially more direct support than a rule-name
comparison suggested:

- 11 mechanisms are production-connected and focused-testable;
- one ordered-factoring path is a tested library-only compatibility utility;
- non-unit subsumption demodulation and constrained forward ground joinability
  are partial;
- UR-resolution and a dedicated term-algebra calculus are missing; and
- theory instantiation/arithmetic simplification belongs to the existing
  arithmetic/QE track.

The most important correction is inner rewriting. Umlaut already implements
the reference operation exactly in `clause_local_rw`: an oriented negative
equality in a clause rewrites the clause's other literals. The selected-clause
forward-modification path calls it under `local_rw`, the CLI exposes
`--local-rw=true`, and proof ancestry records `DC_LOCAL_REWRITE`. The generated
schedules leave it off. This supersedes the older conclusion that Umlaut had
only adjacent mechanisms.

The frozen evaluation therefore tested existing local rewriting rather than
implementing a larger missing rule. It did not meet the decision gate, so
production code and schedules remain unchanged.

## Clean-room boundary

The audit used Umlaut source, focused tests, the bundled E compatibility
source, official public Vampire documentation, and integrity-pinned executable
interfaces. Vampire implementation source was not inspected or copied.
Reference binaries remain experimental/checker-only artifacts and are not
product dependencies.

The official Vampire guide supplied semantic terminology for ordinary
resolution/superposition, redundancy, and equational tautologies:

- <https://vprover.github.io/vampireGuide/docs/lectures/l2/>
- <https://vprover.github.io/vampireGuide/docs/lectures/l4/>
- <https://vprover.github.io/usage.html>

## Capability matrix

The full soundness conditions, exact route markers, proof-operation markers,
test filters, and absence boundaries are machine-readable in
`capability-matrix.json`. The Ubuntu matrix controller validated all 17 rows
and ran 13 focused tests individually. Every filter matched exactly one
passing test.

| Mechanism | Classification | Production and executable evidence |
| --- | --- | --- |
| Ordered superposition and predicate resolution | direct | Indexed/plain paramodulation dispatch; a ground predicate-resolution witness; `DC_PARAMOD`. |
| Equality resolution | direct | Selected-clause generation calls equation resolution; substitution witness; `DC_EQ_RES`. |
| Equality factoring | direct | Enabled by default with an explicit disable option; ordered side-condition witness; `DC_EQ_FACTOR`. |
| Ordered factoring utility | library-only | Constructors, metadata, gating tests, and `DC_ORDERED_FACTOR` exist, but no Rust production caller exists. Bundled E likewise calls equality factoring and leaves `ComputeAllOrderedFactors` dormant. |
| Ordinary demodulation | direct | Forward levels and indexed backward simplification; nested subterm/top rewrite witness; `DC_REWRITE`. |
| Clause subsumption | direct | Unit/non-unit direct and fingerprint-indexed paths; shared-substitution witness. |
| Contextual simplify-reflect | direct | Forward, aggressive, and backward routes; flipped-literal witness; `DC_CONTEXT_SR`. |
| Condensation | direct | Selected/generated controls; fixed-point condensation witness; `DC_CONDENSE`. |
| Equational tautology deletion | direct | Ground completion and normalization, not merely complementary literals; chained negative-equality witness. |
| FOOL lowering and Boolean simplification | direct | FOOL unrolling, ITE/LET lifting, Boolean simplification, and a recorded CNF phase; `DC_FOOL_UNROLL`. |
| Inner rewriting | direct, default-off | `clause_local_rw` is called from selected-clause modification under `--local-rw=true`; semantic and proof-control witnesses; `DC_LOCAL_REWRITE`. |
| Injectivity-definition preprocessing | direct | Recognition and inverse-function replacement options; construction witness; `DC_INV_REC`. |
| Non-unit subsumption demodulation | partial | Contextual simplify-reflect plus unit demodulation do not use a non-unit conditional equality as a demodulator. |
| Forward ground joinability | partial | Full demodulation and strong RHS instantiation do not provide the dedicated constrained equality-deletion rule. |
| UR-resolution | missing specialization | Ordinary ordered predicate resolution covers completeness; no multi-unit specialization or route was found. |
| Term-algebra rules | missing calculus | Generic equality and injectivity-definition preprocessing do not provide constructor distinctness, exhaustiveness, or acyclicity. |
| Theory instantiation/arithmetic simplification | owned elsewhere | The exact-numerics and arithmetic/QE experiments own this architecture and certificate boundary. |

The prior stronger-redundancy experiment remains the executable evidence for
the two partial redundancy rows. Its 752 coordinates found no held-out default
change from the strongest existing approximations, so neither partial rule was
promoted merely because a reference prover names it.

## Frozen shortlist

Only three items survived evidence ranking:

1. selective local/inner rewriting, because it was already implemented,
   production-connected, proof-recorded, and low risk;
2. UR-resolution, as a missing first-order efficiency specialization; and
3. term-algebra rules, as a real but much broader typed-calculus capability.

The audit evaluated the first item. It did not prototype UR-resolution or
term-algebra rules after the lower-risk candidate failed its production gate.
Those two items are precise follow-up candidates, not implied priorities over
the remaining Beads.

## Local-rewriting experiment

The candidate-blind evaluation reused the exact family-aware CASC-30 test
selection from the prior redundancy study: six FEQ, six FNE, two EPS, and six
UEQ problems. No `local_rw=true` result on these coordinates had been
inspected before preregistration.

Baseline and candidate used the same KBO6, full-forward-demodulation, and
`5*Refinedweight + 1*FIFO` configuration. The only difference was
`--local-rw=true`. Every problem ran twice at short 5/7-second and larger
20/23-second soft/hard budgets with proof objects, aggregate telemetry, and
isolated maximum resident pages. All 160 first executions completed; the
second invocation resumed 160/160 without changing a result.

The contract is
`870f13dc65aac6b14973c9a7c85dfbd39d3211761402b8861a4bda855cd7646f`.
The all-feature release binary is
`db84f7d4a12927adb730a46930b065f2e919156a4b77747a5d40b79bd2a78ec6`.

### Coverage and common solves

Both strategies reproducibly solved the same three problems at both budgets:

- FEQ `NUN086+2`;
- UEQ `NUN134-1`; and
- UEQ `REL005-1`.

There were no candidate-only or baseline-only solves, no terminal-polarity
disagreement, no external timeout, and no contradictory status. Across the six
paired repetitions for these common solves:

- generated, processed, high-water, term-storage, and rewrite-step ratios were
  exactly 1.0;
- local rewriting's median CPU ratio was 1.078112 at the larger budget and
  1.078169 at the short budget; and
- the common-solved median RSS ratio was about 1.023.

Thus the candidate had no useful solved-case search effect and was about 7.8%
slower on the only reproducible proofs.

### Timeout-limited search

The option did affect search: 33 larger and 35 short paired coordinates with
telemetry on both sides changed at least one generated, processed, high-water,
final-total, or rewrite metric. Across all runs, including resource-limited
failures, candidate
generated-clause ratios were 0.966464 (larger) and 0.955480 (short).

Two candidate `PLA038-1` runs returned the expected graceful `ResourceOut`
status and exit code 8 but did not publish their optional telemetry file. Their
stdout/stderr and terminal status hashes remain in the contract; ratio
calculations omit the missing metric value, and behavior-effect counts require
telemetry on both sides. The other 158 runs have telemetry. This does not
affect coverage, status polarity, proof validation, or the negative decision.

Those reductions do not satisfy the preregistered gate. They occur mainly in
budget-limited coordinates, add no solve, and are smaller than the required
10% common-solved generated reduction. The maximum-of-runs larger-budget RSS
ratio was 1.046151, within the 1.05 guard but too close to constitute positive
evidence. No emitted successful proof used a visible `local_rw` inference;
the focused semantic and proof-control tests still demonstrate that the rule
and ancestry path fire.

## Independent proof validation

ProofCheck 1.0 matched the pinned release archive and executable hashes and
passed all 117 self-certification tests after the bundled ATP helpers were made
executable on the Linux worker. The existing Skolem-metadata and UEQ
alpha-source adapters then checked one representative larger-budget proof for
each reproducible strategy/problem pair.

All 6/6 claims returned `VerifiedGood`:

- baseline and local-rw on `NUN086+2`;
- baseline and local-rw on `NUN134-1`; and
- baseline and local-rw on `REL005-1`.

The proof-validation report ID is
`6087573c7527cc5ef0ced65ae4077fe0844e4325ec80f87ae2b0d494bfc65ff4`.
All 80 paired statuses agreed exactly.

## Decision

Retain local rewriting as an explicit default-off option. It passed soundness,
behavior-effect, and maximum-RSS validity gates, but failed both promotion
paths:

- zero candidate-only solves rather than the required two; and
- only three common solves rather than four, with generated/high-water ratios
  of 1.0 rather than the required 0.90 generated reduction.

The 7.8% common-proof CPU regression provides additional evidence against
default scheduling. No Rust source or generated schedule changed.

The exact final result report ID is
`5aebb7357075c358479f4ab029871513b7edc97f1aea03b5860f7235c8423f41`.
The tracked JSON SHA-256 is
`0ab6043921919393122144544884a986cbcdd651c75125651a66b92a2acf59ac`.
The complete ignored evidence archive is 5.3 MiB with SHA-256
`6808ea0978b88d7c0af21a1bc47117e0e298952ff02f601fd3bff1213fea9238`.
