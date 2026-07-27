# PCL Proofcheck Real-E Success Marker

## Question

Should Rust `checkproof` continue matching C's accidental requirement for
`%% Proof found!`, or recognize the `% Proof found!` banner emitted by real E
so valid derived steps can actually be verified?

## Source Defect

`eprover/BASICS/clb_defines.h` defines `COMCHAR` as `"%%"` for use inside
`printf` format strings. `eprover/PCL2/pcl_proofcheck.c` passes
`COMCHAR " Proof found!"` directly to `strstr`, where format escaping does not
apply. The C checker therefore searches for two literal percent signs even
though E emits one.

The preceding compatibility audit in
[`experiment 065`](../2026-07-16-065-pcl-proofcheck-edges/FINDINGS.md) captured
this behavior exactly. Carrying it into Rust made the supported checker reject
valid E-proved inference problems, so this follow-up treats the marker as an
intentional correctness divergence rather than a compatibility option.

## Change

The E invocation's success token is now `% Proof found!`. The common scanner
still preserves C's fixed 180-byte `fgets` buffer shape, per-chunk substring
search, embedded-NUL C-string truncation, trace prefixes, command construction,
and temporary-file handling. Otter and SPASS retain their existing markers.

A doubled marker remains accepted because `%% Proof found!` contains the real
single-percent token. Output with no marker still returns `CheckFail`.

The support-tool harness now permits a functional case to declare an exact set
of expected comparison fields. Declared differences are recorded under
`expected-differences/`; a missing, extra, or misspelled difference remains a
failing mismatch. Self-tests ignore declarations and still require archived C
to match itself exactly.

## Reproduction

Build the affected Rust executables and run the paired matrix from the
repository root:

```powershell
cargo build --locked --release --bin checkproof --bin eprover
.\e-interop.ps1 compare-tools -RustBinDir .\target\release -Tool checkproof
```

Run the focused tests with:

```powershell
cargo test --lib --all-features pcl2::proofcheck::tests::
cargo test --lib --all-features prover::checkproof::tests::
.\.venv\Scripts\python.exe tools\e-interop\test_e_interop.py
```

## Results

The accepted report is
`.artifacts/e-compare/20260717-011000-444604-tools/`:

- 16 `checkproof` cases ran;
- zero unexpected mismatches remain; and
- two declared `normalized_stdout` differences are present, one for a real
  paired `eprover` and one for a deterministic shell adapter.

In the real-E case, both companion provers emit `% Proof found!` and
`% SZS status Unsatisfiable`. Archived C then dumps the proved generated
problem, prints `FAILED`, and reports one of two steps checked. Rust prints
`Checked (by prover)` and ends with two of two steps checked and
`Proof verified!`.

The shell adapter produces the same classification difference without relying
on prover search. The existing `echo NO-PROOF` case remains exact between C and
Rust and reports `Failed to verify proof!`. The doubled-percent adapter remains
accepted by both implementations.

## Validation

- All 25 focused `pcl2::proofcheck` tests pass, including single-percent,
  doubled-percent, missing-marker, fixed-chunk, and C-string-view checks.
- All 20 `prover::checkproof` tests pass. The new executable-path tests assert
  `Checked (by prover)` plus `Proof verified!` for a single-percent shell
  adapter and `FAILED` for output without a proof marker.
- All 33 interoperability-harness tests pass, including metadata validation for
  expected mismatch fields.
- The paired real-E matrix proves that this is marker recognition, not a
  synthetic-only result.

## Limits

This intentionally stops matching one C output bug. It does not change the
generated legacy proof problem, fixed-size output chunking, shell command
construction, FOF-parent warnings, split handling, or Setheo/Otter/SPASS
behavior. Structured process spawning and the other proofcheck cleanup work
remain tracked separately.
