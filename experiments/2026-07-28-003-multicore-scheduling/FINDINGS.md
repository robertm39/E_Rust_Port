# Multicore scheduling lifecycle and reproducibility

## Question

Can Umlaut's isolated process portfolio stop promptly on limits, cancellation,
or a winning result; contain worker crashes; leave no descendants or temporary
files; apply the requested 128 GiB Linux limit to every worker; account for
nested CPU use; and reproduce explicitly seeded proofs?

This experiment addresses Bead `E_Rust_Port-9jt.8.4`.

## Required-host result

The first required high-memory start on 2026-07-28 was rejected by Linode's
account plan limit before instance creation. The controller deleted firewall
`92115053`; no Linode existed and no high-memory usage accrued. That historical
failure established provider gate `E_Rust_Port-9jt.8.7` and motivated the
normal fallback retained below.

The guarded retry succeeded on 2026-08-01. It created
`g7-highmem-8` runner `e-rust-codex-260801-024609-61a5` (Linode `101953078`,
firewall `98864782`) in `us-ord`. The source snapshot SHA-256 was
`71de3ebb25e1bdf56922a9a5e83c3861e1282a1d9b1d41570f643495ddf91609`
at root commit `1b014a78658e213667def139ebaebb51d3e18d7e`. The host reported
Ubuntu 24.04.4 LTS, kernel `6.8.0-134-generic`, eight logical CPUs `0-7`, one
eight-core/one-thread-per-core socket, AMD EPYC 7713, and 154,517,244 KiB of
memory.

The release binary was preserved from source commit `4e87dac3` with SHA-256
`8c093b91e7e0de5f37d2f8066199f9b57aaea3a1041f9fa9eb21d116ae1decda`.
Every observed process inherited the exact requested 128 GiB address-space
limit of 137,438,953,472 bytes. Two independent corrected controller runs
passed all 12 checks:

| Replication | Timeout | Cancellation | Killed worker | Seeded proofs | Survivors/files |
| --- | --- | --- | --- | --- | --- |
| `v2` | exit 8; 17.987 s; 5 processes | exit 14; 18.432 s; 5 processes | exit 9; 30.872 s; 5 processes | 4/4 exit 0 and byte-identical within the run | 0/0 in every case |
| `v3` | exit 8; 17.789 s; 5 processes | exit 14; 18.528 s; 4 processes | exit 9; 30.943 s; 3 processes | 4/4 exit 0 and byte-identical within the run | 0/0 in every case |

The first high-memory controller result passed 11 of 12 checks. Killing one
worker left no descendants or files, all remaining workers reached their CPU
limits, and the parent cleanly reported schedule exhaustion, SZS `GaveUp`, and
`ResourceOut` exit 9. The old `worker_crash_recovered` check nevertheless
required another worker to prove the theorem. The corrected
`worker_crash_contained` check accepts either a proof or that terminal clean
exhaustion, but only when it also finds the killed worker's recorded `-1`
status. Five focused tests cover both accepted outcomes and three rejection
boundaries. The corrected controller SHA-256 is
`3b64503fe73101d0247d124b03e6c543265775296c3daff8300f945610ec69b9`.

Within each controller invocation, the four explicitly seeded proof objects
are byte-identical. The `v2` and `v3` proof hashes differ because the proof
contains the randomized temporary source path chosen for that invocation;
cross-path byte identity is not claimed. Resource footers were present and
monotonic in both runs. The compact tracked evidence is
`high-memory-summary.json`. The ignored raw bundle is
`.artifacts/experiments/2026-07-28-003-multicore-scheduling/multicore-high-memory-003-evidence.tar.gz`
(7,083 bytes, SHA-256
`ba3500d1d8d62fc0d6704085b85d68a941f6aa66b833767e0cdc2d7f72bee1f2`).

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

Reproduce the accepted gate on a guarded high-memory runner with:

```text
python3 experiments/2026-07-28-003-multicore-scheduling/stress_multicore_schedule.py \
  --umlaut target/release/umlaut \
  --output /path/to/multicore-high-memory.json \
  --cores 8 \
  --expected-host-cpus 8 \
  --memory-limit-mb 131072 \
  --iterations 4
```

## Historical normal fallback evidence

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

The accepted `g7-highmem-8` run matches the documented proxy's eight dedicated
CPU allocation, approximately 150 GiB host-memory class, Ubuntu process model,
eight-core request, and per-process 128 GiB limit. It still is **not** an exact
competition reproduction: its Ubuntu patch level, kernel, hypervisor, exact
CPU, job wrapper, signal timing, and accounting differ, and it has no external
StarExec aggregate cgroup. A real StarExec job and the future CASC-2027
contract remain separate gates. The historical normal fallback differs
further by having only four CPUs and 8 GiB physical memory.

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
limit defects. The successful guarded provisioning resolves
`E_Rust_Port-9jt.8.7`; two passing corrected replications on the required
8-CPU/150 GiB proxy satisfy `E_Rust_Port-9jt.8.4`. No production Rust change
was needed during the final retry; only the experiment controller's crash
classification and focused tests changed.
