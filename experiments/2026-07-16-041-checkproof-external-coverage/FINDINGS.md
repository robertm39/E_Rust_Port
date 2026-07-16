# checkproof external coverage

## Status

Completed for Bead `E_Rust_Port-j76.1.33` with a release-parity fix,
real-E companion comparison cases, deterministic legacy-prover adapters,
compressed/FOF/parser/error cases, and evidence-backed platform decisions. The
vendored C source remained unchanged.

## Setheo release parity

`checkproof.c` accepts `scheme-setheo`, but `PCLStepCheck` has no `Setheo`
switch arm. Its default contains only `assert(false && "Not yet implemented")`
followed by `break`; `res` was initialized to `CheckFail`. The normal upstream
flags define `NDEBUG`, so a generated Setheo check returns `CheckFail`, not
`CheckNotImplemented`. Assumptions return earlier as `CheckByAssumption`, and
split steps return earlier as `CheckNotImplemented`.

Rust previously put generated Setheo checks in the unchecked bucket. It now
matches the release executable: the three-step regression and permanent
`setheo-release-failure` case produce one checked assumption, one failure, one
unchecked split, and the final `Failed to verify proof!` summary. The internal
Rust `NoProver` adapter remains unchecked so lower-level tests can inspect
generated problems without starting a process.

## Real E subprocess cases

The comparison harness now supports a quoted `{companion:eprover}` argument.
For the reference run it resolves to the archived C `eprover`; for the Windows
candidate it resolves to `eprover.exe` in the Rust binary directory. The
placeholder is substituted independently for each platform and is required
only by cases that use it.

Two permanent cases invoke the real paired binaries at output level 1:

- `real-e-success` checks a derived copy of `p(a)` and must find E's
  `% Proof found!` marker; and
- `real-e-failure` asks E to derive unrelated `q(a)`, must miss the marker,
  dump the generated TPTP check problem, and report failure.

The native Rust companion probe verified both outcomes. The success trace
reached `% SZS status Unsatisfiable`; the failure emitted the two generated
input clauses and the C-shaped failed summary.

## Deterministic command and rendering coverage

Portable `echo` executable text exercises the same shell-backed `popen`/command
surface without requiring third-party installations:

- E success and failure run at output level 3, pinning the command, fixed-size
  `fgets`-shaped `%>` trace, success-marker scan, failed-problem dump, and
  compressed shared-variable/skolem rendering;
- Otter success scans `-------- PROOF --------`, while failure pins the complete
  prolog-variable header and Otter problem rendering; and
- SPASS success scans `Proof found.`, while failure pins the compact DFG symbol
  list, clauses, time-limit setting, and wrapper.

Only temporary tokens matching a path ending in `epr_` plus exactly six
alphanumeric characters are canonicalized, and only on `% Running`/`%>` trace
lines. Executable text, flags, `<` stdin routing, `2> /dev/null` display text,
trace chunking, and unrelated temporary-looking text remain strict.

Current Otter and SPASS binaries are not installed, and the C integration is
explicitly tied to historical Otter/SPASS versions and command dialects. The
shell adapters therefore provide deterministic process, marker, and exact
problem-format coverage; requiring obsolete third-party installations would
add availability/version noise without testing a different Rust path.

## FOF, compressed input, and shell rejection

The compressed cases reuse external variable `X` across protocol steps with
`ClausesHaveLocalVariables` disabled and force generated skolem units. A
separate formula-parent/formula-target case reaches both C warning sites and
pins two exact `checkproof: Warning: Cannot currently handle full first-order
format!` stderr lines. The Setheo release path avoids an unrelated external
process after check generation. `shell-step-rejection` pins the parser failure
caused by C's default-disabled `SupportShellPCL`.

## Temporary, file, and broken-pipe diagnostics

An isolated missing-input case covers the stable two-line scanner-open shape;
only established complete POSIX/Windows not-found suffixes are canonicalized.
A lower-level regression points `TMPDIR` at a missing directory and pins the
file-error category plus `Could not create valid temporary file name ...
(check $TMPDIR):` contract before any prover starts.

C explicitly calls `OutClose` at normal completion, and Rust retains its exact
stable close diagnostic. A real closed pipe can terminate POSIX C through
`SIGPIPE` before `OutClose`, while Windows reports a write error. The comparison
harness keeps its capture reader open, so manufacturing this condition would
require a host-specific shell/process test. Rust keeps deterministic checked
writes and flushes rather than reproducing signal-dependent termination.

## Reference availability and performance

This sandbox has no visible WSL distribution, compiler, or archived C tools,
so the expanded cases cannot run against C in this session. They remain in the
permanent matrix and will exercise the archived C companion/tools when the
normal user-context reference environment is restored. Source control flow,
native exact regressions, and scoped normalization provide the permitted
evidence-backed decision in the meantime.

The Setheo change removes a process-free unchecked classification in favor of
an equally process-free failure classification. Other runtime code is
unchanged; the harness cases are tiny, so a performance benchmark is not
warranted.

## Validation

- `cargo test --locked --lib --quiet -- --test-threads=1`: 4,130 passed
- `cargo test --locked --bins --quiet -- --test-threads=1`: all binary targets passed
- `eprover_schedule`, `e_stratpar`, and `executable_inventory` integration
  suites: 4, 1, and 1 passed
- `tools/e-interop/test_e_interop.py`: 30 passed
- focused `prover::checkproof::tests`: 18 passed
- focused `pcl2::proofcheck::tests`: 24 passed
- `cargo check --locked --all-targets`: passed
- `cargo clippy --locked --all-targets -- -D warnings`: passed
- `cargo build --locked --release --bin checkproof`: passed
- `cargo fmt --all -- --check`: passed
- C-source documentation coverage: 492 source files across 266 unit docs
- Change Later wording and local-link checks: 269 Markdown files each
- documentation regeneration: preserved manual sections in 268 files
- native real Rust-E success and failure probes: expected verified/failed
  summaries, exit 0
- native E/Otter/SPASS shell adapters: expected marker success and rendered
  failure problems, exit 0
- native FOF warning probe: two exact warnings, failed summary, exit 0
- native shell-step rejection probe: syntax exit 3
