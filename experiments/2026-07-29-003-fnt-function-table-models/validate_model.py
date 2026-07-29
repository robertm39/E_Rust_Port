#!/usr/bin/env python3
"""Run the positive-only solution gate with the typed Vampire adapter."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("problem", type=Path)
    parser.add_argument("solution", type=Path)
    parser.add_argument("--validator", type=Path, required=True)
    parser.add_argument("--vampire", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=float, default=120.0)
    args = parser.parse_args()

    adapter = Path(__file__).resolve().with_name("vampire_model_check.py")
    checker_command = [
        sys.executable,
        str(adapter),
        "--vampire",
        str(args.vampire.resolve()),
        "--timeout-seconds",
        str(args.timeout_seconds),
        "{problem}",
        "{artifact}",
    ]
    return subprocess.run(
        [
            sys.executable,
            str(args.validator.resolve()),
            str(args.problem.resolve()),
            str(args.solution.resolve()),
            "--model-command-json",
            json.dumps(checker_command),
            "--timeout-seconds",
            str(args.timeout_seconds),
            "--report",
            str(args.report.resolve()),
        ],
        check=False,
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
