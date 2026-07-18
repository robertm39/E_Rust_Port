# RegMem typed scratch ownership

## Status

Completed for Bead `E_Rust_Port-j76.2.27`. The sole production C `RegMem`
allocation owner now has an explicit typed Rust ownership boundary, and the
fresh allocation previously performed for every collect-style frequency vector
has been removed. The vendored C checkout remains unchanged.

## Owner audit

A complete source search found one production allocation call outside
`clb_regmem` itself: `FVCollectFreqVectorCompute` keeps a static `long*` and
grows it through `RegMemProvide`. It preserves old entries, zeroes the new tail,
never shrinks, and clears only the feature slots touched by each clause before
the next call. `eprover.c` calls `RegMemCleanUp` only in its memory-debug build.
No production C caller directly uses `RegMemAlloc`, `RegMemRealloc`, or
`RegMemFree`.

Rust had ported the general API to opaque handles and initialized byte buffers,
but its actual frequency-vector owner allocated and zeroed a fresh `Vec<i64>`
for every clause. It now keeps a thread-local typed scratch buffer, preserving
C's power-of-two growth, no-shrink, prefix, and zero-tail rules. Thread-local
ownership avoids C's process-global data race while retaining the same
single-threaded prover allocation policy. Focused regressions cover growth,
same-buffer reuse, prefix retention, zeroed growth, repeated clause computation,
and recoverable size overflow without registry mutation.

Rust continues to initialize its general byte-buffer API deliberately. Exposing
C's uninitialized allocation contents would require unsafe reads and has no
production compatibility consumer. Likewise, the panic-shaped public free and
reallocation wrappers keep invalid handles as programming errors; the
`try_regmem_*` adapters preserve recoverable ownership integration without
importing C's invalid-free/reallocation undefined behavior.

## Exact executable comparison

[`compare_feature_scratch.py`](compare_feature_scratch.py) forces BillPlus
feature-vector indexing on the retained
[`inputs/feature-scratch.p`](inputs/feature-scratch.p) fixture. The pinned Linux
C reference and native Windows Rust executable have exactly equal stdout,
stderr, and exit code in [`comparison.json`](comparison.json): both derive the
same five printed clauses, report `Unsatisfiable`, write no stderr, and exit
zero. No path, time, or other output normalization is applied.

## Allocation-policy benchmark

[`run_benchmark.py`](run_benchmark.py) compiles the standalone optimized
[`scratch_bench.rs`](scratch_bench.rs) model and alternates fresh and retained
scratch ownership for seven rounds of 400,000 calls. Each call touches the same
12 slots over the same eight requested lengths. Deterministic checksums are
exact.

The retained [`reference.json`](reference.json) records:

- fresh allocation: 400,000 allocations per round, 83.384 ms median;
- typed retained scratch: three power-of-two growth events per round, 7.509 ms
  median; and
- an 11.105x median speedup for the isolated allocation policy.

This microbenchmark deliberately isolates the ownership difference; it is not a
claim that whole-prover runtime improves by that ratio. The exact BillPlus
executable comparison separately guards proof behavior.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-111-regmem-typed-scratch\run_benchmark.py `
  --output target\regmem-typed-scratch-check.json

cargo build --locked --release --bin eprover `
  --target-dir target\default-reference

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-111-regmem-typed-scratch\compare_feature_scratch.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\default-reference\release\eprover.exe `
  --output target\regmem-feature-scratch-check.json `
  --expected experiments\2026-07-18-111-regmem-typed-scratch\comparison.json
```
