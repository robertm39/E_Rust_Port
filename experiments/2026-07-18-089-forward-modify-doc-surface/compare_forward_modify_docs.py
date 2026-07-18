#!/usr/bin/env python3
"""Compare focused ForwardModifyClause proof events in C and Rust."""

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
    "% Total rewrite steps",
    "% Condensation successes",
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
    for kind in ("rw", "cn", "condense", "sr"):
        if f"inference({kind}" in source:
            return kind
    return None


def extract_events(stdout: str) -> list[dict[str, Any]]:
    event_ids: dict[str, int] = {}
    events: list[dict[str, Any]] = []
    for line in stdout.splitlines():
        match = EVENT_LINE.match(line)
        if match is None:
            continue
        ident, clause, source = match.groups()
        kind = event_kind(source)
        if kind is None:
            continue
        parents = [
            f"event:{event_ids[reference]}" if reference in event_ids else "external"
            for reference in EVENT_REF.findall(source)
        ]
        events.append(
            {
                "kind": kind,
                "clause": clause.replace(" ", ""),
                "parents": parents,
            }
        )
        event_ids[ident] = len(events) - 1
    return events


def extract_counters(stdout: str) -> dict[str, int]:
    counters: dict[str, int] = {}
    for line in stdout.splitlines():
        for name in COUNTER_NAMES:
            if line.startswith(name):
                counters[name.removeprefix("% ")] = int(line.rsplit(":", 1)[1])
    if len(counters) != len(COUNTER_NAMES):
        raise RuntimeError(f"missing focused counters: {counters}")
    return counters


def summarize(completed: subprocess.CompletedProcess[bytes]) -> dict[str, Any]:
    stdout = completed.stdout.decode("utf-8", errors="replace").replace("\r\n", "\n")
    stderr = completed.stderr.decode("utf-8", errors="replace").replace("\r\n", "\n")
    return {
        "exit_code": completed.returncode,
        "stderr": stderr,
        "events": extract_events(stdout),
        "counters": extract_counters(stdout),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-exe", required=True)
    parser.add_argument("--rust-exe", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--expected", type=Path)
    parser.add_argument("--distro", default="Ubuntu-24.04")
    args = parser.parse_args()

    experiment = Path(__file__).resolve().parent
    common = [
        "--output-level=6",
        "--print-statistics",
        "--no-preprocessing",
        "--no-generation",
        "--expert-heuristic=(1*FIFOWeight(ConstPrio))",
        "--detsort-rw",
        "--detsort-new",
    ]
    cases = {
        "rewrite_minimize": ("rewrite-minimize.p", [], ["rw", "cn"], 2),
        "condense": ("condense.p", ["--condense"], ["condense"], 1),
        "positive_sr": ("positive-sr.p", [], ["sr"], 2),
        "negative_sr": ("negative-sr.p", [], ["sr"], 2),
    }

    reports: dict[str, Any] = {}
    exact_cases = 0
    for name, (fixture_name, extra, expected_kinds, limit) in cases.items():
        fixture = experiment / fixture_name
        case_args = common + extra + [f"--processed-clauses-limit={limit}"]
        c = summarize(
            run(
                ["wsl.exe", "-d", args.distro, "--exec", args.c_exe]
                + case_args
                + [windows_to_wsl(fixture)]
            )
        )
        rust = summarize(
            run([str(args.rust_exe.resolve())] + case_args + [str(fixture.resolve())])
        )
        observed_kinds = [event["kind"] for event in c["events"]]
        exact = c == rust and observed_kinds == expected_kinds
        exact_cases += int(exact)
        reports[name] = {
            "display_args": case_args + ["$FIXTURE"],
            "expected_kinds": expected_kinds,
            "c": c,
            "rust": rust,
            "exact": exact,
        }

    report = {
        "schema_version": 1,
        "cases": reports,
        "exact_cases": exact_cases,
        "total_cases": len(cases),
        "all_exact": exact_cases == len(cases),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
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
    if not report["all_exact"]:
        print("focused ForwardModifyClause proof events differ", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
