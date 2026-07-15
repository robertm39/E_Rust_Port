# Multi-CSU Equality-Resolution Eligibility And Cost

## Question

Does Rust match C when a higher-order clause contains a flexible-rigid
disequality that could yield multiple CSU branches, including ordered-literal
eligibility, generated-clause counts, unaffected equality-factor order, and
bounded per-fixture performance?

## Setup

- Upstream C reference: E commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, configured with
  `--enable-ho`.
- C reference SHA-256:
  `50a1ce2444c136f737cdc504233b32e7471de33339d9d2fc963d36ff8a02796a`.
- Linux Rust SHA-256:
  `fb429d88760af7ed6a8845cf0b8cf88041842e39cb2427fb6282f60dd8ff5cc2`.
- Fixture: `input.p` in this directory.
- Source references: `eprover/CLAUSES/ccl_eqnresolution.c`,
  `eprover/CONTROL/cco_eqnresolving.c`, and
  `eprover/HEURISTICS/che_to_weightgen.c`.

The single THF clause contains two positive equalities and the negative
flexible-rigid literal `F @ b != e`. Multi-unification enables one imitation
and one projection, but C's ordered literal filter rejects this disequality
before CSU enumeration because `q(q(q(a))) = c` dominates it.

The exact inference options are:

```text
--unif-mode=multi
--pattern-oracle=false
--fixpoint-oracle=false
--func-proj-limit=1
--imit-limit=1
--max-unifiers=4
--max-unif-steps=32
--processed-clauses-limit=1
```

Commands from the repository root:

```powershell
cargo test --lib run_higher_order_ordering_preserves_c_equality_resolution_eligibility
cargo build --locked --release --bin eprover
.\e-interop.ps1 benchmark `
    -Corpus experiments\2026-07-15-002-equality-resolution-multicsu `
    -Runs 3 -TimeoutSeconds 30
wsl -d Ubuntu-24.04 -- bash `
    /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-002-equality-resolution-multicsu/trace.sh
wsl -d Ubuntu-24.04 -- bash `
    /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-002-equality-resolution-multicsu/benchmark.sh 200 7
```

For diagnosis, an isolated debug copy of the unchanged C source was built
under `/home/rober/.cache/e-rust-port/debug-eprover-20260715-002/` and traced
with:

```powershell
wsl -d Ubuntu-24.04 -- gdb -q -batch `
    -x /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-002-equality-resolution-multicsu/trace-c-maximal.gdb `
    /home/rober/.cache/e-rust-port/debug-eprover-20260715-002/PROVER/eprover-ho
```

## Diagnosis

C's live KBO6 control block stores the relevant function weights as
`[app=0, d=1, c=1, e=1, q=1]` and precedence ranks
`[27, 4, 6, 8, 28]`. Its pairwise literal matrix is:

```text
             q3(Fa)=d   q3(a)=c   F(b)!=e
q3(Fa)=d     equal       lesser     incomparable
q3(a)=c      greater     equal      greater
F(b)!=e      incomparable lesser    equal
```

Before the fix, Rust had the same symbol codes and precedence ranks but stored
`q=0`. That made the flexible-rigid disequality maximal, produced the spurious
resolvent `q(q(q(e))) = d`, and changed the first-generation counts from C's
zero equation resolutions/five generated clauses to one/six.

The equality-resolution implementation and maximal-literal algorithm were not
at fault. Production `run_proof_search` passed `false` as the
`higher_order_problem` argument to proof-control ordering initialization even
after THF parsing had set the process problem type to higher-order. C's
`set_maximal_0` returns immediately for higher-order problems; the false Rust
argument instead zeroed the first maximal user-symbol weight. Passing the
parsed problem type restores the C weight-generation gate.

## Results

After the fix, `trace.sh` verifies both executables report:

```text
% Factorizations                       : 2
% Equation resolutions                 : 0
% Generated clauses                    : 5
```

After normalizing only clause identifiers, C and Rust also retain the same two
equality-factor proof records in projection-first/imitation-second order:

```text
thf(c_0_N, plain, (((q @ (q @ (q @ a)))=(c))|((d)!=(c))|((b)!=(e))),inference(ef,[status(thm)],[c_0_N])).
thf(c_0_N, plain, (((q @ (q @ (q @ a)))=(c))|((d)!=(c))|((a)!=(e))),inference(ef,[status(thm)],[c_0_N])).
```

Raw stdout, stderr, normalized factor lines, and statistic-count lines are
retained under
`.artifacts/experiments/2026-07-15-002-equality-resolution-multicsu/trace/`.

Seven alternating native-Linux batches of 200 exact-fixture runs measured:

| Implementation | Median batch wall time |
| --- | ---: |
| C | 1.088590 s |
| Rust | 1.173769 s |

The Rust/C ratio is `1.078x`, below the project's `1.10x` local regression
threshold. Raw timings are retained in
`.artifacts/experiments/2026-07-15-002-equality-resolution-multicsu/alternating-times.tsv`.

The standard three-run custom-corpus benchmark is retained at
`.artifacts/e-compare/20260715-184215-044753-benchmark/`. It reports matching
`GaveUp` behavior but a `2.909x` Rust/C median wall ratio (`0.009278` versus
`0.003190` seconds). These sub-ten-millisecond samples are dominated by
whole-process startup and do not use the explicit branching-CSU options, so
they are a lower-bound warning rather than the focused acceptance signal.

The post-fix 50-case differential report is retained at
`.artifacts/e-compare/20260715-185002-861980/`. It has six mismatches, down
from seven in `.artifacts/e-compare/20260715-174054-041587/` because
`SWV851-1.p` now matches; no new mismatch appeared. The remaining
normalized-output-only cases are `LUSK6ext.lop` and `sledgehammer.p`, while
`BOO020-1.p`, `GEO288+1.p`, `HEN011-2.p`, and the synthetic one-second
`LUSK6.lop` CPU-limit fixture retain their established status/resource
differences.

The corresponding five-run native benchmark is retained at
`.artifacts/e-compare/20260715-190312-848589-benchmark/`. Its nine
behavior-comparable cases have an aggregate `3.296x` Rust/C median wall-time
ratio, slightly better than the preceding `3.359x` report at
`.artifacts/e-compare/20260715-175337-068625-benchmark/` but still above the
required `1.10x` threshold. `LUSK6.lop` measures `2.737x` and
`LUSK6ext.lop` measures `2.663x`; `BOO020-1.p` is excluded because repeated
native outcomes differ. Thus the focused fixture meets its local regression
threshold, but whole-port performance parity remains incomplete.

## Falsification Checks

- Both unification oracles are disabled, so an eligible disequality would pass
  through binding enumeration rather than the single pattern-MGU path.
- The GDB trace records the unchanged C OCB weights, precedence ranks, and all
  nine pairwise literal comparisons at `EqnListMaximalLiterals`.
- The focused Rust test exercises the complete executable path and checks zero
  equation resolutions plus five generated clauses, so a unit-only
  higher-order flag cannot mask the production integration error.
- The processed-clause limit isolates first-generation inference eligibility.
- `trace.sh` independently checks resolution, generation, and factor counts,
  then diffs normalized factor proof records.
- The alternating benchmark reverses C/Rust order on every sample and validates
  the expected resource-limit exit status for every run.
- The full differential suite removes one prior mismatch without adding a new
  output, proof-path, or resource/status mismatch.

## Conclusion And Limits

The multi-CSU equality-resolution coverage is accepted. Rust now matches C's
higher-order KBO6 weight-generation gate, maximal-literal eligibility, zero
resolvent count, total generated-clause count, and unchanged equality-factor
order, with a focused native-Linux ratio within 7.8% of C.

The timing includes process startup, parsing, preprocessing, selection, and all
first-generation inferences; it is not a standalone microbenchmark of
`ComputeEqRes`. Broader whole-port behavior and performance remain governed by
the standard differential and benchmark suites.
