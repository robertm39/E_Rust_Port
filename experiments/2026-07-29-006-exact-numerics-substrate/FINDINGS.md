# Exact integer/rational substrate findings

Bead: `E_Rust_Port-9jt.5.1`

Date: 2026-07-29

## Decision

Dashu is the preferred first production substrate for exact integers and
rationals, but this experiment does **not** add it to Umlaut. Adoption is
deferred until the first production theory module needs the interface in
[`INTERFACE.md`](INTERFACE.md). At that point, pin `dashu-int` and
`dashu-ratio` 0.5.1 behind private Umlaut facade types, rerun the conformance
suite and clean-package gates, and update the dependency notices and lockfile.

This is a deliberately reversible decision:

- no candidate is present in the root `Cargo.toml` or `Cargo.lock`;
- no candidate source or binary is shipped;
- theory code must not name, re-export, or `Deref` to a backend type; and
- canonical construction, comparison, floor/ceiling, hashing, and wire-format
  behavior are fixed at the Umlaut facade rather than inherited accidentally.

Dashu won because it was exact on the independent oracle, is pure Rust under
permissive licenses, and had the lowest median time in the paper, medium, and
large workloads. Rug was 1.8% faster on the small workload, but was slower on
the other three and statically embedded LGPL GMP through an FFI/build-tool
boundary. That small advantage does not justify the packaging and ownership
cost. Mini-GMP and the `num` stack were materially slower.

## Candidate and license matrix

The exact Cargo inputs are locked in [`Cargo.lock`](Cargo.lock). Licenses below
come from Cargo metadata for the measured feature sets and the reviewed GMP
distribution records. This engineering record is not legal advice.

| Candidate | Measured input | License boundary | Result |
| --- | --- | --- | --- |
| Dashu | `dashu-int` and `dashu-ratio` 0.5.1; crate checksums `b6ee98721d5d223e5b64b642dd9588b79d9ef415554b13720308b77d628c3be6` and `727613e0312d442301147d9e45eb2e7c2797230fc6d223a2c51ecf0e98dfeb5e` | Pure Rust. Direct crates and measured transitives are MIT and/or Apache-2.0. | Preferred. Canonical `RBig`; fastest overall; no optional dynamic library in `ldd`. |
| `num` | `num-bigint` 0.4.8 and `num-rational` 0.4.2; checksums `c89e69e7e0f03bea5ef08013795c25018e101932225a656383bd384495ecc367` and `f83d14da390562dca69fc84082e73e548e1ad308d24accdedd2720017cb37824` | Pure Rust. Direct crates and measured transitives are MIT and/or Apache-2.0. | Rejected for this workload: 4.58–11.57x Dashu's median time. |
| Rug/full GMP | `rug` 1.30.0 checksum `07a8857882aec59d27254b02481c709327c13de6fad1da60bfc4f9783eaaa61e`; `gmp-mpfr-sys` 1.7.1 checksum `7db155b537cb791b133341f99f68371d86ee7fa4c79aacfbc376d72d23c70531`; embedded GMP 6.3.0 | `rug` and `gmp-mpfr-sys` declare LGPL-3.0-or-later. The measured ELF statically contains GMP and therefore needs an exact source/relink/notice review before distribution. The build also requires a C toolchain and `m4`. | Not selected. Only 1.8% faster than Dashu on small operands and 1.27–2.31x slower elsewhere. |
| Mini-GMP/Mini-MPQ | GMP 6.3.0 `mini-gmp.c` SHA-256 `378a9731eb6fd69b93fa074172f79be4089c813365613e414f4da4923dcbda52`; `mini-mpq.c` SHA-256 `41faf6f195a30a7aa28ead1a3d9c2fceac5544436edfb66d860b1d6933043c57` | C source under LGPL-3.0-or-later or GPL-2.0-or-later. Vendoring would add source, FFI ownership, notices, and static-link obligations. No source was copied into the product. | Rejected as the default: smallest study binary, but 3.44–7.13x slower than Dashu. Remains a possible constrained C fallback only after a separate packaging review. |
| `ibig` | Screened at 0.3.6 | MIT or Apache-2.0. | Not benchmarked: integer-only, so it does not directly provide the required canonical rational surface. |
| `malachite-q` | Screened at 0.10.0 | LGPL-3.0-only. | Not benchmarked: it adds another copyleft dependency family without an identified advantage over the already measured GMP and pure-Rust choices. |

The measured Cargo graph contained 18 registry packages. Its full
name/version/license/source inventory is in `report.json` within the retained
artifact and in `raw/cargo-metadata.json` inside the evidence archive.

## Exactness and workload

[`run_experiment.py`](run_experiment.py) uses Python
`fractions.Fraction`, not Umlaut or any candidate package, to generate the
expected canonical results. A denominator is always positive, numerator and
denominator are reduced, and zero is `0/1`.

Seed `0x5a172027` generated 2,720 operand pairs:

| Workload | Cases | Operand scale | Timed iterations |
| --- | ---: | --- | ---: |
| Paper | 96 | Constants extracted from all tracked `viras_docs/*.md`, plus boundary constants | 500 |
| Small | 2,048 | 64-bit numerators, 48-bit denominators | 80 |
| Medium | 512 | 384-bit numerators, 256-bit denominators | 12 |
| Large | 64 | 2,048-bit numerators, 1,536-bit denominators | 2 |

For every case the oracle digest covers both inputs, addition, subtraction,
multiplication, division, floor, ceiling, and comparison. All four backends
matched all four expected digests on each of seven executions: 28 successful
backend runs and 685,440 independently expected canonical observations.
Vector SHA-256 is
`66c1a25ce75e85e124a97cfca9dda6ff96e2c99d74f6da4ebeede55df10d8045`.

The fail-closed path was itself falsified by flipping the low bit of the small
oracle digest from `cac1c0c1b585afd7` to `cac1c0c1b585afd6`. The first
`num-rational` run returned the unmodified correct digest and the harness
raised a mismatch instead of producing a passing report. The retained
`num-rational-0.4.2-run-1-mismatch.json` records both values.

## Performance

Each sample times exact add, subtract, multiply, divide, floor, ceiling, and
comparison, after parsing and before decimal serialization. Values are medians
of seven release/LTO samples on a four-vCPU dedicated Ubuntu 24.04 runner with
an AMD EPYC 9845 host. Lower is better.

| Backend | Paper ns/op | Small ns/op | Medium ns/op | Large ns/op | Median max RSS KiB | ELF bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Dashu 0.5.1 | **35.60** | 162.21 | **924.70** | **6,178.00** | 3,200 | 688,112 |
| Rug 1.30.0/full GMP | 82.39 | **159.22** | 1,178.43 | 8,445.74 | 3,840 | 832,296 |
| Mini-GMP 6.3.0 | 253.66 | 557.40 | 4,298.50 | 42,678.04 | **2,432** | **107,288** |
| `num-rational` 0.4.2 | 162.88 | 1,706.34 | 9,547.85 | 71,464.20 | 3,712 | 599,856 |

Relative to Dashu, Rug ranged from 0.982x to 2.314x, Mini-GMP from 3.436x to
7.125x, and the `num` stack from 4.575x to 11.568x. Sample dispersion was
small enough that none of the selection-relevant gaps is explained by a
single outlier; every raw nanosecond and GNU `time -v` sample is retained.

The experiment is a substrate microbenchmark, not a prover-speed claim.
Operands are deterministic rather than a corpus-derived frequency
distribution, allocation behavior inside complete term/index structures is
not measured, and the C adapter uses integer floor/ceiling temporaries while
the Rust adapters materialize rational results. The latter gives Mini-GMP a
small structural advantage, so it cannot explain Mini-GMP's loss. A production
adoption gate must benchmark complete theory-module traffic and CASC problems.

## Reproduction and validation

The final harness snapshot uploaded by `linode-runner.ps1 sync` had SHA-256
`efe57cccbb3b07f2d8f7af96e2adc2a7e72221e1a58fb230daa57809aa67382a`.
The experiment used Ubuntu 24.04, Linux 6.8.0-134, Rust/Cargo 1.97.1, GCC
13.3.0, Python 3.12.3, and GNU `time`. The experiment crate used
`codegen-units = 1` and thin LTO.

Focused gates:

- `python -m unittest test_experiment.py`: 3 passed locally and on Ubuntu;
- `python -m py_compile run_experiment.py test_experiment.py capture_environment.py`;
- `cargo fmt -- --check`: passed;
- `cargo clippy --release --locked --all-targets -- -D warnings -D clippy::pedantic`: passed;
- the experiment C adapter compiled with GCC `-O3 -DNDEBUG -std=c11 -Wall
  -Wextra -Werror`; upstream Mini-GMP compiled separately without treating its
  existing warnings as Umlaut errors; and
- the final 28-run matrix passed, while the one-bit oracle corruption failed.

The final repository-wide lifecycle ran on the exact completed worktree under
run ID `260729-072437-795f`. It passed 4,493 Rust tests, 37 independent Python
validation tests, Rustfmt, strict Clippy, all Linux release binaries, every
Windows GNU x64 test/release compile target, both isolated C references, and
smoke/Callgrind gates. The 50-case main and 216-case support-tool reports had
zero unexpected mismatches. All ten benchmark outcomes matched at an aggregate
1.0731417262x Rust/C wall-time ratio. Callgrind recorded 9,609,867 Rust versus
7,591,801 C instructions. Both `SUCCESS` and `VALIDATION_COMPLETE` are present
under `.artifacts/linode/260729-072437-795f/`; its runner and firewall were
deleted automatically. The local Markdown checker also resolved every link in
269 files.

The ignored evidence files are:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `.artifacts/experiments/2026-07-29-006-exact-numerics-substrate/report.json` | 55,701 | `98eadd2bbc2da4f3778ac8134fe47c5d3fd659f981769265ad92ce8d88fd6842` |
| `.artifacts/experiments/2026-07-29-006-exact-numerics-substrate/evidence.tar.gz` | 511,170 | `12d6f51ceb31cac886527008078678a34e74c2a63a10b6ac5f1016593240e7b4` |

The archive contains all vectors, Cargo metadata, 28 stdout/stderr/time
records, the valid falsification record, and compiler/CPU/linkage metadata.
The ephemeral runner and firewall were deleted after the hashes matched.
