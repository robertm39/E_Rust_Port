# LUSK6 Release Assertion Cost

Date: 2026-07-11

## Question

How much of the `LUSK6.lop` runtime gap comes from Rust executing C assertion-equivalent term-bank validation in release builds, and does removing that mismatch restore the one-second reference proof?

## Setup

- Baseline: commit `6721a796`, Windows release Rust.
- C reference: upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` under WSL Ubuntu 24.04.
- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Shared search options:

```powershell
--auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new
```

- Detailed runs also used `--print-detailed-statistics`.
- The retained Rust result includes an immediate same-session five-run A/B comparison of the baseline and modified release executables.

## Finding

Upstream's normal build sets `NODEBUG = -DNDEBUG -DFAST_EXIT`. C therefore removes `tb_termtop_insert` assertions that validate the top-cell shape, scan every argument for sharing, confirm inferred types, and look the newly inserted term up in the term store.

Rust used ordinary `assert!` for those same invariants. In release mode, every one of roughly 2.69 million top insertions scanned and cloned the argument vector, and each new shared term performed a second splay-tree lookup. The retained implementation uses `debug_assert!` for these C assertion-equivalent checks. Debug builds preserve the validation; release builds match the C configuration.

## Results

| Build | Search result | Runtime |
| --- | --- | ---: |
| C reference | Unsatisfiable | 1.27 s wall, 1.16 s user |
| Rust baseline, paired A/B | Unsatisfiable | 8.053 s median; 7.876-9.244 s range |
| Rust with release-equivalent assertions, paired A/B | Unsatisfiable | 7.037 s median; 6.988-9.265 s range |

Both A/B sequences had an approximately 9.25-second cold first run. Earlier lower-load modified runs produced a 5.918-second median over a 5.684-6.467-second range, so absolute Windows timings vary materially; the paired 12.6 percent median improvement is the defensible comparison.

The primary Rust search trace remains 5,305 processed and 129,610 generated clauses. C reaches the proof after 4,897 processed and 122,867 generated clauses, so both residual per-operation cost and allocator-sensitive search divergence remain.

## Falsification Checks

- A separate term-tree-key ablation avoided first-order type-check work and argument-handle clones during comparisons. Its warmed runs did not improve the retained median, so it was removed.
- The instrumented release run attributes overlapping totals of about 2.15 seconds to generation/paramodulation, 3.04 seconds to forward modification, 2.76 seconds to forward rewriting, and 3.58 seconds to generated-clause insertion.
- The exact one-second candidate still reports `ResourceOut`; the C reference proves within that limit.
- `GEO288+1.p` and `HEN011-2.p` still report `ResourceOut` at 60 seconds, so this throughput change does not reduce the three remaining behavioral mismatches.
- The final 50-case WSL comparison remains at 29 total mismatches: 26 normalized-output-only and three exit/status/shape mismatches. The report is `.artifacts/e-compare/20260711-022721-603001/`.

## Limits

- Windows Performance Recorder could not start the CPU profile because the account lacks the system-performance profiling policy. No usable ETL trace was produced.
- The standard WSL benchmark remains unavailable because Cargo is not installed in the Ubuntu distro.
- Rust's `Rc<RefCell<TermCell>>` representation and splay-tree links still perform substantially more ownership and borrow bookkeeping than C's arena-like raw term pointers. Closing the remaining runtime gap likely requires stable arena handles or another purpose-built owned term store, not more assertion removal.

## Conclusion

Matching C's release assertion configuration improves the paired `LUSK6.lop` median by 12.6 percent without changing the main search trace. It is a valid hot-path parity fix, but the one-second mismatch remains and needs a deeper term/rewriting representation improvement.
