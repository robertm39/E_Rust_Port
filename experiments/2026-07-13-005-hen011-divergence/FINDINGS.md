# HEN011 Search And Throughput Investigation

## Question

Does Rust fail to prove `HEN011-2.p` within the 60-second compatibility limit
because its proof search diverges from C, or because equivalent work has higher
per-operation cost? Which compatibility-preserving changes improve that path?

## Setup

The C reference and Rust executable used the normal auto strategy, deterministic
rewrite/new-clause sorting, a 2 GiB memory limit, and either a processed-clause
bound or a 60/180-second CPU limit. Representative direct runs were:

```powershell
wsl -d Ubuntu-24.04 -- bash -lc './target/release/eprover --auto --output-level=0 --print-statistics --cpu-limit=600 --memory-limit=2048 --processed-clauses-limit=100000 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p'
target/release/eprover.exe --auto --output-level=0 --print-statistics --cpu-limit=180 --memory-limit=2048 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p
```

Instrumented timing and instruction profiling used:

```powershell
cargo build --release --locked --features instrument-perf-ctr --bin eprover
wsl -d Ubuntu-24.04 -- bash -lc 'valgrind --tool=callgrind --callgrind-out-file=.artifacts/experiments/2026-07-13-005-hen011-divergence/callgrind-picked-5000.out ./.artifacts/target-wsl/release/eprover --auto --output-level=0 --print-statistics --cpu-limit=600 --memory-limit=2048 --processed-clauses-limit=5000 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p'
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
```

## Results

1. C proves HEN011 in about 19.6 seconds. The original Rust candidate reached
   the 60-second limit. Extending the Rust limit showed the same final proof
   search, not a different search: Rust and C both process `265,284` clauses,
   generate `1,062,557`, retain `21,605` processed and `366,053`
   unprocessed clauses, archive `2,952`, perform `74,088` backward-rewrite
   attempts, insert `16,109,932` term-top entries, and collect `66` term
   cells.
2. Rust makes `154,158,405` non-unit clause-subsumption calls and
   `103,395,867` recursive calls in the accepted WSL run, versus C's
   `157,039,605` and `103,879,206`. The small difference is consistent with
   documented pointer-order variation and rules out missing FV pruning as the
   cause of the timeout.
3. HCB selection had rebuilt a generation-qualified liveness snapshot before
   every candidate extraction. Detaching the selected clause first and checking
   parent identifiers directly in stable proof-state owners, backed by a
   maintained processed-non-unit identifier-position index, reduced the
   instrumented 150,000-clause selection timer from about 4.33 seconds to
   `0.210536` seconds.
4. The accepted term and matching changes borrow term argument slices in
   recursive structural comparisons and use 32 inline term-pair jobs before
   spilling first-order matching work to a vector. Focused tests cover spill
   order and complete bindings.
5. Direct clause subsumption now treats C release-only precondition assertions
   as Rust debug assertions and reuses only the candidate-literal picked buffer
   through reentrant thread-local scratch. Reusing the substitution as well
   changed allocation order and the LUSK6ext proof; retaining a fresh
   substitution preserves the direct expected proof.
6. With the accepted changes, the full WSL Rust proof takes `49.43` wall
   seconds and `49.80` user seconds. The same candidate takes about `69.41`
   wall seconds on Windows, with the same principal counters, so the configured
   Windows 60-second differential case still reports `ResourceOut`.
7. At 150,000 processed clauses, instrumented Rust spends `16.508237` seconds
   in FV-index work and `16.848700` seconds in set subsumption out of
   `27.42` wall seconds, with `59,209,580` non-unit checks. Forward
   FV-indexed subsumption is therefore the next dominant HEN011 optimization
   target.
8. The final 50-case differential under heavy host load has eight mismatches.
   BOO020 and SWV851 end in a platform resource kill instead of C's reported
   `ResourceOut`; GEO288 and HEN011 reach Rust's CPU limit; the synthetic
   one-second case still times out; and LUSK6, LUSK6ext, and sledgehammer have
   normalized proof-output differences. Direct accepted LUSK6ext runs retain
   the expected clause-76 proof, so the report remains a load/process-layout
   checkpoint rather than evidence of changed principal HEN011 search.
9. The final five-run benchmark reports a `3.517` aggregate Rust/C median
   wall-time ratio across nine behavior-matching cases. BOO020 is excluded for
   differing outcomes; LUSK6 measures `3.093` and LUSK6ext `2.943`, so every
   included case remains above the required `1.10` threshold.

## Raw Artifacts

- C full proof: `.artifacts/experiments/2026-07-13-005-hen011-divergence/c-full.txt`
- Accepted WSL full proof: `.artifacts/experiments/2026-07-13-005-hen011-divergence/rust-picked-only-reuse-full.txt`
- Accepted 100,000 bound: `.artifacts/experiments/2026-07-13-005-hen011-divergence/rust-picked-only-reuse-100000.txt`
- Instrumented 150,000 bound: `.artifacts/experiments/2026-07-13-005-hen011-divergence/rust-picked-only-instrumented-150000.txt`
- Windows extended proof: `.artifacts/experiments/2026-07-13-005-hen011-divergence/rust-windows-extended-180.txt`
- Accepted LUSK6ext proof: `.artifacts/experiments/2026-07-13-005-hen011-divergence/rust-lusk6ext-picked-only-reuse.txt`
- Post-change callgrind profile: `.artifacts/experiments/2026-07-13-005-hen011-divergence/callgrind-picked-5000.out`
- Final 50-case differential: `.artifacts/e-compare/20260714-050033-758184/`
- Full candidate differential: `.artifacts/e-compare/20260714-032806-696095/`
- Final five-run benchmark: `.artifacts/e-compare/20260714-051731-216712-benchmark/`
- Valid HEN011/LUSK6ext targeted differential: `.artifacts/e-compare/20260714-041015-895108/`

## Falsification Checks

- Full C/Rust principal counters distinguish throughput from search divergence.
- Direct LUSK6ext runs check proof-order sensitivity after every scratch variant.
- Substitution-only, picked-only, combined scratch, byte scratch, thread-local
  counters, an `IntMap` FV tree, thin LTO, and query-clone removal were measured
  separately. Only picked-buffer reuse improved HEN011 while preserving the
  expected direct LUSK6ext proof.
- The valid targeted HEN011 fixture includes `Axioms/HEN001-0.ax`; an earlier
  copied fixture without that relative include was discarded as invalid.

## Conclusion And Limits

HEN011's remaining outcome mismatch is a throughput problem over nearly the same
search, not a missing inference or pruning rule. Stable-owner parent checks and
picked-buffer reuse materially improve the path, and the WSL build now proves
within 60 seconds. Windows remains about nine seconds over the configured limit,
and Rust remains much slower than C overall. The next optimization must reduce
the cost of FV-indexed forward subsumption without changing candidate order or
