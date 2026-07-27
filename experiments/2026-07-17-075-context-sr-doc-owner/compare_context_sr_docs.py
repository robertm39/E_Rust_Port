#!/usr/bin/env python3
"""Compare focused contextual simplify-reflect proof events in C and Rust."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any


EVENT_LINE = re.compile(r"^cnf\((c_0_-?\d+), plain, (.+?),(.*)\)\.$")
EVENT_REF = re.compile(r"c_0_-?\d+")
COUNTER_NAMES = (
    "% Processed clauses",
    "% Backward-subsumed",
    "% Contextual simplify-reflections",
)


def windows_to_wsl(path: Path) -> str:
    resolved = path.resolve()
    drive = resolved.drive
    if len(drive) != 2 or drive[1] != ":":
        raise ValueError(f"expected a drive-qualified Windows path, got {resolved}")
    return f"/mnt/{drive[0].lower()}{resolved.as_posix()[2:]}"


def run(command: list[str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(command, check=False, capture_output=True, timeout=120)


def event_kind(source: str) -> str | None:
    if "inference(csr" in source:
        return "csr"
    if "'new_given'" in source:
        return "new_given"
    if "'subsumed(" in source:
        return "subsumed"
    if "'exists'" in source:
        return "exists"
    return None


def extract_events(stdout: str) -> list[dict[str, Any]]:
    lines = stdout.splitlines()
    scan_index = lines.index("% Scanning for AC axioms")
    event_ids: dict[str, int] = {}
    events: list[dict[str, Any]] = []
    for line in lines[scan_index + 1 :]:
        if not line:
            break
        match = EVENT_LINE.match(line)
        if match is None:
            continue
        ident, clause, source = match.groups()
        kind = event_kind(source)
        if kind is None:
            continue
        parent_roles = [
            f"event:{event_ids[reference]}" if reference in event_ids else "external"
            for reference in EVENT_REF.findall(source)
        ]
        events.append(
            {
                "kind": kind,
                "clause": clause.replace(" ", ""),
                "parent_roles": parent_roles,
            }
        )
        event_ids[ident] = len(events) - 1
    return events


def extract_counters(stdout: str) -> dict[str, int]:
    counters: dict[str, int] = {}
    for line in stdout.splitlines():
        for name in COUNTER_NAMES:
            if line.startswith(name):
                counters[name.removeprefix("% ")] = int(line.rsplit(":", maxsplit=1)[1])
    if len(counters) != len(COUNTER_NAMES):
        raise RuntimeError(f"missing focused counters: {counters}")
    return counters


def summarize(completed: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = completed.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = completed.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    events = extract_events(stdout)
    return {
        "exit_code": completed.returncode,
        "stderr": stderr,
        "events": events,
        "counters": extract_counters(stdout),
        "csr_event_count": sum(event["kind"] == "csr" for event in events),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    fixture = Path(__file__).resolve().parent / "context-sr.p"
    common_args = [
        "--output-level=6",
        "--no-generation",
        "--forward-context-sr",
        "--expert-heuristic=(1*FIFOWeight(ConstPrio))",
    ]
    c_result = summarize(
        run(
            ["wsl.exe", "-d", args.distro, "--exec", args.c_exe]
            + common_args
            + [windows_to_wsl(fixture)]
        )
    )
    rust_result = summarize(
        run([str(args.rust_exe.resolve())] + common_args + [str(fixture)])
    )
    focused_exact = c_result == rust_result and c_result["csr_event_count"] == 1
    report = {
        "schema_version": 1,
        "display_args": common_args + ["$FIXTURE"],
        "c": c_result,
        "rust": rust_result,
        "focused_exact": focused_exact,
    }
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )

    if args.expected is not None:
        expected = json.loads(args.expected.read_text(encoding="utf-8"))
        if report != expected:
            print("report does not match retained reference", file=sys.stderr)
            return 1
    if not focused_exact:
        print("focused contextual simplify-reflect events differ", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
