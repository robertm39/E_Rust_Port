#!/usr/bin/env python3
"""Validate every re-embedded formula with a separate Vampire TFA parser."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any, Sequence

import adapter
import run


ROOT = Path(__file__).resolve().parent
ERROR_MARKERS = (
    "User error",
    "Exception at proof loading",
    "Parsing Error",
    "Type error",
    "SyntaxError",
    "TypeError",
)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def validate_case(
    *,
    name: str,
    source: str,
    vampire: Path,
    output_root: Path,
) -> dict[str, Any]:
    result = adapter.adapt(source)
    problem_path = output_root / "problems" / f"{name}.p"
    stdout_path = output_root / "stdout" / f"{name}.txt"
    stderr_path = output_root / "stderr" / f"{name}.txt"
    for path in [problem_path, stdout_path, stderr_path]:
        path.parent.mkdir(parents=True, exist_ok=True)
    problem_path.write_text(result["reembedded_tff"], encoding="utf-8")
    completed = subprocess.run(
        [
            str(vampire),
            "--mode",
            "tclausify",
            "--time_limit",
            "2s",
            str(problem_path),
        ],
        cwd=output_root,
        check=False,
        capture_output=True,
        timeout=5,
    )
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)
    combined = (completed.stdout + completed.stderr).decode(
        "utf-8", errors="replace"
    )
    markers = [marker for marker in ERROR_MARKERS if marker in combined]
    return {
        "name": name,
        "canonical_id": result["canonical_id"],
        "problem_sha256": sha256_file(problem_path),
        "stdout_sha256": sha256_bytes(completed.stdout),
        "stderr_sha256": sha256_bytes(completed.stderr),
        "returncode": completed.returncode,
        "error_markers": markers,
        "valid": completed.returncode == 0 and not markers,
    }


def validate(vampire: Path, output_root: Path) -> dict[str, Any]:
    cases = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))
    version = subprocess.run(
        [str(vampire), "--version"],
        check=False,
        capture_output=True,
        timeout=10,
    )
    records = []
    for case in cases["accepted"]:
        records.append(
            {
                "population": "frozen",
                **validate_case(
                    name=f"frozen-{case['name']}",
                    source=case["source"],
                    vampire=vampire,
                    output_root=output_root,
                ),
            }
        )
    for index, source in enumerate(run.generated_sources(), start=1):
        records.append(
            {
                "population": "generated",
                **validate_case(
                    name=f"generated-{index:03d}",
                    source=source,
                    vampire=vampire,
                    output_root=output_root,
                ),
            }
        )
    failures = [record["name"] for record in records if not record["valid"]]
    body = {
        "schema_version": 1,
        "vampire_sha256": sha256_file(vampire),
        "vampire_version_returncode": version.returncode,
        "vampire_version_stdout": version.stdout.decode(
            "utf-8", errors="replace"
        ).strip(),
        "vampire_version_stderr": version.stderr.decode(
            "utf-8", errors="replace"
        ).strip(),
        "frozen_count": sum(
            record["population"] == "frozen" for record in records
        ),
        "generated_count": sum(
            record["population"] == "generated" for record in records
        ),
        "valid_count": sum(record["valid"] for record in records),
        "failures": failures,
        "records": records,
    }
    return {
        **body,
        "report_id": hashlib.sha256(adapter.canonical_json(body)).hexdigest(),
    }


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vampire", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    output_root = arguments.output_root.resolve()
    output_root.mkdir(parents=True, exist_ok=True)
    report = validate(arguments.vampire.resolve(), output_root)
    report_path = output_root / "vampire-validation.json"
    report_path.write_bytes(adapter.canonical_json(report) + b"\n")
    print(
        f"RESULT: {report['valid_count']}/"
        f"{report['frozen_count'] + report['generated_count']} valid; "
        f"report {report['report_id']}"
    )
    return 0 if not report["failures"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
