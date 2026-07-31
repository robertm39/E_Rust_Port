# Reproduction commands

The source selection and local focused tests were:

```text
python experiments/2026-07-30-008-instgen-epr-prototype/select_corpus.py
python -m unittest discover \
  -s experiments/2026-07-30-008-instgen-epr-prototype \
  -p "test_*.py" -v
```

The existing Ubuntu runner was synchronized with:

```text
.\linode-runner.ps1 sync
```

The experiment-only adapter and integration test were run under Ubuntu:

```text
g++ -std=c++17 -O2 -Wall -Wextra -Wpedantic -Werror \
  -I/opt/e-rust-port/cadical-3.0.1/src \
  experiments/2026-07-30-008-instgen-epr-prototype/cadical_driver.cpp \
  /opt/e-rust-port/cadical-3.0.1/build/libcadical.a -lpthread \
  -o /opt/e-rust-port/artifacts/instgen-epr-008/bin/cadical-driver

python3 experiments/2026-07-30-008-instgen-epr-prototype/integration_test.py \
  --repo-root /opt/e-rust-port/source \
  --cadical-driver \
    /opt/e-rust-port/artifacts/instgen-epr-008/bin/cadical-driver \
  --drat-trim \
    /opt/e-rust-port/artifacts/prop-sat-007/bin/drat-trim
```

The selected source files were reconstructed and verified from the retained
CASC archive:

```text
python3 experiments/2026-07-30-008-instgen-epr-prototype/prepare_corpus.py \
  --manifest \
    experiments/2026-07-30-008-instgen-epr-prototype/corpus.jsonl \
  --archive \
    /opt/e-rust-port/artifacts/prop-sat-007/casc_2025_corpus.tar.gz \
  --output-root \
    /opt/e-rust-port/artifacts/instgen-epr-008/corpus
```

The measured release binary was built with:

```text
cargo build --locked --release --bin umlaut
```

The train and held-out phases used the same command, changing only `--phase`:

```text
python3 experiments/2026-07-30-008-instgen-epr-prototype/run_experiment.py \
  --repo-root /opt/e-rust-port/source \
  --manifest \
    experiments/2026-07-30-008-instgen-epr-prototype/corpus.jsonl \
  --problem-root \
    /opt/e-rust-port/artifacts/instgen-epr-008/corpus \
  --output-root \
    /opt/e-rust-port/artifacts/instgen-epr-008/results \
  --umlaut /opt/e-rust-port/source/target/release/umlaut \
  --cadical-driver \
    /opt/e-rust-port/artifacts/instgen-epr-008/bin/cadical-driver \
  --drat-trim \
    /opt/e-rust-port/artifacts/prop-sat-007/bin/drat-trim \
  --proofcheck \
    /opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/\
proofcheck-linux-x86_64/proofcheck \
  --validation-gate \
    /opt/e-rust-port/source/tools/validation/validate_tptp_solution.py \
  --phase heldout
```

The final independent replay and analysis were:

```text
python3 experiments/2026-07-30-008-instgen-epr-prototype/\
validate_results.py \
  --repo-root /opt/e-rust-port/source \
  --results-root /opt/e-rust-port/artifacts/instgen-epr-008/results \
  --problem-root /opt/e-rust-port/artifacts/instgen-epr-008/corpus \
  --drat-trim \
    /opt/e-rust-port/artifacts/prop-sat-007/bin/drat-trim \
  --proofcheck \
    /opt/e-rust-port/artifacts/instgen-epr-008/proofcheck/\
proofcheck-linux-x86_64/proofcheck \
  --validation-gate \
    /opt/e-rust-port/source/tools/validation/validate_tptp_solution.py \
  --output /opt/e-rust-port/artifacts/instgen-epr-008/validation.json

python3 experiments/2026-07-30-008-instgen-epr-prototype/analyze.py \
  --results-root /opt/e-rust-port/artifacts/instgen-epr-008/results \
  --output /opt/e-rust-port/artifacts/instgen-epr-008/analysis.json \
  --markdown /opt/e-rust-port/artifacts/instgen-epr-008/RESULTS.md
```

The raw evidence was packaged without the source corpus or ProofCheck bundle:

```text
tar --exclude=instgen-epr-008/corpus \
  --exclude=instgen-epr-008/proofcheck \
  --exclude=instgen-epr-008/proofcheck.tar.gz \
  --exclude=instgen-epr-008/evidence.tar.gz \
  -czf instgen-epr-008-evidence.tar.gz instgen-epr-008
```
