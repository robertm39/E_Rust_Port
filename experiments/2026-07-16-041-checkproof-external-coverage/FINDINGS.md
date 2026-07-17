# checkproof external coverage

## Status

Completed for Bead `E_Rust_Port-j76.1.33` with a release-parity fix,
real-E companion comparison cases, deterministic legacy-prover adapters,
compressed/FOF/parser/error cases, and evidence-backed platform decisions. The
vendored C source remained unchanged.

## Follow-up correction

Once the archived reference became available, the audit in
[`experiment 065`](../2026-07-16-065-pcl-proofcheck-edges/FINDINGS.md) found
that C passes printf-escaped `COMCHAR` (`"%%"`) directly to `strstr`. The
single-percent E marker described below was therefore a Rust-only success, not
C parity. The permanent cases are now renamed, include a double-percent
success oracle, and are exact. Experiment 065 supersedes the earlier marker
interpretation while retaining this experiment's other decisions.

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

- `real-e-single-percent-marker-failure` checks a derived copy of `p(a)`; E
  proves it, but C `checkproof` misses E's single-percent `% Proof found!`
  marker because it searches for two percent signs; and
- `real-e-failure` asks E to derive unrelated `q(a)`, must miss the marker,
  dump the generated TPTP check problem, and report failure.

The original native Rust companion probe reached `% SZS status Unsatisfiable`
and exposed the then-current Rust-only success. The archived comparison later
showed the C marker bug, which Rust now preserves.

## Deterministic command and rendering coverage

Portable `echo` executable text exercises the same shell-backed `popen`/command
surface without requiring third-party installations:

- E single-percent rejection, double-percent acceptance, and generic failure
  run at output level 3, pinning the command, fixed-size `fgets`-shaped `%>`
  trace, accidental success-marker scan, failed-problem dump, and compressed
  shared-variable/skolem rendering;
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

The archived environment was unavailable during this original slice. It was
restored for experiment 065, whose final 16-case report is exact.

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
- pre-correction native real Rust-E probes: the then-current candidate reported
  verified/failed summaries, exposing the marker difference later corrected
  by experiment 065
- native E/Otter/SPASS shell adapters: expected marker and rendered-problem
  paths, exit 0
- native FOF warning probe: two exact warnings, failed summary, exit 0
- native shell-step rejection probe: syntax exit 3
