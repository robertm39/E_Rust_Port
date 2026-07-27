# Multi-CSU Equality-Factor Order And Cost

## Question

Does Rust match C when one higher-order equality-factor candidate yields both
an imitation and a projection unifier, including result-stack order,
proof-documentation order, and bounded per-fixture performance?

## Setup

- Upstream C reference: E commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, configured with
  `--enable-ho`.
- C reference SHA-256:
  `50a1ce2444c136f737cdc504233b32e7471de33339d9d2fc963d36ff8a02796a`.
- Linux Rust SHA-256:
  `2d8ad69fa5c88d53791843338892e93b7b0fa0e1f504dd9a76e5d661171ade08`.
- Fixture: `input.p` in this directory.
- Source references: `eprover/CLAUSES/ccl_factor.c` and
  `eprover/CONTROL/cco_factoring.c`.

The fixture contains one non-Horn clause whose maximal side includes
`F @ a`. Unifying that occurrence with `a` has two enabled CSU branches:
imitation binds `F` to the constant function returning `a`, while projection
binds `F` to the identity. A smaller copied negative literal contains
`F @ b`, so the two factors remain observably distinct.

The trace uses these options:

```text
--unif-mode=multi
--pattern-oracle=false
--fixpoint-oracle=false
--func-proj-limit=1
--imit-limit=1
--max-unifiers=4
--max-unif-steps=32
--output-level=2
--processed-clauses-limit=1
```

Exact build and run commands from the repository root:

```powershell
cargo build --locked --release --bin eprover
.\e-interop.ps1 build-reference
.\e-interop.ps1 benchmark `
    -Corpus experiments\2026-07-15-001-equality-factor-multicsu `
    -Runs 3 -TimeoutSeconds 30
wsl -d Ubuntu-24.04 -- bash `
    /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-001-equality-factor-multicsu/trace.sh
wsl -d Ubuntu-24.04 -- bash `
    /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-001-equality-factor-multicsu/benchmark.sh 200 7
```

The focused Rust regression is:

```powershell
cargo test --lib compute_all_equality_factors_preserves_multi_csu_pop_and_doc_order
```

## Results

After normalizing only clause identifiers, C and Rust emit the same two
proof-creation records in the same order:

```text
thf(c_0_N, plain, (((q @ (q @ (q @ a)))=(c))|((d)!=(c))|((b)!=(e))),inference(ef,[status(thm)],[c_0_N])).
thf(c_0_N, plain, (((q @ (q @ (q @ a)))=(c))|((d)!=(c))|((a)!=(e))),inference(ef,[status(thm)],[c_0_N])).
```

Both executables report exactly two factorizations. The projection-derived
`b != e` factor precedes the imitation-derived `a != e` factor, confirming
that Rust's temporary vector and `pop()` reproduce C's CSU-result `PStack`
drain order. Raw output, stderr, normalized factor lines, and count lines are
retained under
`.artifacts/experiments/2026-07-15-001-equality-factor-multicsu/trace/`.

Seven alternating native-Linux batches of 200 exact-fixture runs measured:

| Implementation | Median batch wall time |
| --- | ---: |
| C | 1.107126 s |
| Rust | 1.155413 s |

The Rust/C ratio is `1.044x`, below the project's `1.10x` local regression
threshold. Raw batch timings are retained at
`.artifacts/experiments/2026-07-15-001-equality-factor-multicsu/alternating-times.tsv`.

The standard three-run custom-corpus benchmark is retained at
`.artifacts/e-compare/20260715-172914-342553-benchmark/`. It reports matching
`GaveUp` outcomes but a `3.688x` Rust/C wall ratio (`0.011636` versus
`0.003155` seconds). Those sub-12-millisecond whole-process samples are
startup dominated and do not use the explicit branching-CSU options, so they
are a whole-executable lower-bound warning rather than the acceptance signal
for the exact inference path.

The required 50-case differential run is retained at
`.artifacts/e-compare/20260715-174054-041587/`. Forty-three cases match. Its
seven mismatches are the established whole-port baseline categories:
resource or exit behavior for `BOO020-1.p`, `GEO288+1.p`, `HEN011-2.p`,
`SWV851-1.p`, and the synthetic one-second `LUSK6.lop` case, plus normalized
proof text for `LUSK6ext.lop` and `sledgehammer.p`. No new factoring-related
mismatch appears.

The required five-run native benchmark is retained at
`.artifacts/e-compare/20260715-175337-068625-benchmark/`. One
behavior-mismatched case is excluded; the nine comparable cases have an
aggregate `3.359x` Rust/C median wall-time ratio. This still fails the
project-wide `1.10x` requirement and confirms that broader port performance
remains incomplete, independently of the focused `1.044x` result above.

## Falsification Checks

- Both unification oracles are disabled, so the two solutions must pass
  through C/Rust binding enumeration rather than the single pattern-MGU path.
- The copied `F @ b` literal distinguishes imitation from projection and
  makes a coincidental duplicate result impossible.
- The processed-clause limit isolates the first selected clause and prevents
  later saturation order from changing the two factor records.
- The focused Rust test checks both rendered clauses, consecutive proof IDs,
  both `ef(44)` records, and the higher-order derivation path.
- `trace.sh` diffs the two normalized proof lines and independently checks the
  reported factorization count from each executable.
- The alternating benchmark reverses C/Rust run order on every sample and
  validates the expected native resource-limit exit status on every run.
- The full differential and five-run benchmark exercise the unchanged wider
  executable and retain the known compatibility and performance gaps without
  introducing a new factoring mismatch.

The complete one-clause outputs also expose a separate equality-resolution
difference: Rust emits `q(q(q(e))) = d` and reports one equation resolution,
while C reports none. That finding is recorded on Bead
`E_Rust_Port-j76.1.4`; it does not affect the matching equality-factor lines.

## Conclusion And Limits

The equality-factor multi-CSU coverage is accepted. Rust matches C's two
generated clauses, reversed stack-pop order, proof-documentation stream, and
factor count, while the exact native-Linux batch is within 4.4% of C.

The batch measures process startup, parsing, preprocessing, one selected
clause, and all enabled first-generation inferences; it is not a standalone
microbenchmark of `ComputeEqualityFactor`. The broader standard run remains
well above whole-executable performance parity, and the newly recorded
equality-resolution discrepancy remains open under its own Bead.
