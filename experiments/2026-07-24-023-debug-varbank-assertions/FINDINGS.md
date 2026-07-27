# Experiment 296: Reject release-disabled variable-bank assertions

## Status

Rejected performance candidate for Bead `E_Rust_Port-j76.5.3`; accepted
Experiment 293 production source is restored byte-for-byte.

## Question

Can optimized Rust match the original prover's `NDEBUG` behavior by retaining
the `VarBankVarAssertAlloc` preconditions and existing-variable invariants only
in debug builds?

## Candidate

Experiment 295 measured `VarBank::var_assert_alloc` at `127,259,653`
instructions across `1,571,112` calls. Only `11` calls allocate; the other
`1,571,101` find an existing variable. The original C implementation expresses
the negative-code, variable-count, type-presence, and identical-type
invariants as `assert()` checks, so its optimized `NDEBUG` executable omits
them.

The candidate:

- changes the negative-code precondition in `f_code_find` to `debug_assert!`;
- changes the negative-code and shared-type preconditions in
  `var_assert_alloc` to debug assertions;
- changes the existing variable's count and type checks to
  `debug_assert_eq!`;
- adds a debug-only regression proving that an existing code requested with a
  different shared type still panics.

The paged variable table, per-type ordered maps, fresh-variable state, shadow
banks, allocation path, and returned `Term` handles are unchanged. Public
debug behavior retains the diagnostics.

## Validation

- All 18 focused variable-bank tests pass with default and all features.
- Formatting passes.
- The native and WSL candidate fingerprints both record exactly
  `features=["default"]`.
- Three parent and eight candidate native runs all exit zero, emit empty
  stderr, and produce the same 378-byte stdout with SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
- The deterministic candidate proves LUSK6, processes the same search, and
  exits zero.

## Deterministic result

Exact default-feature LUSK6 Callgrind falls from accepted Experiment 293's
`8,718,487,029` instructions to `8,606,874,375`:

- delta: `-111,612,654`;
- improvement: `-1.280184%`;
- hypothetical Rust/C ratio: `1.638044`, versus `1.659286`.

The standalone `VarBank::var_assert_alloc` symbol disappears after the smaller
release body is inlined into its callers. Several caller owners grow as work
is redistributed by the optimizer, but the whole deterministic workload still
removes more than 111 million instructions.

Raw profile:

```text
.artifacts/experiments/2026-07-24-023-debug-varbank-assertions/callgrind-debug-varbank-assertions.out
```

## Native result

Four alternating warmup pairs were excluded. All 128 processes in one
64-pair measured block prove and exit zero. Positive percentages below mean
the candidate is slower.

| Metric | All 64 pairs | Last 32 pairs |
| --- | ---: | ---: |
| Wall mean | +0.815827% | +0.476211% |
| CPU mean | +0.635209% | +0.485256% |
| Wall median | +0.068685% | +0.862585% |
| CPU median | +0.591716% | +1.204819% |
| Mean paired wall change | +0.909510% | +0.588727% |
| Mean paired CPU change | +0.737215% | +0.618799% |
| Median paired wall change | +0.579751% | +0.544679% |
| Median paired CPU change | 0.000000% | +0.602410% |
| Candidate wall wins | 20 of 64 | 10 of 32 |
| Candidate CPU wins | 20 of 64, 13 ties | 10 of 32, 6 ties |

The production candidate executable also grows from `8,928,256` to
`8,948,736` bytes, an increase of `20,480` bytes. The stable half preserves
the wall and CPU regression, and the candidate wins less than one third of
those pairs. A second native block is not warranted.

Tracked measured rows are in `native-lusk-block1.csv`; excluded warmups are
retained in the ignored experiment artifact directory.

## Decision

Reject. Omitting the C `assert()`-equivalent work yields a large deterministic
instruction reduction and preserves the exact proof, but it reliably worsens
the production Windows executable in aggregate, paired, median, stable-half,
and binary-size evidence. This is the third rejected form of the variable-type
invariant boundary after Experiments 219 and 272.

Candidate production code and its candidate-only test were removed.
`src/terms/termvars.rs` is restored byte-for-byte to accepted Experiment 293.
Compatibility matrices and full repository gates are skipped after the
decisive native rejection.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=callgrind-debug-varbank-assertions.out \
  target-wsl-296-debug-varbank-assertions/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-293-fuse-always-deref-app-check\release\eprover.exe `
  -CandidateExe .\target\native-296-debug-varbank-assertions\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-023-debug-varbank-assertions\native-lusk-block1.csv
```
