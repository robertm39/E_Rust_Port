# Experiment 270: Borrow the active PD-tree cursor frame

## Status

Accepted for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the first-order PD-tree matching cursor avoid repeated checked indexing of
its active traversal frame without changing its representation, traversal
order, or helper boundaries?

## Change

`search_next_matching_occurrence_impl` now exposes the `RefMut` cursor as a
plain mutable reference and obtains the current traversal frame through
`frames.last_mut()` once per loop iteration. Node, terminal, step,
effective-weight, and variable-link state are read or updated through that
borrow. Non-lexical lifetimes release it before frame pushes, frame pops, and
whole-cursor query helpers.

The change retains:

- safe arena indices and checked `last_mut()` access;
- the existing `PdtTraversalFrame` and cursor layouts;
- symbol/variable traversal order and terminal ordering;
- query, binding, and variable-child representations;
- the existing first-order/higher-order specialization count;
- the measured helper inlining boundaries.

No unsafe code or new allocation is introduced.

## Correctness

- All 41 focused PD-tree tests pass with default features and all features.
- The existing cursor tests cover ordered yielding, live substitutions,
  repeated-variable rejection and backtracking, traversal-order selection,
  terminal payloads, type rejection, node constraints, and deletion updates.
- Direct WSL Callgrind and native Windows runs prove the same LUSK6 theorem and
  exit zero.
- The accepted and candidate Windows executables emit byte-identical standard
  output and standard error.

## Deterministic measurement

The accepted Experiment 267 baseline retires 9,024,090,576 instructions.
Experiment 269 attributed 1,581,288,798 exclusive instructions to the
first-order cursor.

The candidate preserves the exact proof at 8,992,812,925 instructions:

- global delta: -31,277,651;
- global improvement: 0.346602%;
- first-order cursor: 1,560,083,792 instructions;
- cursor delta: -21,205,006, or -1.341%;
- C reference: 5,254,361,329 instructions;
- new Rust/C ratio: 1.711495.

The raw profile is:

```text
.artifacts/experiments/2026-07-23-032-borrow-active-pdt-frame/rust-callgrind-borrow-active-pdt-frame.out
```

## Native production measurement

The default-feature Windows candidate is 8,952,320 bytes, 14,848 bytes larger
than the 8,937,472-byte accepted executable. Four alternating warmup pairs
were excluded. Because the first 64-pair block had positive full-sample means
but nearly flat last-half means, a second independent 64-pair block was run.
All 256 measured processes proved and exited zero.

Both independent blocks improve wall and process-CPU means:

| Block | Wall improvement | CPU improvement |
| --- | ---: | ---: |
| First 64 pairs | 0.794364% | 0.381913% |
| Second 64 pairs | 0.648028% | 0.719770% |

Across the combined 128 pairs:

- wall means improve 0.722713%, from 1.612898 to 1.601242 seconds;
- CPU means improve 0.546960%, from 1.562256 to 1.553711 seconds;
- wall and CPU medians improve 1.803210% and 1.000000%;
- mean paired wall and CPU improvements are 0.572179% and 0.386188%;
- median paired wall and CPU improvements are 1.168943% and 0.975633%;
- the candidate wins 77 wall and 66 CPU pairs, with 11 CPU ties.

The independent final 64 pairs remain positive:

- wall and CPU means improve 0.648028% and 0.719770%;
- wall and CPU medians improve 1.569537% and 2.051282%;
- mean paired wall and CPU improvements are 0.574655% and 0.615984%;
- the candidate wins 39 wall and 33 CPU pairs, with seven CPU ties.

The final 32 pairs also remain positive:

- wall and CPU means improve 0.521954% and 1.707240%;
- wall and CPU medians improve 2.461726% and 5.000000%;
- mean paired wall and CPU improvements are 0.471217% and 1.606784%;
- the candidate wins 20 wall and 19 CPU pairs, with four CPU ties.

Raw warmup and all 128 measured pairs are in `native-warmup.csv` and
`native-lusk.csv`.

## Compatibility and validation

- Maintained report
  `.artifacts/e-compare/20260723-222106-325857` completes all 50 cases with
  zero unexpected mismatches and only the declared `sledgehammer`
  normalized-output difference.
- The report covers first-order and higher-order proofs, protocol modes, the
  one-second LUSK6 case, HEN, GEO, and the BOO/SWV resource boundaries.
- The archived FOL and HO references were rebuilt from exact upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0` in the isolated WSL cache after
  the local manifest was found missing.
- The full serial all-target/all-feature suite passes 4,393 library tests plus
  every integration and binary target.
- Strict default-library, all-feature-library, and all-target/all-feature
  pedantic Clippy pass with incremental compilation disabled.
- The locked all-target/all-feature release build passes.
- Formatting and `git diff --check` pass.
- C-source coverage, Change Later wording, Markdown links, and
  regeneration-preservation checks pass.
- The original `eprover/` checkout remains clean.

This representation-neutral cursor optimization exposes no new C behavior or
post-compatibility design issue, so no C-source documentation change is
needed.

## Decision and limits

Accept. One safe active-frame borrow improves the intended cursor, exact
whole-program instructions, two independent native timing blocks, and the
stable native tail while preserving the full compatibility matrix.

The accepted baseline becomes 8,992,812,925 instructions, or 1.711495 times
C. The port is still not at performance parity: the exact workload retires
71.1495% more instructions than C, and HEN remains materially slower. Keep
Bead `E_Rust_Port-j76.5.3` open.

## Reproduction

```bash
cargo build --locked --release --bin eprover \
  --target-dir target-wsl-callgrind
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-borrow-active-pdt-frame.out \
  target-wsl-callgrind/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-267-insert-repl-fo\release\eprover.exe `
  -CandidateExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-032-borrow-active-pdt-frame\native-lusk-run.csv
```

Run the native command twice independently. The retained `native-lusk.csv`
concatenates the two blocks with pair numbers 1 through 128.

```bash
python3 tools/e-interop/e_interop.py compare \
  --repo-root . \
  --rust-windows target/native-270-borrow-active-pdt-frame/release/eprover.exe \
  --timeout 60 \
  --memory-limit-mb 2048
```
