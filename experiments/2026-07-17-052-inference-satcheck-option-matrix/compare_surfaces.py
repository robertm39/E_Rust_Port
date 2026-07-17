#!/usr/bin/env python3
"""Compare stable inference-processing and SAT-check option surfaces."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(command, check=False, capture_output=True)
    return {
        "exit_code": completed.returncode,
        "stdout": completed.stdout.decode("utf-8", errors="backslashreplace").replace(
            "\r\n", "\n"
        ),
        "stderr": completed.stderr.decode("utf-8", errors="backslashreplace").replace(
            "\r\n", "\n"
        ),
    }


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def digest(text: object) -> str:
    return hashlib.sha256(str(text).encode("utf-8")).hexdigest()


def summarize(result: dict[str, object]) -> dict[str, object]:
    stdout = str(result["stdout"])
    stderr = str(result["stderr"])
    return {
        "exit_code": result["exit_code"],
        "stdout_bytes": len(stdout.encode("utf-8")),
        "stdout_sha256": digest(stdout),
        "stderr_bytes": len(stderr.encode("utf-8")),
        "stderr_sha256": digest(stderr),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    experiment_dir = Path(__file__).resolve().parent
    rewrite = experiment_dir / "rewrite.lop"
    sat = experiment_dir / "sat.lop"
    path_map = {str(path): wsl_path(path) for path in (rewrite, sat)}

    cases = [
        (
            "combined_inference_processing",
            [
                "--lop-in",
                "--no-generation",
                "--sos-uses-input-types",
                "--destructive-er",
                "--strong-destructive-er",
                "--destructive-er-aggressive",
                "--forward-context-sr-aggressive",
                "--backward-context-sr",
                "--prefer-general-demodulators",
                "--forward-demod-level=1",
                "--demod-under-lambda=true",
                "--strong-rw-inst",
                "--strong-forward-subsumption",
                "--lift-lambdas=false",
                str(rewrite),
            ],
        ),
        (
            "aggressive_er_without_enable",
            [
                "--lop-in",
                "--no-generation",
                "--destructive-er-aggressive",
                str(rewrite),
            ],
        ),
        (
            "forward_context_sr",
            ["--lop-in", "--no-generation", "--forward-context-sr", str(rewrite)],
        ),
        (
            "forward_demod_disabled",
            ["--lop-in", "--no-generation", "--forward-demod-level=0", str(rewrite)],
        ),
        (
            "lift_lambdas_optional_default",
            ["--lop-in", "--no-generation", "--lift-lambdas", str(rewrite)],
        ),
        (
            "satcheck_unsat",
            [
                "--lop-in",
                "--satcheck=GlobalMin",
                "--satcheck-proc-interval=1",
                str(sat),
            ],
        ),
        (
            "satcheck_default_grounding",
            ["--lop-in", "--satcheck", "--satcheck-proc-interval=1", str(sat)],
        ),
        (
            "satcheck_plus_generation_trigger",
            [
                "--lop-in",
                "--satcheck=ConjMinMinFreq",
                "--satcheck-proc-interval=1",
                "--satcheck-gen-interval=1",
                str(sat),
            ],
        ),
        (
            "satcheck_plus_ttinsert_trigger",
            [
                "--lop-in",
                "--satcheck=ConjMinMinFreq",
                "--satcheck-proc-interval=1",
                "--satcheck-gen-interval=1",
                "--satcheck-ttinsert-interval=1",
                str(sat),
            ],
        ),
        (
            "satcheck_plus_decision_limit",
            [
                "--lop-in",
                "--satcheck=ConjMinMinFreq",
                "--satcheck-proc-interval=1",
                "--satcheck-gen-interval=1",
                "--satcheck-ttinsert-interval=1",
                "--satcheck-decision-limit=-1",
                str(sat),
            ],
        ),
        (
            "satcheck_plus_normalize_const",
            [
                "--lop-in",
                "--satcheck=ConjMinMinFreq",
                "--satcheck-proc-interval=1",
                "--satcheck-gen-interval=1",
                "--satcheck-ttinsert-interval=1",
                "--satcheck-decision-limit=-1",
                "--satcheck-normalize-const",
                str(sat),
            ],
        ),
        (
            "satcheck_all_controls",
            [
                "--lop-in",
                "--satcheck=ConjMinMinFreq",
                "--satcheck-proc-interval=1",
                "--satcheck-gen-interval=1",
                "--satcheck-ttinsert-interval=1",
                "--satcheck-decision-limit=-1",
                "--satcheck-normalize-const",
                "--satcheck-normalize-unproc",
                str(sat),
            ],
        ),
        (
            "satcheck_optional_defaults",
            [
                "--lop-in",
                "--satcheck",
                "--satcheck-proc-interval",
                "--satcheck-gen-interval",
                "--satcheck-ttinsert-interval",
                "--satcheck-decision-limit",
                str(sat),
            ],
        ),
        ("invalid_forward_demod", ["--forward-demod-level=3"]),
        ("invalid_demod_lambda", ["--demod-under-lambda=maybe"]),
        ("invalid_lift_lambdas", ["--lift-lambdas=maybe"]),
        ("invalid_satcheck", ["--satcheck=Missing"]),
        ("invalid_satcheck_interval", ["--satcheck-proc-interval=0"]),
        ("invalid_satcheck_decision_limit", ["--satcheck-decision-limit=-2"]),
    ]

    def to_c_arg(argument: str) -> str:
        return path_map.get(argument, argument)

    results: list[dict[str, object]] = []
    for case_name, rust_args in cases:
        rust = run([str(args.rust_exe.resolve()), *rust_args])
        c_args = [to_c_arg(argument) for argument in rust_args]
        c = run(["wsl", "-d", args.distro, "--", args.c_exe, *c_args])
        exact_match = rust == c
        result: dict[str, object] = {
            "case": case_name,
            "exact_match": exact_match,
            "rust": summarize(rust),
            "c": summarize(c),
        }
        if not exact_match:
            result["mismatch"] = {"rust": rust, "c": c}
        results.append(result)

    rendered = json.dumps(
        {
            "reference_commit": "17026b1bfe61aaf223cfaae54947c8d2679c31a0",
            "case_count": len(results),
            "exact_count": sum(bool(result["exact_match"]) for result in results),
            "results": results,
        },
        indent=2,
    )
    args.output.write_text(rendered + "\n", encoding="utf-8")
    if not args.quiet:
        print(rendered)


if __name__ == "__main__":
    main()
