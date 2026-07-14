# Indexed Paramodulation Active-Substitution Reuse

## Question

Does Rust's first-order indexed-paramodulation path repeat unification work that C shares across all occurrences of an indexed term pair, and can matching C's substitution lifetime improve `GEO288+1.p` without changing inference results?

## Setup

The C reference was `compute_pos_into_pm_term` and `compute_pos_from_pm_term` in `eprover/CONTROL/cco_paramodulation.c`. The Rust target was `src/clauses/paramodulation.rs`.

The instrumented before/after runs used:

```powershell
cargo build --release --locked --features instrument-perf-ctr --bin eprover
target/release/eprover.exe --auto --output-level=0 --print-statistics --cpu-limit=60 --memory-limit=2048 --processed-clauses-limit=5000 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/TPTP_9.0.0_Problems/GEO/GEO288+1.p
```

The full normal-release run used:

```powershell
cargo build --release --locked --bin eprover
target/release/eprover.exe --auto --output-level=0 --print-statistics --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new eprover/EXAMPLE_PROBLEMS/TPTP_9.0.0_Problems/GEO/GEO288+1.p
```

Native compatibility and performance validation used:

```powershell
.\e-interop.ps1 compare
.\e-interop.ps1 benchmark -Runs 5
```

## Results

1. C creates one substitution/CSU iterator for each indexed term pair and keeps the yielded binding active across every matching clause and position. Rust had first computed a trial MGU for mode selection, backtracked it, then reunified inside the full clause-level constructor for each occurrence.
2. Rust now computes one first-order MGU per indexed term pair, performs ordering and mode selection once, and constructs every matching occurrence while that binding remains active. Both indexed directions use the same active-substitution helpers as the higher-order CSU path.
3. Focused tests cover one indexed source binding reused across repeated target positions, and one target binding reused across multiple indexed source clauses. The reverse-direction test also pins backtracking and C's ordered TPTP metadata-parent semantics.
4. At the 5,000 processed-clause GEO288 bound, all structural and proof counters are identical before and after, including `23,643` generated clauses, `23,201` paramodulants, `2,019,713` non-unit subsumption calls, and `1,083,656` term-top insertions. In one direct before/after pair, `MguTimer` fell from `0.050585` to `0.033682` seconds, `ParamodTimer` from `10.776299` to `9.220360` seconds, and `GenerateTimer` from `10.784368` to `9.227394` seconds.
5. The normal Rust release now proves GEO288 within 60 seconds. Its principal counters match C: `10,215` processed clauses, `128,583` generated clauses, `1,512` contextual simplify-reflect steps, `127,990` paramodulants, `2,694` equality resolutions, and `34,170` rewrite steps. Smaller nonredundant, unification, and insertion-count differences remain consistent with documented address-order variation.
6. The final 50-case differential report has four mismatches: timeout-sensitive `BOO020-1.p`, outcome differences for `HEN011-2.p` and the synthetic CPU-limit `LUSK6.lop` fixture, and normalized stdout for `sledgehammer.p`. GEO288 and `LUSK6ext.lop` now match.
7. The final five-run benchmark reports a `3.274` aggregate Rust/C median wall-time ratio over nine behavior-matching cases. `LUSK6.lop` is `3.239`; `LUSK6ext.lop` is `2.860`. Rust's absolute `LUSK6.lop` median improved from about `3.429` to `3.203` seconds, while `LUSK6ext.lop` stayed near `7.294` seconds. The aggregate ratio worsened because C timing and short cases varied, so performance parity remains incomplete.
8. An intermediate differential run exposed reversed metadata-parent order: MGT generated clauses were typed `plain` instead of C's `negated_conjecture`. Reordering the indexed parent and original selected-clause alias to match C fixed the focused MGT comparison with zero mismatches.

## Raw Artifacts

- Before bound: `.artifacts/experiments/2026-07-13-004-indexed-paramod-substitution/rust-before-5000.txt`
- After bound: `.artifacts/experiments/2026-07-13-004-indexed-paramod-substitution/rust-after-5000.txt`
- Full GEO288 run: `.artifacts/experiments/2026-07-13-004-indexed-paramod-substitution/rust-after-full.txt`
- Focused metadata comparison: `.artifacts/e-compare/20260713-220146-009281/`
- Final differential report: `.artifacts/e-compare/20260713-221524-702622/`
- Final benchmark report: `.artifacts/e-compare/20260713-220250-183282-benchmark/`

## Falsification Checks

- Exact 5,000-bound counters, including term-top insertions, check that substitution reuse did not change the generated inference stream.
- Bidirectional focused tests use multiple positions or clauses under one indexed term pair and assert that every binding is backtracked afterward.
- The reverse-direction test uses a disjoint working clause and asserts `negated_conjecture` output roles, separating construction ownership from ordered metadata parents.
- The full 50-case differential suite checks syntax, proof outcomes, exit behavior, and normalized output beyond GEO288.

## Conclusion And Limits

The repeated first-order unification was real and matching C's active-substitution lifetime closes the GEO288 timeout while preserving bounded proof-search structure. The single before/after timer pair is diagnostic rather than a stable percentage claim because host load varies. `HEN011-2.p`, the synthetic CPU-limit fixture, normalized `sledgehammer.p` output, and the broader roughly 3.3x performance gap remain open.
