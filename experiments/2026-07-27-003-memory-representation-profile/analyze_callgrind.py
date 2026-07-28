#!/usr/bin/env python3
"""Extract self and inclusive event cost for one Callgrind function."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


FUNCTION_RE = re.compile(r"^fn=\((?P<id>\d+)\)(?: (?P<name>.*))?$")
COST_POSITION_RE = re.compile(r"^(?:[+*-]?\d+|\*)$")


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("profile", type=Path)
    parser.add_argument("--function", required=True, dest="function_fragment")
    return parser.parse_args()


def event_schema(lines: list[str]) -> tuple[list[str], list[int]]:
    events: list[str] | None = None
    summary: list[int] | None = None
    for line in lines:
        if line.startswith("events: "):
            events = line.split()[1:]
        elif line.startswith("summary: "):
            summary = [int(value) for value in line.split()[1:]]
    if events is None or summary is None or len(events) != len(summary):
        raise RuntimeError("Callgrind event schema or summary is missing")
    return events, summary


def function_names(lines: list[str]) -> dict[str, str]:
    names: dict[str, str] = {}
    for line in lines:
        match = FUNCTION_RE.match(line)
        if match is not None and match.group("name"):
            names[match.group("id")] = match.group("name")
    return names


def cost_values(line: str, event_count: int) -> list[int] | None:
    fields = line.split()
    if len(fields) <= event_count:
        return None
    positions = fields[:-event_count]
    if not positions or not all(COST_POSITION_RE.match(field) for field in positions):
        return None
    try:
        return [int(value) for value in fields[-event_count:]]
    except ValueError:
        return None


def extract_function_cost(
    lines: list[str],
    target_ids: set[str],
    event_count: int,
) -> tuple[list[int], list[int], int]:
    self_cost = [0] * event_count
    inclusive_cost = [0] * event_count
    current_is_target = False
    next_cost_is_call = False
    blocks = 0

    for line in lines:
        function_match = FUNCTION_RE.match(line)
        if function_match is not None:
            current_is_target = function_match.group("id") in target_ids
            next_cost_is_call = False
            if current_is_target:
                blocks += 1
            continue
        if not current_is_target:
            continue
        if line.startswith("cfn="):
            next_cost_is_call = True
            continue
        if line.startswith("calls="):
            continue
        values = cost_values(line, event_count)
        if values is None:
            continue
        for index, value in enumerate(values):
            inclusive_cost[index] += value
            if not next_cost_is_call:
                self_cost[index] += value
        next_cost_is_call = False

    return self_cost, inclusive_cost, blocks


def main() -> int:
    args = parse_args()
    profile = args.profile.resolve()
    lines = profile.read_text(encoding="utf-8").splitlines()
    events, summary = event_schema(lines)
    names = function_names(lines)
    target_ids = {
        function_id
        for function_id, name in names.items()
        if args.function_fragment in name
    }
    if not target_ids:
        raise RuntimeError(f"no function contains {args.function_fragment!r}")
    matched_names = sorted({names[function_id] for function_id in target_ids})
    self_cost, inclusive_cost, blocks = extract_function_cost(
        lines, target_ids, len(events)
    )
    result = {
        "schema": "umlaut.callgrind-function-cost",
        "schema_version": 1,
        "profile_sha256": sha256_file(profile),
        "function_fragment": args.function_fragment,
        "matched_functions": matched_names,
        "function_ids": sorted(target_ids, key=int),
        "blocks": blocks,
        "events": {
            event: {
                "program_total": summary[index],
                "self": self_cost[index],
                "inclusive": inclusive_cost[index],
                "inclusive_percent": (
                    100.0 * inclusive_cost[index] / summary[index]
                    if summary[index]
                    else 0.0
                ),
            }
            for index, event in enumerate(events)
        },
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
