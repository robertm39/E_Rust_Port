# Shared-Term Weight Cache Experiment

Date: 2026-07-10

## Question

Does matching C's cached `TermWeight` and `TermStandardWeight` macros remove a material proof-search cost without changing the `LUSK6.lop` search?

## Setup

- Baseline: commit `d436cc24` on Windows release Rust, median 11.11 seconds from the preceding LUSK6 experiment.
- C reference: cached upstream build under WSL Ubuntu 24.04, 1.08 seconds.
- Input: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Shared arguments:

```powershell
--auto --silent --print-statistics --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new
```

- First candidate: use C's cached standard `weight` for shared terms, retaining recursive computation for unshared terms.
- Final candidate: also use shared-term `v_count` and `f_count` for arbitrary variable/function weights at all production `TermWeight` call sites.
- Raw outputs: `.artifacts/experiments/2026-07-10-003-shared-term-weight-cache/`.

## Results

| Build | Trial 1 | Trial 2 | Trial 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Rust baseline | - | - | - | 11.11 |
| Cached standard weight | 9.21 | 7.69 | 7.93 | 7.93 |
| Cached standard and parameterized weights | 8.74 | 7.44 | 7.38 | 7.44 |
| C reference | - | - | - | 1.08 |

The final Rust median is about 33% lower than the retained Rust baseline. It remains about 6.9 times the C reference, so this is a material improvement rather than final performance parity.

## Compatibility Findings

- C's `TermStandardWeight` macro reads `term->weight` for shared terms and recursively computes only unshared terms.
- C's `TermWeight` macro computes `v_count * vweight + f_count * fweight` for shared terms and recursively computes only unshared terms.
- C's recursive `TermWeightCompute` counts a normalized pattern applied free variable as one variable. Rust previously counted that application spine structurally; the port now uses the bank-classified pattern property to match C's result.
- Debug builds recursively verify cached shared-term counts and weight. This exposed manually constructed test terms that claimed to be shared without valid bank metadata; those fixtures now satisfy the real term-bank invariant or are explicitly unshared when testing artificial weights.

## Falsification Checks

- Every measured run retained `SZS status Unsatisfiable`, 4,897 processed clauses, 122,867 generated clauses, 259 backward rewrites, and 122,867 paramodulations.
- The observed 92,833/92,847 non-redundant-clause allocation-layout split remained within the previously documented variants.
- Production weight call sites were audited against C macro call sites; the recursive primitive remains available for genuinely unshared terms and invariant checks.

## Conclusion

Rust was recursively traversing shared terms where C relies on immutable term-bank metadata. Restoring the C macro split removes repeated proof-search traversal and also corrects higher-order pattern-applied-variable weighting.

## Limits

- Windows Rust and WSL C timings are not a same-OS final performance certification.
- C hides the shared metadata invariant in macros and lets nominal recursive weighting invoke pattern normalization. The port preserves the result, but a typed shared-term constructor and a side-effect-free pattern classification API would make the invariant clearer later.
- The remaining C/Rust performance gap requires additional measured work in rewriting, generation, and shared-term representation.
