# Typed finite-model function tables: findings

Bead: `E_Rust_Port-9jt.6.8`

## Result

The preregistered production-integration evidence gate passed.

The isolated worker found six independently verified family-held-out finite
models that unchanged Umlaut `--auto` did not report at the same 10-second
soft CPU budget:

- validation: `LCL354+1`;
- test: `SWW880+1`, `SWW886+1`, `SWW894+1`, `SWW918+1`, and `SWW919+1`.

Umlaut returned `ResourceOut` on all six. The positive-only validator and
pinned Vampire 5.0.1 returned `VerifiedGood` for every generated
interpretation. There were no unverified success claims.

No production search path was changed in this experiment. The result
authorizes a separately tracked, removable portfolio integration; it does not
make the Python prototype part of the default prover.

## Implementation

`fnt_model.py` invokes Umlaut's existing clausifier with `--print-types` and
parses its native type declarations plus fully typed `tcf`/`cnf` clauses. It
accepts recursive positive-arity functions, predicates, equality, constants,
and native nonempty TFF sorts. Missing or inconsistent types, interpreted
arithmetic, distinct objects, higher-order types, and other unsupported
syntax fail closed.

For a configured maximum cardinality, the encoding allocates:

- prefix activity literals for each native sort;
- one-hot total constant and function-table rows;
- predicate-table rows;
- one-hot values for nested ground terms linked to shared function rows; and
- guarded predicate/equality truth values and universal clause instances.

Ground instances are added only when a typed variable assignment first
becomes reachable. Every permanent instance is guarded by sort-activity
literals, so it remains sound across incomparable size vectors. One
long-lived instance of Umlaut's statically linked CaDiCaL 3.0.1 service
receives the growing clause set and solves each vector under exact-domain
assumptions.

Every SAT assignment is decoded and evaluated against all active typed clause
instances in Python before rendering. The renderer emits Vampire-compatible
`finite_domain_*` and `distinct_domain_*` formulas for every native sort,
then complete ground definitions for constants, positive-arity functions,
predicates, and propositions.

## Soundness gates

Local validation passes 17 focused Python tests. The tests cover:

- typed declarations, native sorts, and recursive terms;
- inconsistent/missing/interpreted types;
- function tables, nested functions, and universal guarded grounding;
- one-element UNSAT followed by a two-element SAT model;
- interpretation decoding and independent clause evaluation;
- typed model rendering and typed-conjecture adaptation;
- finite-vector and ground-instance limits;
- controller timeout output and process-group handling; and
- summary/status handling.

On Ubuntu 24.04, incremental and fresh SAT sessions agree on all four positive
fixtures. Both modes independently validate:

- a unary no-fixed-point function;
- a nested function term;
- a native two-sort function/predicate problem; and
- a conjecture countermodel.

The negative matrix makes no satisfiability claim:

| Case | Final status |
| --- | --- |
| finite bounds exhausted on an infinite-only theory | `GaveUp` |
| interpreted integer input | `Inappropriate` |
| SAT probe exits without a response | `Error` |
| ground-instance limit | `ResourceOut` |
| size-vector limit | `ResourceOut` |

The positive-only validation gate rejects all six one-change corruptions:
function row, predicate row, constant value, native-sort domain, claimed
status, and declared domain-element type. Five are semantic rejections; the
status corruption is a deliberate coverage-gap rejection.

The experiment-only Rust SAT probe passes `rustfmt` and strict warnings plus
pedantic Clippy against the production `cadical-static` feature.

## Frozen corpus

The corpus is the exact CASC-J11 `Problems.tgz` used by the predecessor
experiment:

- archive bytes: 95,117,986;
- archive SHA-256:
  `c3bb4b916303dfe427ddc666b0144bbb0ccf244e72bdae0f0864faceaca34180`;
- frozen 250-record manifest SHA-256:
  `ef57f0e7234e37e6745c4a8aee4fbcbafcd3ba355554439640c015626845c7c0`;
- family split: 158 train, 30 validation, 62 test.

The predecessor's training evidence guided the architecture. This experiment
did not rerun or tune on training. Validation selected the sole preregistered
configuration: native typed sorts, incremental guarded grounding, maximum
sort size three, at most 2,048 vectors and 5,000,000 ground instances, a
five-second SAT deadline, 15-second controller wall, and 10-second unchanged
Umlaut baseline CPU budget. Test was opened only after the fixture gates and
validation selection were frozen.

Historical include availability is reported rather than hidden. Of the 92
validation/test records:

- 65 enter the typed worker;
- 25 fail clausification because the CASC archive does not contain their
  referenced `Axioms/*.ax` files; and
- two test inputs exceed the 15-second inventory wall limit.

## Held-out results

| Split | Supported | Verified models | Bounds exhausted | Worker timeout | Resource out |
| --- | ---: | ---: | ---: | ---: | ---: |
| Validation | 22 | 1 | 8 | 13 | 0 |
| Test | 43 | 5 | 29 | 8 | 1 |
| **Total** | **65** | **6** | **37** | **21** | **1** |

All six models are unique against unchanged Umlaut under the registered
comparison. The successful final bounds are:

| Problem | Size | SAT variables | Cumulative clauses | Ground instances | Grounding s | Insert s | SAT s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `LCL354+1` | 3 | 1,264 | 6,906 | 2,140 | 0.0708 | 0.0026 | 0.0010 |
| `SWW880+1` | 3 | 42,494 | 731,203 | 9,869 | 6.5181 | 0.3407 | 4.7253 |
| `SWW886+1` | 2 | 18,783 | 199,366 | 963 | 2.7124 | 0.0872 | 0.2305 |
| `SWW894+1` | 2 | 1,411 | 13,977 | 67 | 0.0833 | 0.0043 | 0.0050 |
| `SWW918+1` | 2 | 6,619 | 581,173 | 812 | 5.8102 | 0.3907 | 0.2307 |
| `SWW919+1` | 2 | 10,735 | 173,714 | 1,665 | 1.9686 | 0.0749 | 0.2039 |

Across the 65 worker runs, 48 attempt multiple domain bounds. The 142
recorded bounds have median 788 new ground instances, 2,899.5 new clauses,
47,306 cumulative clauses, 19,570 propositional variables, 0.0492 seconds
grounding, 0.00855 seconds clause insertion, and 0.0214 seconds SAT time.
These new-versus-cumulative counters are direct evidence of incremental
grounding and solver reuse.

The unchanged Umlaut baseline reports 18 held-out model statuses, all on
different problems; it also reports 36 `ResourceOut`, 11 `GaveUp`, and 27
records without an SZS status (principally the same unavailable includes).
The finite worker is therefore complementary rather than a replacement for
the saturation portfolio.

## Provenance and retained evidence

The experiment ran on runner `e-rust-codex-260729-030434-6dcc`, a dedicated
eight-core Ubuntu 24.04.4 LTS host. The source snapshot was rooted at commit
`9c687e7a447f41481202fc00a24b9bc71da2e964`; exact experiment source hashes
are in the evidence archive. Relevant executable hashes are:

| Artifact | SHA-256 |
| --- | --- |
| Umlaut with `cadical-static` | `0237c17d1294afef25a6a48da95c85612c271b0d98406ce9e2cf064070f5a595` |
| experiment SAT probe | `5e672fb4945444f2c7732020b08e63962b0f960f790b21c74ee009aea2cb7b4e` |
| pinned Vampire 5.0.1 | `3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665` |

The ignored evidence archive contains the frozen manifest, all inventory,
prototype, baseline, per-bound, solution, validation, adversarial, fixture,
negative-path, controller-log, summary, and provenance records:

`.artifacts/experiments/2026-07-29-003-fnt-function-table-models/evidence.tar.gz`

It is 4,857,053 bytes with SHA-256
`3403ad8928c4bf2a4e7ac249d880c8b54a9354ee463c6454a8b126bb4e83bf71`.

## Repository quality gate

A second fresh runner, `e-rust-codex-260729-034352-4b9a`, independently
validated the source snapshot rooted at
`9c687e7a447f41481202fc00a24b9bc71da2e964`. It passed:

- formatting and strict Clippy across all features and targets;
- 4,464 Rust library tests plus all binary and integration-test targets;
- nine independent solution-validation controller tests;
- native optimized and Windows GNU x64 test/release builds;
- 50 primary compatibility cases and 216 support-tool cases with zero
  unexpected mismatches;
- the ten-case timing benchmark at a 1.072x aggregate Rust/C wall-time
  ratio, below the registered 1.10x regression threshold; and
- native Rust and C Callgrind smoke runs.

The complete ignored quality artifacts are under
`.artifacts/linode/260729-034352-4b9a`. The runner deleted its Linode and
firewall after collection.

## Decision

The preregistered decision rule passes:

- all emitted successes are independently `VerifiedGood`;
- every malformed/corrupted case is rejected;
- incremental and fresh solve sets agree on the exhaustive small fixtures;
- finite bounds, limits, timeouts, and protocol errors remain fail-closed;
- native sorts and positive-arity function tables are exercised directly;
- telemetry demonstrates incremental/domain-aware grounding and SAT reuse;
  and
- six family-held-out results are unique against unchanged Umlaut at equal
  resources.

The architecture is viable for a removable production portfolio worker. The
next change should port the audited typed encoding/controller boundary into a
Rust-owned worker behind an explicit nondefault option, preserve independent
model validation in CI, and repeat the held-out gate before enabling any
automatic dispatch.
