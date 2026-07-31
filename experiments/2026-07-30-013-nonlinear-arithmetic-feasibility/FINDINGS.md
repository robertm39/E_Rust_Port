# Nonlinear arithmetic and model-based projection: findings

Bead: `E_Rust_Port-9jt.5.8`

## Decision

**Reject the whole-problem QF-NRA service boundary and do not implement it.**
Pinned Z3 decided every eligible query, but only three of the 2,901 CASC-30
problems are whole-problem QF-NRA under the preregistered boundary. That misses
the frozen minimum of five before proof, deployment, or cost is considered.

**Defer model-based projection as an in-search research direction.** The CASC
inventory does not measure branch-local projection demand, the pinned command
surface does not expose a supported nonlinear-QE certificate service, and the
current NLSAT tactic produces no replayable proof. Revisit only after captured
Umlaut branch traces show material nonlinear projection demand. This study
does not create that follow-up or commit an implementation.

Production remains unchanged. Z3 is neither a dependency nor a trusted proof
oracle.

## Frozen setup

The preregistration was frozen before solver execution at SHA-256
`fca40ab740ed18d06b661c86c7e7b02f3b347dd7fa27c544fcaf96875249e25f`.
It fixed:

- a complete hash-verified CASC-30 census;
- pure, whole-problem QF-NRA as the smallest candidate;
- quantified NRA as a separate measurement;
- a two-repetition, 10-second pinned-Z3 protocol;
- zero trusted coverage without dependency-independent replay;
- a `large` implementation threshold above 20,000 nonblank, non-comment
  reference lines; and
- the pursue/defer/reject rule.

The local inventory verified all 2,901 problem hashes against
`benchmarks/casc_2025_manifest.jsonl`. Because the external corpus is excluded
from normal runner source sync, it emitted four content-addressed SMT-LIB
queries. The runner rejected query-hash mismatches before execution and then
ran only those four queries. The inventory JSON SHA-256 is
`0280beca7b0a2a4b2b55d43d269c1506493811b731778cc9a34ef7d55488362d`.

The external reference was the existing MIT-licensed Z3 source commit
`2d48fd119ce5074b880944c2b1c59e537c99cd46`. Its 6,830,870-byte source archive
had SHA-256
`9b78c0cc9f330dab9f39c132aba39c92fdba2dbc0aac26dd07b3946592dd21d8`.
It was built on retained Ubuntu runner `e-rust-codex-260731-021324-06a2` with
GCC 13.3.0, CMake 3.28.3, Ninja, and four build jobs:

```text
cmake -S /opt/e-rust-port/z3-src -B /opt/e-rust-port/z3-build \
  -G Ninja \
  -DCMAKE_BUILD_TYPE=Release \
  -DZ3_BUILD_EXECUTABLE=ON \
  -DZ3_BUILD_TEST_EXECUTABLES=OFF \
  -DZ3_BUILD_LIBZ3_SHARED=OFF
cmake --build /opt/e-rust-port/z3-build --parallel 4
```

The resulting 37,152,512-byte executable reported
`Z3 version 5.0.0 - 64 bit` and matched the previously audited SHA-256
`f331d9f5953deaf88a900f83b45a62a7e3d63319a8dd89ca59c53abe02616bf9`.

## Demand inventory

All 150 TFA problems contain typed arithmetic. The broad nonlinear-syntax
upper bound contains 77 problems: 49 TFI and 28 TFE. This is not candidate
coverage; it deliberately includes integer and mixed-theory products that
NLSAT cannot accept under the frozen boundary.

The exact whole-problem classification is much smaller:

| Fragment | Problems | TFA share | Full-corpus share |
| --- | ---: | ---: | ---: |
| pure real linear | 2 | 1.33% | 0.069% |
| pure whole QF-NRA | 3 | 2.00% | 0.103% |
| pure whole quantified NRA | 1 | 0.67% | 0.034% |
| all eligible nonlinear real | 4 | 2.67% | 0.138% |

All six pure-real problems are in the `ARI` family and the manifest's
validation split. The three QF-NRA problems are `ARI628_1`, `ARI629_1`, and
`ARI631_1`; the quantified case is `ARI536_1`. Thus the candidate offers
neither family nor split diversity.

The 144 ineligible TFA problems have exhaustive, stable first reasons:

| Exclusion | Count |
| --- | ---: |
| user function/predicate or mixed theory | 75 |
| integer/rational variable or used symbol | 42 |
| rounding, remainder, coercion, or other unsupported arithmetic | 21 |
| included external axioms | 4 |
| symbolic division | 2 |

Across all divisions, 736 files contain a numeric sort, defined arithmetic
token, or numeric literal. That lexical number is contextual inventory only;
it must not be presented as nonlinear-real coverage.

## External-solver coverage

Every translated problem was run twice in a fresh shell-free Z3 process. The
QF cases used `qfnra-nlsat`; the quantified case used `nlqsat`.

| Problem | Fragment | Degree | Raw status | Two fresh-process times |
| --- | --- | ---: | --- | --- |
| `ARI628_1` | QF-NRA | 3 | `unsat`, `unsat` | 9.823 ms, 9.392 ms |
| `ARI629_1` | QF-NRA | 3 | `unsat`, `unsat` | 9.451 ms, 9.429 ms |
| `ARI631_1` | QF-NRA | 5 | `unsat`, `unsat` | 67.335 ms, 67.441 ms |
| `ARI536_1` | quantified NRA | 2 | `unsat`, `unsat` | 8.234 ms, 8.817 ms |

All eight raw results were deterministic and matched the manifest's theorem
classification. The QF raw expected-result coverage was 3/3 (100%), clearing
the 80% raw solver gate. The `return_unknown` baseline would return four
`Unknown` results, add zero bytes, and accept no untrusted step.

The result establishes that pinned Z3 can decide the tiny eligible slice. It
does not establish a competitive search benefit: the process timings include
startup, the sample has one family, and only three ranked QF problems are in
scope.

## Proof and trust gap

Trusted solver coverage is **0/4 problems and 0/8 raw decisions**.

The pinned source makes the limitation explicit:

- `src/nlsat/tactic/nlsat_tactic.cpp` calls
  `fail_if_proof_generation("nlsat", g)`;
- `src/nlsat/tactic/qfnra_nlsat_tactic.h` describes the supported QF-NRA
  tactic, but does not define a proof certificate;
- `src/qe/nlqsat.h` registers `nlqsat`, while `nlqe` remains a
  `TBD_TACTIC`;
- `src/qe/mbp/mbp_arith.h` describes Loos-Weispfenning projection for a basic
  conjunction, not a nonlinear proof protocol; and
- `src/qe/qe_mbi.cpp` warns that arithmetic projection is not guaranteed to
  remove non-shared variables.

The live proof probe enabled proof production on an unsatisfiable nonlinear
formula. Z3 returned `unknown`, then
`proof is not available`, and exited nonzero in 7.761 ms. An unsatisfiable core
would identify input assertions but would not justify their nonlinear
inconsistency. A matching CASC status is also not proof. No SAT result occurred
in this sample; accepting one would additionally require exact validation of
real algebraic values and every polynomial relation.

Adoption would therefore require either making Z3 part of Umlaut's trusted
computing base, which violates the frozen gate, or designing a complete
independently checkable certificate/checker path. Raw coverage cannot be
converted into trusted coverage by interface work alone.

## Projection and reimplementation cost

The audited source surface is:

| Pinned subsystem | Files | Physical lines | Nonblank, non-comment lines |
| --- | ---: | ---: | ---: |
| `src/nlsat` | 31 | 15,073 | 11,686 |
| `src/qe` | 51 | 24,546 | 19,108 |
| `src/math/polynomial` | 18 | 21,464 | 15,576 |
| `src/math/realclosure` | 4 | 7,557 | 5,316 |
| **Total** | **104** | **68,640** | **51,686** |

Even omitting the general QE tree, the NLSAT, polynomial, and real-closure
substrates total 32,578 nonblank, non-comment lines. They span all seven frozen
obligations: polynomial normalization, exact real algebraic numbers, Boolean
conflict search, sound projection/cell construction, models, replayable UNSAT
evidence, and integration/hardening. The preregistered classification is
therefore `large`, with a minimum of 12 engineer-months.

For portfolio planning, a paper-level clean-room implementation has an
order-of-magnitude range of **18–35 engineer-months**:

| Work package | Planning range |
| --- | ---: |
| polynomial and real-algebraic substrate | 5–9 engineer-months |
| NLSAT search, cell construction, and projection | 6–12 engineer-months |
| proof certificate and independent checker | 4–8 engineer-months |
| typed integration, cancellation, fuzzing, and cross-platform hardening | 3–6 engineer-months |

This range is deliberately coarse and excludes schedule commitment. The
reference line count is not a port-size prediction, and a smaller algorithm
may exist; however, omitting proof/checker work does not satisfy Umlaut's trust
boundary. External delegation avoids most implementation work but adds the
37.15 MB process, MIT notice/version pinning, executable discovery, StarExec
provisioning, cancellation, and Windows validation.

NLSAT's internal conflict projection is valuable algorithmic evidence, but it
is not the same service as the QE tree's largely linear model-based projection.
Neither exposes the stable, independently replayable nonlinear projection
artifact Umlaut would need.

## Frozen gate result

| Gate | Result |
| --- | --- |
| at least five QF-NRA CASC problems | **fail: 3** |
| at least 80% deterministic expected raw result | pass: 3/3 |
| independent replay for 100% of accepted results | **fail: 0/4** |
| no unresolved deployment blocker | **fail** |
| reimplementation not `large` | **fail: 51,686-line surface** |

The preregistered outcome is `reject_candidate_boundary`. The positive raw
solver result does not override four failed gates.

## Validation and retained evidence

Nine focused tests pass locally and on Ubuntu. They cover tokenization,
comments and nesting, exact constants, degree classification, quantified
rendering, TPTP reverse implication, user symbols, symbolic/zero division,
includes, fail-closed solver output, manifest hash failure, and query-hash
failure.

The final report is 1,699,685 bytes with SHA-256
`ebb779733f19e8150af395b42f5b7d6eadbf7778ea74210ee508c5b1b31cdc28`.
The ignored evidence archive contains the inventory, report, four SMT-LIB
queries, run output, and Z3 configure/build/prerequisite logs:

```text
.artifacts/experiments/2026-07-30-013-nonlinear-arithmetic-feasibility/evidence-v3.tar.gz
```

It is 98,213 bytes with SHA-256
`8a487e93078e6e838b5adefedff47add9d5b80536366df4ec0932c22bbdca85c`.

## Limits

- CASC-30 is one competition snapshot. Current results do not predict future
  nonlinear problem mixes.
- The candidate is deliberately whole-problem and pure-real. It does not
  measure nonlinear atoms generated inside ordinary first-order search.
- The broad 77-problem nonlinear-syntax count is an upper bound, not eligible
  coverage.
- All eligible cases belong to one family and split.
- Solver agreement with competition status is differential evidence, not a
  proof.
- Fresh-process timings are coverage diagnostics, not a persistent-process or
  in-search performance benchmark.
