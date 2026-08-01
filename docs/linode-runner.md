# Billing-aware Linode compute runner

The repository includes a controller for short-lived Linux build and profiling
workers. It supports two Akamai Cloud plans in Chicago (`us-ord`) using the
`linode/ubuntu24.04` image:

| Profile | Selection | Type | Resources | Price |
| --- | --- | --- | --- | --- |
| Normal (default) | no flag | `g8-dedicated-8-4` | 8 GiB RAM, 4 dedicated CPUs, 82 GiB storage | $0.14 an hour |
| 150 GB high memory | `--high-memory` | `g7-highmem-8` | 150 GiB RAM, 8 dedicated CPUs, 200 GiB storage | $0.74 an hour |

High-memory plan deployment is also subject to provider account access. The
read-only check can validate catalog visibility, regional capacity, and the
cost guard while a later instance-creation request is still rejected by an
account plan limit. In that case the controller deletes any firewall it
created; contact provider support for `g7-highmem-8` access rather than
substituting the normal profile for a required high-memory validation.

Use the high-memory profile when the task needs to resemble the CASC compute
configuration more closely. For a closer CASC match, limit each actual prover
process to 128 GiB rather than allowing it to consume the host's full 150 GiB:

```text
--memory-limit=131072
```

The prover option is expressed in MB, so `131072` represents 128 GiB. The
controller does not inject this option automatically; include it in every
CASC-oriented Umlaut command.

High-memory starts have a mandatory bank-adjusted daily cost guard. The base
allowance is four hours per accounting day. Unused base allowance is added to a
bank capped at four hours, so a full bank raises one day's capacity to eight
hours. Existing trusted history is replayed as though the bank was full before
the earliest recorded run; no separate balance file is required.

The balance is fixed at the start of each day. Positive balance is banked usage;
negative balance is uncapped usage debt. The controller computes:

```text
daily capacity = max(0, 4 hours + starting balance)
next balance = min(4 hours, starting balance + 4 hours - actual usage)
```

The controller refuses another high-memory `up` or `run` once actual usage
reaches the day's capacity. A start is allowed while usage is below capacity
even if that run later crosses the threshold; the overshoot becomes debt.
Empty days add four hours, first repaying debt and then filling the bank.
Normal-profile starts are not restricted by high-memory usage.

For example, a two-hour starting bank gives six hours of capacity. Using three
hours leaves a three-hour bank for the next day, using five hours leaves a
one-hour bank, and using seven hours creates one hour of debt. That debt reduces
the next day's capacity to three hours.

An accounting day is midnight-to-midnight at fixed UTC-05:00 ("fixed EST").
Daylight-saving time is never applied. The controller obtains current time
from the Linode API's HTTPS `Date` header and records Linode-provided
creation-to-deletion intervals, rather than trusting the Windows clock.
`check --high-memory` reports the base allowance, bank and debt at day start,
adjusted capacity, actual and remaining time, projected balance at the next
boundary, and projected eligibility when blocked. It returns nonzero when a new
high-memory start would be blocked.

This worker is the project's sole Rust/C execution environment. Do not run
Cargo, `rustc`, Rust project binaries, the C build, C binaries, WSL, Valgrind,
or Callgrind on the local computer. Local PowerShell only orchestrates the
worker; lightweight Python controller tests, PowerShell parsing, Git, and
documentation checks may remain local.

There are no exceptions for quick smoke tests, focused reproductions, or an
already-installed local toolchain. WSL, local containers, and local virtual
machines are not supported substitutes. Any command that formats, compiles,
links, tests, starts, compares, benchmarks, or profiles Umlaut or the C
reference must execute on the Linode.

The normal `run` workflow acquires a compatible runner, reusing a parked one
when possible. If none exists, it creates a Cloud Firewall and Linode and
installs the complete Linux and Windows-cross toolchain. It then uploads a
fresh snapshot of the current worktree, performs every required validation
phase, downloads the artifacts, removes project source and workload residue
from the host, and parks the runner until the current paid billing hour's
guarded cutoff. A later `up` or `run` with the exact same type, region, and
image reactivates that runner instead of provisioning another one.

Before the first `apt-get`, bootstrap waits for `cloud-init` to finish, then
masks and stops `apt-daily.timer`, `apt-daily-upgrade.timer`,
`apt-daily.service`, and `apt-daily-upgrade.service`. Provisioning fails closed
unless every unit is inactive and masked. The controller records the atomic
remote JSON path, its SHA-256, and the verified unit states under
`package_maintenance` in the saved runner state, which `status` exposes. The
units stay quiesced for the worker's lifetime, including reuse cycles. They
need not be restored before parking because the runner remains dedicated to
this controller and is eventually deleted.

## Billing-aware parking and cleanup

Linode bills each instance for elapsed use rounded up to an hour. Deleting
several instances during one wall-clock hour can therefore cost several billed
hours. `down` and successful `run` cleanup now park a runner instead of
immediately deleting it. A parked runner stays powered on and has no project
source tree, workload processes, explicit uploads, or uncollected artifacts.
The firewall continues to accept SSH only from the controller's current `/32`.

The deletion deadline is two minutes before the end of the currently paid
hour measured from the Linode-provided creation time. If a worker has already
crossed another exact hourly boundary, the controller may reuse it through
that newly paid hour. If its current two-minute cutoff has already passed,
parking deletes it immediately. Acquisition also deletes a candidate when
less than 30 seconds remain before its armed deadline rather than racing its
reaper.

Two independent cleanup paths are armed before local state is changed from
active to parked:

- a Windows Scheduled Task named `Umlaut-Linode-Reaper-<linode-id>` invokes
  the wrapper with the exact Linode ID and a random lease ID; the token never
  appears in task arguments;
- a persistent systemd timer on the Linode invokes a small standard-library
  reaper using a root-readable token and state file. It verifies both live
  resource labels, marks the free firewall with the lease outcome, and deletes
  only its own Linode. This path still stops billing if Windows is asleep,
  powered off, or disconnected.

The local reaper later reconciles an already-deleted Linode, removes its free
firewall, archives the state, and removes the Scheduled Task. Reaper commands
are idempotent and lease-checked. A reused runner has both timers disarmed and
its temporary reaper access removed before new source is uploaded.

Parking is deliberately fail-closed. If restricted reaper setup is missing,
incomplete, unexpectedly privileged, or cannot be armed, `down`/`run` delete
the active Linode and firewall instead of leaving an unguarded paid resource.
An incompatible parked runner remains parked until its own deadline while a
new compatible type is acquired. Local lifecycle locking and a provision
claim prevent concurrent commands from provisioning two active runners.

## One-time account preparation

Create the main controller Personal Access Token in Cloud Manager with these
permissions:

- Linodes: read/write
- Firewalls: read/write
- Account: read/write, so the controller can assign and remove exact IAM
  entity roles for the restricted reaper user
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

The main token is decrypted only into the environment of the controller
process. It is not passed on a command line, stored in the repository, or
uploaded to the Linode.

Create a separate restricted Linode user dedicated to reaping. Do not give it
account administrator, billing, user-management, or unrelated entity roles.
While signed in as that user, create a PAT with only `linodes:read_write` and
`firewall:read_write` scopes. Return to the controller account and store that
PAT plus the restricted username using a secure prompt:

```powershell
.\linode-runner.ps1 init-reaper --username umlaut-reaper
```

The wrapper encrypts this PAT with user-scoped Windows DPAPI at
`$env:LOCALAPPDATA\E-Rust-Port\linode-reaper-token.dpapi`; the JSON reaper
configuration contains only the username. On each park/reuse/reap transition,
the main controller replaces the restricted user's entity access with the
exact `linode_admin` and `firewall_admin` roles for the currently parked
resources. Before doing so, it refuses unexpected account roles, entity IDs,
or role names. Only the restricted PAT is copied to a parked Linode, in a
root-only file, and deleting the Linode destroys that copy.

Generate the dedicated, passwordless Ed25519 key used only for disposable
runners:

```powershell
.\linode-runner.ps1 init
.\linode-runner.ps1 check
.\linode-runner.ps1 check --high-memory
```

The key and controller state are stored under
`$env:LOCALAPPDATA\E-Rust-Port\linode-runner`, outside the repository.
The `check` command is read-only: it validates the token, selected plan,
Chicago capacity, Ubuntu image, public source IP, local OpenSSH tools, and—when
configured—the restricted reaper user, PAT, and exact current entity roles,
without creating a billable resource. If reaper setup is absent, it clearly
reports that parking is disabled and immediate deletion will be used.

## Comprehensive remote validation

Run the complete required project-validation lifecycle from the repository
root on the normal $0.14-an-hour profile:

```powershell
.\linode-runner.ps1 run
```

When the validation specifically needs the CASC-like 150 GB host, use the
guarded $0.74-an-hour profile:

```powershell
.\linode-runner.ps1 run --high-memory
```

The same bank-adjusted fixed-EST start guard applies to both the automated
`run` command and the interactive `up` command. The advanced `--type` option
remains available for compatibility, but only the two documented types are
accepted; `--type g7-highmem-8` cannot bypass the high-memory guard.

The controller detects the machine's current public IPv4 address and creates an
ephemeral firewall that accepts only TCP port 22 from that `/32`. Outbound
traffic is allowed so Ubuntu, Rust, and Cargo dependencies can be installed.
No other inbound traffic is accepted.

All project code runs on native Ubuntu 24.04. The workload:

1. verifies the package-maintenance record, forces a systemd manager reexec
   across a transient benchmark probe, confirms the same PID and invocation
   survive, then proves SIGINT cleanup leaves no process, cgroup member, or
   incomplete result and that a hash-valid coordinate resumes without rerun;
2. runs Rustfmt, all-target/all-feature tests, pedantic Clippy, and release
   builds for every Rust binary;
3. cross-compiles every Rust binary and test target for
   `x86_64-pc-windows-gnu`, records PE metadata and hashes, and never executes
   a Windows binary;
4. builds disposable FOL and HO copies of the upstream C reference plus all
   support tools without modifying `eprover/`;
5. runs the maintained main-prover and support-tool C/Rust compatibility
   matrices natively on Linux;
6. runs the seeded five-trial native timing benchmark, smoke tests, and
   Callgrind profiles for Rust and C.

Linux is the runtime, behavioral-compatibility, and performance authority.
Windows GNU x64 is compile-only; MSVC and Windows runtime behavior are not
supported validation targets. Generated Windows binaries are inspected and
hashed but never executed through Wine, emulation, or any other mechanism.

Each run uploads a new archive made directly from the current filesystem. It
therefore includes tracked modifications, untracked source files, and the
ignored-but-required `eprover/` checkout. It excludes Git/Dolt metadata,
credentials, agent state, virtual environments, build outputs, prior artifacts,
Python caches, and the reference-only `cadical/`, `gmp-6.3.0/`, `minisat/`,
`vampire/`, and `z3/` checkouts, as well as the external `problems/` corpus. A
task that needs one of those references or problems must transfer its
explicitly pinned inputs separately. Nothing relies on a remote Git branch
being pushed first.

The retained results are written to:

```text
.artifacts/linode/<workload-id>/
```

Each reuse cycle receives a fresh workload ID, so artifact directories never
collide with results from an earlier workload on the same Linode. They include
Linux Rust quality-gate logs, FOL/HO C build metadata, Windows GNU
cross-compile logs and PE hashes, main and tool compatibility reports, timing
benchmark samples, native smoke output, raw and annotated Callgrind data, Linux
binary hashes, and instruction summaries. They also retain the immutable
`package-maintenance-quiescence.json` and its hash plus
`package-maintenance-lifecycle.json`, the transient-unit journal, and
`PACKAGE_MAINTENANCE_LIFECYCLE_COMPLETE`.

`VALIDATION_COMPLETE` means every phase ran and its reports were collected.
`SUCCESS` additionally means no unexpected main or support-tool compatibility
difference was found. If real compatibility gaps remain, the command returns
nonzero after writing `COMPATIBILITY_MISMATCHES`; partial and complete artifacts
are still downloaded and the runner is still parked or safely deleted. Benchmark
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

Select the 150 GB profile at creation time when an interactive task needs the
CASC-like host:

```powershell
.\linode-runner.ps1 up --high-memory
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && cargo build --locked --release --bin umlaut && target/release/umlaut eprover/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p --auto --silent --cpu-limit=10 --memory-limit=131072"
}
finally {
    .\linode-runner.ps1 down
}
```

The high-memory host alone does not reproduce the CASC memory envelope; retain
`--memory-limit=131072` on every prover invocation intended to model CASC.

Run `sync` again whenever the local files change. It replaces the remote source
directory with a fresh immutable upload; no Git pull or remote working-branch
maintenance is involved.

Ignored or otherwise explicit single-file experiment inputs are never included
by `sync`. Transfer one only when the task authorizes that exact artifact:

```powershell
.\linode-runner.ps1 upload .artifacts\reference.bin /root/reference.bin
```

The source must be a local file. Upload destinations must be non-hidden paths
beneath `/root` and use only a conservative filename character set, which lets
parking terminate consumers and remove the uploaded file safely. Download a
single result archive without silently overwriting an existing local artifact:

```powershell
.\linode-runner.ps1 download /root/results.tar.gz .artifacts\results.tar.gz
```

Pass `--overwrite` only after intentionally selecting the existing destination.
These commands use the active runner's dedicated SSH identity and firewall;
they do not weaken the source snapshot exclusions or make an ignored artifact
distributable.

For a focused Rust build and prover smoke run, replace the `exec` command inside
that same guarded lifecycle with:

```powershell
.\linode-runner.ps1 exec -- `
    "cd /opt/e-rust-port/source && cargo build --locked --release --bin umlaut && target/release/umlaut eprover/EXAMPLE_PROBLEMS/SMOKETEST/socrates.p --auto --silent --cpu-limit=10"
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

`status` returns separate `active` and `parked` entries, including live Linode
and firewall status and each parked deletion deadline.

`down` parks the active runner by default. It reads the exact saved resource
IDs, fetches each live resource, and refuses any lifecycle operation unless
the live label exactly matches the saved `e-rust-codex-` label. Use
`down --now` to delete only the active Linode and firewall immediately. Use
`down --all` for an intentional emergency teardown of the active runner and
every parked runner. Immediate deletion always removes the Linode first, waits
for it to disappear, and then removes the firewall.

## Failure recovery and stale-resource cleanup

The default `run` behavior parks resources after success, command failure, or
interruption. If parking cannot be guarded, it falls back to deletion. If
cleanup itself fails, the local state is retained and the controller prints an
urgent instruction to run:

```powershell
.\linode-runner.ps1 down --now
```

Keeping a failed paid worker requires explicit opt-in:

```powershell
.\linode-runner.ps1 run --keep-on-failure
```

That option leaves the failed runner active for debugging; finish with `down`
to sanitize and park it, or `down --now` to delete it immediately.

Inspect managed resources older than six hours without changing them:

```powershell
.\linode-runner.ps1 gc
```

After reviewing that dry-run list, delete it with:

```powershell
.\linode-runner.ps1 gc --yes
```

Garbage collection considers only resources whose labels start with
`e-rust-codex-`, excludes every active or parked saved ID, and requires
resources to be at least one hour old. It should be a recovery mechanism, not
the normal cleanup path.

## Controller-only local validation

The controller and compatibility-report code use only the Python standard
library and the Windows-bundled OpenSSH and `tar` tools. Their lightweight
Python tests may run locally:

```powershell
.\.venv\Scripts\python.exe -m unittest discover `
    -s tools\linode-runner -p "test_*.py" -v
```

The tests pin both supported Linode profiles, CLI selection, trusted API time,
fixed-EST high-memory bank/debt accounting and blocking, firewall settings,
hour-bucket deletion deadlines, exact configuration reuse, local lifecycle
locking, restricted IAM entity access, lease-checked local and remote reapers,
cloud-init/package-maintenance ordering and fail-closed records, remote-only
quality gates, the daemon-reexec/resume lifecycle contract, Windows
cross-toolchain bootstrap, source-archive exclusions, safe artifact extraction,
stale-resource selection, label-matching deletion guards, compatibility
matrices, report normalization, and disposable C-source preparation. They do
not compile or execute Rust or C.

The bootstrap also installs the POSIX-thread MinGW C++ compiler and downloads
the pinned CaDiCaL 3.0.1 source archive for `--all-features` validation. It
checks SHA-256
`ad639a302b7c4cb4a24f37b7cd0cf7533674e6069c20a561505bccef1c2b4444`
before extraction and exports the source/toolchain variables in the remote
quality script. The ordinary source snapshot and default package continue to
exclude every ignored `cadical/` reference tree.

Current Akamai references:

- [Create a Linode](https://techdocs.akamai.com/linode-api/reference/post-linode-instance)
- [Create a firewall](https://techdocs.akamai.com/linode-api/reference/post-firewalls)
- [Delete a firewall](https://techdocs.akamai.com/linode-api/reference/delete-firewall)
- [Enable backups](https://techdocs.akamai.com/cloud-computing/docs/enable-backups)
- [Billing overview](https://techdocs.akamai.com/cloud-computing/docs/understanding-how-billing-works)
