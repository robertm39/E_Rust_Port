# Term-ordering Precedence-generator State

## Question

Do Rust's sorted precedence arrays and installed low-to-high symbol orders match
the actual C generator state for every implemented method?

The earlier option matrix proved the method-name and diagnostic surface, but
its per-method cases stopped after printing strategy parameters. This
experiment observes the generated order itself.

## Reference instrumentation

[`build_instrumented_reference.sh`](build_instrumented_reference.sh) copies the
isolated higher-order C reference at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` into a new WSL cache directory and
enables the source's existing `PRINT_PRECEDENCE` block. The disposable build
keeps `-DNDEBUG` and `-DENABLE_LFHO`, adds `-O0 -g`, and does not edit a C source
file. The vendored `eprover/` checkout is neither modified nor used as the build
directory.

C prints generated user symbols from high to low. The collector retains that
line and its exact reverse, which is the low-to-high order returned by Rust's
`generate_precedence_order` helper and written into OCB rank/matrix storage.

## Matrix

[`collect_precedence_orders.py`](collect_precedence_orders.py) records all 18
implemented generators in [`reference_orders.json`](reference_orders.json):

- 13 ordinary unary/arity/constant/frequency/conjecture-frequency variants;
- all four `ENABLE_LFHO` type/combined-frequency variants; and
- `arrayopt`, using names that exercise all of its special prefix/classes.

The FOL fixture includes both an axiom and a conjecture, so the three distinct
conjecture-frequency policies have observable orders. The typed fixture uses
two sorts and unequal symbol/type frequencies. The ArrayOpt fixture retains its
symbols in the signature even though preprocessing removes the reflexive
clauses.

`orient_axioms` is not counted among the implemented methods: upstream C
asserts `Precedence generation method unimplemented`, and the executable option
surface reports the existing failure. Rust's separate permanent regression
continues to require the C-shaped `Not yet implemented` diagnostic.

## Rust result

The permanent `instrumented_c_reference_precedence_orders_match` regression
constructs the same post-preprocessing clause/signature states and matches all
18 retained low-to-high user-symbol arrays exactly. Existing tests separately
pin:

- rank-backed KBO installation, including `$true` and insertion-order minimum
  constants;
- tuple-chain installation for matrix-backed LPO;
- predefined-only partial precedence without `PNoMethod` fallthrough; and
- occurrence/property modifiers applied before method-specific sort keys.

No production formula or ownership change was required.

## Reproduction

```powershell
wsl -d Ubuntu-24.04 -- sh `
  /mnt/c/Users/rober/Code/E_Rust_Port/experiments/2026-07-17-072-precgen-state/build_instrumented_reference.sh `
  /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-20260717b `
  /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-precdebug-20260718d

python experiments\2026-07-17-072-precgen-state\collect_precedence_orders.py `
  --c-exe /home/rober/.cache/e-rust-port/src/17026b1bfe61aaf223cfaae54947c8d2679c31a0-ho-precdebug-20260718d/PROVER/eprover-ho `
  --expected experiments\2026-07-17-072-precgen-state\reference_orders.json `
  --output C:\tmp\precgen-reference.json

cargo test --locked --all-features instrumented_c_reference_precedence_orders_match
```

## Compatibility decision

Pure feature-array generation, low-to-high extraction, rank/matrix installation,
predefined partial precedence, proof-control ownership, and the upstream-
unimplemented `orient_axioms` boundary are covered. The migrated item describes
completed behavior rather than remaining port work.
