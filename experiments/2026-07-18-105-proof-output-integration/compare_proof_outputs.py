#!/usr/bin/env python3
"""Compare stable production proof-output surfaces in upstream C and Rust."""

from __future__ import annotations

import argparse
import difflib
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


COMMON = ["--auto", "--silent", "--detsort-rw", "--detsort-new"]

CASES: dict[str, dict[str, Any]] = {
    "mixed_tstp": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-object=1"],
    },
    "mixed_pcl": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-object=1", "--pcl-out"],
    },
    "mixed_full_derivation": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-object=1", "--full-deriv"],
    },
    "mixed_graph_1": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-graph=1"],
    },
    "mixed_graph_2": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-graph=2"],
    },
    "mixed_graph_2_gc": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-graph=2", "--record-gcs"],
    },
    "mixed_statistics_gc": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-object=1", "--proof-statistics", "--record-gcs"],
    },
    "mixed_training": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-object=1", "--training-examples=3"],
    },
    "statistics_without_list": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-statistics", "--record-gcs"],
    },
    "clause_tstp": {
        "fixture": "clause_refutation.p",
        "args": ["--proof-object=1"],
    },
    "saturation_tstp": {
        "fixture": "saturation.p",
        "args": ["--proof-object=1"],
    },
    "saturation_graph_2": {
        "fixture": "saturation.p",
        "args": ["--proof-graph=2"],
    },
    "forced_derivation_1": {
        "fixture": "mixed_refutation.p",
        "args": [
            "--proof-object=1",
            "--force-deriv=1",
            "--processed-clauses-limit=1",
        ],
    },
    "forced_derivation_2": {
        "fixture": "mixed_refutation.p",
        "args": [
            "--proof-object=1",
            "--force-deriv=2",
            "--processed-clauses-limit=1",
        ],
    },
    "proof_object_zero": {
        "fixture": "mixed_refutation.p",
        "args": ["--proof-object=0"],
    },
}


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def normalize_stream(stream: bytes) -> str:
    return stream.decode("utf-8", errors="replace").replace("\r\n", "\n")


def run_cases(executable: str, fixture_dir: Path) -> dict[str, dict[str, Any]]:
    results: dict[str, dict[str, Any]] = {}
    for name, case in CASES.items():
        process = subprocess.run(
            [executable, *COMMON, *case["args"], case["fixture"]],
            cwd=fixture_dir,
            check=False,
            capture_output=True,
            timeout=120,
        )
        results[name] = {
            "exit_code": process.returncode,
            "stdout": normalize_stream(process.stdout),
            "stderr": normalize_stream(process.stderr),
        }
    return results


def stream_summary(stream: str) -> dict[str, Any]:
    encoded = stream.encode("utf-8")
    return {
        "bytes": len(encoded),
        "sha256": hashlib.sha256(encoded).hexdigest(),
    }


def summarize(results: dict[str, dict[str, Any]]) -> dict[str, dict[str, Any]]:
    return {
        name: {
            "exit_code": result["exit_code"],
            "stdout": stream_summary(result["stdout"]),
            "stderr": stream_summary(result["stderr"]),
        }
        for name, result in results.items()
    }


def effect_assertions(results: dict[str, dict[str, Any]]) -> dict[str, bool]:
    stdout = {name: result["stdout"] for name, result in results.items()}
    return {
        "mixed_formula_clause_list": (
            "% SZS output start CNFRefutation" in stdout["mixed_tstp"]
            and "fof(" in stdout["mixed_tstp"]
            and "cnf(" in stdout["mixed_tstp"]
        ),
        "pcl_list_selected": " : initial(" in stdout["mixed_pcl"],
        "full_derivation_adds_irrelevant_root": (
            "irrelevant" not in stdout["mixed_tstp"]
            and "irrelevant" in stdout["mixed_full_derivation"]
        ),
        "graph_level_1_selected": "digraph proof{" in stdout["mixed_graph_1"],
        "graph_level_2_has_derivations": (
            "digraph proof{" in stdout["mixed_graph_2"]
            and "\\ninference(" in stdout["mixed_graph_2"]
        ),
        "gc_graph_has_selected_nodes": (
            "digraph proof{" in stdout["mixed_graph_2_gc"]
            and "shape=ellipse" in stdout["mixed_graph_2_gc"]
        ),
        "gc_statistics_reported": (
            "% Proof object total steps" in stdout["mixed_statistics_gc"]
            and "inference(evalgc" in stdout["mixed_statistics_gc"]
        ),
        "training_polarities_reported": (
            "% Training: Positive examples begin" in stdout["mixed_training"]
            and "% Training: Negative examples begin" in stdout["mixed_training"]
        ),
        "statistics_without_list": (
            "% Proof object total steps" in stdout["statistics_without_list"]
            and "% SZS output start" not in stdout["statistics_without_list"]
        ),
        "clause_only_refutation": (
            "% SZS output start CNFRefutation" in stdout["clause_tstp"]
        ),
        "saturation_list": "% SZS output start Saturation" in stdout["saturation_tstp"],
        "saturation_graph": "digraph proof{" in stdout["saturation_graph_2"],
        "forced_level_1": "% SZS output start Derivation" in stdout["forced_derivation_1"],
        "forced_level_2_adds_roots": (
            "% SZS output start Derivation" in stdout["forced_derivation_2"]
            and stdout["forced_derivation_1"] != stdout["forced_derivation_2"]
        ),
        "proof_object_zero_suppresses_output": (
            "% SZS status Theorem" in stdout["proof_object_zero"]
            and "% SZS output start" not in stdout["proof_object_zero"]
            and "digraph proof{" not in stdout["proof_object_zero"]
        ),
    }


def first_difference(
    c_results: dict[str, dict[str, Any]], rust_results: dict[str, dict[str, Any]]
) -> str | None:
    for name in CASES:
        c_result = c_results[name]
        rust_result = rust_results[name]
        if c_result == rust_result:
            continue
        if c_result["exit_code"] != rust_result["exit_code"]:
            return (
                f"{name}: exit code C={c_result['exit_code']} "
                f"Rust={rust_result['exit_code']}"
            )
        for stream in ("stdout", "stderr"):
            if c_result[stream] != rust_result[stream]:
                return "\n".join(
                    difflib.unified_diff(
                        c_result[stream].splitlines(),
                        rust_result[stream].splitlines(),
                        fromfile=f"C/{name}/{stream}",
                        tofile=f"Rust/{name}/{stream}",
                        lineterm="",
                        n=4,
                    )
                )
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--worker", action="store_true")
    parser.add_argument("--exe")
    parser.add_argument("--fixture-dir", type=Path)
    parser.add_argument("--c-exe")
    parser.add_argument("--rust-exe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    fixture_dir = Path(__file__).resolve().parent
    if args.worker:
        if args.exe is None or args.fixture_dir is None:
            parser.error("--worker requires --exe and --fixture-dir")
        sys.stdout.write(json.dumps(run_cases(args.exe, args.fixture_dir), sort_keys=True))
        return 0
    if args.c_exe is None or args.rust_exe is None or args.output is None:
        parser.error("comparison mode requires --c-exe, --rust-exe, and --output")

    rust_results = run_cases(str(args.rust_exe.resolve()), fixture_dir)
    worker = subprocess.run(
        [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            "python3",
            windows_to_wsl(Path(__file__)),
            "--worker",
            "--exe",
            args.c_exe,
            "--fixture-dir",
            windows_to_wsl(fixture_dir),
        ],
        check=False,
        capture_output=True,
        timeout=900,
    )
    if worker.returncode != 0:
        sys.stderr.buffer.write(worker.stderr)
        return worker.returncode

    c_results = json.loads(worker.stdout.decode("utf-8"))
    difference = first_difference(c_results, rust_results)
    effects = effect_assertions(rust_results)
    report = {
        "schema_version": 1,
        "upstream_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
        "case_count": len(CASES),
        "all_exact": difference is None,
        "all_effects_observed": all(effects.values()),
        "effect_assertions": effects,
        "cases": summarize(rust_results),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if args.expected is not None and encoded != args.expected.read_text(encoding="utf-8"):
        print("proof-output reference changed", file=sys.stderr)
        return 1
    if difference is not None:
        print(difference, file=sys.stderr)
        return 1
    if not report["all_effects_observed"]:
        failed = [name for name, observed in effects.items() if not observed]
        print(f"missing proof-output effects: {', '.join(failed)}", file=sys.stderr)
        return 1
    print(f"validated {len(CASES)}/{len(CASES)} exact C/Rust proof-output cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
