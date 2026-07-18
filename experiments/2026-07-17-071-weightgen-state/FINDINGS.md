# Term-ordering Weight-generator State

## Question

Do Rust's generated OCB weights match the actual C arrays, including the
behaviors that cannot be observed through `--print-strategy`?

The earlier term-ordering option matrix proved all 34 method names and their
diagnostics, but method-only cases stopped after strategy printing. This
experiment inspects generated state directly.

## Reference instrumentation

[`build_instrumented_reference.sh`](build_instrumented_reference.sh) copies the
isolated higher-order C reference at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` to a new WSL cache directory. It
changes only that disposable copy's build flags:

- `-O0 -g` keeps inspection practical;
- `-DNDEBUG` remains enabled, preserving the optimized executable boundary;
- `-DENABLE_LFHO` remains from the configured source; and
- `-DPRINT_FUNWEIGHTS` enables the existing, otherwise compiled-out
  `print_weight_array()` call in `TOGenerateWeights`.

No C source file is edited, and the vendored `eprover/` checkout is not used as
a build directory.

## Matrix

[`collect_weight_arrays.py`](collect_weight_arrays.py) runs 15 cases and parses
the emitted `% Ordering weights:` line. The exact per-symbol arrays and C exit
codes are retained in [`reference_weights.json`](reference_weights.json).

The seven ordinary cases cover:

- generated arity weights, forced constant weights, then a late user override;
- direct and inverse counting through a predefined partial precedence matrix;
- `precrank5` totalization of that partial matrix through alpha rank; and
- inverse-conjecture-frequency, squared-frequency-rank, and modified-inverse-
  frequency zero-sentinel behavior.

The eight typed cases cover every `ENABLE_LFHO` type/combined frequency count
and rank method. In particular, the observed arrays retain C's source
inconsistency: `combfreqcount` aggregates symbol frequencies by type, while its
inverse and rank relatives use clause type distribution.

## Rust result

The permanent
`instrumented_c_reference_weight_arrays_match` regression constructs the same
post-preprocessing FOL clause/signature state and typed THF state. It checks all
15 exact user-symbol arrays. The rank-method portion invokes the method body
before C's common constant post-pass; this is equivalent when
`WConstNoSpecialWeight` is selected and avoids turning C's release-only zero-
weight behavior into a Rust debug-assertion failure. The late-override case
uses the complete public generation path and pins `[a:9, f:2, b:3, g:2]`.

All 15 retained C snapshots match the Rust regression. No production formula
change was required.

## Reproduction

```powershell
wsl -d Ubuntu-24.04 -- sh `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-17-071-weightgen-state/build_instrumented_reference.sh `
  /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b `
  /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-weightdebug-20260718c

python experiments\2026-07-17-071-weightgen-state\collect_weight_arrays.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-weightdebug-20260718c/PROVER/eprover-ho `
  --expected experiments\2026-07-17-071-weightgen-state\reference_weights.json `
  --output C:\tmp\weightgen-reference.json

cargo test --locked --all-features instrumented_c_reference_weight_arrays_match
```

## Compatibility decision

Pure generation, OCB installation, late `TOWeightsParse` overrides,
precedence-source selection, explicit higher-order problem context, and all
documented C quirks are covered. The migrated item describes completed
behavior rather than remaining port work.
