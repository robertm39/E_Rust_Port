# Rational/real Fourier-Motzkin slice findings

Bead: `E_Rust_Port-9jt.5.6`

## Decision

Do not advance this frozen slice into production.

The experiment demonstrates a small sound and proof-replayable arithmetic
inference kernel, and it closes all 15 designed synthetic contradictions
without losing a neutral case. It does not establish useful real-workload
coverage: held-out production extraction yields only one eligible clause in
one family, and native Fourier-Motzkin closes no production-derived workload.
The strict advancement decision therefore remains negative.

The original-source reference result is still strategically interesting.
Pinned Vampire 5.0.1 with ALASCA enabled and VIRAS disabled solves
`ANA135_1`, `ANA136_1`, and `ARI721_1` that its theory-axiom arm does not
solve at the same five-second limit. The frozen native slice reproduces none
of those gains. A future experiment would need a materially richer
ALASCA-style boundary, not tuning of this prototype.

Production Umlaut code, options, packages, and dependencies are unchanged.

## Provenance and calculus map

The experiment inspected only the BSD-3-Clause Vampire files named in
`PREREGISTRATION.md`. It did not inspect or translate the unlicensed VIRAS
implementation. The canonical local-only Vampire executable contains VIRAS
but every ALASCA invocation used the full
`--virtual_integer_real_arithmetic_substitution off` option. No Vampire or Z3
source or binary enters a tracked file or Umlaut package.

The mapped architectural boundary is:

1. Normalize a selected rational/real interpreted literal to an exact
   polynomial greater than, or greater than or equal to, zero.
2. Standardize two premise clauses apart before selecting one same-sort
   arithmetic literal from each.
3. Select opposite-sign occurrences of an eliminable variable and combine the
   literals with exact positive multipliers.
4. Delete the pivot, retain both clause contexts, and make the conclusion
   strict if either premise is strict.
5. Keep ordinary propositional resolution as a separately certified rule.
6. Reject integers, floor/ceiling, equality, nonlinear arithmetic,
   nonconstant division, uninterpreted arithmetic monomials, and mixed sorts.

Vampire's full ALASCA engine replaces or specializes ordinary superposition,
equality factoring, and resolution on interpreted arithmetic literals and adds
ordering, abstraction, normalization, and additional arithmetic rules. The
experiment deliberately does not claim that its propositional-resolution
interaction is a substitute for that term-level integration. A future
production design would need one owner for interpreted polynomial literals so
ordinary superposition and arithmetic inference cannot duplicate or
contradict each other's work, while non-arithmetic literals and terms continue
through the ordinary calculus.

## Prototype and proof boundary

`fm_core.py` implements exact `fractions.Fraction` normalization,
alpha-renaming, constant evaluation, exact subsumption, propositional
resolution, and bounded Fourier-Motzkin saturation. Every FM record identifies
both parents, selected literal indices and variables, positive multipliers,
standardization-apart maps, the normalized conclusion, and stable clause
hashes.

`fm_replay.py` reparses the source workload, requires topological parent
availability and stable hashes, independently recomputes the resolution or
positive linear combination, checks pivot cancellation and strictness, and
accepts `unsat` only when the replay reaches the unique empty clause.

The final robustness suite passes 27 tests. It includes all eight frozen
certificate mutations, three bound paths, timeout, two cancellation paths,
malformed input, unsupported syntax, exact normalization, mixed
FM/propositional closure, production extraction, source selection, and exact
SMT-LIB/TPTP rendering. The final 490 measured native certificates all replay:
450 synthetic certificates plus 40 production-subset certificates. A
post-measurement fail-closed audit tightened only preexisting cancellation and
resource-bound ordering; all 98 representative default-bound certificate
pairs remained identical after removing elapsed-time counters.

## Synthetic corpus

The deterministic generator emits 45 workloads, 15 per partition. Each
partition contains five expected satisfiable cases, five expected
contradictions, and five unsupported cases; ten workloads per partition are in
the supported fragment. Template families do not cross partitions.

| Partition | Supported | Native-only closures | FM generated | Propositional generated |
| --- | ---: | ---: | ---: | ---: |
| Train | 10 | 5 | 14 | 18 |
| Validation | 10 | 5 | 14 | 16 |
| Test | 10 | 5 | 18 | 26 |

All 15 contradictions close only in the FM arm. All 15 satisfiable controls
remain native `unknown`, and every unsupported workload remains `unknown`.
The mixed test family requires an FM-derived propositional context followed by
ordinary resolution, so the result directly exercises the intended
interaction.

Pinned Z3 returns 15 `sat` and 15 `unsat`, agreeing with all 30 supported
expected outcomes. The native arm agrees literally with Z3 on the 15
contradictions but returns `unknown` for all 15 Z3-satisfiable cases; the
frozen status-agreement gate therefore fails 10/20 on held-out workloads.
Both Vampire arms have identical synthetic outcomes: 15 `unsat`, four `sat`,
and 11 `unknown`. ALASCA adds no synthetic solve over Vampire theory axioms.

On Ubuntu, the combined held-out native p95 is 14.601212 ms per workload. The
held-out retained-clause median and p95 growth ratios are both 1.0, no bound is
crossed, and the maximum observed exact coefficient width is four bits.

## Production-derived clauses

The frozen selector examines the committed CASC-30 TFA manifest and chooses up
to five smallest direct sources per partition/family that contain a
`$rat`/`$real` declaration and a linear arithmetic token. It selects:

| Partition | Family | Sources |
| --- | --- | ---: |
| Train | ITP | 3 |
| Validation | ARI | 5 |
| Test | ANA | 5 |

Production `umlaut --cnf --tstp-out` captures all 13 sources without a timeout
or nonzero exit and emits 526,917 transcript bytes. Whole-clause extraction
retains 132 clauses from the three ITP train sources and one clause from
`ARI519_1`; all five ANA sources and the other four ARI sources have no
wholly-supported clause. The principal clause exclusions are 883
equality/disequality clauses, 858 clauses with unsupported quantified sorts,
614 with uninterpreted arithmetic terms, 116 with nonground opaque context,
and 23 with unknown or mixed arithmetic sorts.

The production gate is held-out: it therefore sees one clause in one family,
not the required 20 clauses in three families. Across all partitions there are
133 clauses in two families.

Normalization and subsumption reduce the four production workloads to 5, 6,
7, and 1 active clauses. Neither arm generates an inference; in particular,
there is no pair of selected isolated arithmetic occurrences with opposite
sign. Native FM closes zero workload. Z3 classifies the four retained subsets
as three `sat` and one `unsat`; both Vampire arms report one `unsat` and three
`unknown`. The held-out `ARI519_1` subset is the Z3/Vampire `unsat` case, which
the one-rule native slice leaves `unknown`.

## Original-source controls and discovered defect

At five seconds per source:

| Arm | Solved |
| --- | ---: |
| Production Umlaut | 0 |
| Vampire theory axioms, ALASCA off | 1 |
| Vampire ALASCA on, VIRAS off | 4 |

The three ALASCA-only solves are `ANA135_1`, `ANA136_1`, and `ARI721_1`;
`ARI519_1` is solved by both Vampire arms.

Production Umlaut does not merely time out: every source exits 101 at
`src/terms/signature.rs:1859` with
`choice-symbol scan requires external symbol types`. CNF-only capture succeeds,
so the fault lies on the search path. This independently actionable production
bug is tracked as `E_Rust_Port-kfy`; it is not hidden by the negative arithmetic
decision or fixed inside this experiment.

## Advancement gates

| Gate | Result | Evidence |
| --- | --- | --- |
| All trusted steps replay | Pass | 490/490 measured certificates |
| Mutations and failure paths fail closed | Pass | 27/27 tests; eight mutation classes |
| Held-out native/Z3 statuses agree | **Fail** | 10/20; native cannot certify ten Z3 `sat` cases |
| Production eligibility | **Fail** | 1 held-out clause in 1 family |
| At least three unique closures, including production | **Fail** | 10 synthetic, 0 production |
| No baseline or neutral loss | Pass | no losses |
| Median <=4x and p95 <=10x growth | Pass | 1.0x / 1.0x |
| Native held-out p95 <=50 ms | Pass | 14.601212 ms |
| Coefficient limits enforced | Pass | no silent crossing; maximum 4 bits |
| Optional/removable package <=256 KiB | Pass | experiment-only Python; 0 release-byte and dependency delta |
| Comprehensive Ubuntu lifecycle | Pass | run `260730-183943-8400`; 50 main, 216 tool, and 10 benchmark cases with zero mismatches |

The comprehensive lifecycle also reports zero benchmark behavior mismatches
and a 1.079844x aggregate Rust/C wall-time ratio. Its validation summary
SHA-256 is
`eb14af701ec0a0c89217937ea53dfa318c9be258316d449b54cc66232c902113`.
The final verdict remains `do_not_advance` because three efficacy/coverage
gates fail.

## Reproducibility

The preregistration SHA-256 is
`4d7522fdbaabf8d5468cb65d900dbbb02cc347323033cd891a174e7888c5d7b2`.
The synthetic corpus SHA-256 is
`4536d12567b0ae1e77d9487f879ce1d1b1db738de7ee0b2ecd7931a3a368295a`.
The source selection SHA-256 is
`12d9598e95d195ef4d1079ad62beab5d6a6870646fb5719c3669c911af0fb4a1`,
which matches the production capture metadata.

Raw captures, certificates, external output, and reports remain ignored under
`.artifacts/experiments/2026-07-30-003-rational-fm-slice/`. The final
`results.json` records their key hashes and every gate.
