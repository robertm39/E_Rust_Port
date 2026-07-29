#!/usr/bin/env python3
"""Generate deterministic selector-guarded AVATAR-style SAT sessions."""

from __future__ import annotations

import argparse
import hashlib
import json
import random
from pathlib import Path

SEED = 20_260_729
TARGET_CLAUSES = (96, 127, 128, 160, 192, 238, 255, 256, 320, 372, 384, 512)


def pigeonhole(pigeons: int, holes: int) -> tuple[int, list[list[int]]]:
    def variable(pigeon: int, hole: int) -> int:
        return pigeon * holes + hole + 1

    clauses: list[list[int]] = []
    for pigeon in range(pigeons):
        clauses.append([variable(pigeon, hole) for hole in range(holes)])
        for first in range(holes):
            for second in range(first + 1, holes):
                clauses.append(
                    [-variable(pigeon, first), -variable(pigeon, second)]
                )
    for hole in range(holes):
        for first in range(pigeons):
            for second in range(first + 1, pigeons):
                clauses.append(
                    [-variable(first, hole), -variable(second, hole)]
                )
    return pigeons * holes, clauses


def base_for_target(target: int) -> tuple[int, list[list[int]]]:
    if target < 141:
        return pigeonhole(5, 4)
    if target < 238:
        return pigeonhole(6, 5)
    if target < 372:
        return pigeonhole(7, 6)
    return pigeonhole(8, 7)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def render_session(target: int) -> tuple[str, int]:
    atom_max, component = base_for_target(target)
    selector = atom_max + 1
    filler_selector = selector + 1
    clauses = [[*clause, -selector] for clause in component]
    filler_variable = filler_selector + 1
    while len(clauses) < target:
        clauses.append([filler_variable, -filler_variable, -filler_selector])
        filler_variable += 1
    if len(clauses) != target:
        raise AssertionError(f"target {target} is below its selected component")

    rng = random.Random(SEED ^ target)
    rng.shuffle(clauses)
    lines = [f"p isat {filler_variable - 1}"]
    lines.extend(f"a {' '.join(map(str, clause))} 0" for clause in clauses)
    lines.extend(
        (
            f"q inactive -1 0 {-selector} {-filler_selector} 0",
            f"q active -1 0 {selector} {-filler_selector} 0",
            f"q active_repeat -1 0 {selector} {-filler_selector} 0",
            f"q filler_active -1 0 {-selector} {filler_selector} 0",
            f"q inactive_repeat -1 0 {-selector} {-filler_selector} 0",
        )
    )
    return "\n".join(lines) + "\n", len(component)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    arguments = parser.parse_args()
    arguments.output.mkdir(parents=True, exist_ok=True)
    if any(arguments.output.iterdir()):
        raise FileExistsError(f"output is not empty: {arguments.output}")

    sessions = []
    for target in TARGET_CLAUSES:
        path = arguments.output / f"avatar-selector-{target:03}.isat"
        content, component_clauses = render_session(target)
        path.write_text(content, encoding="ascii", newline="\n")
        sessions.append(
            {
                "session": path.name,
                "sha256": sha256(path),
                "workload_class": "avatar-style",
                "clauses": target,
                "active_component_clauses": component_clauses,
                "seed": SEED,
            }
        )
    (arguments.output / "manifest.json").write_text(
        json.dumps(
            {
                "schema": 1,
                "seed": SEED,
                "thresholds": [128, 256],
                "sessions": sessions,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps({"sessions": len(sessions), "seed": SEED}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
