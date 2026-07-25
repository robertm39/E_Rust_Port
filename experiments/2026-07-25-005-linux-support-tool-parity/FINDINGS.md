# Experiment 306: Native Linux support-tool parity

## Status

Accepted for Bead `E_Rust_Port-j76.5.4`.

## Question

Can the authoritative native-Linux 216-case support-tool matrix reach zero
unexpected mismatches by restoring C `SysError` text and path behavior while
narrowly declaring only the C crashes that Rust must not reproduce?

## Baseline

Accepted source is `7e7cc2e3d` and comprehensive Linode run
`.artifacts/linode/260725-190227-18ae/` reports:

- 216 support-tool cases;
- 33 unexpected mismatches;
- eight existing declared differences; and
- zero maintained main-prover mismatches.

Every unexpected tool case differs on normalized stderr:

- 26 cases differ only on stderr;
- seven cases also differ on exit, process shape, and sometimes output files;
- the 26 ordinary cases render Rust's `io::Error` suffix
  `(os error 2)`, while C uses `strerror(errno)` without that suffix;
- nine of those cases also expose tool-specific missing-input path or program
  name differences; and
- the seven multi-field cases are four `e_axfilter` fixtures and three
  knowledge-base fixtures where unchanged optimized C aborts or corrupts its
  output while safe Rust completes.

## Investigation

1. Add one shared conversion from `io::Error::raw_os_error()` through the
   existing C-runtime `strerror` bridge, falling back to Rust text only for
   synthetic errors without an OS code.
2. Route scanner/file/output diagnostics and executable `SysError` adapters
   through that conversion.
3. Re-run the matrix in both a focused worktree environment and the
   authoritative archived-reference environment with `$TPTP` set.

The first focused run reduced the unexpected count from 33 to eight, but that
environment did not exercise the authoritative `$TPTP` fallback paths and its
summary's `mismatches` field counts unexpected cases only. The first full run
at `.artifacts/linode/260725-195403-7092/` supplied the missing evidence:

- all 50 main-prover cases remained free of unexpected differences;
- 216 support-tool cases contained 16 unexpected plus eight existing expected
  differences;
- nine unexpected cases were deterministic scanner-path or invoked-program
  diagnostic differences; and
- the other seven were stable C aborts that safe Rust must not reproduce.

## Candidate

The candidate adds `c_io_error_message()` beside the existing narrow
`strerror` bridge. Raw operating-system errors use their C-runtime text;
synthetic Rust I/O errors retain their supplied display text. Scanner,
file-opening, output-opening, and executable `SysError` adapters all use that
one conversion.

Seven tools whose C entry points call `CreateScanner(StreamTypeFile, ...)` now
use `Scanner::from_file`, restoring C's local-then-`$TPTP` lookup and resolved
diagnostic path. `term2dag`, whose C entry point opens directly through
`InputOpen`, constructs `InputStream` explicitly and therefore retains the
caller-supplied path. The shared stream opener also rejects non-regular files,
matching C `InputOpen`. `termprops` uses the invocation-initialized global
program name in both lines of its diagnostic.

The tool normalizer replaces both the archived reference binary path and the
Rust candidate binary path with the canonical tool name. This handles the
legitimate `InitIO(argv[0])` behavior symmetrically rather than hiding a
program-specific message.

The following seven exact cases are declared safety differences:

| Tool case | Stable C behavior | Rust behavior |
| --- | --- | --- |
| `e_axfilter/tstp-threshold-file` | aborts after partial output | completes safely |
| `e_axfilter/tstp-gsine-formulas` | aborts after partial output | completes safely |
| `e_axfilter/tstp-lambda-def-formulas` | aborts after partial output | completes safely |
| `e_axfilter/tstp-seeded-all-methods` | aborts after partial output | completes safely |
| `ekb_delete/drop-example` | aborts during valid update | completes safely |
| `ekb_delete/drop-middle-example` | aborts during valid update | completes safely |
| `ekb_insert/stdin-example` | aborts during valid insertion | completes safely |

The declarations pin the exact observed fields. They do not permit any
additional output, exit, process-shape, or file mismatch.

## Results

On ephemeral native Linux worker `e-rust-codex-260725-202247-ffec`:

- library tests: 4,384 passed;
- Python compatibility-controller tests: 41 passed;
- Rustfmt: passed;
- strict all-target/all-feature Clippy: passed;
- release binaries: built;
- C FOL and HO references: built from pinned commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`;
- Rust-versus-C support-tool report
  `/root/.artifacts/e-compare/tools/20260725-203220-150133-tools/`:
  216 cases, zero unexpected mismatches, and 15 exact expected differences;
- C-versus-C self-comparison report
  `/root/.artifacts/e-compare/tools/20260725-203333-284628-tools/`:
  216 cases, zero mismatches and zero expected differences.

One preceding self-comparison pass observed the existing
`classify_problem/merged-zero-fast-child` scheduling-sensitive stdout race;
the immediate full rerun was exact. The Rust-versus-C report was exact on that
case and the accepted inventory does not declare it.

Comprehensive clean-room run
`.artifacts/linode/260725-203453-d4fc/` then confirmed:

- 4,401 Rust tests across the library, binaries, and integration targets;
- Rustfmt and strict all-target/all-feature Clippy;
- native optimized Rust binaries and Windows GNU x64 test/release binaries;
- clean pinned FOL and HO C builds;
- 50 main-prover cases with zero unexpected and one existing expected
  difference;
- 216 support-tool cases with zero unexpected and 15 exact expected
  differences;
- ten benchmark cases with zero behavior mismatches and a `2.649x` aggregate
  Rust/C wall-time ratio; and
- smoke Callgrind counts of 99,794,981 Rust versus 7,590,630 C instructions.

The worker and firewall were deleted after artifact collection.

## Acceptance

- Focused error-rendering and executable regressions pass on native Linux.
- The full native-Linux 216-case tool matrix reports zero unexpected
  mismatches.
- Self-comparison still reports no differences, proving declarations do not
  hide harness defects.
- Full Rust, Windows-cross, C-build, main-matrix, timing, and smoke-Callgrind
  gates pass.
- Documentation checks and vendored-source cleanliness pass locally after the
  final evidence update.
