# PStack Derivation Memory

Date: 2026-07-10

## Question

Why do `COL042-8.p` and `LUSK6ext.lop` exhaust the Windows candidate's 2 GiB limit even though the C reference proves both, and can the Rust representation recover those proofs without narrowing the search?

## Setup

- Baseline: commit `ce33b7e9`, Windows release Rust.
- C reference: upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` under WSL Ubuntu 24.04.
- Shared comparison options:

```powershell
--auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1
```

- Release build: `cargo build --locked --release --bin eprover`.
- Peak private memory was sampled every 500 ms with `System.Diagnostics.Process`.
- Raw outputs and memory samples are under `.artifacts/experiments/2026-07-10-006-pstack-derivation-memory/`.

## Finding

C `PStack` always reserves 128 `IntOrP` slots, where every slot is pointer-sized. Rust preserved the element count in a typed `Vec<T>`. A clause derivation entry is substantially wider than `IntOrP`, so each generated clause reserved about 4 KiB for a derivation that usually contains only two or three entries. At the `COL042-8.p` proof point, 383,712 queued clauses made this representation multiplier the dominant memory consumer.

The retained `PStack` keeps C's logical capacity and doubling boundary, but caps the initial Rust vector capacity to the number of typed entries that fit in C's byte allocation. `Vec` still grows safely before the logical C boundary if a wide typed stack actually needs more entries.

## Results

| Build | `COL042-8.p` outcome | Peak private memory |
| --- | --- | ---: |
| Baseline, 2 GiB | allocation failure | at limit |
| Baseline, 8 GiB | Unsatisfiable | 2,364.0 MiB |
| Compact term fields only | Unsatisfiable at 8 GiB; still fails at 2 GiB | 2,278.9 MiB |
| Duplicate-term reuse pool | Unsatisfiable at 8 GiB; still fails at 2 GiB | 2,276.0 MiB |
| Retained byte-sized `PStack` | Unsatisfiable at 2 GiB | 1,147.8 MiB |

The exact proof-object command now returns `Unsatisfiable` for both former fast-reference failures, and the full comparison converts `SWV851-1.p` from an allocation abort to C's exact normalized `ResourceOut` result:

| Case | Baseline Rust | Retained Rust | C reference |
| --- | --- | --- | --- |
| `COL042-8.p` | allocation failure after 34.7 s | Unsatisfiable after 39.4 s | Unsatisfiable after 0.34 s |
| `LUSK6ext.lop` | allocation failure after 32.4 s | Unsatisfiable after 28.8 s | Unsatisfiable after 2.92 s |
| `SWV851-1.p` | allocation failure after 39.2 s | ResourceOut after 60 s | ResourceOut after 60 s |

## Falsification Checks

- Compacting term fields alone saved only about 85 MiB and did not restore the 2 GiB run, so that broader representation change was removed.
- An arity-keyed duplicate-term reuse pool did not reduce peak memory and slowed the proof; it was removed.
- Doubling the retained typed initial capacity from the C-byte equivalent did not recover the prior allocator-sensitive `LUSK6.lop` trace and used more memory; the byte-equivalent capacity was restored.
- Focused `PStack` and term tests pass, including a regression that checks a four-word Rust entry receives one quarter of C's logical initial element capacity.
- The exact `COL042-8.p` and `LUSK6ext.lop` proof-object commands emit complete refutations under 2 GiB.
- The fresh 50-case WSL comparison reports 29 mismatches, down from 31: `COL042-8.p` and `SWV851-1.p` are exact normalized matches, while `LUSK6ext.lop` now matches exit/status/shape and differs only in proof text. The report is `.artifacts/e-compare/20260711-002549-989300/`.

## Limits

- At this experiment's retained byte-equivalent allocation, `BOO020-1.p` reached about 46.7 seconds before the 2 GiB allocation failure instead of 24.3 seconds. The follow-up `2026-07-11-001-boo020-memory` experiment uses C's six-entry `PSTACK_AVG_MEM` occupancy for derivation stacks and reaches C's 60-second `ResourceOut` result at a 1,889.8 MiB peak.
- `LUSK6.lop` remains allocator-sensitive because C and Rust term stores use object identity in hashes and tie breaks. The retained layout proves the problem but shifts the run from the prior 4,897/122,867 processed/generated trace to 5,305/129,610 and a roughly 6.9-7.1 second warm runtime. C takes about 1.1 seconds.
- This fixes a major semantic memory-limit mismatch; it is not final memory or performance parity.

## Conclusion

Rust was interpreting C's untyped 128-slot allocation as 128 wide enum values. Matching allocation bytes instead of typed element count halves the measured proof-search peak and restores two reference proofs without changing the calculus. The remaining allocator-sensitive ordering and performance gaps require stable semantic handles or a purpose-built proof/term arena rather than larger eager stacks.
