# Multicore scheduling lifecycle and reproducibility

## Question

Can Umlaut's isolated process portfolio stop promptly on limits, cancellation,
or a winning result; contain worker crashes; leave no descendants or temporary
files; apply the requested 128 GiB Linux limit to every worker; account for
nested CPU use; and reproduce explicitly seeded proofs?

This experiment addresses Bead `E_Rust_Port-9jt.8.4`.

## Required-host result

The required high-memory start was attempted first on 2026-07-28. The
fixed-EST guard reported a four-hour base allowance, a four-hour bank, no
usage, and eight hours available. It then requested `g7-highmem-8` in
`us-ord`, created firewall `92115053`, and tried to create runner
`e-rust-codex-260728-085118-61c0`.

Linode rejected the instance before creation:

```text
A limit on your account is preventing the deployment of the selected Linode
plan. To request access to the plan, please contact Support and provide the
Linode plan name.
```

The controller deleted the firewall. No high-memory Linode existed and no
high-memory usage accrued. Human/provider gate `E_Rust_Port-9jt.8.7` now
blocks final acceptance. The results below are deliberately labeled a normal
fallback, not the required 8-CPU, 150 GiB proxy.

## Audit and implementation

The audit found three lifecycle defects or gaps:

1. Generic workers received `SIGTERM` only as direct processes. Abrupt
   controller exit, including the allocation-free hard-timeout `_exit`, could
   leave descendants alive.
2. Cleanup waited indefinitely after `SIGTERM`, so an uncooperative child
   could hang portfolio cancellation.
3. Linux `set_memory_limit` reproduced an upstream branch bug and set
   `RLIMIT_DATA` twice. `--memory-limit=131072` therefore left
   `RLIMIT_AS` unlimited.

The accepted implementation:

- starts every generic subprocess in its own POSIX process group;
- arms Linux `PR_SET_PDEATHSIG` with `SIGTERM` in the narrow `pre_exec`
  boundary and closes the parent-exit race with `getppid`;
- sends cleanup signals to the complete process group, allows one second for
  graceful termination, then escalates to group `SIGKILL`;
- sets both Linux `RLIMIT_DATA` and `RLIMIT_AS`, intentionally fixing the
  upstream duplicate-`RLIMIT_DATA` defect;
- preserves explicit fallible allocation probes through a thread-local
  allocator scope, while a failed infallible Linux allocation writes buffered
  prover output plus a memory `ResourceOut` through allocation-free descriptor
  I/O and exits with `OUT_OF_MEMORY` instead of aborting; and
- retains process isolation rather than introducing shared-memory or threaded
  search without performance evidence.

Permanent regressions cover an uncooperative process group, a worker killed by
signal, 16 synchronized dual-success runs, scheduler cancellation, and a real
`SIGALRM` hard exit. The hard-limit test verifies that the parent and every
observed worker inherit a 512 MiB address-space limit and disappear afterward.
An allocator subprocess regression separately proves that fallible reservation
still returns an error at the address-space boundary before an infallible
allocation emits SZS `ResourceOut`, preserves the program name, and exits 2
without Rust's abort/backtrace path.

## Reusable controller

`stress_multicore_schedule.py` is a dependency-free Linux controller. It:

- records OS, kernel, CPU, memory, binary hash, command, `/proc` limits,
  cpusets, per-process RSS, aggregate observed RSS, stdout/stderr hashes, and
  elapsed times;
- requests eight schedule cores and `--memory-limit=131072`;
- injects `SIGALRM` into the parent, `SIGTERM` for cancellation, and `SIGKILL`
  into one worker;
- identifies surviving descendants by the unique problem path in each process
  command line;
- isolates `TMPDIR` for every case and rejects residue;
- parses nested resource footers; and
- repeats a `RandomWeight` proof with explicit seeds `11`, `13`, and `17`.

Reproduce the required final gate only after the provider gate is resolved:

```text
python3 experiments/2026-07-28-003-multicore-scheduling/stress_multicore_schedule.py \
  --umlaut target/release/umlaut \
  --output /path/to/multicore-high-memory.json \
  --cores 8 \
  --expected-host-cpus 8 \
  --memory-limit-mb 131072 \
  --iterations 4
```

## Normal fallback evidence

The retained fallback used runner `e-rust-codex-260728-085138-57e5`
(Linode `101595238`), source snapshot
`76bb6552068fc12f6bc69b9f59cf69439e3e5cf5b0f32468e840c17daaf10ebf`,
Ubuntu 24.04.4 LTS, kernel `6.8.0-134-generic`, four dedicated virtual CPUs
`0-3`, 8,130,772 KiB host memory, and an AMD EPYC 9845 CPU model. The release
binary SHA-256 was
`a1db57252733293995aaf6e7df3b0c0832a6d7219ba243f5750b9dc19567dab3`.

Although the host had four CPUs, the controller deliberately requested an
eight-core schedule to exercise the complete process portfolio. All observed
processes had `Cpus_allowed_list: 0-3`; Umlaut's core counts are scheduling
reservations and do not establish OS affinity.

| Case | Exit | Elapsed | Processes | Aggregate sampled RSS | Survivors | Temp files |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Parent `SIGALRM` | 8 | 6.271 s | 5 | 42,676 KiB | 0 | 0 |
| Parent `SIGTERM` | 14 | 6.718 s | 5 | 42,876 KiB | 0 | 0 |
| Worker `SIGKILL` | 0 | 20.412 s | 3 | 24,936 KiB | 0 | 0 |

The crash case completed a proof after one worker was killed. Every sampled
parent/worker limit reported exactly `137438953472` bytes for
`Max address space`, proving the 128 GiB option is now applied and inherited.
Nested resource footers were present and their total CPU values were
monotonic. Aggregate sampled RSS is an observation, not a hard group limit:
each process has its own 128 GiB limit, so an external StarExec/job cgroup is
still required to enforce a strict aggregate 128 GiB competition envelope.

All four explicitly seeded runs exited zero and emitted the same proof SHA-256:

```text
3a374ad0997f3018d1cf5e79cd4c6c0cb23d5cdeade0716904b706fe71537bb2
```

Winner labels differed in the nested search stage because the production
policy accepts the first completed successful worker. This timing diagnostic
is not claimed to be deterministic. The proof object—the correctness and
reconstruction contract—was byte-identical. When multiple results are already
ready together, the `BTreeMap` ready-set path uses the lowest descriptor;
the synchronized stress separately requires exactly one complete winner and
clean loser cancellation without pretending OS completion times are ordered.

The raw fallback JSON is retained outside Git at
`.artifacts/experiments/2026-07-28-003-multicore-scheduling/multicore-normal-proxy-final.json`
(22,912 bytes, SHA-256
`92e43ba96fa1dc5c916a898bcff5876ca914b3254843288f586c8faefe922e3a`).
`normal-proxy-summary.json` preserves compact tracked evidence.

## CASC and StarExec comparison

The latest public baseline remains
[CASC-J13](https://tptp.org/CASC/J13/Design.html), not a prediction of
CASC-2027. The code and controller match its public Ubuntu/process-portfolio
shape, eight-core request, 128 GiB configured limit, SZS/proof output,
`SIGALRM`/`SIGXCPU` handling, and no-file-residue requirement.

The fallback does **not** match the competition host: it has four CPUs and
8 GiB physical memory, uses Ubuntu 24.04.4 rather than the published J13
24.04.3 image, and runs no StarExec job wrapper or external aggregate cgroup.
Even the eventual `g7-highmem-8` run will remain a proxy with a different
kernel, hypervisor, exact CPU, job wrapper, signal timing, and accounting
implementation. A real StarExec job and the future CASC-2027 contract remain
separate gates.

## Validation and conclusion

On the normal fallback, Rustfmt, 4,435 library tests, every integration/binary
target, and pedantic Clippy with warnings denied passed. The stress controller
passed all lifecycle, limit, accounting, cleanup, and proof-reproducibility
checks. A later focused Ubuntu gate passed the allocation-failure subprocess
regression, Rustfmt, and strict all-target/all-feature Clippy after the
comprehensive matrix exposed the newly strict 2 GiB `SWV851-1.p` boundary.
That comparison now permits only exit-code and normalized resource-text
differences for this documented `RLIMIT_AS` correction; SZS status, timeout,
output shape, and proof behavior remain mandatory.

Final comprehensive runner `e-rust-codex-260728-103448-52d9` passed Rustfmt,
4,436 library tests plus every integration and binary target, strict pedantic
Clippy, all Linux release binaries, Windows GNU x64 compile-only targets, and
clean FOL/HO reference builds. Its 50-case main matrix has zero unexpected
differences and eight documented differences; `SWV851-1.p` specifically
returns memory `ResourceOut`/2 in 33.50 seconds while E returns time
`ResourceOut`/8 at 60.09 seconds, with equal SZS status and output shape. All
216 support-tool cases have zero unexpected differences and 16 documented
differences. All ten timing cases behavior-match at `1.0879988422x` C, and
smoke Callgrind records 9,604,028 Rust versus 7,590,630 C instructions. The
downloaded artifact is
`.artifacts/linode/260728-103448-52d9/`; its validation-summary SHA-256 is
`62af7a5eec1ba6eb0f8879a063ac7588234093b8090c3ed54c8a6a9165f6958f`.

The implementation is accepted and removes concrete orphan, hang, and memory
limit defects. Bead `E_Rust_Port-9jt.8.4` remains open solely because its
acceptance criteria explicitly require the currently provider-blocked
8-CPU/150 GiB run. Do not substitute this fallback for that gate.
