#!/usr/bin/env python3
"""Compare C and Rust eprover verbose progress streams."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


UPSTREAM_COMMIT = "17026b1bfe61aaf223cfaae54947c8d2679c31a0"


@dataclass(frozen=True)
class Case:
    name: str
    input_name: str
    args: tuple[str, ...]
    configured_output: bool = False


CASES = (
    Case("verbose_level_one", "simple.p", ("--verbose=1", "--silent", "--cnf", "--tstp-in")),
    Case("verbose_level_two", "simple.p", ("--verbose=2", "--silent", "--cnf", "--tstp-in")),
    Case("verbose_negative", "simple.p", ("--verbose=-1", "--silent", "--cnf", "--tstp-in")),
    Case(
        "gsine_seed_count",
        "simple.p",
        (
            "--verbose=1",
            "--silent",
            "--cnf",
            "--tstp-in",
            "--sine=gf500_gu_R04_F100_L20000",
        ),
    ),
    Case("recursive_include", "include-main.p", ("--verbose=2", "--silent", "--cnf", "--tstp-in")),
    Case(
        "manual_lpo",
        "simple.p",
        ("--verbose=1", "--silent", "--cnf", "--tstp-in", "--term-ordering=LPO"),
    ),
    Case(
        "relevance_pruning",
        "simple.p",
        ("--verbose=1", "--silent", "--cnf", "--tstp-in", "--rel-pruning-level=1"),
    ),
    Case("syntax_only", "simple.p", ("--verbose=2", "--silent", "--syntax-only", "--tstp-in")),
    Case(
        "configured_output",
        "simple.p",
        ("--verbose=2", "--silent", "--cnf", "--tstp-in"),
        configured_output=True,
    ),
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def normalized_stream(data: bytes, replacements: dict[bytes, bytes]) -> bytes:
    result = data
    for source in sorted(replacements, key=len, reverse=True):
        result = result.replace(source, replacements[source])
    return result


def path_replacements(path: Path, label: bytes) -> dict[bytes, bytes]:
    resolved = path.resolve()
    native = str(resolved).encode()
    slash = resolved.as_posix().encode()
    wsl = windows_to_wsl(resolved).encode()
    relative = str(path).encode()
    relative_slash = path.as_posix().encode()
    return {
        native: label,
        slash: label,
        wsl: label,
        relative: label,
        relative_slash: label,
    }


def stream_result(data: bytes) -> dict[str, Any]:
    return {
        "bytes": len(data),
        "lines": len(data.splitlines()),
        "sha256": sha256(data),
    }


def captured_result(
    process: subprocess.CompletedProcess[bytes],
    output_path: Path | None,
    replacements: dict[bytes, bytes],
) -> tuple[dict[str, Any], tuple[bytes, bytes, bytes]]:
    stdout = normalized_stream(process.stdout, replacements)
    stderr = normalized_stream(process.stderr, replacements)
    output_created = output_path is not None and output_path.is_file()
    configured = output_path.read_bytes() if output_created and output_path is not None else b""
    configured = normalized_stream(configured, replacements)
    report = {
        "exit_code": process.returncode,
        "stdout": stream_result(stdout),
        "stderr": stream_result(stderr),
        "configured_output_created": output_created,
        "configured_output": stream_result(configured),
    }
    return report, (stdout, stderr, configured)


def executable_hash(path: str | Path, distro: str, is_wsl: bool) -> str:
    if is_wsl:
        process = run(["wsl.exe", "-d", distro, "--exec", "sha256sum", str(path)])
        if process.returncode != 0:
            raise RuntimeError(process.stderr.decode("utf-8", errors="replace"))
        return process.stdout.decode("ascii").split()[0]
    return sha256(Path(path).resolve().read_bytes())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    experiment_dir = Path(__file__).resolve().parent
    input_dir = experiment_dir / "inputs"
    reports = []
    for case in CASES:
        input_path = input_dir / case.input_name
        c_input = windows_to_wsl(input_path)
        rust_input = str(input_path.resolve())
        c_output = args.output.resolve().parent / f"verbose-{case.name}-c.out"
        rust_output = args.output.resolve().parent / f"verbose-{case.name}-rust.out"
        c_output.unlink(missing_ok=True)
        rust_output.unlink(missing_ok=True)

        c_command = [
            "wsl.exe",
            "-d",
            args.distro,
            "--exec",
            args.c_exe,
            *case.args,
        ]
        rust_command = [str(args.rust_exe.resolve()), *case.args]
        if case.configured_output:
            c_command.append(f"--output-file={windows_to_wsl(c_output)}")
            rust_command.append(f"--output-file={rust_output.resolve()}")
        c_command.append(c_input)
        rust_command.append(rust_input)

        replacements: dict[bytes, bytes] = {}
        replacements.update(path_replacements(input_path, b"<INPUT>"))
        replacements.update(path_replacements(input_dir / "included.ax", b"<INCLUDE>"))
        replacements.update(path_replacements(c_output, b"<OUTPUT>"))
        replacements.update(path_replacements(rust_output, b"<OUTPUT>"))

        c_process = run(c_command)
        rust_process = run(rust_command)
        c_report, c_streams = captured_result(
            c_process,
            c_output if case.configured_output else None,
            replacements,
        )
        rust_report, rust_streams = captured_result(
            rust_process,
            rust_output if case.configured_output else None,
            replacements,
        )
        reports.append(
            {
                "name": case.name,
                "args": [*case.args, "$INPUT"],
                "configured_output": case.configured_output,
                "normalization": "input, recursive-include, and configured-output path spelling only",
                "c": c_report,
                "rust": rust_report,
                "all_exact": c_report == rust_report and c_streams == rust_streams,
            }
        )

    report = {
        "schema_version": 1,
        "upstream_commit": UPSTREAM_COMMIT,
        "c_executable_sha256": executable_hash(args.c_exe, args.distro, True),
        "rust_executable_sha256": executable_hash(args.rust_exe, args.distro, False),
        "case_count": len(reports),
        "matching_cases": sum(case["all_exact"] for case in reports),
        "cases": reports,
        "all_exact": all(case["all_exact"] for case in reports),
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    args.output.write_text(encoded, encoding="utf-8", newline="\n")

    if not report["all_exact"]:
        failed = ", ".join(case["name"] for case in reports if not case["all_exact"])
        print(f"verbose progress comparison failed: {failed}", file=sys.stderr)
        return 1
    if args.expected is not None:
        expected = args.expected.read_text(encoding="utf-8")
        if encoded != expected:
            print("verbose progress reference changed", file=sys.stderr)
            return 1
    print(f"matched {report['matching_cases']}/{report['case_count']} verbose cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
