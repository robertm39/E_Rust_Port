#!/usr/bin/env python3
"""Audit the outcome-blind explicit-AC corpus selection."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any, Sequence


EXPERIMENT_ROOT = Path(__file__).resolve().parent
IDENT = r"[a-z][A-Za-z0-9_]*"
VARIABLE = r"[A-Z][A-Za-z0-9_]*"
COMMUTATIVITY = re.compile(
    rf"(?P<f>{IDENT})\((?P<x>{VARIABLE}),(?P<y>{VARIABLE})\)"
    rf"=(?P=f)\((?P=y),(?P=x)\)"
)
ASSOCIATIVITY = re.compile(
    rf"(?P<f>{IDENT})\((?P=f)\((?P<x>{VARIABLE}),"
    rf"(?P<y>{VARIABLE})\),(?P<z>{VARIABLE})\)"
    rf"=(?P=f)\((?P=x),(?P=f)\((?P=y),(?P=z)\)\)"
)
REVERSED_ASSOCIATIVITY = re.compile(
    rf"(?P<f>{IDENT})\((?P<x>{VARIABLE}),"
    rf"(?P=f)\((?P<y>{VARIABLE}),(?P<z>{VARIABLE})\)\)"
    rf"=(?P=f)\((?P=f)\((?P=x),(?P=y)\),(?P=z)\)"
)


class AuditError(RuntimeError):
    """The selection is incomplete, contaminated, or internally inconsistent."""


def canonical_json(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_manifest(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    if not rows or rows[0].get("record_type") != "manifest":
        raise AuditError("invalid CASC manifest")
    return rows[0], rows[1:]


def normalized_source(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"%[^\n]*", "", text)
    return re.sub(r"\s+", "", text)


def matching_ac_symbols(path: Path) -> list[str]:
    source = normalized_source(path)
    commutative = {
        match.group("f")
        for match in COMMUTATIVITY.finditer(source)
    }
    associative = {
        match.group("f")
        for pattern in (ASSOCIATIVITY, REVERSED_ASSOCIATIVITY)
        for match in pattern.finditer(source)
    }
    return sorted(commutative & associative)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--problem-root", type=Path, required=True)
    parser.add_argument(
        "--selection",
        type=Path,
        default=EXPERIMENT_ROOT / "selected-problems.json",
    )
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    arguments = parse_args(argv)
    manifest_path = arguments.manifest.resolve()
    problem_root = arguments.problem_root.resolve()
    selection_path = arguments.selection.resolve()
    metadata, records = load_manifest(manifest_path)
    selection = json.loads(selection_path.read_text(encoding="utf-8"))
    selected_by_phase = {
        phase: set(selection[phase])
        for phase in ("calibration", "validation", "test")
    }
    selected_ids = set().union(*selected_by_phase.values())
    if sum(map(len, selected_by_phase.values())) != len(selected_ids):
        raise AuditError("a problem occurs in more than one phase")

    population = []
    for record in records:
        if record["category"] not in selection["categories"]:
            continue
        path = problem_root / record["path"]
        symbols = matching_ac_symbols(path)
        if symbols:
            population.append({**record, "ac_symbols": symbols})
    population_ids = {record["problem_id"] for record in population}
    if population_ids != selected_ids:
        raise AuditError(
            "selection differs from syntax-derived population: "
            f"missing={sorted(population_ids - selected_ids)}, "
            f"extra={sorted(selected_ids - population_ids)}"
        )

    split_to_phase = {
        "train": "calibration",
        "validation": "validation",
        "test": "test",
    }
    families: dict[str, set[str]] = {
        phase: set() for phase in split_to_phase.values()
    }
    for record in population:
        phase = split_to_phase[record["holdout_split"]]
        if record["problem_id"] not in selected_by_phase[phase]:
            raise AuditError(f"split mismatch for {record['problem_id']}")
        families[phase].add(record["family"])
    phase_names = tuple(families)
    for index, left in enumerate(phase_names):
        for right in phase_names[index + 1 :]:
            overlap = families[left] & families[right]
            if overlap:
                raise AuditError(f"family leakage between {left}/{right}: {overlap}")

    body = {
        "schema_version": 1,
        "manifest_sha256": sha256_file(manifest_path),
        "manifest_problem_count": metadata["problem_count"],
        "selection_sha256": sha256_file(selection_path),
        "population_size": len(population),
        "phase_counts": {
            phase: len(ids) for phase, ids in selected_by_phase.items()
        },
        "phase_families": {
            phase: sorted(values) for phase, values in families.items()
        },
        "records": [
            {
                "problem_id": record["problem_id"],
                "path": record["path"],
                "sha256": record["sha256"],
                "category": record["category"],
                "family": record["family"],
                "holdout_split": record["holdout_split"],
                "ac_symbols": record["ac_symbols"],
            }
            for record in sorted(population, key=lambda value: value["problem_id"])
        ],
    }
    report = {
        **body,
        "report_id": hashlib.sha256(canonical_json(body)).hexdigest(),
    }
    arguments.output.resolve().write_bytes(canonical_json(report) + b"\n")
    print(
        f"OK: {len(population)} explicit-AC problems; "
        f"report {report['report_id']}"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AuditError, OSError, KeyError, ValueError, json.JSONDecodeError) as error:
        print(f"error: {error}")
        raise SystemExit(2) from error
