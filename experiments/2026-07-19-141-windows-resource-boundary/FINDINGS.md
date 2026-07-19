# Windows resource-boundary diagnosis

## Question

Can the remaining `SWV851-1.p` allocator abort at the maintained 60-second
CPU and 2 GiB data limits be reconciled with C by polling the Windows process
CPU clock inside long paramodulation loops?

## Setup

- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Rust baseline: commit `3aad1207` (`Sort sparse clause slots in place`).
- Shared proof arguments: `--auto --silent --cpu-limit=60
  --memory-limit=2048 --detsort-rw --detsort-new --proof-object=1`.
- Focused corpus: `.artifacts/e-corpus/diversity-scratch-139/resource_swv`.

The prior full report at
`.artifacts/e-compare/20260719-144224-592472/` records the maintained failure:
Rust aborts in the allocator while C returns exact `ResourceOut` output and
exit 8. An isolated repeat at
`.artifacts/e-compare/20260719-145336-503287/` fails on a 168-byte request.

## Tested interventions

### Emergency allocation reserve

A safe-Rust reserve was allocated before proof search and released near the
CPU deadline. This failed earlier, at 59.60 seconds, on the same 168-byte
request in `.artifacts/e-compare/20260719-151624-776092/`. Releasing reserved
memory cannot increase the Job Object quota; it only moves existing committed
pages between owners. The change was reverted.

A custom global allocator was not implemented. The repository's unsafe-code
policy permits narrowly scoped external shared-library interop, not a general
allocator replacement.

### Cooperative paramodulation polling

A Windows-only check sampled the process CPU clock every 256 long-loop
iterations in both indexed paramodulation directions. Once latched, proof
control discarded partial generated clauses and allowed the saturation loop
to report the normal hard resource outcome.

Polling at the exact deadline still aborted on a 40-byte allocation at 62.14
seconds in `.artifacts/e-compare/20260719-152709-386083/`. Polling 250 ms
before the deadline still aborted on a 168-byte allocation at 62.20 seconds
in `.artifacts/e-compare/20260719-153342-604417/`. The complete polling patch
was reverted.

## Diagnostic quota result

The same candidate and proof were run with a 4 GiB memory request while the
60-second CPU limit remained unchanged. The comparison at
`.artifacts/e-compare/20260719-153941-718310/` is exact: C returns
`ResourceOut`/8 after 60.11 wall seconds and Rust returns `ResourceOut`/8
after 63.17 wall seconds. This proves that the cooperative deadline and output
path work when memory is available.

During a separate unrestricted diagnostic run, native process sampling near
the deadline recorded:

| Process CPU | Private bytes | Working set |
| ---: | ---: | ---: |
| 46.09 s | 1,720,524,800 | 1,687,793,664 |
| 59.44 s | 2,434,994,176 | 2,370,170,880 |

The latter private-byte value is about 2.27 GiB. The maintained Windows
mapping translates C's 2 GiB `RLIMIT_DATA` request to a 2.25 GiB whole-process
Job Object quota. The Rust search therefore reaches that quota immediately
before its 60-second process-CPU deadline. This is a late-search live-set gap,
not a missing hard-timeout implementation.

## Decision

Reject the emergency reserve and cooperative early-deadline changes. A larger
time slack would redefine the user-requested CPU limit and could suppress
proofs that C finds during the final second. Raising the Job Object allowance
would conceal a memory regression that was absent in the exact experiment-136
SWV runs. Keep the existing CPU and memory semantics and reduce the live proof
state by at least the measured boundary gap instead.
