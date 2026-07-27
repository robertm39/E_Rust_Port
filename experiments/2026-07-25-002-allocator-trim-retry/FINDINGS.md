# Experiment 303: Trim glibc after allocator-cache flush

## Status

Complete and rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

When a Linux allocation fails under E's configured `RLIMIT_DATA`, can returning
the Rust exact-size cache to glibc and calling `malloc_trim(0)` make the
existing one-time `System` retry succeed, so BOO020 and SWV851 reach the
configured CPU-resource result without raising the 2 GiB memory limit?

## Candidate

On glibc Linux only, call `malloc_trim(0)` after `flush_free_lists()` and
before the existing one permitted allocation retry. The ordinary allocation,
cache-hit, cache-miss, deallocation, and non-Linux paths are unchanged.

## Setup

- Ephemeral Linode run: `260725-170858-e1c1`
- Synced source snapshot:
  `db321bca1ac553edac17bfa8dda1de3d5e132c135b91fc00b020a454f95e975f`
- Build: locked fat-LTO release `eprover`
- Focused allocator tests: 4 passed
- Problems and flags:
  - `eprover/EXAMPLE_PROBLEMS/SMOKETEST/BOO020-1.p`
  - `eprover/EXAMPLE_PROBLEMS/TPTP/SWV851-1.p`
  - `--auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw`
    `--detsort-new --proof-object=1`
- Raw focused artifacts:
  `.artifacts/experiments/2026-07-25-002-allocator-trim-retry/experiment-303/`

Commands, run from the repository root:

```powershell
.\linode-runner.ps1 up
.\linode-runner.ps1 sync
.\linode-runner.ps1 exec cargo fmt --all -- --check
.\linode-runner.ps1 exec cargo test --locked --all-features size_class_allocator
.\linode-runner.ps1 exec cargo build --locked --release --bin eprover --manifest-path /opt/e-rust-port/source/Cargo.toml
.\linode-runner.ps1 exec python3 /opt/e-rust-port/source/experiments/2026-07-25-002-allocator-trim-retry/remote_resource_check.py --binary /opt/e-rust-port/source/target/release/eprover --source-root /opt/e-rust-port/source --output-dir /opt/e-rust-port/artifacts/experiment-303
.\linode-runner.ps1 down
```

## Results

| Problem | Exit | Wall (s) | Peak child RSS (KiB) | ResourceOut | stderr |
| --- | ---: | ---: | ---: | --- | --- |
| BOO020-1.p | -6 (`SIGABRT`) | 61.667 | 1,901,704 | no | `memory allocation of 139264 bytes failed` |
| SWV851-1.p | -6 (`SIGABRT`) | 35.592 | 1,997,348 | no | `memory allocation of 2048 bytes failed` |

`resource.getrusage(RUSAGE_CHILDREN).ru_maxrss` is process-wide and
monotonic, so the second row reports the maximum across both completed child
processes. This does not affect the exit-path conclusion.

Compared with the prior untrimmed candidate, BOO020 survived until the CPU
boundary instead of aborting earlier, but it still aborted in Rust's allocation
error handler and emitted no normal E resource result. SWV851 remained an
allocator abort. The retry therefore does not repair either maintained
compatibility case.

## Falsification checks and limits

- Both cases use the same locked optimized production profile and exact limits
  as the comprehensive Linux comparison matrix.
- BOO020 demonstrates that trim can free enough address space to alter timing;
  its eventual allocation abort falsifies the stronger recovery hypothesis.
- SWV851 falsifies the hypothesis independently with a much smaller failed
  request.
- This focused run does not estimate ordinary proof-search performance because
  the candidate hook is reached only after an allocation has already failed.
- Peak RSS is not a direct measurement of glibc arena fragmentation, so the
  experiment rejects the proposed fix but does not prove a single allocator
  root cause.

## Decision

Reject and restore `src/basics/size_class_allocator.rs`. `malloc_trim(0)` can
change failure timing but cannot make a Rust infallible allocation recover
reliably beneath the exact Linux limit. Continue at the control-flow layer:
the prover needs to detect exhausted CPU/memory headroom before entering an
infallible allocation, or otherwise route allocation exhaustion into E's
normal resource-exit path without unwinding through `GlobalAlloc`.
