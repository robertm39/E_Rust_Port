#!/usr/bin/env python3
"""Deduplicate captured SATCheck DIMACS files and build ISAT sessions."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_dimacs(path: Path) -> tuple[int, list[tuple[int, ...]]]:
    maximum: int | None = None
    expected_clauses: int | None = None
    clauses: list[tuple[int, ...]] = []
    pending: list[int] = []
    for line_number, line in enumerate(path.read_text(encoding="ascii").splitlines(), 1):
        stripped = line.strip()
        if not stripped or stripped.startswith("c"):
            continue
        if stripped.startswith("p "):
            fields = stripped.split()
            if fields[:2] != ["p", "cnf"] or len(fields) != 4 or maximum is not None:
                raise ValueError(f"{path}:{line_number}: invalid DIMACS header")
            maximum = int(fields[2])
            expected_clauses = int(fields[3])
            continue
        if maximum is None:
            raise ValueError(f"{path}:{line_number}: clause precedes header")
        for field in stripped.split():
            literal = int(field)
            if literal == 0:
                clauses.append(tuple(pending))
                pending.clear()
            else:
                if abs(literal) > maximum:
                    raise ValueError(
                        f"{path}:{line_number}: literal exceeds declared maximum"
                    )
                pending.append(literal)
    if maximum is None or expected_clauses is None:
        raise ValueError(f"{path}: missing DIMACS header")
    if pending:
        raise ValueError(f"{path}: unterminated final clause")
    if len(clauses) != expected_clauses:
        raise ValueError(
            f"{path}: declared {expected_clauses} clauses, parsed {len(clauses)}"
        )
    return maximum, clauses


def deterministic_assumptions(maximum: int, digest: str, negative: bool) -> list[int]:
    if maximum == 0:
        return []
    count = min(4, maximum)
    selected: list[int] = []
    cursor = 0
    while len(selected) < count:
        chunk = digest[cursor : cursor + 8]
        if len(chunk) < 8:
            digest = hashlib.sha256(digest.encode()).hexdigest()
            cursor = 0
            continue
        variable = int(chunk, 16) % maximum + 1
        cursor += 8
        if variable not in selected:
            selected.append(variable)
    return [-variable for variable in selected] if negative else selected


def render_session(maximum: int, clauses: list[tuple[int, ...]], digest: str) -> str:
    lines = [f"p isat {maximum}"]
    for clause in clauses:
        literals = " ".join(str(literal) for literal in clause)
        lines.append(f"a {literals} 0" if literals else "a 0")
    lines.extend(
        [
            "q cold -1 0 0",
            "q warm1 -1 0 0",
            "q warm2 -1 0 0",
        ]
    )
    positive = deterministic_assumptions(maximum, digest, False)
    negative = deterministic_assumptions(maximum, digest, True)
    for name, assumptions in (("assume_pos", positive), ("assume_neg", negative)):
        rendered = " ".join(str(literal) for literal in assumptions)
        lines.append(f"q {name} -1 0 {rendered} 0" if rendered else f"q {name} -1 0 0")
    return "\n".join(lines) + "\n"


def prepare(
    capture_results: Path, capture_root: Path, output_root: Path
) -> dict[str, object]:
    output_root.mkdir(parents=True, exist_ok=True)
    if any(output_root.iterdir()):
        raise FileExistsError(f"output directory is not empty: {output_root}")
    seen: set[str] = set()
    manifest: list[dict[str, object]] = []
    captured = 0
    for line in capture_results.read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        result = json.loads(line)
        for capture in result["captures"]:
            captured += 1
            source = capture_root / capture["path"]
            digest = sha256(source)
            if digest != capture["sha256"]:
                raise ValueError(f"{source}: hash does not match capture result")
            if digest in seen:
                continue
            seen.add(digest)
            maximum, clauses = parse_dimacs(source)
            if maximum != capture["variables"] or len(clauses) != capture["clauses"]:
                raise ValueError(f"{source}: DIMACS shape does not match capture result")
            filename = (
                f"{result['holdout_split']}-{result['category']}-"
                f"{result['problem_id']}-{digest[:12]}.isat"
            )
            filename = "".join(
                character if character.isalnum() or character in "-_." else "_"
                for character in filename
            )
            destination = output_root / filename
            destination.write_text(
                render_session(maximum, clauses, digest),
                encoding="ascii",
                newline="\n",
            )
            manifest.append(
                {
                    "session": filename,
                    "session_sha256": sha256(destination),
                    "capture_path": capture["path"],
                    "capture_sha256": digest,
                    "problem_id": result["problem_id"],
                    "category": result["category"],
                    "division": result["division"],
                    "holdout_split": result["holdout_split"],
                    "family": result["family"],
                    "variables": maximum,
                    "clauses": len(clauses),
                    "bytes": capture["bytes"],
                }
            )
    manifest.sort(key=lambda record: str(record["session"]))
    (output_root / "manifest.json").write_text(
        json.dumps(
            {
                "schema": 1,
                "captured": captured,
                "unique": len(manifest),
                "sessions": manifest,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
        newline="\n",
    )
    return {"captured": captured, "unique": len(manifest)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("capture_results", type=Path)
    parser.add_argument("capture_root", type=Path)
    parser.add_argument("output_root", type=Path)
    arguments = parser.parse_args()
    print(
        json.dumps(
            prepare(
                arguments.capture_results,
                arguments.capture_root,
                arguments.output_root,
            ),
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
