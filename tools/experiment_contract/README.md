# Experiment-result contract tools

`validate.py` validates the lightweight experiment-result contract documented
in [`docs/experiment-contract.md`](../../docs/experiment-contract.md). The
machine-readable shape is `experiment-result.schema.json`; `template.json` is
a valid starting record.

From the repository root:

```text
python tools/experiment_contract/validate.py path/to/result.json
python tools/experiment_contract/validate.py --verify-artifacts path/to/result.json
python -m unittest tools/experiment_contract/test_validate.py
```

The validator uses only the Python standard library. Artifact paths are
repository-relative POSIX paths. `--verify-artifacts` checks containment,
existence, byte length, and SHA-256.
