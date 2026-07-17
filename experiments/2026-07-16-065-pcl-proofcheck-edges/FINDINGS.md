# PCL proof-checking compatibility edges

> Historical baseline note: this experiment records the exact C-compatible
> doubled-marker behavior at commit `2b616f7e`. Rust intentionally corrected
> the E marker on 2026-07-17 so real `% Proof found!` output verifies; the
> paired real-E evidence is in
> [`experiment 2026-07-17-001`](../2026-07-17-001-pcl-proofcheck-real-e-marker/FINDINGS.md).

## Question

Does the current `pcl_proofcheck` implementation cover the remaining legacy
external-prover, warning, rendering, process, and bookkeeping semantics?

## Source audit

The generated problem path matches C's deliberately limited proof checker:
`PCLCollectPreconds` copies only clausal parents, warns for FOF parents, and
deduplicates references through the expression precondition tree.
`PCLNegSkolemizeClause` skolemizes a clausal target and inserts one
polarity-flipped hypothesis unit for each target literal. With no clausal
preconditions, the step is accepted by assumption. Split steps return
`CheckNotImplemented`; protocol accounting increments `unchecked` and prints
"assuming true" without incrementing the checked count. Setheo has no C switch
arm, so release C retains the initialized failure result.

The E, Otter, and SPASS paths write temporary problems, construct unquoted
shell command text, read prover stdout through `fgets(line, 180, ...)`, and
search each chunk independently. Trace output prefixes every chunk, truncates
at an embedded NUL through C-string semantics, and does not synthesize a final
newline. Otter stdin redirection and its `$T`/`$F` literals, SPASS's compact DFG
signature and `spass_hack`, and failure problem dumps are all compatibility
visible.

## Doubled E marker

The first current archived comparison,
`.artifacts/e-compare/20260716-224814-945305-tools/`, exposed a previously
missed C bug. `clb_defines.h` defines `COMCHAR` as `"%%"` because it is normally
embedded in `printf` format strings. `pcl_verify_eprover` instead passes
`COMCHAR " Proof found!"` directly to `strstr`, making the actual success token
two literal percent signs. Ordinary E emits one `% Proof found!`, so even a
proved generated problem is reported as failed.

The archived C tool directly confirms both sides: `echo % Proof found!` is
rejected and dumps the problem, while `echo %% Proof found!` is accepted. Rust
now preserves this accidental token. The permanent matrix renames the paired-E
case to make the failure explicit and adds a double-percent success oracle.

## Executable diagnostics

The same first report found two neighboring support-tool boundaries:

- stdin parser diagnostics used Rust's `-` instead of C's `<stdin>`; and
- missing named inputs skipped C's `InputOpen` preflight and said "Cannot open"
  instead of "Cannot stat".

`checkproof` now uses `<stdin>` for scanner diagnostics and the shared pre-open
regular-file boundary while retaining output-file creation before a later
input failure. The final expanded report,
`.artifacts/e-compare/20260716-225409-299711-tools/`, has all 16 cases exact.

## Retained regressions and decisions

The 24 core tests pin generated checks, warning routing, E/Otter/SPASS problem
formats and commands, historical truth encodings, the doubled E marker,
fixed-chunk/NUL/unterminated traces, temporary-file failures, assumptions,
Setheo, splits, and protocol summaries. The 18 executable tests pin options,
signal-compatible setup, output ownership, parser configuration, `<stdin>`,
pre-open `stat`, warnings, and final summaries. Portable shell adapters remain
the deterministic oracle for obsolete Otter/SPASS dialects; requiring those
historical binaries would test availability rather than a different Rust path.

The doubled E token is intentionally retained only for drop-in compatibility.
A future non-compatibility mode should recognize the actual single-percent E
marker and use structured process arguments. FOF clausification, split checking,
and modern external-prover formats remain explicit post-compatibility cleanup,
not unimplemented behavior hidden by this closure.

## Validation

The 24 focused core tests, 18 executable tests, and 32 Python interoperability
tests pass. The final 16-case archived-C differential is exact. Final
repository gates pass formatting, all-target/all-feature checking, pedantic
Clippy with warnings denied, all 4,192 library tests plus binary and integration
targets, and a locked release build of every binary. All four C-source
documentation checks also pass.
