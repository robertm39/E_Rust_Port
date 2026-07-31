# TSM ranking feasibility commands

All Rust compilation and execution was performed through the retained Ubuntu
runner. Paths below are the recorded runner paths; large outputs are stored in
the ignored artifact archive named in `FINDINGS.md`.

## Immutable inputs

```powershell
.\linode-runner.ps1 upload -- `
  .artifacts/experiments/2026-07-29-018-tsm-learning-baseline/tsm-learning-018-81232361-complete.tar.gz `
  /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018.tar.gz

.\linode-runner.ps1 exec -- `
  "mkdir -p /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018 && tar -xzf /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018.tar.gz -C /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018"

.\linode-runner.ps1 upload -- `
  problems/casc_2025/UEQ/LCL026-10.p `
  /opt/e-rust-port/source/problems/casc_2025/UEQ/LCL026-10.p
```

## Native profile

Build the ordinary fat-LTO release and preserve it before any symbol-rich
rebuild:

```powershell
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && cargo build --locked --release --bin umlaut --bin umlaut-tsm-classify"

.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && python3 experiments/2026-07-30-011-tsm-ranking-feasibility/profile.py --source-root /opt/e-rust-port/source --prior-root /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018 --prior-archive /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018.tar.gz --output-root /opt/e-rust-port/artifacts/tsm-ranking-011/candidate-native-final-v1 --mode native"
```

The corresponding valid baseline root is `baseline-native-v2`.

## Callgrind profile and analysis

```powershell
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && RUSTFLAGS='-C debuginfo=2' cargo build --locked --release --bin umlaut --bin umlaut-tsm-classify"

.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && python3 experiments/2026-07-30-011-tsm-ranking-feasibility/profile.py --source-root /opt/e-rust-port/source --prior-root /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018 --prior-archive /opt/e-rust-port/artifacts/tsm-ranking-011/prior-018.tar.gz --output-root /opt/e-rust-port/artifacts/tsm-ranking-011/candidate-callgrind-debug-final-v1 --mode callgrind"
```

Run `analyze_profiles.py` with:

- native summary `candidate-native-final-v1/summary.json`;
- Callgrind summary
  `candidate-callgrind-debug-final-v1/summary.json`; and
- that Callgrind root's search-control, search-learned, classifier-empty, and
  classifier-full `.out` files.

Repeat with `baseline-native-v2` and `baseline-callgrind-debug-v1`. The
analyzer verifies each raw instruction total against the controller summary
before producing `candidate-analysis-final.json` or
`baseline-analysis-final.json`.

## Validation

```powershell
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && cargo fmt --all -- --check"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && cargo check --all-targets"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && cargo clippy --all-targets -- -D warnings"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && cargo clippy --all-targets -- -D warnings -W clippy::pedantic"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && cargo test --all-targets"

.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-3.0.1 cargo check --all-targets --all-features"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-3.0.1 cargo clippy --all-targets --all-features -- -D warnings -W clippy::pedantic"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-3.0.1 cargo test --all-targets --all-features"

.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && cargo check --target x86_64-pc-windows-gnu --all-targets"

.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source/experiments/2026-07-30-011-tsm-ranking-feasibility && python3 -m unittest -v test_profile.py test_analyze_profiles.py && python3 -m py_compile profile.py analyze_profiles.py test_profile.py test_analyze_profiles.py"
```

## Artifact packaging

The evidence archive was created under
`/opt/e-rust-port/artifacts/tsm-ranking-011`, downloaded with
`linode-runner.ps1 download`, and verified independently on Windows with:

```powershell
Get-FileHash `
  .artifacts/experiments/2026-07-30-011-tsm-ranking-feasibility/tsm-ranking-011-evidence-v1.tar.gz `
  -Algorithm SHA256
```
