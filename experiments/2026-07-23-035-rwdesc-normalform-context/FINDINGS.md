# Experiment 273: Package recursive normal-form state

## Status

Rejected performance candidate for Bead `E_Rust_Port-j76.5.3`; accepted
production source is restored.

## Question

Can the plain recursive normal-form engine match C's `RWDesc` call shape by
packaging its invariant bank, ordering, demodulator, policy, date, and trace
state into one borrowed Rust descriptor?

## Candidate

`PlainRewriteDesc` borrows the existing rewrite state and carries one mutable
descriptor reference through recursive `normalform` and `rewrite_subterms`
method calls. The descriptor contains:

- the term bank and ordering control block;
- the active demodulator slice and rewrite level;
- the precomputed maximum demodulator date;
- `prefer_general` and the lambda-demodulation policy;
- the existing per-clause SoS trace and reusable substitution.

It introduces no allocation and does not change rewrite selection, traversal
order, rewrite links, normal-form dates, substitution reuse, or public APIs.
This is the direct private Rust analogue of the C `RWDesc` call shape.

## Focused correctness

All 33 rewrite tests pass with default and all features. The all-feature
Windows test build initially exhausted LLVM's available memory with full test
debug information; a serial, incremental-off retry with
`CARGO_PROFILE_TEST_DEBUG=0` compiled and passed normally. This was a compiler
resource condition, not a test failure.

The candidate proves LUSK6 directly and under Callgrind. A direct native
parent/candidate check exits zero and produces byte-identical standard output
and standard error.

## Deterministic result

The candidate improves the accepted Experiment 270 exact baseline:

- accepted Rust: 8,992,812,925 instructions;
- candidate Rust: 8,981,251,147 instructions;
- delta: -11,561,778;
- improvement: 0.128567%;
- C reference: 5,254,361,329 instructions;
- hypothetical Rust/C ratio: 1.709295.

The raw candidate profile is:

```text
.artifacts/experiments/2026-07-23-035-rwdesc-normalform-context/rust-callgrind-rwdesc-normalform-context.out
```

## Native result

The accepted Windows executable is 8,952,320 bytes and the candidate is
8,951,808 bytes, 512 bytes smaller. Four alternating warmup pairs were
excluded. All 128 measured processes prove and exit zero.

Across 64 alternating measured pairs:

- wall means are effectively tied, improving only 0.001141%, from
  1.402950 to 1.402934 seconds;
- CPU means regress 0.376412%, from 1.362061 to 1.367188 seconds;
- wall and CPU medians regress 0.098646% and 0.578035%;
- mean paired wall and CPU changes regress 0.078515% and 0.421777%;
- median paired wall changes regress 0.171513%, while paired CPU medians tie;
- the candidate wins only 28 wall and 19 CPU pairs, with 19 CPU ties.

The stable final 32 pairs are more clearly negative:

- wall and CPU means regress 0.147924% and 0.715564%;
- wall medians improve 0.362490%, while CPU medians tie;
- mean paired wall and CPU changes regress 0.262294% and 0.779498%;
- median paired wall and CPU changes regress 0.070028% and 1.149425%;
- the candidate wins 15 wall and seven CPU pairs, with eight CPU ties.

The final 16 pairs also regress 1.319532% wall and 0.782361% CPU by mean.
Raw warmup and measured rows are in `native-warmup.csv` and
`native-lusk.csv`.

## Validation and restoration

- All 33 focused rewrite tests pass with default and all features.
- Direct, Callgrind, warmup, and measured native processes prove and exit
  zero.
- Direct accepted and candidate native output is byte-identical.
- Compatibility matrices and full repository gates are skipped after the
  stable native rejection.
- `src/clauses/rewrite.rs` is restored byte-for-byte to the accepted version.
- The original `eprover/` checkout remains untouched.

## Decision

Reject. Packaging the recursive state removes real deterministic instruction
work, but it makes native CPU time worse across the full sample and the
stable tail. Keep the explicit recursive arguments and the accepted
Experiment 270 baseline at 8,992,812,925 instructions, or 1.711495 times C.

This also means the documented full `RWDesc` ownership work remains open; a
future descriptor should be revisited only as part of a broader indexed
demodulator/normalization ownership change, not as this isolated call-shape
refactor.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=rust-callgrind-rwdesc-normalform-context.out \
  target-wsl-273-rwdesc-context/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-273-rwdesc-context\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-035-rwdesc-normalform-context\native-lusk.csv
```
