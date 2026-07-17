# eground expanded executable comparison

## Status

Completed for Bead `E_Rust_Port-j76.1.39`. This slice fixes executable exit
status and verbose-progress parity, turns the source-mirrored DIMACS stream
leak into an exact golden, expands the permanent support-tool matrix, and
records the narrow host-resource boundaries that are not deterministic batch
inputs. The vendored C source remained unchanged.

## Archived C reference evidence

The newest surviving real C/Rust report is:

`.artifacts/e-compare/20260715-203258-985096-tools/tool-comparison.json`

It used upstream commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` and
proves byte-for-byte equality for `eground --help`, `--version`, and the small
silent `--lop-in` grounding stream. Each case also has equal exit status zero.
Earlier reports show the same three cases, with the initial help mismatch fixed
before the archived exact runs.

## Diagnostic exit-code fix

The Rust library already attaches C `ErrorCode` values to diagnostics, but
`src/bin/eground.rs` printed every error and returned 1. The wrapper now uses
`error.code().exit_status()` like the other completed support tools. Optimized
executable cases prove:

- malformed term and trailing-token scanner failures exit 3;
- equal hard/soft limits in either option order exit 5;
- missing input and a missing output parent exit 6; and
- the non-ground infinite-Herbrand-universe diagnostic exits 12.

Message content, channel, relative path, and line breaks remain strict. The
comparison harness canonicalizes only the complete known POSIX/Windows system
error suffix on file failures.

## Verbose progress timing

C changes `Verbose` while processing options, before resource setup and
`OpenGlobalOut`. Its successful stdin conjecture path emits, in order:

```text
eground: Output is going to <stdout>
eground: Input is coming from <stdin>
eground: Closing input
eground: Negated conjectures.
eground: CNFization done
eground: Closing output
```

Rust now preserves this timing. Named inputs also retain the level-2 `Trying
file` message before opening and the level-1 `Input file is` message only after
a successful open. Parser failures do not fabricate C's later close message.
The permanent verbose case pins the exact stderr above beside `% Success!` on
stdout.

`--memory-limit=Auto` is likewise option-order sensitive. At verbose level one,
Rust prints the detected physical MB followed by the converted byte limit with
C's misleading `MB` label. Pure regressions pin paired reduced-limit warnings
for `RLIMIT_DATA` and the second `RLIMIT_DATA` call labeled `RLIMIT_AS`; failed
`RLIMIT_DATA` warnings remain masked like C's ARM/macOS workaround.

## DIMACS output-file golden

`GroundSetPrintDimacs()` passes its destination to unit-clause and terminator
writes, but `ClausePrintDimacs()` sends non-unit literal integers directly to
`stdout`. For `fof(ax,axiom,(p(a)|q(a))).` with `--dimacs -o ground.cnf`, the
exact leaked stdout is `  4  6` with deliberately no trailing newline. The
configured file is exactly:

```text
%
p cnf 6 1
 0
% Full and complete proof state written!
```

Both streams are asserted in the Rust unit suite and optimized experiment
runner, and the harness compares stdout and the generated file independently.
This preserves an upstream bug for drop-in compatibility; the existing
`Change Later` item owns any cleaned non-compatibility mode.

## Expanded permanent matrix

The matrix now contains 16 cases: help, version, and 14 functional cases. New
coverage includes:

- TSTP formula-owner grounding and a selected relative include;
- exact verbose conjecture progress and DIMACS file/stdout routing;
- malformed term, trailing token, and semantic non-ground failures;
- the deterministic `--give-up=1` estimate stop;
- successful hard/soft/memory option processing and both ordering errors; and
- isolated missing-input and missing-output-parent failures.

The selected include exercises the shared represented parser without creating
an executable-local grammar. Record and include shapes outside that supported
surface remain owned by Bead `E_Rust_Port-j76.1.40`, which tracks full shared
`FormulaAndClauseSetParse` parity.

## CPU and memory stop boundary

Ground generation polls the shared `TimeIsUp` state and preserves C's timeout
completion state. The deterministic user estimate limit is fully covered.
Actual POSIX `RLIMIT_CPU` hard termination, forced `getrlimit`/`setrlimit`
failures, and allocator exhaustion depend on host ceilings, signal timing, and
fault injection; they are not stable static input workloads.

On Windows, applying a job-object process-time limit would terminate with
`STATUS_QUOTA_EXCEEDED` before the C hard-timeout banner, diagnostic, and exit
code could run. Rust therefore uses cooperative grounding deadlines rather
than that job policy. Memory limits use the existing retained job object, while
Linux uses the source-shaped duplicated `RLIMIT_DATA` calls. Normal option
configuration, exact validation, memory warning rendering, cooperative timeout
state at the grounding layer, and `--give-up` are testable; forced host failures
remain an evidence-backed platform boundary. This is not a claim that an
unobserved hard-stop run was byte-equal.

## Current reference availability

This desktop session has no installed WSL distribution, visible cached C
executable, or native POSIX C toolchain. The 13 new cases could not be rerun
against the archived ELF binary. When a reference is restored, the matrix is
ready without further case construction:

```powershell
cargo build --locked --release --bins
.\e-interop.ps1 build-reference
.\e-interop.ps1 compare-tools -RustBinDir .\target\release -Tool eground
```

## Native verification

`run_native.py` materializes all permanent cases through the comparison-harness
helpers. All 16 optimized cases returned their expected status: nine exit 0,
two syntax cases exit 3, two option cases exit 5, two file cases exit 6, and the
semantic case exits 12. The selected include resolved in its isolated workdir,
the output file existed with exact DIMACS bytes, leaked stdout was exact, and
the failed output path was absent.

Validation:

- `cargo test --locked --lib prover::eground::tests --quiet -- --test-threads=1`:
  29 passed;
- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,145 passed;
- all binary targets passed under `cargo test --locked --bins`;
- integration targets `eprover_schedule`, `e_stratpar`, and
  `executable_inventory`: 4, 1, and 1 passed;
- `cargo check --locked --all-targets`: passed;
- `cargo clippy --locked --all-targets -- -D warnings`: passed;
- `cargo fmt --all -- --check`: passed;
- `cargo build --locked --release --bin eground`: passed;
- bundled-Python `unittest` discovery under `tools/e-interop`: 32 passed;
- isolated optimized executable matrix: 16 expected outcomes passed;
- C-source documentation coverage: 492 sources / 266 unit docs;
- Change Later and local-link checks: 269 Markdown files each; and
- regeneration preservation: 268 Markdown files.

## 2026-07-17 correction

Fresh archived-C execution superseded this experiment's native-only claim that
unconstrained `--give-up=1` stops. In C, `ClauseSetCreateGroundInstances`
assigns `PStackGetSP(default_terms)` to a local `bool tmp`; the estimate is
therefore `1^vars` for every nonempty term set and does not exceed a positive
limit of one. Rust now preserves that bug, while the constrained helper's real
`double` estimate still stops. The paired exact cases and debugger evidence are
recorded in `experiments/2026-07-17-004-eground-give-up-parity/FINDINGS.md`.
