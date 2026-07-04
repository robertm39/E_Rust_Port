<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_signals

## Source Files

- [INOUT/cio_signals.h](../../../eprover/INOUT/cio_signals.h)
- [INOUT/cio_signals.c](../../../eprover/INOUT/cio_signals.c)

## Purpose

Signal handler for limit signals...not really necessary, but may work around some Solaris bugs. Also some support infrastructure... the GNU Lesser General Public License. <1> Fri Nov 6 14:50:28 MET 1998

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CCO_SIGNALS`

### Globals

- `extern bool SilentTimeOut`
- `extern rlim_t HardTimeLimit`
- `extern rlim_t ScheduleTimeLimit`
- `extern rlim_t SoftTimeLimit`
- `extern rlim_t SystemTimeLimit`
- `extern sig_atomic_t SigTermCaught`
- `extern sig_atomic_t TimeIsUp`
- `extern sig_atomic_t TimeLimitIsSoft`

### Exported Functions

- `void ESigTermSchedHandler(int mysignal)`
- `void ESignalHandler(int mysignal)`
- `void ESignalSetup(int mysignal)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ESignalSetup`: Set up ESignalHandler() as handle for mysignal, check for errors.
- `ESignalHandler`: Handle signals...print message and exit or continue, depending on the signal.
- `ESigTermSchedHandler`: Record a caught SIGTERM.

### Dependencies

- `"cio_signals.h"`
- `<cio_tempfile.h>`
- `<signal.h>`
- `<sys/resource.h>`
- `<sys/time.h>`
- `<sys/types.h>`

### Compile-Time Conditions

- `CCO_SIGNALS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_signals.h`, `INOUT/cio_signals.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 264 lines, 11 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Signal handler for limit signals...not really necessary, but may work around some Solaris bugs. Also some support infrastructure... the GNU Lesser General Public License. <1> Fri Nov 6 14:50:28 MET 1998
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `ESignalHandler(SIGXCPU)` uses the first CPU-limit signal as a graceful soft timeout by setting `TimeIsUp`, resetting `RLIMIT_CPU` to the captured system hard limit, and rearming the hard limit, but a hard timeout writes diagnostics and exits directly from the signal path. Rust now installs a Linux `SIGXCPU` trampoline in normal builds, captures the system hard CPU limit during setup, mirrors the soft-timeout reset/rearm sequence, keeps test builds non-mutating, exposes a non-signal finalizer for the hard-timeout `ResourceOut` banner/SZS output, stderr diagnostic, CPU-limit exit status, and unexpected-signal warning text, maps cooperative hard-deadline expiry onto that hard-timeout result path for executable proof search, preserves the direct banner before buffered normal stdout for supported no-output-file runs, and uses the same cooperative process-CPU deadline on native Windows instead of a Job Object process-time quota that would terminate the process before C-shaped output can be emitted. Exact C-style hard-limit process shutdown and signal-time descriptor writes still belong in the platform shutdown abstraction.
- `ESigTermSchedHandler(SIGTERM)` increments `SigTermCaught` and immediately restores the default `SIGTERM` handler so a later termination signal is no longer swallowed by the scheduler handler. Rust now exposes that scheduler-handler outcome, including the post-increment count and default-reset attempt, while confining the real `signal(SIGTERM, SIG_DFL)` call to normal Linux builds.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- The C handler performs high-level output, error reporting, temp-file cleanup, `setrlimit`, and `raise` work from inside signal handling. Rust has moved the soft-timeout signal latch/reset/rearm path, hard-timeout output/status shape, and unexpected-signal warning text closer to C, but after drop-in compatibility is secured, consider moving the remaining hard-timeout and termination-signal logic behind explicit shutdown control flow or a smaller async-signal-safe boundary.
- Hard CPU-timeout output bypasses stdio buffering via `WriteStr(GlobalOutFD, ...)`, so its banner can appear before already-buffered normal output and uses the literal doubled `COMCHAR` string in the default build. Rust now preserves the hard-timeout status, exit code, doubled-prefix text, stderr diagnostic, and no-output-file stdout ordering for cooperative hard-deadline stops; exact process-exit and signal-time descriptor behavior remain compatibility details to revisit once global-output ownership is centralized.
- Native Windows process-time Job Object limits report expiry as `STATUS_QUOTA_EXCEEDED`, bypassing E's hard-timeout banner, SZS status, stderr diagnostic, and exit-code mapping. Rust therefore keeps Windows CPU limits cooperative for compatibility; after drop-in parity is secured, revisit whether an optional native kill-switch is useful for containment-only runs.

<!-- END MANUAL REVIEW: c_source_docs -->
