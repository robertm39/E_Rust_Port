#!/usr/bin/env python3
"""Measure exact-clause overlap in prior consecutive SATCheck captures."""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
import statistics
import tarfile
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


CAPTURE_INDEX = re.compile(r"^(?P<index>\d+)-")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def percentile(values: list[float], proportion: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    position = proportion * (len(ordered) - 1)
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] + fraction * (
        ordered[upper] - ordered[lower]
    )


def summary(values: list[float]) -> dict[str, float | int | None]:
    return {
        "count": len(values),
        "minimum": min(values) if values else None,
        "median": statistics.median(values) if values else None,
        "p95": percentile(values, 0.95),
        "maximum": max(values) if values else None,
    }


def parse_clause_multiset(payload: bytes) -> collections.Counter[tuple[int, ...]]:
    clauses: collections.Counter[tuple[int, ...]] = collections.Counter()
    for raw_line in payload.decode("utf-8", errors="strict").splitlines():
        fields = raw_line.split()
        if not fields or fields[0] != "a":
            continue
        literals = [int(value) for value in fields[1:]]
        if not literals or literals[-1] != 0:
            raise ValueError(f"malformed add line: {raw_line}")
        normalized = tuple(
            sorted(literals[:-1], key=lambda value: (abs(value), value))
        )
        clauses[normalized] += 1
    return clauses


def manifest_members(archive: tarfile.TarFile) -> list[tarfile.TarInfo]:
    return sorted(
        (
            member
            for member in archive.getmembers()
            if member.isfile()
            and member.name.startswith("workloads/")
            and member.name.endswith("/manifest.json")
        ),
        key=lambda member: member.name,
    )


def load_sessions(
    archive: tarfile.TarFile,
) -> list[dict[str, Any]]:
    sessions_by_hash: dict[str, dict[str, Any]] = {}
    for manifest_member in manifest_members(archive):
        handle = archive.extractfile(manifest_member)
        if handle is None:
            raise ValueError(f"cannot read {manifest_member.name}")
        manifest = json.load(handle)
        parent = PurePosixPath(manifest_member.name).parent
        for source in manifest["sessions"]:
            if not {
                "capture_path",
                "problem_id",
                "session",
                "session_sha256",
            }.issubset(source):
                continue
            session_hash = str(source["session_sha256"])
            candidate = {
                **source,
                "archive_member": str(parent / str(source["session"])),
                "manifest_member": manifest_member.name,
            }
            existing = sessions_by_hash.get(session_hash)
            candidate_has_root = "source_workload_root" in candidate
            existing_has_root = (
                existing is not None
                and "source_workload_root" in existing
            )
            if (
                existing is None
                or (candidate_has_root and not existing_has_root)
                or (
                    candidate_has_root == existing_has_root
                    and candidate["archive_member"]
                    < existing["archive_member"]
                )
            ):
                sessions_by_hash[session_hash] = candidate
    return list(sessions_by_hash.values())


def capture_index(record: dict[str, Any]) -> int:
    filename = PurePosixPath(str(record["capture_path"])).name
    match = CAPTURE_INDEX.match(filename)
    if match is None:
        raise ValueError(f"capture path has no numeric index: {filename}")
    return int(match.group("index"))


def group_key(record: dict[str, Any]) -> tuple[str, str]:
    capture_path = PurePosixPath(str(record["capture_path"]))
    source = record.get("source_workload_root")
    if source is None:
        source = str(PurePosixPath(str(record["manifest_member"])).parent)
    return str(source), str(capture_path.parent)


def counter_size(counter: collections.Counter[tuple[int, ...]]) -> int:
    return sum(counter.values())


def analyze(archive_path: Path) -> dict[str, Any]:
    pairs: list[dict[str, Any]] = []
    with tarfile.open(archive_path, mode="r:gz") as archive:
        sessions = load_sessions(archive)
        by_group: dict[
            tuple[str, str], list[dict[str, Any]]
        ] = collections.defaultdict(list)
        for session in sessions:
            by_group[group_key(session)].append(session)

        for key, group in sorted(by_group.items()):
            ordered = sorted(
                group,
                key=lambda record: (
                    capture_index(record),
                    str(record["session_sha256"]),
                ),
            )
            snapshots: list[
                tuple[dict[str, Any], collections.Counter[tuple[int, ...]]]
            ] = []
            for record in ordered:
                handle = archive.extractfile(str(record["archive_member"]))
                if handle is None:
                    raise ValueError(
                        f"cannot read {record['archive_member']}"
                    )
                payload = handle.read()
                observed = hashlib.sha256(payload).hexdigest()
                if observed != record["session_sha256"]:
                    raise ValueError(
                        f"session hash mismatch for "
                        f"{record['archive_member']}"
                    )
                snapshots.append((record, parse_clause_multiset(payload)))

            for (left_record, left), (right_record, right) in zip(
                snapshots, snapshots[1:]
            ):
                intersection = left & right
                left_size = counter_size(left)
                right_size = counter_size(right)
                retained = counter_size(intersection)
                pairs.append(
                    {
                        "source_workload_root": key[0],
                        "capture_group": key[1],
                        "problem_id": left_record["problem_id"],
                        "left_index": capture_index(left_record),
                        "right_index": capture_index(right_record),
                        "left_clauses": left_size,
                        "right_clauses": right_size,
                        "retained_clauses": retained,
                        "removed_clauses": left_size - retained,
                        "added_clauses": right_size - retained,
                        "retained_from_previous": (
                            retained / left_size if left_size else 1.0
                        ),
                        "reusable_in_current": (
                            retained / right_size if right_size else 1.0
                        ),
                        "monotonic_add_only": left == intersection,
                        "identical": left == right,
                    }
                )

    retained_ratios = [
        float(pair["retained_from_previous"]) for pair in pairs
    ]
    reusable_ratios = [
        float(pair["reusable_in_current"]) for pair in pairs
    ]
    return {
        "schema_version": 1,
        "kind": "umlaut-ground-sat-exact-reuse-analysis",
        "source_archive": str(archive_path),
        "source_archive_sha256": sha256_file(archive_path),
        "interpretation": {
            "actual_runtime_cross_call_reuse": 0,
            "metric": (
                "multiset overlap of normalized exact integer clauses in "
                "consecutive captured sessions"
            ),
            "limitation": (
                "local atom renumbering can hide logically unchanged clauses; "
                "overlap is a conservative syntactic reuse estimate"
            ),
        },
        "sessions": len(sessions),
        "capture_groups": len(
            {(pair["source_workload_root"], pair["capture_group"]) for pair in pairs}
        ),
        "consecutive_pairs": len(pairs),
        "monotonic_add_only_pairs": sum(
            int(pair["monotonic_add_only"]) for pair in pairs
        ),
        "identical_pairs": sum(int(pair["identical"]) for pair in pairs),
        "retained_from_previous": summary(retained_ratios),
        "reusable_in_current": summary(reusable_ratios),
        "pairs": pairs,
    }


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def main(argv: Iterable[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("archive", type=Path)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args(argv)
    result = analyze(arguments.archive.resolve())
    if arguments.output is not None:
        write_json(arguments.output.resolve(), result)
    print(
        json.dumps(
            {
                key: result[key]
                for key in (
                    "source_archive_sha256",
                    "sessions",
                    "capture_groups",
                    "consecutive_pairs",
                    "monotonic_add_only_pairs",
                    "identical_pairs",
                    "retained_from_previous",
                    "reusable_in_current",
                )
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
