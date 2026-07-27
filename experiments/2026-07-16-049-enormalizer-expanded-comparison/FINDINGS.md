# enormalizer expanded executable comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.41`. This slice fixes executable
diagnostic exit statuses, matches option-order-sensitive automatic-memory
messages and reduced-limit warnings, and expands the permanent `enormalizer`
matrix from three cases to 22. The vendored C source remained unchanged.

## Archived C reference evidence

The newest surviving real C/Rust report is:

`.artifacts/e-compare/20260715-203258-985096-tools/tool-comparison.json`

It used upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` and
records equal exit status and byte-normalized stdout/stderr for `--help`,
`--version`, and the small LOP term-normalization workload. The 19 new cases
could not be executed against C in this desktop session because no WSL
distribution, cached Windows reference executable, or native POSIX C toolchain
is available. Their permanent definitions are ready for the normal comparison
command when a reference environment is restored.

## Diagnostic exit-code fix

The Rust library already returned C-shaped `ErrorCode` values, but
`src/bin/enormalizer.rs` printed every diagnostic and collapsed every failure to
exit status 1. The executable wrapper now prints `error.message()` with one
program prefix and returns `error.code().exit_status()`, matching the completed
support-tool wrappers.

The optimized matrix pins status 3 for malformed rule, term, clause, and
formula input; status 5 for invalid hard/soft limit order in either option
order; and status 6 for missing rule, target, and output-parent paths. Stable
usage diagnostics and the first line of each file diagnostic are exact. The
second file-error line must retain the program prefix while allowing the host
OS message and error number to vary.

## Expanded normalization and stream coverage

The matrix now contains help, version, and 20 functional cases. Successful
workloads assert exact stdout or generated-file bytes for:

- LOP term and clause normalization;
- TSTP formula-target normalization with discarded source/useful-info fields;
- old-TPTP formula records with a positive-integer name, `lemma` and `unknown`
  role collapse to `axiom`, retained `question`, and
  `negated_conjecture` rendering as `conjecture`;
- a default stdin rule source containing a relative include;
- C's rule-before-target order when the default rule source and `--terms=-`
  compete for one stdin stream;
- the accepted-but-unused `--print-statistics` flag;
- output-file routing; and
- successful hard/soft CPU and zero-memory limit configuration.

Rule parsing uses the shared recursive represented-owner parser, so includes
are supported there. Target term, clause, and formula scanners do not implement
include records, matching their distinct C processing functions. Reading the
default rule source drains stdin before a later stdin target scanner sees EOF;
the dedicated case asserts completely empty stdout and stderr.

## Resource and host-error boundary

C mutates `Verbose` while parsing options. Consequently,
`--verbose=1 --memory-limit=Auto` prints the detected physical-memory MB and
the converted byte limit with C's misleading `MB` label, while reversing those
options prints neither line. Rust now preserves that timing. A reduced memory
limit produces the two C warnings labeled `RLIMIT_DATA` and `RLIMIT_AS`; the
source's masked failed-`RLIMIT_DATA` branch remains silent.

Normal resource option setup and validation are deterministic and covered. On
POSIX, C also calls `getrlimit(RLIMIT_CPU)`, sets the CPU limit, and disables
core dumps; failures are fatal system diagnostics. Rust's shared portable
limit layer uses cooperative deadlines instead, because a Windows job-object
hard stop would terminate before C's own diagnostic path could run. Forced
resource-syscall failures, actual CPU-limit signal delivery, allocator
exhaustion, and reduced-limit outcomes require host ceilings or fault
injection. Broken-pipe termination is likewise signal- and platform-dependent:
POSIX C may terminate through `SIGPIPE`, while Windows reports a write error.
These are recorded as host boundaries rather than claimed byte-equal static
workloads.

## Native verification

`run_native.py` materializes all permanent cases with the comparison-harness
helpers and executes the optimized Rust binary. It checks all 22 statuses,
stable stdout/stderr bytes, generated output bytes, absent failed-output paths,
system-error shape, and the shared-stdin exhaustion invariant.

Validation at completion:

- focused `enormalizer` library tests: 36 passed;
- full library suite: 4,152 passed;
- all binary and integration targets passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- `cargo build --locked --release --bin enormalizer`: passed;
- bundled-Python `tools/e-interop` discovery: 32 passed;
- optimized native `enormalizer` matrix: all 22 expected outcomes passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later wording and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.
