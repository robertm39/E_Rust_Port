# Ephemeral Linode compute runner

The repository includes a controller for short-lived Linux build and profiling
workers. Its default is the Akamai Cloud `G8 Dedicated 8x4` plan in Chicago:

- type: `g8-dedicated-8-4`
- region: `us-ord`
- image: `linode/ubuntu24.04`
- resources: 8 GiB RAM, 4 dedicated CPUs, and 82 GiB storage

This worker is the project's sole Rust/C execution environment. Do not run
Cargo, `rustc`, Rust project binaries, the C build, C binaries, WSL, Valgrind,
or Callgrind on the local computer. Local PowerShell only orchestrates the
worker; lightweight Python controller tests, PowerShell parsing, Git, and
documentation checks may remain local.

There are no exceptions for quick smoke tests, focused reproductions, or an
already-installed local toolchain. WSL, local containers, and local virtual
machines are not supported substitutes. Any command that formats, compiles,
links, tests, starts, compares, benchmarks, or profiles the Rust port or C
reference must execute on the Linode.

The normal `run` workflow creates a Cloud Firewall and Linode, installs the
complete Linux and Windows-cross toolchain, uploads a fresh snapshot of the
current worktree, performs every required validation phase, downloads the
artifacts, and deletes the paid resources in a `finally` cleanup.

## One-time account preparation

Create a Personal Access Token in Cloud Manager with only these permissions:

- Linodes: read/write
- Firewalls: read/write
- all other products: no access

Keep Backup Auto Enrollment disabled under **Administration > Account
Settings**. The controller also sends `backups_enabled: false`, but an enabled
account-wide setting overrides that request and incurs a separate backup
charge.

Store the token using Windows DPAPI, which ties the encrypted value to the
current Windows user and machine:

```powershell
$linodeSecretDir = Join-Path $env:LOCALAPPDATA "E-Rust-Port"
New-Item -ItemType Directory -Path $linodeSecretDir -Force | Out-Null
Read-Host "Paste the Linode token" -AsSecureString |
    ConvertFrom-SecureString |
    Set-Content -LiteralPath (Join-Path $linodeSecretDir "linode-token.dpapi")
```

The token is decrypted only into the environment of the controller process. It
is not passed on a command line, stored in the repository, or uploaded to the
Linode.

Generate the dedicated, passwordless Ed25519 key used only for disposable
runners:

```powershell
.\linode-runner.ps1 init
.\linode-runner.ps1 check
```

The key and controller state are stored under
`$env:LOCALAPPDATA\E-Rust-Port\linode-runner`, outside the repository.
The `check` command is read-only: it validates the token scopes, selected plan,
Chicago capacity, Ubuntu image, public source IP, and local OpenSSH tools
without creating a billable resource.

## Comprehensive remote validation

Run the complete required project-validation lifecycle from the repository
root:

```powershell
.\linode-runner.ps1 run
```

The controller detects the machine's current public IPv4 address and creates an
ephemeral firewall that accepts only TCP port 22 from that `/32`. Outbound
traffic is allowed so Ubuntu, Rust, and Cargo dependencies can be installed.
No other inbound traffic is accepted.

All project code runs on native Ubuntu 24.04. The workload:

1. runs Rustfmt, all-target/all-feature tests, pedantic Clippy, and release
   builds for every Rust binary;
2. cross-compiles every Rust binary and test target for
   `x86_64-pc-windows-gnu`, records PE metadata and hashes, and never executes
   a Windows binary;
3. builds disposable FOL and HO copies of the upstream C reference plus all
   support tools without modifying `eprover/`;
4. runs the maintained main-prover and support-tool C/Rust compatibility
   matrices natively on Linux;
5. runs the seeded five-trial native timing benchmark, smoke tests, and
   Callgrind profiles for Rust and C.

Linux is the runtime, behavioral-compatibility, and performance authority.
Windows GNU x64 is compile-only; MSVC and Windows runtime behavior are not
supported validation targets. Generated Windows binaries are inspected and
hashed but never executed through Wine, emulation, or any other mechanism.

Each run uploads a new archive made directly from the current filesystem. It
therefore includes tracked modifications, untracked source files, and the
ignored-but-required `eprover/` checkout. It excludes Git/Dolt metadata,
credentials, agent state, virtual environments, build outputs, prior artifacts,
and Python caches. Nothing relies on a remote Git branch being pushed first.

The retained results are written to:

```text
.artifacts/linode/<run-id>/
```

They include Linux Rust quality-gate logs, FOL/HO C build metadata, Windows GNU
cross-compile logs and PE hashes, main and tool compatibility reports, timing
benchmark samples, native smoke output, raw and annotated Callgrind data, Linux
binary hashes, and instruction summaries.

`VALIDATION_COMPLETE` means every phase ran and its reports were collected.
`SUCCESS` additionally means no unexpected main or support-tool compatibility
difference was found. If real compatibility gaps remain, the command returns
nonzero after writing `COMPATIBILITY_MISMATCHES`; partial and complete artifacts
are still downloaded and the paid resources are still deleted. Benchmark
ratios above the documented threshold remain warnings rather than lifecycle
failures.

If automatic public-IP detection is unsuitable, provide the controller's
current public IPv4 explicitly:

```powershell
.\linode-runner.ps1 run --allow-ip 192.0.2.10
```

## Interactive lifecycle

For work that requires several remote Rust/C commands, keep one runner only as
long as needed. This is the only supported way to issue Cargo or compiled-code
commands interactively. Always put teardown in a PowerShell `finally` block:

```powershell
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && cargo test --locked --all-targets --all-features"
}
finally {
    .\linode-runner.ps1 down
}
```

Run `sync` again whenever the local files change. It replaces the remote source
directory with a fresh immutable upload; no Git pull or remote working-branch
maintenance is involved.

For a focused Rust build and prover smoke run, replace the `exec` command inside
that same guarded lifecycle with:

```powershell
.\linode-runner.ps1 exec -- `
    "cd /opt/e-rust-port/source && cargo build --locked --release --bin eprover && target/release/eprover eprover/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p --auto --silent --cpu-limit=10"
```

For an isolated C reference build and prover smoke run, use:

```powershell
.\linode-runner.ps1 exec -- `
    "cd /opt/e-rust-port/source && python3 tools/linode-runner/linux_compat.py build-reference --repo-root /opt/e-rust-port/source --eprover-commit worktree-snapshot && /root/.cache/e-rust-port/bin/worktree-snapshot/fol/eprover /root/.cache/e-rust-port/sources/worktree-snapshot/fol/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p --auto --silent --cpu-limit=10"
```

These examples are remote `exec` payloads, not commands to copy into a local
shell. Keep the surrounding `try`/`finally` lifecycle and run `down` even when
an `exec` command fails.

If the controller's public IP changes while a runner is active, update only the
SSH firewall rule:

```powershell
.\linode-runner.ps1 refresh-ip
```

Use `status` to compare saved state with the live API:

```powershell
.\linode-runner.ps1 status
```

`down` reads the exact saved resource IDs, fetches each live resource, and
refuses deletion unless its live label exactly matches the saved
`e-rust-codex-` label. It deletes the Linode first, waits for it to disappear,
and then deletes the firewall.

## Failure recovery and stale-resource cleanup

The default `run` behavior deletes resources after success, command failure, or
interruption. If cleanup itself fails, the local state is retained and the
controller prints an urgent instruction to run:

```powershell
.\linode-runner.ps1 down
```

Keeping a failed paid worker requires explicit opt-in:

```powershell
.\linode-runner.ps1 run --keep-on-failure
```

Inspect managed resources older than six hours without changing them:

```powershell
.\linode-runner.ps1 gc
```

After reviewing that dry-run list, delete it with:

```powershell
.\linode-runner.ps1 gc --yes
```

Garbage collection considers only resources whose labels start with
`e-rust-codex-`, excludes the active saved IDs, and requires resources to be at
least one hour old. It should be a recovery mechanism, not the normal cleanup
path.

## Controller-only local validation

The controller and compatibility-report code use only the Python standard
library and the Windows-bundled OpenSSH and `tar` tools. Their lightweight
Python tests may run locally:

```powershell
.\.venv\Scripts\python.exe -m unittest discover `
    -s tools\linode-runner -p "test_*.py" -v
```

The tests pin the requested Linode and firewall settings, remote-only quality
gates, Windows cross-toolchain bootstrap, source-archive exclusions, safe
artifact extraction, stale-resource selection, label-matching deletion guards,
compatibility matrices, report normalization, and disposable C-source
preparation. They do not compile or execute Rust or C.

Current Akamai references:

- [Create a Linode](https://techdocs.akamai.com/linode-api/reference/post-linode-instance)
- [Create a firewall](https://techdocs.akamai.com/linode-api/reference/post-firewalls)
- [Delete a firewall](https://techdocs.akamai.com/linode-api/reference/delete-firewall)
- [Enable backups](https://techdocs.akamai.com/cloud-computing/docs/enable-backups)
- [Billing overview](https://techdocs.akamai.com/cloud-computing/docs/understanding-how-billing-works)
