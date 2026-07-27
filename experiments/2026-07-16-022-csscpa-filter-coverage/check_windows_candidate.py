#!/usr/bin/env python3
"""Exercise the expanded CSSCPA cases against the optimized Windows binary."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import sys
import tempfile


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=20)
    return parser.parse_args()


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def main() -> int:
    args = parse_args()
    repo_root = args.repo_root.resolve()
    sys.path.insert(0, str(repo_root / "tools" / "e-interop"))
    import e_interop  # pylint: disable=import-error,import-outside-toplevel

    binary = args.binary.resolve()
    if not binary.is_file():
        raise FileNotFoundError(binary)
    cases = {
        case["scenario"]: case
        for case in e_interop.tool_comparison_cases(["CSSCPA_filter"])
    }
    large_case = cases["large-stateful-corpus"]
    missing_case = cases["missing-input"]

    with tempfile.TemporaryDirectory(prefix="csscpa-candidate-") as directory:
        workdir = Path(directory)
        large = e_interop.execute(
            binary,
            large_case["arguments"],
            timeout=30,
            env=os.environ.copy(),
            stdin_text=large_case["stdin"],
            cwd=workdir,
        )
        missing = e_interop.execute(
            binary,
            missing_case["arguments"],
            timeout=30,
            env=os.environ.copy(),
            stdin_text=missing_case["stdin"],
            cwd=workdir,
        )

        if large["exit_code"] != 0 or large["stderr"]:
            raise RuntimeError("large CSSCPA candidate case failed")
        if large["stdout"].count("rejected (subsumed by") != 12:
            raise RuntimeError("large CSSCPA subsumption count changed")
        if large["stdout"].count("rejected (Tautology)") != 4:
            raise RuntimeError("large CSSCPA tautology count changed")
        if large["stdout"].count("accepted from 0 (improved)") != 8:
            raise RuntimeError("large CSSCPA improvement count changed")
        if large["stdout"].count("accepted from 0 (contradicts)") != 4:
            raise RuntimeError("large CSSCPA contradiction count changed")
        if large["stdout"].count("rejected (weighty)") != 4:
            raise RuntimeError("large CSSCPA weight rejection count changed")
        if "% CSSCPAState: requested  by 0, 44, 44," not in large["stdout"]:
            raise RuntimeError("large CSSCPA final state changed")

        normalized_missing = e_interop.normalize_output(missing["stderr"])
        if missing["exit_code"] != 6:
            raise RuntimeError("missing-input CSSCPA exit status changed")
        if not normalized_missing.endswith("CSSCPA_filter: <OS ERROR: NOT FOUND>"):
            raise RuntimeError("missing-input CSSCPA diagnostic changed")

        timings = []
        for _ in range(args.repetitions):
            result = e_interop.execute(
                binary,
                large_case["arguments"],
                timeout=30,
                env=os.environ.copy(),
                stdin_text=large_case["stdin"],
                cwd=workdir,
            )
            if result["exit_code"] != 0:
                raise RuntimeError("timed CSSCPA candidate run failed")
            timings.append(result["wall_seconds"])

    report = {
        "schema_version": 1,
        "binary": str(binary),
        "repetitions": args.repetitions,
        "large_stateful_corpus": {
            "commands": 72,
            "exit_code": large["exit_code"],
            "stdout_bytes": len(large["stdout"].encode("utf-8")),
            "stdout_sha256": sha256(large["stdout"]),
            "stderr_bytes": len(large["stderr"].encode("utf-8")),
            "median_wall_seconds": statistics.median(timings),
            "minimum_wall_seconds": min(timings),
            "maximum_wall_seconds": max(timings),
        },
        "missing_input": {
            "exit_code": missing["exit_code"],
            "stdout_bytes": len(missing["stdout"].encode("utf-8")),
            "stderr_bytes": len(missing["stderr"].encode("utf-8")),
            "normalized_stderr": normalized_missing,
        },
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
