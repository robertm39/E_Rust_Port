# App-encode typed-application type ownership

## Status

Completed for Bead `E_Rust_Port-j76.2.91`. The existing Rust ownership split is
now covered by a permanent executable regression and a live C/Rust comparison.

## Question

When `TermAppEncode` creates a typed binary application symbol, does its
synthetic `(function_type * argument_type) > result_type` signature type become
a normal shared type printed by `TypeBankAppEncodeTypes`, or does it remain
owned only by the signature symbol and appear solely in
`SigPrintAppEncodedDecls`?

## C source behavior

The unchanged reference is commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`.

`SigGetTypedApp` allocates a three-component arrow type and assigns it directly
to `sig->f_info[ret_fcode].type`; it does not pass the outer arrow through
`TypeBankInsertTypeShared`. Its component types are already shared and carry
the UIDs used in the generated `app_<left>_<right>_<result>` name.

This matters at the output boundary:

- `TypeBankAppEncodeTypes` traverses only the bank's shared types, so it must
  omit the synthetic application-symbol arrows.
- `SigPrintAppEncodedDecls` reads those unshared outer types from the signature
  and prints declarations for the generated application symbols.

Rust's `Signature::get_typed_app` has the same split: `alloc_arrow_type` creates
the unshared outer type, while `declare_type` retains it on the symbol without
inserting it into `TypeBank`.

## Fixture and live result

[`input.p`](input.p) declares user sort `person`, constants `a`, `b`, and `c`,
a binary function `h: (person * person) > person`, and formula `h(a,b)=c`.
Application encoding needs two generated symbols and the real shared suffix
type `person > person`.

[`compare_app_encode.py`](compare_app_encode.py) runs the Windows Rust release
and cached WSL C reference, checks the exact set of `%--` shared-type comments,
and compares the remainder after normalizing only `typedeclN` order/labels.
Both binaries:

- exit 0 with empty stderr;
- print exactly six real shared types;
- print no unexpected synthetic type comment;
- produce identical type UIDs, typed-application symbol declarations, and
  encoded formula; and
- match completely after type-declaration order normalization.

Raw stdout intentionally does not match. C numbers types while walking
pointer-hashed buckets, and two live invocations during this investigation
already produced different orders. Rust's stable UID order is the existing
documented compatibility decision. [`results-summary.json`](results-summary.json)
retains the compact outcome; the harness prints the complete transcripts.

## Permanent regressions

The new executable test pins the entire stable Rust output for the fixture,
including exactly six type declarations, both application-symbol declarations,
and the final encoded formula. Existing lower-level tests independently verify
that `Signature::get_typed_app` does not increase `TypeBank::types_count` and
that `print_app_encoded_decls` still prints the retained outer type.

No production change or benchmark is warranted: the required ownership and
output behavior were already present, and the new work closes an evidence gap.

## Reproduction

```powershell
& 'C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' `
  experiments\2026-07-17-046-app-encode-typed-application-types\compare_app_encode.py `
  --rust-exe target\release\eprover.exe `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --distro Ubuntu-24.04
```

## Validation

- live C/Rust normalized executable comparison: exact
- C and Rust unexpected synthetic type-comment sets: empty
- focused permanent executable regression: passed
- full serial suite: 4,253 library tests plus all binary/integration targets
- strict all-target/all-feature pedantic Clippy: passed
- formatting and all four C-source documentation integrity gates: passed
