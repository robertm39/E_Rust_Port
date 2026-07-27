#!/usr/bin/env python3
"""Compare C/Rust app-encode typed-application declaration ownership."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


EXPECTED_TYPE_COMMENTS = {
    "%-- $o > $o.",
    "%-- ($o * $o) > $o.",
    "%-- $i > $o.",
    "%-- person.",
    "%-- (person * person) > person.",
    "%-- person > person.",
}


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


def normalized_sections(stdout: str) -> dict[str, object]:
    lines = stdout.splitlines()
    prelude: list[str] = []
    declarations: list[str] = []
    tail: list[str] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith("%-- "):
            if index + 1 >= len(lines) or not lines[index + 1].startswith(
                "tff(typedecl"
            ):
                raise ValueError(f"type comment lacks declaration: {line!r}")
            declaration = re.sub(r"typedecl\d+", "typedecl", lines[index + 1])
            declarations.append(f"{line}\n{declaration}")
            index += 2
        elif declarations:
            tail.append(line)
            index += 1
        else:
            prelude.append(line)
            index += 1

    return {
        "prelude": prelude,
        "type_declarations": sorted(declarations),
        "tail": tail,
    }


def type_comments(stdout: str) -> set[str]:
    return {line for line in stdout.splitlines() if line.startswith("%-- ")}


def wsl_path(path: Path) -> str:
    windows_path = path.resolve().as_posix()
    if len(windows_path) < 3 or windows_path[1:3] != ":/":
        raise ValueError(f"expected an absolute Windows path: {windows_path}")
    return f"/mnt/{windows_path[0].lower()}{windows_path[2:]}"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    input_path = Path(__file__).resolve().with_name("input.p")
    common = ["--app-encode", "--tstp-in"]
    rust = run([str(args.rust_exe.resolve()), *common, str(input_path)])
    c = run(
        [
            "wsl",
            "-d",
            args.distro,
            "--",
            args.c_exe,
            *common,
            wsl_path(input_path),
        ]
    )

    rust_comments = type_comments(str(rust["stdout"]))
    c_comments = type_comments(str(c["stdout"]))
    rust_normalized = normalized_sections(str(rust["stdout"]))
    c_normalized = normalized_sections(str(c["stdout"]))
    print(
        json.dumps(
            {
                "raw_stdout_match": c["stdout"] == rust["stdout"],
                "normalized_stdout_match": c_normalized == rust_normalized,
                "exit_and_stderr_match": c["exit_code"]
                == rust["exit_code"]
                == 0
                and c["stderr"] == rust["stderr"] == "",
                "c_type_comments_match_expected": c_comments
                == EXPECTED_TYPE_COMMENTS,
                "rust_type_comments_match_expected": rust_comments
                == EXPECTED_TYPE_COMMENTS,
                "c_unexpected_type_comments": sorted(
                    c_comments - EXPECTED_TYPE_COMMENTS
                ),
                "rust_unexpected_type_comments": sorted(
                    rust_comments - EXPECTED_TYPE_COMMENTS
                ),
                "c": c,
                "rust": rust,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
