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

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for bounded non-POSIX allocation failure on 2026-07-21.

Source files reviewed: `INOUT/cio_signals.h`, `INOUT/cio_signals.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 264 lines, 11 scanned public declarations, 0 scanned internal function definitions, and 3 structured function-comment blocks.
- Signal handler for limit signals...not really necessary, but may work around some Solaris bugs. Also some support infrastructure... the GNU Lesser General Public License. <1> Fri Nov 6 14:50:28 MET 1998
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `ESignalHandler(SIGXCPU)` uses the first CPU-limit signal as a graceful soft timeout by setting `TimeIsUp`, resetting `RLIMIT_CPU` to the captured system hard limit, and rearming the hard limit, but a hard timeout writes diagnostics and exits directly from the signal path. Rust now installs a Linux `SIGXCPU` trampoline in normal builds, captures the system hard CPU limit during setup, mirrors the soft-timeout reset/rearm sequence, keeps test builds non-mutating, exposes a non-signal finalizer for the hard-timeout `ResourceOut` banner/SZS output, stderr diagnostic, CPU-limit exit status, and unexpected-signal warning text, maps cooperative hard-deadline expiry onto that hard-timeout result path for executable proof search, and records the active executable output file descriptor for signal-time `GlobalOutFD` writes. The normal Linux hard-timeout trampoline writes the C banner/SZS and fatal stderr text through descriptors, then drains an allocation-free atomic snapshot of the bytes pending in Rust's 8-KiB C-stdio-compatible output buffer before exiting with `CPU_LIMIT_ERROR`. This reproduces libc `exit()` flushing C's private `FILE *` buffer even though libc cannot see Rust's private `Vec`. Native Windows still uses the cooperative process-CPU deadline instead of a Job Object process-time quota that would terminate the process before C-shaped output can be emitted.
- `ESignalHandler(SIGTERM|SIGINT)` performs temp-file cleanup once, resets the active handler to `SIG_DFL`, and re-raises the same signal so ordinary process termination continues. Rust now preserves the testable cleanup outcome separately, and the normal Linux signal trampoline performs the default-handler restoration and re-raise after that outcome is produced.
- `ESigTermSchedHandler(SIGTERM)` increments `SigTermCaught` and immediately restores the default `SIGTERM` handler so a later termination signal is no longer swallowed by the scheduler handler. Rust now exposes that scheduler-handler outcome, including the post-increment count and default-reset attempt, while confining the real `signal(SIGTERM, SIG_DFL)` call to normal Linux builds.

### Compatibility Evidence

- Retained WSL-native logs under `.artifacts/experiments/2026-07-13-005-hen011-divergence/`, created after commit `e11b51e9` installed the Linux hard-limit trampoline, record actual `SIGXCPU` delivery with the exact doubled-comment failure banner, `SZS status ResourceOut`, fatal stderr text, exit status 8, and approximately 60 seconds of user CPU. Experiment 305 corrects the remaining native-Linux exit-flush gap exposed by `SWB008+1.p`: the focused Rust run now emits the exact 430-byte C reference stdout, including the direct failure bytes before the pending ordinary stdout, with SHA-256 `e697408814db9c024e69d7678eaf6bf109357a7d3500d928f65c34e0124717fc`. Comprehensive Linode run `.artifacts/linode/260725-190227-18ae/` then reports zero unexpected mismatches across the 50-case main matrix, with matching `SWB008+1.p` normalized output, `ResourceOut`, and exit 8.
- The same-tree comparison `.artifacts/e-compare/20260717-002556-450711/comparison.json` matches actual C `SIGXCPU` expiry on Linux to Rust's cooperative Windows process-CPU deadline for `SWB008+1.p` and `SWV851-1.p`: both sides report silent `ResourceOut`, exit status 8, equal normalized output, and no mismatches.
- Experiment 309 audits a native-Linux race exposed after term-store GC startup work was removed. Rust's independent cooperative hard-deadline finalizer could begin writing the terminal banner just before the kernel delivered `SIGXCPU`, allowing the trampoline to write the same banner and diagnostic again. Native non-test Linux now transfers cooperative hard expiry to the installed `SIGXCPU` handler with `raise`, leaving the trampoline as the single owner of direct failure output, pending-buffer replay, diagnostic output, and libc exit. Test and non-Linux builds retain the cooperative return path. The focused maintained case restores C's exact 430-byte stdout and 46-byte stderr, and comprehensive run `.artifacts/linode/260725-231530-96af/` returns the 50-case main matrix to zero unexpected differences.
- No retained artifact live-injects `SIGTERM`/`SIGINT` into the normal Linux trampoline. Rust's cleanup-once, outcome, scheduler-latch, and default-reset behavior are source-aligned and deterministically tested, but live delivery is not claimed. Moving the remaining high-level work behind a smaller async-signal-safe boundary remains the post-compatibility item below.

### Rust Port Status Notes

- `src/inout/signals.rs` ports the public signal globals, setup/handler/scheduler-handler outcomes, CPU-limit configuration and deadline checks, soft-timeout latch and rearm behavior, hard-timeout finalizer, unexpected-signal warning text, termination cleanup outcome, Linux normal-build signal trampolines, and test-only non-mutating paths. The executable proof-search path uses the cooperative hard-deadline finalizer where native signal delivery is not available or would bypass C-shaped output. The ordinary non-POSIX allocation lookahead is capped at 5% of the configured CPU window so a one-second limit is not consumed at startup. A separate immediate latch is available only when an active configured deadline exists; bounded non-POSIX insertion paths use it after a fallible capacity reservation is rejected, allowing normal hard-limit `ResourceOut` finalization instead of an allocator abort.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- The C handler performs high-level output, error reporting, temp-file cleanup, `setrlimit`, `raise`, and `exit` work from inside signal handling. Rust has moved the soft-timeout signal latch/reset/rearm path, Linux hard-timeout descriptor output/process exit, termination default-reset/re-raise, and unexpected-signal warning text closer to C, but after drop-in compatibility is secured, consider moving these shutdown effects behind explicit control flow or a smaller async-signal-safe boundary.
- Hard CPU-timeout output bypasses stdio buffering via `WriteStr(GlobalOutFD, ...)`, so its banner can appear before already-buffered normal output and uses the literal doubled `COMCHAR` string in the default build. Rust now preserves the hard-timeout status, exit code, doubled-prefix text, stderr diagnostic, cooperative hard-deadline ordering, and normal Linux signal-time descriptor writes through the recorded executable output fd. A fixed atomic mirror publishes only complete output-buffer appends, lets the trampoline copy committed bytes in bounded stack chunks, and avoids borrowing the interrupted mutable `Vec`; keep this compatibility boundary isolated so later output ownership can avoid signal-time formatted shutdown work.
- Native Windows process-time Job Object limits report expiry as `STATUS_QUOTA_EXCEEDED`, bypassing E's hard-timeout banner, SZS status, stderr diagnostic, and exit-code mapping. Rust therefore keeps Windows CPU limits cooperative for compatibility; after drop-in parity is secured, revisit whether an optional native kill-switch is useful for containment-only runs.

<!-- END MANUAL REVIEW: c_source_docs -->
