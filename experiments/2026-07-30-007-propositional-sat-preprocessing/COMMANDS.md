# Exact experiment commands

These commands ran inside the Ubuntu worker's synced
`/opt/e-rust-port/source` tree. Each block was invoked through
`.\linode-runner.ps1 exec '...'` from the Windows repository root.

## Validate and build

```bash
python3 -m unittest discover \
  -s experiments/2026-07-30-007-propositional-sat-preprocessing \
  -p test_run_experiment.py
python3 -m py_compile \
  experiments/2026-07-30-007-propositional-sat-preprocessing/run_experiment.py
rustfmt --edition 2021 --check \
  experiments/2026-07-30-007-propositional-sat-preprocessing/internal_probe.rs

gcc -O2 \
  -o /opt/e-rust-port/artifacts/prop-sat-007/bin/drat-trim \
  /opt/e-rust-port/cadical-3.0.1/test/cnf/drat-trim.c
g++ -O3 -std=c++17 -Wall -Wextra -Wpedantic \
  -I/opt/e-rust-port/cadical-3.0.1/src \
  experiments/2026-07-30-007-propositional-sat-preprocessing/cadical_probe.cpp \
  /opt/e-rust-port/cadical-3.0.1/build/libcadical.a \
  -lpthread \
  -o /opt/e-rust-port/artifacts/prop-sat-007/bin/cadical-probe

mkdir -p examples
cp experiments/2026-07-30-007-propositional-sat-preprocessing/internal_probe.rs \
  examples/prop_sat_internal_probe.rs
cargo build --release --bin umlaut --example prop_sat_internal_probe
```

CaDiCaL was checked at commit
`c60730422e758ef1cebe7aeddf2dda31c996bf04`, version `3.0.1`, with
`git fsck --strict` before compilation.

## Prepare

```bash
python3 \
  experiments/2026-07-30-007-propositional-sat-preprocessing/run_experiment.py \
  prepare \
  --casc-manifest benchmarks/casc_2025_manifest.jsonl \
  --casc-archive \
    /opt/e-rust-port/artifacts/prop-sat-007/casc_2025_corpus.tar.gz \
  --sat-archive \
    /opt/e-rust-port/artifacts/prop-sat-007/sat012-results.tar.gz \
  --output /opt/e-rust-port/artifacts/prop-sat-007/prepared
```

## Measure and prove restartability

```bash
python3 \
  experiments/2026-07-30-007-propositional-sat-preprocessing/run_experiment.py \
  run \
  --prepared /opt/e-rust-port/artifacts/prop-sat-007/prepared \
  --results /opt/e-rust-port/artifacts/prop-sat-007/results.jsonl \
  --internal-probe \
    /opt/e-rust-port/source/target/release/examples/prop_sat_internal_probe \
  --cadical-probe \
    /opt/e-rust-port/artifacts/prop-sat-007/bin/cadical-probe \
  --umlaut-binary /opt/e-rust-port/source/target/release/umlaut \
  --workers 8 \
  --repetitions 20
```

The same command ran a second time and printed:

```text
completed 0 new records; resumed 38100
```

## Certify and analyze

```bash
python3 \
  experiments/2026-07-30-007-propositional-sat-preprocessing/run_experiment.py \
  certify \
  --prepared /opt/e-rust-port/artifacts/prop-sat-007/prepared \
  --results /opt/e-rust-port/artifacts/prop-sat-007/results.jsonl \
  --cadical-probe \
    /opt/e-rust-port/artifacts/prop-sat-007/bin/cadical-probe \
  --drat-trim /opt/e-rust-port/artifacts/prop-sat-007/bin/drat-trim \
  --output /opt/e-rust-port/artifacts/prop-sat-007/certificates

python3 \
  experiments/2026-07-30-007-propositional-sat-preprocessing/run_experiment.py \
  analyze \
  --prepared /opt/e-rust-port/artifacts/prop-sat-007/prepared \
  --results /opt/e-rust-port/artifacts/prop-sat-007/results.jsonl \
  --certificates /opt/e-rust-port/artifacts/prop-sat-007/certificates \
  --output /opt/e-rust-port/artifacts/prop-sat-007/report.json
```

## Retain raw evidence

```bash
cd /opt/e-rust-port/artifacts/prop-sat-007
sha256sum \
  prepared/manifest.json \
  results.jsonl \
  certificates/certificates.json \
  report.json \
  bin/cadical-probe \
  bin/drat-trim \
  results-invalid-partial-model.jsonl \
  certificates-invalid-weak-mutation/certificates.json \
  >checksums.sha256
tar \
  --exclude=casc_2025_corpus.tar.gz \
  --exclude=sat012-results.tar.gz \
  -czf /opt/e-rust-port/artifacts/prop-sat-007-results.tar.gz \
  .
```
