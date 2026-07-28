# Incremental SAT service and backend bake-off

Bead: `E_Rust_Port-9jt.4.1`

Status: complete; CaDiCaL selected for an optional production-integration
follow-up, with no dependency or default changed by this experiment.

## Question

What is the smallest backend-neutral incremental SAT contract that supports
Umlaut's near-term SAT-guided work, and which existing backend best satisfies
that contract on prover-shaped workloads without compromising proof integrity,
cancellation, licensing, or package reversibility?

## Pinned inputs

| Input | Revision |
| --- | --- |
| Umlaut | recorded by the run contract |
| CaDiCaL | `c60730422e758ef1cebe7aeddf2dda31c996bf04` (`3.0.1`) |
| MiniSat | `37dc6c67e2af26379d88ce349eb9c4c6160e8543` |
| PicoSAT | `965`, from E revision `17026b1bfe61aaf223cfaae54947c8d2679c31a0` |

The reference trees are ignored inputs. No reference implementation code is
copied into Umlaut. The experiment adapters compile against each backend's
documented public API.

## Preregistered contract

The prototype service uses signed nonzero 32-bit literals and provides:

1. deterministic permanent clause insertion;
2. repeated solve calls without resetting permanent clauses;
3. per-call assumptions that disappear after the solve;
4. four explicit outcomes: `sat`, `unsat`, `unknown`, and `error`;
5. a complete model for `sat`;
6. a sound subset of failed assumptions for assumption-dependent `unsat`;
7. a bounded native work limit and a deadline/cancellation path;
8. optional proof output whose format, scope, and independent checker are
   explicit;
9. telemetry that separates build, insertion, solve, core, proof, and
   process-level memory costs;
10. deterministic reset/destruction with no process-global solver state.

An implementation may report an unsupported capability, but it may not
silently approximate a model, core, proof, or resource outcome.

The tracked `.isat` protocol is deliberately simpler than a production Rust
API:

```text
p isat MAX_VARIABLE
a LITERAL ... 0
q QUERY_ID NATIVE_LIMIT DEADLINE_US ASSUMPTION ... 0
```

`a` permanently extends the session. `q` solves the current clause database
under only the listed assumptions. A negative native limit means unlimited; a
zero deadline disables the wall deadline. Each query produces one JSON record.

## Workloads and split

The suite has three disjoint roles.

- Exhaustive and seeded small CNFs are the semantic oracle set. Brute-force
  enumeration checks every status, model, and returned failed-assumption core.
- Instrumented Umlaut SATCheck captures are the performance set. A tracked
  patch writes the exact post-grounding, post-pure-filter DIMACS passed to the
  current solver. Problem identity, source hash, capture ordinal, variables,
  clauses, and originating solver configuration are retained.
- Structured pigeonhole, parity, and selector sessions stress limits,
  cancellation, incremental reuse, and core quality beyond the captured size
  distribution.

No workload used to debug an adapter may enter the held-out performance
summary. Seeds, corpus membership, repetitions, resource limits, and hashes
are frozen in the run contract before timed runs.

## Correctness and trust gates

- Small sessions must agree with exact brute-force enumeration.
- Every reported SAT model must satisfy all permanent clauses and active
  assumptions.
- Every reported failed-assumption core must itself be UNSAT with the permanent
  clauses. Core minimization is measured separately and is not required.
- Every global UNSAT claim selected for proof testing must have a proof accepted
  by an independently built checker. A matching second solver is not a proof.
- Forced tiny limits and deadlines must return `unknown`, never `sat` or
  `unsat` without the corresponding certificate checks.
- Adapter crashes, malformed output, timeouts, and unsupported operations are
  distinct failure classes.

Any false status, invalid model, invalid core, or rejected claimed proof is a
hard stop for that backend.

## Measurements

For each backend and session class the report records:

- status agreement and unknown/error counts;
- cold and warm solve latency, throughput, and reuse/fresh ratio;
- insertion and core-extraction time;
- returned core cardinality and independently minimized cardinality;
- native limit kind and forced-limit behavior;
- cancellation latency at several deadlines;
- proof bytes, proof generation overhead, and checker result;
- peak RSS, stripped adapter/library bytes, dynamic dependencies, build flags,
  source hash, and license notice;
- static and runtime-loaded packaging feasibility.

Performance comparisons use identical session order, pinned CPU affinity,
multiple repetitions, randomized backend order, warm-up outside measurement,
and medians plus tail percentiles. Process startup is reported separately from
in-process query time.

## Decision rule

A production backend recommendation requires:

1. zero correctness/trust-gate failures;
2. assumptions, sound failed cores, limits, and cancellation;
3. independently checkable UNSAT proof support, or an explicit architecture
   that replays every proof-producing claim through a proof-capable backend;
4. reproducible static Linux and Windows-GNU compile feasibility, or a
   documented optional runtime boundary with fail-closed fallback;
5. a material win on held-out prover captures (at least 2x median throughput,
   at least 25% p95 latency reduction, or a capability unavailable in the
   current backend) without a material memory/package regression.

Otherwise the result is an interface proposal and a decision to defer backend
adoption. A standalone SAT benchmark win is insufficient.

## Results

### Decision

Adopt the backend-neutral service contract and advance CaDiCaL 3.0.1 to an
optional production-integration follow-up. Do not adopt MiniSat, do not replace
the existing runtime-loaded PicoSAT path, and do not change Umlaut's default
solver in this Bead.

CaDiCaL was not the smallest or the fastest adapter at every percentile. It
was the only candidate that combined:

- zero semantic, model, core, or proof failures;
- direct standard DRAT proof output accepted by a standalone checker;
- the tightest tested native cancellation;
- reproducible static Linux and Windows-GNU builds;
- a held-out 75.2% p95 query-latency reduction under the validation-selected
  dispatch policy; and
- an MIT license already represented by a verbatim tracked notice.

MiniSat's validation-selected 128-clause policy had the best aggregate
latency, but MiniSat has no proof output, had looser cancellation, required
`-fpermissive`, a temporary MinGW source compatibility patch, and a cross-zlib
build input. PicoSAT has proof support, but its wrapped trace needs a validated
normalization step, its tail latency was worse, and its 100 microsecond
cancellation request returned after a 6.75 millisecond median.

This is a backend recommendation, not a silent dependency adoption. The
held-out dispatch-positive sample contains only 18 distinct captures from six
problems in two coarse families. Production work must therefore remain
optional and fail closed while it broadens end-to-end SATCheck and AVATAR
evidence.

### Proposed production boundary

The `.isat` prototype exercised the intended ownership boundary. A production
Rust interface should retain these semantics:

1. A service instance owns one permanent clause database and one backend.
2. `add_clause` is deterministic and is illegal while a solve is active.
3. `solve(assumptions, budget)` returns exactly `Sat(model)`,
   `Unsat(core, optional_proof)`, `Unknown(reason)`, or `Error(reason)`.
4. Models are complete assignments and cores contain only active assumptions.
5. Native work limits and cancellation are capability-reported; unsupported
   controls never masquerade as successful enforcement.
6. Proof format, assumption scope, finalization, and checker identity are
   explicit. A proof-required claim fails closed if proof creation or checking
   fails.
7. Telemetry separates insertion, solve, core, proof, and process memory costs.
8. Reset/destruction is deterministic, terminates callback ownership, and
   leaves no process-global solver state.
9. Backend choice is made at a stable service boundary. The initial measured
   dispatch feature is clause count, with the internal solver retained below
   the threshold and as the complete dependency-free disablement path.

The tracked C++ and Rust adapters are executable interface prototypes, not
production FFI. They call only public solver APIs. No upstream implementation
source was copied or translated into Umlaut.

### Workloads

The deterministic generated suite contains 85 semantic sessions and 11
structured sessions. The semantic run produced 1,316 query records across the
four backends: 524 SAT and 792 UNSAT. The captured performance corpus came from
an isolated instrumented Umlaut build that wrote the exact post-grounding,
post-pure-filter CNF entering `SATCheck`.

The first balanced capture pass produced 119 snapshots from 23 CASC-30
problems and 80 hash-distinct sessions. Training used 19 sessions and
validation used 34. The initial held-out set had no session at the selected
threshold, so it was correctly rejected as a coverage-gate failure rather
than reported as a win. Three deterministic, rank-frozen capture expansions
were combined and hash-deduplicated into the final held-out set:

| Property | Final held-out value |
| --- | ---: |
| Distinct sessions | 127 |
| Sessions at least 128 clauses | 18 |
| Source problems for those 18 sessions | 6 |
| Coarse families for those 18 sessions | 2 (`SEU`, `SWX`) |
| Categories | `EPU` 9, `FEQ` 37, `FNE` 40, `UEQ` 41 |
| Variable range | 5 to 6,363 in the initial balanced capture pass |
| Clause range | 0 to 3,656 in the initial balanced capture pass |
| Queries per session | 5: cold, two warm, positive and negative assumption |
| Measured repetitions | 5 after one unmeasured warm-up |

The final held-out benchmark produced 12,700 query records: 12,000 SAT and 700
UNSAT. Backend order was seeded and randomized, CPU affinity was pinned, and
process RSS/startup were measured separately from in-solver timings.

### Correctness and core trust

All four backends agreed on every completed semantic, structured, and captured
query. The exact small-CNF oracle checked every status, every externally
returned complete SAT model, and every failed-assumption core. There were no
process failures, malformed records, invalid models, invalid cores, or status
disagreements.

The current internal recursive DPLL exposes neither a complete model nor a
proof. Its adapter therefore returns an empty model and declares the capability
gap; it never fabricates a certificate. The three fully static Linux external
adapters separately passed all 85 semantic sessions: 987 records with zero
failures.

The large held-out formulas yielded 640 assumption-dependent UNSAT records.
Those reduced to 32 unique formula/core pairs. Both CaDiCaL and PicoSAT
independently re-solved every returned core, for 64 additional checks and zero
failures. The slowest such checking solve was 0.594 milliseconds.

### Captured performance

The headline distributions below are in-solver query time. RSS is process peak
RSS and includes the experiment adapter.

| Split/backend | Median solve | p95 solve | Median RSS | p95 RSS |
| --- | ---: | ---: | ---: | ---: |
| Train/internal | 3,966 ns | 19,673,807 ns | — | — |
| Train/CaDiCaL | 12,379 ns | 214,664 ns | — | — |
| Train/MiniSat | 10,686 ns | 309,147 ns | — | — |
| Train/PicoSAT | 11,537 ns | 736,502 ns | — | — |
| Validation/internal | 1,662 ns | 393,334 ns | 2,176 KiB | 2,176 KiB |
| Validation/CaDiCaL | 4,010.5 ns | 27,683 ns | 4,864 KiB | 5,120 KiB |
| Validation/MiniSat | 5,447.5 ns | 44,217 ns | 3,968 KiB | 4,096 KiB |
| Validation/PicoSAT | 5,799.5 ns | 81,033 ns | 3,840 KiB | 4,096 KiB |
| Test/internal | 2,975 ns | 146,801 ns | 2,176 KiB | 2,176 KiB |
| Test/CaDiCaL | 3,455 ns | 36,485 ns | 4,864 KiB | 5,120 KiB |
| Test/MiniSat | 3,836 ns | 48,894 ns | 3,968 KiB | 4,096 KiB |
| Test/PicoSAT | 3,766 ns | 101,944 ns | 3,840 KiB | 4,096 KiB |

The internal solver wins most tiny medians; an unconditional external backend
is therefore the wrong design. The internal solver's recursive tail dominates
larger captures, which makes a size dispatch effective.

On validation, the best raw policy was MiniSat for sessions with at least 128
clauses: total cost ratio 0.176845 relative to internal. Because the decision
rule requires either direct proof support or proof-capable replay, the selected
policy was CaDiCaL at the same frozen threshold. Its validation total-cost
ratio was 0.245981, with no session more than 25% slower than internal.

The fixed CaDiCaL/128 policy was then evaluated once on the expanded held-out
set:

| Metric | Internal | Selected policy | Ratio/change |
| --- | ---: | ---: | ---: |
| Aggregate insertion + five-query cost | baseline | measured | 0.657400 |
| Query p95 | 147,574 ns | 36,566 ns | 0.247781 |
| Aggregate reduction | — | — | 34.3% |
| p95 reduction | — | — | 75.2% |
| Candidate process samples | — | 90 / 635 | 18 sessions × 5 |
| More-than-25% session losses | — | 3 | all 146-clause `SWX022` captures |

The three losses had ratios 1.62, 1.30, and 1.25. A post-test sensitivity
check at 256 clauses had essentially the same aggregate ratio (0.659721) and
zero material losses, but it was not the selected policy and is not reported
as the confirmatory result. A production follow-up may compare 128 and 256 on
new data; it must not tune on this held-out set.

### Cancellation and native limits

Pigeonhole 14-to-13 sessions forced cancellation at three deadlines. Each
external backend returned `unknown` for all 30 repetitions at every deadline:
270 `unknown` results and zero process or validation failures.

| Backend | Deadline | Median return | p95 return | Maximum |
| --- | ---: | ---: | ---: | ---: |
| CaDiCaL | 100 µs | 110 µs | 122 µs | 133 µs |
| CaDiCaL | 1 ms | 1.015 ms | 1.034 ms | 1.049 ms |
| CaDiCaL | 10 ms | 10.077 ms | 10.112 ms | 10.118 ms |
| MiniSat | 100 µs | 1.093 ms | 1.600 ms | 1.612 ms |
| MiniSat | 1 ms | 2.137 ms | 2.467 ms | 2.490 ms |
| MiniSat | 10 ms | 11.235 ms | 11.427 ms | 11.458 ms |
| PicoSAT | 100 µs | 6.751 ms | 6.812 ms | 6.826 ms |
| PicoSAT | 1 ms | 6.755 ms | 6.801 ms | 6.802 ms |
| PicoSAT | 10 ms | 18.100 ms | 18.250 ms | 18.280 ms |

CaDiCaL reports decision budgets, MiniSat reports conflict budgets, and
PicoSAT's public solve limit is passed through as its decision limit. Trivial
structured formulas can validly finish SAT or UNSAT before a zero/tiny budget
interrupts them; every such result still passed its model/status checks.
Umlaut's current internal DPLL has a decision limit but no native deadline
callback.

### Proofs and proof cost

The checker was built as a standalone executable from the pinned CaDiCaL
distribution's `drat-trim.c`; it is not linked to any measured solver. PicoSAT
965 emits a `%RUPD32` wrapper. The tracked normalizer validates that wrapper
and extracts its DRAT payload before checking.

| Formula/backend | Raw proof | Normalized proof | Checker |
| --- | ---: | ---: | --- |
| Complementary units/CaDiCaL | 5 bytes | 5 bytes | `VERIFIED` |
| Complementary units/PicoSAT | 259-byte wrapper | 2 bytes | `VERIFIED` |
| Pigeonhole 6→5/CaDiCaL | 1,258 bytes | 1,258 bytes | `VERIFIED` |
| Pigeonhole 6→5/PicoSAT | 3,375 bytes | 3,118 bytes | `VERIFIED` |
| Pigeonhole 8→7/CaDiCaL | 117,585 bytes | 117,585 bytes | `VERIFIED` |
| Pigeonhole 8→7/PicoSAT | 231,883 bytes | 231,626 bytes | `VERIFIED` |

On pigeonhole 8→7, 20 randomized repetitions measured proof mode against the
same backend without proof:

| Backend | Median solve without proof | With proof | Ratio | Median RSS change |
| --- | ---: | ---: | ---: | ---: |
| CaDiCaL | 35.486 ms | 36.898 ms | 1.0398 | 0 KiB |
| PicoSAT | 33.330 ms | 36.485 ms | 1.0947 | +768 KiB |

MiniSat and the internal DPLL cannot produce proofs. That is a capability
failure for direct proof-producing adoption, not a correctness failure in the
status-only benchmark.

### Build, package, and license evidence

All external candidates are MIT licensed. Their exact license files hash to
the tracked verbatim notices. No solver code or binary is incorporated into
the product by this experiment.

| Backend | Stripped dynamic Linux | Static Linux | Static Windows-GNU |
| --- | ---: | ---: | ---: |
| Internal DPLL adapter | 355,312 bytes | not measured | product path only |
| CaDiCaL | 1,548,864 bytes | 3,441,616 bytes | 4,342,733 bytes |
| MiniSat | 84,288 bytes | 2,088,192 bytes | 2,746,655 bytes |
| PicoSAT | 121,144 bytes | 2,017,936 bytes | 2,737,730 bytes |

The static Linux binaries have no dynamic dependencies and all three passed
the exact semantic suite. The Windows artifacts are PE32+ x86-64 console
executables; repository policy permits cross-compilation but not execution.
Their SHA-256 values are:

- CaDiCaL:
  `c79d9eb4cd935fae311e436dfc22564a6d3dbc9f37c09d8523d14532833727ac`
- MiniSat:
  `33adbe33d0d2a65fa9a71fe08c945d54bb398c083bd63c2d9a4ddf0990610e6e`
- PicoSAT:
  `a22e4e08c25739f4922c73d152ce980775c1f6896d26d44c81c50c65a4434040`

CaDiCaL cross-compilation disables unlocked stdio and bypasses configure's
target-execution probe while compiling the public C API library directly.
PicoSAT disables Unix `getrusage`. MiniSat needs `-fpermissive`, MinGW zlib
headers/libraries, and a temporary correction to the fallback
`memUsedPeak(bool)` definition in its isolated build copy. These maintenance
costs contribute to the CaDiCaL choice.

### Reproduction and retained evidence

Python generators, capture preparation, benchmarking, validation, dispatch
selection, proof normalization, cross-build, and metadata scripts are all
tracked in this directory. Representative commands on the required Ubuntu
24.04 runner are:

```text
python3 generate_workloads.py WORKLOAD_ROOT
python3 benchmark.py --backend NAME=EXECUTABLE --sessions WORKLOAD_ROOT ...
python3 validate_results.py WORKLOAD_ROOT RESULTS...
python3 validate_large_cores.py WORKLOAD_ROOT RESULTS \
  --checker cadical=... --checker picosat=...
python3 analyze.py RESULTS --output SUMMARY
python3 select_dispatch.py RESULTS --output POLICY
bash cross_build_windows.sh
```

The ignored raw archive
`.artifacts/experiments/2026-07-28-012-incremental-sat-service/results.tar.gz`
contains the result JSONL/JSON, prepared workloads/manifests, proof traces, and
checker logs. It is 7,034,071 bytes with SHA-256
`85356e073a26234f51e07898019d0a9a7685066eff21dd9350d621ede3158375`.

Pinned source archives used on the runner were:

- CaDiCaL 3.0.1:
  `ad639a302b7c4cb4a24f37b7cd0cf7533674e6069c20a561505bccef1c2b4444`
- MiniSat `37dc6c6`:
  `6745034de8380bfce917a8722669d0f93b2ef4e320ab8c49dbed8856a1ecc9cd`

### Limits and follow-up

This experiment does not demonstrate broad AVATAR traffic, Windows runtime
behavior, production Rust FFI safety, proof checking for every future proof
scope, or a default-on end-to-end prover gain. The capture-positive threshold
sample is deliberately reported as narrow.

The follow-up should implement the service boundary with CaDiCaL as an
optional backend, preserve the internal fallback and a complete disable path,
validate assumption-proof finalization, and run a new family-held-out
SATCheck/AVATAR matrix before enabling any automatic dispatch by default.
