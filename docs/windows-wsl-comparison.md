# Building and comparing E on Windows

The upstream C implementation is built as a Linux reference under WSL 2. The
Windows Rust executable is compared with that reference for compatibility.
Performance is measured separately using native Linux builds of both programs
inside the same WSL instance.

The tooling never configures or builds in `eprover/`. It clones the current
upstream commit into `~/.cache/e-rust-port/`, which is on WSL's Linux filesystem.

## One-time setup

The supported distribution name is `Ubuntu-24.04`. On this machine it is
installed for the normal Windows user context, `robert_2023\rober`, and reports
WSL version 2. WSL distributions are registered per Windows user, so sandboxed
Codex commands can report no installed distributions even though normal
PowerShell for `rober` can run the harness. Run the commands below from a normal
`rober` PowerShell prompt; Codex tool runs that need WSL may need approval to run
outside the sandbox in that user context.

Confirm the distro first:

```powershell
wsl --list --verbose
```

If `Ubuntu-24.04` is missing, install it from an elevated PowerShell prompt:

```powershell
wsl --install -d Ubuntu-24.04
```

Restart if requested, launch Ubuntu once, and create the Linux user. Then, from
the repository root in a normal PowerShell prompt, install the C build tools:

```powershell
.\e-interop.ps1 setup
```

The `setup` command installs `build-essential`, `gawk`, `git`, Python 3, and GNU
time. It validates the distribution and compiler after installation.

For Linux Rust benchmarks, also install Rust inside WSL using
[rustup](https://rustup.rs/). The Windows Rust installation cannot produce the
native WSL benchmark binary. The harness supports rustup's default
`~/.cargo/bin` install path, including `/root/.cargo/bin` when the WSL commands
run as root.

## Standard runbook

From the repository root in normal PowerShell:

```powershell
wsl --list --verbose
.\e-interop.ps1 setup
.\e-interop.ps1 build-reference
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
.\e-interop.ps1 benchmark -Runs 5
```

The reference build writes its manifest under the WSL cache, normally
`~/.cache/e-rust-port/reference.json`. Compatibility and benchmark reports are
written under `.artifacts/e-compare/`; benchmark report directories end with
`-benchmark`. A nonzero `compare` exit can mean compatibility mismatches were
found. Inspect the generated report before treating that as a setup failure.

## Build the C references

```powershell
.\e-interop.ps1 build-reference
```

This builds both `eprover` and `eprover-ho`, and archives the support-tool
binaries produced by the normal C build. The cache contains isolated source
trees, binaries, and `~/.cache/e-rust-port/reference.json`, which records the
upstream commit, compiler, distribution, configuration, versions, hashes, and
binary paths. Rerunning the command replaces builds for the same commit.

The command refuses to run if the nested `eprover` Git repository is dirty and
checks it again after the build.

## Compatibility comparison

Once the Rust port provides the required binary target named `eprover`, build
the Windows release executable and compare it:

```powershell
cargo build --locked --release --bin eprover
.\e-interop.ps1 compare -RustExe .\target\release\eprover.exe
```

Use `-Corpus C:\path\to\TPTP` for a different corpus. The default run covers
the bundled smoke, TPTP, LOP, higher-order, stdin, malformed-input, resource
limit, proof-output, and included-axiom paths. Every command receives the same
strategy, deterministic-sort, CPU-limit, and memory-limit options. WSL and
Windows paths, including `TPTP`, are translated independently.

To validate the harness before the Rust executable exists, compare the C
reference with itself:

```powershell
.\e-interop.ps1 compare -SelfTest
```

Compatibility reports are written to `.artifacts/e-compare/<timestamp>/` as
JSON and CSV. Complete stdout, stderr, and normalized output are retained for
each mismatch. A mismatch in exit code, timeout state, SZS status, output
structure, or normalized output makes the command fail.

## Support-tool comparison

After building Rust support tools in release mode, compare their C-shaped help,
supported version, and selected functional command/stdin surfaces against the
archived C support binaries:

```powershell
cargo build --locked --release --bins
.\e-interop.ps1 compare-tools -RustBinDir .\target\release
```

Use `-Tool classify_problem -Tool eground` to restrict the comparison. Use
`.\e-interop.ps1 compare-tools -SelfTest` to compare the archived C tools with
themselves. Tool reports are written beside the main compatibility reports under
`.artifacts/e-compare/<timestamp>-tools/`.

## Performance comparison

```powershell
.\e-interop.ps1 benchmark -Runs 5
```

The benchmark uses WSL Cargo to build `eprover` in release mode. Cargo's target
directory, both binaries, and a copied corpus are kept on WSL's ext4 filesystem;
the benchmark refuses `/mnt/c` artifacts. It performs discarded warmups and
seeded, interleaved C/Rust trials, one process at a time.

Reports contain every sample plus median wall time, median CPU time, maximum
resident memory, per-problem Rust/C ratios, and the aggregate geometric-mean
ratio. Ratios above `1.10` are warnings rather than failures. Override this with
`-RegressionThreshold`, and use `-TimeoutSeconds` or `-MemoryLimitMb` to change
the shared limits.

## Tool tests

Run the standard-library unit tests inside WSL:

```powershell
$repo = wsl -d Ubuntu-24.04 -- wslpath -a $PWD.Path
wsl -d Ubuntu-24.04 -- bash -lc "cd '$repo/tools/e-interop' && python3 -m unittest -v"
```
