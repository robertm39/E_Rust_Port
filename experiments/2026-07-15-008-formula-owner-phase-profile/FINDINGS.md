# Formula-owner phase profile

## Question

Which phase accounts for the residual Rust/C time and memory gap on the 20,000-owner formula-drain corpus: parsing, CNF preprocessing, or the post-CNF proof path?

## Setup

All commands were run from the repository root on 2026-07-15 (America/New_York). `benchmark-phases.sh` alternates C and Rust execution order over five runs for each of `--syntax-only`, `--cnf`, and `--auto`. It records wall time, CPU time, peak RSS, exit code, and SZS status. Both binaries use the generated corpus from `2026-07-15-007-formula-set-front-drain`.

```powershell
wsl.exe -d Ubuntu-24.04 -- bash `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-15-008-formula-owner-phase-profile/benchmark-phases.sh `
  /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  /home/rober/.cache/e-rust-port/rust-target/17026b1bfe61aaf223cfaae54947c8d2679c31a0/release/eprover `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-007-formula-set-front-drain/corpus/formula-drain.p `
  /mnt/c/Users/rober/Code/E_Rust_Port/.artifacts/experiments/2026-07-15-008-formula-owner-phase-profile/shared-buffer

.\.venv\Scripts\python.exe `
  experiments\2026-07-15-008-formula-owner-phase-profile\analyze.py `
  baseline=.artifacts\experiments\2026-07-15-008-formula-owner-phase-profile\raw\phase-metrics.csv `
  source_handle=.artifacts\experiments\2026-07-15-008-formula-owner-phase-profile\shared-source\phase-metrics.csv `
  shared_buffer=.artifacts\experiments\2026-07-15-008-formula-owner-phase-profile\shared-buffer\phase-metrics.csv
```

The source-handle-only Linux binary SHA-256 was `ff3146a11595c88db960cc0cb4f32b43371a9da20905ea1debe221027791c147`. The shared-buffer binary SHA-256 was `e17324e15380febc80a8c35496907c1fef8008c7137125ed37c171873c595e00`.

## Results

The initial split localized the time gap to parsing: Rust syntax-only used 1.620 seconds median CPU, while CNF added about 0.170 seconds wall and the post-CNF theorem path added about 0.020 seconds. Rust's syntax-only peak RSS was only 14,544 KiB, while CNF raised it to 106,300 KiB and full auto to 140,276 KiB. Thus time and memory have different dominant phases.

The scanner audit found two related C/Rust allocation differences:

- C token cells use `DStrGetRef(Source(in))`, sharing an immutable source label. Rust copied the label into each token. Sharing the label restores C's ownership shape but was performance-neutral here: Rust syntax-only changed from 1.630 to 1.620 seconds wall.
- Several formula-support probes clone the Rust `Scanner` for speculative parsing. `InputStream` derived `Clone` over a `Vec<u8>`, so every probe copied the entire input file. The 20,001 formula records therefore copied the roughly 650 KiB corpus tens of thousands of times. Storing immutable input bytes in `Arc<[u8]>` makes scanner snapshots share file contents while retaining independent cursor, line, column, and lookahead state.

Five-run phase medians before and after shared input bytes:

| Phase | Baseline Rust wall | Shared-buffer Rust wall | Ratio | Baseline Rust CPU | Shared-buffer Rust CPU | Ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Syntax only | 1.630 s | 0.380 s | 0.233x | 1.620 s | 0.400 s | 0.247x |
| CNF only | 1.800 s | 0.540 s | 0.300x | 1.790 s | 0.580 s | 0.324x |
| Full auto | 1.820 s | 0.580 s | 0.319x | 1.810 s | 0.610 s | 0.337x |

This is a 76.7% syntax wall reduction, 70.0% CNF wall reduction, and 68.1% full-auto wall reduction. All 30 post-change runs had the same exit code and SZS status as C: `Unknown` for syntax/CNF and `Theorem` for auto.

The focused C/Rust harness provides a comparison with the corpus staged on Linux rather than read through DrvFS. Before shared input bytes, `.artifacts/e-compare/20260715-221921-115583-benchmark/benchmark.json` measured Rust at 1.8657 seconds and C at 0.0729 seconds, a `25.587x` wall ratio. After the change, `.artifacts/e-compare/20260715-222314-891259-benchmark/benchmark.json` measured Rust at 0.6270 seconds and C at 0.0756 seconds, reducing the ratio to `8.290x` with identical behavior.

## Falsification checks

The source-label-only build had no material improvement, which falsifies token-label allocation as the main cause. The large improvement appears only after sharing input bytes across speculative scanner clones, and it is concentrated in syntax-only time as predicted.

The cloned-stream regression asserts both storage sharing and cursor independence. Existing scanner filename/position tests ensure shared source labels do not alter diagnostics. The benchmark alternates execution order, uses five runs per phase, and checks all exit codes; the analyzer rejects missing runs, nonzero exits, and negative timing samples.

Peak RSS is effectively unchanged by this fix because only transient speculative clones copied the buffer; they did not coexist in large numbers. The remaining memory gap begins during CNF ownership and term construction, not scanning.

## Conclusion and limits

Shared immutable scanner storage restores C-like reference behavior and eliminates a large accidental input-copy multiplier without changing parser state semantics. It materially narrows the focused formula-owner time gap, but does not complete `E_Rust_Port-j76.1.8`: the staged-corpus benchmark remains `8.290x` and full-auto peak RSS remains roughly 103 MiB above C. CNF allocation/owner storage remains the next profiling target.

## Validation

Focused and full repository gates passed:

```powershell
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings -D clippy::pedantic
cargo test --all-targets --all-features
cargo build --locked --release --bin eprover
```

The full test run passed 4,086 library tests and all three schedule integration tests. The new regressions verify that source labels and input bytes are shared by pointer, cloned cursors advance independently, and existing source-position diagnostics remain unchanged.

The standard 50-case C/Rust comparison report is `.artifacts/e-compare/20260715-223054-766299/comparison.json`. Its six mismatches exactly match the established baseline: `BOO020-1.p`, `LUSK6ext.lop`, `GEO288+1.p`, `HEN011-2.p`, `sledgehammer.p`, and `synthetic/cpu-limit-LUSK6.lop`.

The standard five-run benchmark report is `.artifacts/e-compare/20260715-223055-177439-benchmark/benchmark.json`. Its aggregate Rust/C wall ratio is `3.533x`; the already-known `BOO020-1.p` resource-limit difference is the single excluded behavior mismatch. Most cases in this suite are startup-dominated and do not exercise repeated scanner snapshots, so the focused corpus is the discriminating performance check for this change.
