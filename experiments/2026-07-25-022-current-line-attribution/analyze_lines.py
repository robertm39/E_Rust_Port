#!/usr/bin/env python3
"""Summarize self-instruction costs from a line-table Callgrind profile."""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
from collections.abc import Iterable


DEFINITION = re.compile(
    r"^(?P<kind>fl|fi|fe|fn|cfi|cfn)=\((?P<identifier>\d+)\)(?: (?P<value>.*))?$"
)
COST = re.compile(r"^(?P<line>[+*-]?\d+|\*)\s+(?P<instructions>\d+)$")


def definitions(lines: Iterable[str]) -> tuple[dict[int, str], dict[int, str]]:
    """Collect compressed file and function-name definitions."""
    files: dict[int, str] = {}
    functions: dict[int, str] = {}
    for raw_line in lines:
        match = DEFINITION.match(raw_line.rstrip("\n"))
        if match is None or match.group("value") is None:
            continue
        identifier = int(match.group("identifier"))
        if match.group("kind") in {"fl", "fi", "fe", "cfi"}:
            files[identifier] = match.group("value")
        else:
            functions[identifier] = match.group("value")
    return files, functions


def compressed_position(token: str, previous: int) -> int:
    """Expand one Callgrind line-position token."""
    if token == "*":
        return previous
    if token.startswith("+"):
        return previous + int(token[1:])
    if token.startswith("-"):
        return previous - int(token[1:])
    return int(token)


def source_path(profile_path: str) -> str | None:
    """Map a worker source path to its repository-relative form."""
    marker = "/source/"
    normalized = profile_path.replace("\\", "/")
    if marker not in normalized:
        return None
    relative = normalized.split(marker, maxsplit=1)[1]
    return relative if relative.startswith("src/") else None


def source_text(
    source_root: pathlib.Path, relative_path: str, line_number: int
) -> str:
    """Return a compact source line when the mapped line exists."""
    if line_number <= 0:
        return ""
    path = source_root / pathlib.PurePosixPath(relative_path)
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return ""
    if line_number > len(lines):
        return ""
    return lines[line_number - 1].strip()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("profile", type=pathlib.Path)
    parser.add_argument("--source-root", type=pathlib.Path, required=True)
    parser.add_argument("--limit", type=int, default=100)
    args = parser.parse_args()

    raw_lines = args.profile.read_text(encoding="utf-8").splitlines()
    files, functions = definitions(raw_lines)

    current_file: int | None = None
    current_function: int | None = None
    current_line = 0
    skip_call_cost = False
    by_line: collections.Counter[tuple[str, int, str]] = collections.Counter()
    by_file: collections.Counter[str] = collections.Counter()
    by_function: collections.Counter[tuple[str, str]] = collections.Counter()

    for raw_line in raw_lines:
        definition = DEFINITION.match(raw_line)
        if definition is not None:
            identifier = int(definition.group("identifier"))
            if definition.group("kind") in {"fl", "fi", "fe"}:
                current_file = identifier
            elif definition.group("kind") == "fn":
                current_function = identifier
            continue
        if raw_line.startswith("calls="):
            skip_call_cost = True
            continue

        cost = COST.match(raw_line)
        if cost is None:
            continue
        current_line = compressed_position(cost.group("line"), current_line)
        if skip_call_cost:
            skip_call_cost = False
            continue
        if current_file is None:
            continue
        relative_path = source_path(files.get(current_file, ""))
        if relative_path is None:
            continue

        instruction_count = int(cost.group("instructions"))
        function_name = functions.get(
            current_function, f"<function {current_function}>"
        )
        key = (relative_path, current_line, function_name)
        by_line[key] += instruction_count
        by_file[relative_path] += instruction_count
        by_function[(relative_path, function_name)] += instruction_count

    print("SOURCE FILE SELF INSTRUCTIONS")
    for path, instructions in by_file.most_common():
        print(f"{instructions:>12}  {path}")

    print("\nSOURCE FUNCTION SELF INSTRUCTIONS")
    for (path, function), instructions in by_function.most_common(args.limit):
        print(f"{instructions:>12}  {path}  {function}")

    print("\nSOURCE LINE SELF INSTRUCTIONS")
    for (path, line, function), instructions in by_line.most_common(args.limit):
        text = source_text(args.source_root, path, line)
        print(f"{instructions:>12}  {path}:{line}  {function}")
        if text:
            print(f"{'':>14}{text}")


if __name__ == "__main__":
    main()
