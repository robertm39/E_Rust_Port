# Propositional SAT preprocessing findings

Bead: `E_Rust_Port-9jt.4.7`

## Decision

Do not adopt CaDiCaL default preprocessing for extracted SAT workloads. Do not
advance the exact whole-problem specialist, and do not reuse transformed SAT
state across SATCheck calls.

Correctness is perfect after the experiment adapter reconstructs a total model:
all arms agree on every scope, all 36,000 SAT record assignments validate, and
all 70 required DRAT traces verify. Default preprocessing nevertheless adds no
solve, loses no solve, and misses the frozen cost gate. Relative to CaDiCaL
`plain`, its paired median total-wall ratio is `1.043207`, p95 is `1.763558`,
and maximum paired RSS ratio is `1.170732`.

The strict whole-problem recognizer accepts none of the 2,901 CASC-30
problems. Cross-call reuse also fails independently: only 28 of 92
same-problem consecutive pairs are add-only, median simplified-clause
retention is zero, and the retained captures have no stable atom or source
clause identity.

Production Umlaut code, options, schedules, packages, and dependencies remain
unchanged.

## Coverage

The whole-problem scan authenticated and classified all 2,901 manifest
records. The deliberately narrow complete fragment accepted zero:

| Rejection | Problems |
| --- | ---: |
| Non-CNF record | 1,998 |
| `include` statement | 528 |
| Non-propositional literal | 375 |
| **Total** | **2,901** |

This is a coverage result, not a claim that the rejected problems lack useful
ground or propositional structure. The recognizer refuses equality, variables,
function application, included axioms, and non-CNF records rather than
silently treating a first-order problem as Boolean SAT.

All 127 unique family-held-out sessions in the prior incremental-SAT archive
passed their dedicated `session_sha256`. The prior manifest's generic `bytes`
field describes the source capture rather than the generated `.isat` payload;
all 127 generated payload hashes match.

Every session contributes five exact query scopes, for 635 total:

| Query | Scopes |
| --- | ---: |
| Cold | 127 |
| Warm 1 | 127 |
| Warm 2 | 127 |
| Positive assumptions | 127 |
| Negative assumptions | 127 |

The scopes cover seven families (`MVA`, `NUN`, `PLA`, `REL`, `SEU`, `SEV`,
and `SWX`), four categories (`EPU`, `FEQ`, `FNE`, and `UEQ`), 16 to 1,551
declared variables, and 0 to 422 materialized clauses. The median is 71
variables and 26 clauses; p95 is 858 variables and 413 clauses. Median DIMACS
materialization cost is 98,508 ns and p95 is 804,777 ns.

## Correctness and reconstruction

The final benchmark has 38,100 unique records, 12,700 per arm. Each arm
reports the same 600 SAT and 35 UNSAT coordinates, corresponding to 12,000 SAT
and 700 UNSAT repetition records. There are no timeouts, errors, invalid
records, polarity disagreements, added solves, lost solves, or arm-only
solves.

All 12,000 SAT records from each arm have a complete assignment satisfying the
exact materialized DIMACS scope. CaDiCaL `plain` and default each produce a
proof for all 35 unique UNSAT scopes. Standalone `drat-trim` accepts all 70
traces against the original DIMACS:

| Proof measure | Result |
| --- | ---: |
| Required / attempted / checked | 70 / 70 / 70 |
| Aggregate trace bytes | 16,406 |
| Median checker cost | 44,262,849.5 ns |
| p95 checker cost | 45,837,705 ns |
| Maximum checker cost | 49,478,979 ns |

Exhaustive enumeration independently checks 20 small SAT scopes. The mutation
suite rejects a truncated model, corrupted input hash, corrupted source
mapping, and an empty proof for a non-unit-refutable two-variable UNSAT
fixture. The corresponding valid CaDiCaL proof verifies first. No
whole-problem mapping exists to exercise from the corpus, so mapping corruption
uses a synthetic mapping with the same deterministic atom, literal-polarity,
source-name, and DIMACS-clause contract.

The benchmark is restartable: an unchanged second invocation executes zero
solvers and resumes all 38,100 records.

## Costs and transformations

All times below are per process record. Wall time includes process startup;
insertion, simplify, and solve are measured inside the adapters.

| Arm | Median insertion | Median simplify | Median solve | Median wall | p95 wall | Median RSS | p95 RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Internal | 2,373 ns | 0 ns | 6,740.5 ns | 6.135 ms | 15.771 ms | 2,176 KiB | 2,304 KiB |
| CaDiCaL plain | 44,457 ns | 8,122 ns | 9,704.5 ns | 6.657 ms | 16.123 ms | 4,992 KiB | 5,248 KiB |
| CaDiCaL default | 44,562 ns | 123,330.5 ns | 6,500.5 ns | 7.117 ms | 17.164 ms | 5,376 KiB | 5,760 KiB |

Default preprocessing is effective as a transformer. Across all records its
median active-variable count goes from 4 to 0 and median irredundant-clause
count goes from 6 to 0. On the preregistered cold-session decision surface, 84
of 127 sessions reduce active variables or irredundant clauses by at least
10%, and the median remaining ratio is zero.

That reduction does not translate into an equal-budget solve gain or a total
cost win:

| Default versus plain | Result |
| --- | ---: |
| Common solved scopes | 635 |
| Added / lost solves | 0 / 0 |
| Median paired wall ratio | 1.043207 |
| p95 paired wall ratio | 1.763558 |
| Maximum paired wall ratio | 2.342131 |
| Median paired RSS ratio | 1.078947 |
| p95 paired RSS ratio | 1.121951 |
| Maximum paired RSS ratio | 1.170732 |

Process startup dominates these very small scopes, but the frozen decision uses
total wall and process RSS precisely so a cheaper in-solver phase cannot hide
integration overhead.

## Cross-call reuse

Cold scopes are ordered by source capture path within each source problem.
Their 92 consecutive pairs have:

| Reuse measure | Result |
| --- | ---: |
| Add-only pairs | 28 / 92 (30.43%) |
| Median original-clause retention | 11.76% |
| Median simplified-clause retention | 0% |
| Stable atom/source-clause identity | Absent |

The add-only and simplified-retention thresholds both fail. Stable identity is
an independent veto even if integer overlap had been favorable: atom integers
are local to a fresh SATCheck snapshot and cannot safely identify a source
atom or proof clause across calls.

## Frozen gates

| Gate | Result |
| --- | --- |
| Completed-arm polarity agreement | Pass; zero disagreements |
| Complete SAT model validation | Pass; 36,000 / 36,000 records |
| Required independent proof checking | Pass; 70 / 70 traces |
| Exhaustive small oracle | Pass; 20 scopes |
| Corruption rejection | Pass; all five recorded checks |
| Default loses no plain solve | Pass; 0 lost |
| Default adds a solve | **Fail**; 0 added |
| Default total-cost/RSS alternative | **Fail**; median, p95, and RSS thresholds missed |
| At least 20% materially reduced cold sessions | Pass; 84 / 127 |
| Whole specialist coverage | **Fail**; 0 accepted |
| Reuse add-only rate | **Fail**; 30.43% |
| Reuse simplified retention | **Fail**; 0% |
| Stable reuse identity | **Fail**; absent |
| Comprehensive lifecycle | Pass; zero compatibility or behavior mismatches |

The final controller decision has `correctness_passed: true` and all three
recommendation fields set to `false`.

## Repository validation

The final Ubuntu comprehensive lifecycle passed:

- `cargo fmt --all -- --check`;
- locked all-target/all-feature Rust tests, including 4,545 library tests;
- locked all-target/all-feature Clippy with warnings and `clippy::pedantic`
  denied;
- release builds for every canonical native binary;
- 42 independent solution-validation tests with one expected skip;
- Windows GNU all-target/all-feature test compilation and all canonical
  release binaries;
- native Rust and C reference smoke runs;
- 50 main comparisons with zero mismatches and 29 expected differences;
- 216 support-tool comparisons with zero mismatches and 16 expected
  differences;
- ten benchmark cases with zero behavior mismatches; and
- native Rust and C Callgrind smoke runs.

The aggregate Rust/C benchmark wall ratio was `1.1026098767111063`, which
triggered the lifecycle's warning for exceeding `1.100`. Production source is
unchanged in this Bead, and the preceding comprehensive run for the same
production commit measured `1.087790942227777`; the narrow overage is retained
as timing variance, not suppressed or used to change the experiment decision.

The validation summary SHA-256 is
`0c716ae70f5a790040c1e0685479f37e723c21f3b5e765563ab9629c9b4ec6a0`.
The ignored comprehensive archive SHA-256 is
`cc9ba84ea2d3e8196c99d46cd6e8b80cc84ad34759179beb0a553f0239714f28`.

## Pre-finalization diagnostics

Two instrumentation defects were retained rather than hidden:

1. The first timed matrix exposed that `InternalSatService` publishes a
   satisfying assignment only for variables it encountered. Its adapter
   therefore emitted incomplete models for 9,460 SAT records. All 9,460 had
   unique, in-range partial assignments and all 9,460 validate when every
   unmentioned declared variable is assigned false. The adapter was changed to
   perform that deterministic totalization, and the entire frozen 38,100-record
   matrix was rerun from scratch. The rejected first matrix has SHA-256
   `bc8ab0b277b9c63db7471f76fb596378c09b6479d33a5c9972b816c587874815`.
2. The first proof-mutation check flipped one byte in a real trace, but that
   trace remained sufficient. The real 70 proofs had already verified. The
   negative test was replaced with the non-unit-refutable synthetic fixture
   described above and certification alone was rerun. The earlier certificate
   report has SHA-256
   `380f8d480030a76e8f1f5fbd8739205b05318517115b47cd8ee74529f6754bdd`.

Before candidate execution, the input-only preparation pass was also corrected
to traverse the compressed CASC archive once, materialize every query in the
multi-query sessions, interpret the manifest byte field correctly, parse TPTP
statements lazily, enforce the frozen one-second timeout, and implement the
complete no-loss/RSS decision gates. No candidate measurement informed those
changes, and no corpus, arm, budget, repetition, or threshold changed.

## Reproducibility

Key SHA-256 values:

| Artifact | SHA-256 |
| --- | --- |
| Preregistration | `9b569467c46b2b71de6884a31d806261117c7120c0af73a796edac194c15387e` |
| Prepared manifest | `7099a7c83a1f6cef7b0585480e3ccc1563159dce2ac736f3703cc5bf36288737` |
| Final results JSONL | `ebe21a706b9af715ebed037201b6e419a5b9ffc82237b9efc00069a0eddfe02a` |
| Certificate report | `17de509020e72b3f498bf086ffc80cea49666a8eb42f5b407c1aa67241de5d10` |
| Final analysis report | `e027b19e8498befab30e4e2a91b9f4b18a678473e0086bbeeb8c9eeb8319e474` |
| Raw retained archive | `c1523f2126f3d976be63a1ac50b5b2417a8b993bbcd1d56b856eb055c45a631b` |
| Comprehensive archive | `cc9ba84ea2d3e8196c99d46cd6e8b80cc84ad34759179beb0a553f0239714f28` |
| CaDiCaL probe | `812afdff08f9bfa68473c445143c3c645e25cbb044145c2234ac39df2ebd16db` |
| Standalone `drat-trim` | `d55cfb5a2bd0d09884141515be0da78bbbcf796fae277aee8da3e96e73aa2c9a` |

The raw archive is ignored under
`.artifacts/experiments/2026-07-30-007-propositional-sat-preprocessing/`.
It contains 1,327 entries and an internal `checksums.sha256`; the separately
pinned large input archives are not duplicated.
