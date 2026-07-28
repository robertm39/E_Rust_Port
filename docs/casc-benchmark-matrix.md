# CASC-30 benchmark matrix and batch protocol

The checked-in CASC-30 (2025) corpus is the reproducible public baseline for
Umlaut's post-port experiments. It is not an unseen CASC-2027 set, and the
pinned local Vampire executable is not claimed to reproduce the official
Vampire entry or the StarExec environment.

The authoritative machine-readable inventory is
[`benchmarks/casc_2025_manifest.jsonl`](../benchmarks/casc_2025_manifest.jsonl).
It contains 2,901 problems in 12 categories and eight divisions, 100
indivisible source families, and exact problem, axiom-tree, and official-CSV
hashes. The current file is 1,730,358 bytes with SHA-256
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`.
Regenerate or verify it with:

```powershell
.\.venv\Scripts\python.exe tools\casc_benchmark\manifest.py --repo-root .
.\.venv\Scripts\python.exe tools\casc_benchmark\manifest.py --repo-root . --check
```

The generator reconciles every normalized official result-table identifier
with one local problem file. In particular, it removes the HTML best-result
asterisk and normalizes non-breaking hyphens without changing the TPTP file
identifier.

## Metadata and holdout policy

The official [CASC-30 design](https://tptp.org/CASC/30/Design.html) says that
TPTP headers were stripped during competition obfuscation and that the
TPTP-based problems were supplied in increasing difficulty order. The manifest
therefore records the official within-category order and five ordinal bands.
These are explicitly a difficulty proxy, not recovered numeric TPTP ratings.
SLH and ICU retain their competition-specific ordering rules.

Each TPTP or entrant family is normally its three-letter prefix. SLH families
use the first path component of the retained TPTP `Names` header, which
separates the underlying Isabelle theories instead of treating all 1,000
problems as one family. A salted SHA-256 assignment starts from a 70/15/15
train/validation/test target. A deterministic repair moves whole families
until every category has at least one family in every split. No family is ever
split. The resulting problem counts are 1,911 train, 533 validation, and 457
test. Categories with only a few source families, notably the three-family TFE
category, cannot also have problem-balanced splits; reports must show the
actual denominators.

The category semantics come from the CASC division definitions:

- theorem: TNE, TEQ, TFI, TFE, FNE, FEQ, SLH, and ICU;
- non-theorem: TFN;
- unsatisfiable: EPU and UEQ; and
- satisfiable: EPS.

The harness checks a solver's terminal SZS status against this independent
category contract. It never treats Vampire, Umlaut, or the official CSV result
of another system as a soundness oracle.

## Resource and command contract

[`tools/casc_benchmark/batch.py`](../tools/casc_benchmark/batch.py) is a
Linux-only sequential harness. Each solver/problem pair receives a fresh
cgroup-v2 boundary with:

- aggregate `memory.max` of 128 GiB and swap disabled;
- aggregate CPU accounting from `cpu.stat`;
- aggregate peak memory from `memory.peak`;
- a 512-process ceiling;
- monotonic wall accounting; and
- process-session termination followed by `cgroup.kill`, so descendants cannot
  survive a timeout, crash, or result.

The harness refuses the canonical run unless the host exposes at least eight
CPUs and 128 GiB plus 4 GiB of host overhead. It also sets per-process
`RLIMIT_AS` as a second boundary. The 15-second SLH CPU limit is enforced from
aggregate cgroup CPU usage with both provers restricted to one core. All other
divisions use the official CASC-30 wall limits: 120 seconds for TFA, TFN, and
EPR; 240 seconds for THF, FOF, and UEQ; and 480 seconds for ICU. Those limits
and the 128 GiB competition memory limit are published on the
[CASC-30 archive page](https://tptp.org/CASC/30/).

Umlaut uses the complete `satauto-schedule` for satisfiable/non-theorem
categories and `auto-schedule` elsewhere. Vampire uses the pinned
`casc_sat_2025` or `casc_2025` schedule, a fixed seed of one, and disabled
per-worker seed randomization. The canonical ignored Vampire executable must
have SHA-256
`3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665`;
any other binary is rejected.

The output root contains:

- `contract.json`, whose identity covers the manifest, selected problems,
  binary hashes, adapters, seeds, limits, and presentation;
- one host/session record per Linode;
- atomic JSON results plus complete stdout and stderr for every pair; and
- a regenerable `summary.json`.

An existing result is reused only after its contract, problem hash, and output
hashes pass. A changed binary, manifest, presentation, seed, selection, or
limit makes the output directory incompatible instead of silently mixing
results. A later presentation-randomization experiment must supply a new
hash-complete manifest and presentation ID; it cannot reuse this contract.

## Canonical high-memory run

The ignored Vampire artifact is uploaded separately because repository sync
must never include it. The corpus is also under the repository's ignored
`problems/` boundary, so build its deterministic regular-file-only transfer
archive first:

```powershell
$vampire = ".artifacts\vampire\3677326861181f990ce3ef461e90471ba9749225\linode-ubuntu24.04-x86_64\vampire"
$corpus = ".artifacts\casc-benchmark\casc_2025_corpus.tar.gz"
$statePath = Join-Path $env:LOCALAPPDATA "E-Rust-Port\linode-runner\current.json"

.\.venv\Scripts\python.exe tools\casc_benchmark\corpus_archive.py pack --repo-root . --output $corpus
$corpusHash = (Get-FileHash $corpus -Algorithm SHA256).Hash.ToLower()

.\linode-runner.ps1 up --high-memory
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 upload $vampire /root/vampire-5.0.1
    .\linode-runner.ps1 upload $corpus /root/casc_2025_corpus.tar.gz
    $runner = Get-Content -Raw $statePath | ConvertFrom-Json
    $snapshotHash = $runner.snapshot.archive_sha256
    .\linode-runner.ps1 exec -- "echo '$corpusHash  /root/casc_2025_corpus.tar.gz' | sha256sum -c - && cd /opt/e-rust-port/source && python3 tools/casc_benchmark/corpus_archive.py extract --archive /root/casc_2025_corpus.tar.gz --destination /opt/e-rust-port/source --manifest benchmarks/casc_2025_manifest.jsonl && chmod 0555 /root/vampire-5.0.1 && cargo build --locked --release --bin umlaut"
    $identity = "--runner-label=$($runner.label) --runner-run-id=$($runner.run_id) --linode-id=$($runner.linode_id)"
    .\linode-runner.ps1 exec -- "cd /opt/e-rust-port/source && python3 tools/casc_benchmark/batch.py --manifest benchmarks/casc_2025_manifest.jsonl --problem-root /opt/e-rust-port/source --output-root /opt/e-rust-port/casc-runs/casc30-v1 --umlaut-binary target/release/umlaut --vampire-binary /root/vampire-5.0.1 --source-snapshot-sha256=$snapshotHash $identity --verify-only"
    .\linode-runner.ps1 exec -- "cd /opt/e-rust-port/source && python3 tools/casc_benchmark/batch.py --manifest benchmarks/casc_2025_manifest.jsonl --problem-root /opt/e-rust-port/source --output-root /opt/e-rust-port/casc-runs/casc30-v1 --umlaut-binary target/release/umlaut --vampire-binary /root/vampire-5.0.1 --source-snapshot-sha256=$snapshotHash $identity"
    .\linode-runner.ps1 exec -- "cd /opt/e-rust-port/source && python3 tools/casc_benchmark/report.py --manifest benchmarks/casc_2025_manifest.jsonl --run-root /opt/e-rust-port/casc-runs/casc30-v1"
    .\linode-runner.ps1 exec -- "tar -C /opt/e-rust-port/casc-runs -czf /root/casc30-v1.tar.gz casc30-v1"
    .\linode-runner.ps1 download /root/casc30-v1.tar.gz .artifacts\casc-benchmark\casc30-v1.tar.gz
}
finally {
    .\linode-runner.ps1 down
}
```

If the run needs another guarded session, upload and extract the downloaded
archive before invoking the same batch command. Resume succeeds only when the
rebuilt Umlaut hash and every other contract input remain identical. Create
the archive before each teardown so partial work is not lost.

The `g7-highmem-8` plan is currently rejected by the Linode account before
instance creation. The canonical 128 GiB run therefore remains an explicit
external gate under `E_Rust_Port-9jt.2.7`; a normal 8 GiB runner cannot satisfy
it.

## Noncanonical smoke validation

A normal runner may validate plumbing on one easy FOF problem, but the
resulting contract is deliberately noncanonical:

```text
python3 tools/casc_benchmark/batch.py \
  --manifest benchmarks/casc_2025_manifest.jsonl \
  --problem-root /opt/e-rust-port/source \
  --output-root /opt/e-rust-port/casc-runs/smoke \
  --umlaut-binary target/release/umlaut \
  --vampire-binary /root/vampire-5.0.1 \
  --problem KRS203+1 \
  --cores 4 \
  --memory-limit-mib 4096 \
  --allow-noncanonical-host
```

Use
[`tools/casc_benchmark/report.py`](../tools/casc_benchmark/report.py) with
`--allow-partial` while inspecting an interrupted canonical run. A complete
report includes per-category, division, split, and difficulty-band coverage;
classification counts; wall/CPU/peak-memory distributions; time curves;
overlap and unique solves; final-status pairs; and proof/model polarity
disagreements.
