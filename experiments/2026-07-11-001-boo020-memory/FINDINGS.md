# BOO020 Derivation-Stack Memory

Date: 2026-07-11

## Question

Can the remaining `BOO020-1.p` allocation abort be converted to the C reference's 60-second CPU `ResourceOut` result without changing the logical `PStack` growth contract or perturbing known allocator-sensitive proof traces?

## Setup

- Baseline: commit `f4f3e3e8`, Windows release Rust.
- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/BOO020-1.p`.
- Search options:

```powershell
--auto --silent --print-detailed-statistics --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new
```

- An 8 GiB baseline established the memory needed to reach the reference CPU boundary.
- Peak private memory was sampled every 500 ms with `System.Diagnostics.Process`.
- Raw output and samples are under `.artifacts/experiments/2026-07-11-001-boo020-memory/`.

## Finding

C `PStackAlloc` eagerly reserves 128 pointer-sized entries, but `PSTACK_AVG_MEM`, which contributes to `CLAUSECELL_MEM`, budgets only six entries. Rust's byte-equivalent typed allocation reduced the original representation multiplier, but a derivation entry is still wide enough that every clause eagerly reserved about 1 KiB for a stack that usually contains two or three entries.

The retained implementation starts clause and formula derivation stacks with physical capacity for the six entries assumed by C's aggregate-memory estimate. It preserves the logical 128-entry allocation size and C doubling boundary. Rust's `Vec` grows safely when occupancy exceeds the initial physical capacity.

## Results

| Build | Limit | Outcome | Peak private memory |
| --- | ---: | --- | ---: |
| Byte-equivalent typed `PStack` baseline | 8 GiB | CPU `ResourceOut` | 2,299.6 MiB |
| Six-entry derivation capacity | 2 GiB | CPU `ResourceOut` | 1,889.8 MiB |

The derivation-specific allocation removes about 409.8 MiB, or 17.8 percent of the measured baseline peak. The candidate now reaches the 60-second CPU limit under the normal 2 GiB memory limit instead of aborting in the allocator around 46 seconds.

## Falsification Checks

- The focused `PStack` regression pushes past six entries and confirms that the logical allocation remains 128.
- `LUSK6.lop` retains the allocator-sensitive 5,305 processed / 129,610 generated trace from the byte-equivalent baseline and proves in about 6.9 seconds.
- `LUSK6ext.lop` still proves under 2 GiB with 6,209 processed / 344,148 generated clauses.
- `COL042-8.p` still proves under 2 GiB with 4,207 processed / 410,145 generated clauses.
- The fresh 50-case WSL comparison keeps 29 total mismatches but reduces exit/status/shape mismatches from four to three. `BOO020-1.p` now differs only by one normalized strategy-trace line. The report is `.artifacts/e-compare/20260711-011431-873406/`.

## Limits

- This is a Rust representation optimization guided by C's memory estimate, not a byte-for-byte reproduction of `PStackAlloc` for derivation stacks.
- C's actual eager allocation and its aggregate-memory estimate remain inconsistent. A future C cleanup should benchmark demand-grown or small-buffer derivation storage and make memory accounting reflect actual allocation.
- The standard five-run WSL benchmark was unavailable because Cargo is not installed in the Ubuntu distro; Windows release timings and the full Windows/WSL compatibility comparison were used instead.
- The remaining C/Rust runtime gap and allocator-sensitive identity ordering are not addressed here.

## Conclusion

Using C's own six-entry aggregate estimate as the initial physical capacity for derivation stacks restores `BOO020-1.p` resource-result parity and saves roughly 410 MiB without changing the logical stack contract or the checked proof traces.
