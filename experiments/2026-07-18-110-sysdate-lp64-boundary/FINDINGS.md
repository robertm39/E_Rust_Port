# SysDate LP64 compatibility boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.28`. No runtime behavior change is
required: Rust's host-independent `i64` representation is the correct match for
the normative unchanged Linux C reference, including when Rust itself runs on
Windows. The vendored C checkout remains unchanged.

## Ownership decision

Upstream defines `SysDate` as C `long`. This repository explicitly builds the C
reference under Linux in WSL 2 and compares the native Windows Rust executable
against that binary. The pinned reference therefore uses the LP64 data model: a
signed 64-bit `long` and unsigned 64-bit `unsigned long`. Upstream's supplied
build variables identify generic GCC plus tested Linux and macOS toolchains; no
native Windows C build is part of the current reference contract.

Making `SysDateRaw` depend on the Rust host's C ABI would select 32 bits on
Windows and would stop matching the actual LP64 C reference. The alias therefore
remains `i64` on every Rust host, with an explicit width regression and API
documentation recording that intent.

The operational prover uses `SysDate` for monotonic rewrite timestamps,
sentinels, comparisons, and increments. A complete source call-site search found
no use of `SysDatePrint` outside its declaration and implementation, in either C
or Rust. Its unusual signed-`long` argument with a `%lu` conversion is therefore
not on an executable output path, but the Rust helper still preserves the
observed LP64 bit interpretation.

## Direct unchanged-C evidence

[`compare_sysdate.py`](compare_sysdate.py) compiles the unchanged
`BASICS/clb_sysdate.c` at pinned commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0` under WSL and calls the actual
`SysDatePrint`. The retained [`reference.json`](reference.json) verifies:

- `sizeof(long) == 8` and a 64-bit `long`;
- exact signed and unsigned maximum values; and
- exact creation, ordinary, invalid-sentinel, and signed-maximum rendering
  against Rust's host-independent model.

All four rendering cases and the ABI checks are exact. No throughput benchmark
is warranted because the representation and runtime code are unchanged; this
slice makes the existing compatibility boundary explicit and reproducible.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-110-sysdate-lp64-boundary\compare_sysdate.py `
  --output target\sysdate-lp64-reference-check.json `
  --expected experiments\2026-07-18-110-sysdate-lp64-boundary\reference.json

cargo test --locked --lib basics::sysdate::tests
```
