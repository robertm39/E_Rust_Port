#!/usr/bin/env python3
"""Compare stable watchlist, subsumption-index, fingerprint, and PDT surfaces."""

from __future__ import annotations

import argparse
import hashlib
import json
import shlex
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


def quote_wsl_shell_metacharacters(argument: str) -> str:
    if any(character in argument for character in "<>|&;()$`"):
        return shlex.quote(argument)
    return argument


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args()

    experiment_dir = Path(__file__).resolve().parent
    input_path = experiment_dir / "input.lop"
    watch_path = experiment_dir / "watch.lop"
    inline_path = experiment_dir / "inline.p"
    index_path = experiment_dir / "index.lop"
    known_paths = [input_path, watch_path, inline_path, index_path]

    cases = [
        (
            "dynamic_watchlist",
            [
                "--lop-in",
                "--no-generation",
                f"--watchlist={watch_path}",
                str(input_path),
            ],
        ),
        (
            "static_watchlist",
            [
                "--lop-in",
                "--no-generation",
                f"--static-watchlist={watch_path}",
                str(input_path),
            ],
        ),
        (
            "watchlist_no_simplification",
            [
                "--lop-in",
                "--no-generation",
                "--no-watchlist-simplification",
                f"--watchlist={watch_path}",
                str(input_path),
            ],
        ),
        (
            "inline_watchlist_explicit",
            [
                "--no-generation",
                "--watchlist=Use inline watchlist type",
                str(inline_path),
            ],
        ),
        (
            "inline_watchlist_default",
            ["--no-generation", "--watchlist", str(inline_path)],
        ),
        (
            "combined_index_configuration",
            [
                "--lop-in",
                "--no-generation",
                "--fw-subsumption-aggressive",
                "--subsumption-indexing=PermOpt",
                "--fvindex-featuretypes=BillPlus",
                "--fvindex-maxfeatures=200",
                "--fvindex-slack=3",
                "--fp-index=FP7M",
                str(index_path),
            ],
        ),
        (
            "split_fingerprint_and_pdt_configuration",
            [
                "--lop-in",
                "--no-generation",
                "--rw-bw-index=NPDT",
                "--pm-from-index=NoIndex",
                "--pm-into-index=NPDT",
                "--pdt-no-size-constr",
                "--pdt-no-age-constr",
                str(index_path),
            ],
        ),
        (
            "conventional_subsumption",
            [
                "--lop-in",
                "--no-generation",
                "--conventional-subsumption",
                str(index_path),
            ],
        ),
        (
            "subsumption_order_dependence",
            [
                "--lop-in",
                "--no-generation",
                "--subsumption-indexing=PermOpt",
                "--subsumption-indexing=Direct",
                str(index_path),
            ],
        ),
        ("invalid_subsumption_index", ["--subsumption-indexing=Missing"]),
        ("invalid_feature_type", ["--fvindex-featuretypes=Missing"]),
        ("invalid_max_features", ["--fvindex-maxfeatures=0"]),
        ("invalid_fingerprint_index", ["--fp-index=Missing"]),
    ]

    path_map = {str(path): wsl_path(path) for path in known_paths}

    def to_c_arg(argument: str) -> str:
        if argument in path_map:
            return path_map[argument]
        for windows_path, linux_path in path_map.items():
            suffix = f"={windows_path}"
            if argument.endswith(suffix):
                return f"{argument[:-len(suffix)]}={linux_path}"
        return argument

    results: list[dict[str, object]] = []
    for case_name, rust_args in cases:
        rust = run([str(args.rust_exe.resolve()), *rust_args])
        c_args = [to_c_arg(argument) for argument in rust_args]
        c = run(
            [
                "wsl",
                "-d",
                args.distro,
                "--",
                args.c_exe,
                *(quote_wsl_shell_metacharacters(argument) for argument in c_args),
            ]
        )
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
