# Production finite-model worker: findings

Bead: `E_Rust_Port-9jt.6.9`

## Question

Does the independently implemented Rust production worker preserve the six
family-held-out finite models established by experiment 003, emit complete
typed function-table interpretations that pass an independent semantic
checker, reject one-change corruptions, and remain complementary rather than
entering automatic dispatch?

## Setup

The test used ephemeral Ubuntu 24.04 runner
`e-rust-codex-260729-042846-89b5`, the pinned CaDiCaL 3.0.1 source at Git
object `c60730422e758ef1cebe7aeddf2dda31c996bf04`, and the canonical pinned
Vampire 5.0.1 binary. Vampire's verified SHA-256 was
`3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665`.

The production binary was built with:

```text
UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-3.0.1 \
  cargo build --locked --release --features cadical-static --bin umlaut
```

Fresh copies of `LCL354+1`, `SWW880+1`, `SWW886+1`, `SWW894+1`,
`SWW918+1`, and `SWW919+1` were fetched through the official TPTP individual
problem interface on 2026-07-29. The response's presentation-only HTML anchor
tags were removed before parsing; the logical TPTP text was otherwise
unchanged. Each problem ran through:

```text
target/release/umlaut \
  --finite-model-search --finite-model-max-size=3 PROBLEM
```

Every success was passed to
`tools/validation/validate_tptp_solution.py`, whose external model command was
the experiment-003 typed wrapper around pinned Vampire `--mode model_check`.
The same release binary then ran ordinary `--auto --cpu-limit=10` on all six
problems. Because finite-model search is not scheduled automatically, this is
the current unchanged-saturation comparison.

The production worker also generated its unary-function, nested-function, and
native-two-sort fixture models. The predecessor's six single-change
adversarial harness mutated one function row, predicate row, constant value,
native domain declaration, SZS status, and domain-element type in those new
production outputs, then submitted every mutation to the same independent
controller and Vampire checker.

## Results

All six held-out problems retained their production finite-model result and
all six interpretations were independently `verified` / `VerifiedGood`.
Ordinary `--auto` reported `ResourceOut` on every problem at ten CPU seconds.

| Problem | Production status | Size | SAT variables | SAT clauses | Ground instances | SAT seconds | Auto |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `LCL354+1` | `Satisfiable` | 3 | 1,264 | 6,906 | 2,140 | 0.0016 | `ResourceOut` |
| `SWW880+1` | `CounterSatisfiable` | 3 | 42,503 | 731,346 | 9,934 | 5.0413 | `ResourceOut` |
| `SWW886+1` | `CounterSatisfiable` | 2 | 18,783 | 199,369 | 966 | 0.1881 | `ResourceOut` |
| `SWW894+1` | `CounterSatisfiable` | 2 | 1,411 | 13,980 | 70 | 0.0047 | `ResourceOut` |
| `SWW918+1` | `CounterSatisfiable` | 2 | 6,619 | 581,173 | 812 | 0.1505 | `ResourceOut` |
| `SWW919+1` | `CounterSatisfiable` | 2 | 10,735 | 173,717 | 1,669 | 0.1561 | `ResourceOut` |

The modest count differences from experiment 003 arise at the direct
clausifier-owner import boundary; selected sizes and model results are
unchanged.

All six one-change corruptions were refused. Function, predicate, constant,
native-domain, and type mutations were `rejected`; the deliberately
proof-status mutation was the controller's fail-closed `coverage_gap`. No
corrupted artifact was verified.

Focused Rust gates passed:

- ten encoding/model tests, covering nested and positive-arity functions,
  native two-sort tables, semantic corruption, bounded infinite-only failure,
  resource exhaustion, injected SAT timeout and backend error, and
  incremental/fresh equivalence;
- three `cadical-static` end-to-end CLI tests for a checked typed model,
  bounded exhaustion, and interpreted-arithmetic rejection;
- the default-package end-to-end disablement test; and
- strict all-target/all-feature Clippy with warnings and pedantic findings
  denied.

The mandatory clean lifecycle then passed on fresh Ubuntu 24.04 runner
`e-rust-codex-260729-051609-fb44`. Its uploaded snapshot contained 3,705
files and had SHA-256
`013b416a62f0388317949181363d9f87358d1635fee0da8dbd83b195631a52fa`.
The lifecycle recorded:

- 4,493 native Rust tests passed across the library, binaries, and
  integration suites, with no failures;
- formatting, strict all-target/all-feature Clippy, release builds, the
  independent validation-controller suite, the Linux release-binary
  inventory, and Windows GNU x64 compile gates passed;
- all 50 main compatibility cases and all 216 tool cases had zero unexpected
  mismatches;
- the ten-case benchmark had zero behavioral mismatches and a 1.071x
  aggregate Rust/C wall-time ratio; and
- Callgrind smoke counts were 9,609,867 Rust instructions and 7,591,801 C
  instructions.

The lifecycle emitted both `SUCCESS` and `VALIDATION_COMPLETE`, downloaded
its evidence, and deleted its Linode and firewall.

## Falsification and limits

The worker independently re-evaluates every decoded model before rendering,
but that checker shares the imported clause representation with the encoder.
Vampire validation against the original problem is therefore the independent
semantic boundary. The adversarial matrix probes the renderer/controller
boundary rather than proving completeness of all possible corruption classes.

This rerun uses the current TPTP v9.3.0 copies, not the archived CASC-J11 tar
bytes retained by experiment 003. It is a fresh regression of the same
problem identities and family split, not a claim that the two distributions
are byte-identical.

The result does not authorize automatic scheduling. Portfolio selection,
budget allocation, and broader held-out comparisons remain separate work.

## Retained evidence

The ignored archive contains the six problem texts, production outputs,
positive-only validation reports, ordinary-auto outputs, fixture outputs, and
all adversarial artifacts:

```text
.artifacts/experiments/2026-07-29-004-fnt-production-worker/evidence.tar.gz
```

It is 214,127 bytes with SHA-256
`d1253b676925a19c013f25a4ba55a7e33eb3facb352f9824711e464c8dd33657`.

The clean lifecycle evidence is retained at:

```text
.artifacts/linode/260729-051609-fb44
```

## Conclusion

The explicit Rust production path preserves all six complementary held-out
models, independently validates every success, rejects every registered
corruption, and remains absent from `--auto`. The production integration gate
passes; automatic portfolio dispatch remains intentionally deferred.
