# Persistent SATCheck design commands

## Local falsification

```powershell
Set-Location experiments/2026-07-30-012-persistent-satcheck-design
python -m unittest -v test_model.py
python -m py_compile model.py campaign.py test_model.py
python campaign.py --output campaign-result.json
```

## Ubuntu falsification

After uploading the four Python/preregistration inputs to the same experiment
path below `/opt/e-rust-port/source`:

```powershell
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source/experiments/2026-07-30-012-persistent-satcheck-design && python3 -m unittest -v test_model.py && python3 -m py_compile model.py campaign.py test_model.py && python3 campaign.py --output campaign-result-ubuntu.json"

.\linode-runner.ps1 download --overwrite `
  /opt/e-rust-port/source/experiments/2026-07-30-012-persistent-satcheck-design/campaign-result-ubuntu.json `
  experiments/2026-07-30-012-persistent-satcheck-design/campaign-result.json
```

The unchanged production boundaries were checked with all features and the
pinned CaDiCaL source:

```powershell
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-3.0.1 cargo test --locked --all-features satcheck"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-3.0.1 cargo test --locked --all-features permanent_clauses_survive_and_assumptions_expire"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-3.0.1 cargo test --locked --all-features internal_service_keeps_permanent_clauses_and_drops_assumptions"
.\linode-runner.ps1 exec -- `
  "cd /opt/e-rust-port/source && UMLAUT_CADICAL_SOURCE=/opt/e-rust-port/cadical-3.0.1 cargo test --locked --all-features complete_models_and_failed_cores_are_validated"
```

## Documentation and repository checks

```powershell
python tools/c_source_docs/check_markdown_links.py
git diff --check
```
