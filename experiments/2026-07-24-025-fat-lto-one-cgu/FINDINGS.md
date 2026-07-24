# Experiment 298: Accept fat LTO with one release codegen unit

## Status

Accepted performance configuration for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the production Rust prover translate its accumulated local instruction
improvements into native throughput by enabling whole-program optimization,
without changing proof search, feature selection, resource outcomes, or
drop-in compatibility?

## Baseline and prior evidence

- Accepted executable source: Experiment 293.
- Exact default-feature LUSK6 Callgrind:
  `8,718,487,029` instructions.
- Original FOL C Callgrind: `5,254,361,329` instructions.
- Exact Rust/C ratio: `1.659286`.
- Accepted native executable: `8,928,256` bytes.

`Cargo.toml` previously had no explicit release profile, so Cargo used 16
codegen units with LTO disabled. Experiment 144 observed fat-LTO results only
on top of a replacement cache that failed the BOO020 memory boundary; its
decision explicitly rejected the confounded combination. The earlier HEN011
investigation's thin-LTO ablation likewise did not isolate the current
accepted source.

## Candidate

The candidate adds:

```toml
[profile.release]
codegen-units = 1
lto = "fat"
```

It was first built without changing the manifest by setting
`CARGO_PROFILE_RELEASE_LTO=fat` and
`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`. Verbose native compiler output
records `-C linker-plugin-lto -C codegen-units=1` for the library and
`-C lto=fat -C codegen-units=1` for the final executable.

The environment-built native and WSL fingerprints both record exactly
`features=["default"]` and profile hash `11264489599640293354`. After the
runtime gates passed, the manifest was changed and a clean build without
environment overrides reproduced the same feature list, profile hash, and
`8,617,472`-byte executable size. Three fresh runs of that manifest-built
binary reproduce the accepted proof hash.

Fat LTO increases a clean native release build from roughly 45 seconds to
roughly 2 minutes 40 seconds on this host. That build-time cost is accepted in
exchange for the measured production runtime improvement.

## Proof identity

Three accepted-parent and eight environment-built candidate runs, followed by
three manifest-built candidate runs, all:

- exit zero;
- emit empty stderr;
- produce the same 378-byte stdout;
- have SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

The native executable shrinks from `8,928,256` to `8,617,472` bytes:

- delta: `-310,784` bytes;
- reduction: `-3.480904%`.

## Deterministic result

Exact default-feature LUSK6 Callgrind falls to `8,400,364,984`
instructions:

- delta: `-318,122,045`;
- improvement: `-3.648822%`;
- Rust/C ratio: `1.598741`.

The profile preserves the expected proof and 4,873 processed clauses.
Whole-program optimization redistributes several formerly standalone owners
into callers, so exact program totals are the authoritative comparison. Major
stable owners also improve: the PD-tree matching cursor falls from
`1,560,083,792` to `1,489,925,474`, substitution normalization falls from
`444,445,091` to `439,641,877`, `term_deref` falls from `165,438,715` to
`150,917,494`, and structural weight comparison falls from `169,568,398` to
`150,666,205` instructions.

Raw profile:

```text
.artifacts/experiments/2026-07-24-025-fat-lto-one-cgu/callgrind-fat-lto-one-cgu.out
```

## Native result

Each independent 64-pair block has four separate alternating warmup pairs.
All 256 measured processes prove and exit zero. Negative percentages mean the
candidate is faster.

| Sample | Wall mean | CPU mean | Paired wall mean | Paired CPU mean |
| --- | ---: | ---: | ---: | ---: |
| Block 1 | -2.866886% | -3.008622% | -2.754274% | -2.916345% |
| Block 2 | -1.553000% | -1.926639% | -1.468782% | -1.806825% |
| Combined 128 | -2.214155% | -2.470274% | -2.111528% | -2.361585% |
| Combined stable halves | -1.844504% | -1.819870% | -1.736880% | -1.684916% |

Central tendency and win counts agree:

| Sample | Wall median | CPU median | Paired wall median | Paired CPU median | Wall wins | CPU wins/ties |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Block 1 | -2.389031% | -2.380952% | -2.172199% | -2.424331% | 58/64 | 46/64, 10 ties |
| Block 2 | -1.495943% | -2.380952% | -1.665185% | -0.588235% | 47/64 | 32/64, 16 ties |
| Combined 128 | -1.982262% | -2.380952% | -1.816682% | -2.339261% | 105/128 | 78/128, 26 ties |
| Combined stable halves | -1.391998% | -1.204819% | -1.420997% | 0.000000% | 52/64 | 30/64, 20 ties |

Tracked evidence is in `native-lusk-block1.csv` and
`native-lusk-block2.csv`; excluded warmups are retained in the ignored
experiment artifact directory.

## Compatibility and resource result

The maintained 50-case comparison report is:

```text
.artifacts/e-compare/20260724-165458-636169
```

It has 50 cases, zero mismatches, and one declared sledgehammer output
difference. In particular:

- HEN011-2 reaches the C proof outcome;
- the synthetic one-second LUSK6 case reaches the C proof outcome;
- BOO020-1 and SWV851-1 preserve C-compatible resource outcomes;
- all standard-input, syntax, formula-printing, pruning, CNF, app-encoding,
  malformed-input, CPU-limit, and memory-limit cases match.

## Repository validation

- The serial all-target/all-feature test suite runs with one Cargo job and one
  test thread; all 4,394 library tests and every executable/integration target
  pass.
- Strict all-target/all-feature pedantic Clippy passes.
- Formatting passes.
- Default-feature and all-feature optimized `eprover` builds pass with the
  manifest profile.
- The C-source documentation coverage, Change Later wording, Markdown-link,
  and regeneration-preservation gates pass.
- The vendored `eprover/` checkout remains clean.

## Decision

Accept fat LTO with one release codegen unit. It is proof-exact, removes
3.648822% of deterministic instructions, improves two independent native
blocks and their stable halves, shrinks the executable, preserves the full
compatibility/resource matrix, and passes every repository gate.

Keep the main parity Bead open: the deterministic Rust/C ratio is improved to
`1.598741`, but the port has not yet reached the required comparable-
performance threshold.

## Reproduction

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-298-manifest-fat-lto-one-cgu
```

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-fat-lto-one-cgu.out \
  target-wsl-298-fat-lto-one-cgu/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-293-fuse-always-deref-app-check\release\eprover.exe `
  -CandidateExe .\target\native-298-fat-lto-one-cgu\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-025-fat-lto-one-cgu\native-lusk-block1.csv
```
