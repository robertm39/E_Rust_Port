# Reversible CASC packaging

## Question

Can the current Umlaut source be packaged and rebuilt on the mandatory Ubuntu
24.04 target without any ignored theorem-prover checkout or local artifact,
while producing a minimal runtime archive with no optional solver/backend
linked or bundled?

The falsifying outcomes are:

- the extracted source package needs `eprover/` or another ignored path;
- the lock file contains an undeclared dependency;
- an ignored reference, paper PDF, experiment, or artifact enters an archive;
- an optional SAT/SMT/numeric/ML backend is bundled or linked;
- any declared Rust binary fails to build from the extracted archive offline;
- the resulting primary binary fails its version smoke test; or
- the package contents, sizes, hashes, toolchain, or dynamic libraries cannot
  be recorded reproducibly.

## Setup

The experiment uses the repository's normal ephemeral Ubuntu 24.04 Linode,
not the high-memory profile. It runs
`tools/packaging/verify_casc_package.py` against the exact synchronized
worktree. The verifier creates a Cargo source archive from the manifest
allowlist, extracts it under a temporary directory, builds every declared
binary with the dependency-free lock file and `--offline`, then creates a
minimal deterministic runtime candidate.

The local preflight was:

```powershell
.\.venv\Scripts\python.exe -m unittest discover `
    -s tools\packaging -p "test_*.py" -v
.\.venv\Scripts\python.exe -m unittest discover `
    -s tools\linode-runner -p "test_*.py" -v
git diff --check
```

The final guarded remote lifecycle used:

```powershell
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && python3 tools/packaging/verify_casc_package.py --output-dir /opt/e-rust-port/package-audit"
    # Download /opt/e-rust-port/package-audit with the active runner identity.
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && cargo fmt --all -- --check"
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && cargo test --locked --all-targets --all-features -- --quiet"
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && cargo clippy --locked --all-targets --all-features -- -D warnings -W clippy::pedantic"
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && cargo test --locked --all-targets --all-features --target x86_64-pc-windows-gnu --no-run"
    .\linode-runner.ps1 exec -- `
        "cd /opt/e-rust-port/source && cargo build --locked --release --bins --target x86_64-pc-windows-gnu"
}
finally {
    .\linode-runner.ps1 down
}
```

The retained raw files are:

- tracked manifest:
  [`package-audit.json`](package-audit.json);
- ignored source and runtime archives:
  `.artifacts/package-audit/2026-07-27-001/`.

## Results

The first target-Linux attempt falsified the initial allowlist. Cargo patterns
without a leading slash are not root-anchored, so the plain `LICENSE` pattern
included `cadical/LICENSE` from the ignored checkout. The verifier rejected the
archive before compilation. Root-anchoring every include pattern and adding a
regression closed that leak.

The same run exposed generic-runner over-inclusion. Before the snapshot fix,
the routine archive contained 17,825 files and measured 392.0 MiB compressed.
Excluding the five unrelated prover references still left 5,343 files from the
ignored external `problems/` corpus and produced 8,848 files/373.8 MiB. The
completed boundary keeps the required ignored E checkout but excludes those
five reference trees and `problems/`. Final runs consistently synchronized
3,521 files; the authoritative final snapshot was 7,295,251 bytes with SHA-256
`7e0d4007127ce66c81b2ce16bf5de2e71070de912b9e29b2b47f853b229473f3`.

The final run was normal-profile runner
`e-rust-codex-260728-020943-14cb` (Linode `101569252`) on Ubuntu 24.04,
Linux 6.8.0-134, x86-64, glibc 2.39, Rust
`1.97.1 (8bab26f4f 2026-07-14)`, and Cargo
`1.97.1 (c980f4866 2026-06-30)`.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Cargo/CASC source `.tgz` | 1,908,463 | `e55002e237d22adea396b8fb0fd10b10f07a60cf1f5b806aacf4518e37acf645` |
| Minimal runtime candidate `.tgz` | 2,754,919 | `ffcac4ccba770e13fd534918f3366e0a98fa859c21525b93a05537567424d53b` |
| Primary `umlaut` ELF before archive compression | 8,160,376 | `7fa340a6abb21d712869187e42977d71399ab0ddf8abdabeb6d6d125904cc3dc` |

The source archive contains 307 files and 13.5 MiB uncompressed. From its clean
temporary extraction, all 26 declared binaries built with
`cargo build --locked --release --bins --offline` in 2 minutes 33 seconds. No
ignored checkout was available in that extraction.

The runtime archive contains exactly:

```text
umlaut-0.1.0/LICENSE
umlaut-0.1.0/README.md
umlaut-0.1.0/THIRD_PARTY_NOTICES.md
umlaut-0.1.0/bin/umlaut
```

`ldd` reports only the Linux loader, `libgcc_s.so.1`, `libm.so.6`, and
`libc.so.6`. PicoSAT, CaDiCaL, MiniSat, Z3, GMP, Vampire, and VIRAS are absent
from both archive members and dynamic linkage. `umlaut --version` identifies
Umlaut 0.1.0 and the E 3.3.5 compatibility baseline.

The final remote quality gates all passed:

- Rustfmt over the full project;
- locked all-target/all-feature Linux tests;
- locked all-target/all-feature pedantic Clippy with warnings denied;
- compile-only locked all-target/all-feature tests for
  `x86_64-pc-windows-gnu`; and
- release builds of all Windows-GNU binaries without execution.

The final worker and firewall were deleted by the guarded lifecycle.

## Conclusion and limits

The hypothesis is confirmed for the current dependency-free baseline. Umlaut
now has a self-contained, reproducibly audited source boundary and a minimal
runtime candidate. Clean buildability no longer depends on the ignored E
checkout because the exact schedule input is tracked with provenance, and the
optional-backend boundary is both documented and mechanically checked.

The negative first result was material: without root-anchored Cargo patterns,
an ignored reference license entered the supposedly allowlisted source package.
The new regression and archive-member audit specifically prevent recurrence.

The runtime candidate is not a final CASC-2027 StarExec package. The exact
StarExec wrapper must come from the organizer's then-current exemplar and pass
an actual StarExec installation/job. This experiment also does not resolve the
VIRAS license, change Umlaut's current GPL-2.0-or-later declaration to the
intended LGPL-3.0, or adopt any optional solver, arithmetic, SMT, or ML backend.
Each such change remains rejected by default until the matrix gate is repeated.
