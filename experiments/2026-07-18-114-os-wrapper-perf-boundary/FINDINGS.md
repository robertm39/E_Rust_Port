# OS-wrapper and performance-counter boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.24`. The Rust OS-wrapper now preserves
the unchanged C resource-query and optional performance-counter contracts at
their production owners, with explicit platform decisions for native Windows.
The vendored C checkout remains unchanged.

## Performance counters

An isolated copy of the pinned C source was built under WSL with
`BUILDFLAGS=-DINSTRUMENT_PERF_CTR`. C's `eprover` defines and prints exactly 13
counters. Rust previously printed seven additional Rust-only phase counters and
started `SatTimer` inside propositional SAT checks. Those extra names have been
removed. Rust now prints the exact C names and order, and `SatTimer` wraps the
main saturation owner after presaturation, matching C's entry/exit sites.

C's default `GETTIME` is `GetUSecClock`, which measures process CPU time, and
its macros keep one overwriteable start slot per counter. Rust's feature-gated
guards now use the ported process CPU clock and one atomic start slot per
counter, while retaining RAII cleanup and a safe non-instrumented zero-cost
surface.

[`compare_perf_counters.py`](compare_perf_counters.py) sends `LUSK6.lop` to
both instrumented executables through stdin. The retained
[`comparison.json`](comparison.json) records the executable and fixture hashes,
successful proof and exit behavior, empty stderr, and the exact 13-name output
surface. Both sets of values are nonnegative and both saturation timers are
positive. Numeric durations are deliberately not compared across Linux and
Windows: the native Windows process-time API exposed 15.625-millisecond
quantization in this run, while Linux provided finer resolution.

## Resource and file wrappers

Linux resource usage prefers `getrusage` and retains `/proc/self/stat` plus
`/proc/self/status` as a fallback. The fallback now obtains its process-clock
rate from `sysconf(_SC_CLK_TCK)` instead of assuming 100 ticks per second. The
hard-coded selector remains a documented Linux/glibc compatibility boundary,
along with the other `sysconf` selectors in this module.

The duplicated `RLIMIT_DATA` memory-limit call is retained because unchanged C
labels its second call `RLIMIT_AS` but still passes `RLIMIT_DATA`. Exact
resource-limit diagnostics, status, timeout, and footer behavior were already
captured in
[`experiment 107`](../2026-07-18-107-resource-limit-ownership/FINDINGS.md).

Native Windows keeps the memory Job Object quota but no longer exposes an
unused hard CPU Job Object quota. The executable's cooperative CPU deadline is
the intentional platform contract because a process-time Job quota can
terminate the process before the C-shaped timeout banner, SZS status,
diagnostic, and exit code are emitted. Twenty native unit tests cover the
OS-wrapper surface. The direct C `SecureFOpen` owner audit finds exactly three
call sites, all using mode `"w"`, which the safe Rust wrapper supports.

[`audit_os_wrapper.py`](audit_os_wrapper.py) retains these source-owner checks
in [`owner-audit.json`](owner-audit.json), including the exact counter surface,
saturation owner, CPU-clock/start-slot shape, Windows quota decision, Linux
resource fallback, duplicated memory limit, and file-open callers.

## Reproduction

The instrumented C build lives outside the workspace at
`/home/rober/.cache/e-rust-port/experiments/114-os-wrapper-perf/c-source` and
was produced from pinned commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0`:

```powershell
wsl.exe -d Ubuntu-24.04 --exec make `
  -C /home/rober/.cache/e-rust-port/experiments/114-os-wrapper-perf/c-source `
  -j2 BUILDFLAGS=-DINSTRUMENT_PERF_CTR

cargo build --locked --release --bin eprover `
  --features instrument-perf-ctr --target-dir target\perf-reference

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-114-os-wrapper-perf-boundary\compare_perf_counters.py `
  --c-exe /home/rober/.cache/e-rust-port/experiments/114-os-wrapper-perf/c-source/PROVER/eprover `
  --rust-exe target\perf-reference\release\eprover.exe `
  --output target\os-wrapper-perf-comparison-check.json `
  --expected experiments\2026-07-18-114-os-wrapper-perf-boundary\comparison.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-114-os-wrapper-perf-boundary\audit_os_wrapper.py `
  --output target\os-wrapper-owner-audit-check.json `
  --expected experiments\2026-07-18-114-os-wrapper-perf-boundary\owner-audit.json
```
