#!/usr/bin/env bash
set -euo pipefail

repo="$(pwd)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

python3 - "$repo" "$scratch/eprover" <<'PY'
import sys
from pathlib import Path

repo = Path(sys.argv[1])
destination = Path(sys.argv[2])
sys.path.insert(0, str(repo / "tools" / "linode-runner"))
from linux_compat import prepare_reference_source

prepare_reference_source(repo / "eprover", destination)
PY
(
  cd "$scratch/eprover"
  ./configure >/dev/null
  make -j"$(nproc)" >/dev/null
)

cargo build --locked --release --bin eprover >/dev/null

python3 - \
  "$scratch/eprover/PROVER/eprover" \
  "$repo/target/release/eprover" \
  "$repo/experiments/2026-07-25-033-interpreted-completeness-boundary/interpreted.lop" <<'PY'
import hashlib
import json
import subprocess
import sys

c_exe, rust_exe, fixture = sys.argv[1:]


def run(exe):
    completed = subprocess.run(
        [exe, "--lop-in", fixture],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    stdout = completed.stdout.decode("utf-8", errors="replace")
    stderr = completed.stderr.decode("utf-8", errors="replace")
    completion = [
        line
        for line in stdout.splitlines()
        if line == "% Clause set closed under restricted calculus!"
    ]
    status = [line for line in stdout.splitlines() if line.startswith("% SZS status ")]
    return {
        "exit_code": completed.returncode,
        "stderr": stderr,
        "stdout_sha256": hashlib.sha256(completed.stdout).hexdigest(),
        "completion": completion,
        "status": status,
    }


c = run(c_exe)
rust = run(rust_exe)
expected = {
    "exit_code": 10,
    "stderr": "",
    "completion": ["% Clause set closed under restricted calculus!"],
    "status": ["% SZS status GaveUp"],
}
fields = ("exit_code", "stderr", "completion", "status")
all_exact = all(c[field] == expected[field] == rust[field] for field in fields)
report = {
    "schema_version": 1,
    "all_exact": all_exact,
    "expected": expected,
    "c": c,
    "rust": rust,
}
print(json.dumps(report, indent=2, sort_keys=True))
if not all_exact:
    raise SystemExit(1)
PY
