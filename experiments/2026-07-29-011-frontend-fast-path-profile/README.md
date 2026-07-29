# Frontend fast-path profile

This experiment supports Bead `E_Rust_Port-9jt.8.3`. It measures the current
parser, formula-to-CNF, and clausal-preprocessing boundaries across four TPTP
dialects before deciding whether one bounded specialization is justified.

The design and immutable decision rules are in
[`PREREGISTRATION.md`](PREREGISTRATION.md). Generated corpora and raw profiler
artifacts are ignored; the final findings retain hashes, aggregate metrics,
commands, and the evidence archive identity.

Controller tests may run locally:

```powershell
python experiments/2026-07-29-011-frontend-fast-path-profile/test_frontend_profile.py
```

All prover execution must use the Ubuntu runner. The remote sequence is:

```bash
python3 experiments/2026-07-29-011-frontend-fast-path-profile/frontend_profile.py \
  generate --corpus-root /opt/e-rust-port/frontend-profile/corpus

python3 experiments/2026-07-29-011-frontend-fast-path-profile/frontend_profile.py \
  timing \
  --corpus-root /opt/e-rust-port/frontend-profile/corpus \
  --rust-bin /opt/e-rust-port/source/target/release/umlaut \
  --c-fol-bin /root/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover \
  --c-ho-bin /root/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/ho/eprover-ho \
  --output-root /opt/e-rust-port/frontend-profile/timing

python3 experiments/2026-07-29-011-frontend-fast-path-profile/frontend_profile.py \
  analyze \
  --timing /opt/e-rust-port/frontend-profile/timing/timing.jsonl \
  --output /opt/e-rust-port/frontend-profile/analysis.json
```

The analysis names the held-out profile case. `profile` then collects DHAT and
Callgrind evidence for that case:

```bash
python3 experiments/2026-07-29-011-frontend-fast-path-profile/frontend_profile.py \
  profile \
  --analysis /opt/e-rust-port/frontend-profile/analysis.json \
  --corpus-root /opt/e-rust-port/frontend-profile/corpus \
  --rust-bin /opt/e-rust-port/source/target/release/umlaut \
  --output-root /opt/e-rust-port/frontend-profile/profile
```

When the frozen go/no-go rule permits a prototype, `candidate` compares all
corpus modes, exact 1,000- and 10,000-record TSTP outputs, held-out timing,
allocations, and peak memory:

```bash
python3 experiments/2026-07-29-011-frontend-fast-path-profile/frontend_profile.py \
  candidate \
  --corpus-root /opt/e-rust-port/frontend-profile/corpus \
  --baseline-bin /opt/e-rust-port/frontend-profile/baseline-umlaut \
  --candidate-bin /opt/e-rust-port/source/target/release/umlaut \
  --baseline-profile /opt/e-rust-port/frontend-profile/profile/profile.json \
  --output-root /opt/e-rust-port/frontend-profile/candidate
```
