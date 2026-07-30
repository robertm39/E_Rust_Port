# Clean-room base VIRAS QE prototype: findings

Bead: `E_Rust_Port-9jt.5.2`

## Decision

The one-conjunction base VIRAS kernel passes its preregistered milestone and is
credible enough to preserve as the reference candidate for a later Rust
implementation. It is not integrated into production Umlaut.

The result is deliberately narrower than general quantifier elimination. The
prototype eliminates one existential real variable from a nonempty conjunction
of normalized exact LIRA literals. Free real parameters are supported.
Arbitrary Boolean normalization, nested quantifier wrappers, typed TPTP
import/export, conflict-driven search, proof-object publication, and automatic
portfolio use remain outside this experiment.

## Clean-room implementation

`prototype.py` implements the tracked paper recurrences without importing
Umlaut or the independent oracle:

- canonical exact-rational terms and quantifier-free Boolean formulas;
- outer and segment slopes, rational periods, symbolic bounds, right limits,
  core intervals, segment zeros, and discontinuity grids;
- covering open/closed symbolic grid intersection, including the documented
  zero-width extension;
- every no-break and discontinuous literal-candidate case;
- plain, epsilon, corrected infinity, and grid virtual terms;
- V1, V2, and V3 finite grid flattening;
- complete one-conjunction finite virtual substitution;
- stable candidate/flattening derivation records; and
- step, candidate, grid, grid-point, formula-node, and rational-bit limits that
  return `Unknown(ResourceLimit)`.

Unsupported formula shape returns `Unknown(UnsupportedFragment)`. A successful
formula cannot contain the eliminated variable or any epsilon, grid, or
infinity marker.

The implementation used only the tracked `viras_docs/` packet and Python's
standard-library `fractions.Fraction`. It did not inspect, import, execute, or
derive cases from the unlicensed VIRAS source tree.

## Setup and commands

The final run used normal-profile Ubuntu 24.04 runner
`e-rust-codex-260730-213435-9f49` (Linode `101806373`). The uploaded 3,985-file
repository snapshot was 8.3 MiB with SHA-256
`e9465beb7b59444c9a4d001d675447e782b2ae61656a8b8705aac9497112a3b4`.

The separately licensed Z3 source was the same pinned commit/archive as
experiment 005:

- commit `2d48fd119ce5074b880944c2b1c59e537c99cd46`;
- archive SHA-256
  `9b78c0cc9f330dab9f39c132aba39c92fdba2dbc0aac26dd07b3946592dd21d8`;
- `CMAKE_BUILD_TYPE=Release`;
- executable enabled, test executables disabled, shared `libz3` disabled; and
- four parallel Ninja build jobs.

The resulting Ubuntu executable reported `Z3 version 5.0.0 - 64 bit` and had
SHA-256
`27e61d2004b0739d2a101d51d92e92e660657b214ba7fb70db7469e0d94cd6d7`.
An explicit probe confirmed that its `to_int` encoding uses mathematical floor
for `-1/2`.

The reproducible controller was:

```text
bash experiments/2026-07-30-004-base-viras-qe-prototype/run_on_ubuntu.sh
```

It ran the focused tests, executed the seeded experiment twice, required the
two reports to be byte-identical, and archived the reports and build/run logs.
The underlying experiment command was:

```text
python3 experiments/2026-07-30-004-base-viras-qe-prototype/run_experiment.py \
  --z3 /opt/e-rust-port/viras-004/z3-build/z3 \
  --seed 0xB451E2026 \
  --cases 1000 \
  --output /opt/e-rust-port/viras-004/evidence/report-1.json
```

The Linode and firewall were deleted after evidence collection.

## Results

All 21 focused tests passed. They cover the paper's exact-rational, grid,
profile, candidate, epsilon/infinity, V1/V2/V3, motivating-example, and
floor-free LRA vectors, the nonempty-break/zero-segment periodic edge, plus
structural properties and fail-closed outcomes.

The independent grid gate generated 1,000 open/closed intersections. All 9,261
concrete grid points were present in the symbolic covering sets. The candidate
also produced 527 permitted extra covering points, confirming that the test
does not incorrectly demand exact rather than covering intersection.

All 1,000 frozen generated conjunctions agreed in three independent views:

- paper-derived candidate output;
- the separately implemented exact bounded cell oracle; and
- the pinned unbounded Z3 process.

The explicit `[-8,8]` literals in each generated conjunction make the bounded
cell oracle complete for the same unbounded query sent to Z3. Outcomes were 475
SAT and 525 UNSAT, with:

- zero candidate/exact-oracle disagreements;
- zero candidate/Z3 disagreements;
- zero literal-order or duplicate-literal metamorphic disagreements; and
- no `Unknown` or solver protocol result.

The aggregate per-case record SHA-256 is
`f95ef1ac2c3cd56946a652c3256ce9aa018d37db9f65e85bec655ab4ccfb1c5c`.
The largest generated case used 26 candidates, eight grid descriptors, 13
finite grid representatives, and 51 counted kernel steps under the frozen
instrumentation.

The corrected motivating formula retained the paper's four expected candidate
shapes:

- `floor(a) + 1/3`;
- negative infinity;
- `Z`; and
- `Z + epsilon`.

Across negative, integral, and nonintegral `a` values and values of `c` below,
at, and above `2/3`, its output was equivalent to `c <= 2/3`.

## Falsification

All four deliberate corruptions changed a frozen expected truth value and were
rejected:

- mathematical floor replaced by truncation at negative `-1/2`;
- the paper's printed periodic/aperiodic infinity reversal;
- strict epsilon substitution used without the required right-limit
  relaxation; and
- omission of the sole required candidate.

Tiny independent limits for steps, candidates, grids, grid points, formula
nodes, and rational bit length all returned `Unknown(ResourceLimit)` with no
formula. Unsupported Boolean shape returned
`Unknown(UnsupportedFragment)`. These checks demonstrate that incomplete work
is not misreported as false.

The two complete JSON reports were byte-identical. Each has SHA-256
`87b7df78e8bdc1ae91dc2e9c26eeea1794a236097a45cd8eb29dde7854cb6304`.

## Limits

This is strong differential evidence for the declared one-conjunction
fragment, not a formal proof of every paper recurrence or a general arithmetic
decision procedure. The randomized corpus is closed and explicitly bounded;
free parameters are exercised by paper/LRA matrices rather than by the 1,000
external-solver queries.

The derivation record explains candidate origin and grid-flattening rules, but
it is not yet an independently replayable proof object. The prototype is
Python experiment code and does not satisfy production performance, Rust API,
package, TSTP proof, recursive Boolean/quantifier, or typed-adapter gates.

Accordingly, automatic schedules and production arithmetic behavior remain
unchanged. A production follow-up should port this exact boundary to Rust,
connect experiment 023's conservative typed adapter, add checkable
source-to-result derivations, and rerun family-held-out performance and solve
coverage before enabling even an opt-in mode.

## Retained evidence

Ignored raw evidence is retained at:

```text
.artifacts/experiments/2026-07-30-004-base-viras-qe-prototype/
```

The principal artifacts are:

- `report.json`, 1,000-case canonical report, SHA-256
  `87b7df78e8bdc1ae91dc2e9c26eeea1794a236097a45cd8eb29dde7854cb6304`;
- `evidence.tar.gz`, both identical reports and all focused/Z3 logs, SHA-256
  `ccfedbea39638de5be4ff4c4fc26fd5323056bc825cca54807f28c1983a8f2e9`;
  and
- `SHA256SUMS`, the remote report, archive, and Z3 hashes.
