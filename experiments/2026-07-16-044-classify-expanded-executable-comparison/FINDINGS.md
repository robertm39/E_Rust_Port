# classify_problem expanded executable comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.36`. This slice turns the remaining
classifier-specific comparison surfaces into permanent support-tool cases,
pins exact malformed feature diagnostics in Rust, and records the two narrow
boundaries that cannot be treated as byte-for-byte data: C's uninitialized
legacy feature suffix and host/process syscall failures. The vendored C source
remained unchanged.

## Archived C reference evidence

The latest surviving real C/Rust classifier report is:

`.artifacts/e-compare/20260715-203258-985096-tools/`

It used upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`, GCC
13.3.0, and Ubuntu 24.04 under WSL2. Its classifier cases prove:

- `--help` is byte-for-byte equal;
- `--version` is byte-for-byte equal; and
- the original `--parse-features` case has equal exit status and stderr, with
  its sole stdout difference confined to fields that C never initializes.

The archived reference prints stack-dependent values such as `22319`, `32767`,
and `true` in the six post-legacy fields, while Rust prints deterministic zero
and `false` values. The first 22 serialized numeric fields and the stable class
prefix are equal.

## Legacy feature suffix decision

`SpecFeaturesParse()` fills the 22 values present in the legacy line and
recovers only the old class invariants. `process_feature_files()` allocates its
`SpecFeatureCell` without initialization, then `SpecFeaturesAddEval()`,
`SpecFeaturesPrint()`, and `SpecTypePrint()` consume six fields that the parser
did not write. Those fields also influence the last seven class characters.
Their C bytes are undefined behavior, not a stable executable contract.

The comparison harness now has an explicit per-case
`normalize_legacy_classify_feature_suffix` flag. Only the two standard legacy
feature cases enable it. The normalizer retains all 22 parsed values and the
first 14 class characters, replacing only the six uninitialized printed fields
and seven derived class characters. Raw feature parsing remains fully strict
because `RawSpecFeaturesParse()` initializes every printed field. Real-input
classifications also remain fully strict because their feature cells are
computed rather than legacy-parsed.

A Python regression embeds the archived C and Rust lines. It proves they remain
different under normal comparison and become equal only when this classifier-
specific flag is requested.

## Expanded permanent matrix

The classifier support-tool matrix now contains 27 total cases: help, version,
and 25 functional cases. The new coverage adds:

- valid raw feature-line parsing and reclassification;
- exact missing-colon, short standard-class, and short raw-class failures;
- successful feature output-file routing and a missing output parent;
- missing feature input and missing real input diagnostics;
- raw and standard FOF definition/conjecture inputs;
- `--specsig` over mixed predicate/function arities;
- `--generate-tptp-header` over non-Horn, equality, variable, and depth data;
- explicit `--merged-classification=-1` branch suppression; and
- a completed higher-order merged child.

The preceding parser slice's cases remain: raw LOP, old TPTP records, mixed
TFF/FOF/TCF/CNF, typed FOOL, raw THF, include selectors, positive CNF/FOOL
children, zero fallback, and the non-sentinel negative timeout.

## Malformed and platform diagnostics

Rust regressions now pin the complete missing-colon scanner message, including
stdin position, empty EOF literal, token descriptions, and trailing space. They
also pin C's historical `(to short)` typo and the misleading raw-class message
that says 10 characters even though the implementation requires 14.

The executable matrix compares syntax exit code 3 and file exit code 6 exactly.
For file-open failures it keeps program name, relative path, action wording,
line breaks, channel, and exit code strict. The shared harness canonicalizes
only the complete known POSIX/Windows OS error suffix. Successful output-file
creation is compared as file content; failure to create a file below a missing
parent also verifies that no output artifact appears.

## Merged child failure boundary

The stable observable contract is now covered from four directions:

- completed first-order, FOOL, and THF children write a complete 22-byte class
  buffer and produce a 36-character raw/CNF class concatenation;
- timeout zero produces the 21-hyphen short-read fallback shape;
- `-1` bypasses the merged branch and uses normal classification; and
- `-2` preserves the normal Linux effectively-unbounded behavior.

The lower-level Rust regression retains exact short-buffer fallback behavior.
Forcing C `pipe()`, `fork()`, `setrlimit()`, `write()`, or the parent's fixed
read to fail requires host-specific fault injection or resource exhaustion and
does not define a portable input/output workload. Likewise, mutating a named
input between Rust's parent parse and re-exec child reopen is outside the static
batch-input contract recorded in experiment 043. These cases are kept as
source-reviewed process-policy boundaries rather than nondeterministic
permanent tests.

## Current reference availability

This desktop session no longer has an installed WSL distribution, the WSL
reference cache is not visible, and no native POSIX C toolchain is installed.
The expanded cases therefore could not be rerun against the archived ELF
binary in this session. This is an environment limitation, not an assertion
that unobserved outputs matched.

When WSL is restored, the complete differential command is:

```powershell
cargo build --locked --release --bins
.\e-interop.ps1 build-reference
.\e-interop.ps1 compare-tools -RustBinDir .\target\release -Tool classify_problem
```

The new matrix and opt-in normalization are ready for that run without further
case construction.

## Native verification

An isolated native runner materialized every classifier case exactly as the
comparison harness does. All 27 cases returned their expected statuses; all 21
success cases returned 0, the three malformed feature cases returned 3, and
the three filesystem cases returned 6. The successful output file existed, the
failed output path did not, and the real optimized merged cases exercised the
hidden re-exec child.

Validation:

- `cargo test --locked --lib prover::classify_problem::tests --quiet -- --test-threads=1`:
  37 passed;
- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,136 passed;
- all binary targets passed under `cargo test --locked --bins`;
- integration targets `eprover_schedule`, `e_stratpar`, and
  `executable_inventory`: 4, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- `cargo build --locked --release --bin classify_problem`: passed;
- bundled-Python `unittest` discovery under `tools/e-interop`: 32 passed; and
- isolated optimized executable matrix: 27 expected outcomes passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
