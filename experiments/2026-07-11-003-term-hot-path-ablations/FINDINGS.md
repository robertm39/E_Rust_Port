# Term Hot-Path Ablations

## Question

Can safe Rust representation changes reduce the remaining `LUSK6.lop` proof-search gap without changing C-compatible search behavior?

## Setup

- Baseline commit: `f336e21d` (`Match C release term-bank assertions`).
- Candidate and baseline were built with `cargo build --locked --release --bin eprover` using the same toolchain.
- The exact baseline was built from a detached worktree at `C:\tmp\e-rust-baseline` into `C:\tmp\e-rust-baseline-target`.
- Workload: `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`.
- Arguments: `--auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw --detsort-new`.
- Paired runs alternated baseline/candidate order and read `Process.TotalProcessorTime` after successful exit.

Representative build and run commands:

```powershell
cargo build --locked --release --bin eprover
git worktree add --detach C:\tmp\e-rust-baseline f336e21d
cargo build --manifest-path C:\tmp\e-rust-baseline\Cargo.toml --locked --release --bin eprover --target-dir C:\tmp\e-rust-baseline-target
```

The benchmark harness launched each executable with `System.Diagnostics.Process`, drained stdout/stderr, required exit code zero, and recorded both CPU and wall time.

## Results

| Candidate | Baseline median CPU | Candidate median CPU | Change | Decision |
| --- | ---: | ---: | ---: | --- |
| Borrow term arguments in metadata and recursive rewriting | 5.734 s | 5.891 s | -2.74% | Rejected |
| Borrow term arguments in metadata only | 5.797 s | 5.906 s | -1.88% | Rejected |
| Store splay-tree links in a safe node arena | 5.969 s | 6.047 s | -1.31% | Rejected |

Additional ablations rejected before the strict CPU-time comparison:

- A 32-pair safe inline matcher stack produced a `5.760 s` wall median versus `5.751 s` for its adjacent candidate baseline.
- Borrowed argument traversal across recursive term-bank insertion produced `6.056 s` versus `5.751 s`.
- Borrowed type comparison plus release-mode matcher assertions produced `6.020 s` versus `5.813 s`.
- A prior term-tree key/type argument-comparison change did not establish a gain.

The combined borrowed-argument prototype initially appeared faster in non-interleaved wall runs (`6.422 s` to `5.813 s`). Alternating exact-baseline CPU runs falsified that result, showing why the prototype was not retained.

The successful Rust search trace remained unchanged across retained-behavior candidates:

- 5,305 processed clauses.
- 129,610 generated clauses.
- 549,877 rewrite steps.
- 2,691,308 term-top insertions.

For scale, the reference C run completed in about `1.16 s` user CPU, `1.27 s` wall time, and `119,360 KiB` maximum resident memory, with 4,897 processed clauses, 122,867 generated clauses, 518,389 rewrite steps, and 2,548,241 term-top insertions.

## Falsification Checks

- Used an exact detached baseline after discovering that the older `target/head_check` snapshot predated `f336e21d` and still contained release assertions.
- Alternated execution order to reduce warm-cache and background-load bias.
- Used process CPU time rather than only wall time.
- Required successful prover exit on every measured run.
- Ran focused term-tree, term-cell-store, term-bank, matcher, and rewrite tests while evaluating the corresponding prototypes.

Windows Performance Recorder could not start because the host lacks the required system-performance profiling policy. WSL profiling was also unavailable because the installed Ubuntu 24.04 distro does not currently have Cargo. These limits prevent attribution below the measured end-to-end candidates.

## Conclusion

None of the candidates produced a repeatable end-to-end improvement, so no production optimization from this experiment was retained. The C-compatible intrusive splay tree and cloned argument traversal remain in place. Future work should profile with sampling support before changing shared-term representation; the roughly 4.8x C/Rust CPU gap is not explained by these isolated clone/link costs.
