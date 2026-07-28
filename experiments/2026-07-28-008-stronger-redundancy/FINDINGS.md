# Stronger redundancy and demodulation study

Bead: `E_Rust_Port-9jt.7.2`

## Question and preregistration

Can any already-ported stronger redundancy option reduce Umlaut's active and
passive search state enough to repay its checking cost, without losing held-out
coverage or weakening proof validation? If so, which transparent configuration
is suitable for selective dispatch?

This section is preregistered before benchmark results are inspected.

## Capability and license audit

The Vampire source tree was inspected only to identify algorithm boundaries;
no Vampire implementation is copied. This preserves Umlaut's LGPL-3.0
compatibility boundary.

| Reference mechanism | Semantic operation | Closest Umlaut mechanism | Overlap |
| --- | --- | --- | --- |
| global subsumption | Incremental propositional abstraction and failed-assumption minimization remove literals justified by multiple clauses | indexed/direct clause subsumption, strong unit forward subsumption, contextual simplify-reflect | Partial. Umlaut has no SAT-backed multi-clause global-subsumption rule. |
| forward/backward subsumption demodulation | A conditional equality clause rewrites an unmatched literal after its remaining literals match the target | contextual simplify-reflect plus ordinary unit demodulation | Partial. Umlaut does not use non-unit conditional equalities as demodulators. |
| forward ground joinability | Deletes a positive unit equality when older demodulators join both sides under an ordering constraint | full forward demodulation and strong rewrite instantiation | Partial. Umlaut has no dedicated constrained joinability deletion rule. |
| inner rewriting | A negative equality in a clause rewrites the other literals in that same clause | contextual simplify-reflect, equality resolution, and external demodulation | Adjacent but not equivalent. |
| ordinary forward/backward demodulation | Unit equalities rewrite new or retained clauses | forward-demodulation levels, backward rewrite index, strong RHS instantiation, general-demodulator preference | Direct coverage. |
| condensation | A clause is replaced by a proper substitution instance that subsumes it | `condensation.rs` | Direct coverage. |

The study therefore evaluates the strongest existing approximations before
opening any clean-room implementation Bead for a genuinely missing rule.

## Corpus and split

The immutable CASC-30 manifest is used with its source-family partition. No
family crosses train, validation, and test. Four first-order categories expose
different redundancy workloads:

- FEQ: theorem problems with equality;
- FNE: theorem problems without equality, serving as an overhead control;
- EPS: satisfiable effectively-propositional problems; and
- UEQ: unsatisfiable unit-equality problems.

Calibration and validation each contain six problems per category (24 total).
The held-out test contains six FEQ, six FNE, both available EPS problems, and
six UEQ problems (20 total). Selection is deterministic, family-balanced within
each category where the manifest offers multiple families, and evenly spaced
within each family by official category order.

## Fixed baseline and candidates

Every configuration uses KBO6, full forward demodulation, and the same
`5*Refinedweight + 1*FIFO` given-clause policy. The only differences are the
redundancy controls:

1. indexed baseline;
2. direct, non-indexed baseline as the slow reference;
3. strong unit forward subsumption;
4. aggressive generated-clause forward subsumption;
5. selected-clause contextual simplify-reflect;
6. selected/generated plus backward contextual simplify-reflect;
7. selected-clause condensation;
8. selected/generated condensation;
9. strong rewrite instantiation plus preference for general demodulators; and
10. a bundle enabling all of the above.

Calibration ranks the eight candidates and advances three. Validation uses two
repetitions and advances one without looking at test data. Test runs the fixed
winner, baseline, and direct-subsumption twins of both, at 5-second and
20-second soft CPU budgets with two repetitions. The direct twin of the winner
is constructed from the exact selected flags plus
`--conventional-subsumption`.

## Slow-reference and soundness rules

The direct subsumption implementation is the slow semantic reference for the
indexed retrieval path. The final report must enumerate every proof/model
polarity disagreement between indexed and direct twins. A performance timeout
is not called a semantic disagreement, but every problem where both twins
produce a proof/model status must agree in polarity.

Every reproducible larger-budget proof claim will be checked independently with
ProofCheck 1.0 through the repository's positive-only validation gate. Model
claims are reported but are not treated as externally verified unless an
independent model checker is available. Any contradictory proof/model polarity,
checker rejection, corrupt/missing telemetry, or result-coordinate mismatch
invalidates advancement.

## Metrics and decision rule

The report will include:

- reproducible and category-specific coverage, unique solves, and portfolio
  union;
- aggregate CPU, generated/processed/final/high-water clauses, and maximum RSS;
- forward/aggressive/backward subsumption, contextual reflection,
  condensation, rewrite, and index-call counters;
- paired indexed/direct status decisions and overhead; and
- checker verdicts for all reproducible proof claims.

A configuration advances for selective dispatch only if the final selected
mechanism fires on held-out data, all proof claims verify, direct/indexed twins
have zero proof/model polarity disagreements, and one of these held-out
conditions holds:

1. at least two selected-only solves with no baseline-only solve; or
2. no coverage loss, paired median CPU at most 0.95 of baseline, generated
   clauses at most 0.90, high-water clauses at most 0.95, and maximum RSS at
   most 1.05.

Otherwise the existing defaults remain unchanged. A missing Vampire rule is
recommended for a later clean-room implementation only when the overlap and
activation evidence identify a gap that existing candidates cannot cover; this
experiment does not infer that a new rule is profitable merely because it is
absent.

## Results

### Reproducible execution

The authoritative run used Ubuntu 24.04 runner
`e-rust-codex-260728-145514-6af9` and the release Umlaut binary with SHA-256
`bfa6905a29c80c50420279ded641d46f0517de03ea85a9f4c28140a0c9065ea0`.
The immutable CASC-30 manifest SHA-256 remained
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`.

The three phase contracts were:

- calibration:
  `e83336ffed9cae076250117fbd23e5a836ae9ad29a8aa7fef88e6eba136dd867`;
- validation:
  `7a81d01f3353f80e2626ce1a7104457932be3e383a1c8058ccd94cf3193d9741`;
  and
- test:
  `40ece7588c959d3e57efa0b37d4f0b06b4e8d94699d22f5414b9cb6dfadb7f99`.

All 752 coordinates completed: 240 calibration, 192 validation, and 320
test runs. A second invocation of every phase hash-validated and resumed
240/240, 192/192, and 320/320 results respectively.

The ignored raw archive is
`.artifacts/stronger-redundancy/stronger-redundancy-raw.tar.gz`. It contains
the three phase contracts and runs, selections, analysis, checker commands,
adapted checker views, and proof reports. It is 27,931,970 bytes with SHA-256
`9fc08ba6f63e437f1611998a654797fa8eb2cd937030d6170e8291aed2452c0d`.

### Selection result

Calibration advanced the redundancy bundle, aggressive generated-clause
subsumption, and condensation. The bundle solved four calibration problems,
aggressive subsumption three, and condensation two; the bundle's extra
coverage came with much larger generated and high-water counts.

On family-disjoint validation:

| Candidate | Reproducible solves | Median solved CPU (s) | Median generated | Median high-water |
| --- | ---: | ---: | ---: | ---: |
| condensation | 6 | 0.733754 | 42,668 | 5,417.5 |
| aggressive forward subsumption | 5 | 0.144304 | 19,001 | 2,321 |
| redundancy bundle | 5 | 0.154078 | 19,001 | 2,195 |

The preregistered ranking therefore selected ordinary selected-clause
condensation. Its validation selection ID was
`40fcbe9eaaee568e071c510451c7dfb51d86f3cae8a97bc210f35f4615c4103d`.

### Held-out end-to-end result

Condensation and baseline each reproducibly solved the same three held-out
problems at both budgets: FEQ `NUN086+2` and UEQ `NUN134-1` and `REL005-1`.
There were no unique solves in either direction.

Across all 40 larger-budget paired coordinates, condensation divided by
baseline had:

- median aggregate CPU ratio `1.000137`;
- generated-clause ratio `0.994428`;
- processed-clause ratio `0.999790`;
- final-clause and high-water ratios `0.999257`; and
- maximum-resident-page ratio `0.998779`.

The short-budget ratios were likewise effectively one. Among reproducibly
solved larger-budget pairs, condensation was 1.9% slower by median CPU. The
feature was not dormant: in the larger-budget test, available telemetry
recorded 555,791 condensation attempts and 1,824 successful clause
condensations, with successes in 20 of 36 telemetry-bearing runs. Its actual
clause reduction was nevertheless too small to repay the checking cost.

The aggressive candidates did fire substantially during calibration.
Aggressive forward subsumption removed 446,899 generated clauses across 20
positive telemetry records. The bundle removed 272,656 clauses through that
path, performed 2,465,726 condensation attempts with 14,483 successes, and
recorded 912 contextual reflections. Their calibration-only coverage did not
survive validation, so no transparent category/family dispatch is justified.

### Slow-reference result

The indexed and direct baseline twins had 12 paired proof/model-terminal
coordinates; the indexed condensation and direct condensation twins had
another 12. Both audits had:

- identical three-problem reproducible coverage at both budgets;
- no proof/model polarity disagreement;
- no unique solve in either direction; and
- generated and high-water ratios exactly `1.0` on common solves.

The direct implementation was not slower on this small selected workload:
indexed/direct median CPU ratios were `1.009224` and `1.024375` for the
baseline at short and larger budgets, and `1.020745` and `1.024892` for
condensation. This does not argue for replacing the index—the direct audit is
small and time-censored—but it validates the observed indexed decisions
without revealing a semantic discrepancy.

### Proof validation and discovered output gap

ProofCheck 1.0 passed its 117-test self-certification and verified all 12
reproducible larger-budget proof claims. The report ID is
`806913cdada5b6d2bb252f07c271061e867cbcc6ae4e087b28252afc1e4efa58`.

UEQ proofs used the previously audited alpha-source controller. The FEQ proof
exposed a production TSTP interoperability gap: Umlaut emitted
`skolemize/status(esa)` without the required `new_symbols` and
variable-to-Skolem-term records, and nested the existential step under
variable-renaming, NNF, and distribution source terms. The narrow adapter in
this experiment:

1. identifies the newly introduced `eskN_A` symbols;
2. reconstructs each existential-variable to Skolem-term record from the
   cited parent's quantifier environment;
3. inserts an explicit named Skolemization intermediate for the audited
   compound wrapper set; and
4. asserts and hashes that formula kind, name, role, logical body, and original
   problem parent are unchanged.

ProofCheck changed from `VerifiedBad`, then `Unknown`, to `VerifiedGood` only
after those missing records and intermediates were present. Production repair
is tracked separately as P1 Bead `E_Rust_Port-ay6`; the experiment adapter is
not treated as the product fix.

### Decision

Retain the existing redundancy defaults. Condensation clearly activated and
was sound, but it produced no held-out complementarity, no material
active/passive reduction, and no CPU improvement. Aggressive subsumption and
the full bundle overfit their calibration families and lost one validation
solve relative to baseline.

Do not implement Vampire's SAT-global subsumption, conditional
subsumption-demodulation, constrained ground joinability, or inner rewriting
on the strength of this experiment. Those remain genuine capability gaps, but
the strongest existing approximations did not identify a profitable dispatch
regime. A future clean-room rule study should first capture rule-specific
candidate workloads and measure standalone hit rate before modifying the
saturation loop.
