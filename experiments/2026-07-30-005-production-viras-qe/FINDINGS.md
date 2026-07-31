# Production base VIRAS QE findings

Bead: `E_Rust_Port-9jt.5.11`

## Decision

The clean-room base VIRAS kernel advances into Umlaut as a standalone,
feature-required Rust subsystem and CLI. It does not advance into the primary
prover, proof publisher, or any automatic schedule.

The production boundary passed the frozen kernel/importer records, bounded
Boolean and quantifier evaluation, derivation replay and corruption checks,
all exposed resource limits, family-disjoint analytic evaluation, Linux and
Windows compilation, license/dependency audit, clean default packaging, and
the repository's comprehensive compatibility gates.

This is a useful but narrow result. The opt-in tool decided all 120 analytic
held-out formulas that default `umlaut` gave up on, but accepted only one of
100 untouched CASC-2025 TFI documents because the frozen interface accepts
exactly one pure-arithmetic annotated formula rather than a general TPTP
problem. The standalone path is therefore ready for deliberate arithmetic
use and further proof work, not automatic CASC scheduling.

## Production implementation

The new `src/arithmetic/viras.rs` uses arbitrary-precision exact rationals and
implements:

- the canonical LIRA term, literal, formula, grid, and virtual-term layers;
- exact profile, discontinuity, core, right-limit, and candidate recurrences;
- ordinary, epsilon, positive/negative infinity, and grid substitution;
- V1, V2, and V3 finite grid flattening;
- complete one-conjunction elimination with candidate and grid derivations;
- bounded NNF/DNF recursion for arbitrary Boolean structure;
- exact universal duality and nested quantifier elimination;
- shared step, candidate, grid, grid-point, DNF, formula-node, and rational-bit
  budgets; and
- fresh-kernel replay of every successful conjunction before publication.

The port found and fixed an edge case that the frozen Python generator did not
exercise. With bounds `[-8,8]`, the conjunction
`floor(-3x)-1 = 0` and `-2x-1+floor(-x) >= 0` is satisfiable at `x=-1/2`.
The equality is false at the descending zero-segment core boundary but true
immediately to its right, so the complete equality candidate set needs that
right-side representative. The new regression and the full 1,000-case Rust
differential matrix cover it.

`src/arithmetic/typed_lira.rs` consumes Umlaut's already parsed and typed AST.
It reproduces all 12 frozen accepted and 16 frozen rejected experiment-023
documents and keeps the stable rejection taxonomy. It additionally
truth-table checks every supported TPTP connective. Integer binders become
real binders with exact integrality guards. Oversized numeric literals are
rejected before lowering under the caller's rational-bit budget, including
ground formulas that would otherwise simplify during import.

`umlaut-viras-qe` exposes canonical JSON or transformed TFF. Successful JSON
contains the typed trace, normalized source, result, complete per-branch
candidate/grid derivations, resource counts, and
`"replay_validated":true`. Unknown records contain no result formula. The
primary `umlaut` executable does not call this path.

## Frozen and focused validation

The focused all-feature arithmetic run passed 25 tests. Those tests cover all
experiment-004 rational/profile/grid/candidate/V1/V2/V3/motivating vectors,
the four deliberate calculus mutations, 1,000 exact decision cases, 1,000
grid-covering cases, 441 generated universal checks, nested alternation,
Boolean DNF distribution, the typed adapter corpus, every connective, replay
corruption, and fail-closed limits.

The production 1,000-case matrix retained the corrected implementation's exact
475 SAT and 525 UNSAT split. Every Rust decision agreed with the independently
ported exact cell oracle in the test module. Production code embeds neither
Python, Z3, nor the unlicensed VIRAS implementation.

## Family-disjoint analytic evaluation

The controller generated 20 cases in each of six preregistered families using
seed `0x51A52026`: integer intervals, real floor bands, scaled-floor interval
intersections, universal gaps, Boolean point alternatives, and nested affine
quantifiers. Expected truth values came directly from independent Python
integer and `fractions.Fraction` interval algebra.

All 120 documents imported, eliminated, replayed, and independently evaluated
correctly: 59 true and 61 false, with no Unknown or rejection. Twelve
stratified repeated runs were byte-identical. Maximum aggregate candidates per
document were five and maximum grid-flattening records were one.

Whole-process latency on Ubuntu 24.04 was:

| Cases | Median | p95 | Maximum |
| ---: | ---: | ---: | ---: |
| 120 | 0.987 ms | 1.097 ms | 1.416 ms |

Imported formulas had median 17, p95 20, and maximum 20 canonical nodes. Every
closed result simplified to one Boolean node. The result/import growth ratio
therefore had median 0.0588, p95 0.0769, and maximum 0.0909; this held-out
surface shrank rather than grew.

Under `--auto --cpu-limit=1 --memory-limit=2048`, default `umlaut` returned
`GaveUp` on all 120 documents. The exact correct-solve matrix was:

| Outcome | Count |
| --- | ---: |
| Correct in both | 0 |
| Correct only in opt-in QE | 120 |
| Correct only in default `umlaut` | 0 |
| Correct default solves | 0 |

This demonstrates standalone solve complementarity, not integrated schedule
benefit. No automatic prover arm invoked the arithmetic tool.

## Untouched CASC-2025 TFI coverage

The ignored 100-file TFI corpus was transferred explicitly to the Ubuntu
runner without extracting or rewriting formulas. Results were:

| Outcome | Documents |
| --- | ---: |
| Success | 1 |
| `MALFORMED_INPUT` | 77 |
| `UNSUPPORTED_ROLE` | 17 |
| `UNSUPPORTED_DIALECT` | 4 |
| `UNSUPPORTED_OPERATOR` | 1 |

Median whole-process latency was 0.813 ms, p95 was 3.607 ms, and maximum was
294.120 ms on a very large rejected document. The sole success,
`ARI056_1.p`, is the single conjecture `? [X:$int] : X != 12`; its
replay-validated result evaluated exactly to true. These retained corpus files
contain no TPTP status comments, so the report records zero status-comment
comparisons rather than inventing expected metadata.

The 1% document coverage is an intentional consequence of the experiment-023
boundary. Most TFI problems contain type declarations, multiple formulas,
includes, or mixed interpreted/uninterpreted theories. The standalone CLI
does not silently select one formula or expand a problem into a different
claim.

## Proof, corruption, and resource gates

Every successful branch was regenerated in a fresh kernel and compared for
the complete candidate set, every virtual substitution, result formula, and
grid trace. The external evaluation controller independently evaluated each
closed canonical result. Four publication corruptions were rejected:

- flipping the result changed the exact truth value;
- flipping the transformed TFF broke cross-field agreement;
- clearing the replay flag violated the positive-validation gate; and
- deleting a candidate differed from byte-replayed canonical derivation.

The Rust checker separately rejects candidate deletion and result-formula
corruption by semantic replay, rather than only by record identity.

Setting each CLI limit to zero produced JSON `Unknown(ResourceLimit)`, exit
status 2, and no result formula for steps, candidates, grids, grid points, DNF
branches, formula nodes, and rational bits. The rational-bit case failed
during import at 9 bits over an 8-bit limit, demonstrating that even a ground
constant cannot bypass the budget.

## Dependency, license, and package boundary

The `viras-qe` feature pins `num-bigint` 0.4.8, `num-integer` 0.1.46,
`num-rational` 0.4.2, and `num-traits` 0.2.19 exactly. The lock graph adds only
`autocfg` 1.5.1. All five crates are pure Rust and offered under MIT or
Apache-2.0. Their exact notices are tracked in `licenses/`, attributed in
`THIRD_PARTY_NOTICES.md`, and enforced along with exact versions, checksums,
registry source, transitive edges, feature membership, and feature-required
binary metadata by the package audit.

The final clean-package audit built all 26 default-eligible binaries offline
from the extracted source archive. It omitted `umlaut-viras-qe`; the default
runtime linked only the ordinary Linux loader, `libgcc_s`, `libm`, and `libc`.
The minimal five-member runtime contained no optional crate or backend code.
Exact final archive measurements are recorded below after the final audit.

## Reproduction and retained evidence

The analytic and TFI controller command was:

```text
python3 experiments/2026-07-30-005-production-viras-qe/run_evaluation.py \
  --viras-binary /opt/e-rust-port/source/target/release/umlaut-viras-qe \
  --umlaut-binary /opt/e-rust-port/source/target/release/umlaut \
  --tfi-corpus /opt/e-rust-port/artifacts/viras-005/corpus/TFI \
  --output /opt/e-rust-port/artifacts/viras-005/report.json
```

The canonical report SHA-256 is
`edfe800929d391493135adac55089f8069a0a7b9e0310afa6174c85f0909892f`.
The 100-file TFI transfer archive SHA-256 is
`a624aa3c42f8694942c31b1fca1b697d63b7296d1891d34321e5e6034430c666`.
Raw reports, the transfer archive, package manifests, and comprehensive logs
are retained under:

```text
.artifacts/experiments/2026-07-30-005-production-viras-qe/
```

Controller self-tests and documentation/package checks use:

```text
python -m unittest \
  tools.packaging.test_verify_casc_package \
  experiments/2026-07-30-005-production-viras-qe/test_run_evaluation.py
.\.venv\Scripts\python.exe tools\c_source_docs\check_markdown_links.py
python3 tools/packaging/verify_casc_package.py --output-dir OUTPUT
```

The authoritative comprehensive command was `.\linode-runner.ps1 run`. Its
fresh Ubuntu snapshot contained 3,999 files and had SHA-256
`dc731ac0115cb8187e8452a2264dff7f76a4be0ab6335dea3cd93d5da4f822d4`.
Run `e-rust-codex-260730-232627-5cfc` passed formatting, all-target/all-feature
native tests, strict all-target/all-feature Clippy, the default release build,
42 independent validation-controller tests with one environment-dependent
skip, Windows all-target/all-feature test compilation, Windows default release
compilation, both C reference builds, native smoke and Callgrind runs, and the
complete compatibility suite. The suite covered 50 main cases and 216 tool
cases with zero unexpected mismatches; the 29 main and 16 tool differences
were all predeclared contracts. The ten-case benchmark had zero behavioral
mismatches and an aggregate Rust/C wall-time ratio of 1.0815, below the 1.10
regression threshold. The retained `validation-summary.json` has SHA-256
`337aef2a9f0b6c8a8f2dc7f9a34516701ca075fc60731d427eecae1ae2ea0d7f`.

An earlier comprehensive attempt,
`e-rust-codex-260730-232128-f4d1`, stopped at the executable-inventory
regression because that test counted the new feature-required binary as a
default package binary. The regression was corrected to assert 26
default-eligible binaries plus exactly one `viras-qe`-required binary, and no
result from the stopped attempt was reused as final evidence. Both that runner
and the successful comprehensive runner were deleted with their firewalls.

The final package audit used a second clean Ubuntu snapshot with SHA-256
`46f70c74bbe83f13a552dd993096863f9d2c06e712355471f6abd73b634a0859`.
The audit JSON has SHA-256
`4ada831108463fff251bb183cc99e9461c543b5af33724e04951b5164ddfa489`.
The 325-member source archive is 2,031,375 bytes with SHA-256
`bfa04e717ecd631404f7f0357d7b53f767f0746d45b69f8b8c97773766080a36`;
the five-member StarExec runtime is 2,808,046 bytes with SHA-256
`700d83e62f30eb06982948a237e5193c188400f41adc7c90c78aec7ce98478d1`;
and its default `umlaut` ELF is 8,288,088 bytes with SHA-256
`d4c8f51d5c6209af4905c45dca4196ae8fa52997c5d83f4d1d1ccb3641a52278`.
All 13 package checks passed, including offline extracted-source compilation,
the exact optional dependency graph, default-binary exclusion, minimal runtime
membership, wrapper/include emulation, and signal cleanup. The focused runner
and firewall were deleted after all three artifacts were downloaded.

## Limits and next boundary

The derivation is auditable and replay-validated but is not a TSTP proof rule
accepted by Umlaut's first-order proof publisher. This blocks silent insertion
into a refutation. The one-formula pure-arithmetic gate also excludes 99% of
the untouched TFI sample. Formula-level extraction from mixed problems,
proof-checker support, parameterized output growth, and measured integration
with saturation search require separate Beads and fresh held-out evidence.
That boundary is tracked by `E_Rust_Port-9jt.5.12`.

No source-level conclusion is claimed for the unlicensed VIRAS
implementation. No automatic schedule changed. The adopted result is the
removable opt-in feature and standalone executable only.
