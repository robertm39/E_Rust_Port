# Independent arithmetic and QE oracle: findings

Bead: `E_Rust_Port-9jt.5.5`

## Question

Can a reusable test oracle, implemented independently of Umlaut's arithmetic
code, reproduce the tracked VIRAS paper errata, reject deliberately faulty
arithmetic transformations, shrink a failing query, distinguish every
solver-outcome class, and agree with an independent SMT solver on generated
bounded arithmetic and quantifier-elimination cases?

## Setup

The tracked Python oracle uses exact `fractions.Fraction` arithmetic. It
decides bounded one-variable real queries by enumerating every rounding
discontinuity, atom zero, boundary point, and intervening open cell. It
decides bounded integer queries by complete exact enumeration. Resource caps
produce `unknown`, never `unsat`.

The clean-room inputs were the paper-derived documents in `viras_docs/`.
Neither the experiment nor the reusable oracle imports Umlaut or inspects the
unlicensed VIRAS implementation.

The independent process oracle was Z3 source commit
`2d48fd119ce5074b880944c2b1c59e537c99cd46`, archived from the ignored,
clean reference checkout. The source archive was 6,830,870 bytes with
SHA-256
`9b78c0cc9f330dab9f39c132aba39c92fdba2dbc0aac26dd07b3946592dd21d8`.
It was built on ephemeral Ubuntu 24.04 runner
`e-rust-codex-260729-060000-fca3` using GCC 13.3.0 and CMake 3.28.3:

```text
cmake -S /opt/e-rust-port/z3-src -B /opt/e-rust-port/z3-build \
  -DCMAKE_BUILD_TYPE=Release \
  -DZ3_BUILD_EXECUTABLE=ON \
  -DZ3_BUILD_TEST_EXECUTABLES=OFF \
  -DZ3_BUILD_LIBZ3_SHARED=OFF
cmake --build /opt/e-rust-port/z3-build --parallel 4
```

The resulting dynamically linked executable identified itself as
`Z3 version 5.0.0 - 64 bit` and had SHA-256
`ce28f1294998e78e9595b569482afd15affb33cf9cb6055e43af48ce22e00618`.
The focused solver runner's final corrected repository snapshot contained
3,709 files and had
SHA-256
`42dbaafa9fd17cb95dbcc3b7acc80dda4ee6941d1e1bca027c5d2d555ec6ebe2`.

The live-solver unit gate used:

```text
UMLAUT_Z3=/opt/e-rust-port/z3-build/z3 \
  python3 tools/validation/test_arithmetic_qe_oracle.py -v
```

The seeded experiment used:

```text
python3 experiments/2026-07-29-005-arithmetic-qe-oracle/run_experiment.py \
  --z3 /opt/e-rust-port/z3-build/z3 \
  --seed 0x5A172026 \
  --cases 500 \
  --output /opt/e-rust-port/arithmetic-qe-report.json
```

## Results

All 28 focused tests passed with the live external solver. The complete local
validation discovery passed 37 tests, with only the deliberately opt-in
external-Z3 probe skipped locally.

The experiment passed every gate:

- 2,000 generated exact quotient/remainder properties and all documented
  rational-LCM vectors passed.
- Z3 confirmed the required mathematical floor and ceiling behavior at
  negative rationals and the absence of an integer in `(-1, 0)`.
- The classifier independently exercised `sat`, `unsat`, `unknown`,
  `disagreement`, and `error`; none are conflated.
- All four tracked paper defects were detected: the periodic/aperiodic
  infinity-substitution swap, the ceiling-to-floor change in Example 10, the
  negated lower witness, and equality in place of the blocking disequality.
- All three seeded faulty transformations changed a verdict: truncating
  negative floor, replacing ceiling with floor, and weakening a strict
  relation.
- The deterministic shrinker reduced the negative-floor failure from
  complexity 16 to the complexity-4 atom `floor(x) = -1` in ten attempts
  while preserving the disagreement.
- The pure-linear QE golden matrix passed all 25 parameter pairs. The typed
  integer versus real-plus-floor adapter matrix passed all five cases.
- All 600 generated expression metamorphisms and all 200 generated query
  metamorphisms preserved their expected semantics.
- All 500 seeded nested floor/ceiling differential cases agreed with Z3:
  330 were `sat` and 170 were `unsat`, with no `unknown`, `disagreement`, or
  `error`.

The report records Python 3.12.3 on Linux 6.8/glibc 2.39, the exact oracle
and solver hashes, the seed, every matrix row, all generated differential
outcomes, and the independence declaration.

The mandatory clean repository lifecycle then passed on fresh Ubuntu 24.04
runner `e-rust-codex-260729-062415-bd01`. Its 3,710-file worktree snapshot
had SHA-256
`522f81e4e56a425945d3d4405a15577f7423e9ef8b3a8b7ca3f5f70129c33552`.
The lifecycle recorded:

- 4,493 native Rust tests passed across the library, binaries, and
  integration suites;
- formatting, strict all-target/all-feature Clippy, release builds, all 37
  independently discovered validation tests, Linux binary inventory, and
  Windows GNU x64 compile gates passed;
- all 50 main compatibility cases and all 216 tool cases had zero unexpected
  mismatches;
- the ten-case benchmark had zero behavior mismatches and a 1.080x aggregate
  Rust/C wall-time ratio; and
- Callgrind smoke counts were 9,609,881 Rust instructions and 7,591,885 C
  instructions.

It emitted both `SUCCESS` and `VALIDATION_COMPLETE`, downloaded its evidence,
and deleted its Linode and firewall.

## Falsification and limits

The first 500-case run failed rather than being discarded: 83 cases were
classified `error` because the SMT renderer asserted a generated fixed
parameter that was unused in the formula but had not declared it. The
retained failing report exposes Z3's `unknown constant a` diagnostic. A
regression test now requires every fixed parameter to be declared, including
unused ones. The same seed then produced the clean 500/500 agreement above.

The exact oracle is complete only for the represented bounded,
one-quantified-variable fragment and its configured resource cap. It is not
a general decision procedure for nonlinear arithmetic, multiple quantified
variables, or unbounded formulas. Generated agreement with Z3 is strong
differential evidence, not a proof that either implementation is universally
correct.

The SMT encoding relies on Z3's `to_int` having mathematical floor semantics
for reals. Explicit negative-rational probes guard that assumption. The
external process remains a validation-only tool: Z3 is not a Cargo
dependency, is not packaged with Umlaut, and no Z3 source entered the tracked
tree.

The paper errata checks are executable regression witnesses for the four
implementation-relevant mistakes catalogued in `viras_docs/`; they do not
claim to formalize every proof-only typographical slip in the paper.

## Retained evidence

The ignored evidence archive contains the final JSON report, the deliberately
retained first failing report, and both Z3 build logs:

```text
.artifacts/experiments/2026-07-29-005-arithmetic-qe-oracle/evidence.tar.gz
```

It is 16,250 bytes with SHA-256
`cebdc036872638cde01dad6bcd6afdca6c1be23bc381ca45b872ff425c0b2c19`.
The final report itself is 20,705 bytes with SHA-256
`4ea88c1f423ee06d025ed2aabec18b545f2845d478776ebc9c7de2a3df5dae93`.

The clean lifecycle evidence is retained at:

```text
.artifacts/linode/260729-062415-bd01
```

Both runners and their firewalls were deleted after evidence retrieval.

## Conclusion

The repository now has an Umlaut-independent, fail-closed arithmetic and QE
test oracle that reproduces every implementation-relevant tracked erratum,
catches and minimizes seeded faulty transformations, classifies all solver
outcomes explicitly, and agrees with a pinned external Z3 process across the
seeded golden, metamorphic, and 500-case differential suites.
