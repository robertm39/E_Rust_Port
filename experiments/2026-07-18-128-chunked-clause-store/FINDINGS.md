# Rejected chunked clause store

## Question

Can bounded clause chunks eliminate the native Windows allocation aborts in
the standard main-executable matrix without regressing sustained proof-search
memory or throughput?

The fresh 50-case report at
`.artifacts/e-compare/20260719-020657-645711/comparison.json` exposed the exact
`BOO020-1.p` failure with `RUST_BACKTRACE=full`. `ClauseSet::insert` asked the
contiguous `Vec<Option<Clause>>` owner to grow by one element, which attempted
to replace its 192 MiB allocation with a 384 MiB allocation under the 2 GiB
Windows Job Object limit.

## Candidate

The temporary candidate replaced the private sparse vector with 4,096-clause
chunks. Numeric slots still mapped to a fixed chunk and offset, compaction and
sorting rebuilt every internal index, and borrowed, mutable, and owned
iterators preserved set order. The first new chunk required 786,432 bytes
instead of a whole-store geometric reallocation. A focused regression crossed
the first chunk boundary, and all 49 clause-set tests passed.

The exact baseline is commit `493e1ad58429c8364098469d34d32b7ff42511bb`.
Its Windows binary SHA-256 is
`AFA6EC4871FD0DB8DA047E6DE6416BBEB8078587CC3F9F967DBD07B12F323202`;
the temporary candidate SHA-256 is
`2AD1C0334795A51DCD5E231C39DC3D35709E9316B549B62B3EBC1DD800489A16`.

## Results

[`benchmark_windows.ps1`](benchmark_windows.ps1) ran alternating native
Windows baseline/candidate pairs with the standard proof-object, 60-second CPU,
and 2 GiB memory options. [`lusk6.csv`](lusk6.csv) retains five pairs:

| Metric | Baseline median | Candidate median | Change |
| --- | ---: | ---: | ---: |
| Wall time | 3.611538 s | 3.569333 s | -1.17% |
| CPU time | 3.546875 s | 3.531250 s | -0.44% |
| Sampled peak RSS | 247,400 KiB | 246,544 KiB | -856 KiB |

All ten runs proved `LUSK6.lop`. Candidate proof-object byte counts occupied
two already-known allocator-layout variants, so the small timing and RSS
movement does not justify the representation by itself.

The decisive one-pair resource control is retained in
[`boo020.csv`](boo020.csv):

| Metric | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| Wall time to abort | 49.621060 s | 52.821038 s | +3.199978 s |
| CPU time to abort | 48.281250 s | 51.875000 s | +3.593750 s |
| Sampled peak RSS | 1,846,676 KiB | 1,922,316 KiB | +75,640 KiB |
| Failed allocation | 402,653,184 B | 786,432 B | bounded request |

Both processes still terminated with Windows status `0xC0000409`, no SZS
status, and no stdout. Chunking postponed the failure but raised operational
peak RSS by 4.10% (73.87 MiB), moving the resource-bound case farther from C's
reported `ResourceOut` outcome.

## Decision

Reject and revert the chunked store. It fixes the single-allocation shape but
does not fix the live-memory deficit, and the relevant resource case gets
materially worse. This corroborates the earlier boxed-store rejection: a new
container representation must reduce the live clause/evaluation ownership
footprint, not only divide allocation requests. The production source is back
to the exact baseline representation; the vendored C checkout was untouched.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --target-dir target\chunked-clause-store

.\experiments\2026-07-18-128-chunked-clause-store\benchmark_windows.ps1 `
  -BaselineBinary .\target\default-reference\release\eprover.exe `
  -CandidateBinary .\target\chunked-clause-store\release\eprover.exe `
  -Problem .\eprover\EXAMPLE_PROBLEMS\SMOKETEST\LUSK6.lop `
  -OutputCsv .\target\chunked-clause-store-lusk6.csv -Runs 5
```
