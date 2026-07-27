# Experiment 309: Hard-timeout single emission

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`.

## Question

Can native Linux route cooperatively detected hard CPU deadlines through the
already-installed `SIGXCPU` trampoline so the failure banner, fatal
diagnostic, pending-output replay, and exit have one owner even when the
kernel limit expires at the same boundary?

## Baseline

Experiment 308's comprehensive run
`.artifacts/linode/260725-223007-9e34/` has one unexpected main-prover
difference. On `SWB008+1.p`, both C and Rust return exit 8 and `ResourceOut`,
but Rust emits the hard-timeout banner and stderr diagnostic twice. Rust's
normalized stdout contains the duplicate before the same pending
preprocessing and search text that C emits once.

The Rust saturation loop can observe its process-CPU deadline and enter
`finalize_hard_time_limit_stop`. That cooperative finalizer writes directly to
the configured output and stderr. A kernel `SIGXCPU` delivered during this
short finalization window enters the native trampoline, writes the same two
records, replays the pending output mirror, and exits. The Experiment 308
speedup shifted `SWB008+1.p` into that race window.

Raw baseline evidence:

```text
.artifacts/linode/260725-223007-9e34/validation-summary.json
.artifacts/linode/260725-223007-9e34/compatibility/main/20260725-224018-507033/comparison.json
.artifacts/linode/260725-223007-9e34/compatibility/main/20260725-224018-507033/mismatches/0025/candidate.stdout
.artifacts/linode/260725-223007-9e34/compatibility/main/20260725-224018-507033/mismatches/0025/candidate.stderr
```

## Candidate

On native non-test Linux, cooperative hard-timeout detection raises
`SIGXCPU` instead of independently formatting the report. The installed
trampoline remains the sole owner of the C-shaped direct banner, diagnostic,
atomic pending-output replay, and libc exit. POSIX blocks another delivery of
the same signal while its handler is active, eliminating the competing
finalizers without adding a hot-path branch or lock.

If `SIGXCPU` is inherited as blocked or `raise` cannot deliver it, the helper
falls through to the same direct finalizer and exits before a pending delivery
can run. Unit-test builds and non-Linux builds retain the cooperative
return-based finalizer.

A native-Linux integration regression launches the real executable eight
times with an immediate hard CPU limit and requires exactly one stdout banner,
one stderr diagnostic, and exit 8 on every process.

`eprover/INOUT/cio_signals.c` is the source reference and remains unchanged.

## Setup and exact commands

Focused validation used fresh dedicated worker
`e-rust-codex-260725-230501-2bf0` with Rust 1.97.1 and immutable source
snapshot
`3bc2dc9ecb5bef5a327eddd0f6aa58c2a3ae4ed7981f9a2d0848cd31cfc710f9`.
The exact controller commands were:

```powershell
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-008-hard-timeout-single-emission/remote_validate.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-309
}
finally {
    .\linode-runner.ps1 down
}
.\linode-runner.ps1 run
```

## Results

The focused gates pass:

- Rustfmt;
- the native executable integration regression, covering eight immediate
  hard-limit child processes;
- the existing return-based cooperative hard-limit unit regression;
- strict all-target/all-feature pedantic Clippy;
- the locked release `eprover` build; and
- a clean same-tree FOL C reference build.

The exact maintained `SWB008+1.p` comparison then runs both executables with
the main matrix's auto/silent, 60-second CPU, 2-GiB memory, deterministic-sort,
and proof-object options. C and Rust both exit 8. Their raw stdout and stderr
compare byte-for-byte:

- stdout is 430 bytes with SHA-256
  `e697408814db9c024e69d7678eaf6bf109357a7d3500d928f65c34e0124717fc`;
- stderr is 46 bytes with SHA-256
  `476a0cb5efc32c241ab84d8e949ec7ae93267a0751a11c04fc4d121a1b14441b`;
- each stdout contains exactly one hard-timeout failure banner; and
- each stderr contains exactly one fatal CPU-limit diagnostic.

This restores the exact stdout hash accepted in Experiment 305 while retaining
Experiment 308's GC candidate.

Fresh comprehensive run `.artifacts/linode/260725-231530-96af/` validates the
exact combined source:

- 4,405 Rust tests across 33 result groups, Rustfmt, strict
  all-target/all-feature pedantic Clippy, and native release builds pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean same-tree FOL and higher-order C references build and pass smoke
  checks;
- the 50-case main matrix reports zero unexpected differences and the one
  declared `sledgehammer.p` output-order difference;
- `SWB008+1.p` has equal normalized output, `ResourceOut`, and exit 8, while
  BOO020 and SWV851 retain their equal bounded resource outcomes;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior, with aggregate Rust/C wall ratio
  `1.1481929570688398x`; and
- smoke Callgrind records `9,610,372` Rust versus `7,590,630` C instructions.

The signal correction adds no measurable deterministic work relative to
Experiment 308's failed-gate run (`9,610,482` Rust instructions). The
lifecycle writes `VALIDATION_COMPLETE`, collects all reports, and deletes its
Linode and firewall.

Raw focused artifacts:

```text
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/integration-test.stdout
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/unit-test.stdout
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/rust-clippy.stderr
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/rust-build.stderr
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/c-build.stdout
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/exit-status.txt
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/reference.stdout
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/candidate.stdout
.artifacts/experiments/2026-07-25-008-hard-timeout-single-emission/output-sha256.txt
```

## Falsification checks and limits

- The real executable regression must pass repeatedly; unit-only state-machine
  coverage cannot reproduce asynchronous delivery.
- Focused `SWB008+1.p` must match C's raw stdout, stderr, status, and one-banner
  count under the maintained 60-second hard limit.
- The complete 50-case main matrix must return to zero unexpected
  differences, while BOO020 and SWV851 retain their expected resource exits.
- All Rust, Clippy, Linux release, Windows GNU x64 compile-only, C reference,
  support-tool, benchmark-behavior, and resource gates must remain green.
- This correction is an output/finalization boundary and is not expected to
  improve the remaining approximately `1.15x` aggregate performance gap.

## Decision

Accept. Native Linux now has one C-shaped hard-timeout finalization owner,
the focused asynchronous boundary is byte-exact, and the full maintained
matrix has zero unexpected differences. The separate whole-prover performance
target remains open.
