# Bounded finite-model prototype for FNT

Bead: `E_Rust_Port-9jt.6.1`

## Question and preregistration

Can a deliberately isolated bounded finite-model worker add sound
CounterSatisfiable/Satisfiable results on function-free FNT problems, while
making every finite-bound failure explicit and emitting complete TPTP
interpretations that an independent semantic checker accepts?

This section is preregistered before corpus results are inspected.

## Scope

The prototype accepts first-order TPTP through Umlaut's existing parser and
clausifier, then consumes the emitted CNF. Its initial supported fragment is:

- untyped `cnf` clauses after clausification;
- variables, constants, and predicate symbols;
- equality and disequality between variables and constants;
- no positive-arity function symbols, interpreted arithmetic, distinct
  objects, or formula features that survive clausification.

Unsupported inputs must produce `SZS status Inappropriate`, never a
satisfiability claim. Exhausting the configured finite bounds must produce
`SZS status GaveUp`, because absence of a model up to a finite bound does not
establish unsatisfiability.

## Encodings

Three configurations are compared:

1. `naive`: one untyped domain of size `k`;
2. `sorted`: infer independent argument-position sorts by joining positions
   that share a clause variable, constant, or equality, and search size `k`
   for every inferred sort; and
3. `sorted-symmetry`: the sorted encoding plus value-precedence constraints
   for constants, fixing the first constant of each sort to element zero and
   restricting the `i`th constant to the first `min(i + 1, k)` elements.

The sorted model is reconstructed over the disjoint union of inferred
domains. Predicate arguments outside their inferred domain are retracted to
the first element of that domain. This makes the emitted untyped
interpretation total and preserves the sorted ground instances.

For every size, the worker writes a fresh propositional CNF containing:

- exactly-one constraints for every constant interpretation;
- one Boolean variable for every predicate-table row;
- guarded instances of every first-order clause for every variable grounding
  and relevant constant-value combination; and
- the selected symmetry constraints.

CaDiCaL is the pinned SAT backend. Fresh solves are intentional for this
prototype; incremental reuse is deferred until the encoding establishes
sound solve-set value.

## Corpora and partitions

The smoke set contains hand-written positive, negative, equality, conjecture,
and unsupported-function cases. The external calibration set uses the
CASC-J13 required FNT samples `NLP042+1` and `SWV017+1` plus the published FNN
and FNQ examples `KRS173+1` and `MGT033+2`. All downloaded inputs are hashed
and retained with source URLs.

If a broader TPTP FNT corpus is available, a deterministic family-level split
is formed before solving: train for fragment/encoding calibration, validation
for selecting at most one configuration, and test for the frozen comparison.
No test result may influence the selected configuration.

At equal per-problem wall limits, compare the worker with Umlaut's unchanged
automatic mode and the bundled pinned Vampire FMB reference. Reference
statuses are performance context, not model validation.

## Validation and measurements

Every emitted success is passed through
`tools/validation/validate_tptp_solution.py` with a shell-free model-check
adapter. The adapter combines the original problem and extracted
interpretation under Vampire's `model_check` directives. A model counts only
if Vampire exhaustively evaluates every problem formula to true and the
adapter emits `SZS status VerifiedGood`. Deliberately corrupted predicate,
constant, domain, and status outputs must be rejected.

Per problem and configuration, report:

- the attempted domain-size vector and final outcome;
- propositional variable and clause counts at every bound;
- encoding time, SAT wall time, conflicts, decisions, and propagations;
- interpretation size and independent validation verdict;
- solves unique against unchanged Umlaut at equal wall resources; and
- sorted/naive and symmetry/no-symmetry size and time ratios.

## Decision rule

The prototype is viable for further production integration only if:

- every emitted success is independently `VerifiedGood`;
- every corrupted model is rejected;
- unsupported syntax and finite-bound exhaustion make no success claim;
- inferred sorting or symmetry reduces median final-bound CNF clauses or SAT
  time without losing any naive solve; and
- the worker contributes at least one independently verified solve not
  reported by unchanged Umlaut at the equal wall limit.

Otherwise the worker remains experimental and the measured coverage or
encoding blocker is recorded as a follow-up rather than being wired into the
default prover.

## Results

### Implementation

The isolated Python worker in `finite_model.py` implements the preregistered
boundary. It uses Umlaut only to parse and clausify the original problem,
rejects any positive-arity function or interpreted term that remains, infers
argument-position sorts, enumerates independent nonempty sort-size vectors by
increasing total cardinality, writes a fresh DIMACS encoding, and invokes
CaDiCaL. It never changes Umlaut's production search or default options.

The model renderer expands inferred sorts into a disjoint untyped domain and
makes each predicate total by retracting an argument from the wrong inferred
sort to that sort's first element. The generated interpretation explicitly
defines the finite domain, pairwise domain-element disequalities, every
constant, and every predicate tuple. `vampire_model_check.py` independently
turns an original conjecture into a negated axiom and asks Vampire to evaluate
the original formulas and extracted interpretation; it does not reuse the
worker's clauses or SAT assignment.

The resumable `run.py` controller preserves a JSON report after every encoded
or solved bound. A controller timeout therefore retains completed bound
telemetry and the size vector whose encoding was in progress. A forced
inventory rerun also overwrites a stale prior report when the new attempt
times out. `fetch_samples.py`, `prepare_corpus.py`, `summarize.py`, and
`adversarial_validation.py` make acquisition, partitioning, aggregation, and
negative validation reproducible.

### Provenance and pinned executables

The experiment ran on Ubuntu 24.04 runner
`e-rust-codex-260728-145514-6af9` at Umlaut commit
`268bdaa93ef70e4b241d61708bdce8f27a69d92c`. The relevant hashes are:

| Artifact | SHA-256 |
| --- | --- |
| Umlaut release executable | `4b1d7c264eabfb5ce4e7867e65e5fdd26e3270697044b335be68809cb13b1972` |
| CaDiCaL 3.0.1 source archive | `ad639a302b7c4cb4a24f37b7cd0cf7533674e6069c20a561505bccef1c2b4444` |
| CaDiCaL 3.0.1 executable | `d753923bd2908f1a798b4b4bfccf427a180d430faad24e094a3e1e5b9da6f0e8` |
| Canonical pinned Vampire 5.0.1 executable | `3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665` |
| `validate_tptp_solution.py` | `4c90eea3faa207af374f6c000276f7d1268e64ecbf13a78800b29abf399733d0` |
| `finite_model.py` | `27630c1bb4887480c427b35b853eef8fb35078d7e2e9323169bd408f5be2ff31` |
| `vampire_model_check.py` | `d841d3f19a8ed65b36e93201e67eb3112e65dd1c12bd1b23047023dd42efb975` |

The external family-held-out corpus is the official CASC-J11
`Problems.tgz` from
`https://tptp.org/CASC/J11/Problems.tgz`: 95,117,986 bytes,
SHA-256
`c3bb4b916303dfe427ddc666b0144bbb0ccf244e72bdae0f0864faceaca34180`.
It contains 100 FNN and 150 FNQ problems from 25 families. The deterministic
whole-family partition, created before any solve, has 158 train problems from
17 families, 30 validation problems from four families, and 62 test problems
from four families.

The current TPTP v9.3.0 online problem pages supplied the four preregistered
calibration examples:

| Problem | Bytes | SHA-256 |
| --- | ---: | --- |
| `NLP042+1` | 5,571 | `5d6ece73d3b3c0242abd38979d1c420a643c73f5b84fd3316924e4db6d1985a7` |
| `SWV017+1` | 6,036 | `ed277ce1f6a052bc3d1f09822edc780975879f013aaa3e4cc93a6ffd269aeca5` |
| `KRS173+1` | 2,734 | `c1eb783059ba4df80b9898a4c90a69199d055298682085a568bec2696a1f6568` |
| `MGT033+2` | 6,707 | `170842a4a780558dd0d6015ac3051b812d8c2b8b7f17545c4a359e5041ab6280` |

### Fragment census

The final 30-second inventory classification, including the slow-case
overlay, is:

| Split | Supported | Positive-arity function | Inventory timeout | Input error |
| --- | ---: | ---: | ---: | ---: |
| Train | 16 | 137 | 1 | 4 |
| Validation | 0 | 28 | 2 | 0 |
| Test | 0 | 61 | 1 | 0 |
| **Total** | **16** | **226** | **4** | **4** |

All 226 controlled rejections are due to a positive-arity function after
clausification. The four input errors are three Umlaut clausification stack
overflows in `HWV` and one unavailable historical `SET007` include. The slow
validation/test cases expand 27-70 MB current TPTP axiom files and do not
finish inventory inside 30 seconds. Thus every supported historical problem
falls in the training families; no held-out problem is silently substituted
or moved across families.

Of the four current samples, only `NLP042+1` reaches the fragment. The other
three are explicitly `Inappropriate` because their clausified forms contain
positive-arity functions.

### Staged encoding results

The 16 supported training problems were run at a five-second SAT budget,
eight-second controller wall budget, maximum per-sort size three, and
2,000,000-ground-instance safety limit:

| Mode | Models | Bounds/resource exhausted | Controller timeout |
| --- | ---: | ---: | ---: |
| `naive` | 0 | 9 | 7 |
| `sorted` | 0 | 13 | 3 |
| `sorted-symmetry` | 0 | 13 | 3 |

The large `HWV` inputs exceed the grounding limit, often by orders of
magnitude; the three smaller `SWV` cases reach a second size and time out
during its encoding. Every completed bound retains its domain vector,
propositional variable/clause counts, ground instances, encoding and SAT
wall time, and CaDiCaL conflicts/decisions/propagations in
`results/prototype-train.jsonl`. No training configuration emitted a model.

The supported official calibration problem has a known four-element model.
All three modes found and independently verified it:

| Mode | Tried domain sizes | Successful size | SAT vars | SAT clauses | Ground instances | Encode s | SAT s |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| `naive` | `[1] [2] [3] [4]` | `[4]` | 760 | 1,287 | 1,508 | 0.008334 | 0.002002 |
| `sorted` | `[1,1] [1,2] [2,1] [1,3] [2,2] [3,1] [1,4]` | `[1,4]` | 202 | 342 | 377 | 0.002215 | 0.001434 |
| `sorted-symmetry` | same vectors | `[1,4]` | 202 | 348 | 377 | 0.002285 | 0.001419 |

Sorting reduces successful-bound variables and clauses to 26.6% of naive
(73.4% fewer) without losing the solve. The symmetry constraints add six
clauses and do not add a solve. Plain `sorted` was therefore frozen as the
candidate before held-out execution. The validation and test result files are
empty because their partitions contain no supported record; no test outcome
influenced the choice.

### Independent validation and fail-closed behavior

The canonical repository-pinned Vampire 5.0.1 binary emitted its explicit
`All formulas evaluated to True!` aggregate for:

- all three `NLP042+1` interpretations;
- the conjecture countermodel fixture;
- the independent-sort/two-element fixture; and
- the named-constant/two-element fixture.

The positive-only gate recorded `verdict: verified` for all six. Four
single-change corruptions were then checked: flipping a predicate row,
coalescing two named constants, deleting a domain element, and changing the
claimed status. Every corruption returned nonzero and `verdict: rejected`.

The controlled negative paths emit no success claim:

| Case | Final SZS status | Report outcome |
| --- | --- | --- |
| positive-arity function | `Inappropriate` | `unsupported` |
| all sizes through two are UNSAT | `GaveUp` | `bounds_exhausted` |
| size-vector enumeration truncated | `ResourceOut` | `resource_out` |

The second row is deliberately not `Unsatisfiable`: exhausting finite sizes
does not prove that an infinite model is absent.

### Equal-resource reference comparison

On the 16 supported training problems at five seconds per system, unchanged
Umlaut reported two `CounterSatisfiable` statuses and 14 `ResourceOut`
statuses. Pinned Vampire's current `--mode casc --intent sat` schedule
reported five `CounterSatisfiable` statuses and 11 `Timeout` statuses. The
prototype reported no model. On the 30-second `NLP042+1` calibration, Umlaut,
Vampire, and all prototype modes reported `CounterSatisfiable`.

The prototype therefore has **zero unique independently verified FNT solves**
against unchanged Umlaut at equal configured resources. Its successful
calibration output is stronger as an artifact—an independently verified
finite interpretation rather than an unchecked saturation—but it is not a
unique solve under the preregistered performance definition.

### Reproduction and retained evidence

Local Python validation passes 13 unit tests. On the required Ubuntu 24.04
runner, Rustfmt and strict all-target/all-feature pedantic Clippy pass, the
native all-target/all-feature suite passes its 4,445 library tests plus all
binary and integration targets, every release binary builds, and the Windows
GNU all-target test executables and release binaries compile without
execution.

The raw manifests, per-bound reports, solver outputs, extracted models,
validation reports, adversarial reports, quality log, and machine-readable
`summary-final.json` are retained in the ignored archive:

`.artifacts/experiments/2026-07-28-011-fnt-finite-model-prototype/results.tar.gz`

The archive is 2,578,445 bytes with SHA-256
`330e4f24818014a5b066329ce3f9e1958714f410c9a789e8b850197f9a0bf0c7`.
The 95 MB corpus archive and third-party binaries remain separately ignored
and are identified by the hashes above.

## Conclusion

The isolated prototype satisfies the soundness and reporting acceptance
criteria: it emits complete independently checked interpretations, rejects
every adversarial corruption, never turns finite-bound exhaustion or
unsupported syntax into a satisfiability claim, and records every attempted
domain vector and SAT cost. Inferred sorting is materially useful on the one
supported current sample.

It fails the preregistered production-integration decision. The initial
function-free boundary covers only 6.4% of the historical FNT corpus, all
supported historical records land in training families, grounding explodes
on those records, and the equal-resource unique solve count is zero. No code
is wired into Umlaut's default prover.

The next model-finding investigation should not optimize this grounding loop
in isolation. It should first add positive-arity function tables, native
many-sorted type preservation, and incremental/domain-aware grounding; those
features address the measured 90.4% function-symbol rejection and the
training resource failures. A new held-out evaluation is required before any
production integration.
