# Standalone executable inventory audit

## Question

Does every standalone C entry point in the reviewed `PROVER`, `SIMPLE_APPS`,
and `EXTERNAL` application directories have an exact-name Rust binary
registration and an actual Rust `main` wrapper?

This resolves Bead `E_Rust_Port-j76.1.1`. Broader filesystem workload and
byte-for-byte executable comparisons remain independently tracked by
`E_Rust_Port-j76.1.2` and the executable-specific pending Beads.

## Method

The audit treats a `.c` file containing `int main(` as an upstream standalone
entry point. It scans:

- `eprover/PROVER`;
- `eprover/SIMPLE_APPS`;
- `eprover/EXTERNAL`.

The Rust side is the set of `[[bin]]` names in `Cargo.toml`. Each registration
must point to an existing source file containing `fn main(`. The new
`tests/executable_inventory.rs` integration test computes both sorted sets and
requires exact equality, so additions or removals on either side produce a
diagnostic set difference.

The focused audit is reproducible with:

```text
cargo test --test executable_inventory
```

## Result

Both sets contain exactly these 26 programs:

```text
CSSCPA_filter
checkproof
classify_problem
direct_examples
e_axfilter
e_client
e_deduction_server
e_ltb_runner
e_server
e_stratpar
edpll
eground
ekb_create
ekb_delete
ekb_ginsert
ekb_insert
enormalizer
epatternize
epclanalyse
epclextract
epcllemma
eprover
ex_commandline
term2dag
termprops
tsm_classify
```

The upstream `PROVER/Makefile` comments out the `termprops` and `tsm_classify`
link rules, but both source files still define standalone `main` functions.
Rust already exposes both, so scanning source entry points is stricter than
checking only the default upstream build list. `EHOH` is empty in the checked-in
`Makefile.vars`; alternate higher-order `eprover` builds reuse the same
`eprover.c` entry point and do not represent another wrapper source.

## Validation

The following gates passed:

- `git diff --check`;
- `cargo fmt --all -- --check`;
- `cargo check --all-targets --all-features`;
- pedantic Clippy over all targets and features with warnings denied;
- the focused inventory test;
- all 4,090 library tests, all 26 binary targets, all three schedule tests, and
  the new inventory integration test.

No runtime implementation or representation changed, so a C/Rust performance
comparison would not measure this patch. Existing executable workload expansion
remains under the separate comparison Beads rather than being used to keep the
inventory item open.

## Conclusion

The reviewed standalone application inventory is complete: there is no missing
Rust wrapper in the three directories named by the legacy work item. The
permanent exact-set regression test makes this an evidence-backed closure rather
than a one-time manual assertion.
